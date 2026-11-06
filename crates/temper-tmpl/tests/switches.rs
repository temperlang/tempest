//! `Switch` as `if`s ([`temper_tmpl::switches`]), for targets without
//! it: labels become equalities, joined by `||`, the default the last
//! `else`, and a scrutinee that isn't a local is held in one first; the
//! result validates without the `SWITCH` feature.

use temper_tmpl::switches::to_if_chains;
use temper_tmpl::{Features, Profile, text, validate};

const PROGRAM: &str = "tmpl 1 model 1 library switched 0.1.0

module switched#0 {
  fn next#1(): Int32 {
    return 1;
  }

  fn name#2(n#3: Int32): String {
    switch (n#3) {
      case 0 {
        return \"zero\";
      }
      case 1, 2 {
        switch (next#1()) {
          case 1 {
            return \"one\";
          }
        }
        return \"few\";
      }
      default {
        return \"many\";
      }
    }
  }

  fn only#4(n#5: Int64): Void {
    switch (n#5) {
      default {
        return;
      }
    }
    switch (next#1()) {}
  }

  init {}
}
";

const LOWERED: &str = "tmpl 1 model 1 library switched 0.1.0

module switched#0 {
  fn next#1(): Int32 {
    return 1;
  }

  fn name#2(n#3: Int32): String {
    if (%Int32.eq(n#3, 0)) {
      return \"zero\";
    } else if (%Int32.eq(n#3, 1) || %Int32.eq(n#3, 2)) {
      {
        let ~switched#6: Int32 = next#1();
        if (%Int32.eq(~switched#6, 1)) {
          return \"one\";
        }
      }
      return \"few\";
    } else {
      return \"many\";
    }
  }

  fn only#4(n#5: Int64): Void {
    {
      return;
    }
    {
      let ~switched#7: Int32 = next#1();
    }
  }

  init {}
}
";

#[test]
fn switches_become_if_chains() {
    let program = text::read(PROGRAM).unwrap();
    assert_eq!(validate(&program, &Profile::full()), []);
    let without = Profile {
        native: Features::STATEMENT_EXPRESSIONS,
        ..Profile::full()
    };
    // S4: a profile without `SWITCH` doesn't allow one.
    assert!(
        validate(&program, &without)
            .iter()
            .any(|p| p.to_string().contains("doesn't allow Switch")),
    );
    let lowered = to_if_chains(&program).expect("switches to lower");
    assert_eq!(validate(&lowered, &without), []);
    assert_eq!(text::write(&lowered), LOWERED);
    // No switches, nothing new.
    assert!(to_if_chains(&lowered).is_none());
}
