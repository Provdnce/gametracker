// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod instance;
mod pages;
mod theme;
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
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx: &mut App| {
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

            // Another start of the exe asks us to show the window.
            cx.spawn(async move |cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(400))
                        .await;
                    if rx.try_recv().is_ok() {
                        cx.update(show_window);
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
