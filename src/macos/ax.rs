use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2_app_kit::NSWorkspace;
use objc2_application_services::{
    AXError, AXIsProcessTrustedWithOptions, AXUIElement, AXValue, AXValueType,
};
use objc2_core_foundation::{
    CFBoolean, CFDictionary, CFHash, CFRetained, CFString, CFType, CGPoint, CGSize,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString};

use crate::engine::WindowId;
use crate::geometry::Rect;

pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrustedWithOptions(None) }
}

pub fn request_trust() -> bool {
    let key = NSString::from_str("AXTrustedCheckOptionPrompt");
    let options: Retained<NSDictionary<NSString, NSNumber>> =
        NSDictionary::from_slices(&[&*key], &[&*NSNumber::new_bool(true)]);
    let options: *const CFDictionary = Retained::as_ptr(&options).cast();
    unsafe { AXIsProcessTrustedWithOptions(Some(&*options)) }
}

pub fn focused_window() -> Option<AxWindow> {
    let system = unsafe { AXUIElement::new_system_wide() };
    let app = copy_element(&system, &cfstr("AXFocusedApplication")).or_else(|| {
        let front = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        Some(unsafe { AXUIElement::new_application(front.processIdentifier()) })
    })?;
    copy_element(&app, &cfstr("AXFocusedWindow")).map(AxWindow)
}

pub struct AxWindow(CFRetained<AXUIElement>);

impl AxWindow {
    pub fn frame(&self) -> Option<Rect> {
        let position = copy_attribute(&self.0, &cfstr("AXPosition"))?
            .downcast::<AXValue>()
            .ok()?;
        let size = copy_attribute(&self.0, &cfstr("AXSize"))?
            .downcast::<AXValue>()
            .ok()?;
        let mut point = CGPoint::ZERO;
        let mut cg_size = CGSize::ZERO;
        let ok = unsafe {
            position.value(AXValueType::CGPoint, NonNull::from(&mut point).cast())
                && size.value(AXValueType::CGSize, NonNull::from(&mut cg_size).cast())
        };
        ok.then(|| Rect::new(point.x, point.y, cg_size.width, cg_size.height))
    }

    pub fn id(&self) -> WindowId {
        let mut pid: i32 = 0;
        unsafe { self.0.pid(NonNull::from(&mut pid)) };
        let element: &CFType = &self.0;
        let hash = CFHash(Some(element)) as u64;
        WindowId((pid as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ hash)
    }

    pub fn is_fullscreen(&self) -> bool {
        copy_attribute(&self.0, &cfstr("AXFullScreen"))
            .and_then(|value| value.downcast::<CFBoolean>().ok())
            .map(|value| value.value())
            .unwrap_or(false)
    }

    pub fn is_resizable(&self) -> bool {
        let mut settable: u8 = 0;
        let err = unsafe {
            self.0
                .is_attribute_settable(&cfstr("AXSize"), NonNull::from(&mut settable))
        };
        err == AXError::Success && settable != 0
    }

    pub fn set_frame(&self, frame: &Rect) -> Option<Rect> {
        let mut point = CGPoint::new(frame.x, frame.y);
        let mut size = CGSize::new(frame.width, frame.height);
        let point_value =
            unsafe { AXValue::new(AXValueType::CGPoint, NonNull::from(&mut point).cast()) }?;
        let size_value =
            unsafe { AXValue::new(AXValueType::CGSize, NonNull::from(&mut size).cast()) }?;
        unsafe {
            self.0
                .set_attribute_value(&cfstr("AXPosition"), &point_value);
            self.0.set_attribute_value(&cfstr("AXSize"), &size_value);
            self.0
                .set_attribute_value(&cfstr("AXPosition"), &point_value);
        }
        self.frame()
    }
}

fn copy_element(element: &AXUIElement, attribute: &CFString) -> Option<CFRetained<AXUIElement>> {
    copy_attribute(element, attribute)?
        .downcast::<AXUIElement>()
        .ok()
}

fn copy_attribute(element: &AXUIElement, attribute: &CFString) -> Option<CFRetained<CFType>> {
    let mut value: *const CFType = std::ptr::null();
    let err = unsafe { element.copy_attribute_value(attribute, NonNull::from(&mut value)) };
    if err != AXError::Success {
        return None;
    }
    NonNull::new(value.cast_mut()).map(|p| unsafe { CFRetained::from_raw(p) })
}

fn cfstr(name: &'static str) -> CFRetained<CFString> {
    CFString::from_static_str(name)
}
