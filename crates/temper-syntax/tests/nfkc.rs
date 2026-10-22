//! `nfkc::is_nfkc` against the JDK's answers in nfkc.txt, which
//! compat/gen_nfkc.py writes along with the tables.

use temper_syntax::nfkc::is_nfkc;

#[test]
fn matches_the_jdk() {
    let cases = include_str!("nfkc.txt");
    let mut wrong = Vec::new();
    let mut count = 0;
    for line in cases.lines().filter(|l| !l.starts_with('#')) {
        let (code_points, expected) = line.split_once('\t').unwrap();
        let text: String = code_points
            .split(' ')
            .map(|hex| char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap())
            .collect();
        count += 1;
        if is_nfkc(&text) != (expected == "yes") {
            wrong.push(line);
        }
    }
    assert!(count > 1000);
    assert!(wrong.is_empty(), "{} of {count} differ, as {:?}", wrong.len(), &wrong[..wrong.len().min(10)]);
}
