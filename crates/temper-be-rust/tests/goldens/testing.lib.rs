mod r#mod;
mod support;

pub use r#mod::*;

/// Initializes the library, once, with `config` or the default
/// configuration, and gives the runner of its async tasks.
pub fn init(config: Option<temper_core::Config>) -> temper_core::Result<temper_core::AsyncRunner> {
    support::CONFIG.get_or_init(|| config.unwrap_or_default());
    r#mod::init()?;
    Ok(support::config().runner().clone())
}
