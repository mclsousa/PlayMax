// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // libmpv resolves DLLs from the executable directory; cargo run / tauri dev may
    // start the process with a different working directory than a direct exe launch.
    #[cfg(windows)]
    set_exe_working_directory();

    play_max_lib::run()
}

#[cfg(windows)]
fn set_exe_working_directory() {
    use std::os::windows::ffi::OsStrExt;

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let _ = std::env::set_current_dir(exe_dir);
            let wide: Vec<u16> = exe_dir
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                SetDllDirectoryW(wide.as_ptr());
            }
        }
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
}
