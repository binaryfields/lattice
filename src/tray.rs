use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub const QUIT_ID: &str = "quit";

pub struct Tray {
    icon: TrayIcon,
}

impl Tray {
    pub fn new() -> Option<Tray> {
        let menu = build_menu()?;
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
}

fn build_menu() -> Option<Menu> {
    let menu = Menu::new();
    menu.append_items(&[
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(QUIT_ID, "Quit Lattice", true, None),
    ])
    .ok()?;
    Some(menu)
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
