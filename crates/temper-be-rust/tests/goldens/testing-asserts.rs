use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let _name = Arc::new("case".to_string());
            Ok(())
        })
        .clone()
}

struct Vec2Struct {
    x: f64,
    y: f64,
}

#[derive(Clone)]
pub struct Vec2(Arc<Vec2Struct>);

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Vec2 {
        Vec2(Arc::new(Vec2Struct { x, y }))
    }

    pub fn to_string(&self) -> Arc<String> {
        Arc::new(format!(
            "({}, {}) size squared {}",
            temper_core::float64::to_string(self.0.x),
            temper_core::float64::to_string(self.0.y),
            temper_core::float64::to_string(norm_squared(self.clone()))
        ))
    }

    pub fn x(&self) -> f64 {
        self.0.x
    }

    pub fn y(&self) -> f64 {
        self.0.y
    }
}

impl std::fmt::Display for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

temper_core::impl_any_value_trait!(Vec2, []);

pub(crate) fn norm_squared(vec: Vec2) -> f64 {
    vec.0.x * vec.0.x + vec.0.y * vec.0.y
}

struct TestHelperStruct {
    i: i32,
    j: i32,
}

#[derive(Clone)]
pub(crate) struct TestHelper(Arc<TestHelperStruct>);

impl TestHelper {
    pub fn new(i: i32, j: i32) -> TestHelper {
        TestHelper(Arc::new(TestHelperStruct { i, j }))
    }
}

temper_core::impl_any_value_trait!(TestHelper, []);

pub(crate) fn assert_more_things(
    t: temper_std::testing::Test,
    helper: TestHelper,
    strings: impl temper_core::ToList<temper_core::List<Arc<String>>>,
) -> temper_core::Result<()> {
    let strings = strings.to_list();
    let mut i: i32 = 0;
    while i < temper_core::ListedTrait::len(&strings) {
        {
            let mut j: i32 = 0;
            while j < temper_core::ListedTrait::len(&temper_core::ListedTrait::get(&strings, i)) {
                {
                    let actual = helper.0.i;
                    let expected = helper.0.j;
                    t.assert(
                        actual == expected,
                        Arc::new(move || {
                            Arc::new(format!("expected helper.i == ({expected}) not ({actual})"))
                        }),
                    );
                }
                let s =
                    temper_core::ListedTrait::get(&temper_core::ListedTrait::get(&strings, i), j);
                {
                    let actual2 = s.clone();
                    let expected2 = Arc::new("hi".to_string());
                    t.assert(actual2 == expected2, {
                        let actual2 = actual2.clone();
                        let expected2 = expected2.clone();
                        Arc::new(move || {
                            Arc::new(format!("expected s == ({expected2}) not ({actual2})"))
                        })
                    });
                }
                j = j.wrapping_add(1);
            }
        }
        i = i.wrapping_add(1);
    }
    Ok(())
}

pub mod builders {
    use super::*;

    #[derive(Clone)]
    pub struct Vec2Builder {
        pub x: f64,
        pub y: f64,
    }

    impl Vec2Builder {
        pub fn build(self) -> Vec2 {
            Vec2::new(self.x, self.y)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    #[test]
    fn a_test_case() -> temper_core::Result<()> {
        crate::init(None)?;
        temper_std::init(None)?;
        let test = temper_std::testing::Test::new();
        test.assert(
            temper_core::float64::cmp(super::norm_squared(super::Vec2::new(0.0, 0.0)), 0.0) == 0,
            Arc::new(move || Arc::new("Oh noes".to_string())),
        );
        test.soft_fail_to_hard()
    }

    #[test]
    fn a_failing_test() -> temper_core::Result<()> {
        crate::init(None)?;
        temper_std::init(None)?;
        let test = temper_std::testing::Test::new();
        test.assert(
            false,
            Arc::new(move || Arc::new("because it's wrong".to_string())),
        );
        let helper = super::TestHelper::new(3, 2);
        super::assert_more_things(
            test.clone(),
            helper.clone(),
            Arc::new(vec![Arc::new(vec![Arc::new("bye".to_string())])]),
        )?;
        test.soft_fail_to_hard()
    }

    #[test]
    fn exported() -> temper_core::Result<()> {
        crate::init(None)?;
        temper_std::init(None)?;
        let test = temper_std::testing::Test::new();
        let vec = super::Vec2::new(1.5, 2.5);
        {
            let actual = vec.to_string();
            let expected = Arc::new("(1.5, 2.5) size squared 8.5".to_string());
            test.assert(actual == expected, {
                let actual = actual.clone();
                let expected = expected.clone();
                Arc::new(move || {
                    Arc::new(format!(
                        "expected vec.toString() == ({expected}) not ({actual})"
                    ))
                })
            });
        }
        test.soft_fail_to_hard()
    }
}
