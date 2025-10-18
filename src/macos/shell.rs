use std::path::Path;
use std::process::Command;

use winit::event_loop::EventLoopBuilder;
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

pub fn set_accessory_activation_policy<T>(builder: &mut EventLoopBuilder<T>) {
    builder.with_activation_policy(ActivationPolicy::Accessory);
}

pub fn open_in_default_app(path: &Path) -> std::io::Result<()> {
    Command::new("open").arg(path).spawn().map(|_| ())
}
