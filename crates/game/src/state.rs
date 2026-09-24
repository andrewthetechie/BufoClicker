//! Authoritative game state and derived-value calculation.
//!
//! This is the single source of truth for the durable player state and for
//! every value derived from it (click power, production multipliers). The
//! browser crate holds one `GameState` and calls the rule functions; nothing
//! else computes these values, so they cannot drift.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::catalog::Catalogs;

/// Each lifetime Bufoplier point adds this much to the global multiplier.
pub const PRESTIGE_BONUS_PER_POINT: f64 = 0.10;
/// Minimum total bufos this run required before transcending is allowed.
pub const PRESTIGE_MIN_TOTAL_BUFOS: f64 = 1_000_000_000.0;
/// Scale factor for the prestige points curve: `sqrt(totalBufos / this)`.
pub const PRESTIGE_CURVE_DIVISOR: f64 = 1_000_000_000.0;
/// Permanent multiplier granted per defeated boss (additive: 1 + n·bonus).
pub const BOSS_BONUS_PER_DEFEAT: f64 = 0.25;

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resources {
    pub bufos: f64,
    pub total_bufos: f64,
    #[serde(default = "one")]
    pub base_click_power: f64,
    #[serde(default = "one")]
    pub click_multiplier: f64,
    #[serde(default = "one")]
    pub production_multiplier: f64,
    #[serde(default)]
    pub click_count: f64,
    /// Transient Golden Bufo "Bufo Frenzy" (1 = none). Never persisted.
    #[serde(default = "one")]
    pub frenzy_production_multiplier: f64,
    /// Transient Golden Bufo "Click Frenzy" (1 = none). Never persisted.
    #[serde(default = "one")]
    pub frenzy_click_multiplier: f64,
}

impl Default for Resources {
    fn default() -> Self {
        Resources {
            bufos: 0.0,
            total_bufos: 0.0,
            base_click_power: 1.0,
            click_multiplier: 1.0,
            production_multiplier: 1.0,
            click_count: 0.0,
            frenzy_production_multiplier: 1.0,
            frenzy_click_multiplier: 1.0,
        }
    }
}

