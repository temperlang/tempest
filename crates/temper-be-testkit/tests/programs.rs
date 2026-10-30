//! The shared programs (`programs/`), checked against tempest's
//! interpreter.
//!
//! - `programs_log_as_the_interpreter_does`: each Temper program logs, on
//!   the interpreter, what its log block says, so every backend that runs
//!   it is checked against the interpreter.

use temper_be_testkit::{self as kit};

#[test]
fn programs_log_as_the_interpreter_does() {
    kit::with_stack(|| {
        for case in kit::cases("rust", &kit::PROGRAMS) {
            let [(path, text)] = &case.sources[..] else {
                continue;
            };
            if !path.ends_with(".temper.md") || path.contains("functional-tests") {
                continue;
            }
            let (failure, output) = kit::interpret(&case);
            assert_eq!(failure, None, "{}", case.key);
            let expected = temper_funtests::expected_log(text, "interp");
            assert_eq!(output, expected, "{}", case.key);
        }
    });
}
