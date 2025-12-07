use std::collections::BTreeMap;

use global_hotkey::hotkey::HotKey;
use strum::IntoEnumIterator;

use crate::action::Action;
use crate::layout::Gaps;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub gaps: Gaps,
    pub bindings: BTreeMap<Action, HotKey>,
}

impl Config {
    pub fn bound_action(&self, id: u32) -> Option<Action> {
        self.bindings
            .iter()
            .find_map(|(&action, hotkey)| (hotkey.id() == id).then_some(action))
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            gaps: Gaps::default(),
            bindings: default_bindings(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_across_every_action() {
        let bindings: Vec<_> = default_bindings().into_iter().collect();
        for (i, (action, combo)) in bindings.iter().enumerate() {
            if let Some((clash, _)) = bindings[i + 1..].iter().find(|(_, other)| other == combo) {
                panic!("{action} and {clash} share the default binding {combo}");
            }
        }
    }
}
