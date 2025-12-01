#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
    pub const ALL: [Action; 19] = [
        Action::LeftHalf,
        Action::RightHalf,
        Action::TopHalf,
        Action::BottomHalf,
        Action::TopLeftQuarter,
        Action::TopRightQuarter,
        Action::BottomLeftQuarter,
        Action::BottomRightQuarter,
        Action::FirstThird,
        Action::CenterThird,
        Action::LastThird,
        Action::FirstTwoThirds,
        Action::LastTwoThirds,
        Action::Maximize,
        Action::AlmostMaximize,
        Action::Center,
        Action::Restore,
        Action::NextDisplay,
        Action::PrevDisplay,
    ];

    pub fn from_config_key(key: &str) -> Option<Action> {
        Self::ALL.into_iter().find(|a| a.config_key() == key)
    }

    pub fn config_key(self) -> &'static str {
        match self {
            Action::LeftHalf => "left-half",
            Action::RightHalf => "right-half",
            Action::TopHalf => "top-half",
            Action::BottomHalf => "bottom-half",
            Action::TopLeftQuarter => "top-left-quarter",
            Action::TopRightQuarter => "top-right-quarter",
            Action::BottomLeftQuarter => "bottom-left-quarter",
            Action::BottomRightQuarter => "bottom-right-quarter",
            Action::FirstThird => "first-third",
            Action::CenterThird => "center-third",
            Action::LastThird => "last-third",
            Action::FirstTwoThirds => "first-two-thirds",
            Action::LastTwoThirds => "last-two-thirds",
            Action::Maximize => "maximize",
            Action::AlmostMaximize => "almost-maximize",
            Action::Center => "center",
            Action::Restore => "restore",
            Action::NextDisplay => "next-display",
            Action::PrevDisplay => "prev-display",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::LeftHalf => "Left Half",
            Action::RightHalf => "Right Half",
            Action::TopHalf => "Top Half",
            Action::BottomHalf => "Bottom Half",
            Action::TopLeftQuarter => "Top Left Quarter",
            Action::TopRightQuarter => "Top Right Quarter",
            Action::BottomLeftQuarter => "Bottom Left Quarter",
            Action::BottomRightQuarter => "Bottom Right Quarter",
            Action::FirstThird => "First Third",
            Action::CenterThird => "Center Third",
            Action::LastThird => "Last Third",
            Action::FirstTwoThirds => "First Two Thirds",
            Action::LastTwoThirds => "Last Two Thirds",
            Action::Maximize => "Maximize",
            Action::AlmostMaximize => "Almost Maximize",
            Action::Center => "Center",
            Action::Restore => "Restore",
            Action::NextDisplay => "Next Display",
            Action::PrevDisplay => "Previous Display",
        }
    }

    pub fn default_binding(self) -> &'static str {
        match self {
            Action::LeftHalf => "ctrl+alt+left",
            Action::RightHalf => "ctrl+alt+right",
            Action::TopHalf => "ctrl+alt+up",
            Action::BottomHalf => "ctrl+alt+down",
            Action::TopLeftQuarter => "ctrl+alt+u",
            Action::TopRightQuarter => "ctrl+alt+i",
            Action::BottomLeftQuarter => "ctrl+alt+j",
            Action::BottomRightQuarter => "ctrl+alt+k",
            Action::FirstThird => "ctrl+alt+d",
            Action::CenterThird => "ctrl+alt+f",
            Action::LastThird => "ctrl+alt+g",
            Action::FirstTwoThirds => "ctrl+alt+e",
            Action::LastTwoThirds => "ctrl+alt+t",
            Action::Maximize => "ctrl+alt+enter",
            Action::AlmostMaximize => "ctrl+alt+shift+enter",
            Action::Center => "ctrl+alt+c",
            Action::Restore => "ctrl+alt+backspace",
            Action::NextDisplay => "ctrl+alt+cmd+right",
            Action::PrevDisplay => "ctrl+alt+cmd+left",
        }
    }

    pub fn resizes(self) -> bool {
        !matches!(self, Action::Center | Action::Restore)
    }
}
