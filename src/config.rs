use std::collections::BTreeMap;
use std::path::Path;

use global_hotkey::hotkey::{HotKey, Modifiers};
use serde::{Deserialize, Deserializer};
use strum::IntoEnumIterator;

use crate::action::Action;
use crate::layout::Gaps;

pub const TEMPLATE: &[u8] = include_bytes!("../assets/config-template.toml");

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub gaps: Gaps,
    pub drag: Drag,
    pub sweep: Sweep,
    pub bindings: BTreeMap<Action, HotKey>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, String> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
        Config::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Config, String> {
        let wire: Wire = toml::from_str(text).map_err(|e| e.to_string())?;
        let mut config = Config {
            gaps: wire.gaps,
            drag: wire.drag,
            sweep: wire.sweep,
            bindings: default_bindings(),
        };
        config.apply_keys(&wire.keys)?;
        config.validate()?;
        Ok(config)
    }

    fn apply_keys(&mut self, keys: &BTreeMap<String, String>) -> Result<(), String> {
        for (name, value) in keys {
            let action = Action::from_config_key(name)
                .ok_or_else(|| format!("[keys] unknown action {name:?}"))?;
            if value.is_empty() {
                self.bindings.remove(&action);
            } else {
                let combo = value
                    .parse::<HotKey>()
                    .map_err(|e| format!("[keys] {name} = {value:?}: {e}"))?;
                self.bindings.insert(action, combo);
            }
        }
        let mut seen = BTreeMap::new();
        for (&action, &combo) in &self.bindings {
            if let Some(other) = seen.insert(combo.id(), action) {
                return Err(format!(
                    "[keys] {other} and {action} are both bound to {combo}"
                ));
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), String> {
        if self.gaps.outer < 0.0 || self.gaps.inner < 0.0 {
            return Err("gaps must be >= 0".into());
        }
        if self.drag.edge <= 0.0 || self.drag.corner <= 0.0 {
            return Err("drag.edge and drag.corner must be > 0".into());
        }
        if self.sweep.distance <= 0.0 {
            return Err("sweep.distance must be > 0".into());
        }
        if self.sweep.modifier.is_empty() {
            return Err("sweep.modifier must name at least one modifier".into());
        }
        Ok(())
    }

    pub fn bound_action(&self, id: u32) -> Option<Action> {
        self.bindings
            .iter()
            .find_map(|(&action, hotkey)| (hotkey.id() == id).then_some(action))
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::parse("").expect("an empty config parses")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Drag {
    pub enabled: bool,
    pub edge: f64,
    pub corner: f64,
}

impl Default for Drag {
    fn default() -> Self {
        Drag {
            enabled: true,
            edge: 8.0,
            corner: 64.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sweep {
    pub enabled: bool,
    pub distance: f64,
    #[serde(deserialize_with = "deserialize_modifiers")]
    pub modifier: Modifiers,
}

impl Default for Sweep {
    fn default() -> Self {
        Sweep {
            enabled: true,
            distance: 60.0,
            modifier: Modifiers::CONTROL | Modifiers::SUPER,
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Wire {
    gaps: Gaps,
    drag: Drag,
    sweep: Sweep,
    keys: BTreeMap<String, String>,
}

fn default_bindings() -> BTreeMap<Action, HotKey> {
    Action::iter()
        .map(|a| {
            let combo = a
                .default_binding()
                .parse::<HotKey>()
                .expect("default bindings parse; guaranteed by tests");
            (a, combo)
        })
        .collect()
}

fn deserialize_modifiers<'de, D: Deserializer<'de>>(d: D) -> Result<Modifiers, D::Error> {
    let s = String::deserialize(d)?;
    let parts = s.split('+').map(str::trim).filter(|p| !p.is_empty());
    parts
        .map(modifier)
        .try_fold(Modifiers::empty(), |all, one| Ok(all | one?))
}

fn modifier<E: serde::de::Error>(part: &str) -> Result<Modifiers, E> {
    match part.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Ok(Modifiers::CONTROL),
        "alt" | "option" | "opt" => Ok(Modifiers::ALT),
        "shift" => Ok(Modifiers::SHIFT),
        "cmd" | "command" | "super" | "meta" => Ok(Modifiers::SUPER),
        _ => Err(E::custom(format!("unknown modifier {part:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(text: &str) -> HotKey {
        text.parse().expect("combo parses")
    }

    #[test]
    fn parse_with_empty_binding() {
        let config = Config::parse("[keys]\nright-half = \"\"\nleft-half = \"ctrl+alt+right\"\n")
            .expect("unbinding frees the combo");
        assert_eq!(config.bindings.get(&Action::RightHalf).copied(), None);
        assert_eq!(
            config.bindings.get(&Action::LeftHalf).copied(),
            Some(combo("ctrl+alt+right"))
        );
        assert_eq!(config.bindings.len(), Action::iter().count() - 1);
    }

    #[test]
    fn parse_with_duplicate_combo() {
        let err = Config::parse("[keys]\nleft-half = \"ctrl+alt+c\"\n")
            .expect_err("a collision is rejected");
        assert!(err.contains("left-half"), "{err}");
        assert!(err.contains("center"), "{err}");
    }

    #[test]
    fn parse_with_unlisted_actions() {
        let config = Config::parse("[keys]\nmaximize = \"ctrl+alt+m\"\n").expect("parses");
        assert_eq!(config.bindings.len(), Action::iter().count());
        assert_eq!(
            config.bindings.get(&Action::Maximize).copied(),
            Some(combo("ctrl+alt+m"))
        );
        assert_eq!(
            config.bindings.get(&Action::LeftHalf).copied(),
            Some(combo(Action::LeftHalf.default_binding()))
        );
    }

    #[test]
    fn parse_with_template() {
        let text = std::str::from_utf8(TEMPLATE).expect("the template is UTF-8");
        assert_eq!(Config::parse(text), Ok(Config::default()));
    }
}
