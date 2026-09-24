//! Golden Bufo UI: random spawns, a clickable sprite, reward application, and
//! frenzy countdown badges. Random values and wall-clock time come from the
//! browser; the rules live in the game crate.

use game::achievements;
use game::catalog::Catalogs;
use game::economy;
use game::golden::{self, RewardType};
use game::state::GameState;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone)]
struct Spawn {
    id: u64,
    reward: RewardType,
    x_pct: f64,
    y_pct: f64,
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn schedule_next(
    spawn: RwSignal<Option<Spawn>>,
    next_id: RwSignal<u64>,
    first_done: RwSignal<bool>,
) {
    let delay = if first_done.get_untracked() {
        golden::SPAWN_MIN_MS
            + js_sys::Math::random() * (golden::SPAWN_MAX_MS - golden::SPAWN_MIN_MS)
    } else {
        golden::FIRST_SPAWN_MIN_MS
            + js_sys::Math::random() * (golden::FIRST_SPAWN_MAX_MS - golden::FIRST_SPAWN_MIN_MS)
    };
    gloo_timers::callback::Timeout::new(delay as u32, move || {
        if spawn.get_untracked().is_some() {
            schedule_next(spawn, next_id, first_done);
            return;
        }
        first_done.set(true);
        let (x_pct, y_pct) = golden::spawn_position(js_sys::Math::random(), js_sys::Math::random());
        let id = next_id.get_untracked();
        next_id.update(|n| *n += 1);
        spawn.set(Some(Spawn {
            id,
            reward: golden::roll_reward(js_sys::Math::random()),
            x_pct,
            y_pct,
        }));
        gloo_timers::callback::Timeout::new(golden::ON_SCREEN_TTL_MS as u32, move || {
            if spawn.get_untracked().as_ref().map(|s| s.id) == Some(id) {
                spawn.set(None);
                schedule_next(spawn, next_id, first_done);
            }
        })
        .forget();
    })
    .forget();
}

#[component]
pub fn GoldenUi(state: RwSignal<GameState>, catalogs: Catalogs) -> impl IntoView {
    let spawn = RwSignal::new(None::<Spawn>);
    let next_id = RwSignal::new(1_u64);
    let first_done = RwSignal::new(false);
    let production_frenzy_ends = RwSignal::new(0.0_f64);
    let click_frenzy_ends = RwSignal::new(0.0_f64);
    let notice = RwSignal::new(None::<(String, String, String)>);
    let now_tick = RwSignal::new(now_ms());

    // Initial spawn schedule.
    schedule_next(spawn, next_id, first_done);

    // Frenzy countdown: clear expired frenzies on a 250ms tick.
    let frenzy_state = state;
    let frenzy_tick = gloo_timers::callback::Interval::new(250, move || {
        let now = now_ms();
        now_tick.set(now);
        if production_frenzy_ends.get_untracked() > 0.0
            && production_frenzy_ends.get_untracked() <= now
        {
            production_frenzy_ends.set(0.0);
            frenzy_state.update(|s| s.resources.frenzy_production_multiplier = 1.0);
        }
        if click_frenzy_ends.get_untracked() > 0.0 && click_frenzy_ends.get_untracked() <= now {
            click_frenzy_ends.set(0.0);
            frenzy_state.update(|s| s.resources.frenzy_click_multiplier = 1.0);
        }
    });
    frenzy_tick.forget();

    // Buffs don't persist while the game is paused: end any active frenzy the
    // moment the tab is hidden.
    {
        let frenzy_state = state;
        let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            let hidden = web_sys::window()
                .and_then(|w| w.document())
                .map(|d| d.visibility_state() == web_sys::VisibilityState::Hidden)
                .unwrap_or(false);
            if hidden {
                production_frenzy_ends.set(0.0);
                click_frenzy_ends.set(0.0);
                frenzy_state.update(|s| {
                    s.resources.frenzy_production_multiplier = 1.0;
                    s.resources.frenzy_click_multiplier = 1.0;
                });
            }
        }) as Box<dyn FnMut()>);
        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("visibilitychange", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    }

    view! {
        <div class="golden-bufo-layer">
            {move || match spawn.get() {
                None => crate::util::empty_view(),
                Some(s) => {
                    let catalogs = catalogs.clone();
                    let x = s.x_pct;
                    let y = s.y_pct;
                    view! {
                        <button class="golden-bufo"
                            aria-label="Golden Bufo — click me!"
                            style=move || format!("left: {x}vw; top: {y}vh; --ttl: {}ms", golden::ON_SCREEN_TTL_MS)
                            on:click=move |_| {
                                spawn.set(None);
                                let per_second = economy::total_production(&state.get_untracked(), &catalogs);
                                let applied = state
                                    .try_update(|st| golden::apply_reward(st, s.reward, now_ms(), per_second))
                                    .expect("state signal");
                                match s.reward {
                                    RewardType::BufoFrenzy => production_frenzy_ends.set(applied.frenzy_ends_at),
                                    RewardType::ClickFrenzy => click_frenzy_ends.set(applied.frenzy_ends_at),
                                    RewardType::Lucky => {}
                                }
                                state.update(|st| { achievements::trigger_custom_event(st, &catalogs, "golden_bufo_caught"); });
                                let kind = match s.reward {
                                    RewardType::BufoFrenzy => "bufo_frenzy",
                                    RewardType::ClickFrenzy => "click_frenzy",
                                    RewardType::Lucky => "lucky",
                                };
                                notice.set(Some((kind.to_string(), applied.label, applied.detail)));
                                gloo_timers::callback::Timeout::new(3_000, move || notice.set(None)).forget();
                                schedule_next(spawn, next_id, first_done);
                            }>
                            <img src="./assets/images/generators/bufo-has-midas-touch.png" alt="Golden Bufo" draggable="false" />
                        </button>
                    }.into_any()
                }
            }}
            {move || match notice.get() {
                None => crate::util::empty_view(),
                Some((kind, label, detail)) => view! {
                    <div class=format!("golden-bufo-toast golden-bufo-toast--{kind} is-visible")>
                        <span class="golden-bufo-toast__label">{label}</span>
                        <span class="golden-bufo-toast__detail">{detail}</span>
                    </div>
                }.into_any(),
            }}
        </div>
        <div class="frenzy-indicator-layer">
            {move || {
                let now = now_tick.get();
                let end = production_frenzy_ends.get();
                if end > now {
                    let remaining = end - now;
                    view! {
                        <div class="frenzy-badge frenzy-badge--production">
                            <span class="frenzy-badge__label">"Bufo Frenzy x7"</span>
                            <span class="frenzy-badge__time">{format!("{:.1}s", remaining / 1000.0)}</span>
                            <span class="frenzy-badge__bar"><span class="frenzy-badge__bar-fill" style=format!("width: {}%", (remaining / golden::BUFO_FRENZY_MS * 100.0).clamp(0.0, 100.0))></span></span>
                        </div>
                    }.into_any()
                } else { crate::util::empty_view() }
            }}
            {move || {
                let now = now_tick.get();
                let end = click_frenzy_ends.get();
                if end > now {
                    let remaining = end - now;
                    view! {
                        <div class="frenzy-badge frenzy-badge--click">
                            <span class="frenzy-badge__label">"Click Frenzy x7"</span>
                            <span class="frenzy-badge__time">{format!("{:.1}s", remaining / 1000.0)}</span>
                            <span class="frenzy-badge__bar"><span class="frenzy-badge__bar-fill" style=format!("width: {}%", (remaining / golden::CLICK_FRENZY_MS * 100.0).clamp(0.0, 100.0))></span></span>
                        </div>
                    }.into_any()
                } else { crate::util::empty_view() }
            }}
        </div>
    }
}
