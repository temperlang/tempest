use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let v = temper_core::DenseBitVector::with_capacity(16);
            println!("v[2]={}", v.get(2));
            println!("v[3]={}", v.get(3));
            println!("v[3] <- true");
            v.set(3, true);
            println!("v[2]={}", v.get(2));
            println!("v[3]={}", v.get(3));
            println!("v[3] <- false");
            v.set(3, false);
            println!("v[2]={}", v.get(2));
            println!("v[3]={}", v.get(3));
            Ok(())
        })
        .clone()
}
