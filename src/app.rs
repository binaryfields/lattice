use std::path::PathBuf;
use std::time::{Duration, Instant};

use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use tray_icon::menu::MenuEvent;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId as WinitWindowId;

use crate::action::Action;
use crate::config::Config;
use crate::engine::{Engine, Snapshot};
use crate::hotkey::Hotkeys;
use crate::macos;
use crate::tray;

const CONFIG_POLL_INTERVAL: Duration = Duration::from_secs(2);

pub fn run() {
    let mut builder = EventLoop::<UserEvent>::with_user_event();
    macos::set_accessory_activation_policy(&mut builder);
    let event_loop = builder
        .build()
        .expect("lattice: failed to create event loop");

    let proxy = event_loop.create_proxy();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        let _ = proxy.send_event(UserEvent::Hotkey(event));
    }));
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let mut app = App::new(config_path());
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("lattice: event loop error: {err}");
    }
}

struct App {
    engine: Engine,
    config_path: PathBuf,
    config_mtime: Option<std::time::SystemTime>,
    config_error: Option<String>,
    hotkeys: Option<Hotkeys>,
    hotkey_failures: Vec<String>,
    tray: Option<tray::Tray>,
    trusted: bool,
    initialized: bool,
}

impl App {
    fn new(config_path: PathBuf) -> App {
        App {
            engine: Engine::new(Config::default()),
            config_path,
            config_mtime: None,
            config_error: None,
            hotkeys: None,
            hotkey_failures: Vec::new(),
            tray: None,
            trusted: false,
            initialized: false,
        }
    }

    fn init(&mut self) {
        self.trusted = macos::request_trust();
        if !self.trusted {
            eprintln!(
                "lattice: waiting for Accessibility permission (System Settings > Privacy & Security > Accessibility)"
            );
        }
        self.load_config();
        self.hotkeys = Hotkeys::new();
        self.bind_hotkeys();
        self.tray = tray::Tray::new(self.engine.config(), &self.status());
    }

    fn load_config(&mut self) {
        self.config_mtime = std::fs::metadata(&self.config_path)
            .and_then(|m| m.modified())
            .ok();
        match Config::load(&self.config_path) {
            Ok(config) => {
                self.engine.set_config(config);
                self.config_error = None;
            }
            Err(err) => {
                eprintln!("lattice: config error, keeping last good config: {err}");
                self.config_error = Some(err);
            }
        }
    }

    fn bind_hotkeys(&mut self) {
        if let Some(hotkeys) = self.hotkeys.as_mut() {
            self.hotkey_failures = hotkeys.bind(self.engine.config());
        }
    }

    fn poll_config(&mut self) {
        let mtime = std::fs::metadata(&self.config_path)
            .and_then(|m| m.modified())
            .ok();
        if mtime != self.config_mtime {
            self.reload_config();
        }
    }

    fn reload_config(&mut self) {
        self.load_config();
        self.bind_hotkeys();
        self.update_tray();
        if self.config_error.is_none() {
            eprintln!("lattice: config reloaded");
        }
    }

    fn open_config(&self) {
        if !self.config_path.exists() {
            if let Some(dir) = self.config_path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(err) = std::fs::write(&self.config_path, crate::config::TEMPLATE) {
                eprintln!("lattice: could not write config template: {err}");
            }
        }
        if let Err(err) = macos::open_in_default_app(&self.config_path) {
            eprintln!("lattice: could not open config: {err}");
        }
    }

    fn handle_menu(&mut self, event_loop: &ActiveEventLoop, id: &str) {
        match id {
            tray::OPEN_CONFIG_ID => self.open_config(),
            tray::RELOAD_CONFIG_ID => self.reload_config(),
            tray::GRANT_ACCESS_ID => macos::open_accessibility_settings(),
            tray::LOGIN_ID => {
                macos::set_start_at_login(!macos::starts_at_login());
                self.update_tray();
            }
            tray::QUIT_ID => event_loop.exit(),
            key => {
                if let Some(action) = Action::from_config_key(key) {
                    self.perform(action);
                }
            }
        }
    }

    fn perform(&mut self, action: Action) {
        if !self.trusted {
            self.trusted = macos::is_trusted();
            if !self.trusted {
                eprintln!("lattice: ignoring {action:?}: no Accessibility permission");
                return;
            }
            self.update_tray();
        }
        let Some(window) = macos::focused_window() else {
            eprintln!("lattice: no focused window");
            return;
        };
        if window.is_fullscreen() {
            eprintln!("lattice: focused window is full screen; ignoring {action:?}");
            return;
        }
        if action.resizes() && !window.is_resizable() {
            eprintln!("lattice: focused window is not resizable; ignoring {action:?}");
            return;
        }
        let Some(frame) = window.frame() else {
            eprintln!("lattice: could not read focused window frame");
            return;
        };
        let visible_frames = macos::visible_frames();
        if visible_frames.is_empty() {
            return;
        }
        let id = window.id();
        let snapshot = Snapshot {
            window: frame,
            visible_frames,
        };
        let Some(target) = self.engine.place(id, action, &snapshot) else {
            return;
        };
        if window.set_frame(&target).is_none() {
            eprintln!("lattice: failed to apply frame for {action:?}");
        }
    }

    fn status(&self) -> tray::Status<'_> {
        tray::Status {
            trusted: self.trusted,
            config_error: self.config_error.as_deref(),
            hotkey_failures: &self.hotkey_failures,
        }
    }

    fn update_tray(&self) {
        if let Some(tray) = &self.tray {
            tray.update(self.engine.config(), &self.status());
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if cause == StartCause::Init && !self.initialized {
            self.initialized = true;
            self.init();
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WinitWindowId,
        _event: WindowEvent,
    ) {
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Hotkey(hotkey) => {
                if hotkey.state() == HotKeyState::Pressed
                    && let Some(action) = self.hotkeys.as_ref().and_then(|h| h.action(hotkey.id()))
                {
                    self.perform(action);
                }
            }
            UserEvent::Menu(menu) => {
                let id = menu.id().0.clone();
                self.handle_menu(event_loop, &id);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.poll_config();
        if self.initialized && !self.trusted && macos::is_trusted() {
            self.trusted = true;
            eprintln!("lattice: Accessibility permission granted");
            self.update_tray();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + CONFIG_POLL_INTERVAL,
        ));
    }
}

pub enum UserEvent {
    Hotkey(GlobalHotKeyEvent),
    Menu(MenuEvent),
}

fn config_path() -> PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(".config/lattice/config.toml")
}
