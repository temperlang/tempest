use std::sync::OnceLock;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            {
                let mut nci: i32 = 1;
                println!("nci = {nci}");
                {
                    nci = 2;
                };
                println!("nci = {nci}");
                let mut nc: i32 = 1;
                println!("nc = {nc}");
                {
                    nc = 2;
                };
                println!("nc = {nc}");
                {
                    let mut i: i32 = 0;
                    while i < 3 {
                        let ncl: i32;
                        {
                            ncl = i;
                        };
                        println!("ncl = {ncl}");
                        i = i.wrapping_add(1);
                    }
                }
                let ci: i32 = 1;
                println!("ci = {ci}");
                println!("failed to set ci");
                println!("ci = {ci}");
                let c: i32 = 1;
                println!("c = {c}");
                println!("failed to set c");
                println!("c = {c}");
                {
                    let mut i2: i32 = 0;
                    while i2 < 3 {
                        let cl: i32;
                        {
                            cl = i2;
                        };
                        println!("cl = {cl}");
                        i2 = i2.wrapping_add(1);
                    }
                }
            }
            Ok(())
        })
        .clone()
}
