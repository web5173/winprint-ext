use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

#[non_exhaustive]
pub struct ComInitializer {
    initialized: bool,
}
impl ComInitializer {
    pub fn new() -> ComInitializer {
        let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
        ComInitializer { initialized }
    }
}
impl Drop for ComInitializer {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
