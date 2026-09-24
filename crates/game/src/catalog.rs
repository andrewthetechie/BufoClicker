//! Game content catalogs: typed parsing and validation of the three runtime
//! JSON files (`generators.json`, `upgrades.json`, `achievements.json`).
//!
//! The browser fetches these at runtime and passes the raw bytes here. The
//! game must not start (and must not write a save) until all three parse and
//! validate. Validation checks finite numbers, known effect/condition types,
//! and cross-catalog ID references.

use serde::Deserialize;

/// One validation problem, human-readable for the retry screen / logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogError {
    pub message: String,
}

impl CatalogError {
    fn new(message: impl Into<String>) -> Self {
        CatalogError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

// ---------------------------------------------------------------------------
// Generator catalog (a JSON object keyed by generator id)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratorDef {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub detailed_description: Option<String>,
    #[serde(default)]
    pub icon_path: Option<String>,
    #[serde(default)]
    pub category: String,
    /// Initial owned count (0 in the shipped catalog).
    #[serde(default)]
    pub count: f64,
    pub base_production: f64,
    #[serde(default)]
    pub current_production: f64,
    #[serde(default)]
    pub total_production: f64,
    pub base_cost: f64,
    #[serde(default)]
    pub current_cost: f64,
    pub cost_multiplier: f64,
    #[serde(default)]
    pub unlock_requirements: Vec<UnlockRequirement>,
    #[serde(default)]
    pub unlocked: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub boosts: Vec<ProductionBoost>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockRequirement {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionBoost {
    pub id: String,
    #[serde(default)]
    pub multiplier: f64,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub source: String,
}

// ---------------------------------------------------------------------------
// Upgrade catalog (a JSON array)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeDef {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub flavor_text: Option<String>,
    #[serde(default)]
    pub category: String,
    pub cost: f64,
    #[serde(default)]
    pub icon_path: Option<String>,
    #[serde(default)]
    pub effects: Vec<UpgradeEffect>,
    #[serde(default)]
    pub unlock_conditions: Vec<UnlockCondition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeEffect {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub multiplier: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockCondition {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
}

// ---------------------------------------------------------------------------
// Achievement catalog (a JSON array)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementDef {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub flavor_text: Option<String>,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub order: f64,
    pub requirement: AchievementRequirement,
    #[serde(default)]
    pub reward: Option<AchievementReward>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementRequirement {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementReward {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// The parsed, validated catalog set
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Catalogs {
    /// Generators in catalog order (the JSON object preserves insertion order
    /// only via a map; we keep a Vec plus an index for deterministic lookup).
    pub generators: Vec<GeneratorDef>,
    pub upgrades: Vec<UpgradeDef>,
    pub achievements: Vec<AchievementDef>,
}

fn default_true() -> bool {
    true
}

/// Known upgrade effect types that actually do something. Anything else must
/// be rejected so an unsupported effect cannot take currency for nothing.
pub const UPGRADE_EFFECT_TYPES: &[&str] =
    &["clickMultiplier", "generatorProduction", "globalMultiplier"];

/// Known upgrade unlock-condition types.
pub const UNLOCK_CONDITION_TYPES: &[&str] = &["generatorCount", "totalBufos", "upgrade"];

/// Known achievement requirement types.
pub const ACHIEVEMENT_REQUIREMENT_TYPES: &[&str] = &[
    "bossesDefeated",
    "bufosPerSecond",
    "clickCount",
    "consoleOpened",
    "customEvent",
    "generatorType",
    "prestigePoints",
    "totalBufos",
    "totalGenerators",
    "transcendences",
    "upgradeCount",
];

/// Known achievement reward types.
pub const ACHIEVEMENT_REWARD_TYPES: &[&str] = &[
    "bufoBonus",
    "clickBoost",
    "generatorBoost",
    "productionBoost",
];

/// Known generator unlock-requirement types.
pub const GENERATOR_UNLOCK_TYPES: &[&str] = &["bufos", "generators"];

impl Catalogs {
    /// Parse and validate all three catalogs from raw JSON text.
    pub fn from_json(
        generators_json: &str,
        upgrades_json: &str,
        achievements_json: &str,
    ) -> Result<Self, Vec<CatalogError>> {
        let mut generators: Vec<GeneratorDef> =
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(generators_json)
                .map_err(|e| {
                    vec![CatalogError::new(format!(
                        "generators.json: invalid JSON: {e}"
                    ))]
                })?
                .into_values()
                .map(serde_json::from_value::<GeneratorDef>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| vec![CatalogError::new(format!("generators.json: {e}"))])?;
        // serde_json's map order is alphabetical by default, while the shop
        // presents generators in progression order, cheapest first.
        generators.sort_by(|a, b| a.base_cost.total_cmp(&b.base_cost));

        let upgrades: Vec<UpgradeDef> = serde_json::from_str(upgrades_json).map_err(|e| {
            vec![CatalogError::new(format!(
                "upgrades.json: invalid JSON: {e}"
            ))]
        })?;

        let achievements: Vec<AchievementDef> =
            serde_json::from_str(achievements_json).map_err(|e| {
                vec![CatalogError::new(format!(
                    "achievements.json: invalid JSON: {e}"
                ))]
            })?;

        let catalogs = Catalogs {
            generators,
            upgrades,
            achievements,
        };
        let errors = catalogs.validate();
        if errors.is_empty() {
            Ok(catalogs)
        } else {
            Err(errors)
        }
    }

    pub fn generator(&self, id: &str) -> Option<&GeneratorDef> {
        self.generators.iter().find(|g| g.id == id)
    }

    pub fn upgrade(&self, id: &str) -> Option<&UpgradeDef> {
        self.upgrades.iter().find(|u| u.id == id)
    }

    fn generator_ids(&self) -> std::collections::HashSet<&str> {
        self.generators.iter().map(|g| g.id.as_str()).collect()
    }

    fn upgrade_ids(&self) -> std::collections::HashSet<&str> {
        self.upgrades.iter().map(|u| u.id.as_str()).collect()
    }

    /// Validate the whole catalog set. Returns every problem found.
    pub fn validate(&self) -> Vec<CatalogError> {
        let mut errors = Vec::new();

        if self.generators.is_empty() {
            errors.push(CatalogError::new("generators.json: no generators"));
        }
        if self.upgrades.is_empty() {
            errors.push(CatalogError::new("upgrades.json: no upgrades"));
        }
        if self.achievements.is_empty() {
            errors.push(CatalogError::new("achievements.json: no achievements"));
        }

        let gen_ids = self.generator_ids();
        let up_ids = self.upgrade_ids();

        // Duplicate ids within a catalog would silently shadow; reject them.
        let mut seen = std::collections::HashSet::new();
        for g in &self.generators {
            if !seen.insert(g.id.as_str()) {
                errors.push(CatalogError::new(format!(
                    "generators.json: duplicate id '{}'",
                    g.id
                )));
            }
            self.validate_generator(g, &mut errors);
        }
        seen.clear();
        for u in &self.upgrades {
            if !seen.insert(u.id.as_str()) {
                errors.push(CatalogError::new(format!(
                    "upgrades.json: duplicate id '{}'",
                    u.id
                )));
            }
        }
        seen.clear();
        for a in &self.achievements {
            if !seen.insert(a.id.as_str()) {
                errors.push(CatalogError::new(format!(
                    "achievements.json: duplicate id '{}'",
                    a.id
                )));
            }
        }

        // Cross-catalog references and type checks.
        for u in &self.upgrades {
            if !u.cost.is_finite() {
                errors.push(CatalogError::new(format!(
                    "upgrade '{}': cost is not finite",
                    u.id
                )));
            }
            for effect in &u.effects {
                if !UPGRADE_EFFECT_TYPES.contains(&effect.kind.as_str()) {
                    errors.push(CatalogError::new(format!(
                        "upgrade '{}': unknown effect type '{}'",
                        u.id, effect.kind
                    )));
                }
                if !effect.multiplier.is_finite() {
                    errors.push(CatalogError::new(format!(
                        "upgrade '{}': effect multiplier is not finite",
                        u.id
                    )));
                }
                if effect.kind == "generatorProduction" {
                    match &effect.target {
                        Some(t) if !gen_ids.contains(t.as_str()) => {
                            errors.push(CatalogError::new(format!(
                                "upgrade '{}': generatorProduction target '{}' is not a generator",
                                u.id, t
                            )))
                        }
                        None => errors.push(CatalogError::new(format!(
                            "upgrade '{}': generatorProduction effect missing target",
                            u.id
                        ))),
                        _ => {}
                    }
                }
            }
            for cond in &u.unlock_conditions {
                if !UNLOCK_CONDITION_TYPES.contains(&cond.kind.as_str()) {
                    errors.push(CatalogError::new(format!(
                        "upgrade '{}': unknown unlock condition type '{}'",
                        u.id, cond.kind
                    )));
                }
                match cond.kind.as_str() {
                    "generatorCount" => {
                        if let Some(t) = &cond.target {
                            if !gen_ids.contains(t.as_str()) {
                                errors.push(CatalogError::new(format!(
                                    "upgrade '{}': generatorCount target '{}' is not a generator",
                                    u.id, t
                                )));
                            }
                        }
                    }
                    "upgrade" => {
                        if let Some(id) = &cond.id {
                            if !up_ids.contains(id.as_str()) {
                                errors.push(CatalogError::new(format!(
                                    "upgrade '{}': prerequisite upgrade '{}' does not exist",
                                    u.id, id
                                )));
                            }
                        } else {
                            errors.push(CatalogError::new(format!(
                                "upgrade '{}': 'upgrade' condition missing id",
                                u.id
                            )));
                        }
                    }
                    _ => {}
                }
            }
        }

        for a in &self.achievements {
            if !ACHIEVEMENT_REQUIREMENT_TYPES.contains(&a.requirement.kind.as_str()) {
                errors.push(CatalogError::new(format!(
                    "achievement '{}': unknown requirement type '{}'",
                    a.id, a.requirement.kind
                )));
            }
            if a.requirement.kind == "generatorType" {
                if let Some(t) = &a.requirement.target {
                    if !gen_ids.contains(t.as_str()) {
                        errors.push(CatalogError::new(format!(
                            "achievement '{}': generatorType target '{}' is not a generator",
                            a.id, t
                        )));
                    }
                }
            }
            if let Some(reward) = &a.reward {
                if !ACHIEVEMENT_REWARD_TYPES.contains(&reward.kind.as_str()) {
                    errors.push(CatalogError::new(format!(
                        "achievement '{}': unknown reward type '{}'",
                        a.id, reward.kind
                    )));
                }
                if !reward.value.is_finite() {
                    errors.push(CatalogError::new(format!(
                        "achievement '{}': reward value is not finite",
                        a.id
                    )));
                }
                if reward.kind == "generatorBoost" {
                    if let Some(t) = &reward.target {
                        if !gen_ids.contains(t.as_str()) {
                            errors.push(CatalogError::new(format!(
                                "achievement '{}': generatorBoost target '{}' is not a generator",
                                a.id, t
                            )));
                        }
                    }
                }
            }
        }

        errors
    }

    fn validate_generator(&self, g: &GeneratorDef, errors: &mut Vec<CatalogError>) {
        for f in [
            ("baseProduction", g.base_production),
            ("baseCost", g.base_cost),
            ("costMultiplier", g.cost_multiplier),
        ] {
            if !f.1.is_finite() {
                errors.push(CatalogError::new(format!(
                    "generator '{}': {} is not finite",
                    g.id, f.0
                )));
            }
        }
        if g.cost_multiplier <= 0.0 {
            errors.push(CatalogError::new(format!(
                "generator '{}': costMultiplier must be > 0",
                g.id
            )));
        }
        for req in &g.unlock_requirements {
            if !GENERATOR_UNLOCK_TYPES.contains(&req.kind.as_str()) {
                errors.push(CatalogError::new(format!(
                    "generator '{}': unknown unlock requirement type '{}'",
                    g.id, req.kind
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENERATORS_JSON: &str = include_str!("../../../assets/data/generators.json");
    const UPGRADES_JSON: &str = include_str!("../../../assets/data/upgrades.json");
    const ACHIEVEMENTS_JSON: &str = include_str!("../../../assets/data/achievements.json");

    #[test]
    fn shipped_catalogs_parse_and_validate() {
        let catalogs = Catalogs::from_json(GENERATORS_JSON, UPGRADES_JSON, ACHIEVEMENTS_JSON)
            .expect("shipped catalogs must validate");
        assert_eq!(catalogs.generators.len(), 14);
        assert!(!catalogs.upgrades.is_empty());
        assert!(!catalogs.achievements.is_empty());
    }

    #[test]
    fn missing_file_yields_error() {
        let err = Catalogs::from_json("", UPGRADES_JSON, ACHIEVEMENTS_JSON)
            .expect_err("empty generators must fail");
        assert!(!err.is_empty());
        assert!(err[0].message.contains("generators.json"));
    }

    #[test]
    fn malformed_json_yields_error() {
        let err = Catalogs::from_json("{ not json", UPGRADES_JSON, ACHIEVEMENTS_JSON)
            .expect_err("malformed generators must fail");
        assert!(!err.is_empty());
    }

    #[test]
    fn unknown_effect_type_is_rejected() {
        let bad_upgrades = r#"[{
            "id": "bad", "name": "Bad", "description": "d", "cost": 1,
            "effects": [{"type": "clickBpsBonus", "multiplier": 2}],
            "unlockConditions": []
        }]"#;
        let err = Catalogs::from_json(GENERATORS_JSON, bad_upgrades, ACHIEVEMENTS_JSON)
            .expect_err("unknown effect type must fail validation");
        assert!(
            err.iter().any(|e| e.message.contains("clickBpsBonus")),
            "expected unknown effect type to be flagged, got {err:?}"
        );
    }
}
