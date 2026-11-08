# /// script
# requires-python = ">=3.11"
# ///
"""Builds the docs site: README.md and docs/, rendered by Kelpto.

    uv run site/docs/build.py [--out DIR] [--kelpto DIR] [--tempest BIN]

Kelpto, the Markdown and HTML library written in Temper, renders every
page, built for Python by tempest itself: this copies Kelpto's four
libraries (kelpto-core, kelpto-md, kelpto-html, and kelpto) from
`--kelpto DIR` (default `../kelpto`, beside this repository; CI checks
out the commit in `site/docs/kelpto-ref`) into `site/_out/kelpto/`,
runs `tempest build -b py` there, and imports the result. Nothing else
is needed but Python's standard library.

Each page is parsed as GitHub Flavored Markdown, with raw HTML parsed as
HTML, and rendered by Kelpto's `renderSafe`. Then this script adds what
Kelpto doesn't do (yet): GitHub's heading anchors, GitHub's alerts,
Temper highlighting (with the VS Code extension's grammar, through
editors/vscode/check_grammar.py's tokenizer), and links rewritten for
the site: `.md` to `.html` (`README.md` to `index.html`), and the
playground's absolute link to its relative path. Links to files outside
docs/ (sources in the repository) become plain text, and are counted.

Writes to `--out DIR` (default `site/_out/site/`): `index.html` from
README.md, `docs/…` from docs/, `assets/site.css`, and the licenses as
`.txt`. The playground goes in `playground/` beside them, built by
site/playground/build.py. Then it checks that every link in the site
leads to a page, file, and heading that exist, and exits with 1 if not.
"""

import argparse
import hashlib
import html
import posixpath
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path
from urllib.parse import unquote

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
DOCS = ROOT / "docs"
OUT = ROOT / "site" / "_out" / "site"
WORK = ROOT / "site" / "_out" / "kelpto"
KELPTO_LIBS = ["kelpto-core", "kelpto-md", "kelpto-html", "kelpto"]
# The playground's published address, from README.md, as pages.yml reads it.
SITE_URL = re.compile(r"https://temperlang\.github\.io/tempest/[0-9a-f]{32}/")
# The header's sections: docs/ folders and their names.
SECTIONS = [
    ("guide", "Guide"),
    ("language", "Language"),
    ("std", "Std"),
    ("tools", "Tools"),
    ("backends", "Backends"),
    ("contributing", "Contributing"),
    ("internals", "Internals"),
    ("release-notes", "Release notes"),
]
# Files at the top of the repository published as text, for the license links.
TEXT_FILES = ["COPYRIGHT", "NOTICE", "LICENSE-APACHE", "LICENSE-MIT", "LICENSE-CC-BY-SA-4.0"]
ALERTS = {"NOTE": "Note", "TIP": "Tip", "IMPORTANT": "Important", "WARNING": "Warning", "CAUTION": "Caution"}
# TextMate scopes to the stylesheet's classes, most specific first.
SCOPE_CLASSES = [
    ("comment", "c"),
    ("punctuation.section.embedded", "e"),
    ("string", "s"),
    ("constant.numeric", "n"),
    ("constant.language", "k"),
    ("entity.name.function.decorator", "d"),
    ("entity.name.function", "f"),
    ("entity.name.type", "t"),
    ("storage", "k"),
    ("keyword.operator", "o"),
    ("keyword", "k"),
]
GRAMMARS = {"temper": "source.temper", "tempermd": "text.html.markdown.temper"}


# Building Kelpto


