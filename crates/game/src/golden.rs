//! Golden Bufo rules: spawn timing, reward roll, and the three reward types.
//!
//! Random values and wall-clock time are passed in as arguments so native
//! tests are deterministic. The browser crate owns the timers and the on-screen
//! spawn; this crate owns the math.

use crate::state::GameState;

// Timing (ms)
pub const FIRST_SPAWN_MIN_MS: f64 = 45_000.0;
pub const FIRST_SPAWN_MAX_MS: f64 = 90_000.0;
pub const SPAWN_MIN_MS: f64 = 90_000.0;
pub const SPAWN_MAX_MS: f64 = 190_000.0;
pub const ON_SCREEN_TTL_MS: f64 = 13_000.0;

pub const BUFO_FRENZY_MULT: f64 = 7.0;
pub const BUFO_FRENZY_MS: f64 = 30_000.0;
pub const CLICK_FRENZY_MULT: f64 = 7.0;
pub const CLICK_FRENZY_MS: f64 = 15_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardType {
    BufoFrenzy,
    ClickFrenzy,
    Lucky,
}

/// Pick the reward from a uniform roll in [0, 1): <0.5 production frenzy,
/// <0.8 lucky, else click frenzy.
pub fn roll_reward(roll: f64) -> RewardType {
    if roll < 0.5 {
        RewardType::BufoFrenzy
    } else if roll < 0.8 {
        RewardType::Lucky
    } else {
        RewardType::ClickFrenzy
    }
}

/// Spawn position as viewport percentages, kept away from the edges. `x_roll`
/// and `y_roll` are uniform in [0, 1).
pub fn spawn_position(x_roll: f64, y_roll: f64) -> (f64, f64) {
    (8.0 + x_roll * 76.0, 14.0 + y_roll * 64.0)
}

/// The "Lucky" windfall: 15% of the bank, capped at 20 minutes of production,
/// plus 13 for luck.
pub fn lucky_gain(bufos: f64, per_second: f64) -> f64 {
    (bufos * 0.15).min(per_second * 60.0 * 20.0).floor() + 13.0
}

/// Apply a Golden Bufo reward to the state. `now_ms` is wall-clock time for
/// the frenzy expiry timestamps the browser crate reads back. `per_second` is
/// the current production rate (used by the Lucky windfall).
pub fn apply_reward(
    state: &mut GameState,
    reward: RewardType,
    now_ms: f64,
    per_second: f64,
) -> AppliedReward {
    match reward {
        RewardType::BufoFrenzy => {
            state.resources.frenzy_production_multiplier = BUFO_FRENZY_MULT;
            AppliedReward {
                reward,
                label: "Bufo Frenzy!".to_string(),
                detail: format!(
                    "x{BUFO_FRENZY_MULT:.0} production for {}s",
                    BUFO_FRENZY_MS / 1000.0
                ),
                frenzy_ends_at: now_ms + BUFO_FRENZY_MS,
            }
        }
        RewardType::ClickFrenzy => {
            state.resources.frenzy_click_multiplier = CLICK_FRENZY_MULT;
            AppliedReward {
                reward,
                label: "Click Frenzy!".to_string(),
                detail: format!(
                    "x{CLICK_FRENZY_MULT:.0} click power for {}s",
                    CLICK_FRENZY_MS / 1000.0
                ),
                frenzy_ends_at: now_ms + CLICK_FRENZY_MS,
            }
        }
        RewardType::Lucky => {
            let gain = lucky_gain(state.resources.bufos, per_second);
            state.resources.bufos += gain;
            state.resources.total_bufos += gain;
            AppliedReward {
                reward,
                label: "Lucky!".to_string(),
                detail: format!("+{} bufos", crate::number::format_number(gain, 1)),
                frenzy_ends_at: 0.0,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppliedReward {
    pub reward: RewardType,
    pub label: String,
    pub detail: String,
    /// Wall-clock expiry for frenzy rewards (0 for lucky).
    pub frenzy_ends_at: f64,
}

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
    fn reward_roll_thresholds() {
        assert_eq!(roll_reward(0.0), RewardType::BufoFrenzy);
        assert_eq!(roll_reward(0.49), RewardType::BufoFrenzy);
        assert_eq!(roll_reward(0.5), RewardType::Lucky);
        assert_eq!(roll_reward(0.79), RewardType::Lucky);
        assert_eq!(roll_reward(0.8), RewardType::ClickFrenzy);
        assert_eq!(roll_reward(0.99), RewardType::ClickFrenzy);
    }

    #[test]
    fn spawn_position_stays_in_bounds() {
        let (x, y) = spawn_position(0.0, 0.0);
        assert_eq!(x, 8.0);
        assert_eq!(y, 14.0);
        let (x, y) = spawn_position(1.0, 1.0);
        assert_eq!(x, 84.0);
        assert_eq!(y, 78.0);
    }

    #[test]
    fn lucky_gain_formula() {
        // 15% of a small bank, but capped at 20 minutes of production (0 here).
        assert_eq!(lucky_gain(100.0, 0.0), 13.0); // floor(min(15, 0)) + 13
                                                  // Capped at 20 minutes of production: floor(10*1200) + 13.
        assert_eq!(lucky_gain(1_000_000.0, 10.0), 12_013.0);
    }

    #[test]
    fn frenzy_reward_sets_multiplier_and_expiry() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        let r = apply_reward(&mut s, RewardType::BufoFrenzy, 1_000_000.0, 0.0);
        assert_eq!(s.resources.frenzy_production_multiplier, 7.0);
        assert_eq!(r.frenzy_ends_at, 1_030_000.0);
        let r = apply_reward(&mut s, RewardType::ClickFrenzy, 1_000_000.0, 0.0);
        assert_eq!(s.resources.frenzy_click_multiplier, 7.0);
        assert_eq!(r.frenzy_ends_at, 1_015_000.0);
    }

    #[test]
    fn lucky_reward_grants_bufos() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.bufos = 100.0;
        let r = apply_reward(&mut s, RewardType::Lucky, 0.0, 0.0);
        assert_eq!(r.detail, "+13 bufos");
        assert_eq!(s.resources.bufos, 113.0);
        assert_eq!(s.resources.total_bufos, 13.0);
    }
}
