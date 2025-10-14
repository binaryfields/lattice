use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSBackingStoreType, NSBox, NSBoxType, NSColor, NSScreen,
    NSStatusWindowLevel, NSTitlePosition, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};

use crate::geometry::Rect;

const RING_SIZE: f64 = 24.0;

pub struct Overlay {
    ring: Retained<NSWindow>,
    outline: Retained<NSWindow>,
}

impl Overlay {
    pub fn new() -> Option<Overlay> {
        let mtm = MainThreadMarker::new()?;
        Some(Overlay {
            ring: overlay_window(mtm, 2.0, RING_SIZE / 2.0),
            outline: overlay_window(mtm, 2.5, 10.0),
        })
    }

    pub fn show_ring(&self, x: f64, y: f64) {
        let rect = Rect::new(
            x - RING_SIZE / 2.0,
            y - RING_SIZE / 2.0,
            RING_SIZE,
            RING_SIZE,
        );
        show_at(&self.ring, &rect);
    }

    pub fn show_outline(&self, frame: &Rect) {
        show_at(&self.outline, frame);
    }

    pub fn hide_outline(&self) {
        self.outline.orderOut(None);
    }

    pub fn hide(&self) {
        self.ring.orderOut(None);
        self.outline.orderOut(None);
    }
}

fn overlay_window(mtm: MainThreadMarker, border: f64, corner: f64) -> Retained<NSWindow> {
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            CGRect::ZERO,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { window.setReleasedWhenClosed(false) };
    window.setLevel(NSStatusWindowLevel);
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setIgnoresMouseEvents(true);
    window.setHasShadow(false);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );

    let border_box = NSBox::initWithFrame(NSBox::alloc(mtm), CGRect::ZERO);
    border_box.setBoxType(NSBoxType::Custom);
    border_box.setTitlePosition(NSTitlePosition::NoTitle);
    border_box.setBorderWidth(border);
    border_box.setCornerRadius(corner);
    border_box.setBorderColor(&NSColor::controlAccentColor());
    border_box.setFillColor(&NSColor::clearColor());
    border_box.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    window.setContentView(Some(&border_box));
    window
}

fn show_at(window: &NSWindow, frame: &Rect) {
    let Some(appkit) = appkit_rect(frame) else {
        return;
    };
    window.setFrame_display(appkit, true);
    window.orderFrontRegardless();
}

fn appkit_rect(rect: &Rect) -> Option<CGRect> {
    let mtm = MainThreadMarker::new()?;
    let primary_display_height = NSScreen::screens(mtm).iter().next()?.frame().size.height;
    let flipped = rect.flip_vertical(primary_display_height);
    Some(CGRect::new(
        CGPoint::new(flipped.x, flipped.y),
        CGSize::new(flipped.width, flipped.height),
    ))
}
