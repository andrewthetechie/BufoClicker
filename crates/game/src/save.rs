//! Save schema v2, legacy-save migration, and state validation.
//!
//! The legacy save lives under `bufo_idle_save` and is a JSON envelope whose
//! nested `state` is authoritative. v2 lives under `bufo_idle_save_v2` with a
//! distinct `schema_version`. The legacy key is never written; migration
//! happens in memory and only a validated v2 save is ever written.

use serde::{Deserialize, Serialize};

use crate::catalog::Catalogs;
use crate::economy;
use crate::state::GameState;
use crate::{achievements, upgrades, SAVE_SCHEMA_VERSION};

/// The v2 envelope written under `bufo_idle_save_v2`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveV2 {
    pub schema_version: u32,
    pub timestamp: f64,
    pub state: GameState,
}

/// The legacy envelope under `bufo_idle_save`. Only `state` is authoritative;
/// the other fields are accepted and ignored.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyEnvelope {
    pub state: GameState,
}

/// Parse a legacy save envelope and return its authoritative `state`.
pub fn parse_legacy(json: &str) -> Result<GameState, String> {
    let envelope: LegacyEnvelope =
        serde_json::from_str(json).map_err(|e| format!("invalid legacy save: {e}"))?;
    Ok(envelope.state)
}

/// Parse a v2 save. Returns an error for a wrong schema version or bad JSON.
pub fn parse_v2(json: &str) -> Result<SaveV2, String> {
    let save: SaveV2 = serde_json::from_str(json).map_err(|e| format!("invalid v2 save: {e}"))?;
    if save.schema_version != SAVE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema_version {} (expected {SAVE_SCHEMA_VERSION})",
            save.schema_version
        ));
    }
    Ok(save)
}

/// The result of deciding what to load at startup.
#[derive(Debug, Clone)]
pub enum LoadDecision {
    /// A valid v2 save takes precedence.
    V2(GameState),
    /// No v2 save; a legacy save must be migrated.
    MigrateLegacy(GameState),
    /// Neither key exists; start fresh.
    Fresh,
}

/// Decide what to load: a valid v2 save wins, then a legacy save, then fresh.
/// A corrupt v2 save is an error (the caller shows recovery, it never silently
/// falls back to the legacy key).
pub fn decide_load(
    v2_json: Option<&str>,
    legacy_json: Option<&str>,
) -> Result<LoadDecision, String> {
    if let Some(v2) = v2_json {
        let save = parse_v2(v2)?;
        return Ok(LoadDecision::V2(save.state));
    }
    if let Some(legacy) = legacy_json {
        let state = parse_legacy(legacy)?;
        return Ok(LoadDecision::MigrateLegacy(state));
    }
    Ok(LoadDecision::Fresh)
}

/// Parse a decoded import string as either v2 or legacy, selected by the
/// presence of `schemaVersion` (the v2 envelope field).
pub fn parse_any(json: &str) -> Result<GameState, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("invalid import JSON: {e}"))?;
    if value.get("schemaVersion").is_some() {
        parse_v2(json).map(|s| s.state)
    } else {
        parse_legacy(json)
    }
}

// ---------------------------------------------------------------------------
// Storage-backed load / save / reset / import
// ---------------------------------------------------------------------------

pub const LEGACY_KEY: &str = "bufo_idle_save";
pub const V2_KEY: &str = "bufo_idle_save_v2";

/// A key-value store the save logic reads from and writes to. The browser
/// crate implements this with `localStorage`; native tests use an in-memory
/// map so reset ordering and failed-write atomicity are testable.
pub trait Storage {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&mut self, key: &str, value: &str) -> Result<(), String>;
}

/// What happened at startup.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum LoadOutcome {
    Ready(GameState),
    /// A corrupt v2 save or failed migration: writes are blocked until the
    /// player picks a recovery action.
    Recovery(String),
}

