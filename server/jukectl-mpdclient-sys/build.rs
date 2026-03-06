use std::env;
use std::path::PathBuf;
use std::fs;
use std::process::Command;

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let lib_src_dir = out_dir.join("libmpdclient-src");

    // Fetch the source if it doesn't exist
    if !lib_src_dir.exists() {
        println!("cargo:warning=Fetching libmpdclient source to OUT_DIR...");
        let status = Command::new("git")
            .args(&[
                "clone",
                "--depth", "1",
                "--branch", "v2.22",
                "https://github.com/MusicPlayerDaemon/libmpdclient.git",
                lib_src_dir.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to execute git clone");

        if !status.success() {
            panic!("Failed to clone libmpdclient repository");
        }
    }

    let include_path = lib_src_dir.join("include");
    let src_path = lib_src_dir.join("src");

    // Create dummy version.h if it doesn't exist
    let version_h = include_path.join("mpd/version.h");
    if !version_h.exists() {
        fs::create_dir_all(include_path.join("mpd")).expect("Failed to create mpd include dir");
        let content = "#ifndef MPD_VERSION_H\n#define MPD_VERSION_H\n#define LIBMPDCLIENT_MAJOR_VERSION 2\n#define LIBMPDCLIENT_MINOR_VERSION 22\n#define LIBMPDCLIENT_PATCH_VERSION 0\n#endif\n";
        fs::write(&version_h, content).expect("Failed to write dummy version.h");
    }

    // Compile libmpdclient
    let mut build = cc::Build::new();
    build.include(&include_path);
    build.include(&src_path);
    build.include(&root); // For local config.h
    
    // Automatically find all .c files in src/ to avoid fragile explicit lists
    for entry in fs::read_dir(&src_path).expect("Failed to read src dir") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "c") {
            let filename = path.file_name().unwrap().to_str().unwrap();
            // Skip example.c and test files if any
            if filename != "example.c" && !filename.starts_with("t_") {
                build.file(&path);
            }
        }
    }

    build.compile("mpdclient");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header(include_path.join("mpd/client.h").to_str().unwrap())
        .clang_arg(format!("-I{}", include_path.to_str().unwrap()))
        .allowlist_function("mpd_.*")
        .allowlist_type("mpd_.*")
        .allowlist_var("MPD_.*")
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=config.h");
}
