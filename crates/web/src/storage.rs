//! Browser storage adapter: a thin `localStorage` implementation of the game
//! crate's `Storage` trait, plus the existing export/import codec
//! (`btoa(encodeURIComponent(JSON))`). All save/reset/import logic lives in
//! `crates/game/src/save.rs` and is natively tested against an in-memory mock.

use game::catalog::Catalogs;
use game::save::{self, Storage};
use game::state::GameState;

pub use game::save::LoadOutcome;

struct LocalStorage {
    inner: web_sys::Storage,
}

impl Storage for LocalStorage {
    fn get(&self, key: &str) -> Option<String> {
        self.inner.get_item(key).ok().flatten()
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        self.inner
            .set_item(key, value)
            .map_err(|e| format!("setItem failed: {e:?}"))
    }
}

fn storage() -> Option<LocalStorage> {
    let window = web_sys::window()?;
    let inner = window.local_storage().ok().flatten()?;
    Some(LocalStorage { inner })
}

/// Read and decide what to load (v2 precedence, legacy migration, recovery).
pub fn load_game(catalogs: &Catalogs) -> LoadOutcome {
    match storage() {
        Some(s) => save::load_game(&s, catalogs, js_sys::Date::now()),
        None => LoadOutcome::Recovery("localStorage unavailable".to_string()),
    }
}

/// Validate and write a v2 save.
pub fn save_v2(state: &GameState, catalogs: &Catalogs) -> Result<(), String> {
    let mut s = storage().ok_or_else(|| "localStorage unavailable".to_string())?;
    save::save_v2(&mut s, state, catalogs, js_sys::Date::now())
}

/// Reset: validate + serialize + write a fresh v2 state before committing.
pub fn reset(catalogs: &Catalogs) -> Result<GameState, String> {
    let mut s = storage().ok_or_else(|| "localStorage unavailable".to_string())?;
    save::reset(&mut s, catalogs, js_sys::Date::now())
}

/// Recovery action: explicitly restore from the legacy key (even when a
/// corrupt v2 exists). Migrates and writes v2 once.
pub fn restore_legacy(catalogs: &Catalogs) -> Result<GameState, String> {
    let mut s = storage().ok_or_else(|| "localStorage unavailable".to_string())?;
    save::restore_legacy(&mut s, catalogs, js_sys::Date::now())
}

/// Export the current state as the existing codec:
/// `btoa(encodeURIComponent(JSON.stringify(v2)))` (pure-Rust, tested in the
/// game crate).
pub fn export_save(state: &GameState, catalogs: &Catalogs) -> Result<String, String> {
    let errors = save::validate(state, catalogs);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let json = save::serialize_v2(state, js_sys::Date::now());
    Ok(game::codec::encode_export(&json))
}

/// Import an encoded save (legacy or v2). Decodes the codec, then hands the
/// JSON to the game crate, which parses, migrates, validates, writes v2 once,
/// and returns the state to commit. Any failure leaves both keys unchanged.
pub fn import_save(encoded: &str, catalogs: &Catalogs) -> Result<GameState, String> {
    let json = game::codec::decode_export(encoded)?;
    let mut s = storage().ok_or_else(|| "localStorage unavailable".to_string())?;
    save::import_save(&mut s, &json, catalogs, js_sys::Date::now())
}
