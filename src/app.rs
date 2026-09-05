use std::path::PathBuf;
use std::time::{Duration, Instant};

use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use tray_icon::menu::MenuEvent;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowId as WinitWindowId;

use crate::action::Action;
use crate::config::Config;
use crate::engine::{Engine, Grab, Snapshot};
use crate::hotkey::Hotkeys;
use crate::macos::{self, AxWindow, SweepEvent};
use crate::sweep;
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
    let mut app = App::new(config_path, event_loop.create_proxy());
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("lattice: event loop error: {err}");
    }
}

pub enum UserEvent {
    Hotkey(GlobalHotKeyEvent),
    Menu(MenuEvent),
    Sweep(SweepEvent),
}

struct App {
    engine: Engine,
    config: Config,
    config_path: PathBuf,
    config_error: Option<String>,
    hotkeys: Option<Hotkeys>,
    hotkey_failures: usize,
    tray: Option<tray::Tray>,
    sweep_monitor: Option<macos::SweepMonitor>,
    sweep: Option<sweep::Session<macos::AxWindow>>,
    overlay: macos::Overlay,
    proxy: EventLoopProxy<UserEvent>,
    trusted: bool,
    initialized: bool,
}

impl App {
    fn new(config_path: PathBuf, proxy: EventLoopProxy<UserEvent>) -> App {
        App {
            engine: Engine::new(),
            config: Config::default(),
            config_path,
            config_error: None,
            hotkeys: None,
            hotkey_failures: 0,
            tray: None,
            sweep_monitor: None,
            sweep: None,
            overlay: macos::Overlay::new(),
            proxy,
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
            UserEvent::Sweep(event) => self.handle_sweep(event),
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
    // ---- Setup ----

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
        if self.trusted {
            self.install_sweep();
        }
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

    fn install_sweep(&mut self) {
        let proxy = self.proxy.clone();
        self.sweep_monitor = macos::SweepMonitor::install(
            &self.config.sweep,
            Box::new(move |event| {
                let _ = proxy.send_event(UserEvent::Sweep(event));
            }),
        );
    }

    fn refresh_trust(&mut self) -> bool {
        if !self.trusted && macos::is_trusted() {
            self.trusted = true;
            eprintln!("lattice: Accessibility permission granted");
            self.update_tray();
            if self.sweep_monitor.is_none() {
                self.install_sweep();
            }
        }
        self.trusted
    }

    fn update_tray(&self) {
        if let Some(tray) = &self.tray {
            tray.update(&self.config, &self.status());
        }
    }

    // ---- Menu ----

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
        if let Some(monitor) = self.sweep_monitor.as_mut() {
            monitor.set_config(&self.config.sweep);
        }
        self.cancel_sweep();
        if self.config_error.is_none() {
            eprintln!("lattice: config reloaded");
        }
    }

    // ---- Actions ----

    fn perform(&mut self, action: Action) {
        if !self.refresh_trust() {
            eprintln!("lattice: ignoring {action:?}: no Accessibility permission");
            return;
        }
        let Some(window) = macos::focused_window() else {
            eprintln!("lattice: no focused window");
            return;
        };
        let grab = match grab(window) {
            Ok(grab) => grab,
            Err(why) => {
                eprintln!("lattice: focused window {why}; ignoring {action:?}");
                return;
            }
        };
        if action.resizes() && !grab.window.is_resizable() {
            eprintln!("lattice: focused window is not resizable; ignoring {action:?}");
            return;
        }
        self.apply(&grab, action);
    }

    fn apply(&mut self, grab: &Grab<AxWindow>, action: Action) {
        let Some(target) = self.engine.place(grab.id, action, &grab.snapshot) else {
            return;
        };
        if let Err(err) = grab.window.set_frame(&target) {
            eprintln!("lattice: failed to apply frame for {action:?}: {err:?}");
        }
    }

    // ---- Mouse ----

    fn handle_sweep(&mut self, event: SweepEvent) {
        match event {
            SweepEvent::Armed { x, y } => self.arm_sweep(x, y),
            SweepEvent::Moved { x, y } => self.preview_sweep(x, y),
            SweepEvent::Released { x, y } => self.commit_sweep(x, y),
            SweepEvent::Cancelled => self.cancel_sweep(),
        }
    }

    // ---- Sweep ----

    fn arm_sweep(&mut self, x: f64, y: f64) {
        self.sweep = None;
        let window = macos::window_at(x, y).filter(AxWindow::is_resizable);
        let Some(grab) = window.and_then(|window| grab(window).ok()) else {
            return;
        };
        self.sweep = Some(sweep::Session::begin(grab, (x, y)));
        self.overlay.show_ring(x, y);
    }

    fn preview_sweep(&self, x: f64, y: f64) {
        if let Some(session) = &self.sweep {
            self.overlay
                .set_outline(session.track((x, y), &self.engine, &self.config.sweep));
        }
    }

    fn commit_sweep(&mut self, x: f64, y: f64) {
        self.overlay.hide();
        let Some(session) = self.sweep.take() else {
            return;
        };
        let Some(action) = session.commit((x, y), &self.config.sweep) else {
            return;
        };
        self.apply(session.grab(), action);
        session.grab().window.raise();
    }

    fn cancel_sweep(&mut self) {
        self.sweep = None;
        self.overlay.hide();
    }
}

fn grab(window: AxWindow) -> Result<Grab<AxWindow>, &'static str> {
    if window.is_fullscreen() {
        return Err("is full screen");
    }
    let frame = window.frame().ok_or("frame could not be read")?;
    let visible_frames = macos::visible_frames();
    if visible_frames.is_empty() {
        return Err("is on no display");
    }
    let id = window.id().ok_or("could not be identified")?;
    let snapshot = Snapshot {
        window: frame,
        visible_frames,
    };
    Ok(Grab {
        window,
        id,
        snapshot,
    })
}
