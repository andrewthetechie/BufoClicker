# Parity matrix — TypeScript → Rust

Every player-facing flow in the TypeScript client, mapped to its source of
truth and the Rust test that proves the port matches it. "Native" = a
`#[test]` in `crates/game` (no browser). "Browser" = a manual checklist step
(see `safari-run-record.md`); browser behavior is verified manually by the
owner, per the goal contract.

## Game rules (native tests)

| # | Flow | TS source | Rust test |
|---|------|-----------|-----------|
| 1 | Click earns bufos (base click power) | `src/game/gameCore.ts` `click()` | `economy::tests::click_earns_click_power` |
| 2 | Click count canonical counter (click + boss hits) | `src/game/gameCore.ts` `registerClick()` | `economy::tests::register_click_counts_without_income` |
| 3 | Generator unit cost `count==0 → baseCost`, else `ceil(baseCost·r^count)` | `src/models/generators.ts` `recalculateGenerator()` | `economy::tests::cost_formula_matches_ts` |
| 4 | Bulk cost `ceil(a·(1−rⁿ)/(1−r))` for 1/10/100 | `src/models/generators.ts` `calculateBulkCost()` | `economy::tests::bulk_cost_matches_ts` |
| 5 | Max affordable `floor(log(bufos·(r−1)/(a·r^owned)+1)/log(r))` | `src/models/generators.ts` `calculateMaxAffordable()` | `economy::tests::max_affordable_matches_ts` |
| 6 | Generator production `baseProduction·boostMult·globalMult` | `src/models/generators.ts` `recalculateGenerator()` | `economy::tests::buy_generator_deducts_and_increments` |
| 7 | Total production = Σ generator `totalProduction` | `src/models/generators.ts` `calculateTotalProduction()` | `economy::tests::tick_accumulates_production` |
| 8 | Generator unlock (bufos / generators / achievement / special) | `src/models/generators.ts` `checkGeneratorUnlock()` | `economy::tests::unlock_opens_shop_when_threshold_met` |
| 9 | Unaffordable purchase returns failure (button stays clickable) | `src/managers/generatorManager.ts` `purchaseGenerator()` | `economy::tests::unaffordable_purchase_fails` |
| 10 | Locked generator cannot be bought | `src/managers/generatorManager.ts` | `economy::tests::locked_generator_cannot_be_bought` |
| 11 | Upgrade availability + prerequisite conditions | `src/models/upgrades.ts` `meetsUnlockConditions()` | `upgrades::tests::stronger_clicks_requires_total_bufos`, `prerequisite_upgrade_gate_is_enforced` |
| 12 | Upgrade effects: `clickMultiplier`, `generatorProduction`, `globalMultiplier` | `src/managers/upgradeManager.ts` `applySingleEffect()` | `upgrades::tests::click_upgrade_doubles_click_power` |
| 13 | Unknown effect type is rejected (never silently passes) | `src/managers/upgradeManager.ts` `applySingleEffect()` default | `catalog::tests::unknown_effect_type_is_rejected` |
| 14 | Achievement requirement / progress / reward-once | `src/managers/achievementManager.ts` | `achievements::tests::first_bufo_unlocks_once`, `click_count_achievement` |
| 15 | `clickBoost` achievement reward multiplies `clickMultiplier` once | `src/managers/achievementManager.ts` | `achievements::tests::first_bufo_unlocks_once` |
| 16 | `console_opened` viewport/keyboard predicate | `src/managers/achievementManager.ts` + browser bindings | `achievements::tests::console_shortcut_predicate`, `console_size_predicate` |
| 17 | Prestige points `floor(sqrt(totalBufos/1e9))`, min 1e9 | `src/models/prestige.ts` `prestigePointsFor()` | `state::tests::prestige_points_curve` |
| 18 | Prestige multiplier `1 + lifetimePoints·0.10` | `src/models/prestige.ts` `getPrestigeMultiplier()` | `state::tests::prestige_multiplier_uses_lifetime` |
| 19 | Transcend: wipe run, keep prestige + boss rewards | `src/managers/prestigeManager.ts` `transcend()` | `prestige::tests::transcend_awards_points_and_resets_run`, `below_threshold_is_ignored` |
| 20 | Boss eligibility (in-order, threshold-gated) | `src/models/boss.ts` `getAvailableBoss()` | `boss::tests::available_boss_is_in_order_and_threshold_gated` |
| 21 | Boss health `ceil(baseHealth·prestigeMult·bossMult)` | `src/models/boss.ts` `getBossHealth()` | `boss::tests::boss_health_scales_with_prestige_and_boss_bonus` |
| 22 | Boss multiplier `1 + (defeated.length + lifetimeDefeats)·0.25` | `src/models/boss.ts` `getBossMultiplier()` | `state::tests::boss_multiplier_adds_defeats_and_lifetime` |
| 23 | Boss win/loss/retreat: loss clears bufos, keeps gens/upgrades/prestige; retreat is penalty-free | `src/managers/bossManager.ts` | `boss::tests::win_records_defeat_lose_clears_bufos`, `boss::tests::retreat_leaves_state_unchanged` |
| 24 | Golden Bufo spawn/collect/lucky/frenzy (seeded RNG + time) | `src/managers/goldenBufoManager.ts` | `golden::tests::reward_roll_thresholds`, `spawn_position_stays_in_bounds`, `lucky_gain_formula`, `frenzy_reward_sets_multiplier_and_expiry`, `lucky_reward_grants_bufos`; achievement flag: `achievements::tests::golden_bufo_caught_unlocks_golden_first` |
| 25 | Frenzy multipliers are transient (not saved) | `src/managers/goldenBufoManager.ts` | `save::tests::v2_roundtrip_preserves_canonical_fields` (frenzy reset in `migrate`) |
| 26 | Elapsed production: 1-min floor on reload, none on tab return, 12h cap | `src/game/gameSave.ts` `applyElapsedProduction()` | `economy::tests::elapsed_twenty_second_*`, `elapsed_over_twelve_hours_is_capped` |
| 27 | Number formatting: `formatNumber` / `formatNumberWithPrecision` (en-US, K/M/B/T…) | `src/utils/numberUtils.ts` | `number::tests::format_number_reference_values`, `format_precision_reference_values`, `non_finite_is_zero` |
| 28 | Click power `baseClickPower·clickMult·prestige·boss·frenzy` | `src/utils/stateUtils.ts` `calculateDerivedState()` | `state::tests::click_power_combines_all_sources` |

