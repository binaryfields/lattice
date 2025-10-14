use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId as WinitWindowId;

use crate::action::Action;
use crate::layout::{self, Gaps};
use crate::macos;

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

    let mut app = App {
        manager: None,
        trusted: false,
        initialized: false,
    };
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("lattice: event loop error: {err}");
    }
}

struct App {
    manager: Option<GlobalHotKeyManager>,
    trusted: bool,
    initialized: bool,
}

impl App {
    fn init(&mut self) {
        self.trusted = macos::request_trust();
        if !self.trusted {
            eprintln!(
                "lattice: waiting for Accessibility permission (System Settings > Privacy & Security > Accessibility)"
            );
        }
        match GlobalHotKeyManager::new() {
            Ok(manager) => {
                let hotkey =
                    HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowLeft);
                if let Err(err) = manager.register(hotkey) {
                    eprintln!("lattice: could not register ctrl+alt+left: {err}");
                }
                self.manager = Some(manager);
            }
            Err(err) => eprintln!("lattice: failed to create hotkey manager: {err}"),
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
        let Some(frame) = window.frame() else {
            eprintln!("lattice: could not read focused window frame");
            return;
        };
        let Some(visible_frame) = macos::main_visible_frame() else {
            return;
        };
        let Some(target) = layout::place(action, 0.5, &frame, &visible_frame, Gaps::default())
        else {
            return;
        };
        if !window.set_frame(&target) {
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

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Hotkey(hotkey) => {
                if hotkey.state() == HotKeyState::Pressed {
                    self.perform(Action::LeftHalf);
                }
            }
        }
    }
}

pub enum UserEvent {
    Hotkey(GlobalHotKeyEvent),
}