/// Read and decide what to load. A valid v2 save wins; otherwise the legacy
/// save is migrated; otherwise a fresh state. A corrupt v2 save is a recovery
/// state, never a silent fallback to the legacy key. Credits closed-browser
/// offline production (one-minute floor, 12h cap) from the saved last-tick.
pub fn load_game(storage: &dyn Storage, catalogs: &Catalogs, now: f64) -> LoadOutcome {
    let v2 = storage.get(V2_KEY);
    let legacy = storage.get(LEGACY_KEY);
    match decide_load(v2.as_deref(), legacy.as_deref()) {
        Ok(decision) => {
            let mut state = match decision {
                LoadDecision::V2(s) | LoadDecision::MigrateLegacy(s) => s,
                LoadDecision::Fresh => GameState::default_state(catalogs),
            };
            migrate(&mut state, catalogs);
            if state.game_settings.first_start_time.is_none() {
                state.game_settings.first_start_time = Some(now);
            }
            let last_tick = state.game_settings.last_tick;
            if last_tick > 0.0 {
                economy::apply_elapsed_production(&mut state, catalogs, last_tick, now, 60_000.0);
            }
            let errors = validate(&state, catalogs);
            if errors.is_empty() {
                LoadOutcome::Ready(state)
            } else {
                LoadOutcome::Recovery(errors.join("; "))
            }
        }
        Err(e) => LoadOutcome::Recovery(e),
    }
}

/// Validate and write a v2 save. Returns an error (without writing) if the
/// state is invalid or the write fails.
pub fn save_v2(
    storage: &mut dyn Storage,
    state: &GameState,
    catalogs: &Catalogs,
    now: f64,
) -> Result<(), String> {
    let errors = validate(state, catalogs);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let json = serialize_v2(state, now);
    storage.set(V2_KEY, &json)
}

/// Reset: validate + serialize a fresh state, write it with one `set`, and
/// only then return the fresh state to commit. A failed write leaves both
/// stored keys untouched and returns an error (the caller must NOT commit).
pub fn reset(
    storage: &mut dyn Storage,
    catalogs: &Catalogs,
    now: f64,
) -> Result<GameState, String> {
    let fresh = GameState::default_state(catalogs);
    let errors = validate(&fresh, catalogs);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let json = serialize_v2(&fresh, now);
    storage.set(V2_KEY, &json)?;
    Ok(fresh)
}

/// Import a decoded save (legacy or v2). Parses, migrates, validates, writes
/// v2 once, then returns the state to commit. Any failure leaves both stored
/// keys unchanged.
pub fn import_save(
    storage: &mut dyn Storage,
    decoded_json: &str,
    catalogs: &Catalogs,
    now: f64,
) -> Result<GameState, String> {
    let mut state = parse_any(decoded_json)?;
    migrate(&mut state, catalogs);
    let errors = validate(&state, catalogs);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let json = serialize_v2(&state, now);
    storage.set(V2_KEY, &json)?;
    Ok(state)
}

/// Recovery action: explicitly restore from the legacy `bufo_idle_save` key
/// even though a corrupt v2 exists. Migrates and writes v2 once, then returns
/// the state to commit. Fails if there is no legacy save.
pub fn restore_legacy(
    storage: &mut dyn Storage,
    catalogs: &Catalogs,
    now: f64,
) -> Result<GameState, String> {
    let legacy = storage
        .get(LEGACY_KEY)
        .ok_or_else(|| "no legacy save to restore".to_string())?;
    let mut state = parse_legacy(&legacy)?;
    migrate(&mut state, catalogs);
    let errors = validate(&state, catalogs);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let json = serialize_v2(&state, now);
    storage.set(V2_KEY, &json)?;
    Ok(state)
}

/// Serialize the current state as a v2 envelope.
pub fn serialize_v2(state: &GameState, timestamp: f64) -> String {
    let save = SaveV2 {
        schema_version: SAVE_SCHEMA_VERSION,
        timestamp,
        state: state.clone(),
    };
    serde_json::to_string(&save).expect("GameState serialization cannot fail")
}

/// Migrate a freshly-parsed legacy (or v2) state: reset the upgrade/achievement
/// multipliers, re-apply purchased upgrades and unlocked achievement rewards
/// exactly once, and recompute unlocks. This mirrors the TS load path.
pub fn migrate(state: &mut GameState, catalogs: &Catalogs) {
    state.resources.frenzy_production_multiplier = 1.0;
    state.resources.frenzy_click_multiplier = 1.0;
    upgrades::reapply_all_upgrades(state, catalogs);
    achievements::reapply_all_achievement_rewards(state, catalogs);
    economy::check_unlocks(state, catalogs);
}

