//! Registry model + persistence.
//! Real registration after absorb/generation (explicitly wired from CLI generate).

pub mod model;
pub mod sqlite;
pub mod store;
pub mod keys;

pub use store::register_generated;

