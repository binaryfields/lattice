use std::ffi::OsStr;
use std::process::Command;

use objc2_service_management::{SMAppService, SMAppServiceStatus};
use winit::event_loop::EventLoopBuilder;
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

pub const ACCESSIBILITY_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

pub fn set_accessory_activation_policy<T>(builder: &mut EventLoopBuilder<T>) {
    builder.with_activation_policy(ActivationPolicy::Accessory);
}

pub fn starts_at_login() -> bool {
    let status = unsafe { SMAppService::mainAppService().status() };
    status == SMAppServiceStatus::Enabled
}

pub fn set_start_at_login(enable: bool) {
    let service = unsafe { SMAppService::mainAppService() };
    let result = unsafe {
        if enable {
            service.registerAndReturnError()
        } else {
            service.unregisterAndReturnError()
        }
    };
    if let Err(err) = result {
        let verb = if enable { "enable" } else { "disable" };
        eprintln!("lattice: could not {verb} start-at-login: {err}");
    }

    if enable && unsafe { service.status() } == SMAppServiceStatus::RequiresApproval {
        unsafe { SMAppService::openSystemSettingsLoginItems() };
    }
}

pub fn open(target: impl AsRef<OsStr>) {
    let target = target.as_ref();
    if let Err(err) = Command::new("open").arg(target).spawn() {
        eprintln!("lattice: could not open {target:?}: {err}");
    }
}
