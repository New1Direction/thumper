//! Registry model + persistence.
//! Real registration after absorb/generation (explicitly wired from CLI generate).

pub mod model;
pub mod store;

pub use store::register_generated;
