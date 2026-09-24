//! Clicker Boss ladder: a sequential set of click-power checkpoints.
//!
//! Boss damage is `click_power`, and a fight grants no bufo income. Win banks
//! a permanent multiplier; lose clears current bufos (generators, upgrades,
//! prestige, achievements untouched). Health scales with prestige and boss
//! bonuses so the ladder stays a click checkpoint regardless of prestige.

use crate::state::{boss_multiplier, prestige_multiplier, GameState};

/// How long a fight lasts, in milliseconds.
pub const BOSS_FIGHT_DURATION_MS: f64 = 30_000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct BossDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub flavor_text: &'static str,
    /// Total bufos ever earned needed before this boss can be challenged.
    pub threshold: f64,
    /// Health for a player with no prestige and no defeated bosses.
    pub base_health: f64,
    pub icon_path: &'static str,
}

pub const BOSSES: [BossDefinition; 7] = [
    BossDefinition {
        id: "furious_froglet",
        name: "Furious Froglet",
        flavor_text: "It's smaller than you, but it is FURIOUS about it.",
        threshold: 10_000.0,
        base_health: 975.0,
        icon_path: "./assets/images/bosses/bufo-very-angry.png",
    },
    BossDefinition {
        id: "the_enraged_bufo",
        name: "The Enraged Bufo",
        flavor_text: "Every click you’ve ever made has led to this moment of pure rage.",
        threshold: 100_000_000.0,
        base_health: 53_800.0,
        icon_path: "./assets/images/bosses/bufo-enraged.png",
    },
    BossDefinition {
        id: "bufo_dragon",
        name: "Bufo Dragon",
        flavor_text: "Legends spoke of a bufo that ascended beyond amphibian. This is it.",
        threshold: 5_000_000_000.0,
        base_health: 38_700_000.0,
        icon_path: "./assets/images/bosses/bufo-dragon.png",
    },
    BossDefinition {
        id: "bufo_devil",
        name: "Bufo Devil",
        flavor_text: "It offers you a deal. You should probably just click it instead.",
        threshold: 100_000_000_000.0,
        base_health: 65_600_000.0,
        icon_path: "./assets/images/bosses/bufo-devil.png",
    },
    BossDefinition {
        id: "mega_bufo",
        name: "MEGA BUFO",
        flavor_text: "The one all other bufos speak of in hushed croaks. Surely nothing tops this... right?",
        threshold: 10_000_000_000_000.0,
        base_health: 7_070_000_000.0,
        icon_path: "./assets/images/bosses/mega-bufo.png",
    },
    BossDefinition {
        id: "interdimensional_bufo",
        name: "Interdimensional Bufo",
        flavor_text: "It rests atop the terrarium of existence, watching your entire pond like it were a fish tank.",
        threshold: 50_000_000_000_000.0,
        base_health: 78_200_000_000.0,
        icon_path: "./assets/images/bosses/terrarium.png",
    },
    BossDefinition {
        id: "omniscient_bufo",
        name: "The Omniscient Bufo",
        flavor_text: "It already knows how this fight ends. Prove it wrong.",
        threshold: 2_000_000_000_000_000.0,
        base_health: 1_670_000_000_000.0,
        icon_path: "./assets/images/bosses/omniscient.png",
    },
];

pub fn find_boss(id: &str) -> Option<&'static BossDefinition> {
    BOSSES.iter().find(|b| b.id == id)
}

/// The next boss the player can challenge: the first not-yet-defeated boss
/// whose threshold `total_bufos` meets. Bosses must be beaten in order.
pub fn get_available_boss(
    defeated: &[String],
    total_bufos: f64,
) -> Option<&'static BossDefinition> {
    for boss in BOSSES.iter() {
        if defeated.iter().any(|d| d == boss.id) {
            continue;
        }
        return if total_bufos >= boss.threshold {
            Some(boss)
        } else {
            None
        };
    }
    None
}

/// The real health of a fight against `boss` for the player in `state`.
/// Prestige and boss bonuses are multiplied back in so the click target stays
/// invariant to prestige (see the ADR note in the TS `boss.ts`).
pub fn get_boss_health(boss: &BossDefinition, state: &GameState) -> f64 {
    let passive = prestige_multiplier(state) * boss_multiplier(state);
    (boss.base_health * passive).ceil().max(1.0)
}

/// Win a fight: record the defeat (permanent multiplier updates immediately).
pub fn win_fight(state: &mut GameState, boss_id: &str) {
    if !state.bosses.defeated.iter().any(|d| d == boss_id) {
        state.bosses.defeated.push(boss_id.to_string());
    }
}

/// Lose a fight: clear current bufos, keep everything else.
pub fn lose_fight(state: &mut GameState) {
    state.resources.bufos = 0.0;
}

/// Retreat from a fight in progress: no penalty, no durable state change.
/// The fight itself is transient UI state owned by the browser crate; this
/// exists so the no-penalty rule is explicit and testable.
pub fn retreat_fight(_state: &GameState) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalogs;
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
    fn available_boss_is_in_order_and_threshold_gated() {
        assert!(get_available_boss(&[], 9_999.0).is_none());
        assert_eq!(
            get_available_boss(&[], 10_000.0).unwrap().id,
            "furious_froglet"
        );
        // The next boss is gated by its own (much higher) threshold.
        assert!(get_available_boss(&["furious_froglet".to_string()], 10_000.0).is_none());
        assert_eq!(
            get_available_boss(&["furious_froglet".to_string()], 100_000_000.0)
                .unwrap()
                .id,
            "the_enraged_bufo"
        );
    }

    #[test]
    fn boss_health_scales_with_prestige_and_boss_bonus() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        let boss = find_boss("furious_froglet").unwrap();
        // No prestige, no bosses: 975.
        assert_eq!(get_boss_health(boss, &s), 975.0);
        // 5 lifetime prestige points (1.5x) and 3 lifetime defeats (1.75x).
        s.prestige.lifetime_points = 5.0;
        s.bosses.lifetime_defeats = 3.0;
        // 975 * 1.5 * 1.75 = 2559.375 -> ceil 2560
        assert_eq!(get_boss_health(boss, &s), 2560.0);
    }

    #[test]
    fn win_records_defeat_lose_clears_bufos() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.bufos = 999.0;
        s.generators.get_mut("tadpole").unwrap().count = 3.0;
        win_fight(&mut s, "furious_froglet");
        assert!(s.bosses.defeated.iter().any(|d| d == "furious_froglet"));
        lose_fight(&mut s);
        assert_eq!(s.resources.bufos, 0.0);
        assert_eq!(s.generator_count("tadpole"), 3.0); // generators kept
        assert!(s.bosses.defeated.iter().any(|d| d == "furious_froglet")); // boss kept
    }

    #[test]
    fn retreat_leaves_state_unchanged() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.bufos = 100.0;
        s.generators.get_mut("tadpole").unwrap().count = 2.0;
        retreat_fight(&s);
        // No penalty: no defeat recorded, no bufos lost, no generators lost.
        assert_eq!(s.resources.bufos, 100.0);
        assert_eq!(s.generator_count("tadpole"), 2.0);
        assert!(s.bosses.defeated.is_empty());
    }
}
