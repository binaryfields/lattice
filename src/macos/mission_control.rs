use objc2_core_foundation::{CFDictionary, CFEqual, CFNumber, CFString, CFType};
use objc2_core_graphics::{
    CGWindowListCopyWindowInfo, CGWindowListOption, kCGNullWindowID, kCGWindowLayer,
    kCGWindowNumber, kCGWindowOwnerName,
};

/// Whether Mission Control has opened since a drag captured its window.
///
/// macOS 26 draws Mission Control through WindowManager: a shield over each
/// display, a Spaces Bar and one thumbnail per desktop, all at positive window
/// levels, up within about 100 ms of a dragged window touching the top edge
/// and gone again once the drop is handled. Whatever WindowManager shows all
/// the time, such as the wallpaper, is in the baseline and ignored. The window
/// list reports owners and levels without Screen Recording permission; window
/// names it does not, so none are relied on.
pub struct MissionControl {
    baseline: Vec<i64>,
}

impl MissionControl {
    pub fn new() -> MissionControl {
        MissionControl {
            baseline: window_manager_windows(),
        }
    }

    pub fn is_open(&self) -> bool {
        window_manager_windows()
            .into_iter()
            .any(|id| !self.baseline.contains(&id))
    }
}

/// Numbers of WindowManager's on-screen windows above the desktop level.
fn window_manager_windows() -> Vec<i64> {
    let Some(list) =
        CGWindowListCopyWindowInfo(CGWindowListOption::OptionOnScreenOnly, kCGNullWindowID)
    else {
        return Vec::new();
    };
    let owner = CFString::from_static_str("WindowManager");
    let list = unsafe { list.cast_unchecked::<CFType>() };
    list.iter()
        .filter_map(|info| {
            let info = info.downcast::<CFDictionary>().ok()?;
            let info = unsafe { info.cast_unchecked::<CFType, CFType>() };
            let owned = info
                .get(unsafe { kCGWindowOwnerName })
                .is_some_and(|name| CFEqual(Some(&name), Some(&owner)));
            let raised = number(info, unsafe { kCGWindowLayer })? > 0;
            (owned && raised).then(|| number(info, unsafe { kCGWindowNumber }))?
        })
        .collect()
}

fn number(info: &CFDictionary<CFType, CFType>, key: &CFString) -> Option<i64> {
    info.get(key)?.downcast::<CFNumber>().ok()?.as_i64()
}
