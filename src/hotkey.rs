use std::collections::HashMap;

use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use strum::IntoEnumIterator;

use crate::action::Action;

const CTRL_ALT: Modifiers = Modifiers::CONTROL.union(Modifiers::ALT);

pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    actions: HashMap<u32, Action>,
}

impl Hotkeys {
    pub fn new() -> Option<Hotkeys> {
        let manager = GlobalHotKeyManager::new()
            .inspect_err(|err| eprintln!("lattice: failed to create hotkey manager: {err}"))
            .ok()?;
        Some(Hotkeys {
            manager,
            actions: HashMap::new(),
        })
    }

    pub fn bind_defaults(&mut self) -> Vec<String> {
        let mut failures = Vec::new();
        for action in Action::iter() {
            let hotkey = default_hotkey(action);
            match self.manager.register(hotkey) {
                Ok(()) => {
                    self.actions.insert(hotkey.id(), action);
                }
                Err(err) => {
                    eprintln!(
                        "lattice: could not register {} for {action}: {err}",
                        action.default_binding()
                    );
                    failures.push(action.label().to_string());
                }
            }
        }
        failures
    }

    pub fn action(&self, hotkey_id: u32) -> Option<Action> {
        self.actions.get(&hotkey_id).copied()
    }
}

fn default_hotkey(action: Action) -> HotKey {
    let (modifiers, code) = match action {
        Action::LeftHalf => (CTRL_ALT, Code::ArrowLeft),
        Action::RightHalf => (CTRL_ALT, Code::ArrowRight),
        Action::TopHalf => (CTRL_ALT, Code::ArrowUp),
        Action::BottomHalf => (CTRL_ALT, Code::ArrowDown),
        Action::TopLeftQuarter => (CTRL_ALT, Code::KeyU),
        Action::TopRightQuarter => (CTRL_ALT, Code::KeyI),
        Action::BottomLeftQuarter => (CTRL_ALT, Code::KeyJ),
        Action::BottomRightQuarter => (CTRL_ALT, Code::KeyK),
        Action::Maximize => (CTRL_ALT, Code::Enter),
    };
    HotKey::new(Some(modifiers), code)
}
