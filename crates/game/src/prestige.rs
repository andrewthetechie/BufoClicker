//! Prestige ("Transcendence Bufoplier"): the soft reset and its point award.
//!
//! Transcending wipes bufos, generators and upgrades, banks Bufoplier points
//! from this run's total bufos, and keeps permanent prestige and boss rewards.
//! Matches the TypeScript exactly, including resetting the upgrade/achievement
//! multipliers to 1 without re-applying achievement rewards (the reference
//! behavior — see the parity matrix).

use crate::catalog::Catalogs;
use crate::state::{prestige_points_for, GameState};

/// Whether a transcend is currently allowed (would yield at least 1 point).
pub fn can_transcend(state: &GameState) -> bool {
    prestige_points_for(state.resources.total_bufos) >= 1.0
}

/// Perform the soft reset and bank the earned points. Returns points gained
/// (0 if not allowed).
pub fn transcend(state: &mut GameState, catalogs: &Catalogs) -> f64 {
    let gained = prestige_points_for(state.resources.total_bufos);
    if gained < 1.0 {
        return 0.0;
    }

    state.prestige.points += gained;
    state.prestige.lifetime_points += gained;
    state.prestige.transcendences += 1.0;

    // Bank this run's boss defeats so their multiplier carries over, then
    // clear the ladder so bosses are offered again.
    let banked = state.bosses.lifetime_defeats + state.bosses.defeated.len() as f64;
    state.bosses.defeated.clear();
    state.bosses.lifetime_defeats = banked;

    reset_run(state, catalogs);
    gained
}

/// The soft-reset half of transcending (also used by the in-memory reset).
pub fn reset_run(state: &mut GameState, catalogs: &Catalogs) {
    state.resources.bufos = 0.0;
    state.resources.total_bufos = 0.0;
    state.resources.click_multiplier = 1.0;
    state.resources.production_multiplier = 1.0;
    state.resources.frenzy_production_multiplier = 1.0;
    state.resources.frenzy_click_multiplier = 1.0;
    state.upgrades.purchased.clear();
    state.upgrades.available.clear();
    for def in &catalogs.generators {
        if let Some(gen) = state.generators.get_mut(&def.id) {
            gen.count = 0.0;
            gen.unlocked = crate::state::initial_unlocked(def);
            gen.boosts.clear();
        }
    }
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
    fn below_threshold_is_ignored() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.total_bufos = 999_999_999.0;
        assert_eq!(transcend(&mut s, &c), 0.0);
        assert_eq!(s.prestige.transcendences, 0.0);
    }

    #[test]
    fn transcend_awards_points_and_resets_run() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.total_bufos = 1_000_000_000.0; // 1 point
        s.resources.bufos = 500.0;
        s.generators.get_mut("tadpole").unwrap().count = 5.0;
        s.upgrades.purchased.push("stronger_clicks_1".to_string());
        s.bosses.defeated = vec!["furious_froglet".to_string()];
        s.bosses.lifetime_defeats = 2.0;

        let gained = transcend(&mut s, &c);
        assert_eq!(gained, 1.0);
        assert_eq!(s.prestige.points, 1.0);
        assert_eq!(s.prestige.lifetime_points, 1.0);
        assert_eq!(s.prestige.transcendences, 1.0);
        // Run wiped.
        assert_eq!(s.resources.bufos, 0.0);
        assert_eq!(s.resources.total_bufos, 0.0);
        assert_eq!(s.generator_count("tadpole"), 0.0);
        assert!(s.upgrades.purchased.is_empty());
        // Bosses banked, ladder cleared.
        assert!(s.bosses.defeated.is_empty());
        assert_eq!(s.bosses.lifetime_defeats, 3.0);
    }
}