/// Per-generator player state. Derived values (current production, total
/// production, current cost) are computed on demand, never stored, so they
/// cannot drift from the canonical `count` + `boosts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratorState {
    /// Owned count.
    #[serde(default)]
    pub count: f64,
    /// Whether this generator is currently visible/unlocked in the shop.
    #[serde(default)]
    pub unlocked: bool,
    /// Production boosts (from `generatorProduction` upgrades).
    #[serde(default)]
    pub boosts: Vec<Boost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Boost {
    pub id: String,
    #[serde(default)]
    pub multiplier: f64,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeState {
    #[serde(default)]
    pub purchased: Vec<String>,
    #[serde(default)]
    pub available: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementState {
    #[serde(default)]
    pub unlocked: Vec<String>,
    #[serde(default)]
    pub progress: HashMap<String, f64>,
    #[serde(default)]
    pub click_count: f64,
    #[serde(default)]
    pub custom_events: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSettings {
    #[serde(default)]
    pub last_saved: f64,
    #[serde(default)]
    pub last_tick: f64,
    #[serde(default = "default_true")]
    pub auto_save: bool,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub first_start_time: Option<f64>,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            last_saved: 0.0,
            last_tick: 0.0,
            auto_save: true,
            version: "1.0.0".to_string(),
            first_start_time: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrestigeState {
    #[serde(default)]
    pub points: f64,
    #[serde(default)]
    pub lifetime_points: f64,
    #[serde(default)]
    pub transcendences: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BossState {
    #[serde(default)]
    pub defeated: Vec<String>,
    #[serde(default)]
    pub lifetime_defeats: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub resources: Resources,
    /// Keyed by generator id (string, matching the catalog — no enum to drift).
    pub generators: HashMap<String, GeneratorState>,
    pub upgrades: UpgradeState,
    /// Older saves predate these slices; default them so migration succeeds.
    #[serde(default)]
    pub achievements: AchievementState,
    #[serde(default)]
    pub game_settings: GameSettings,
    #[serde(default)]
    pub prestige: PrestigeState,
    #[serde(default)]
    pub bosses: BossState,
}

fn default_true() -> bool {
    true
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl GameState {
    /// A fresh state: every generator at count 0, unlocked per its initial
    /// unlock requirements (a `bufos` requirement of 0 starts unlocked).
    pub fn default_state(catalogs: &Catalogs) -> Self {
        let mut generators = HashMap::new();
        for def in &catalogs.generators {
            let unlocked = initial_unlocked(def);
            generators.insert(
                def.id.clone(),
                GeneratorState {
                    count: 0.0,
                    unlocked,
                    boosts: Vec::new(),
                },
            );
        }
        GameState {
            resources: Resources::default(),
            generators,
            upgrades: UpgradeState::default(),
            achievements: AchievementState::default(),
            game_settings: GameSettings::default(),
            prestige: PrestigeState::default(),
            bosses: BossState::default(),
        }
    }

    pub fn generator_count(&self, id: &str) -> f64 {
        self.generators.get(id).map(|g| g.count).unwrap_or(0.0)
    }
}

pub fn initial_unlocked(def: &crate::catalog::GeneratorDef) -> bool {
    // A generator whose unlock requirements are all already satisfied at zero
    // progress starts unlocked (the tadpole has `bufos >= 0`).
    def.unlock_requirements
        .iter()
        .all(|req| match req.kind.as_str() {
            "bufos" => 0.0 >= req.value,
            _ => false,
        })
}

/// How many Bufoplier points a run worth `totalBufos` is worth right now.
pub fn prestige_points_for(total_bufos: f64) -> f64 {
    if !total_bufos.is_finite() || total_bufos < PRESTIGE_MIN_TOTAL_BUFOS {
        return 0.0;
    }
    (total_bufos / PRESTIGE_CURVE_DIVISOR).sqrt().floor()
}

/// Permanent global multiplier from prestige (never less than 1).
pub fn prestige_multiplier(state: &GameState) -> f64 {
    1.0 + state.prestige.lifetime_points.max(0.0) * PRESTIGE_BONUS_PER_POINT
}

/// Permanent multiplier from defeated bosses (never less than 1).
pub fn boss_multiplier(state: &GameState) -> f64 {
    1.0 + (state.bosses.defeated.len() as f64 + state.bosses.lifetime_defeats)
        * BOSS_BONUS_PER_DEFEAT
}

/// Global production multiplier: upgrade globals × prestige × boss × frenzy.
pub fn global_production_multiplier(state: &GameState) -> f64 {
    state.resources.production_multiplier
        * prestige_multiplier(state)
        * boss_multiplier(state)
        * state.resources.frenzy_production_multiplier
}

/// Current click power: base × click upgrades × prestige × boss × click frenzy.
pub fn click_power(state: &GameState) -> f64 {
    state.resources.base_click_power
        * state.resources.click_multiplier
        * prestige_multiplier(state)
        * boss_multiplier(state)
        * state.resources.frenzy_click_multiplier
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

    #[test]
    fn default_state_has_all_generators_zeroed() {
        let c = catalogs();
        let s = GameState::default_state(&c);
        assert_eq!(s.generators.len(), 14);
        assert_eq!(s.generator_count("tadpole"), 0.0);
        assert!(s.generators["tadpole"].unlocked);
        assert!(!s.generators["froglet"].unlocked);
    }

    #[test]
    fn prestige_points_curve() {
        assert_eq!(prestige_points_for(999_999_999.0), 0.0);
        assert_eq!(prestige_points_for(1_000_000_000.0), 1.0);
        assert_eq!(prestige_points_for(100_000_000_000.0), 10.0);
    }

    #[test]
    fn prestige_multiplier_uses_lifetime() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.prestige.lifetime_points = 5.0;
        assert!((prestige_multiplier(&s) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn boss_multiplier_adds_defeats_and_lifetime() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.bosses.defeated = vec!["a".into(), "b".into()];
        s.bosses.lifetime_defeats = 3.0;
        assert!((boss_multiplier(&s) - 2.25).abs() < 1e-9);
    }

    #[test]
    fn click_power_combines_all_sources() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.click_multiplier = 4.0;
        s.prestige.lifetime_points = 5.0; // 1.5x
        s.bosses.defeated = vec!["a".into()]; // 1.25x
                                              // 1 * 4 * 1.5 * 1.25 * 1 = 7.5
        assert!((click_power(&s) - 7.5).abs() < 1e-9);
    }
}