/// Validate a state before it may be written or committed. Returns every
/// problem found (empty = valid). Rejects non-finite values and impossible
/// state (negative counters, unknown purchased upgrade ids).
pub fn validate(state: &GameState, catalogs: &Catalogs) -> Vec<String> {
    let mut errors = Vec::new();

    for (name, v) in [
        ("bufos", state.resources.bufos),
        ("totalBufos", state.resources.total_bufos),
        ("clickMultiplier", state.resources.click_multiplier),
        (
            "productionMultiplier",
            state.resources.production_multiplier,
        ),
        ("baseClickPower", state.resources.base_click_power),
    ] {
        if !v.is_finite() {
            errors.push(format!("resources.{name} is not finite"));
        }
    }
    if state.resources.bufos < 0.0 || state.resources.total_bufos < 0.0 {
        errors.push("negative bufo counters".to_string());
    }

    for (id, gen) in &state.generators {
        if !gen.count.is_finite() || gen.count < 0.0 {
            errors.push(format!("generator '{id}' has an invalid count"));
        }
    }

    // Purchased upgrades must exist in the catalog.
    let known_upgrades: std::collections::HashSet<&str> =
        catalogs.upgrades.iter().map(|u| u.id.as_str()).collect();
    for id in &state.upgrades.purchased {
        if !known_upgrades.contains(id.as_str()) {
            errors.push(format!("purchased upgrade '{id}' is not in the catalog"));
        }
    }

    if !state.prestige.points.is_finite()
        || !state.prestige.lifetime_points.is_finite()
        || !state.prestige.transcendences.is_finite()
        || state.prestige.lifetime_points < 0.0
    {
        errors.push("invalid prestige state".to_string());
    }
    if !state.bosses.lifetime_defeats.is_finite() || state.bosses.lifetime_defeats < 0.0 {
        errors.push("invalid boss state".to_string());
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalogs;

    fn catalogs() -> Catalogs {
        Catalogs::from_json(
            include_str!("../../../assets/data/generators.json"),
            include_str!("../../../assets/data/upgrades.json"),
            include_str!("../../../assets/data/achievements.json"),
        )
        .unwrap()
    }

    fn fixture(name: &str) -> String {
        let content = match name {
            "fresh.json" => include_str!("../../../tests/fixtures/fresh.json"),
            "purchased.json" => include_str!("../../../tests/fixtures/purchased.json"),
            "upgraded.json" => include_str!("../../../tests/fixtures/upgraded.json"),
            "achievement.json" => include_str!("../../../tests/fixtures/achievement.json"),
            "prestige.json" => include_str!("../../../tests/fixtures/prestige.json"),
            "boss.json" => include_str!("../../../tests/fixtures/boss.json"),
            "missing-fields.json" => include_str!("../../../tests/fixtures/missing-fields.json"),
            "malformed.json" => include_str!("../../../tests/fixtures/malformed.json"),
            "corrupt.json" => include_str!("../../../tests/fixtures/corrupt.json"),
            other => panic!("unknown fixture {other}"),
        };
        content.to_string()
    }

    #[test]
    fn legacy_fixtures_parse_and_validate() {
        let c = catalogs();
        for name in [
            "fresh.json",
            "purchased.json",
            "upgraded.json",
            "achievement.json",
            "prestige.json",
            "boss.json",
        ] {
            let mut state = parse_legacy(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
            migrate(&mut state, &c);
            let errors = validate(&state, &c);
            assert!(errors.is_empty(), "{name}: {errors:?}");
        }
    }

    #[test]
    fn missing_fields_fixture_migrates_with_defaults() {
        let c = catalogs();
        let mut state = parse_legacy(&fixture("missing-fields.json")).unwrap();
        migrate(&mut state, &c);
        assert!(state.achievements.unlocked.is_empty());
        assert_eq!(state.prestige.lifetime_points, 0.0);
        assert!(validate(&state, &c).is_empty());
    }

    #[test]
    fn malformed_and_corrupt_are_rejected() {
        assert!(parse_legacy(&fixture("malformed.json")).is_err());
        assert!(parse_legacy(&fixture("corrupt.json")).is_err());
    }

    #[test]
    fn migration_reapplies_upgrade_and_achievement_effects_once() {
        let c = catalogs();
        // upgraded.json has stronger_clicks_1+2 (2x, 2x) and global_production_1 (2x).
        let mut state = parse_legacy(&fixture("upgraded.json")).unwrap();
        migrate(&mut state, &c);
        assert!((state.resources.click_multiplier - 4.0).abs() < 1e-9);
        // global_production_1 is 1.5x.
        assert!((state.resources.production_multiplier - 1.5).abs() < 1e-9);
        // Re-running must not double-apply.
        migrate(&mut state, &c);
        assert!((state.resources.click_multiplier - 4.0).abs() < 1e-9);
    }

    #[test]
    fn v2_roundtrip_preserves_canonical_fields() {
        let c = catalogs();
        let mut state = parse_legacy(&fixture("boss.json")).unwrap();
        migrate(&mut state, &c);
        let json = serialize_v2(&state, 1_700_000_000_000.0);
        let back = parse_v2(&json).unwrap();
        assert_eq!(back.state.resources.bufos, state.resources.bufos);
        assert_eq!(
            back.state.resources.total_bufos,
            state.resources.total_bufos
        );
        assert_eq!(back.state.bosses.defeated, state.bosses.defeated);
        assert_eq!(
            back.state.bosses.lifetime_defeats,
            state.bosses.lifetime_defeats
        );
        assert_eq!(back.state.upgrades.purchased, state.upgrades.purchased);
        assert_eq!(
            back.state.generator_count("tadpole"),
            state.generator_count("tadpole")
        );
    }

    #[test]
    fn wrong_schema_version_is_rejected() {
        let json = r#"{"schemaVersion": 99, "timestamp": 1, "state": {}}"#;
        assert!(parse_v2(json).is_err());
    }

    #[test]
    fn v2_takes_precedence_over_legacy() {
        let legacy = parse_legacy(&fixture("purchased.json")).unwrap();
        let v2_json = serialize_v2(&legacy, 1_700_000_000_000.0);
        let decision = decide_load(Some(&v2_json), Some(&fixture("boss.json"))).unwrap();
        assert!(matches!(decision, LoadDecision::V2(_)));
    }

    #[test]
    fn legacy_falls_back_when_no_v2() {
        let decision = decide_load(None, Some(&fixture("boss.json"))).unwrap();
        assert!(matches!(decision, LoadDecision::MigrateLegacy(_)));
    }

    #[test]
    fn corrupt_v2_is_an_error_not_a_fallback() {
        let decision = decide_load(Some("not json"), Some(&fixture("boss.json")));
        assert!(decision.is_err());
    }

    #[test]
    fn fresh_when_both_absent() {
        let decision = decide_load(None, None).unwrap();
        assert!(matches!(decision, LoadDecision::Fresh));
    }

    #[test]
    fn parse_any_selects_v2_and_legacy() {
        let legacy = parse_legacy(&fixture("purchased.json")).unwrap();
        let v2_json = serialize_v2(&legacy, 1_700_000_000_000.0);
        // v2 JSON has schemaVersion -> parsed as v2.
        let parsed = parse_any(&v2_json).unwrap();
        assert_eq!(parsed.resources.bufos, legacy.resources.bufos);
        // Legacy JSON has no schemaVersion -> parsed as legacy.
        let parsed = parse_any(&fixture("purchased.json")).unwrap();
        assert_eq!(parsed.resources.bufos, legacy.resources.bufos);
    }

    // --- Storage-backed logic -------------------------------------------------

    struct MockStorage {
        map: std::collections::HashMap<String, String>,
        fail_writes: bool,
    }

    impl MockStorage {
        fn new() -> Self {
            MockStorage {
                map: std::collections::HashMap::new(),
                fail_writes: false,
            }
        }
    }

    impl Storage for MockStorage {
        fn get(&self, key: &str) -> Option<String> {
            self.map.get(key).cloned()
        }
        fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
            if self.fail_writes {
                return Err("denied".to_string());
            }
            self.map.insert(key.to_string(), value.to_string());
            Ok(())
        }
    }

    #[test]
    fn reset_writes_v2_and_leaves_legacy_untouched() {
        let c = catalogs();
        let mut storage = MockStorage::new();
        storage
            .map
            .insert(LEGACY_KEY.to_string(), fixture("purchased.json"));
        let fresh = reset(&mut storage, &c, 1_700_000_000_000.0).expect("reset succeeds");
        assert_eq!(fresh.generator_count("tadpole"), 0.0);
        assert!(storage.get(V2_KEY).is_some());
        // Legacy key is read-only and untouched.
        assert_eq!(storage.get(LEGACY_KEY).unwrap(), fixture("purchased.json"));
    }

    #[test]
    fn failed_reset_write_leaves_both_keys_unchanged() {
        let c = catalogs();
        let mut storage = MockStorage::new();
        storage
            .map
            .insert(LEGACY_KEY.to_string(), fixture("purchased.json"));
        storage.map.insert(
            V2_KEY.to_string(),
            "{\"schemaVersion\":2,\"timestamp\":1,\"state\":{}}".to_string(),
        );
        storage.fail_writes = true;
        let result = reset(&mut storage, &c, 1_700_000_000_000.0);
        assert!(result.is_err());
        // Both keys unchanged after the failed write.
        assert_eq!(storage.get(LEGACY_KEY).unwrap(), fixture("purchased.json"));
        assert_eq!(
            storage.get(V2_KEY).unwrap(),
            "{\"schemaVersion\":2,\"timestamp\":1,\"state\":{}}"
        );
    }

    #[test]
    fn failed_import_leaves_both_keys_unchanged() {
        let c = catalogs();
        let mut storage = MockStorage::new();
        storage
            .map
            .insert(LEGACY_KEY.to_string(), fixture("purchased.json"));
        let before_v2 = fixture("boss.json");
        storage.map.insert(V2_KEY.to_string(), before_v2.clone());
        let result = import_save(&mut storage, "not json", &c, 1_700_000_000_000.0);
        assert!(result.is_err());
        assert_eq!(storage.get(LEGACY_KEY).unwrap(), fixture("purchased.json"));
        assert_eq!(storage.get(V2_KEY).unwrap(), before_v2);
    }

    #[test]
    fn successful_import_writes_v2_once() {
        let c = catalogs();
        let mut storage = MockStorage::new();
        let decoded = fixture("purchased.json");
        let state = import_save(&mut storage, &decoded, &c, 1_700_000_000_000.0).unwrap();
        assert!(state.generator_count("tadpole") > 0.0);
        let v2 = storage.get(V2_KEY).expect("v2 written");
        assert!(v2.contains("schemaVersion"));
    }

    #[test]
    fn save_v2_rejects_invalid_state_without_writing() {
        let c = catalogs();
        let mut storage = MockStorage::new();
        let mut state = GameState::default_state(&c);
        state.resources.bufos = f64::NAN;
        let result = save_v2(&mut storage, &state, &c, 1_700_000_000_000.0);
        assert!(result.is_err());
        assert!(storage.get(V2_KEY).is_none());
    }

    #[test]
    fn restore_legacy_migrates_and_writes_v2() {
        let c = catalogs();
        let mut storage = MockStorage::new();
        storage
            .map
            .insert(LEGACY_KEY.to_string(), fixture("purchased.json"));
        let state = restore_legacy(&mut storage, &c, 1_700_000_000_000.0).unwrap();
        assert!(state.generator_count("tadpole") > 0.0);
        assert!(storage.get(V2_KEY).is_some());
        assert_eq!(storage.get(LEGACY_KEY).unwrap(), fixture("purchased.json"));
    }

    #[test]
    fn restore_legacy_fails_without_legacy_save() {
        let c = catalogs();
        let mut storage = MockStorage::new();
        let result = restore_legacy(&mut storage, &c, 1_700_000_000_000.0);
        assert!(result.is_err());
        assert!(storage.get(V2_KEY).is_none());
    }
}
