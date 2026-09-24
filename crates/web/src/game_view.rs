//! The main game view: click area, resources, production stats, owned
//! generators, shop, and upgrades. Holds the authoritative `GameState` in an
//! `RwSignal` and calls the game crate for every rule.

use game::achievements;
use game::catalog::{AchievementReward, Catalogs};
use game::economy;
use game::number::{format_number, format_number_with_precision};
use game::prestige;
use game::state::GameState;
use game::upgrades;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

fn category_icon(category: &str) -> &'static str {
    match category {
        "generators" => "🏭",
        "production" => "💰",
        "clicks" => "👆",
        "special" => "🎮",
        _ => "🏆",
    }
}

fn reward_text(reward: &AchievementReward) -> String {
    let detail = match reward.kind.as_str() {
        "productionBoost" => format!("{}x production boost", reward.value),
        "clickBoost" => format!("{}x click boost", reward.value),
        "generatorBoost" => format!(
            "{}x boost to {} generator",
            reward.value,
            reward.target.as_deref().unwrap_or("unknown")
        ),
        "bufoBonus" => format!("{} Bufos", reward.value),
        "unlockGenerator" => format!(
            "Unlocked {} generator",
            reward.target.as_deref().unwrap_or("unknown")
        ),
        "unlockUpgrade" => "Unlocked new upgrade".to_string(),
        "unlockFeature" => "Unlocked new feature".to_string(),
        _ => "Special bonus".to_string(),
    };
    format!("Reward: {detail}")
}

