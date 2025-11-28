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
    Maximize,
    Restore,
}

impl Action {
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
            Action::Maximize => ("Maximize", "ctrl+alt+enter"),
            Action::Restore => ("Restore", "ctrl+alt+backspace"),
        }
    }

    pub fn label(self) -> &'static str {
        self.spec().0
    }
}
