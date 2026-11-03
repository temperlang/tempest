use std::sync::{Arc, OnceLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            println!("and32");
            and32(0, 0);
            and32(3, 1);
            and32(2, 3);
            and32(-2, 7);
            and32(-809434545, 901070595);
            println!("and64");
            and64(0, 0);
            and64(3, 1);
            and64(2, 3);
            and64(-2, 7);
            and64(297421889964898413, -3505189831424144594);
            println!("or32");
            or32(0, 0);
            or32(0, 1);
            or32(2, 1);
            or32(2, 2);
            or32(3, 1);
            or32(2, 3);
            or32(-2, 7);
            or32(-809434545, 901070595);
            println!("or64");
            or64(0, 0);
            or64(0, 1);
            or64(2, 1);
            or64(2, 2);
            or64(3, 1);
            or64(2, 3);
            or64(-2, 7);
            or64(297421889964898413, -3505189831424144594);
            println!("bComp32");
            b_comp32(0);
            b_comp32(1);
            b_comp32(-1);
            b_comp32(2);
            b_comp32(-2);
            b_comp32(-809434545);
            b_comp32(901070595);
            println!("bComp64");
            b_comp64(0);
            b_comp64(1);
            b_comp64(-1);
            b_comp64(2);
            b_comp64(-2);
            b_comp64(297421889964898413);
            b_comp64(-3505189831424144594);
            println!("shl32");
            shl32(5, 0);
            shl32(5, 1);
            shl32(5, 2);
            shl32(5, 3);
            shl32(5, 4);
            shl32(5, 5);
            shl32(5, 20);
            shl32(5, 28);
            shl32(5, 29);
            shl32(5, 30);
            shl32(5, 31);
            shl32(5, 32);
            shl32(5, 33);
            shl32(5, -1);
            shl32(5, -30);
            shl32(-1, 0);
            shl32(-1, 1);
            shl32(-1, 2);
            println!("shl64");
            shl64(5, 0);
            shl64(5, 1);
            shl64(5, 2);
            shl64(5, 3);
            shl64(5, 4);
            shl64(5, 5);
            shl64(5, 33);
            shl64(5, 62);
            shl64(5, 63);
            shl64(5, 64);
            shl64(5, -1);
            shl64(5, -60);
            shl64(-1, 0);
            shl64(-1, 1);
            shl64(-1, 2);
            println!("shr32");
            shr32(128, 0);
            shr32(128, 1);
            shr32(128, 2);
            shr32(128, 3);
            shr32(128, 4);
            shr32(128, 5);
            shr32(128, 6);
            shr32(128, 7);
            shr32(128, 8);
            shr32(128, 9);
            shr32(128, 31);
            shr32(128, 32);
            shr32(128, 33);
            shr32(128, -1);
            shr32(128, -30);
            shr32(-128, 0);
            shr32(-128, 1);
            shr32(-128, 2);
            shr32(-128, 3);
            shr32(-128, 4);
            println!("shr64");
            shr64(128, 0);
            shr64(128, 1);
            shr64(128, 2);
            shr64(128, 3);
            shr64(128, 4);
            shr64(128, 5);
            shr64(128, 6);
            shr64(128, 7);
            shr64(128, 8);
            shr64(128, 9);
            shr64(128, 32);
            shr64(128, 63);
            shr64(128, 64);
            shr64(128, 65);
            shr64(128, -1);
            shr64(128, -62);
            shr64(-128, 0);
            shr64(-128, 1);
            shr64(-128, 2);
            shr64(-128, 3);
            shr64(-128, 4);
            println!("uShr32");
            u_shr32(128, 0);
            u_shr32(128, 1);
            u_shr32(128, 2);
            u_shr32(128, 3);
            u_shr32(128, 4);
            u_shr32(128, 5);
            u_shr32(128, 6);
            u_shr32(128, 7);
            u_shr32(128, 8);
            u_shr32(128, 9);
            u_shr32(128, 31);
            u_shr32(128, 32);
            u_shr32(128, 33);
            u_shr32(128, -1);
            u_shr32(128, -30);
            u_shr32(-128, 0);
            u_shr32(-128, 1);
            u_shr32(-128, 2);
            u_shr32(-128, 3);
            u_shr32(-128, 4);
            println!("uShr64");
            u_shr64(128, 0);
            u_shr64(128, 1);
            u_shr64(128, 2);
            u_shr64(128, 3);
            u_shr64(128, 4);
            u_shr64(128, 5);
            u_shr64(128, 6);
            u_shr64(128, 7);
            u_shr64(128, 8);
            u_shr64(128, 9);
            u_shr64(128, 32);
            u_shr64(128, 63);
            u_shr64(128, 64);
            u_shr64(128, 65);
            u_shr64(128, -1);
            u_shr64(128, -62);
            u_shr64(-128, 0);
            u_shr64(-128, 1);
            u_shr64(-128, 2);
            u_shr64(-128, 3);
            u_shr64(-128, 4);
            Ok(())
        })
        .clone()
}

