use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            {
                let mut i: i32 = 0;
                while i < 5 {
                    if i == 1 || i == 3 {
                        {
                            i = i.wrapping_add(1);
                            continue;
                        }
                    }
                    println!("{i}");
                    i = i.wrapping_add(1);
                }
            }
            {
                let mut i2: i32 = 0;
                while i2 < 5 {
                    if i2 == 3 {
                        break;
                    }
                    println!("{i2}");
                    i2 = i2.wrapping_add(1);
                }
            }
            let mut n: i32 = 0;
            {
                {
                    let mut i3: i32 = 0;
                    'outer: while i3 < 4 {
                        let mut str = Arc::new(format!("row {i3} ="));
                        let mut j: i32 = 0;
                        loop {
                            str = Arc::new(format!("{str} {}", {
                                let t = n;
                                n = t.wrapping_add(1);
                                t
                            }));
                            if i3 <= j {
                                println!("{str}");
                                {
                                    i3 = i3.wrapping_add(1);
                                    continue 'outer;
                                }
                            }
                            j = j.wrapping_add(1);
                        }
                    }
                }
            }
            let mut prime: i32 = 1;
            {
                {
                    let mut i4: i32 = 0;
                    'outer2: while i4 < 25 {
                        {
                            {
                                let mut j2: i32 = 0;
                                'outer3: while j2 < 25 {
                                    {
                                        {
                                            let mut k: i32 = 0;
                                            while k < 25 {
                                                prime = prime.wrapping_add(1);
                                                if prime != 2 && prime.wrapping_rem(2) == 0 {
                                                    {
                                                        i4 = i4.wrapping_add(1);
                                                        continue 'outer2;
                                                    }
                                                }
                                                if prime != 3 && prime.wrapping_rem(3) == 0 {
                                                    {
                                                        j2 = j2.wrapping_add(1);
                                                        continue 'outer3;
                                                    }
                                                }
                                                if prime != 5 && prime.wrapping_rem(5) == 0 {
                                                    {
                                                        k = k.wrapping_add(1);
                                                        continue;
                                                    }
                                                }
                                                if prime != 7 && prime.wrapping_rem(7) == 0 {
                                                    {
                                                        k = k.wrapping_add(1);
                                                        continue;
                                                    }
                                                }
                                                println!("{prime}");
                                                k = k.wrapping_add(1);
                                            }
                                        }
                                    }
                                    j2 = j2.wrapping_add(1);
                                }
                            }
                        }
                        i4 = i4.wrapping_add(1);
                    }
                }
            }
            let mut val: i32 = 0;
            {
                {
                    let mut i5: i32 = 0;
                    'outer4: while i5 < 10 {
                        let mut str2 = Arc::new(format!("row {i5} ="));
                        let mut j3: i32 = 0;
                        loop {
                            str2 = Arc::new(format!("{str2} {}", {
                                let t2 = val;
                                val = t2.wrapping_add(1);
                                t2
                            }));
                            if i5 == 4 {
                                println!("break");
                                break 'outer4;
                            }
                            if i5 <= j3 {
                                println!("{str2}");
                                {
                                    i5 = i5.wrapping_add(1);
                                    continue 'outer4;
                                }
                            }
                            j3 = j3.wrapping_add(1);
                        }
                    }
                }
            }
            Ok(())
        })
        .clone()
}
