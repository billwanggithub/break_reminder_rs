// Windows system "Exclamation" sound — matches WPF SystemSounds.Exclamation.Play().
//
// MessageBeep returns immediately; the sound plays asynchronously on a system
// thread. The return value is the BOOL result; we discard it because the only
// realistic failure is "the user set Exclamation to (None) in Sound Settings",
// which is silence-by-design, not an app error.

const MB_ICONEXCLAMATION: u32 = 0x00000030;

extern "system" {
    fn MessageBeep(utype: u32) -> i32;
}

pub fn play_alert() {
    unsafe {
        let _ = MessageBeep(MB_ICONEXCLAMATION);
    }
}
