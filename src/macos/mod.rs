use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;

use crate::geometry::Rect;

pub use ax::*;
pub use mission_control::*;
pub use mouse::*;
pub use overlay::*;
pub use shell::*;

mod ax;
mod mission_control;
mod mouse;
mod overlay;
mod shell;

pub fn visible_frames() -> Vec<Rect> {
    let Some(mtm) = MainThreadMarker::new() else {
        return Vec::new();
    };
    let screens = NSScreen::screens(mtm);
    let Some(primary_display_height) = screens.iter().next().map(|s| s.frame().size.height) else {
        return Vec::new();
    };
    screens
        .iter()
        .map(|screen| {
            let vf = screen.visibleFrame();
            Rect::new(vf.origin.x, vf.origin.y, vf.size.width, vf.size.height)
                .flip_vertical(primary_display_height)
        })
        .collect()
}
