use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            println!("both {}", both(2));
            let checked = renamed::Checked::new(renamed::lambda(renamed::native()));
            println!(
                "renamed {} {} {}",
                checked.delete(),
                checked.local(),
                checked.end()
            );
            let b = renamed::Builtins::new(4);
            println!(
                "builtins {} {} {} {} {} {} {}",
                b.hash(),
                b.get(1),
                b.set(3),
                b.map(5),
                b.format(),
                b.int(),
                b.later()
            );
            println!("{}", b.print());
            Ok(())
        })
        .clone()
}

pub fn both(n: i32) -> i32 {
    modules::sum(n)
        .wrapping_add(modules::helpers::twice(n))
        .wrapping_add(modules::helpers::base())
}
