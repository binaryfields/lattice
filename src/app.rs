use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use tray_icon::menu::MenuEvent;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId as WinitWindowId;

use crate::action::Action;
use crate::config::Config;
use crate::engine::{Engine, Snapshot};
use crate::hotkey::Hotkeys;
use crate::macos;
use crate::tray;

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

    let mut app = App::new();
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("lattice: event loop error: {err}");
    }
}

struct App {
    engine: Engine,
    hotkeys: Option<Hotkeys>,
    hotkey_failures: Vec<String>,
    tray: Option<tray::Tray>,
    trusted: bool,
    initialized: bool,
}

impl App {
    fn new() -> App {
        App {
            engine: Engine::new(),
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
        self.hotkeys = Hotkeys::new();
        self.bind_hotkeys();
        self.tray = tray::Tray::new();
    }

    fn bind_hotkeys(&mut self) {
        let config = Config::default();
        if let Some(hotkeys) = self.hotkeys.as_mut() {
            self.hotkey_failures = hotkeys.bind(&config);
        }
    }

    fn handle_menu(&mut self, event_loop: &ActiveEventLoop, id: &str) {
        match id {
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
        let Some(visible_frame) = macos::main_visible_frame() else {
            return;
        };
        let id = window.id();
        let snapshot = Snapshot {
            window: frame,
            visible_frames: vec![visible_frame],
        };
        let Some(target) = self.engine.place(id, action, &snapshot) else {
            return;
        };
        if window.set_frame(&target).is_none() {
            eprintln!("lattice: failed to apply frame for {action:?}");
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
}

pub enum UserEvent {
    Hotkey(GlobalHotKeyEvent),
    Menu(MenuEvent),
}
