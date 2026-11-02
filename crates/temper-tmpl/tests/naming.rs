//! The naming pass ([`temper_tmpl::naming`]): Kotlin's snake and Pascal
//! cases, and
//! names shared, prefixed, escaped, and suffixed only on real conflicts.

use temper_tmpl::naming::{Case, NamingRules, assign, pascal_case, snake_case};
use temper_tmpl::{DeclId, DeclKind, text};

#[test]
fn snake_case_as_kotlin() {
    for (camel, snake) in [
        ("fib", "fib"),
        ("halfAndHalf", "half_and_half"),
        ("sayHello", "say_hello"),
        ("emptyString", "empty_string"),
        ("HTMLParser", "html_parser"),
        ("fooHTML_HTTPQux", "foo_html_http_qux"),
        ("foo123", "foo123"),
        ("a1b", "a1_b"),
        ("x2", "x2"),
        ("snake_case", "snake_case"),
        ("_private", "private"),
        ("όνομα", "όνομα"),
        ("Int32", "int32"),
        ("1st", "_1_st"),
    ] {
        assert_eq!(snake_case(camel), snake, "{camel}");
    }
}

/// As Kotlin's C# output names them (`DisplayP3_ToSrgbLinear` and
/// `A98_Rgb` are in Chromarite's).
#[test]
fn pascal_case_as_kotlin() {
    for (camel, pascal) in [
        ("fib", "Fib"),
        ("halfAndHalf", "HalfAndHalf"),
        ("sayHello", "SayHello"),
        ("HTMLParser", "HtmlParser"),
        ("fooHTML_HTTPQux", "FooHtmlHttpQux"),
        ("foo123", "Foo123"),
        ("a1b", "A1_B"),
        ("a98Rgb", "A98_Rgb"),
        ("displayP3ToSrgbLinear", "DisplayP3_ToSrgbLinear"),
        ("xyzD65", "XyzD65"),
        ("snake_case", "SnakeCase"),
        ("όνομα", "Όνομα"),
        ("Int32", "Int32"),
        ("1st", "_1_St"),
    ] {
        assert_eq!(pascal_case(camel), pascal, "{camel}");
    }
}

fn case(_: DeclKind, _: bool) -> Case {
    Case::Snake
}

fn identifier(c: char, first: bool) -> bool {
    c == '_' || c.is_alphabetic() || (!first && c.is_alphanumeric())
}

const RULES: NamingRules = NamingRules {
    reserved: &[&["if", "str", "min"]],
    case,
    private_prefix: "_",
    escape_suffix: "_",
    raw_keywords: &["type"],
    raw_prefix: "r#",
    identifier,
    block_scoped: false,
    same_type_shares: false,
};

const PROGRAM: &str = "tmpl 1 model 1 library t 0.0.0

module t#0 {
  fn f#1(a#2: Int32): Int32 {
    if (%Int32.gt(a#2, 0)) {
      let x#3: Int32 = %Int32.mul(a#2, 2);
    } else {
      let x#4: Int32 = %Int32.sub(0, a#2);
    }
    let x#5: Int32 = %Int32.add(a#2, 1);
    {
      let x#6: Int32 = 3;
      let str#7: Int32 = x#6;
    }
    let y#8: Int32 = x#5;
    ~loop#9: while (%Int32.gt(y#8, 0)) {
      let y#10: Int32 = 1;
      let min#11: Int32 = y#10;
    }
    let y#12: Int32 = %Int32.add(y#8, 1);
    return y#12;
  }

  export fn halfAndHalf#13(): Int32 {
    return 1;
  }

  init {
  }
}
";

#[test]
fn shares_and_suffixes_only_on_conflict() {
    let p = text::read(PROGRAM).unwrap_or_else(|e| panic!("{e}"));
    let names = assign(&p, &RULES);
    let name = |d: u32| names.get(DeclId(d));
    assert_eq!(name(1), "_f");
    assert_eq!(name(13), "half_and_half");
    assert_eq!(name(2), "a");
    // Lifetimes apart: one name.
    assert_eq!((name(3), name(4), name(5)), ("x", "x", "x"));
    // Alive while #5 is (read after its block): suffixed.
    assert_eq!(name(6), "x2");
    // Reserved: escaped.
    assert_eq!(name(7), "str_");
    assert_eq!(name(11), "min_");
    // #8 is read by the loop's next iteration, so the loop's own `y`
    // can't share its name; #12 starts after #8's last read.
    assert_eq!(name(8), "y");
    assert_eq!(name(10), "y2");
    assert_eq!(name(12), "y");
    // Labels aren't named.
    assert_eq!(name(9), "");
    assert_eq!(names.suffixed(), [DeclId(6), DeclId(10)]);
    assert!(names.clashes().is_empty());
}

#[test]
fn raw_keywords_take_the_prefix() {
    let p = text::read(
        "tmpl 1 model 1 library t 0.0.0

module t#0 {
  fn f#1(type#2: Int32, if#3: Int32): Int32 {
    return %Int32.add(type#2, if#3);
  }

  init {
  }
}
",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let names = assign(&p, &RULES);
    assert_eq!(names.get(DeclId(2)), "r#type");
    assert_eq!(names.get(DeclId(3)), "if_");
    assert!(names.suffixed().is_empty());
}

#[test]
fn block_scoped_shares_only_across_blocks_that_dont_nest() {
    let rules = NamingRules {
        block_scoped: true,
        same_type_shares: false,
        ..RULES
    };
    let p = text::read(PROGRAM).unwrap_or_else(|e| panic!("{e}"));
    let names = assign(&p, &rules);
    let name = |d: u32| names.get(DeclId(d));
    // The `if`'s two blocks don't nest: one name.
    assert_eq!((name(3), name(4)), ("x", "x"));
    // The function's own block holds both: a second name. The bare
    // block's is inside #5's block but apart from the `if`'s.
    assert_eq!((name(5), name(6)), ("x2", "x"));
    // The loop's block is inside the function's, so its `y` can't shadow
    // #8, and #12 is in #8's block.
    assert_eq!((name(8), name(10), name(12)), ("y", "y2", "y3"));
    assert_eq!(names.suffixed(), [DeclId(5), DeclId(10), DeclId(12)]);
}

#[test]
fn same_type_shares_only_with_or_without_null() {
    let rules = NamingRules {
        same_type_shares: true,
        ..RULES
    };
    let p = text::read(
        "tmpl 1 model 1 library t 0.0.0

uses `temper-core`#7 0.1.0 {
  class Int32#8 {
    connected method toString#9(radix?: Int32): String;
  }
  class String#11 {
    connected get isEmpty#10(): Boolean;
  }
}

module t#0 {
  fn f#1(x#2: Int32?): Int32 {
    let x#3: Int32 = (coalesce(x#2, 1)): Int32;
    let s#4: String = x#3.toString#9(omitted);
    let s#5: Boolean = s#4.isEmpty#10();
    let s#6: String = \"done\";
    return 0;
  }

  init {
  }
}
",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let names = assign(&p, &rules);
    let name = |d: u32| names.get(DeclId(d));
    // `Int32?` and `Int32` may share, as a default's local does.
    assert_eq!((name(2), name(3)), ("x", "x"));
    // Lifetimes apart, but a `Boolean` can't take a `String`'s name.
    assert_eq!((name(4), name(5), name(6)), ("s", "s2", "s"));
}
