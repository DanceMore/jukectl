use std::env;
use std::path::PathBuf;
use std::fs;

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let libmpdclient_path = root.join("../../vendor/libmpdclient");
    let include_path = libmpdclient_path.join("include");
    let src_path = libmpdclient_path.join("src");

    // Create dummy version.h if it doesn't exist (since we're not using meson)
    let version_h = include_path.join("mpd/version.h");
    if !version_h.exists() {
        let content = "#ifndef MPD_VERSION_H\n#define MPD_VERSION_H\n#define LIBMPDCLIENT_MAJOR_VERSION 2\n#define LIBMPDCLIENT_MINOR_VERSION 22\n#define LIBMPDCLIENT_PATCH_VERSION 0\n#endif\n";
        fs::write(&version_h, content).expect("Failed to write dummy version.h");
    }

    // Compile libmpdclient
    let mut build = cc::Build::new();
    build.include(&include_path);
    build.include(&src_path);
    build.include(&root); // For config.h
    
    // Updated source files based on directory listing
    let src_files = [
        "albumart.c", "async.c", "audio_format.c", "binary.c", "capabilities.c",
        "cmessage.c", "cmount.c", "cneighbor.c", "connection.c", "coutput.c",
        "cpartition.c", "cplaylist.c", "cstats.c", "cstatus.c", "database.c",
        "directory.c", "entity.c", "error.c", "fd_util.c", "feature.c",
        "fingerprint.c", "idle.c", "ierror.c", "iso8601.c", "kvlist.c",
        "list.c", "message.c", "mixer.c", "mount.c", "neighbor.c",
        "output.c", "parser.c", "partition.c", "password.c", "player.c",
        "playlist.c", "position.c", "queue.c", "quote.c", "rdirectory.c",
        "readpicture.c", "recv.c", "replay_gain.c", "request.c", "resolver.c",
        "response.c", "rplaylist.c", "run.c", "search.c", "send.c",
        "settings.c", "socket.c", "song.c", "stats.c", "status.c",
        "sticker.c", "stringnormalization.c", "sync.c", "tag.c",
    ];

    for file in src_files {
        build.file(src_path.join(file));
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

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rerun-if-changed=../../vendor/libmpdclient");
    println!("cargo:rerun-if-changed=config.h");
}
