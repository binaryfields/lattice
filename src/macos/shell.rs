use std::path::Path;
use std::process::Command;

use objc2::rc::Retained;
use objc2_service_management::{SMAppService, SMAppServiceStatus};
use winit::event_loop::EventLoopBuilder;
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

pub fn set_accessory_activation_policy<T>(builder: &mut EventLoopBuilder<T>) {
    builder.with_activation_policy(ActivationPolicy::Accessory);
}

pub fn open_in_default_app(path: &Path) -> std::io::Result<()> {
    Command::new("open").arg(path).spawn().map(|_| ())
}

pub fn starts_at_login() -> bool {
    let status = unsafe { login_service().status() };
    status == SMAppServiceStatus::Enabled
}

pub fn set_start_at_login(enable: bool) -> bool {
    let service = login_service();
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

    let status = unsafe { service.status() };
    if enable && status == SMAppServiceStatus::RequiresApproval {
        unsafe { SMAppService::openSystemSettingsLoginItems() };
    }
    status == SMAppServiceStatus::Enabled
}

fn login_service() -> Retained<SMAppService> {
    unsafe { SMAppService::mainAppService() }
}
