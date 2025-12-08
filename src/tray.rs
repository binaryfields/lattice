use std::str::FromStr;

use tray_icon::menu::accelerator::{Accelerator, Code, Modifiers};
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::action::Action;
use crate::config::{Config, KeyCombo};
use crate::macos;

pub const OPEN_CONFIG_ID: &str = "open-config";
pub const RELOAD_CONFIG_ID: &str = "reload-config";
pub const QUIT_ID: &str = "quit";
pub const GRANT_ACCESS_ID: &str = "grant-access";
pub const LOGIN_ID: &str = "login";

const GROUPS: [&[Action]; 5] = [
    &[
        Action::LeftHalf,
        Action::RightHalf,
        Action::TopHalf,
        Action::BottomHalf,
    ],
    &[
        Action::TopLeftQuarter,
        Action::TopRightQuarter,
        Action::BottomLeftQuarter,
        Action::BottomRightQuarter,
    ],
    &[
        Action::FirstThird,
        Action::CenterThird,
        Action::LastThird,
        Action::FirstTwoThirds,
        Action::LastTwoThirds,
    ],
    &[
        Action::Maximize,
        Action::AlmostMaximize,
        Action::Center,
        Action::Restore,
    ],
    &[Action::NextDisplay, Action::PrevDisplay],
];

pub struct Tray {
    icon: TrayIcon,
}

impl Tray {
    pub fn new(config: &Config, status: &Status<'_>) -> Option<Tray> {
        let menu = build_menu(config, status)?;
        match TrayIconBuilder::new()
            .with_icon(tray_icon())
            .with_icon_as_template(true)
            .with_tooltip("Lattice")
            .with_menu(Box::new(menu))
            .build()
        {
            Ok(icon) => Some(Tray { icon }),
            Err(err) => {
                eprintln!("lattice: failed to create menu bar item: {err}");
                None
            }
        }
    }

    pub fn update(&self, config: &Config, status: &Status<'_>) {
        if let Some(menu) = build_menu(config, status) {
            self.icon.set_menu(Some(Box::new(menu)));
        }
    }
}

pub struct Status<'a> {
    pub trusted: bool,
    pub config_error: Option<&'a str>,
    pub hotkey_failures: &'a [String],
}

fn build_menu(config: &Config, status: &Status<'_>) -> Option<Menu> {
    let menu = Menu::new();
    if !status.trusted {
        let grant = MenuItem::with_id(
            GRANT_ACCESS_ID,
            "(!) Grant Accessibility Access...",
            true,
            None,
        );
        menu.append_items(&[&grant, &PredefinedMenuItem::separator()])
            .ok()?;
    }
    if status.config_error.is_some() {
        let flag = MenuItem::new("(!) Config error, keeping last good config", false, None);
        menu.append_items(&[&flag, &PredefinedMenuItem::separator()])
            .ok()?;
    }
    if !status.hotkey_failures.is_empty() {
        let label = format!(
            "(!) {} hotkey(s) not registered",
            status.hotkey_failures.len()
        );
        let flag = MenuItem::new(&label, false, None);
        menu.append_items(&[&flag, &PredefinedMenuItem::separator()])
            .ok()?;
    }
    for (i, actions) in GROUPS.iter().enumerate() {
        if i > 0 {
            menu.append(&PredefinedMenuItem::separator()).ok()?;
        }
        for &action in *actions {
            menu.append(&action_item(action, config)).ok()?;
        }
    }
    let login = CheckMenuItem::with_id(
        LOGIN_ID,
        "Start at Login",
        true,
        macos::starts_at_login(),
        None,
    );
    menu.append_items(&[
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(OPEN_CONFIG_ID, "Open Config", true, None),
        &MenuItem::with_id(RELOAD_CONFIG_ID, "Reload Config", true, None),
        &PredefinedMenuItem::separator(),
        &login,
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(QUIT_ID, "Quit Lattice", true, None),
    ])
    .ok()?;
    Some(menu)
}

fn action_item(action: Action, config: &Config) -> MenuItem {
    let combo = config
        .bindings
        .iter()
        .find(|(a, _)| *a == action)
        .map(|(_, combo)| *combo);
    MenuItem::with_id(
        action.config_key(),
        action.label(),
        true,
        combo.and_then(|combo| Accelerator::try_from(combo).ok()),
    )
}

impl TryFrom<KeyCombo> for Accelerator {
    type Error = ();

    fn try_from(combo: KeyCombo) -> Result<Accelerator, Self::Error> {
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
        let code = Code::from_str(&combo.key.to_code()).map_err(|_| ())?;
        Ok(Accelerator::new(Some(modifiers), code))
    }
}

fn tray_icon() -> Icon {
    let png = include_bytes!("../assets/tray.png");
    let mut reader = png::Decoder::new(png.as_slice())
        .read_info()
        .expect("tray.png is a valid PNG");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("decode tray.png");
    buf.truncate(info.buffer_size());
    Icon::from_rgba(buf, info.width, info.height).expect("valid tray icon")
}
