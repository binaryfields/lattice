mod ax;
mod shell;

pub use ax::*;
pub use shell::*;

use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;

use crate::geometry::Rect;

pub fn main_visible_frame() -> Option<Rect> {
    let mtm = MainThreadMarker::new()?;
    let screens = NSScreen::screens(mtm);
    let primary_display_height = screens.iter().next()?.frame().size.height;
    let vf = NSScreen::mainScreen(mtm)?.visibleFrame();
    Some(
        Rect::new(vf.origin.x, vf.origin.y, vf.size.width, vf.size.height)
            .flip_vertical(primary_display_height),
    )
}
