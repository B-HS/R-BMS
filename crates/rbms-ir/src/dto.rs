//! Wire DTOs for the rbms IR-superset contract, grouped by the resource each one belongs to.

mod account;
mod chart;
mod query;
mod replay;
mod score;
mod settings;
mod system;

pub use account::*;
pub use chart::*;
pub use query::*;
pub use replay::*;
pub use score::*;
pub use settings::*;
pub use system::*;

pub(crate) fn current_api_version() -> u32 {
    crate::API_VERSION
}
