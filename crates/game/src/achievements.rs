//! Achievement rules: requirement checking, unlock-once, and reward
//! application. Console-open detection predicates are pure so they can be
//! tested natively; the browser crate wires them to keyboard/resize events.

use crate::catalog::{AchievementDef, Catalogs};
use crate::economy::total_production;
use crate::state::{Boost, GameState};

/// Whether a key event is a devtools/inspect shortcut (F12, or
/// ctrl/meta+shift+i/j/c). Best-effort browser signal, kept pure for tests.
pub fn console_shortcut_detects(key: &str, ctrl: bool, meta: bool, shift: bool) -> bool {
    let key = key.to_ascii_lowercase();
    if key == "f12" {
        return true;
    }
    (ctrl || meta) && shift && (key == "i" || key == "j" || key == "c")
}

/// Whether a docked-devtools size gap is detected: the viewport shrinks
/// without the window shrinking, so a large outer/inner gap means the panel
/// is open.
pub fn console_size_detects(
    outer_w: f64,
    inner_w: f64,
    outer_h: f64,
    inner_h: f64,
    threshold: f64,
) -> bool {
    (outer_w - inner_w) > threshold || (outer_h - inner_h) > threshold
}

/// The values achievement requirements read from (fully owned so the checker
/// can mutate state while iterating).
struct CheckContext {
    total_bufos: f64,
    bufos_per_second: f64,
    total_generators: f64,
    click_count: f64,
    console_opened: bool,
    upgrades_purchased: f64,
    bosses_defeated: f64,
    transcendences: f64,
    prestige_points: f64,
    generator_counts: std::collections::HashMap<String, f64>,
    custom_events: std::collections::HashMap<String, bool>,
}

fn build_context(state: &GameState, catalogs: &Catalogs, console_opened: bool) -> CheckContext {
    let total_generators: f64 = state.generators.values().map(|g| g.count).sum();
    let generator_counts: std::collections::HashMap<String, f64> = state
        .generators
        .iter()
        .map(|(k, g)| (k.clone(), g.count))
        .collect();
    CheckContext {
        total_bufos: state.resources.total_bufos,
        bufos_per_second: total_production(state, catalogs),
        total_generators,
        click_count: state.achievements.click_count,
        console_opened,
        upgrades_purchased: state.upgrades.purchased.len() as f64,
        // Lifetime, not this run: transcending clears `defeated` and banks it
        // in `lifetime_defeats`, and an earned achievement must survive.
        bosses_defeated: state.bosses.defeated.len() as f64 + state.bosses.lifetime_defeats,
        transcendences: state.prestige.transcendences,
        prestige_points: state.prestige.lifetime_points,
        generator_counts,
        custom_events: state.achievements.custom_events.clone(),
    }
}

/// Whether a single achievement's requirement is met.
fn check_requirement(a: &AchievementDef, ctx: &CheckContext) -> bool {
    let req = &a.requirement;
    match req.kind.as_str() {
        "totalBufos" => ctx.total_bufos >= req.value,
        "bufosPerSecond" => ctx.bufos_per_second >= req.value,
        "totalGenerators" => ctx.total_generators >= req.value,
        "generatorType" => match &req.target {
            Some(t) => ctx.generator_counts.get(t).copied().unwrap_or(0.0) >= req.value,
            None => false,
        },
        "clickCount" => ctx.click_count >= req.value,
        "consoleOpened" => ctx.console_opened,
        "upgradeCount" => ctx.upgrades_purchased >= req.value,
        "bossesDefeated" => ctx.bosses_defeated >= req.value,
        "transcendences" => ctx.transcendences >= req.value,
        "prestigePoints" => ctx.prestige_points >= req.value,
        "customEvent" => match &req.target {
            Some(t) => ctx.custom_events.get(t) == Some(&true),
            None => false,
        },
        _ => false,
    }
}

/// Check every achievement, unlock those whose requirements are now met, and
/// apply each reward exactly once. Returns the ids newly unlocked.
pub fn check_achievements(
    state: &mut GameState,
    catalogs: &Catalogs,
    console_opened: bool,
) -> Vec<String> {
    let ctx = build_context(state, catalogs, console_opened);
    let mut newly = Vec::new();
    for a in &catalogs.achievements {
        if state.achievements.unlocked.iter().any(|u| u == &a.id) {
            continue;
        }
        if check_requirement(a, &ctx) {
            state.achievements.unlocked.push(a.id.clone());
            apply_reward(a, state, catalogs);
            newly.push(a.id.clone());
        }
    }
    newly
}

