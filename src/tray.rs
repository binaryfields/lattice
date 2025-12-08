use strum::IntoEnumIterator;
use tray_icon::menu::accelerator::Accelerator;
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::action::Action;
use crate::config::Config;
use crate::macos;

pub const GRANT_ACCESS_ID: &str = "grant-access";

const GROUP_STARTS: [Action; 4] = [
    Action::TopLeftQuarter,
    Action::FirstThird,
    Action::Maximize,
    Action::NextDisplay,
];

pub const LOGIN_ID: &str = "login";
pub const OPEN_CONFIG_ID: &str = "open-config";
pub const RELOAD_CONFIG_ID: &str = "reload-config";
pub const QUIT_ID: &str = "quit";

pub struct Tray {
    icon: TrayIcon,
}

impl Tray {
    pub fn new(config: &Config, status: &Status<'_>) -> Option<Tray> {
        let menu = build_menu(config, status)?;
        let icon = TrayIconBuilder::new()
            .with_icon(tray_icon())
            .with_icon_as_template(true)
            .with_tooltip("Lattice")
            .with_menu(Box::new(menu))
            .build()
            .inspect_err(|err| eprintln!("lattice: failed to create menu bar item: {err}"))
            .ok()?;
        Some(Tray { icon })
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
    pub hotkey_failures: usize,
}

fn build_menu(config: &Config, status: &Status<'_>) -> Option<Menu> {
    let menu = Menu::new();
    let failures = format!("(!) {} hotkey(s) not registered", status.hotkey_failures);
    let warnings = [
        (!status.trusted).then(|| {
            MenuItem::with_id(
                GRANT_ACCESS_ID,
                "(!) Grant Accessibility Access...",
                true,
                None,
            )
        }),
        status
            .config_error
            .map(|_| MenuItem::new("(!) Config error, keeping last good config", false, None)),
        (status.hotkey_failures > 0).then(|| MenuItem::new(&failures, false, None)),
    ];
    for warning in warnings.iter().flatten() {
        menu.append_items(&[warning, &PredefinedMenuItem::separator()])
            .ok()?;
    }
    for action in Action::iter() {
        if GROUP_STARTS.contains(&action) {
            menu.append(&PredefinedMenuItem::separator()).ok()?;
        }
        menu.append(&action_item(action, config)).ok()?;
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
    let hotkey = config.bindings.get(&action);
    let accelerator = hotkey.map(|h| Accelerator::new(Some(h.mods), h.key));
    MenuItem::with_id(action.to_string(), action.label(), true, accelerator)
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
