use std::collections::HashMap;
use std::str::FromStr;

use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};

use crate::action::Action;
use crate::config::{Config, KeyCombo};

pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    registered: Vec<HotKey>,
    actions: HashMap<u32, Action>,
}

impl Hotkeys {
    pub fn new() -> Option<Hotkeys> {
        match GlobalHotKeyManager::new() {
            Ok(manager) => Some(Hotkeys {
                manager,
                registered: Vec::new(),
                actions: HashMap::new(),
            }),
            Err(err) => {
                eprintln!("lattice: failed to create hotkey manager: {err}");
                None
            }
        }
    }

    pub fn bind(&mut self, config: &Config) -> Vec<String> {
        if let Err(err) = self.manager.unregister_all(&self.registered) {
            eprintln!("lattice: failed to unregister previous hotkeys: {err}");
        }
        self.registered.clear();
        self.actions.clear();

        let mut failures = Vec::new();
        for (action, combo) in &config.bindings {
            let hotkey = HotKey::from(*combo);
            match self.manager.register(hotkey) {
                Ok(()) => {
                    self.actions.insert(hotkey.id(), *action);
                    self.registered.push(hotkey);
                }
                Err(err) => {
                    eprintln!(
                        "lattice: could not register {combo} for {}: {err}",
                        action.config_key()
                    );
                    failures.push(format!("{} ({combo})", action.label()));
                }
            }
        }
        failures
    }

    pub fn action(&self, hotkey_id: u32) -> Option<Action> {
        self.actions.get(&hotkey_id).copied()
    }
}

impl From<KeyCombo> for HotKey {
    fn from(combo: KeyCombo) -> HotKey {
        let mut modifiers = Modifiers::empty();
        if combo.modifiers.ctrl {
            modifiers |= Modifiers::CONTROL;
        }
        if combo.modifiers.alt {
            modifiers |= Modifiers::ALT;
        }
        if combo.modifiers.shift {
            modifiers |= Modifiers::SHIFT;
        }
        if combo.modifiers.cmd {
            modifiers |= Modifiers::SUPER;
        }
        let code = Code::from_str(&combo.key.to_code())
            .expect("config::Key::to_code emits valid keyboard_types codes");
        HotKey::new(Some(modifiers), code)
    }
}
