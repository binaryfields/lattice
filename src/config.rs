use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Deserializer};

use crate::action::Action;
use crate::layout::Gaps;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub gaps: Gaps,
    pub sweep: Sweep,
    pub bindings: Vec<(Action, KeyCombo)>,
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
            let slot = self.bindings.iter().position(|(a, _)| *a == action);
            if value.is_empty() {
                if let Some(i) = slot {
                    self.bindings.remove(i);
                }
            } else {
                let combo =
                    value.parse::<KeyCombo>().map_err(|e| format!("[keys] {name} = {value:?}: {e}"))?;
                match slot {
                    Some(i) => self.bindings[i].1 = combo,
                    None => self.bindings.push((action, combo)),
                }
            }
        }
        for (i, (a, combo)) in self.bindings.iter().enumerate() {
            if let Some((b, _)) = self.bindings[i + 1..]
                .iter()
                .find(|(_, other)| other == combo)
            {
                return Err(format!(
                    "[keys] {} and {} are both bound to {combo}",
                    a.config_key(),
                    b.config_key()
                ));
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), String> {
        if self.gaps.outer < 0.0 || self.gaps.inner < 0.0 {
            return Err("gaps must be >= 0".into());
        }
        if self.sweep.distance <= 0.0 {
            return Err("sweep.distance must be > 0".into());
        }
        if self.sweep.modifier == Modifiers::default() {
            return Err("sweep.modifier must name at least one modifier".into());
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            gaps: Gaps::default(),
            sweep: Sweep::default(),
            bindings: default_bindings(),
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
            modifier: Modifiers {
                ctrl: true,
                cmd: true,
                ..Modifiers::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyCombo {
    pub modifiers: Modifiers,
    pub key: Key,
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.modifiers)?;
        match self.key {
            Key::Left => write!(f, "←"),
            Key::Right => write!(f, "→"),
            Key::Up => write!(f, "↑"),
            Key::Down => write!(f, "↓"),
            Key::Enter => write!(f, "↩"),
            Key::Backspace => write!(f, "⌫"),
            Key::Space => write!(f, "Space"),
            Key::Tab => write!(f, "⇥"),
            Key::Escape => write!(f, "⎋"),
            Key::Home => write!(f, "↖"),
            Key::End => write!(f, "↘"),
            Key::PageUp => write!(f, "⇞"),
            Key::PageDown => write!(f, "⇟"),
            Key::Char(c) => write!(f, "{}", c.to_ascii_uppercase()),
            Key::F(n) => write!(f, "F{n}"),
        }
    }
}

impl FromStr for KeyCombo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut modifiers = Modifiers::default();
        let mut key = None;
        for part in s.split('+').map(str::trim) {
            if part.is_empty() {
                return Err("empty component".into());
            }
            if add_modifier(&mut modifiers, part).is_some() {
                continue;
            }
            if key.is_some() {
                return Err(format!("more than one key: {part:?}"));
            }
            key = Some(parse_key(part)?);
        }
        let key = key.ok_or("no key, only modifiers")?;
        Ok(KeyCombo { modifiers, key })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Enter,
    Backspace,
    Space,
    Tab,
    Escape,
    Home,
    End,
    PageUp,
    PageDown,
    Char(char),
    F(u8),
}

impl Key {
    pub fn to_code(self) -> String {
        match self {
            Key::Left => "ArrowLeft".into(),
            Key::Right => "ArrowRight".into(),
            Key::Up => "ArrowUp".into(),
            Key::Down => "ArrowDown".into(),
            Key::Enter => "Enter".into(),
            Key::Backspace => "Backspace".into(),
            Key::Space => "Space".into(),
            Key::Tab => "Tab".into(),
            Key::Escape => "Escape".into(),
            Key::Home => "Home".into(),
            Key::End => "End".into(),
            Key::PageUp => "PageUp".into(),
            Key::PageDown => "PageDown".into(),
            Key::F(n) => format!("F{n}"),
            Key::Char(c) if c.is_ascii_alphabetic() => format!("Key{}", c.to_ascii_uppercase()),
            Key::Char(c) if c.is_ascii_digit() => format!("Digit{c}"),
            Key::Char(c) => match c {
                '-' => "Minus",
                '=' => "Equal",
                '[' => "BracketLeft",
                ']' => "BracketRight",
                ';' => "Semicolon",
                '\'' => "Quote",
                ',' => "Comma",
                '.' => "Period",
                '/' => "Slash",
                '`' => "Backquote",
                _ => "Backslash",
            }
            .into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub cmd: bool,
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            write!(f, "⌃")?;
        }
        if self.alt {
            write!(f, "⌥")?;
        }
        if self.shift {
            write!(f, "⇧")?;
        }
        if self.cmd {
            write!(f, "⌘")?;
        }
        Ok(())
    }
}

impl FromStr for Modifiers {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut modifiers = Modifiers::default();
        for part in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
            add_modifier(&mut modifiers, part).ok_or_else(|| format!("unknown modifier {part:?}"))?;
        }
        Ok(modifiers)
    }
}

fn default_bindings() -> Vec<(Action, KeyCombo)> {
    Action::ALL
        .into_iter()
        .map(|a| {
            let combo = a.default_binding().parse::<KeyCombo>()
                .expect("default bindings parse; guaranteed by tests");
            (a, combo)
        })
        .collect()
}

fn parse_key(part: &str) -> Result<Key, String> {
    let lower = part.to_ascii_lowercase();
    let key = match lower.as_str() {
        "left" => Key::Left,
        "right" => Key::Right,
        "up" => Key::Up,
        "down" => Key::Down,
        "enter" | "return" => Key::Enter,
        "backspace" | "delete" => Key::Backspace,
        "space" => Key::Space,
        "tab" => Key::Tab,
        "escape" | "esc" => Key::Escape,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        _ => {
            if let Some(n) = lower.strip_prefix('f').and_then(|n| n.parse::<u8>().ok())
                && (1..=12).contains(&n)
                && lower.len() > 1
            {
                return Ok(Key::F(n));
            }
            let mut chars = lower.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphanumeric() || "-=[];',./`\\".contains(c) => {
                    Key::Char(c)
                }
                _ => return Err(format!("unknown key {part:?}")),
            }
        }
    };
    Ok(key)
}

fn deserialize_modifiers<'de, D: Deserializer<'de>>(d: D) -> Result<Modifiers, D::Error> {
    let s = String::deserialize(d)?;
    s.parse::<Modifiers>().map_err(serde::de::Error::custom)
}

fn add_modifier(modifiers: &mut Modifiers, part: &str) -> Option<()> {
    match part.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => modifiers.ctrl = true,
        "alt" | "option" | "opt" => modifiers.alt = true,
        "shift" => modifiers.shift = true,
        "cmd" | "command" | "super" | "meta" => modifiers.cmd = true,
        _ => return None,
    }
    Some(())
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Wire {
    gaps: Gaps,
    sweep: Sweep,
    keys: BTreeMap<String, String>,
}

pub const TEMPLATE: &[u8] = include_bytes!("../assets/config-template.toml");
