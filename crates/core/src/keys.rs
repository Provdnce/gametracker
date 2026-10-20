//! Global keyboard listener.
//!
//! Privacy: we only ever count *which physical key* was pressed on *which day*.
//! No order, no timing, no text is stored, so typed text (passwords, chats)
//! can't be reconstructed from the data.

use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Holding a key makes the OS send repeated key-down events. We only want to
/// count real presses, so repeats are filtered out here.
#[derive(Default)]
pub struct RepeatFilter {
    keys: HashMap<String, KeyState>,
}

struct KeyState {
    down: bool,
    /// When the current physical press started
    pressed_at: Instant,
    /// Last press/release event
    last: Instant,
}

/// Longest gap between auto-repeat events we expect (initial repeat delay is at most ~1s).
const REPEAT_WINDOW: Duration = Duration::from_millis(1200);
/// X11 sends release+press pairs (at the same moment) for auto-repeat.
const X11_REPEAT_GAP: Duration = Duration::from_millis(15);
/// Auto-repeat only kicks in after the key was held at least this long.
const MIN_REPEAT_DELAY: Duration = Duration::from_millis(200);

impl RepeatFilter {
    /// Returns true if this key-down is a new press that should be counted.
    pub fn press(&mut self, id: &str, now: Instant) -> bool {
        if let Some(k) = self.keys.get_mut(id) {
            let since_last = now.saturating_duration_since(k.last);
            // Windows/macOS style: key-downs without a key-up in between
            let held_repeat = k.down && since_last < REPEAT_WINDOW;
            // X11 style: release + press right after each other, after a long hold
            let x11_repeat = !k.down
                && since_last <= X11_REPEAT_GAP
                && k.last.saturating_duration_since(k.pressed_at) >= MIN_REPEAT_DELAY;
            if held_repeat || x11_repeat {
                k.down = true;
                k.last = now;
                return false;
            }
        }
        self.keys.insert(
            id.to_string(),
            KeyState {
                down: true,
                pressed_at: now,
                last: now,
            },
        );
        true
    }

    pub fn release(&mut self, id: &str, now: Instant) {
        match self.keys.get_mut(id) {
            Some(k) => {
                k.down = false;
                k.last = now;
            }
            None => {
                self.keys.insert(
                    id.to_string(),
                    KeyState {
                        down: false,
                        pressed_at: now,
                        last: now,
                    },
                );
            }
        }
    }
}

/// Starts the global keyboard listener on its own thread.
/// `on_press` is called once per real key press with the key id (see [`crate::keymap`]).
/// `on_error` is called if the listener can't start (e.g. Wayland without X11).
pub fn spawn_listener(
    on_press: impl Fn(&str) + Send + Sync + 'static,
    on_error: impl Fn(String) + Send + 'static,
) -> Result<()> {
    platform::spawn(Box::new(on_press), Box::new(on_error))
}

type PressFn = Box<dyn Fn(&str) + Send + Sync>;
type ErrorFn = Box<dyn Fn(String) + Send>;

#[cfg(not(windows))]
mod platform {
    use super::*;

    pub fn spawn(on_press: PressFn, on_error: ErrorFn) -> Result<()> {
        std::thread::Builder::new().name("key-listener".into()).spawn(move || {
            let mut filter = RepeatFilter::default();
            let result = rdev::listen(move |event| match event.event_type {
                rdev::EventType::KeyPress(key) => {
                    let id = key_id(key);
                    if filter.press(&id, Instant::now()) {
                        on_press(&id);
                    }
                }
                rdev::EventType::KeyRelease(key) => filter.release(&key_id(key), Instant::now()),
                _ => {}
            });
            if let Err(e) = result {
                on_error(format!(
                    "Tastatur-Listener konnte nicht starten ({e:?}). Unter Linux wird X11 benötigt, \
                     unter macOS die Bedienungshilfen-Berechtigung."
                ));
            }
        })?;
        Ok(())
    }

    fn key_id(key: rdev::Key) -> String {
        // rdev's variant names are exactly our key ids.
        format!("{key:?}")
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use crate::keymap::from_windows_scancode;
    use std::sync::{Mutex, OnceLock};
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_EXTENDED,
        LLKHF_INJECTED, MSG, SetWindowsHookExW, TranslateMessage, WH_KEYBOARD_LL, WM_KEYDOWN,
        WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    struct HookState {
        on_press: PressFn,
        filter: Mutex<RepeatFilter>,
    }

    static STATE: OnceLock<HookState> = OnceLock::new();

    pub fn spawn(on_press: PressFn, on_error: ErrorFn) -> Result<()> {
        if STATE
            .set(HookState {
                on_press,
                filter: Mutex::new(RepeatFilter::default()),
            })
            .is_err()
        {
            anyhow::bail!("Tastatur-Listener läuft bereits");
        }
        std::thread::Builder::new()
            .name("key-listener".into())
            .spawn(move || unsafe {
                let module = GetModuleHandleW(std::ptr::null());
                let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0);
                if hook.is_null() {
                    on_error("Tastatur-Hook konnte nicht installiert werden.".into());
                    return;
                }
                // A low-level hook needs a message loop on the thread that installed it.
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            })?;
        Ok(())
    }

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            if let Some(state) = STATE.get() {
                let kb = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
                let injected = kb.flags & LLKHF_INJECTED != 0;
                // AltGr sends a fake Ctrl with bit 0x200 set in the scan code: skip it.
                let fake = kb.scanCode & 0x200 != 0;
                if !injected && !fake {
                    let ext = kb.flags & LLKHF_EXTENDED != 0;
                    let id = from_windows_scancode(kb.scanCode & 0xFF, ext, kb.vkCode)
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("VK_{:02X}", kb.vkCode));
                    let now = Instant::now();
                    match wparam as u32 {
                        WM_KEYDOWN | WM_SYSKEYDOWN => {
                            let new_press = state
                                .filter
                                .lock()
                                .map(|mut f| f.press(&id, now))
                                .unwrap_or(false);
                            if new_press {
                                (state.on_press)(&id);
                            }
                        }
                        WM_KEYUP | WM_SYSKEYUP => {
                            if let Ok(mut f) = state.filter.lock() {
                                f.release(&id, now);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeats_are_not_counted() {
        let mut f = RepeatFilter::default();
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        assert!(f.press("KeyW", t0));
        // Windows-style repeat: key-downs without release
        assert!(!f.press("KeyW", ms(500)));
        assert!(!f.press("KeyW", ms(530)));
        f.release("KeyW", ms(600));
        assert!(f.press("KeyW", ms(700)));
        // X11-style repeat after holding: release + press at the same moment
        f.release("KeyW", ms(1000));
        assert!(!f.press("KeyW", ms(1000)));
        f.release("KeyW", ms(1030));
        assert!(!f.press("KeyW", ms(1031)));
        // A missed key-up doesn't block the key forever
        assert!(f.press("KeyW", ms(5000)));
        // Different keys are independent
        assert!(f.press("Space", ms(5000)));
    }

    #[test]
    fn fast_taps_are_counted() {
        // Events can arrive in bursts, so even very short gaps must count
        // as long as the key wasn't held.
        let mut f = RepeatFilter::default();
        let t0 = Instant::now();
        let mut n = 0;
        for i in 0..5u64 {
            let t = t0 + Duration::from_millis(i * 2);
            if f.press("KeyW", t) {
                n += 1;
            }
            f.release("KeyW", t + Duration::from_millis(1));
        }
        assert_eq!(n, 5);
    }
}
