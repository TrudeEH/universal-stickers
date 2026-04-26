fn main() {
    cxx_build::bridges(["src/lib.rs"])
        .flag_if_supported("/std:c++17")
        .flag_if_supported("-std=c++17")
        .compile("universal_stickers_ffi");

    println!("cargo:rerun-if-changed=src/lib.rs");
}