/// Apply one achievement's reward (incremental, once).
fn apply_reward(a: &AchievementDef, state: &mut GameState, catalogs: &Catalogs) {
    let Some(reward) = &a.reward else { return };
    match reward.kind.as_str() {
        "productionBoost" => {
            state.resources.production_multiplier *= reward.value;
        }
        "clickBoost" => {
            state.resources.click_multiplier *= reward.value;
        }
        "generatorBoost" => {
            if let Some(target) = &reward.target {
                if let Some(gen) = state.generators.get_mut(target) {
                    gen.boosts.push(Boost {
                        id: format!("achievement_{}", a.id),
                        multiplier: reward.value,
                        active: true,
                        source: format!("Achievement: {}", a.name),
                    });
                }
            }
        }
        "bufoBonus" => {
            state.resources.bufos += reward.value;
            state.resources.total_bufos += reward.value;
        }
        _ => {}
    }
    let _ = catalogs;
}

/// Record a one-way custom-event latch (e.g. "beat this boss", "caught a
/// Golden Bufo") and check achievements gated on it. Returns newly unlocked.
pub fn trigger_custom_event(
    state: &mut GameState,
    catalogs: &Catalogs,
    event: &str,
) -> Vec<String> {
    state
        .achievements
        .custom_events
        .insert(event.to_string(), true);
    check_achievements(state, catalogs, false)
}

/// Re-apply every unlocked achievement's reward exactly once. Used on
/// load/migration after upgrade effects have been re-applied.
pub fn reapply_all_achievement_rewards(state: &mut GameState, catalogs: &Catalogs) {
    let unlocked = state.achievements.unlocked.clone();
    for id in &unlocked {
        if let Some(a) = catalogs.achievements.iter().find(|a| &a.id == id) {
            apply_reward(a, state, catalogs);
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
    fn console_shortcut_predicate() {
        assert!(console_shortcut_detects("F12", false, false, false));
        assert!(console_shortcut_detects("i", true, false, true));
        assert!(console_shortcut_detects("j", false, true, true));
        assert!(!console_shortcut_detects("i", true, false, false)); // no shift
        assert!(!console_shortcut_detects("x", true, false, true));
    }

    #[test]
    fn console_size_predicate() {
        assert!(console_size_detects(1000.0, 800.0, 800.0, 800.0, 160.0));
        assert!(!console_size_detects(1000.0, 900.0, 800.0, 800.0, 160.0));
    }

    #[test]
    fn first_bufo_unlocks_once() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.total_bufos = 1.0;
        let newly = check_achievements(&mut s, &c, false);
        assert!(newly.contains(&"first_bufo".to_string()));
        assert!(s.achievements.unlocked.iter().any(|u| u == "first_bufo"));
        // first_bufo has a clickBoost reward of 1.1x
        assert!((s.resources.click_multiplier - 1.1).abs() < 1e-9);
        // Second check unlocks nothing new and does not double the reward.
        let again = check_achievements(&mut s, &c, false);
        assert!(!again.contains(&"first_bufo".to_string()));
        assert!((s.resources.click_multiplier - 1.1).abs() < 1e-9);
    }

    #[test]
    fn click_count_achievement() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.achievements.click_count = 10.0;
        let newly = check_achievements(&mut s, &c, false);
        assert!(newly.contains(&"click_10".to_string()));
    }

    #[test]
    fn custom_event_unlocks_gated_achievement() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        // boss_omniscient_bufo gates the `final_boss` achievement via customEvent.
        let newly = trigger_custom_event(&mut s, &c, "boss_omniscient_bufo");
        assert!(s.achievements.custom_events.get("boss_omniscient_bufo") == Some(&true));
        assert!(newly.contains(&"final_boss".to_string()));
    }

    #[test]
    fn golden_bufo_caught_unlocks_golden_first() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        let newly = trigger_custom_event(&mut s, &c, "golden_bufo_caught");
        assert!(s.achievements.custom_events.get("golden_bufo_caught") == Some(&true));
        // `golden_first` gates on the golden_bufo_caught custom event.
        assert!(newly.contains(&"golden_first".to_string()));
        assert!(s.achievements.unlocked.iter().any(|u| u == "golden_first"));
    }
}
