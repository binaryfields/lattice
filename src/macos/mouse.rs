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

use crate::config::Sweep;

const MIN_MOVE_DISTANCE: f64 = 2.0;

pub struct SweepMonitor {
    state: *mut SweepState,
    source: CFRetained<CFRunLoopSource>,
}

impl SweepMonitor {
    pub fn install(sweep: &Sweep, emit: Box<dyn Fn(SweepEvent)>) -> Option<SweepMonitor> {
        let state = Box::into_raw(Box::new(SweepState {
            tap: None,
            arm_flags: arm_flags(sweep),
            armed_at: None,
            emit,
        }));
        let mask: CGEventMask = event_bit(CGEventType::MouseMoved)
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
            eprintln!("lattice: could not install the sweep monitor; sweep disabled");
            drop(unsafe { Box::from_raw(state) });
            return None;
        };
        unsafe {
            (*state).tap = tap;
            run_loop.add_source(Some(&source), kCFRunLoopCommonModes);
        }
        Some(SweepMonitor { state, source })
    }

    pub fn set_config(&mut self, sweep: &Sweep) {
        let state = unsafe { &mut *self.state };
        state.arm_flags = arm_flags(sweep);
        state.armed_at = None;
    }
}

impl Drop for SweepMonitor {
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
pub enum SweepEvent {
    Armed { x: f64, y: f64 },
    Moved { x: f64, y: f64 },
    Released { x: f64, y: f64 },
    Cancelled,
}

struct SweepState {
    tap: Option<CFRetained<CFMachPort>>,
    arm_flags: Option<CGEventFlags>,
    armed_at: Option<(f64, f64)>,
    emit: Box<dyn Fn(SweepEvent)>,
}

impl SweepState {
    fn step(&mut self, input: Input, (x, y): (f64, f64)) -> Option<SweepEvent> {
        match input {
            Input::Flags(flags) => {
                let held = self.arm_flags.is_some_and(|arm| flags.contains(arm));
                match (held, self.armed_at.is_some()) {
                    (true, false) => {
                        self.armed_at = Some((x, y));
                        Some(SweepEvent::Armed { x, y })
                    }
                    (false, true) => {
                        self.armed_at = None;
                        Some(SweepEvent::Released { x, y })
                    }
                    _ => None,
                }
            }
            Input::Moved => {
                let (lx, ly) = self.armed_at?;
                if (x - lx).hypot(y - ly) < MIN_MOVE_DISTANCE {
                    return None;
                }
                self.armed_at = Some((x, y));
                Some(SweepEvent::Moved { x, y })
            }
            Input::TapDisabled => self.armed_at.take().map(|_| SweepEvent::Cancelled),
        }
    }
}

enum Input {
    Flags(CGEventFlags),
    Moved,
    TapDisabled,
}

unsafe extern "C-unwind" fn tap_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    let state = unsafe { &mut *user_info.cast::<SweepState>() };
    let cg_event = unsafe { event.as_ref() };
    let input = match event_type {
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            if let Some(tap) = &state.tap {
                CGEvent::tap_enable(tap, true);
            }
            Input::TapDisabled
        }
        CGEventType::FlagsChanged => Input::Flags(CGEvent::flags(Some(cg_event))),
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
