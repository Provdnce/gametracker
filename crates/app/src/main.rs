// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod instance;
mod pages;
mod theme;
mod tray;
mod view;

use gametracker_core::{Shared, Tracker, autostart};
use gpui::{
    App, Bounds, Global, KeyBinding, QuitMode, TitlebarOptions, WindowBounds, WindowHandle,
    WindowOptions, actions, prelude::*, px, size,
};
use gpui_elements::editable_text::actions::{DEFAULT_INPUT_CONTEXT, default_bindings};
use std::sync::Arc;
use std::time::Duration;
use view::MainView;

actions!(gametracker, [Quit, CloseWindow]);

/// App-wide state: the tracker and the (optional) open window.
pub struct AppState {
    pub shared: Arc<Shared>,
    pub window: Option<WindowHandle<MainView>>,
}

impl Global for AppState {}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let rx = match instance::acquire() {
        instance::Instance::Primary(rx) => rx,
        instance::Instance::AlreadyRunning => {
            log::info!("GameTracker läuft bereits – Fenster wird angezeigt.");
            return;
        }
    };

    let data_dir = gametracker_core::data_dir();
    let shared = match Tracker::start(&data_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("GameTracker konnte nicht starten: {e:#}");
            std::process::exit(1);
        }
    };
    let start_hidden = std::env::args().any(|a| a == autostart::HIDDEN_FLAG);

    gpui_platform::application()
        .with_assets(assets::Assets)
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx: &mut App| {
            if let Err(e) = cx.text_system().add_fonts(assets::fonts()) {
                log::error!("Schriften konnten nicht geladen werden: {e:#}");
            }
            cx.bind_keys(default_bindings().as_keybindings(Some(DEFAULT_INPUT_CONTEXT)));
            cx.bind_keys([
                KeyBinding::new("ctrl-q", Quit, None),
                KeyBinding::new("cmd-q", Quit, None),
            ]);
            cx.bind_keys([
                KeyBinding::new("ctrl-w", CloseWindow, None),
                KeyBinding::new("cmd-w", CloseWindow, None),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.on_action(|_: &CloseWindow, cx| {
                log::info!("Fenster wird geschlossen, Tracking läuft weiter.");
                // Deferred: while the action runs, the window is busy and can't be updated.
                if let Some(handle) = cx.global::<AppState>().window {
                    cx.defer(move |cx| {
                        handle
                            .update(cx, |_, window, _| window.remove_window())
                            .ok();
                    });
                }
            });

            cx.set_global(AppState {
                shared: shared.clone(),
                window: None,
            });

            let quit_shared = shared.clone();
            cx.on_app_quit(move |_| {
                Tracker::shutdown(&quit_shared);
                async {}
            })
            .detach();

            cx.on_window_closed(|cx, _| {
                log::info!("Fenster geschlossen");
                cx.global_mut::<AppState>().window = None;
                let keep = cx.global::<AppState>().shared.settings().run_in_background;
                if !keep && cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            if !start_hidden {
                show_window(cx);
            }

            // Polls for: another start of the exe, and clicks on the tray icon.
            let mut tray = tray::Tray::new();
            cx.spawn(async move |cx| {
                let mut ticks = 0u32;
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(250))
                        .await;
                    if rx.try_recv().is_ok() {
                        cx.update(show_window);
                    }
                    let Some(tray) = tray.as_mut() else { continue };
                    for action in tray.poll() {
                        cx.update(|cx| match action {
                            tray::TrayAction::Open => show_window(cx),
                            tray::TrayAction::Quit => cx.quit(),
                            tray::TrayAction::ToggleTracking => {
                                let shared = cx.global::<AppState>().shared.clone();
                                let on =
                                    shared.settings().track_games || shared.settings().track_keys;
                                shared
                                    .update_settings(|s| {
                                        s.track_games = !on;
                                        s.track_keys = !on;
                                    })
                                    .ok();
                            }
                        });
                    }
                    ticks += 1;
                    if ticks % 8 == 0 {
                        let (tooltip, paused) = cx.update(|cx| {
                            let shared = &cx.global::<AppState>().shared;
                            let s = shared.settings();
                            let paused = !s.track_games && !s.track_keys;
                            let tip = match shared.active_games().first() {
                                _ if paused => "GameTracker – pausiert".to_string(),
                                Some(g) => format!("GameTracker – spielt {}", g.name),
                                None => "GameTracker".to_string(),
                            };
                            (tip, paused)
                        });
                        tray.update(tooltip, paused);
                    }
                }
            })
            .detach();
        });
}

pub fn show_window(cx: &mut App) {
    if let Some(handle) = cx.global::<AppState>().window
        && handle.update(cx, |_, window, _| window.activate()).is_ok()
    {
        return;
    }
    let shared = cx.global::<AppState>().shared.clone();
    let bounds = Bounds::centered(None, size(px(1280.), px(840.)), cx);
    let handle = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("GameTracker".into()),
                ..Default::default()
            }),
            window_min_size: Some(size(px(900.), px(600.))),
            app_id: Some("gametracker".into()),
            ..Default::default()
        },
        |window, cx| cx.new(|cx| MainView::new(shared, window, cx)),
    );
    match handle {
        Ok(h) => {
            cx.global_mut::<AppState>().window = Some(h);
            cx.activate(true);
        }
        Err(e) => log::error!("Fenster konnte nicht geöffnet werden: {e:#}"),
    }
}
