pub mod errors;
#[macro_use]
mod macros;
pub mod pref_storage;

pub use crate::pref_storage::{Config, GSchema, GSchemaExt, XiConfig};
