use std::{env, path::PathBuf};

static COMPILER_ARGS: [&str; 6] = [
    "-march=armv6k",
    "-mtune=mpcore",
    "-mfloat-abi=hard",
    "-mtp=soft",
    "-D__3DS__",
    "-nostdinc",
];

static COMPILER_LIBDIRS: [&str; 5] = [
    "portlibs/3ds/include",
    "devkitARM/lib/gcc/arm-none-eabi/12.2.0/include",
    "devkitARM/lib/gcc/arm-none-eabi/12.2.0/include-fixed",
    "devkitARM/arm-none-eabi/include",
    "libctru/include",
];

fn do_bindgen(dkp: &str, name: &str) {
    println!("cargo:rerun-if-changed=bindgen/{name}.h");
    let mut builder = bindgen::Builder::default()
        .header(format!("bindgen/{name}.h"))
        .detect_include_paths(false);
    for arg in COMPILER_ARGS {
        builder = builder.clang_arg(arg);
    }
    for dir in COMPILER_LIBDIRS {
        builder = builder.clang_arg(format!("-I{dkp}/{dir}"));
    }
    let bindings = builder
        .blocklist_file(".*/(?:mii|miiselector|frd)\\.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join(format!("{name}.rs")))
        .expect("failed to write bindings");
}

fn main() {
    // add C file to allow calling inlined functions
    let out = env::var("OUT_DIR").unwrap();
    let dkp = env::var("DEVKITPRO").unwrap();
    let mut build = cc::Build::new();
    println!("cargo:rerun-if-changed=bindgen/toot3d.c");
    build.file("bindgen/toot3d.c");
    build.compiler("arm-none-eabi-gcc");
    for arg in COMPILER_ARGS {
        build.flag(arg);
    }
    for dir in COMPILER_LIBDIRS {
        build.flag(&format!("-I{dkp}/{dir}"));
    }
    build.compile("libtoot3d.a");
    // link curl, citro2d, and custom library
    println!("cargo:rustc-link-search={dkp}/portlibs/3ds/lib");
    println!("cargo:rustc-link-search={dkp}/libctru/lib");
    println!("cargo:rustc-link-search={out}");
    println!("cargo:rustc-link-lib=curl");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=mbedtls");
    println!("cargo:rustc-link-lib=mbedx509");
    println!("cargo:rustc-link-lib=mbedcrypto");
    println!("cargo:rustc-link-lib=citro2d");
    println!("cargo:rustc-link-lib=citro3d");
    println!("cargo:rustc-link-lib=toot3d");
    // run bindgen for libraries
    do_bindgen(&dkp, "citro2d");
    do_bindgen(&dkp, "curl");
}
