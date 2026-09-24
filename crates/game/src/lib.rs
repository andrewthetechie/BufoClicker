//! BufoClicker game rules and state.
//!
//! This crate is pure Rust with no browser dependencies so the authoritative
//! game logic (state transitions, economy, save migration, catalog validation)
//! can run under native `cargo test`. The browser crate (`crates/web`) owns
//! Leptos views and browser adapters and calls into this crate for every rule.

/// The v2 save schema version written under the `bufo_idle_save_v2` key.
pub const SAVE_SCHEMA_VERSION: u32 = 2;

pub mod achievements;
pub mod boss;
pub mod catalog;
pub mod codec;
pub mod economy;
pub mod golden;
pub mod number;
pub mod prestige;
pub mod save;
pub mod state;
pub mod upgrades;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_is_2() {
        assert_eq!(SAVE_SCHEMA_VERSION, 2);
    }
}
