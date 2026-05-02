use std::path::Path;

use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
use winreg::RegKey;

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "BreakReminderRs";

/// Reconcile the HKCU Run-key entry with the desired state.
///
/// `enabled = true`  → write `"<exe path>"` (full path, quoted).
/// `enabled = false` → delete the value if it exists.
///
/// All errors are logged to stderr and swallowed — autostart is
/// non-essential. The app must keep running even if registry writes fail.
pub fn apply(enabled: bool, exe_path: &Path) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = match hkcu.open_subkey_with_flags(RUN_KEY_PATH, KEY_SET_VALUE) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("[break_reminder_rs] failed to open Run key: {e}");
            return;
        }
    };

    if enabled {
        let Some(path_str) = exe_path.to_str() else {
            eprintln!("[break_reminder_rs] exe path is not valid UTF-8");
            return;
        };
        let quoted = format!("\"{path_str}\"");
        if let Err(e) = run_key.set_value(VALUE_NAME, &quoted) {
            eprintln!("[break_reminder_rs] failed to set Run value: {e}");
        }
    } else {
        match run_key.delete_value(VALUE_NAME) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {} // already absent
            Err(e) => eprintln!("[break_reminder_rs] failed to delete Run value: {e}"),
        }
    }
}