def build_kelpto(src: Path, tempest: Path) -> Path:
    """Kelpto's libraries, built for Python by tempest, in WORK; rebuilt
    only when they or tempest change."""
    missing = [lib for lib in KELPTO_LIBS if not (src / lib / "config.temper.md").is_file()]
    if missing:
        sys.exit(f"build.py: no Kelpto at {src} (missing {', '.join(missing)}); pass --kelpto DIR")
    digest = hashlib.sha256()
    for lib in KELPTO_LIBS:
        for f in sorted((src / lib).rglob("*")):
            if f.is_file():
                digest.update(f.relative_to(src).as_posix().encode() + b"\0" + f.read_bytes())
    stat = tempest.stat()
    digest.update(f"{tempest.resolve()} {stat.st_size} {stat.st_mtime_ns}".encode())
    stamp = WORK / "stamp"
    py = WORK / "temper.out" / "py"
    if stamp.is_file() and stamp.read_text() == digest.hexdigest() and py.is_dir():
        return py
    shutil.rmtree(WORK, ignore_errors=True)
    WORK.mkdir(parents=True)
    for lib in KELPTO_LIBS:
        shutil.copytree(src / lib, WORK / lib)
    run = subprocess.run([str(tempest), "build", "-b", "py", "-w", str(WORK)], capture_output=True, text=True)
    if run.returncode != 0 or not py.is_dir():
        sys.exit(f"build.py: tempest build -b py failed for Kelpto:\n{run.stdout}{run.stderr}")
    stamp.write_text(digest.hexdigest())
    return py


class Kelpto:
    """GFM with raw HTML, rendered safely, through the built Kelpto."""

    def __init__(self, py: Path):
        for lib in sorted(py.iterdir()):
            sys.path.insert(0, str(lib))
        from kelpto import ParsedRawHtml, render_safe
        from kelpto_html import SanitizerPolicy
        from kelpto_md import MarkdownParser

        self.parser = MarkdownParser.gfm.with_raw_html_handler(ParsedRawHtml())
        self.render_safe = render_safe
        # Standard, plus the README's centered header and icon table.
        self.policy = SanitizerPolicy.standard.with_attributes("p", ["align"]).with_attributes("tr", ["valign"])

    def render(self, text: str) -> tuple[str, list[str]]:
        safe = self.render_safe(self.parser.parse(text), self.policy)
        return safe.html, [f"{r.kind} {r.name}" for r in safe.removals]


# Highlighting


class Highlighter:
    def __init__(self) -> None:
        sys.path.insert(0, str(ROOT / "editors" / "vscode"))
        from check_grammar import Tokenizer

        self.tokenizers = {lang: Tokenizer(scope) for lang, scope in GRAMMARS.items()}

    @staticmethod
    def css_class(scopes: list[str]) -> str | None:
        for scope in reversed(scopes):
            for prefix, cls in SCOPE_CLASSES:
                if scope == prefix or scope.startswith(prefix + "."):
                    return cls
        return None

    def highlight(self, lang: str, code: str) -> str | None:
        """`code` as HTML with spans, or None to leave it plain."""
        tokenizer = self.tokenizers.get(lang)
        if tokenizer is None:
            return None
        try:
            tokens = tokenizer.tokenize(code)
        except Exception:  # a grammar the small tokenizer can't follow
            return None
        out, at = [], 0
        # The tokenizer drops line breaks, so put them back from the source.
        for text, scopes in tokens:
            while code.startswith("\n", at) and not code.startswith(text, at):
                out.append("\n")
                at += 1
            if not code.startswith(text, at):
                return None
            cls = self.css_class(scopes)
            escaped = html.escape(text, quote=False)
            out.append(f'<span class="{cls}">{escaped}</span>' if cls else escaped)
            at += len(text)
        if code[at:].strip("\n"):
            return None
        out.append(code[at:])
        return "".join(out)


# Pages


def out_path(src: str) -> str:
    """Where a Markdown file, relative to ROOT, goes in the site."""
    if src == "README.md":
        return "index.html"
    if src.endswith("/README.md"):
        return src[: -len("README.md")] + "index.html"
    return src[: -len(".md")] + ".html"


