use std::ffi::c_void;
use std::ptr::NonNull;

use global_hotkey::hotkey::Modifiers;
use objc2_core_foundation::{
    CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource, kCFRunLoopCommonModes,
};
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventMask, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventTapProxy, CGEventType,
};

use crate::config::{Drag, Sweep};

const MIN_MOVE_DISTANCE: f64 = 2.0;

pub struct MouseMonitor {
    state: *mut MouseState,
    source: CFRetained<CFRunLoopSource>,
}

impl MouseMonitor {
    pub fn install(
        sweep: &Sweep,
        drag: &Drag,
        emit: Box<dyn Fn(MouseEvent)>,
    ) -> Option<MouseMonitor> {
        let state = Box::into_raw(Box::new(MouseState {
            tap: None,
            arm_flags: arm_flags(sweep),
            drag_enabled: drag.enabled,
            gesture: None,
            emit,
        }));
        let mask: CGEventMask = event_bit(CGEventType::MouseMoved)
            | event_bit(CGEventType::LeftMouseDown)
            | event_bit(CGEventType::LeftMouseUp)
            | event_bit(CGEventType::LeftMouseDragged)
            | event_bit(CGEventType::FlagsChanged);
        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::SessionEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                mask,
                Some(tap_callback),
                state.cast(),
            )
        };
        let source = tap
            .as_deref()
            .and_then(|tap| CFMachPort::new_run_loop_source(None, Some(tap), 0));
        let Some((source, run_loop)) = source.zip(CFRunLoop::current()) else {
            eprintln!("lattice: could not install the mouse monitor; sweep and drag disabled");
            drop(unsafe { Box::from_raw(state) });
            return None;
        };
        unsafe {
            (*state).tap = tap;
            run_loop.add_source(Some(&source), kCFRunLoopCommonModes);
        }
        Some(MouseMonitor { state, source })
    }

    pub fn set_config(&mut self, sweep: &Sweep, drag: &Drag) {
        let state = unsafe { &mut *self.state };
        state.arm_flags = arm_flags(sweep);
        state.drag_enabled = drag.enabled;
        state.gesture = None;
    }
}

impl Drop for MouseMonitor {
    fn drop(&mut self) {
        let state = unsafe { &*self.state };
        if let Some(tap) = &state.tap {
            CGEvent::tap_enable(tap, false);
            tap.invalidate();
        }
        if let Some(run_loop) = CFRunLoop::current() {
            unsafe { run_loop.remove_source(Some(&self.source), kCFRunLoopCommonModes) };
        }
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
    tap: Option<CFRetained<CFMachPort>>,
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

unsafe extern "C-unwind" fn tap_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    let state = unsafe { &mut *user_info.cast::<MouseState>() };
    let cg_event = unsafe { event.as_ref() };
    let input = match event_type {
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            if let Some(tap) = &state.tap {
                CGEvent::tap_enable(tap, true);
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
    if let Some(out) = state.step(input, (loc.x, loc.y)) {
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

fn event_bit(event_type: CGEventType) -> CGEventMask {
    1u64 << event_type.0
}
