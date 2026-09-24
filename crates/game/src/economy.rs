//! Economy rules: generator costs, production, unlocks, clicking, and buying.
//!
//! Every formula here mirrors the TypeScript implementation exactly (f64
//! arithmetic, `ceil`/`floor` in the same places) so native tests can pin the
//! same numbers the old client produced.

use crate::catalog::{Catalogs, GeneratorDef};
use crate::state::{click_power, global_production_multiplier, Boost, GameState, GeneratorState};

/// Cost of the next unit of `def` at the given owned count.
pub fn generator_cost(def: &GeneratorDef, count: f64) -> f64 {
    if count == 0.0 {
        def.base_cost
    } else {
        (def.base_cost * def.cost_multiplier.powf(count)).ceil()
    }
}

/// Cost of buying `quantity` units starting from `count` owned (geometric sum).
pub fn bulk_cost(def: &GeneratorDef, count: f64, quantity: f64) -> f64 {
    if quantity <= 1.0 {
        return generator_cost(def, count);
    }
    let a = generator_cost(def, count);
    let r = def.cost_multiplier;
    (a * (1.0 - r.powf(quantity)) / (1.0 - r)).ceil()
}

/// Maximum units affordable with `bufos`, starting from `count` owned.
pub fn max_affordable(def: &GeneratorDef, count: f64, bufos: f64) -> f64 {
    let current = generator_cost(def, count);
    if bufos < current {
        return 0.0;
    }
    let r = def.cost_multiplier;
    if r == 1.0 {
        return (bufos / current).floor();
    }
    let numerator = bufos * (r - 1.0) / (def.base_cost * r.powf(count)) + 1.0;
    if numerator <= 0.0 {
        return 0.0;
    }
    (numerator.ln() / r.ln()).floor().max(0.0)
}

fn boost_multiplier(boosts: &[Boost]) -> f64 {
    let mut m = 1.0;
    for b in boosts {
        if b.active {
            m *= b.multiplier;
        }
    }
    m
}

/// Production per second of one unit of `def` for the given generator state.
pub fn production_per_unit(def: &GeneratorDef, gen: &GeneratorState, state: &GameState) -> f64 {
    def.base_production * boost_multiplier(&gen.boosts) * global_production_multiplier(state)
}

/// Total production per second of all owned units of `def`.
pub fn generator_total_production(
    def: &GeneratorDef,
    gen: &GeneratorState,
    state: &GameState,
) -> f64 {
    production_per_unit(def, gen, state) * gen.count
}

/// Total production per second across all generators.
pub fn total_production(state: &GameState, catalogs: &Catalogs) -> f64 {
    catalogs
        .generators
        .iter()
        .filter_map(|def| {
            state
                .generators
                .get(&def.id)
                .map(|gen| generator_total_production(def, gen, state))
        })
        .sum()
}

/// Whether a generator's unlock requirements are all met.
pub fn meets_unlock_requirements(
    def: &GeneratorDef,
    state: &GameState,
    unlocked_achievements: &std::collections::HashSet<String>,
) -> bool {
    def.unlock_requirements
        .iter()
        .all(|req| match req.kind.as_str() {
            "bufos" => state.resources.total_bufos >= req.value,
            "generators" => match &req.target {
                Some(t) => state.generator_count(t) >= req.value,
                None => false,
            },
            "achievement" => match &req.target {
                Some(t) => unlocked_achievements.contains(t),
                None => false,
            },
            "special" => false,
            _ => false,
        })
}