## Persistence (native tests)

| # | Flow | TS source | Rust test |
|---|------|-----------|-----------|
| 29 | Legacy envelope: nested `state` authoritative; `bufo_idle_save` | `src/utils/saveManager.ts` | `save::tests::legacy_fixtures_parse_and_validate` |
| 30 | Legacy → v2 migration, field preservation | `src/game/gameSave.ts` `loadGame()` | `save::tests::migration_reapplies_upgrade_and_achievement_effects_once` |
| 31 | v2 round-trip (no canonical field changes) | (new) | `save::tests::v2_roundtrip_preserves_canonical_fields` |
| 32 | v2 precedence over legacy; legacy key untouched | (new) | `save::tests::v2_takes_precedence_over_legacy`, `legacy_falls_back_when_no_v2` |
| 33 | Corrupt v2 / failed migration → recovery state, writes blocked | (new) | `save::tests::corrupt_v2_is_an_error_not_a_fallback`, `malformed_and_corrupt_are_rejected`, `save::tests::restore_legacy_migrates_and_writes_v2`, `save::tests::restore_legacy_fails_without_legacy_save` |
| 34 | Reset ordering: validate+serialize+setItem before state commit | (new) | `save::tests::reset_writes_v2_and_leaves_legacy_untouched`, `save::tests::failed_reset_write_leaves_both_keys_unchanged` |
| 35 | Export/import codec `btoa(encodeURIComponent(JSON))`, parser by `schema_version` | `src/utils/saveManager.ts` | `save::tests::parse_any_selects_v2_and_legacy`, `codec::tests::encoded_export_fixture_decodes_and_roundtrips`, `codec::tests::v2_export_roundtrips_through_codec` |
| 36 | Failed import leaves both stored keys unchanged | (new) | `save::tests::failed_import_leaves_both_keys_unchanged` |
| 37 | Explorer/RPG field accepted but behavior not restored | `src/game/gameSave.ts` | `save::tests::missing_fields_fixture_migrates_with_defaults` |