def relative(target: str, page_out: str) -> str:
    """A link from `page_out` to `target`, both relative to the site's root;
    index pages as their folder."""
    rel = posixpath.relpath(target, posixpath.dirname(page_out) or ".")
    if rel == "index.html":
        return "./"
    if rel.endswith("/index.html"):
        return rel[: -len("index.html")]
    return rel


def text_of(fragment: str) -> str:
    return html.unescape(re.sub(r"<[^>]+>", "", fragment)).strip()


def slug(text: str) -> str:
    """GitHub's heading anchor, as compat/check_anchors.py makes it."""
    return re.sub(r"[^\w\- ]", "", text.strip().lower()).replace(" ", "-")


class Site:
    def __init__(self, kelpto: Kelpto, highlighter: Highlighter, out: Path):
        self.kelpto = kelpto
        self.highlighter = highlighter
        self.out = out
        self.sources = ["README.md"] + sorted(p.relative_to(ROOT).as_posix() for p in DOCS.rglob("*.md"))
        self.offsite: dict[str, int] = {}  # source paths linked from pages, with counts
        self.stale: list[str] = []  # links to the site at another path than README.md's
        found = SITE_URL.search((ROOT / "README.md").read_text())
        if not found:
            sys.exit("build.py: no playground link with a 32-hex path in README.md")
        self.site_url = found.group(0)
        self.removed: list[str] = []
        self.titles: dict[str, str] = {}
        self.headings: dict[str, list[tuple[str, str]]] = {}  # h2s: id, text

    # Links

    def target(self, href: str, src: str) -> tuple[str | None, str | None]:
        """A link's site address, relative to the site's root, with its
        fragment; or None and the repository path it named, when that isn't
        on the site."""
        path, hash_, fragment = href.partition("#")
        path = posixpath.normpath(posixpath.join(posixpath.dirname(src), unquote(path)))
        if path.startswith("../"):
            return None, path
        file = ROOT / path
        if file.is_dir() and (file / "README.md").is_file() and (path == "docs" or path.startswith("docs/")):
            path = posixpath.join(path, "README.md")
            file = ROOT / path
        tail = hash_ + fragment
        if path.startswith("docs/") and file.is_file():
            return (out_path(path) if path.endswith(".md") else path) + tail, None
        if path == "README.md":
            return "index.html" + tail, None
        if path in TEXT_FILES:
            return path + ".txt" + tail, None
        return None, path

    def rewrite_links(self, body: str, src: str, page: str) -> str:
        def site_relative(href: str) -> str | None:
            """A link's new href, or None if it can't be on the site."""
            if href.startswith("#"):
                return href
            if m := SITE_URL.match(href):
                if m.group(0) != self.site_url:
                    self.stale.append(f"{src}: {href}: not README.md's path, {self.site_url}")
                return relative(href[m.end() :] or "index.html", page)
            if re.match(r"[A-Za-z][A-Za-z0-9+.-]*:|//", href):
                return href
            target, outside = self.target(href, src)
            if target is None:
                self.offsite[outside or href] = self.offsite.get(outside or href, 0) + 1
                return None
            path, hash_, fragment = target.partition("#")
            return relative(path, page) + hash_ + fragment

        def anchor(m: re.Match) -> str:
            attrs, inner = m.group(1), m.group(2)
            found = re.search(r'\bhref="([^"]*)"', attrs)
            if not found:
                return m.group(0)
            href = html.unescape(found.group(1))
            new = site_relative(href)
            if new is None:
                note = html.escape(f"In the tempest repository: {posixpath.normpath(posixpath.join(posixpath.dirname(src), href.partition('#')[0]))}")
                return f'<span class="offsite" title="{note}">{inner}</span>'
            attrs = attrs.replace(found.group(0), 'href="' + html.escape(new) + '"')
            return f"<a{attrs}>{inner}</a>"

        def image(m: re.Match) -> str:
            href = html.unescape(m.group(2))
            new = site_relative(href)
            if new is None:
                self.offsite[href] = self.offsite.get(href, 0) + 1
                return m.group(0)
            return f'{m.group(1)}"{html.escape(new)}"'

        body = re.sub(r"<a\b([^>]*)>(.*?)</a>", anchor, body, flags=re.S)
        return re.sub(r'(<img\b[^>]*?\bsrc=)"([^"]*)"', image, body)

    # Blocks

    def code_blocks(self, body: str) -> str:
        def block(m: re.Match) -> str:
            lang, code = m.group(1) or "", m.group(2)
            if lang == "mermaid":
                return (
                    '<figure class="diagram"><figcaption>Diagram, as Mermaid source</figcaption>'
                    f"<pre><code>{code}</code></pre></figure>"
                )
            lit = self.highlighter.highlight(lang, html.unescape(code))
            cls = f' class="language-{lang}"' if lang else ""
            return f"<pre><code{cls}>{lit if lit is not None else code}</code></pre>"

        return re.sub(r'<pre><code(?: class="language-([^"]+)")?>(.*?)</code></pre>', block, body, flags=re.S)

    @staticmethod
    def alerts(body: str) -> str:
        """GitHub's alerts: a block quote starting `[!NOTE]` and the like."""
        marker = re.compile(r"<blockquote>\s*<p>\[!(" + "|".join(ALERTS) + r")\]\s*")
        while m := marker.search(body):
            depth, at = 1, m.end()
            for tag in re.finditer(r"<(/?)blockquote>", body[at:]):
                depth += -1 if tag.group(1) else 1
                if depth == 0:
                    end = at + tag.start()
                    break
            else:
                break
            kind = m.group(1)
            inner = re.sub(r"^</p>\s*", "", "<p>" + body[m.end() : end])
            inner = inner.removeprefix("<p></p>")
            head = f'<div class="alert alert-{kind.lower()}"><p class="alert-title">{ALERTS[kind]}</p>'
            body = body[: m.start()] + head + inner + "</div>" + body[end + len("</blockquote>") :]
        return body

    def heading_ids(self, body: str, src: str) -> str:
        counts: dict[str, int] = {}
        h2s: list[tuple[str, str]] = []

        def heading(m: re.Match) -> str:
            level, attrs, inner = m.group(1), m.group(2), m.group(3)
            text = text_of(inner)
            if src not in self.titles and level == "1":
                self.titles[src] = text
            base = slug(text)
            n = counts.get(base, 0)
            counts[base] = n + 1
            if "id=" in attrs:
                return m.group(0)
            ident = base if n == 0 else f"{base}-{n}"
            if level == "2":
                h2s.append((ident, text))
            return f'<h{level}{attrs} id="{ident}">{inner}<a class="hash" href="#{ident}" aria-label="Link to this section">#</a></h{level}>'

        body = re.sub(r"<h([1-6])([^>]*)>(.*?)</h\1>", heading, body, flags=re.S)
        self.headings[src] = h2s
        return body

    # Navigation

    def section_of(self, src: str) -> str | None:
        parts = src.split("/")
        if len(parts) > 2 and parts[0] == "docs" and parts[1] in dict(SECTIONS):
            return parts[1]
        return None

    def section_pages(self, section: str) -> list[str]:
        """A section's pages: its README first, then in the order the
        README links them, then the rest by path."""
        index = f"docs/{section}/README.md"
        pages = [s for s in self.sources if s.startswith(f"docs/{section}/")]
        order = [index]
        for href in re.findall(r"\]\(([^)\s#]+)", (ROOT / index).read_text()):
            target, _ = self.target(href, index)
            for s in pages:
                if target and out_path(s) == target and s not in order:
                    order.append(s)
        return order + [s for s in pages if s not in order]

    def sidebar(self, src: str, page: str) -> str:
        section = self.section_of(src)
        if section:
            label = dict(SECTIONS)[section]
            pages = self.section_pages(section)
        else:
            label = "Documentation"
            pages = ["docs/README.md"] + [f"docs/{s}/README.md" for s, _ in SECTIONS] + ["docs/inconsistencies.md"]
        items = []
        for s in pages:
            depth = s.count("/") - (2 if section else 1) - (1 if s.endswith("/README.md") else 0)
            title = self.titles.get(s, s)
            current = ' aria-current="page"' if s == src else ""
            sub = ""
            if s == src and len(self.headings.get(s, [])) > 1:
                links = "".join(f'<li><a href="#{i}">{html.escape(t)}</a></li>' for i, t in self.headings[s])
                sub = f'<ul class="toc">{links}</ul>'
            items.append(
                f'<li class="depth-{max(depth, 0)}"><a href="{relative(out_path(s), page)}"{current}>'
                f"{html.escape(title)}</a>{sub}</li>"
            )
        return f'<nav class="side" aria-label="{label}"><p class="side-title">{label}</p><ul>{"".join(items)}</ul></nav>'

    def header(self, src: str, page: str) -> str:
        section = self.section_of(src)
        links = []
        for s, name in SECTIONS:
            current = ' aria-current="true"' if s == section else ""
            links.append(f'<a href="{relative(f"docs/{s}/index.html", page)}"{current}>{name}</a>')
        links.append(f'<a class="play" href="{relative("playground/index.html", page)}">Playground</a>')
        logo = relative("docs/images/temper-logo-text-auto.svg", page)
        return (
            f'<header class="top"><a class="brand" href="{relative("index.html", page)}">'
            f'<img src="{logo}" alt="Temper" width="106" height="30"></a>'
            f'<nav class="sections" aria-label="Sections">{"".join(links)}</nav></header>'
        )

    def page(self, src: str, body: str) -> str:
        page = out_path(src)
        title = "Temper" if src == "README.md" else f"{self.titles.get(src, src)} · Temper"
        css = relative("assets/site.css", page)
        licenses = relative("COPYRIGHT.txt", page)
        return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex, nofollow">