/// Re-evaluate every generator's unlocked flag. Returns ids newly unlocked.
pub fn check_unlocks(state: &mut GameState, catalogs: &Catalogs) -> Vec<String> {
    let unlocked_achievements: std::collections::HashSet<String> =
        state.achievements.unlocked.iter().cloned().collect();
    let mut newly = Vec::new();
    for def in &catalogs.generators {
        let should = meets_unlock_requirements(def, state, &unlocked_achievements);
        if let Some(gen) = state.generators.get_mut(&def.id) {
            if should && !gen.unlocked {
                gen.unlocked = true;
                newly.push(def.id.clone());
            } else if !should {
                gen.unlocked = false;
            }
        }
    }
    newly
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClickOutcome {
    pub earned: f64,
    pub click_power: f64,
}

/// The live client adds 5% for each click within 500 ms of the previous
/// click, capped at ten combo steps. The combo is transient UI state.
pub fn click_combo(last_click_ms: f64, now_ms: f64, current_combo: u8) -> (u8, f64) {
    let next = if now_ms - last_click_ms < 500.0 {
        current_combo.saturating_add(1).min(10)
    } else {
        0
    };
    (next, 1.0 + f64::from(next) * 0.05)
}

/// A normal click on the main bufo: earns `click_power` bufos and counts one
/// click. Boss hits call `register_click` separately (no income).
pub fn click(state: &mut GameState) -> ClickOutcome {
    click_with_multiplier(state, 1.0)
}

/// Register a main-bufo click with its transient combo multiplier.
pub fn click_with_multiplier(state: &mut GameState, combo_multiplier: f64) -> ClickOutcome {
    let power = click_power(state) * combo_multiplier;
    state.resources.bufos += power;
    state.resources.total_bufos += power;
    state.resources.click_count += 1.0;
    state.achievements.click_count += 1.0;
    ClickOutcome {
        earned: power,
        click_power: power,
    }
}

/// Count a click without granting income (boss hits). Keeps the click counter
/// and the achievement counter in one place.
pub fn register_click(state: &mut GameState) {
    state.resources.click_count += 1.0;
    state.achievements.click_count += 1.0;
}

/// Advance production by `dt_seconds`. Adds `total_production * dt` to both
/// the spendable and lifetime bufo counters.
pub fn tick(state: &mut GameState, catalogs: &Catalogs, dt_seconds: f64) {
    if !dt_seconds.is_finite() || dt_seconds <= 0.0 {
        return;
    }
    let prod = total_production(state, catalogs);
    state.resources.bufos += prod * dt_seconds;
    state.resources.total_bufos += prod * dt_seconds;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElapsedOutcome {
    pub time_away_seconds: f64,
    pub production: f64,
    pub capped_production: f64,
    pub is_capped: bool,
}

/// Credit production for wall-clock time the game wasn't ticking.
///
/// `minimum_ms` is the one-minute floor on a closed-page reload (a quick
/// refresh shouldn't pop a "welcome back"), and 0 for a tab merely being
/// backgrounded. Credits at most 12 hours on both paths.
pub fn apply_elapsed_production(
    state: &mut GameState,
    catalogs: &Catalogs,
    last_tick: f64,
    now: f64,
    minimum_ms: f64,
) -> Option<ElapsedOutcome> {
    let away_ms = now - last_tick;
    if away_ms < minimum_ms || away_ms <= 0.0 {
        return None;
    }
    let away_seconds = away_ms / 1000.0;
    let capped_seconds = away_seconds.min(12.0 * 3600.0);
    let rate = total_production(state, catalogs);
    let production = rate * away_seconds;
    let capped = rate * capped_seconds;
    let is_capped = away_seconds > capped_seconds;
    if capped > 0.0 {
        state.resources.bufos += capped;
        state.resources.total_bufos += capped;
    }
    Some(ElapsedOutcome {
        time_away_seconds: away_seconds,
        production,
        capped_production: capped,
        is_capped,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct PurchaseOutcome {
    pub success: bool,
    pub cost: f64,
    pub quantity: f64,
    pub production_increase: f64,
    /// Human-readable failure reason (locked / unaffordable / unknown id).
    pub reason: Option<String>,
}

/// Buy `quantity` of a generator (`quantity == -1` means "max affordable").
pub fn buy_generator(
    state: &mut GameState,
    catalogs: &Catalogs,
    id: &str,
    quantity: f64,
) -> PurchaseOutcome {
    let Some(def) = catalogs.generator(id) else {
        return PurchaseOutcome {
            success: false,
            cost: 0.0,
            quantity: 0.0,
            production_increase: 0.0,
            reason: Some(format!("unknown generator '{id}'")),
        };
    };
    let Some(gen) = state.generators.get(&def.id) else {
        return PurchaseOutcome {
            success: false,
            cost: 0.0,
            quantity: 0.0,
            production_increase: 0.0,
            reason: Some(format!("generator '{id}' missing from state")),
        };
    };
    if !gen.unlocked {
        return PurchaseOutcome {
            success: false,
            cost: 0.0,
            quantity: 0.0,
            production_increase: 0.0,
            reason: Some(format!("generator '{id}' is locked")),
        };
    }

    let count = gen.count;
    let quantity = if quantity == -1.0 {
        max_affordable(def, count, state.resources.bufos)
    } else {
        quantity
    };
    if quantity <= 0.0 {
        return PurchaseOutcome {
            success: false,
            cost: 0.0,
            quantity: 0.0,
            production_increase: 0.0,
            reason: Some("cannot afford any".to_string()),
        };
    }

    let cost = bulk_cost(def, count, quantity);
    if state.resources.bufos < cost {
        return PurchaseOutcome {
            success: false,
            cost,
            quantity: 0.0,
            production_increase: 0.0,
            reason: Some("unaffordable".to_string()),
        };
    }

    let before = generator_total_production(def, gen, state);
    state.resources.bufos -= cost;
    {
        let gen = state.generators.get_mut(&def.id).expect("checked above");
        gen.count += quantity;
    }
    // Recompute production increase with the updated count.
    let gen = state.generators.get(&def.id).expect("checked above");
    let after = generator_total_production(def, gen, state);

    check_unlocks(state, catalogs);

    PurchaseOutcome {
        success: true,
        cost,
        quantity,
        production_increase: after - before,
        reason: None,
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

    fn tadpole(c: &Catalogs) -> &GeneratorDef {
        c.generator("tadpole").unwrap()
    }

    #[test]
    fn cost_formula_matches_ts() {
        let c = catalogs();
        let t = tadpole(&c);
        assert_eq!(generator_cost(t, 0.0), 10.0);
        assert_eq!(generator_cost(t, 1.0), 12.0); // ceil(10*1.15)
        assert_eq!(generator_cost(t, 5.0), 21.0); // ceil(10*1.15^5)
    }

    #[test]
    fn bulk_cost_matches_ts() {
        let c = catalogs();
        let t = tadpole(&c);
        // 10 units starting from count 5: ceil(21*(1-1.15^10)/(1-1.15)) = 427
        assert_eq!(bulk_cost(t, 5.0, 10.0), 427.0);
    }

    #[test]
    fn max_affordable_matches_ts() {
        let c = catalogs();
        let t = tadpole(&c);
        // 1000 bufos at count 0 -> floor(log(16)/log(1.15)) = 19
        assert_eq!(max_affordable(t, 0.0, 1000.0), 19.0);
    }

    #[test]
    fn click_earns_click_power() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.click_multiplier = 2.0;
        let out = click(&mut s);
        assert_eq!(out.earned, 2.0);
        assert_eq!(s.resources.bufos, 2.0);
        assert_eq!(s.resources.total_bufos, 2.0);
        assert_eq!(s.resources.click_count, 1.0);
    }

    #[test]
    fn rapid_click_combo_matches_live_client() {
        assert_eq!(click_combo(0.0, 1_000.0, 0), (0, 1.0));
        assert_eq!(click_combo(1_000.0, 1_100.0, 0), (1, 1.05));
        assert_eq!(click_combo(1_000.0, 1_500.0, 8), (0, 1.0));
        assert_eq!(click_combo(1_000.0, 1_100.0, 10), (10, 1.5));

        let c = catalogs();
        let mut s = GameState::default_state(&c);
        let out = click_with_multiplier(&mut s, 1.5);
        assert_eq!(out.earned, 1.5);
        assert_eq!(s.resources.bufos, 1.5);
        assert_eq!(s.resources.click_count, 1.0);
    }

    #[test]
    fn register_click_counts_without_income() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.bufos = 50.0;
        s.resources.total_bufos = 100.0;
        register_click(&mut s);
        // Counts a click toward both counters…
        assert_eq!(s.resources.click_count, 1.0);
        assert_eq!(s.achievements.click_count, 1.0);
        // …but grants no bufo income (boss hits are the fight, not income).
        assert_eq!(s.resources.bufos, 50.0);
        assert_eq!(s.resources.total_bufos, 100.0);
    }

    #[test]
    fn buy_generator_deducts_and_increments() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.bufos = 100.0;
        let out = buy_generator(&mut s, &c, "tadpole", 3.0);
        assert!(out.success);
        // TS bulk formula: ceil(10 * (1 - 1.15^3) / (1 - 1.15)) = 35
        // (continuous geometric sum from the ceiled next-unit cost).
        assert_eq!(out.cost, 35.0);
        assert_eq!(s.generator_count("tadpole"), 3.0);
        assert_eq!(s.resources.bufos, 65.0);
    }

    #[test]
    fn unaffordable_purchase_fails() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.bufos = 5.0;
        let out = buy_generator(&mut s, &c, "tadpole", 1.0);
        assert!(!out.success);
        assert_eq!(s.generator_count("tadpole"), 0.0);
        assert_eq!(s.resources.bufos, 5.0);
    }

    #[test]
    fn locked_generator_cannot_be_bought() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.bufos = 1_000_000.0;
        let out = buy_generator(&mut s, &c, "froglet", 1.0);
        assert!(!out.success);
        assert_eq!(s.generator_count("froglet"), 0.0);
    }

    #[test]
    fn unlock_opens_shop_when_threshold_met() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.resources.total_bufos = 50.0; // froglet requires 50
        let newly = check_unlocks(&mut s, &c);
        assert!(newly.contains(&"froglet".to_string()));
        assert!(s.generators["froglet"].unlocked);
    }

    #[test]
    fn tick_accumulates_production() {
        let c = catalogs();
        let mut s = GameState::default_state(&c);
        s.generators.get_mut("tadpole").unwrap().count = 1.0;
        let before = s.resources.bufos;
        tick(&mut s, &c, 10.0);
        // one tadpole at 0.1/s for 10s = 1.0
        assert!((s.resources.bufos - before - 1.0).abs() < 1e-9);
        assert!((s.resources.total_bufos - 1.0).abs() < 1e-9);
    }

    fn with_tadpole(c: &Catalogs) -> GameState {
        let mut s = GameState::default_state(c);
        s.generators.get_mut("tadpole").unwrap().count = 1.0;
        s
    }

    #[test]
    fn elapsed_twenty_second_tab_return_credits() {
        let c = catalogs();
        let mut s = with_tadpole(&c);
        let out = apply_elapsed_production(&mut s, &c, 1_000_000.0, 1_020_000.0, 0.0)
            .expect("20s tab return should credit");
        assert!((out.capped_production - 2.0).abs() < 1e-9); // 0.1/s * 20s
        assert!((s.resources.bufos - 2.0).abs() < 1e-9);
    }

    #[test]
    fn elapsed_twenty_second_reload_does_not_credit() {
        let c = catalogs();
        let mut s = with_tadpole(&c);
        // One-minute floor on reload.
        let out = apply_elapsed_production(&mut s, &c, 1_000_000.0, 1_020_000.0, 60_000.0);
        assert!(out.is_none());
        assert_eq!(s.resources.bufos, 0.0);
    }

    #[test]
    fn elapsed_over_twelve_hours_is_capped() {
        let c = catalogs();
        let mut s = with_tadpole(&c);
        let out = apply_elapsed_production(&mut s, &c, 0.0, 20.0 * 3600.0 * 1000.0, 0.0)
            .expect("long gap should credit");
        assert!(out.is_capped);
        assert!((out.capped_production - 0.1 * 12.0 * 3600.0).abs() < 1e-6);
    }
}