## UI behavior (manual browser checks — owner)

Control parity (contract item 20): the Rust UI ports the non-persistence
controls — click, shop 1/10/100/Max, upgrades (with hover tooltips), the
achievements view with a category filter, prestige confirm, boss fight overlay
with a win/loss result modal that survives stray clicks (input-locked
800ms), Golden Bufo with frenzy countdown, statistics (time played, clicks,
production sources), manual save, reset, export/import, and the recovery
screen (retry / import backup / restore legacy / reset). Native
`window.confirm`/`alert`/`prompt` are used for simple one-line prompts; the
boss result and recovery surfaces are real modals.

| # | Flow | Reference |
|---|------|-----------|
| U1 | Click bufo → counter + click effects (no clipping near edges) | `src/ui/components/clickArea.ts` |
| U2 | Shop purchase 1/10/100/Max + shake/flash feedback, no reflow | `src/ui/components/shopItem.ts` |
| U3 | Upgrade cards + unaffordable feedback | `src/ui/components/upgradeItem.ts` |
| U4 | Achievements view + notifications | `src/managers/UIManager.ts` |
| U5 | Prestige confirm modal | `src/ui/components/*` |
| U6 | Boss fight overlay intercepts clicks; result modal survives stray clicks | `src/ui/components/bossFight.ts` |
| U7 | Golden Bufo badge + frenzy countdown | `src/ui/components/goldenBufo.ts` |
| U8 | Statistics / reset / export / import / menus | `src/managers/UIManager.ts` |
| U9 | Mobile: no horizontal scroll, ≥44px targets, touch-action | `styles/mobile.css` |
| U10 | Root `/` and `/BufoClicker/` load (HTML/WASM/CSS/images/JSON) | Trunk public URL |

## Reference values (source-derived)

Source functions (pre-port line refs, from `git show main:`):
`src/models/generators.ts` `recalculateGenerator` (L233), `calculateBulkCost`
(L381), `calculateMaxAffordable` (L401); `src/models/prestige.ts`
`prestigePointsFor` (L29), `getPrestigeMultiplier` (L40); `src/models/boss.ts`
`getBossMultiplier` (L147), `getBossHealth` (L185); `src/utils/numberUtils.ts`
`formatNumber` (L48), `formatNumberWithPrecision` (L90);
`src/utils/stateUtils.ts` `calculateDerivedState` (L163).

| Value | Result |
|-------|--------|
| tadpole cost, count 0 | 10 |
| tadpole cost, count 1 | `ceil(10·1.15)` = 12 |
| tadpole cost, count 5 | `ceil(10·1.15⁵)` = 21 |
| froglet cost, count 1 | `ceil(100·1.2)` = 120 |
| tadpole bulk cost, 10 @ count 5 | `ceil(21·(1−1.15¹⁰)/(1−1.15))` = 427 |
| tadpole max affordable, 1000 bufos @ count 0 | `floor(log(16)/log(1.15))` = 19 |
| `prestigePointsFor(999_999_999)` | 0 |
| `prestigePointsFor(1e9)` | 1 |
| `prestigePointsFor(1e11)` | 10 |
| `getPrestigeMultiplier(lifetimePoints=5)` | 1.5 |
| `getBossMultiplier(defeated=2, lifetime=3)` | 1 + 5·0.25 = 2.25 |
| `getBossHealth(furious_froglet, prestige=1, boss=1)` | `ceil(975)` = 975 |
| `formatNumber(999)` | "999" |
| `formatNumber(1000)` | "1,000" |
| `formatNumber(1_234_567)` | "1.2M" |
| `formatNumberWithPrecision(1_234_567)` | "1,234,567" |
| `formatNumberWithPrecision(1.5e12)` | "1.500T" |
| click power (base 1, clickMult 4, prestige 1.5, boss 1, frenzy 1) | 6 |
