//! Registry model + persistence.
//! Real registration after absorb/generation (explicitly wired from CLI generate).

pub mod keys;
pub mod model;
pub mod sqlite;
pub mod store;

pub use store::register_generated;
