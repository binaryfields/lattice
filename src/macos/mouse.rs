use std::ffi::c_void;
use std::ptr::NonNull;

use global_hotkey::hotkey::Modifiers;
use objc2_core_foundation::{
    CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource, kCFRunLoopCommonModes,
};
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventMask, CGEventTapCallBack, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType,
};

use crate::config::{Drag, Sweep};

/// Events that can start or end a gesture. The trigger tap listens for
/// these the whole time the monitor is installed.
const TRIGGER_MASK: CGEventMask = event_bit(CGEventType::FlagsChanged)
    | event_bit(CGEventType::LeftMouseDown)
    | event_bit(CGEventType::LeftMouseUp);

/// Cursor motion. The motion tap is enabled only while a gesture is live,
/// so an idle process is not woken for every mouse move.
const MOTION_MASK: CGEventMask =
    event_bit(CGEventType::MouseMoved) | event_bit(CGEventType::LeftMouseDragged);

const MIN_MOVE_DISTANCE: f64 = 2.0;

pub struct MouseMonitor {
    state: *mut MouseState,
}

impl MouseMonitor {
    pub fn install(
        sweep: &Sweep,
        drag: &Drag,
        emit: Box<dyn Fn(MouseEvent)>,
    ) -> Option<MouseMonitor> {
        let state = Box::into_raw(Box::new(MouseState {
            trigger: None,
            motion: None,
            arm_flags: arm_flags(sweep),
            drag_enabled: drag.enabled,
            gesture: None,
            emit,
        }));
        let trigger = Tap::install(TRIGGER_MASK, Some(deliver::<false>), state);
        let motion = Tap::install(MOTION_MASK, Some(deliver::<true>), state);
        let Some((trigger, mut motion)) = trigger.zip(motion) else {
            eprintln!("lattice: could not install the mouse monitor; sweep and drag disabled");
            drop(unsafe { Box::from_raw(state) });
            return None;
        };
        motion.set_enabled(false);
        unsafe {
            (*state).trigger = Some(trigger);
            (*state).motion = Some(motion);
        }
        Some(MouseMonitor { state })
    }

    pub fn set_config(&mut self, sweep: &Sweep, drag: &Drag) {
        let state = unsafe { &mut *self.state };
        state.arm_flags = arm_flags(sweep);
        state.drag_enabled = drag.enabled;
        state.gesture = None;
        state.sync_taps();
    }
}

impl Drop for MouseMonitor {
    fn drop(&mut self) {
        drop(unsafe { Box::from_raw(self.state) });
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseEvent {
    Sweep(Phase),
    Drag(Phase),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Began { x: f64, y: f64 },
    Moved { x: f64, y: f64 },
    Ended { x: f64, y: f64 },
    Cancelled,
}

struct MouseState {
    trigger: Option<Tap>,
    motion: Option<Tap>,
    arm_flags: Option<CGEventFlags>,
    drag_enabled: bool,
    gesture: Option<Gesture>,
    emit: Box<dyn Fn(MouseEvent)>,
}

impl MouseState {
    fn step(&mut self, input: Input, (x, y): (f64, f64)) -> Option<MouseEvent> {
        match input {
            Input::Flags(flags) => {
                let held = self.arm_flags.is_some_and(|arm| flags.contains(arm));
                match (held, &self.gesture) {
                    (true, None | Some(Gesture::Drag(_))) => {
                        self.gesture = Some(Gesture::Sweep((x, y)));
                        Some(MouseEvent::Sweep(Phase::Began { x, y }))
                    }
                    (false, Some(Gesture::Sweep(_))) => {
                        self.gesture = None;
                        Some(MouseEvent::Sweep(Phase::Ended { x, y }))
                    }
                    _ => None,
                }
            }
            Input::ButtonDown if self.drag_enabled && self.gesture.is_none() => {
                self.gesture = Some(Gesture::Drag((x, y)));
                Some(MouseEvent::Drag(Phase::Began { x, y }))
            }
            Input::ButtonUp if matches!(self.gesture, Some(Gesture::Drag(_))) => {
                self.gesture = None;
                Some(MouseEvent::Drag(Phase::Ended { x, y }))
            }
            Input::Moved => {
                let gesture = self.gesture.as_mut()?;
                let (Gesture::Sweep(at) | Gesture::Drag(at)) = gesture;
                if (x - at.0).hypot(y - at.1) < MIN_MOVE_DISTANCE {
                    return None;
                }
                *at = (x, y);
                let moved = Phase::Moved { x, y };
                Some(match gesture {
                    Gesture::Sweep(_) => MouseEvent::Sweep(moved),
                    Gesture::Drag(_) => MouseEvent::Drag(moved),
                })
            }
            Input::TapDisabled => match self.gesture.take()? {
                Gesture::Sweep(_) => Some(MouseEvent::Sweep(Phase::Cancelled)),
                Gesture::Drag(_) => Some(MouseEvent::Drag(Phase::Cancelled)),
            },
            Input::ButtonDown | Input::ButtonUp => None,
        }
    }

    /// Keeps the trigger tap on, and the motion tap on exactly while a
    /// gesture is live.
    fn sync_taps(&mut self) {
        let live = self.gesture.is_some();
        if let Some(trigger) = self.trigger.as_mut() {
            trigger.set_enabled(true);
        }
        if let Some(motion) = self.motion.as_mut() {
            motion.set_enabled(live);
        }
    }
}

enum Gesture {
    Sweep((f64, f64)),
    Drag((f64, f64)),
}

enum Input {
    Flags(CGEventFlags),
    ButtonDown,
    ButtonUp,
    Moved,
    TapDisabled,
}

struct Tap {
    port: CFRetained<CFMachPort>,
    source: CFRetained<CFRunLoopSource>,
    enabled: bool,
}

impl Tap {
    /// Installs a listen-only session tap on the current run loop. The tap
    /// starts enabled, as `CGEventTapCreate` leaves it.
    fn install(
        mask: CGEventMask,
        callback: CGEventTapCallBack,
        state: *mut MouseState,
    ) -> Option<Tap> {
        let port = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::SessionEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                mask,
                callback,
                state.cast(),
            )
        }?;
        let source = CFMachPort::new_run_loop_source(None, Some(&port), 0)?;
        let run_loop = CFRunLoop::current()?;
        unsafe { run_loop.add_source(Some(&source), kCFRunLoopCommonModes) };
        Some(Tap {
            port,
            source,
            enabled: true,
        })
    }

    fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            CGEvent::tap_enable(&self.port, enabled);
        }
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        CGEvent::tap_enable(&self.port, false);
        self.port.invalidate();
        if let Some(run_loop) = CFRunLoop::current() {
            unsafe { run_loop.remove_source(Some(&self.source), kCFRunLoopCommonModes) };
        }
    }
}

/// Handles an event from the motion tap if `MOTION`, else the trigger tap.
unsafe extern "C-unwind" fn deliver<const MOTION: bool>(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    let state = unsafe { &mut *user_info.cast::<MouseState>() };
    let cg_event = unsafe { event.as_ref() };
    let input = match event_type {
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            // Disabling a tap makes the system post this to the tap itself,
            // so a notice for a tap we parked is our own doing. Only a tap
            // we hold as enabled was disabled behind our back; it is then
            // re-enabled by `sync_taps`.
            let tap = if MOTION {
                &mut state.motion
            } else {
                &mut state.trigger
            };
            match tap {
                Some(tap) if tap.enabled => tap.enabled = false,
                _ => return event.as_ptr(),
            }
            Input::TapDisabled
        }
        CGEventType::FlagsChanged => Input::Flags(CGEvent::flags(Some(cg_event))),
        CGEventType::LeftMouseDown => Input::ButtonDown,
        CGEventType::LeftMouseUp => Input::ButtonUp,
        CGEventType::MouseMoved | CGEventType::LeftMouseDragged => Input::Moved,
        _ => return event.as_ptr(),
    };
    let loc = CGEvent::location(Some(cg_event));
    let out = state.step(input, (loc.x, loc.y));
    state.sync_taps();
    if let Some(out) = out {
        (state.emit)(out);
    }
    event.as_ptr()
}

fn arm_flags(sweep: &Sweep) -> Option<CGEventFlags> {
    let flags = [
        (Modifiers::CONTROL, CGEventFlags::MaskControl),
        (Modifiers::ALT, CGEventFlags::MaskAlternate),
        (Modifiers::SHIFT, CGEventFlags::MaskShift),
        (Modifiers::SUPER, CGEventFlags::MaskCommand),
    ];
    let held = flags
        .into_iter()
        .filter(|&(m, _)| sweep.modifier.contains(m));
    sweep.enabled.then(|| held.map(|(_, f)| f).collect())
}

const fn event_bit(event_type: CGEventType) -> CGEventMask {
    1u64 << event_type.0
}
