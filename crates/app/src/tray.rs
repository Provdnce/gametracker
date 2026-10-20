//! Icon in the notification area of the Windows taskbar (next to the clock).
//! Left click opens the window, right click shows a small menu.

#[cfg(windows)]
mod imp {
    use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    };

    pub enum TrayAction {
        Open,
        ToggleTracking,
        Quit,
    }

    pub struct Tray {
        icon: TrayIcon,
        open: MenuId,
        pause: MenuItem,
        quit: MenuId,
        tooltip: String,
    }

    impl Tray {
        pub fn new() -> Option<Self> {
            let rgba = include_bytes!("../resources/tray-32.rgba").to_vec();
            let image = Icon::from_rgba(rgba, 32, 32).ok()?;
            let open = MenuItem::new("GameTracker öffnen", true, None);
            let pause = MenuItem::new("Tracking pausieren", true, None);
            let quit = MenuItem::new("Beenden", true, None);
            let menu = Menu::new();
            menu.append_items(&[
                &open,
                &PredefinedMenuItem::separator(),
                &pause,
                &PredefinedMenuItem::separator(),
                &quit,
            ])
            .ok()?;
            let icon = TrayIconBuilder::new()
                .with_icon(image)
                .with_tooltip("GameTracker")
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
                .map_err(|e| log::error!("Tray-Icon konnte nicht erstellt werden: {e}"))
                .ok()?;
            Some(Self {
                icon,
                open: open.id().clone(),
                quit: quit.id().clone(),
                pause,
                tooltip: String::new(),
            })
        }

        /// Collects clicks since the last call.
        pub fn poll(&self) -> Vec<TrayAction> {
            let mut out = Vec::new();
            while let Ok(e) = TrayIconEvent::receiver().try_recv() {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = e
                {
                    out.push(TrayAction::Open);
                }
            }
            while let Ok(e) = MenuEvent::receiver().try_recv() {
                if e.id == self.open {
                    out.push(TrayAction::Open);
                } else if &e.id == self.pause.id() {
                    out.push(TrayAction::ToggleTracking);
                } else if e.id == self.quit {
                    out.push(TrayAction::Quit);
                }
            }
            out
        }

        pub fn update(&mut self, tooltip: String, paused: bool) {
            if tooltip != self.tooltip {
                self.icon.set_tooltip(Some(&tooltip)).ok();
                self.tooltip = tooltip;
            }
            self.pause.set_text(if paused {
                "Tracking fortsetzen"
            } else {
                "Tracking pausieren"
            });
        }
    }
}

#[cfg(not(windows))]
mod imp {
    #[allow(dead_code)]
    pub enum TrayAction {
        Open,
        ToggleTracking,
        Quit,
    }

    /// Only Windows has a tray icon for now.
    pub struct Tray;

    impl Tray {
        pub fn new() -> Option<Self> {
            None
        }
        pub fn poll(&self) -> Vec<TrayAction> {
            Vec::new()
        }
        pub fn update(&mut self, _tooltip: String, _paused: bool) {}
    }
}

pub use imp::*;
