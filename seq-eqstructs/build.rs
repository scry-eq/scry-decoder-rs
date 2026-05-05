use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Default: sibling-relative path (mirrors showeq-web → ../showeq-proto).
    // Override with EVERQUEST_H for CI / out-of-tree builds.
    let header = match env::var("EVERQUEST_H") {
        Ok(p) => PathBuf::from(p),
        Err(_) => manifest_dir.join("../../showeq-daemon/src/everquest.h"),
    };
    let include_dir = header.parent().expect("everquest.h must have a parent dir");

    if !header.exists() {
        panic!(
            "everquest.h not found at {}. Set EVERQUEST_H or check out showeq-daemon as a sibling of showeq-decoder-rs.",
            header.display()
        );
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rerun-if-env-changed=EVERQUEST_H");

    // Allowlist grows as opcodes are ported. Keep it explicit so bindgen
    // doesn't drag in 117 structs (most of which we don't parse in Rust
    // yet) and so unrelated header churn doesn't ripple here.
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", include_dir.display()))
        .clang_arg("-x").clang_arg("c++")
        .clang_arg("-std=c++17")
        .allowlist_type("spawnPositionUpdate")
        .allowlist_type("deleteSpawnStruct")
        .allowlist_type("removeSpawnStruct")
        .allowlist_type("hpNpcUpdateStruct")
        .allowlist_type("mobHealthStruct")
        .allowlist_type("spawnAppearanceStruct")
        .allowlist_type("expUpdateStruct")
        .allowlist_type("levelUpUpdateStruct")
        .allowlist_type("skillIncStruct")
        .allowlist_type("manaDecrementStruct")
        .allowlist_type("staminaStruct")
        .allowlist_type("endUpdateStruct")
        .allowlist_type("considerStruct")
        .allowlist_type("spawnRenameStruct")
        .allowlist_type("clientTargetStruct")
        .allowlist_type("newCorpseStruct")
        .derive_default(true)
        .derive_copy(true)
        .layout_tests(true)
        .generate()
        .expect("bindgen failed on everquest.h");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    bindings.write_to_file(&out).expect("write bindings.rs");
}