pub(crate) fn insert_underscores(hex_string: impl ToArcString) -> Arc<String> {
    let hex_string = hex_string.to_arc_string();
    let sb = Arc::new(std::sync::RwLock::new(String::new()));
    let mut idx: usize = 0usize;
    let end = hex_string.len();
    if (Some(idx).cmp(&Some(end)) as i32) < 0 {
        let possible_sign = temper_core::string::get(&hex_string, idx);
        if possible_sign == 43 || possible_sign == 45 {
            idx = temper_core::string::next(&hex_string, idx);
            temper_core::string::builder::append_between(&sb, hex_string.clone(), 0usize, idx);
        }
    }
    let n_first_group = temper_core::string::count_between(&hex_string, idx, end).wrapping_rem(4);
    let mut needs_underscore = false;
    if n_first_group != 0 {
        let group_start = idx;
        {
            let mut i: i32 = 0;
            while i < n_first_group {
                idx = temper_core::string::next(&hex_string, idx);
                i = i.wrapping_add(1);
            }
        }
        temper_core::string::builder::append_between(&sb, hex_string.clone(), group_start, idx);
        needs_underscore = true;
    }
    while (Some(idx).cmp(&Some(end)) as i32) < 0 {
        let group_start = idx;
        idx = temper_core::string::step(&hex_string, idx, 4);
        if needs_underscore {
            temper_core::string::builder::append(&sb, Arc::new("_".to_string()));
        }
        temper_core::string::builder::append_between(&sb, hex_string.clone(), group_start, idx);
        needs_underscore = true;
    }
    temper_core::string::builder::to_string(&sb)
}

pub(crate) fn fmt32(n: i32) -> Arc<String> {
    insert_underscores(temper_core::int_to_string(n, Some(16)))
}

pub(crate) fn fmt64(n: i64) -> Arc<String> {
    insert_underscores(temper_core::int64_to_string(n, Some(16)))
}

pub(crate) fn and32(a: i32, b: i32) {
    let c = a & b;
    println!("0x:{} & {} == {}", fmt32(a), fmt32(b), fmt32(c));
}

pub(crate) fn and64(a: i64, b: i64) {
    let c = a & b;
    println!("0x:{} & {} == {}", fmt64(a), fmt64(b), fmt64(c));
}

pub(crate) fn or32(a: i32, b: i32) {
    let c = a | b;
    println!("0x:{} | {} == {}", fmt32(a), fmt32(b), fmt32(c));
}

pub(crate) fn or64(a: i64, b: i64) {
    let c = a | b;
    println!("0x:{} | {} == {}", fmt64(a), fmt64(b), fmt64(c));
}

pub(crate) fn b_comp32(n: i32) {
    println!("0x:~{} == {}", fmt32(n), fmt32(!n));
}

pub(crate) fn b_comp64(n: i64) {
    println!("0x:~{} == {}", fmt64(n), fmt64(!n));
}

pub(crate) fn shl32(a: i32, b: i32) {
    let c = a.wrapping_shl(b as u32);
    println!("0x:{} << {b} == {}", fmt32(a), fmt32(c));
}

pub(crate) fn shl64(a: i64, b: i32) {
    let c = a.wrapping_shl(b as u32);
    println!("0x:{} << {b} == {}", fmt64(a), fmt64(c));
}

pub(crate) fn shr32(a: i32, b: i32) {
    let c = a.wrapping_shr(b as u32);
    println!("0x:{} >> {b} == {}", fmt32(a), fmt32(c));
}

pub(crate) fn shr64(a: i64, b: i32) {
    let c = a.wrapping_shr(b as u32);
    println!("0x:{} >> {b} == {}", fmt64(a), fmt64(c));
}

pub(crate) fn u_shr32(a: i32, b: i32) {
    let c = (a as u32).wrapping_shr(b as u32) as i32;
    println!("0x:{} >>> {b} == {}", fmt32(a), fmt32(c));
}

pub(crate) fn u_shr64(a: i64, b: i32) {
    let c = (a as u64).wrapping_shr(b as u32) as i64;
    println!("0x:{} >>> {b} == {}", fmt64(a), fmt64(c));
}
