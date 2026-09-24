use game::catalog::Catalogs;
use game::state::GameState;
use leptos::prelude::*;
use leptos::task::spawn_local;

mod boss_ui;
mod click_effects;
mod console_detection;
mod fetch;
mod game_view;
mod golden_ui;
mod storage;
mod util;

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
enum LoadState {
    Loading,
    Error(String),
    Ready {
        catalogs: Catalogs,
        state: GameState,
    },
    Recovery {
        catalogs: Catalogs,
        message: String,
    },
}

fn main() {
    console_error_panic_hook::set_once();
    // The static shell is useful until WASM starts, but must not remain above
    // the mounted game or its loading and recovery screens.
    if let Some(loading) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("loading"))
    {
        loading.remove();
    }
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let (state, set_state) = signal(LoadState::Loading);

    let load = move || {
        set_state.set(LoadState::Loading);
        spawn_local(async move {
            match fetch::load_catalogs().await {
                Ok(catalogs) => {
                    let outcome = storage::load_game(&catalogs);
                    match outcome {
                        storage::LoadOutcome::Ready(game) => {
                            set_state.set(LoadState::Ready {
                                catalogs,
                                state: game,
                            });
                        }
                        storage::LoadOutcome::Recovery(message) => {
                            set_state.set(LoadState::Recovery { catalogs, message });
                        }
                    }
                }
                Err(e) => set_state.set(LoadState::Error(e.to_string())),
            }
        });
    };

    // Initial load.
    load();

    view! {
        <div class="game-container">
            {move || match state.get() {
                LoadState::Loading => view! {
                    <div class="loading-screen">"Loading game content…"</div>
                }
                .into_any(),
                LoadState::Error(msg) => view! {
                    <div class="error-screen">
                        <p>{msg.clone()}</p>
                        <button on:click=move |_| load()>"Retry"</button>
                    </div>
                }
                .into_any(),
                LoadState::Ready { catalogs, state: game } => view! {
                    <game_view::GameView catalogs=catalogs initial_state=game />
                }
                .into_any(),
                LoadState::Recovery { catalogs, message } => {
                    let catalogs_for_reset = catalogs.clone();
                    let catalogs_for_import = catalogs.clone();
                    let catalogs_for_legacy = catalogs.clone();
                    view! {
                        <div class="error-screen recovery-screen">
                            <p>"Your save could not be loaded."</p>
                            <p class="recovery-message">{message.clone()}</p>
                            <button on:click=move |_| load()>"Retry"</button>
                            <button on:click=move |_| {
                                let window = web_sys::window().expect("window");
                                if let Ok(Some(s)) = window.prompt_with_message("Paste your backup save:") {
                                    match storage::import_save(&s, &catalogs_for_import) {
                                        Ok(fresh) => set_state.set(LoadState::Ready {
                                            catalogs: catalogs_for_import.clone(),
                                            state: fresh,
                                        }),
                                        Err(e) => {
                                            let _ = window.alert_with_message(&format!("Import failed: {e}"));
                                        }
                                    }
                                }
                            }>
                                "Import backup"
                            </button>
                            <button on:click=move |_| {
                                let window = web_sys::window().expect("window");
                                match storage::restore_legacy(&catalogs_for_legacy) {
                                    Ok(fresh) => set_state.set(LoadState::Ready {
                                        catalogs: catalogs_for_legacy.clone(),
                                        state: fresh,
                                    }),
                                    Err(e) => {
                                        let _ = window.alert_with_message(&format!("Restore failed: {e}"));
                                    }
                                }
                            }>
                                "Restore from old save"
                            </button>
                            <button on:click=move |_| {
                                match storage::reset(&catalogs_for_reset) {
                                    Ok(fresh) => set_state.set(LoadState::Ready {
                                        catalogs: catalogs_for_reset.clone(),
                                        state: fresh,
                                    }),
                                    Err(e) => {
                                        let _ = web_sys::window()
                                            .expect("window")
                                            .alert_with_message(&format!("Reset failed: {e}"));
                                    }
                                }
                            }>
                                "Reset (start fresh)"
                            </button>
                        </div>
                    }
                    .into_any()
                }
            }}
        </div>
    }
}
