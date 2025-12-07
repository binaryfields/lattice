use strum::{Display, EnumIter, EnumString};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display, EnumIter, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum Action {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeftQuarter,
    TopRightQuarter,
    BottomLeftQuarter,
    BottomRightQuarter,
    FirstThird,
    CenterThird,
    LastThird,
    FirstTwoThirds,
    LastTwoThirds,
    Maximize,
    AlmostMaximize,
    Center,
    Restore,
    NextDisplay,
    PrevDisplay,
}

impl Action {
    pub fn from_config_key(key: &str) -> Option<Action> {
        key.parse().ok()
    }

    pub fn default_binding(self) -> &'static str {
        self.spec().1
    }

    fn spec(self) -> (&'static str, &'static str) {
        match self {
            Action::LeftHalf => ("Left Half", "ctrl+alt+left"),
            Action::RightHalf => ("Right Half", "ctrl+alt+right"),
            Action::TopHalf => ("Top Half", "ctrl+alt+up"),
            Action::BottomHalf => ("Bottom Half", "ctrl+alt+down"),
            Action::TopLeftQuarter => ("Top Left Quarter", "ctrl+alt+u"),
            Action::TopRightQuarter => ("Top Right Quarter", "ctrl+alt+i"),
            Action::BottomLeftQuarter => ("Bottom Left Quarter", "ctrl+alt+j"),
            Action::BottomRightQuarter => ("Bottom Right Quarter", "ctrl+alt+k"),
            Action::FirstThird => ("First Third", "ctrl+alt+d"),
            Action::CenterThird => ("Center Third", "ctrl+alt+f"),
            Action::LastThird => ("Last Third", "ctrl+alt+g"),
            Action::FirstTwoThirds => ("First Two Thirds", "ctrl+alt+e"),
            Action::LastTwoThirds => ("Last Two Thirds", "ctrl+alt+t"),
            Action::Maximize => ("Maximize", "ctrl+alt+enter"),
            Action::AlmostMaximize => ("Almost Maximize", "ctrl+alt+shift+enter"),
            Action::Center => ("Center", "ctrl+alt+c"),
            Action::Restore => ("Restore", "ctrl+alt+backspace"),
            Action::NextDisplay => ("Next Display", "ctrl+alt+cmd+right"),
            Action::PrevDisplay => ("Previous Display", "ctrl+alt+cmd+left"),
        }
    }

    pub fn label(self) -> &'static str {
        self.spec().0
    }

    pub fn resizes(self) -> bool {
        !matches!(self, Action::Center | Action::Restore)
    }
}
