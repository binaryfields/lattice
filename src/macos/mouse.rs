use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_core_foundation::{CFMachPort, CFRetained, CFRunLoop, kCFRunLoopCommonModes};
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventMask, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventTapProxy, CGEventType,
};

use crate::config::Modifiers;

const MIN_MOVE_DISTANCE: f64 = 2.0;

pub struct SweepMonitor {
    state: *mut SweepState,
}

struct SweepState {
    tap: Option<CFRetained<CFMachPort>>,
    armed: bool,
    last_emitted: (f64, f64),
    arm_mods: Modifiers,
    enabled: bool,
    emit: Box<dyn Fn(SweepEvent)>,
}

impl SweepMonitor {
    pub fn install(
        arm_mods: Modifiers,
        enabled: bool,
        emit: Box<dyn Fn(SweepEvent)>,
    ) -> Option<SweepMonitor> {
        let state = Box::into_raw(Box::new(SweepState {
            tap: None,
            armed: false,
            last_emitted: (0.0, 0.0),
            arm_mods,
            enabled,
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
        let Some(source) = source else {
            eprintln!("lattice: could not install the sweep monitor; sweep disabled");
            drop(unsafe { Box::from_raw(state) });
            return None;
        };
        unsafe {
            (*state).tap = tap;
            if let Some(run_loop) = CFRunLoop::main() {
                run_loop.add_source(Some(&source), kCFRunLoopCommonModes);
            }
        }
        Some(SweepMonitor { state })
    }

    pub fn set_config(&self, arm_mods: Modifiers, enabled: bool) {
        unsafe {
            (*self.state).arm_mods = arm_mods;
            (*self.state).enabled = enabled;
            if !enabled {
                (*self.state).armed = false;
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SweepEvent {
    Armed { x: f64, y: f64 },
    Moved { x: f64, y: f64 },
    Released { x: f64, y: f64 },
}

fn event_bit(event_type: CGEventType) -> CGEventMask {
    1u64 << event_type.0
}

unsafe extern "C-unwind" fn tap_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    let state = unsafe { &mut *user_info.cast::<SweepState>() };
    let event = unsafe { event.as_ref() };
    match event_type {
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            if let Some(tap) = &state.tap {
                CGEvent::tap_enable(tap, true);
            }
            state.armed = false;
        }
        CGEventType::FlagsChanged => {
            let held = state.enabled && mods_held(CGEvent::flags(Some(event)), state.arm_mods);
            let loc = CGEvent::location(Some(event));
            if held && !state.armed {
                state.armed = true;
                state.last_emitted = (loc.x, loc.y);
                (state.emit)(SweepEvent::Armed { x: loc.x, y: loc.y });
            } else if !held && state.armed {
                state.armed = false;
                (state.emit)(SweepEvent::Released { x: loc.x, y: loc.y });
            }
        }
        CGEventType::MouseMoved | CGEventType::LeftMouseDragged if state.armed => {
            let loc = CGEvent::location(Some(event));
            let (lx, ly) = state.last_emitted;
            if (loc.x - lx).hypot(loc.y - ly) >= MIN_MOVE_DISTANCE {
                state.last_emitted = (loc.x, loc.y);
                (state.emit)(SweepEvent::Moved { x: loc.x, y: loc.y });
            }
        }
        _ => {}
    }
    std::ptr::from_ref(event).cast_mut()
}

fn mods_held(flags: CGEventFlags, arm_mods: Modifiers) -> bool {
    (!arm_mods.ctrl || flags.contains(CGEventFlags::MaskControl))
        && (!arm_mods.alt || flags.contains(CGEventFlags::MaskAlternate))
        && (!arm_mods.shift || flags.contains(CGEventFlags::MaskShift))
        && (!arm_mods.cmd || flags.contains(CGEventFlags::MaskCommand))
}
