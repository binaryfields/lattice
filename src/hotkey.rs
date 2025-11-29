use std::collections::HashMap;

use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;

use crate::action::Action;
use crate::config::Config;

pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    registered: Vec<HotKey>,
    actions: HashMap<u32, Action>,
}

impl Hotkeys {
    pub fn new() -> Option<Hotkeys> {
        let manager = GlobalHotKeyManager::new()
            .inspect_err(|err| eprintln!("lattice: failed to create hotkey manager: {err}"))
            .ok()?;
        Some(Hotkeys {
            manager,
            registered: Vec::new(),
            actions: HashMap::new(),
        })
    }

    pub fn bind(&mut self, config: &Config) -> Vec<String> {
        if let Err(err) = self.manager.unregister_all(&self.registered) {
            eprintln!("lattice: failed to unregister previous hotkeys: {err}");
        }
        self.registered.clear();
        self.actions.clear();

        let mut failures = Vec::new();
        for (&action, &hotkey) in &config.bindings {
            match self.manager.register(hotkey) {
                Ok(()) => {
                    self.actions.insert(hotkey.id(), action);
                    self.registered.push(hotkey);
                }
                Err(err) => {
                    eprintln!("lattice: could not register {hotkey} for {action}: {err}");
                    failures.push(format!("{} ({hotkey})", action.label()));
                }
            }
        }
        failures
    }

    pub fn action(&self, hotkey_id: u32) -> Option<Action> {
        self.actions.get(&hotkey_id).copied()
    }
}
