use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;

use crate::config::Config;

pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    registered: Vec<HotKey>,
}

impl Hotkeys {
    pub fn new() -> Option<Hotkeys> {
        let manager = GlobalHotKeyManager::new()
            .inspect_err(|err| eprintln!("lattice: failed to create hotkey manager: {err}"))
            .ok()?;
        Some(Hotkeys {
            manager,
            registered: Vec::new(),
        })
    }

    pub fn bind(&mut self, config: &Config) -> usize {
        if let Err(err) = self.manager.unregister_all(&self.registered) {
            eprintln!("lattice: failed to unregister previous hotkeys: {err}");
        }
        self.registered.clear();

        let mut failures = 0;
        for (&action, &hotkey) in &config.bindings {
            match self.manager.register(hotkey) {
                Ok(()) => self.registered.push(hotkey),
                Err(err) => {
                    eprintln!("lattice: could not register {hotkey} for {action}: {err}");
                    failures += 1;
                }
            }
        }
        failures
    }
}
