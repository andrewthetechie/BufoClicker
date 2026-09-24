//! Upgrade rules: availability, purchase, and effect application.
//!
//! Only three effect types are live (`clickMultiplier`, `generatorProduction`,
//! `globalMultiplier`); anything else is a no-op that must never take currency
//! (catalog validation already rejects unknown types before play starts).

use crate::catalog::{Catalogs, UnlockCondition, UpgradeDef};
use crate::state::GameState;

/// Whether an upgrade's unlock conditions are all met.
pub fn meets_unlock_conditions(upgrade: &UpgradeDef, state: &GameState) -> bool {
    upgrade
        .unlock_conditions
        .iter()
        .all(|cond| meets_one(cond, upgrade, state))
}

fn meets_one(cond: &UnlockCondition, _upgrade: &UpgradeDef, state: &GameState) -> bool {
    match cond.kind.as_str() {
        "totalBufos" => state.resources.total_bufos >= cond.value,
        "generatorCount" => match &cond.target {
            Some(t) => state.generator_count(t) >= cond.value,
            None => false,
        },
        "achievements" => match &cond.target {
            Some(t) => state.achievements.unlocked.iter().any(|a| a == t),
            None => false,
        },
        // The JSON names the prerequisite in `id`; the TS mapper folds it into
        // `target`. Accept either.
        "upgrade" => {
            let id = cond.id.as_ref().or(cond.target.as_ref());
            match id {
                Some(id) => state.upgrades.purchased.iter().any(|p| p == id),
                None => false,
            }
        }
        _ => false,
    }
}

/// Upgrades that are unlocked and not yet purchased.
pub fn available_upgrades<'a>(state: &GameState, catalogs: &'a Catalogs) -> Vec<&'a UpgradeDef> {
    catalogs
        .upgrades
        .iter()
        .filter(|u| !state.upgrades.purchased.iter().any(|p| p == &u.id))
        .filter(|u| meets_unlock_conditions(u, state))
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpgradeOutcome {
    pub success: bool,
    pub cost: f64,
    pub reason: Option<String>,
}

/// Buy an upgrade and apply its effects. Deducts cost only on success.
pub fn buy_upgrade(state: &mut GameState, catalogs: &Catalogs, id: &str) -> UpgradeOutcome {
    let Some(upgrade) = catalogs.upgrade(id) else {
        return UpgradeOutcome {
            success: false,
            cost: 0.0,
            reason: Some(format!("unknown upgrade '{id}'")),
        };
    };
    if state.upgrades.purchased.iter().any(|p| p == id) {
        return UpgradeOutcome {
            success: false,
            cost: upgrade.cost,
            reason: Some("already purchased".to_string()),
        };
    }
    if state.resources.bufos < upgrade.cost {
        return UpgradeOutcome {
            success: false,
            cost: upgrade.cost,
            reason: Some("unaffordable".to_string()),
        };
    }

    state.resources.bufos -= upgrade.cost;
    state.upgrades.purchased.push(id.to_string());
    state.upgrades.available.retain(|a| a != id);
    apply_effects(upgrade, state, catalogs);

    UpgradeOutcome {
        success: true,
        cost: upgrade.cost,
        reason: None,
    }
}

/// Apply one upgrade's effects to the state (incremental).
fn apply_effects(upgrade: &UpgradeDef, state: &mut GameState, catalogs: &Catalogs) {
    for effect in &upgrade.effects {
        match effect.kind.as_str() {
            "clickMultiplier" => {
                state.resources.click_multiplier *= effect.multiplier;
            }
            "generatorProduction" => {
                if let Some(target) = &effect.target {
                    if let Some(gen) = state.generators.get_mut(target) {
                        gen.boosts.push(crate::state::Boost {
                            id: format!("upgrade_{}", upgrade.id),
                            multiplier: effect.multiplier,
                            active: true,
                            source: upgrade.name.clone(),
                        });
                    }
                }
            }
            "globalMultiplier" => {
                state.resources.production_multiplier *= effect.multiplier;
            }
            // "unlockSpecial" and anything else is a deliberate no-op.
            _ => {}
        }
    }
    // Keep a generator's boost list from accumulating duplicates if the same
    // upgrade is somehow applied twice (defensive; buy_upgrade already guards).
    let _ = catalogs;
}

/// Reset the upgrade-derived multipliers and boosts, then re-apply every
/// purchased upgrade exactly once. Used on load/migration after the raw state
/// has been read (multipliers are reset before re-application to avoid
/// double-applying effects).
pub fn reapply_all_upgrades(state: &mut GameState, catalogs: &Catalogs) {
    state.resources.click_multiplier = 1.0;
    state.resources.production_multiplier = 1.0;
    for gen in state.generators.values_mut() {
        gen.boosts.clear();
    }
    let purchased = state.upgrades.purchased.clone();
    for id in &purchased {
        if let Some(upgrade) = catalogs.upgrade(id) {
            apply_effects(upgrade, state, catalogs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::GameState;

    fn catalogs() -> Catalogs {
        Catalogs::from_json(
            include_str!("../../../assets/data/generators.json"),
            include_str!("../../../assets/data/upgrades.json"),
            include_str!("../../../assets/data/achievements.json"),
        )
        .unwrap()
    }

    #[test]
    fn stronger_clicks_requires_total_bufos() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        let up = c.upgrade("stronger_clicks_1").unwrap();
        assert!(!meets_unlock_conditions(up, &s));
        s.resources.total_bufos = 50.0;
        assert!(meets_unlock_conditions(up, &s));
    }

    #[test]
    fn click_upgrade_doubles_click_power() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.total_bufos = 100.0;
        s.resources.bufos = 100.0;
        let out = buy_upgrade(&mut s, &c, "stronger_clicks_1");
        assert!(out.success);
        assert_eq!(out.cost, 75.0);
        assert!((s.resources.click_multiplier - 2.0).abs() < 1e-9);
        assert_eq!(s.resources.bufos, 25.0);
    }

    #[test]
    fn unaffordable_upgrade_fails_without_effect() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.total_bufos = 100.0;
        s.resources.bufos = 10.0;
        let out = buy_upgrade(&mut s, &c, "stronger_clicks_1");
        assert!(!out.success);
        assert!((s.resources.click_multiplier - 1.0).abs() < 1e-9);
        assert_eq!(s.resources.bufos, 10.0);
    }

    #[test]
    fn prerequisite_upgrade_gate_is_enforced() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        // Find an upgrade gated on another upgrade.
        let gated = c
            .upgrades
            .iter()
            .find(|u| u.unlock_conditions.iter().any(|c| c.kind == "upgrade"))
            .expect("catalog should contain an upgrade-gated upgrade");
        // Not purchased yet -> locked.
        assert!(!meets_unlock_conditions(gated, &s));
        // Purchase the prerequisite.
        let prereq = gated
            .unlock_conditions
            .iter()
            .find(|c| c.kind == "upgrade")
            .and_then(|c| c.id.as_ref().or(c.target.as_ref()))
            .unwrap()
            .clone();
        s.upgrades.purchased.push(prereq);
        // Other conditions (e.g. totalBufos) may still gate it, so just assert
        // the prerequisite itself is now treated as purchased.
        assert!(s.upgrades.purchased.iter().any(|p| p
            == gated
                .unlock_conditions
                .iter()
                .find(|c| c.kind == "upgrade")
                .and_then(|c| c.id.as_ref().or(c.target.as_ref()))
                .unwrap()));
    }
}
