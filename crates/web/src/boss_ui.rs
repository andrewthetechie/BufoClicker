//! Boss fight UI: banner (available boss + Fight), full-screen fight overlay
//! (HP bar, countdown, click-to-hit, win/loss/retreat), and a win/loss result
//! modal that survives the stray clicks already in flight. Boss hits count
//! toward click achievements but grant no bufo income; the countdown pauses
//! while the tab is hidden.

use game::boss;
use game::catalog::Catalogs;
use game::economy;
use game::state::{boss_multiplier, click_power, GameState};
use game::{achievements, number::format_number};
use leptos::prelude::*;

#[derive(Clone)]
struct ActiveFight {
    boss_id: String,
    name: String,
    icon_path: String,
    health: f64,
    max_health: f64,
    remaining_ms: f64,
    sprite_x: f64,
    sprite_y: f64,
    move_in_ms: f64,
}

#[derive(Clone)]
struct BossResult {
    won: bool,
    name: String,
    icon_path: String,
    multiplier: f64,
}

fn is_hidden() -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .map(|d| d.visibility_state() == web_sys::VisibilityState::Hidden)
        .unwrap_or(false)
}

#[component]
pub fn BossUi(state: RwSignal<GameState>, catalogs: Catalogs) -> impl IntoView {
    let fight = RwSignal::new(None::<ActiveFight>);
    let result = RwSignal::new(None::<BossResult>);
    let locked_until = RwSignal::new(0.0_f64);
    let snoozed_until = RwSignal::new(0.0_f64);

    Effect::new(move || {
        let stage = state.get().bosses.defeated.len();
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            let _ = body.set_attribute("data-boss-stage", &stage.to_string());
            if fight.get().is_some() {
                let _ = body.class_list().add_1("boss-fight-active");
            } else {
                let _ = body.class_list().remove_1("boss-fight-active");
            }
        }
    });

    let available = move || {
        if fight.get().is_some()
            || result.get().is_some()
            || js_sys::Date::now() < snoozed_until.get()
        {
            return None;
        }
        let s = state.get();
        boss::get_available_boss(&s.bosses.defeated, s.resources.total_bufos)
    };

    // Countdown timer while a fight is active. Pauses while the tab is hidden
    // (a fight is opt-in and never runs without the player watching).
    let timer_state = state;
    let timer_fight = fight;
    let timer_result = result;
    let timer_locked = locked_until;
    let fight_interval = gloo_timers::callback::Interval::new(100, move || {
        if is_hidden() {
            return;
        }
        let Some(mut f) = timer_fight.get_untracked() else {
            return;
        };
        f.remaining_ms -= 100.0;
        f.move_in_ms -= 100.0;
        if f.move_in_ms <= 0.0 {
            f.sprite_x = 0.1 + js_sys::Math::random() * 0.75;
            f.sprite_y = 0.15 + js_sys::Math::random() * 0.65;
            f.move_in_ms = 3_000.0;
        }
        if f.remaining_ms <= 0.0 {
            timer_fight.set(None);
            timer_state.update(|s| {
                boss::lose_fight(s);
            });
            timer_result.set(Some(BossResult {
                won: false,
                name: f.name.clone(),
                icon_path: f.icon_path.clone(),
                multiplier: 1.0,
            }));
            timer_locked.set(js_sys::Date::now() + 800.0);
            gloo_timers::callback::Timeout::new(800, move || timer_locked.set(0.0)).forget();
        } else {
            timer_fight.set(Some(f));
        }
    });
    fight_interval.forget();

    view! {
        <div class="boss-layer">
                {move || match available() {
                    None => crate::util::empty_view(),
                    Some(b) => {
                        let id = b.id.to_string();
                        let name = b.name.to_string();
                        let icon = b.icon_path.to_string();
                        let max_health = {
                            let s = state.get();
                            boss::get_boss_health(b, &s)
                        };
                        view! {
                            <div class="boss-banner">
                                <img class="boss-banner__portrait" src=icon.clone() alt=name.clone() />
                                <div class="boss-banner__body">
                                    <div class="boss-banner__title">"A Boss Has Appeared!"</div>
                                    <div class="boss-banner__name">{name.clone()}</div>
                                    <div class="boss-banner__flavor">{b.flavor_text.to_string()}</div>
                                </div>
                                <div class="boss-banner__actions">
                                    <button class="boss-banner__fight" on:click=move |_| {
                                        fight.set(Some(ActiveFight {
                                            boss_id: id.clone(),
                                            name: name.clone(),
                                            icon_path: icon.clone(),
                                            health: max_health,
                                            max_health,
                                            remaining_ms: boss::BOSS_FIGHT_DURATION_MS,
                                            sprite_x: 0.1 + js_sys::Math::random() * 0.75,
                                            sprite_y: 0.15 + js_sys::Math::random() * 0.65,
                                            move_in_ms: 3_000.0,
                                        }));
                                    }>
                                        {format!("Fight! ({} HP, 30s)", format_number(max_health, 1))}
                                    </button>
                                    <button class="boss-banner__later" on:click=move |_| {
                                        snoozed_until.set(js_sys::Date::now() + 60_000.0 + js_sys::Math::random() * 120_000.0);
                                    }>"Not yet"</button>
                                </div>
                            </div>
                        }.into_any()
                    }
                }}
        </div>

        {move || match fight.get() {
            None => crate::util::empty_view(),
            Some(f) => {
                let catalogs = catalogs.clone();
                let boss_id = f.boss_id.clone();
                let name = f.name.clone();
                let icon = f.icon_path.clone();
                let health = f.health;
                let max_health = f.max_health;
                let remaining = f.remaining_ms;
                let sprite_x = f.sprite_x;
                let sprite_y = f.sprite_y;
                view! {
                    <div class="boss-fight-overlay">
                        <div class="boss-fight-hud">
                            <div class="boss-fight-hud__name">{name.clone()}</div>
                            <div class="boss-health-bar">
                                <div class="boss-health-bar__fill" style=move || {
                                    let pct = if max_health > 0.0 { (health / max_health * 100.0).clamp(0.0, 100.0) } else { 0.0 };
                                    format!("width: {pct}%")
                                }></div>
                                <div class="boss-health-bar__text">{format!("{} / {} HP", format_number(health, 1), format_number(max_health, 1))}</div>
                            </div>
                            <div class="boss-fight-hud__timer" class:boss-fight-hud__timer--urgent=remaining <= 10_000.0>
                                {format!("{:.1}s", (remaining / 1000.0).max(0.0))}
                            </div>
                            <button class="boss-fight-hud__retreat" on:click=move |_| {
                                fight.set(None);
                                state.update(|s| { boss::retreat_fight(s); });
                            }>"Retreat"</button>
                        </div>
                        <button
                                class="boss-sprite"
                                style=format!("left: {}vw; top: {}vh", sprite_x * 100.0, sprite_y * 100.0)
                                aria-label=format!("Click {name}")
                                on:click=move |_| {
                                    let damage = click_power(&state.get_untracked());
                                    let mut f = fight.get_untracked().expect("fight");
                                    f.health = (f.health - damage).max(0.0);
                                    state.update(|s| {
                                        economy::register_click(s);
                                    });
                                    if f.health <= 0.0 {
                                        fight.set(None);
                                        state.update(|s| {
                                            boss::win_fight(s, &boss_id);
                                        });
                                        state.update(|s| {
                                            achievements::trigger_custom_event(s, &catalogs, &format!("boss_{boss_id}"));
                                        });
                                        result.set(Some(BossResult {
                                            won: true,
                                            name: name.clone(),
                                            icon_path: icon.clone(),
                                            multiplier: boss_multiplier(&state.get_untracked()),
                                        }));
                                        locked_until.set(js_sys::Date::now() + 800.0);
                                        gloo_timers::callback::Timeout::new(800, move || locked_until.set(0.0)).forget();
                                    } else {
                                        fight.set(Some(f));
                                    }
                            }>
                                <img src=icon.clone() alt=name.clone() draggable="false" />
                            </button>
                    </div>
                }.into_any()
            }
        }}

        {move || match result.get() {
            None => crate::util::empty_view(),
            Some(r) => {
                let won = r.won;
                let name = r.name.clone();
                let icon_path = r.icon_path.clone();
                let multiplier = r.multiplier;
                let title = if won { format!("{name} Defeated!") } else { format!("Defeated by {name}...") };
                view! {
                    <div class=move || {
                        if locked_until.get() > js_sys::Date::now() {
                            "modal visible modal--input-locked".to_string()
                        } else {
                            "modal visible".to_string()
                        }
                    }>
                        <div class="modal-content">
                            <div class="modal-header">
                                <h2>{title}</h2>
                                <button class="modal-close" on:click=move |_| result.set(None)>"×"</button>
                            </div>
                            <div class="modal-body">
                                <div class=if won { "boss-result boss-result--win" } else { "boss-result boss-result--lose" }>
                                    <img class="boss-result__portrait" src=icon_path alt=name.clone() />
                                    {if won {
                                        view! {
                                            <p>"Your bufos will remember this croak for generations."</p>
                                            <p class="boss-result__area">"The world around you has changed - you've entered a new area."</p>
                                            <p class="boss-result__reward">"Permanent multiplier is now "<strong>{format!("x{multiplier:.2}")}</strong>" to all bufo production and click power."</p>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <p>"Your bufos scatter, and your bufo stash resets to 0."</p>
                                            <p class="boss-result__hint">"Your generators, upgrades and prestige are untouched - buy some more click upgrades and try again whenever you're ready."</p>
                                        }.into_any()
                                    }}
                                </div>
                            </div>
                            <div class="modal-footer">
                                <button class=if won { "modal-button confirm-button" } else { "modal-button cancel-button" } on:click=move |_| result.set(None)>
                                    {if won { "Nice!" } else { "Try again later" }}
                                </button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            }
        }}
    }
}
