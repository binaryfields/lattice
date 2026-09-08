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
use crate::drag;
use crate::engine::{Engine, Grab, Snapshot};
use crate::hotkey::Hotkeys;
use crate::macos::{self, AxWindow, MouseEvent, Phase};
use crate::sweep;
use crate::tray;

const POLL: Duration = Duration::from_secs(2);
const MISSION_CONTROL_INTERVAL: Duration = Duration::from_millis(100);
const SETTLE_INTERVAL: Duration = Duration::from_millis(50);
const SETTLE_ATTEMPTS: u32 = 6;

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
    Mouse(MouseEvent),
}

struct App {
    engine: Engine,
    config: Config,
    config_path: PathBuf,
    config_error: Option<String>,
    hotkeys: Option<Hotkeys>,
    hotkey_failures: usize,
    tray: Option<tray::Tray>,
    monitor: Option<macos::MouseMonitor>,
    sweep: Option<sweep::Session<macos::AxWindow>>,
    drag: Option<drag::Session<macos::AxWindow>>,
    watch: Option<Watch>,
    dropped: Option<Dropped>,
    overlay: macos::Overlay,
    proxy: EventLoopProxy<UserEvent>,
    poll_due: Instant,
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
            monitor: None,
            sweep: None,
            drag: None,
            watch: None,
            dropped: None,
            overlay: macos::Overlay::new(),
            proxy,
            poll_due: Instant::now(),
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
            UserEvent::Mouse(event) => self.handle_mouse(event),
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WinitWindowId,
        _event: WindowEvent,
    ) {
    }

    /// Runs after every pass of the run loop, so it does no I/O, and the
    /// wake instant it hands winit is a stored deadline: recomputing it each
    /// pass would make winit re-arm its timer every time.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if self.initialized {
            self.refresh_trust();
        }
        if self.dropped.as_ref().is_some_and(|d| d.due <= now) {
            self.settle_drop();
        }
        if self.poll_due <= now {
            self.poll_due = now + POLL;
        }
        let poll = (!self.trusted).then_some(self.poll_due);
        let settle = self.dropped.as_ref().map(|dropped| dropped.due);
        let wake = poll.into_iter().chain(settle).min();
        event_loop.set_control_flow(wake.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
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
            self.install_monitor();
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

    fn install_monitor(&mut self) {
        let proxy = self.proxy.clone();
        self.monitor = macos::MouseMonitor::install(
            &self.config.sweep,
            &self.config.drag,
            Box::new(move |event| {
                let _ = proxy.send_event(UserEvent::Mouse(event));
            }),
        );
    }

    fn refresh_trust(&mut self) -> bool {
        if !self.trusted && macos::is_trusted() {
            self.trusted = true;
            eprintln!("lattice: Accessibility permission granted");
            self.update_tray();
            if self.monitor.is_none() {
                self.install_monitor();
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
        if let Some(monitor) = self.monitor.as_mut() {
            monitor.set_config(&self.config.sweep, &self.config.drag);
        }
        self.cancel_sweep();
        self.cancel_drag();
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
        match grab.window.set_frame(&target) {
            Ok(actual) => self.engine.applied(grab.id, actual),
            Err(err) => eprintln!("lattice: failed to apply frame for {action:?}: {err:?}"),
        }
    }

    // ---- Mouse ----

    fn handle_mouse(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Sweep(Phase::Began { x, y }) => self.arm_sweep(x, y),
            MouseEvent::Sweep(Phase::Moved { x, y }) => self.preview_sweep(x, y),
            MouseEvent::Sweep(Phase::Ended { x, y }) => self.commit_sweep(x, y),
            MouseEvent::Sweep(Phase::Cancelled) => self.cancel_sweep(),
            MouseEvent::Drag(Phase::Began { x, y }) => self.begin_drag(x, y),
            MouseEvent::Drag(Phase::Moved { x, y }) => self.track_drag(x, y),
            MouseEvent::Drag(Phase::Ended { x, y }) => self.commit_drag(x, y),
            MouseEvent::Drag(Phase::Cancelled) => self.cancel_drag(),
        }
    }

    // ---- Sweep ----

    fn arm_sweep(&mut self, x: f64, y: f64) {
        self.sweep = None;
        self.cancel_drag();
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

    // ---- Drag ----

    fn begin_drag(&mut self, x: f64, y: f64) {
        self.drag = Some(drag::Session::begin((x, y)));
    }

    fn track_drag(&mut self, x: f64, y: f64) {
        let Some(session) = self.drag.as_ref() else {
            return;
        };
        match session.probe((x, y)) {
            drag::Probe::Window => self.capture_drag(x, y),
            drag::Probe::Frame => self.confirm_drag(x, y),
            drag::Probe::Idle => {}
        }
        self.yield_to_mission_control();
        if let Some(session) = &self.drag {
            self.overlay
                .set_outline(session.track((x, y), &self.engine, &self.config.drag));
        }
    }

    fn commit_drag(&mut self, x: f64, y: f64) {
        self.overlay.set_outline(None);
        let watch = self.watch.take();
        let Some(session) = self.drag.take() else {
            return;
        };
        // A drop made while Mission Control is up is its to handle: the
        // window is on its way to another desktop, or back where it was.
        if watch.is_some_and(|watch| watch.mission_control.is_open()) {
            return;
        }
        self.dropped = Some(Dropped {
            session,
            cursor: (x, y),
            attempts: 0,
            due: Instant::now(),
        });
        self.settle_drop();
    }

    fn cancel_drag(&mut self) {
        self.drag = None;
        self.watch = None;
        self.dropped = None;
        self.overlay.set_outline(None);
    }

    fn capture_drag(&mut self, x: f64, y: f64) {
        let Some(session) = self.drag.as_mut() else {
            return;
        };
        // Hit-test the press point before the current one: capture waits for
        // 12 pt of travel, and a straight-up drag has by then carried the
        // cursor clear of the title bar onto whatever sits behind it, whereas
        // the press point stays on the window the drag actually grabbed.
        let captured = [session.start(), (x, y)].into_iter().find_map(|(x, y)| {
            let window = macos::window_at(x, y).filter(AxWindow::is_resizable)?;
            grab(window).ok()
        });
        let Some(grab) = captured else {
            session.abandon();
            return;
        };
        session.capture(grab, (x, y));
        self.watch = Some(Watch {
            mission_control: macos::MissionControl::new(),
            polled: Instant::now(),
        });
    }

    fn confirm_drag(&mut self, x: f64, y: f64) {
        let Some(session) = self.drag.as_mut() else {
            return;
        };
        let Some(frame) = session.window().and_then(macos::AxWindow::frame) else {
            session.abandon();
            return;
        };
        session.confirm(&frame, (x, y));
    }

    /// Yields the drag to Mission Control. Dragging a window to the top edge
    /// is also macOS's gesture for moving it to another desktop: Mission
    /// Control opens within about 100 ms of the cursor touching the edge, too
    /// soon to tell from a drag that means to maximize, and its drop targets,
    /// the Spaces Bar and the "+" for a new desktop, sit inside the top zones.
    /// So a session that sees Mission Control open goes inert, its outline
    /// hides, and the drop is left to macOS. The window list is asked at most
    /// every `MISSION_CONTROL_INTERVAL`.
    fn yield_to_mission_control(&mut self) {
        let now = Instant::now();
        let Some(watch) = self.watch.as_mut() else {
            return;
        };
        if now < watch.polled + MISSION_CONTROL_INTERVAL {
            return;
        }
        watch.polled = now;
        if !watch.mission_control.is_open() {
            return;
        }
        self.watch = None;
        if let Some(session) = self.drag.as_mut() {
            session.abandon();
        }
    }

    /// Lands a released drag. The dropped window's frame trails the cursor
    /// (see `drag::Session::confirm`), so a release the session has not yet
    /// judged is re-read every `SETTLE_INTERVAL` until the frame catches up
    /// or `SETTLE_ATTEMPTS` run out; a flick faster than the lag still lands.
    fn settle_drop(&mut self) {
        let Some(mut dropped) = self.dropped.take() else {
            return;
        };
        let Some(frame) = dropped.session.window().and_then(macos::AxWindow::frame) else {
            return;
        };
        dropped.session.confirm(&frame, dropped.cursor);
        if dropped.session.undecided() && dropped.attempts < SETTLE_ATTEMPTS {
            dropped.attempts += 1;
            dropped.due = Instant::now() + SETTLE_INTERVAL;
            self.dropped = Some(dropped);
            return;
        }
        let cursor = dropped.cursor;
        match dropped.session.release(cursor, &frame, &self.config.drag) {
            Some(drag::Landing::Snap(grab, action)) => self.apply(&grab, action),
            Some(drag::Landing::Unsnap(grab)) => {
                let Some(restored) = self.engine.unsnap(grab.id, cursor, &frame) else {
                    return;
                };
                if let Err(err) = grab.window.set_frame(&restored) {
                    eprintln!("lattice: failed to restore the unsnapped frame: {err:?}");
                }
            }
            None => {}
        }
    }
}

/// Mission Control lookout for the drag in progress, posted when the drag
/// captured its window.
struct Watch {
    mission_control: macos::MissionControl,
    polled: Instant,
}

struct Dropped {
    session: drag::Session<macos::AxWindow>,
    cursor: (f64, f64),
    attempts: u32,
    due: Instant,
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
