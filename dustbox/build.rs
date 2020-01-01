// windows build to pick up gtk-3 libs installed with vcpkg correctly
// jan 2019 still an issue, see https://github.com/gtk-rs/gtk/issues/702#issuecomment-438049273

#[cfg(windows)]
extern crate vcpkg;

#[cfg(windows)]
fn ensure_lib_file_win(lib_path: &str, src_name: &str, dst_name: &str) {
    let src_path = std::path::Path::new(lib_path).join(src_name);
    let dst_path = std::path::Path::new(lib_path).join(dst_name);

    if !dst_path.exists() {
        std::fs::copy(src_path, dst_path).unwrap();
    }
}

#[cfg(windows)]
fn win_main() {
    let target_triple = std::env::var("TARGET").unwrap();
    println!("XXX VCPKG_PATH IS {}", std::env::var("VCPKG_ROOT").unwrap());
    if target_triple == "x86_64-pc-windows-msvc" {
        std::env::set_var("GTK_LIB_DIR",
            std::path::Path::new(&std::env::var("VCPKG_ROOT").unwrap()).join("installed\\x64-windows\\lib"));
    } else if target_triple == "i686-pc-windows-msvc" {
        std::env::set_var("GTK_LIB_DIR",
            std::path::Path::new(&std::env::var("VCPKG_ROOT").unwrap()).join("installed\\x86-windows\\lib"));
    } else {
        panic!("");
    }

    let lib_path = std::env::var("GTK_LIB_DIR").unwrap();
    ensure_lib_file_win(&lib_path, "gtk-3.0.lib", "gtk-3.lib");
    ensure_lib_file_win(&lib_path, "gdk-3.0.lib", "gdk-3.lib");

    vcpkg::find_package("gtk").unwrap();
    vcpkg::find_package("glib").unwrap();
    vcpkg::find_package("harfbuzz").unwrap();
}

fn main() {
    #[cfg(windows)]
    win_main();
}