<meta name="referrer" content="no-referrer">
<meta name="color-scheme" content="light dark">
<title>{html.escape(title)}</title>
<link rel="stylesheet" href="{css}">
</head>
<body>
<a class="skip" href="#content">Skip to the content</a>
{self.header(src, page)}
<div class="layout">
<main id="content" class="doc">
{body}
</main>
{self.sidebar(src, page)}
</div>
<footer class="foot">
<p>Documentation <a href="{licenses}">CC BY-SA 4.0</a>; code Apache 2.0 or MIT.
Rendered by <a href="https://github.com/temperlang/kelpto">Kelpto</a>, a Temper library, built for Python by tempest.</p>
</footer>
</body>
</html>
"""

    # Building

    def build(self) -> int:
        bodies = {}
        for src in self.sources:
            body, removed = self.kelpto.render((ROOT / src).read_text())
            self.removed += [f"{src}: {r}" for r in removed]
            page = out_path(src)
            body = self.code_blocks(body)
            body = self.alerts(body)
            body = self.heading_ids(body, src)
            body = self.rewrite_links(body, src, page)
            body = re.sub(r"<table>", '<div class="table-wrap"><table>', body).replace("</table>", "</table></div>")
            bodies[src] = body
        if self.out.exists():
            for child in self.out.iterdir():
                if child.name != "playground":  # built by site/playground/build.py
                    shutil.rmtree(child) if child.is_dir() else child.unlink()
        self.out.mkdir(parents=True, exist_ok=True)
        for src, body in bodies.items():
            dest = self.out / out_path(src)
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_text(self.page(src, body))
        for f in DOCS.rglob("*"):
            if f.is_file() and f.suffix != ".md":
                dest = self.out / f.relative_to(ROOT)
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(f, dest)
        for name in TEXT_FILES:
            shutil.copyfile(ROOT / name, self.out / f"{name}.txt")
        (self.out / "assets").mkdir(exist_ok=True)
        shutil.copyfile(HERE / "style.css", self.out / "assets" / "site.css")
        return len(bodies)


# Checking


def check(out: Path) -> tuple[list[str], int, list[str]]:
    """Every relative href and src in the site leads to a file, and every
    fragment to an id in it. Gives the problems, the links checked, and
    what wasn't checked (the playground, when it isn't built)."""
    pages = {p: p.read_text() for p in out.rglob("*.html") if "playground" not in p.relative_to(out).parts[:1]}
    ids = {p: set(re.findall(r'\bid="([^"]+)"', t)) for p, t in pages.items()}
    problems, skipped, count = [], [], 0
    for page, text in pages.items():
        where = page.relative_to(out).as_posix()
        for href in re.findall(r'\b(?:href|src)="([^"]*)"', text):
            href = html.unescape(href)
            if re.match(r"[A-Za-z][A-Za-z0-9+.-]*:|//", href):
                continue
            count += 1
            path, _, fragment = href.partition("#")
            target = (page.parent / unquote(path)).resolve() if path else page
            if target.is_dir():
                target = target / "index.html"
            if not target.is_relative_to(out.resolve()):
                problems.append(f"{where}: {href}: outside the site")
            elif not target.exists():
                if target.relative_to(out.resolve()).parts[0] == "playground":
                    skipped.append(f"{where}: {href}")
                else:
                    problems.append(f"{where}: {href}: no such file")
            elif fragment and target.suffix == ".html" and unquote(fragment) not in ids.get(target, set()):
                if target in ids or target.relative_to(out.resolve()).parts[0] != "playground":
                    problems.append(f"{where}: {href}: no such heading")
    return problems, count, skipped


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--out", type=Path, default=OUT, help="the site's root (default site/_out/site)")
    parser.add_argument("--kelpto", type=Path, default=ROOT.parent / "kelpto", help="a Kelpto checkout")
    parser.add_argument("--tempest", type=Path, help="tempest (default target/release/tempest, else target/small/tempest)")
    args = parser.parse_args()
    started = time.monotonic()
    tempest = args.tempest or next(
        (p for p in (ROOT / "target/release/tempest", ROOT / "target/small/tempest") if p.is_file()), None
    )
    if tempest is None or not tempest.is_file():
        sys.exit("build.py: no tempest; run `cargo build --release -p tempest`, or pass --tempest BIN")
    sys.dont_write_bytecode = True
    py = build_kelpto(args.kelpto.resolve(), tempest)
    built = time.monotonic()
    site = Site(Kelpto(py), Highlighter(), args.out.resolve())
    n = site.build()
    rendered = time.monotonic()
    problems, count, skipped = check(args.out.resolve())
    problems = site.stale + problems
    print(f"build.py: {n} pages in {args.out} (Kelpto {built - started:.1f} s, pages {rendered - built:.1f} s)")
    offsite = sum(site.offsite.values())
    if offsite:
        tops: dict[str, int] = {}
        for path, k in site.offsite.items():
            top = path.split("/")[0] if "/" in path else path
            tops[top] = tops.get(top, 0) + k
        listed = ", ".join(f"{t} {k}" for t, k in sorted(tops.items(), key=lambda x: -x[1]))
        print(f"build.py: {offsite} links to {len(site.offsite)} repository files not on the site, shown as text ({listed})")
    if site.removed:
        kinds: dict[str, int] = {}
        for r in site.removed:
            kind = r.split(": ", 1)[1].strip()
            kinds[kind] = kinds.get(kind, 0) + 1
        listed = ", ".join(f"{k} {n}" for k, n in sorted(kinds.items(), key=lambda x: -x[1]))
        print(f"build.py: Kelpto's sanitizer removed {len(site.removed)} things ({listed})")
    if skipped:
        print(f"build.py: {len(skipped)} links into playground/ not checked (it isn't built here)")
    for p in problems:
        print(f"build.py: {p}", file=sys.stderr)
    print(f"build.py: {count} links checked, {len(problems)} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
