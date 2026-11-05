"""Checks generated Python modules' module-level names.

Each module given must set `__all__` once, to a tuple or list of string
literals, each a name the module binds, and every other name it binds
at module level must start with `_`: its exports are its only public
names. Run by `functional.rs`'s `runs` over every library module that
the functional tests generate, as `python module_names.py FILE...`;
prints each problem and exits with 1 if there are any.
"""

import ast
import sys


def targets(node: ast.AST, out: list[tuple[str, int]]) -> None:
    """Names an assignment's target binds."""
    if isinstance(node, ast.Name):
        out.append((node.id, node.lineno))
    elif isinstance(node, (ast.Tuple, ast.List)):
        for elt in node.elts:
            targets(elt, out)
    elif isinstance(node, ast.Starred):
        targets(node.value, out)


def walruses(node: ast.AST, out: list[tuple[str, int]]) -> None:
    """Names `:=` binds in an expression, outside lambdas."""
    if isinstance(node, ast.Lambda):
        return
    if isinstance(node, ast.NamedExpr):
        targets(node.target, out)
    for child in ast.iter_child_nodes(node):
        if not isinstance(child, ast.stmt):
            walruses(child, out)


def bindings(body: list[ast.stmt], out: list[tuple[str, int]]) -> None:
    """Names statements bind in the scope they're in, nested blocks too."""
    for s in body:
        for child in ast.iter_child_nodes(s):
            if not isinstance(child, (ast.stmt, ast.excepthandler)):
                if not isinstance(s, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                    walruses(child, out)
        if isinstance(s, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            out.append((s.name, s.lineno))
        elif isinstance(s, ast.Assign):
            for t in s.targets:
                targets(t, out)
        elif isinstance(s, (ast.AnnAssign, ast.AugAssign)):
            targets(s.target, out)
        elif isinstance(s, (ast.Import, ast.ImportFrom)):
            for alias in s.names:
                name = alias.asname or alias.name.split(".")[0]
                out.append((name, s.lineno))
        elif isinstance(s, (ast.For, ast.AsyncFor)):
            targets(s.target, out)
            bindings(s.body, out)
            bindings(s.orelse, out)
        elif isinstance(s, (ast.While, ast.If)):
            bindings(s.body, out)
            bindings(s.orelse, out)
        elif isinstance(s, (ast.With, ast.AsyncWith)):
            for item in s.items:
                if item.optional_vars is not None:
                    targets(item.optional_vars, out)
            bindings(s.body, out)
        elif isinstance(s, (ast.Try, ast.TryStar)):
            bindings(s.body, out)
            for h in s.handlers:
                if h.name is not None:
                    out.append((h.name, h.lineno))
                bindings(h.body, out)
            bindings(s.orelse, out)
            bindings(s.finalbody, out)
        elif isinstance(s, ast.Match):
            for case in s.cases:
                bindings(case.body, out)


def exports(tree: ast.Module) -> list[str] | str:
    """`__all__`'s names, or what's wrong with it."""
    found: list[str] | None = None
    for s in tree.body:
        if isinstance(s, ast.Assign) and any(
            isinstance(t, ast.Name) and t.id == "__all__" for t in s.targets
        ):
            if found is not None:
                return "sets __all__ more than once"
            if not isinstance(s.value, (ast.Tuple, ast.List)) or not all(
                isinstance(e, ast.Constant) and isinstance(e.value, str)
                for e in s.value.elts
            ):
                return "sets __all__ to something but string literals"
            found = [
                e.value
                for e in s.value.elts
                if isinstance(e, ast.Constant) and isinstance(e.value, str)
            ]
    if found is None:
        return "doesn't set __all__"
    return found


def check(path: str) -> list[str]:
    with open(path, encoding="utf-8") as f:
        tree = ast.parse(f.read(), path)
    listed = exports(tree)
    if isinstance(listed, str):
        return [f"{path}: {listed}"]
    bound: list[tuple[str, int]] = []
    bindings(tree.body, bound)
    names = {name for name, _ in bound}
    problems = [f"{path}: __all__ names {n!r}, which it doesn't bind" for n in listed if n not in names]
    if len(set(listed)) != len(listed):
        problems.append(f"{path}: __all__ names something twice")
    seen = set()
    for name, line in bound:
        if name in listed or name == "__all__" or name.startswith("_") or name in seen:
            continue
        seen.add(name)
        problems.append(f"{path}:{line}: {name!r} is public but not in __all__")
    return problems


def main() -> None:
    problems = [p for path in sys.argv[1:] for p in check(path)]
    for p in problems:
        print(p)
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
