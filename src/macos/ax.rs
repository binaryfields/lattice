use std::ptr::NonNull;

use objc2_app_kit::NSWorkspace;
use objc2_application_services::{
    AXError, AXIsProcessTrustedWithOptions, AXUIElement, AXValue, AXValueType,
    kAXTrustedCheckOptionPrompt,
};
use objc2_core_foundation::{
    CFBoolean, CFDictionary, CFRetained, CFString, CFType, CGPoint, CGSize,
};

use crate::geometry::Rect;

pub struct AxWindow(CFRetained<AXUIElement>);

impl AxWindow {
    pub fn frame(&self) -> Option<Rect> {
        let position = copy_attribute(&self.0, &cfstr("AXPosition"))?
            .downcast::<AXValue>()
            .ok()?;
        let extent = copy_attribute(&self.0, &cfstr("AXSize"))?
            .downcast::<AXValue>()
            .ok()?;
        let (mut origin, mut size) = (CGPoint::ZERO, CGSize::ZERO);
        let read = unsafe {
            position.value(AXValueType::CGPoint, NonNull::from(&mut origin).cast())
                && extent.value(AXValueType::CGSize, NonNull::from(&mut size).cast())
        };
        read.then(|| Rect::new(origin.x, origin.y, size.width, size.height))
    }

    pub fn set_frame(&self, frame: &Rect) -> Result<(), AXError> {
        let origin = CGPoint::new(frame.x, frame.y);
        let size = CGSize::new(frame.width, frame.height);
        self.set_value("AXPosition", AXValueType::CGPoint, origin)?;
        self.set_value("AXSize", AXValueType::CGSize, size)
    }

    fn set_value<T>(
        &self,
        attribute: &'static str,
        kind: AXValueType,
        mut value: T,
    ) -> Result<(), AXError> {
        let value = unsafe { AXValue::new(kind, NonNull::from(&mut value).cast()) }
            .ok_or(AXError::Failure)?;
        match unsafe { self.0.set_attribute_value(&cfstr(attribute), &value) } {
            AXError::Success => Ok(()),
            err => Err(err),
        }
    }
}

pub fn request_trust() -> bool {
    let prompt = unsafe { kAXTrustedCheckOptionPrompt };
    let options = CFDictionary::from_slices(&[prompt], &[CFBoolean::new(true)]);
    unsafe { AXIsProcessTrustedWithOptions(Some(options.as_opaque())) }
}

pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrustedWithOptions(None) }
}

pub fn focused_window() -> Option<AxWindow> {
    let system = unsafe { AXUIElement::new_system_wide() };
    let app = copy_element(&system, &cfstr("AXFocusedApplication")).or_else(|| {
        let front = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        Some(unsafe { AXUIElement::new_application(front.processIdentifier()) })
    })?;
    copy_element(&app, &cfstr("AXFocusedWindow")).map(AxWindow)
}

fn copy_element(element: &AXUIElement, attribute: &CFString) -> Option<CFRetained<AXUIElement>> {
    copy_attribute(element, attribute)?
        .downcast::<AXUIElement>()
        .ok()
}

fn cfstr(name: &'static str) -> CFRetained<CFString> {
    CFString::from_static_str(name)
}

fn copy_attribute(element: &AXUIElement, attribute: &CFString) -> Option<CFRetained<CFType>> {
    let mut value: *const CFType = std::ptr::null();
    let err = unsafe { element.copy_attribute_value(attribute, NonNull::from(&mut value)) };
    if err != AXError::Success {
        return None;
    }
    NonNull::new(value.cast_mut()).map(|p| unsafe { CFRetained::from_raw(p) })
}