#[component]
pub fn GameView(catalogs: Catalogs, initial_state: GameState) -> impl IntoView {
    let state = RwSignal::new(initial_state);
    let amount = RwSignal::new(1.0_f64);
    let console_opened = RwSignal::new(false);
    let storage_error = RwSignal::new(None::<String>);
    let active_modal = RwSignal::new(None::<&'static str>);
    let last_click_ms = RwSignal::new(0.0_f64);
    let click_combo = RwSignal::new(0_u8);

    crate::console_detection::setup_console_detection(state, catalogs.clone(), console_opened);

    // Persist: 60s autosave + save on pagehide / tab-hidden.
    let save_now = {
        let catalogs = catalogs.clone();
        move || match crate::storage::save_v2(&state.get_untracked(), &catalogs) {
            Ok(()) => storage_error.set(None),
            Err(e) => storage_error.set(Some(e)),
        }
    };
    let autosave = gloo_timers::callback::Interval::new(60_000, save_now.clone());
    autosave.forget();
    {
        let window = web_sys::window().expect("window");
        {
            let save_now = save_now.clone();
            let cb = wasm_bindgen::closure::Closure::wrap(Box::new(save_now) as Box<dyn FnMut()>);
            let _ =
                window.add_event_listener_with_callback("pagehide", cb.as_ref().unchecked_ref());
            cb.forget();
        }
        {
            let save_now = save_now.clone();
            let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                if web_sys::window()
                    .map(|w| {
                        w.document()
                            .map(|d| d.visibility_state() == web_sys::VisibilityState::Hidden)
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
                {
                    save_now();
                }
            }) as Box<dyn FnMut()>);
            let _ = window
                .add_event_listener_with_callback("visibilitychange", cb.as_ref().unchecked_ref());
            cb.forget();
        }
    }

    // Wall-clock production tick. Uses the elapsed delta each interval so a
    // throttled background tab still earns its full production, and a tab
    // return credits the whole hidden gap (no one-minute floor, 12h cap).
    let notice = RwSignal::new(None::<String>);
    let tick_state = state;
    let tick_catalogs = catalogs.clone();
    let last_tick = RwSignal::new(js_sys::Date::now());
    let tick_interval = gloo_timers::callback::Interval::new(100, move || {
        let now = js_sys::Date::now();
        let dt_ms = now - last_tick.get_untracked();
        if dt_ms > 0.0 {
            let dt_seconds = (dt_ms / 1000.0).min(12.0 * 3600.0);
            tick_state.update(|s| {
                economy::tick(s, &tick_catalogs, dt_seconds);
                s.game_settings.last_tick = now;
            });
            last_tick.set(now);
        }
        let newly = tick_state
            .try_update(|s| {
                let newly = achievements::check_achievements(
                    s,
                    &tick_catalogs,
                    console_opened.get_untracked(),
                );
                economy::check_unlocks(s, &tick_catalogs);
                newly
            })
            .unwrap_or_default();
        if let Some(id) = newly.first() {
            notice.set(Some(id.clone()));
            gloo_timers::callback::Timeout::new(4_000, move || notice.set(None)).forget();
        }
    });
    tick_interval.forget();

    let total_per_second = Memo::new({
        let catalogs = catalogs.clone();
        move |_| economy::total_production(&state.get(), &catalogs)
    });
    let notice_catalogs = catalogs.clone();
    let click_catalogs = catalogs.clone();

    let reset = {
        let catalogs = catalogs.clone();
        move || match crate::storage::reset(&catalogs) {
            Ok(fresh) => {
                state.set(fresh);
                active_modal.set(None);
                storage_error.set(None);
                notice.set(Some("Game reset successfully.".to_string()));
            }
            Err(e) => storage_error.set(Some(format!("Reset failed: {e}"))),
        }
    };
    let export = {
        let catalogs = catalogs.clone();
        move || match crate::storage::export_save(&state.get_untracked(), &catalogs) {
            Ok(encoded) => {
                let _ = web_sys::window()
                    .expect("window")
                    .prompt_with_message_and_default("Copy your save:", &encoded);
            }
            Err(e) => storage_error.set(Some(format!("Export failed: {e}"))),
        }
    };
    let import = {
        let catalogs = catalogs.clone();
        move || {
            let window = web_sys::window().expect("window");
            if let Ok(Some(encoded)) = window.prompt_with_message("Paste your save:") {
                match crate::storage::import_save(&encoded, &catalogs) {
                    Ok(fresh) => {
                        state.set(fresh);
                        active_modal.set(None);
                        storage_error.set(None);
                    }
                    Err(e) => storage_error.set(Some(format!("Import failed: {e}"))),
                }
            }
        }
    };
    view! {
        <header class="game-header"><h1 class="game-title"></h1></header>
        {move || match storage_error.get() {
            None => crate::util::empty_view(),
            Some(e) => view! {
                <div class="storage-warning">
                    "Saving failed: " {e.clone()} " — progress is in memory only."
                </div>
            }
            .into_any(),
        }}
        {move || match notice.get() {
            None => crate::util::empty_view(),
            Some(id) => match notice_catalogs.achievements.iter().find(|a| a.id == id) {
                None => crate::util::empty_view(),
                Some(a) => view! {
                    <div class="achievement-notification visible">
                        <div class="achievement-notification-icon"><div class="achievement-icon-emoji">{category_icon(&a.category)}</div></div>
                        <div class="achievement-notification-content">
                            <div class="achievement-notification-title">"Achievement Unlocked"</div>
                            <div class="achievement-notification-name">{a.name.clone()}</div>
                            <div class="achievement-notification-description">{a.description.clone()}</div>
                            {a.reward.as_ref().map(|r| view! { <div class="achievement-notification-reward">{reward_text(r)}</div> })}
                        </div>
                    </div>
                }.into_any(),
            },
        }}
        <main class="game-content">
            <div class="three-column-layout">
                <div class="column left-column">
                    <div class="resource-display">
                        <div class="resource-count">
                            <span class="number-value">
                                {move || format_number_with_precision(state.get().resources.bufos.floor())}
                            </span>
                            <span class="number-label">"Bufos"</span>
                        </div>
                        <div class="production-rate">
                            {move || format!("{} bufos/sec", format_number(total_per_second.get(), 1))}
                        </div>
                    </div>
                    <div class="frog-display" on:click=move |event: web_sys::MouseEvent| {
                        let now = js_sys::Date::now();
                        let (next_combo, multiplier) = economy::click_combo(last_click_ms.get_untracked(), now, click_combo.get_untracked());
                        last_click_ms.set(now);
                        click_combo.set(next_combo);
                        let (outcome, newly) = state.try_update(|s| {
                            let outcome = economy::click_with_multiplier(s, multiplier);
                            economy::check_unlocks(s, &click_catalogs);
                            let newly = achievements::check_achievements(s, &click_catalogs, console_opened.get_untracked());
                            (outcome, newly)
                        }).expect("game state is available");
                        crate::click_effects::show(&event, &outcome, next_combo > 0);
                        if let Some(id) = newly.first() {
                            notice.set(Some(id.clone()));
                            gloo_timers::callback::Timeout::new(4_000, move || notice.set(None)).forget();
                        }
                    }>
                        <img src="./assets/images/bufo.png" alt="Bufo" class="bufo-image" />
                    </div>
                    <div class="production-stats panel">
                        <div class="panel-content">
                            <div class="stats-container"></div>
                            <div class="contributions">
                                <h3>"Production Sources"</h3>
                                <div class="contributions-list">
                                    <ProductionSources state=state catalogs=catalogs.clone() />
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="column center-column">
                    <div class="game-menu">
                        <button class="menu-button" on:click=move |_| active_modal.set(Some("stats"))>"Stats"</button>
                        <button class="menu-button" on:click=move |_| active_modal.set(Some("achievements"))>"Achievements"</button>
                        <button class="menu-button menu-button--prestige"
                            hidden=move || game::state::prestige_points_for(state.get().resources.total_bufos) < 1.0
                            on:click=move |_| active_modal.set(Some("prestige"))>"Transcend"</button>
                        <button class="menu-button save-button" title="Right-click to export or import a save"
                            on:click=move |_| save_now()
                            on:contextmenu=move |event: web_sys::MouseEvent| {
                                event.prevent_default();
                                active_modal.set(Some("transfer"));
                            }>"Save"</button>
                        <button class="menu-button" on:click=move |_| active_modal.set(Some("reset"))>"Reset"</button>
                    </div>
                    <OwnedGenerators state=state catalogs=catalogs.clone() />
                </div>

                <div class="column right-column">
                    <UpgradesPanel state=state catalogs=catalogs.clone() />
                    <Shop state=state catalogs=catalogs.clone() amount=amount />
                </div>
            </div>
        </main>
        <crate::boss_ui::BossUi state=state catalogs=catalogs.clone() />
        <crate::golden_ui::GoldenUi state=state catalogs=catalogs.clone() />
        <div class="modal-container">
            <div class="modal" class:visible=move || active_modal.get() == Some("stats")>
                <div class="modal-content">
                    <div class="modal-header"><h2>"Game Statistics"</h2><button class="modal-close" on:click=move |_| active_modal.set(None)>"×"</button></div>
                    <div class="modal-body"><StatsPanel state=state catalogs=catalogs.clone() /></div>
                </div>
            </div>
            <div class="modal" class:visible=move || active_modal.get() == Some("transfer")>
                <div class="modal-content">
                    <div class="modal-header"><h2>"Transfer Save"</h2><button class="modal-close" on:click=move |_| active_modal.set(None)>"×"</button></div>
                    <div class="modal-body"><p>"Copy a backup or restore one from another site."</p></div>
                    <div class="modal-footer">
                        <button class="modal-button" on:click=move |_| export()>"Export save"</button>
                        <button class="modal-button" on:click=move |_| import()>"Import save"</button>
                    </div>
                </div>
            </div>
            <div class="modal" class:visible=move || active_modal.get() == Some("achievements")>
                <div class="modal-content">
                    <div class="modal-header"><h2>"Achievements"</h2><button class="modal-close" on:click=move |_| active_modal.set(None)>"×"</button></div>
                    <div class="modal-body"><AchievementsPanel state=state catalogs=catalogs.clone() /></div>
                </div>
            </div>
            <div class="modal" class:visible=move || active_modal.get() == Some("prestige")>
                <div class="modal-content">
                    <div class="modal-header"><h2>"Transcendence Bufoplier"</h2><button class="modal-close" on:click=move |_| active_modal.set(None)>"×"</button></div>
                    <div class="modal-body"><PrestigePanel state=state catalogs=catalogs.clone() active_modal=active_modal /></div>
                </div>
            </div>
            <div class="modal" class:visible=move || active_modal.get() == Some("reset")>
                <div class="modal-content">
                    <div class="modal-header"><h2>"Reset Game"</h2><button class="modal-close" on:click=move |_| active_modal.set(None)>"×"</button></div>
                    <div class="modal-body"><p>"Are you sure you want to reset your game?"</p><p>"All progress will be lost!"</p></div>
                    <div class="modal-footer">
                        <button class="modal-button cancel-button" on:click=move |_| active_modal.set(None)>"Cancel"</button>
                        <button class="modal-button confirm-button" on:click=move |_| reset()>"Reset Game"</button>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn ProductionSources(state: RwSignal<GameState>, catalogs: Catalogs) -> impl IntoView {
    view! {
        {move || {
            let s = state.get();
            let total = economy::total_production(&s, &catalogs);
            let owned: Vec<_> = catalogs.generators.iter()
                .filter(|d| s.generator_count(&d.id) > 0.0)
                .collect();
            if owned.is_empty() {
                view! { <div class="empty-contributions">"No production sources yet."</div> }.into_any()
            } else {
                view! {
                    {owned.into_iter().map(|d| {
                        let count = s.generator_count(&d.id);
                        let production = economy::generator_total_production(d, &s.generators[&d.id], &s);
                        let share = if total > 0.0 { production / total * 100.0 } else { 0.0 };
                        view! {
                            <div class="contribution-item">
                                <span class="contribution-name">{format!("{} (x{count:.0})", d.name)}</span>
                                <span class="contribution-value">{format!("{}/sec ({share:.1}%)", format_number(production, 1))}</span>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                }.into_any()
            }
        }}
    }
}

#[component]
fn AchievementsPanel(state: RwSignal<GameState>, catalogs: Catalogs) -> impl IntoView {
    let catalogs_count = catalogs.achievements.len();
    view! {
        <div class="achievements-container">
            <div class="achievement-progress-bar">
                <div class="achievement-progress-text">
                    {move || {
                        let count = state.get().achievements.unlocked.len();
                        format!("{count} of {catalogs_count} Achievements Unlocked ({}%)", if catalogs_count > 0 { count * 100 / catalogs_count } else { 0 })
                    }}
                </div>
                <div class="achievement-progress-outer">
                    <div class="achievement-progress-inner" style=move || {
                        let count = state.get().achievements.unlocked.len();
                        format!("width: {}%", if catalogs_count > 0 { count * 100 / catalogs_count } else { 0 })
                    }></div>
                </div>
            </div>
            <div class="achievements-list">
                {move || {
                    let catalogs = catalogs.clone();
                    let s = state.get();
                    let unlocked: Vec<_> = catalogs.achievements.iter()
                        .filter(|a| s.achievements.unlocked.iter().any(|u| u == &a.id))
                        .collect();
                    if unlocked.is_empty() {
                        view! { <div class="empty-achievements"><p>"No achievements unlocked yet."</p><p>"Keep playing to unlock achievements!"</p></div> }.into_any()
                    } else {
                        view! {
                            {unlocked.into_iter().map(|a| {
                                let category = format!("achievement-item category-{}", a.category);
                                let name = a.name.clone();
                                let icon = category_icon(&a.category);
                                view! {
                                    <div class=category>
                                        <div class="achievement-icon">
                                            <div class="achievement-icon-emoji">{icon}</div>
                                        </div>
                                        <div class="achievement-info">
                                            <div class="achievement-name">{name}</div>
                                            <div class="achievement-description">{a.description.clone()}</div>
                                            {a.reward.as_ref().map(|r| view! { <div class="achievement-reward">{reward_text(r)}</div> })}
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn StatsPanel(state: RwSignal<GameState>, catalogs: Catalogs) -> impl IntoView {
    let catalogs_for_stats = catalogs.clone();
    view! {
        <div class="stats-modal-content">
            <div class="stats-container">
                {move || {
                    let s = state.get();
                    let total_generators: f64 = s.generators.values().map(|g| g.count).sum();
                    let played = s.game_settings.first_start_time
                        .map(|t| (js_sys::Date::now() - t).max(0.0) / 1000.0)
                        .unwrap_or(0.0);
                    let achievement_count = s.achievements.unlocked.len();
                    let total_achievements = catalogs_for_stats.achievements.len();
                    let unlocked_generators = s.generators.values().filter(|g| g.unlocked).count();
                    let production = economy::total_production(&s, &catalogs_for_stats);
                    view! {
                        <h3 class="stats-section-title">"Resources"</h3>
                        <div class="stats-row">
                            <div class="stat-item"><div class="stat-label">"Total Bufos Produced"</div><div class="stat-value">{format_number(s.resources.total_bufos, 1)}</div></div>
                            <div class="stat-item"><div class="stat-label">"Current Bufos"</div><div class="stat-value">{format_number(s.resources.bufos, 1)}</div></div>
                        </div>
                        <div class="stats-row">
                            <div class="stat-item"><div class="stat-label">"Total Bufos Spent"</div><div class="stat-value">{format_number((s.resources.total_bufos - s.resources.bufos).max(0.0), 1)}</div></div>
                            <div class="stat-item"><div class="stat-label">"Current Production"</div><div class="stat-value">{format!("{}/sec", format_number(production, 1))}</div></div>
                        </div>
                        <h3 class="stats-section-title">"Game Progress"</h3>
                        <div class="stats-row">
                            <div class="stat-item"><div class="stat-label">"Time Played"</div><div class="stat-value">{game::number::format_duration(played)}</div></div>
                            <div class="stat-item"><div class="stat-label">"Total Clicks"</div><div class="stat-value">{format_number(s.resources.click_count, 1)}</div></div>
                        </div>
                        <div class="stats-row">
                            <div class="stat-item">
                                <div class="stat-label">"Achievements"</div>
                                <div class="stat-value">{format!("{achievement_count}/{total_achievements} ({}%)", if total_achievements > 0 { achievement_count * 100 / total_achievements } else { 0 })}</div>
                                <div class="achievement-progress-indicator"><div class="achievement-progress-bar" style=format!("width: {}%", if total_achievements > 0 { achievement_count * 100 / total_achievements } else { 0 })></div></div>
                            </div>
                            <div class="stat-item"><div class="stat-label">"Upgrades Purchased"</div><div class="stat-value">{s.upgrades.purchased.len()}</div></div>
                        </div>
                        <div class="stats-row">
                            <div class="stat-item"><div class="stat-label">"Generators Unlocked"</div><div class="stat-value">{format!("{unlocked_generators}/{}", catalogs_for_stats.generators.len())}</div></div>
                            <div class="stat-item"><div class="stat-label">"Total Generators Owned"</div><div class="stat-value">{format_number(total_generators, 1)}</div></div>
                        </div>
                        <h3 class="stats-section-title">"Production Details"</h3>
                        <div class="stats-row production-details">
                            <div class="stat-item"><div class="stat-label">"Per Second"</div><div class="stat-value">{format_number(production, 1)}</div></div>
                            <div class="stat-item"><div class="stat-label">"Per Minute"</div><div class="stat-value">{format_number(production * 60.0, 1)}</div></div>
                            <div class="stat-item"><div class="stat-label">"Per Hour"</div><div class="stat-value">{format_number(production * 3600.0, 1)}</div></div>
                        </div>
                    }
                }}
            </div>
            <div class="contributions-container"><h3 class="stats-section-title">"Production Sources"</h3><div class="contributions-list"><ProductionSources state=state catalogs=catalogs.clone() /></div></div>
        </div>
    }
}

#[component]
fn PrestigePanel(
    state: RwSignal<GameState>,
    catalogs: Catalogs,
    active_modal: RwSignal<Option<&'static str>>,
) -> impl IntoView {
    let transcend = move || {
        if game::state::prestige_points_for(state.get_untracked().resources.total_bufos) < 1.0 {
            return;
        }
        state.update(|s| {
            prestige::transcend(s, &catalogs);
            achievements::check_achievements(s, &catalogs, false);
        });
        if let Err(e) = crate::storage::save_v2(&state.get_untracked(), &catalogs) {
            let _ = web_sys::window()
                .expect("window")
                .alert_with_message(&format!("Saving failed: {e}"));
        }
        active_modal.set(None);
    };

    view! {
        <div class="prestige-modal">
            <p class="prestige-modal__blurb">
                "Transcend to fold this run into the " <strong>"Transcendence Bufoplier"</strong>
                ". Your bufos, generators and upgrades reset, but every Bufoplier point permanently multiplies "
                <em>"all"</em> " bufo production and click power by +10%."
            </p>
            <div class="prestige-modal__stats">
                {move || {
                    let s = state.get();
                    let pending = game::state::prestige_points_for(s.resources.total_bufos);
                    let next = 1.0 + (s.prestige.lifetime_points + pending) * 0.1;
                    view! {
                        <div><span>"Bufoplier points"</span><strong>{format!("{:.0}", s.prestige.lifetime_points)}</strong></div>
                        <div><span>"Times transcended"</span><strong>{format!("{:.0}", s.prestige.transcendences)}</strong></div>
                        <div><span>"Current multiplier"</span><strong>{format!("x{:.2}", game::state::prestige_multiplier(&s))}</strong></div>
                        <div class="is-gain"><span>"Points if you transcend now"</span><strong>{format!("+{pending:.0}")}</strong></div>
                        <div class="is-gain"><span>"New multiplier"</span><strong>{format!("x{next:.2}")}</strong></div>
                    }
                }}
            </div>
            <div class="modal-footer">
                <button class="modal-button cancel-button" on:click=move |_| active_modal.set(None)>"Not yet"</button>
                <button class="modal-button confirm-button"
                    disabled=move || game::state::prestige_points_for(state.get().resources.total_bufos) < 1.0
                    on:click=move |_| transcend()>
                    {move || format!("Transcend for +{:.0}", game::state::prestige_points_for(state.get().resources.total_bufos))}
                </button>
            </div>
        </div>
    }
}

#[component]
fn OwnedGenerators(state: RwSignal<GameState>, catalogs: Catalogs) -> impl IntoView {
    let empty_catalogs = catalogs.clone();
    let title_catalogs = catalogs.clone();
    view! {
        <div class="owned-generators-container panel">
            <div class="panel-header"><h2>{move || {
                let s = state.get();
                let total: f64 = title_catalogs.generators.iter()
                    .map(|d| s.generator_count(&d.id))
                    .sum();
                if total > 0.0 {
                    format!("Your Frogs ({total:.0})")
                } else {
                    "Your Frogs".to_string()
                }
            }}</h2></div>
            <div class="panel-content generators-container">
                {move || if empty_catalogs.generators.iter().any(|d| state.get().generator_count(&d.id) > 0.0) {
                    crate::util::empty_view()
                } else {
                    view! { <div class="empty-generators">"No frogs yet! Buy some from the shop."</div> }.into_any()
                }}
            </div>
            {move || {
                let s = state.get();
                catalogs.generators.iter()
                    .filter(|d| s.generator_count(&d.id) > 0.0)
                    .map(|d| {
                                let count = s.generator_count(&d.id);
                                let per_unit = economy::production_per_unit(d, &s.generators[&d.id], &s);
                                let icon_path = d.icon_path.clone();
                                let name = d.name.clone();
                                view! {
                                    <div id=format!("generator-{}", d.id) class=format!("owned-generator category-{}", d.category)>
                                        <div class="generator-icon">
                                            <div class="generator-icon-wrapper">
                                                {match icon_path {
                                                    Some(path) => view! { <img class="generator-icon-img" src=path alt=name.clone() /> }.into_any(),
                                                    None => view! { <div class="generator-icon-fallback">"🐸"</div> }.into_any(),
                                                }}
                                            </div>
                                        </div>
                                        <div class="generator-info">
                                            <div class="name-count-container">
                                                <div class="generator-name">{name}</div>
                                                <div class="generator-count">{format!("x{count:.0}")}</div>
                                            </div>
                                            <div class="generator-production">
                                                <span class="production-value">{format_number(per_unit * count, 1)}</span>"/sec"
                                            </div>
                                        </div>
                                    </div>
                                }
                    }).collect::<Vec<_>>()
            }}
        </div>
    }
}

#[component]
fn UpgradesPanel(state: RwSignal<GameState>, catalogs: Catalogs) -> impl IntoView {
    #[derive(Clone)]
    struct Tooltip {
        name: String,
        category: String,
        description: String,
        flavor: String,
        cost: f64,
        x: f64,
        y: f64,
    }

    let tooltip = RwSignal::new(None::<Tooltip>);
    let failed_purchase = RwSignal::new(None::<String>);
    let indicator_catalogs = catalogs.clone();
    let header_catalogs = catalogs.clone();
    view! {
        <div class="upgrades-panel panel" class:has-new-upgrades=move || {
            !upgrades::available_upgrades(&state.get(), &indicator_catalogs).is_empty()
        }>
            <div class="panel-header"><h2>{
                move || {
                    let count = upgrades::available_upgrades(&state.get(), &header_catalogs).len();
                    if count == 0 { "Upgrades".to_string() } else { format!("Upgrades ({count})") }
                }
            }</h2></div>
            <div class="panel-content">
                <div class="upgrades-grid">
                    {move || {
                        let catalogs = catalogs.clone();
                        let s = state.get();
                        let available = upgrades::available_upgrades(&s, &catalogs);
                        if available.is_empty() {
                            view! { <div class="empty-upgrades">"No upgrades available yet."</div> }.into_any()
                        } else {
                            view! {
                                {available.into_iter().map(|u| {
                                    let catalogs = catalogs.clone();
                                    let id = u.id.clone();
                                    let name = u.name.clone();
                                    let description = u.description.clone();
                                    let cost = u.cost;
                                    let affordable = s.resources.bufos >= cost;
                                    let icon_path = u.icon_path.clone();
                                    let category = u.category.clone();
                                    let tooltip_data = Tooltip {
                                        name: name.clone(),
                                        category: category.clone(),
                                        description,
                                        flavor: u.flavor_text.clone().unwrap_or_default(),
                                        cost,
                                        x: 0.0,
                                        y: 0.0,
                                    };
                                    let failed_id = id.clone();
                                    view! {
                                        <div class="upgrade-icon-container"
                                            class:affordable=affordable
                                            class:not-affordable=!affordable
                                            class:purchase-error=move || failed_purchase.get().as_deref() == Some(failed_id.as_str())
                                            data-category=category
                                            on:mouseenter=move |ev: web_sys::MouseEvent| {
                                                let Some(element) = ev.current_target().and_then(|e| e.dyn_into::<web_sys::Element>().ok()) else { return; };
                                                let rect = element.get_bounding_client_rect();
                                                let mut data = tooltip_data.clone();
                                                data.x = rect.left() + rect.width() / 2.0;
                                                data.y = rect.bottom() + 10.0;
                                                tooltip.set(Some(data));
                                            }
                                            on:mouseleave=move |_| tooltip.set(None)>
                                            <button class="upgrade-icon"
                                                aria-label=name.clone()
                                                on:click=move |_| {
                                                    let outcome = state.try_update(|s| upgrades::buy_upgrade(s, &catalogs, &id));
                                                    if outcome.as_ref().is_some_and(|o| o.success) {
                                                        tooltip.set(None);
                                                    } else {
                                                        failed_purchase.set(Some(id.clone()));
                                                        gloo_timers::callback::Timeout::new(500, move || failed_purchase.set(None)).forget();
                                                    }
                                                }>
                                                {match icon_path {
                                                    Some(path) => view! { <img class="upgrade-icon-img" src=path alt=name.clone() /> }.into_any(),
                                                    None => view! { <div class="upgrade-icon-emoji">"✨"</div> }.into_any(),
                                                }}
                                            </button>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        </div>
        {move || match tooltip.get() {
            None => crate::util::empty_view(),
            Some(data) => {
                let category_label = match data.category.as_str() {
                    "click" => "Click Upgrade",
                    "generator" => "Generator Upgrade",
                    "global" => "Global Upgrade",
                    _ => "Upgrade",
                };
                view! {
                    <div class="game-tooltip visible" style=format!("left: {}px; top: {}px; transform: translateX(-50%)", data.x, data.y)>
                        <div class=format!("tooltip-upgrade tooltip-category-{}", data.category)>
                            <div class="tooltip-header"><span class="tooltip-title">{data.name}</span><span class="tooltip-category">{category_label}</span></div>
                            <div class="tooltip-description">{data.description}</div>
                            <div class="tooltip-flavor">{data.flavor}</div>
                            <div class="tooltip-cost">{format!("{} bufos", format_number(data.cost, 1))}</div>
                        </div>
                    </div>
                }.into_any()
            }
        }}
    }
}

#[component]
fn Shop(state: RwSignal<GameState>, catalogs: Catalogs, amount: RwSignal<f64>) -> impl IntoView {
    let amounts = [("1", 1.0), ("10", 10.0), ("100", 100.0), ("Max", -1.0)];

    view! {
        <div class="shop-panel panel">
            <div class="panel-header"><h2>"Frog Shop"</h2></div>
            <div class="panel-content">
                <div class="purchase-controls" aria-hidden="true"></div>
                <div class="purchase-controls">
                    <div class="purchase-amount-buttons">
                        {amounts.into_iter().map(|(label, value)| {
                            view! {
                                <button
                                    class="purchase-amount-button"
                                    class:active=move || amount.get() == value
                                    on:click=move |_| amount.set(value)
                                >
                                    {label}
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
                <div class="buildings-container">
                    {move || {
                        let catalogs = catalogs.clone();
                        let s = state.get();
                        let unlocked: Vec<_> = catalogs.generators.iter()
                            .filter(|d| s.generators.get(&d.id).map(|g| g.unlocked).unwrap_or(false))
                            .collect();
                        view! {
                            {unlocked.into_iter().map(|d| {
                                let catalogs = catalogs.clone();
                                let id = d.id.clone();
                                let name = d.name.clone();
                                let tooltip = d.detailed_description.clone().unwrap_or_else(|| d.description.clone());
                                let count = s.generator_count(&d.id);
                                let cost = economy::generator_cost(d, count);
                                let per_unit = economy::production_per_unit(d, &s.generators[&d.id], &s);
                                let quantity = if amount.get() < 0.0 {
                                    economy::max_affordable(d, count, s.resources.bufos)
                                } else { amount.get() };
                                let purchase_cost = if quantity < 1.0 { cost } else { economy::bulk_cost(d, count, quantity) };
                                let affordable = quantity >= 1.0 && s.resources.bufos >= purchase_cost;
                                let icon_path = d.icon_path.clone();
                                let category = format!("building-item category-{}", d.category);
                                view! {
                                    <div class=category title=tooltip.clone()>
                                        <div class="generator-row">
                                            <div class="generator-left">
                                                <div class="generator-icon">
                                                    <div class="generator-icon-wrapper">
                                                        {match icon_path {
                                                            Some(path) => view! { <img class="generator-icon-img" src=path alt=name.clone() /> }.into_any(),
                                                            None => view! { <div class="generator-icon-fallback">"🐸"</div> }.into_any(),
                                                        }}
                                                    </div>
                                                </div>
                                                <div class="generator-info">
                                                    <div class="generator-name-section">
                                                        <span class="generator-name">{name.clone()}</span>
                                                    </div>
                                                    <div class="generator-production">
                                                        {format!("{}/sec per unit", format_number(per_unit, 2))}
                                                    </div>
                                                </div>
                                            </div>
                                            <button
                                                class="buy-button"
                                                class:disabled=move || !affordable
                                                on:click=move |_| {
                                                    state.update(|s| {
                                                        economy::buy_generator(s, &catalogs, &id, amount.get());
                                                    });
                                                }
                                            >
                                                {format!("{} bufos", format_number(purchase_cost, 1))}
                                            </button>
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
