use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/icon.png");
    println!("cargo:rerun-if-changed=icons/32x32.png");
    tauri_build::build();
    copy_libmpv_dlls();
}

/// Copy libmpv DLLs next to the binary and under `lib/` so LoadLibrary finds them
/// regardless of process working directory (cargo run / tauri dev vs direct exe).
fn copy_libmpv_dlls() {
    #[cfg(not(target_os = "windows"))]
    return;

    #[cfg(target_os = "windows")]
    {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let lib_src = manifest_dir.join("lib");
        let dlls = ["libmpv-2.dll", "libmpv-wrapper.dll"];

        let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
        // OUT_DIR = target/{profile}/build/{pkg}-{hash}/out
        let target_dir = out_dir.join("../../..");
        let target_lib = target_dir.join("lib");

        if fs::create_dir_all(&target_lib).is_err() {
            return;
        }

        for dll in dlls {
            let src = lib_src.join(dll);
            println!("cargo:rerun-if-changed=lib/{dll}");
            if !src.exists() {
                panic!(
                    "Missing {dll} in src-tauri/lib — run: npm run setup:lib:manual (or npm run setup:lib)"
                );
            }
            // Copy when the destination is missing or differs in size, so a
            // swapped libmpv build (e.g. GPL -> LGPL) is never left stale.
            for dest in [target_dir.join(dll), target_lib.join(dll)] {
                copy_if_stale(&src, &dest);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn copy_if_stale(src: &std::path::Path, dest: &std::path::Path) {
    let src_len = fs::metadata(src).map(|m| m.len()).ok();
    let dest_len = fs::metadata(dest).map(|m| m.len()).ok();
    if src_len.is_some() && src_len == dest_len {
        return;
    }
    fs::copy(src, dest).unwrap_or_else(|err| {
        panic!("Failed to copy {} to {}: {err}", src.display(), dest.display())
    });
}
