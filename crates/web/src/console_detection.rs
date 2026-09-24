//! Console-open detection (best-effort browser signal) wired to the pure
//! predicates in the game crate. Keyboard shortcuts and a docked-devtools
//! size-gap check both mark the console opened and unlock anything gated on it.

use game::achievements;
use game::catalog::Catalogs;
use game::state::GameState;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

pub fn setup_console_detection(
    state: RwSignal<GameState>,
    catalogs: Catalogs,
    console_opened: RwSignal<bool>,
) {
    let window = web_sys::window().expect("no window");

    let mark = move || {
        if console_opened.get_untracked() {
            return;
        }
        console_opened.set(true);
        let catalogs = catalogs.clone();
        state.update(|s| {
            achievements::check_achievements(s, &catalogs, true);
        });
    };

    // Keyboard shortcuts (F12, or ctrl/meta+shift+i/j/c).
    {
        let mark = mark.clone();
        let cb = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if achievements::console_shortcut_detects(
                &event.key(),
                event.ctrl_key(),
                event.meta_key(),
                event.shift_key(),
            ) {
                mark();
            }
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        let _ = window.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        cb.forget();
    }

    // Docked devtools shrink the viewport without shrinking the window.
    let check_size = {
        let window = window.clone();
        move || {
            if console_opened.get_untracked() {
                return;
            }
            let outer_w = window
                .outer_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let inner_w = window
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let outer_h = window
                .outer_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let inner_h = window
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if achievements::console_size_detects(outer_w, inner_w, outer_h, inner_h, 160.0) {
                mark();
            }
        }
    };

    {
        let check_size = check_size.clone();
        let cb = Closure::wrap(Box::new(check_size) as Box<dyn FnMut()>);
        let _ = window.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
        cb.forget();
    }
    {
        let cb = Closure::wrap(Box::new(check_size) as Box<dyn FnMut()>);
        let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            2000,
        );
        cb.forget();
    }
}
