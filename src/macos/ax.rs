use std::ptr::NonNull;

use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
use objc2_application_services::{
    AXError, AXIsProcessTrustedWithOptions, AXUIElement, AXValue, AXValueType,
    kAXTrustedCheckOptionPrompt,
};
use objc2_core_foundation::{
    CFBoolean, CFDictionary, CFEqual, CFRetained, CFString, CFType, CGPoint, CGSize, Type,
};

use crate::engine::WindowId;
use crate::geometry::Rect;

pub struct AxWindow(CFRetained<AXUIElement>);

impl AxWindow {
    pub fn is_fullscreen(&self) -> bool {
        copy_attribute(&self.0, &cfstr("AXFullScreen"))
            .and_then(|value| value.downcast::<CFBoolean>().ok())
            .is_some_and(|value| value.value())
    }

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

    pub fn id(&self) -> Option<WindowId> {
        unsafe extern "C" {
            fn _AXUIElementGetWindow(element: &AXUIElement, window: NonNull<u32>) -> AXError;
        }
        let mut id: u32 = 0;
        let err = unsafe { _AXUIElementGetWindow(&self.0, NonNull::from(&mut id)) };
        (err == AXError::Success).then_some(WindowId(u64::from(id)))
    }

    pub fn is_resizable(&self) -> bool {
        let mut settable: u8 = 0;
        let err = unsafe {
            self.0
                .is_attribute_settable(&cfstr("AXSize"), NonNull::from(&mut settable))
        };
        err == AXError::Success && settable != 0
    }

    pub fn set_frame(&self, frame: &Rect) -> Result<Rect, AXError> {
        let origin = CGPoint::new(frame.x, frame.y);
        let size = CGSize::new(frame.width, frame.height);
        self.set_value("AXPosition", AXValueType::CGPoint, origin)?;
        self.set_value("AXSize", AXValueType::CGSize, size)?;
        self.set_value("AXPosition", AXValueType::CGPoint, origin)?;
        Ok(self.frame().unwrap_or(*frame))
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

    pub fn raise(&self) {
        unsafe {
            self.0
                .set_attribute_value(&cfstr("AXMain"), CFBoolean::new(true));
            self.0.perform_action(&cfstr("AXRaise"));
        }
        let mut pid: i32 = 0;
        let err = unsafe { self.0.pid(NonNull::from(&mut pid)) };
        if err == AXError::Success
            && let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        {
            #[allow(deprecated)]
            app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
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
    copied(err, value)
}

fn copied<T: Type>(err: AXError, raw: *const T) -> Option<CFRetained<T>> {
    if err != AXError::Success {
        return None;
    }
    NonNull::new(raw.cast_mut()).map(|p| unsafe { CFRetained::from_raw(p) })
}

pub fn window_at(x: f64, y: f64) -> Option<AxWindow> {
    let system = unsafe { AXUIElement::new_system_wide() };
    let mut raw: *const AXUIElement = std::ptr::null();
    let err =
        unsafe { system.copy_element_at_position(x as f32, y as f32, NonNull::from(&mut raw)) };
    let element = copied(err, raw)?;
    let is_window = copy_attribute(&element, &cfstr("AXRole"))
        .is_some_and(|role| CFEqual(Some(&role), Some(&cfstr("AXWindow"))));
    if is_window {
        return Some(AxWindow(element));
    }
    copy_element(&element, &cfstr("AXWindow")).map(AxWindow)
}
