use std::sync::{Arc, OnceLock};

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            let builder = Arc::new(std::sync::RwLock::new(String::new()));
            temper_core::string::builder::append(&builder, Arc::new("Hi ".to_string()));
            temper_core::string::builder::append_code_point(&builder, 127759)?;
            let source = Arc::new("Wow!! Neato!".to_string());
            let bang_index = temper_core::string::step(&source, 0usize, 3);
            let space_index = temper_core::string::step(&source, bang_index, 2);
            temper_core::string::builder::append_between(
                &builder,
                source.clone(),
                bang_index,
                space_index,
            );
            let built = temper_core::string::builder::to_string(&builder);
            temper_core::string::builder::append(&builder, Arc::new("\n".to_string()));
            temper_core::string::builder::append(&builder, built.clone());
            println!("{}", temper_core::string::builder::to_string(&builder));
            if temper_core::string::builder::append_code_point(&builder, 1114112).is_err() {
                println!("High code blocked.");
            }
            if temper_core::string::builder::append_code_point(&builder, 55296).is_err() {
                println!("Surrogate blocked.");
            }
            println!("{}", temper_core::string::builder::to_string(&builder));
            temper_core::string::builder::clear(&builder);
            temper_core::string::builder::append(
                &builder,
                Arc::new("Back to square zero.".to_string()),
            );
            println!("{}", temper_core::string::builder::to_string(&builder));
            let owner = StringBuilderOwner::new(Arc::new(std::sync::RwLock::new(String::new())));
            temper_core::string::builder::append(
                &owner.0.builder,
                Arc::new("Hi from inside.".to_string()),
            );
            println!(
                "{}",
                temper_core::string::builder::to_string(&owner.0.builder)
            );
            Ok(())
        })
        .clone()
}

struct StringBuilderOwnerStruct {
    builder: Arc<std::sync::RwLock<String>>,
}

#[derive(Clone)]
pub(crate) struct StringBuilderOwner(Arc<StringBuilderOwnerStruct>);

impl StringBuilderOwner {
    pub fn new(builder: Arc<std::sync::RwLock<String>>) -> StringBuilderOwner {
        StringBuilderOwner(Arc::new(StringBuilderOwnerStruct { builder }))
    }
}

temper_core::impl_any_value_trait!(StringBuilderOwner, []);
