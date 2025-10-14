use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2_app_kit::NSWorkspace;
use objc2_application_services::{
    AXError, AXIsProcessTrustedWithOptions, AXUIElement, AXValue, AXValueType,
};
use objc2_core_foundation::{CFDictionary, CFRetained, CFString, CFType, CGPoint, CGSize};
use objc2_foundation::{NSDictionary, NSNumber, NSString};

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

    pub fn set_frame(&self, frame: &Rect) -> bool {
        let mut point = CGPoint::new(frame.x, frame.y);
        let mut size = CGSize::new(frame.width, frame.height);
        let Some(point_value) =
            (unsafe { AXValue::new(AXValueType::CGPoint, NonNull::from(&mut point).cast()) })
        else {
            return false;
        };
        let Some(size_value) =
            (unsafe { AXValue::new(AXValueType::CGSize, NonNull::from(&mut size).cast()) })
        else {
            return false;
        };
        unsafe {
            self.0
                .set_attribute_value(&cfstr("AXPosition"), &point_value)
                == AXError::Success
                && self.0.set_attribute_value(&cfstr("AXSize"), &size_value) == AXError::Success
        }
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
