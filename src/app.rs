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

const POLL: Duration = Duration::from_secs(2);

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

    let home = std::env::var_os("HOME").unwrap_or_default();
    let config_path = PathBuf::from(home).join(".config/lattice/config.toml");
    let mut app = App::new(config_path);
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("lattice: event loop error: {err}");
    }
}

pub enum UserEvent {
    Hotkey(GlobalHotKeyEvent),
    Menu(MenuEvent),
}

struct App {
    engine: Engine,
    config: Config,
    config_path: PathBuf,
    config_error: Option<String>,
    hotkeys: Option<Hotkeys>,
    hotkey_failures: usize,
    tray: Option<tray::Tray>,
    trusted: bool,
    initialized: bool,
}

impl App {
    fn new(config_path: PathBuf) -> App {
        App {
            engine: Engine::new(),
            config: Config::default(),
            config_path,
            config_error: None,
            hotkeys: None,
            hotkey_failures: 0,
            tray: None,
            trusted: false,
            initialized: false,
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

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Hotkey(hotkey) => {
                if hotkey.state() == HotKeyState::Pressed
                    && let Some(action) = self.config.bound_action(hotkey.id())
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

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WinitWindowId,
        _event: WindowEvent,
    ) {
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            self.refresh_trust();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + POLL));
    }
}

impl App {
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
        self.tray = tray::Tray::new(&self.config, &self.status());
    }

    fn load_config(&mut self) {
        match Config::load(&self.config_path) {
            Ok(config) => {
                self.engine.set_gaps(config.gaps);
                self.config = config;
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
            self.hotkey_failures = hotkeys.bind(&self.config);
        }
    }

    fn status(&self) -> tray::Status<'_> {
        tray::Status {
            trusted: self.trusted,
            config_error: self.config_error.as_deref(),
            hotkey_failures: self.hotkey_failures,
        }
    }

    fn refresh_trust(&mut self) -> bool {
        if !self.trusted && macos::is_trusted() {
            self.trusted = true;
            eprintln!("lattice: Accessibility permission granted");
            self.update_tray();
        }
        self.trusted
    }

    fn update_tray(&self) {
        if let Some(tray) = &self.tray {
            tray.update(&self.config, &self.status());
        }
    }

    fn handle_menu(&mut self, event_loop: &ActiveEventLoop, id: &str) {
        match id {
            tray::OPEN_CONFIG_ID => self.open_config(),
            tray::RELOAD_CONFIG_ID => self.reload_config(),
            tray::GRANT_ACCESS_ID => macos::open(macos::ACCESSIBILITY_SETTINGS),
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

    fn open_config(&self) {
        if !self.config_path.exists() {
            if let Some(dir) = self.config_path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(err) = std::fs::write(&self.config_path, crate::config::TEMPLATE) {
                eprintln!("lattice: could not write config template: {err}");
            }
        }
        macos::open(&self.config_path);
    }

    fn reload_config(&mut self) {
        self.load_config();
        self.bind_hotkeys();
        self.update_tray();
        if self.config_error.is_none() {
            eprintln!("lattice: config reloaded");
        }
    }

    fn perform(&mut self, action: Action) {
        if !self.refresh_trust() {
            eprintln!("lattice: ignoring {action:?}: no Accessibility permission");
            return;
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
        let Some(id) = window.id() else {
            eprintln!("lattice: could not identify focused window");
            return;
        };
        let snapshot = Snapshot {
            window: frame,
            visible_frames,
        };
        let Some(target) = self.engine.place(id, action, &snapshot) else {
            return;
        };
        if let Err(err) = window.set_frame(&target) {
            eprintln!("lattice: failed to apply frame for {action:?}: {err:?}");
        }
    }
}
