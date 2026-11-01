use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let entries: temper_core::ListBuilder<Entry> =
                temper_core::ListedTrait::to_list_builder(&Arc::new(vec![
                    Entry::new(0, 0),
                    Entry::new(1, 1),
                    Entry::new(2, 2),
                    Entry::new(3, 0),
                    Entry::new(4, 1),
                    Entry::new(5, 2),
                    Entry::new(6, 0),
                    Entry::new(7, 1),
                    Entry::new(8, 2),
                ]));
            let sorted_entries: temper_core::List<Entry> = temper_core::listed::sorted(
                &entries,
                &(|a: Entry, b: Entry| a.0.group.wrapping_sub(b.0.group)),
            );
            println!(
                "{}",
                temper_core::listed::join(
                    &sorted_entries,
                    Arc::new("\n".to_string()),
                    &(|a: Entry| a.to_string()),
                )
            );
            println!("{}", temper_core::ListedTrait::get(&entries, 1));
            temper_core::listed::sort(
                &entries,
                &(|a: Entry, b: Entry| a.0.group.wrapping_sub(b.0.group)),
            );
            println!(
                "{}",
                temper_core::listed::join(
                    &entries,
                    Arc::new("\n".to_string()),
                    &(|a: Entry| a.to_string()),
                )
            );
            munge_ints(Arc::new(vec![2, 1]));
            munge_floats(Arc::new(vec![3.4, 1.2]));
            Ok(())
        })
        .clone()
}

struct EntryStruct {
    id: i32,
    group: i32,
}

#[derive(Clone)]
pub(crate) struct Entry(Arc<EntryStruct>);

impl Entry {
    pub fn new(id: i32, group: i32) -> Entry {
        Entry(Arc::new(EntryStruct { id, group }))
    }

    pub fn to_string(&self) -> Arc<String> {
        Arc::new(format!("{{ id: {}, group: {} }}", self.0.id, self.0.group))
    }
}

impl std::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

temper_core::impl_any_value_trait!(Entry, []);

pub(crate) fn munge_ints(ints: impl temper_core::ToList<i32>) {
    let ints = ints.to_list();
    let builder: temper_core::ListBuilder<i32> = temper_core::ListedTrait::to_list_builder(&ints);
    temper_core::listed::sort(&builder, &(|a: i32, b: i32| a.wrapping_sub(b)));
    println!(
        "{}",
        temper_core::listed::join(
            &temper_core::listed::sorted(&ints, &(|a: i32, b: i32| a.wrapping_sub(b))),
            Arc::new(", ".to_string()),
            &(|a: i32| Arc::new(a.to_string())),
        )
    );
    println!(
        "{}",
        temper_core::listed::join(
            &builder,
            Arc::new(", ".to_string()),
            &(|a: i32| Arc::new(a.to_string())),
        )
    );
}

pub(crate) fn munge_floats(floats: impl temper_core::ToList<f64>) {
    let floats = floats.to_list();
    let builder: temper_core::ListBuilder<f64> = temper_core::ListedTrait::to_list_builder(&floats);
    let compare_floats = Arc::new(move |a: f64, b: f64| temper_core::float64::sign(a - b) as i32);
    temper_core::listed::sort(&builder, &*compare_floats);
    println!(
        "{}",
        temper_core::listed::join(
            &temper_core::listed::sorted(&floats, &*compare_floats),
            Arc::new(", ".to_string()),
            &(|a: f64| temper_core::float64::to_string(a)),
        )
    );
    println!(
        "{}",
        temper_core::listed::join(
            &builder,
            Arc::new(", ".to_string()),
            &(|a: f64| temper_core::float64::to_string(a)),
        )
    );
}
