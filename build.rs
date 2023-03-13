use std::{path::PathBuf, env};

fn main() {
    // link mbedtls
    let dkp = env::var("DEVKITPRO").unwrap();
    println!("cargo:rustc-link-search={dkp}/portlibs/3ds/lib");
    println!("cargo:rustc-link-lib=mbedtls");
    println!("cargo:rustc-link-lib=mbedx509");
    println!("cargo:rustc-link-lib=mbedcrypto");

    println!("cargo:rerun-if-changed=bindgen/mbedtls.h");
    let bindings = bindgen::Builder::default()
        .header("bindgen/mbedtls.h")
        .detect_include_paths(false)
        .clang_arg("-march=armv6k")
        .clang_arg("-mtune=mpcore")
        .clang_arg("-mfloat-abi=hard")
        .clang_arg("-mtp=soft")
        .clang_arg("-D__3DS__")
        .clang_arg("-nostdinc")
        .clang_arg(format!("-I{dkp}/portlibs/3ds/include"))
        .clang_arg(format!("-I{dkp}/devkitARM/lib/gcc/arm-none-eabi/12.2.0/include"))
        .clang_arg(format!("-I{dkp}/devkitARM/lib/gcc/arm-none-eabi/12.2.0/include-fixed"))
        .clang_arg(format!("-I{dkp}/devkitARM/arm-none-eabi/include"))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("mbedtls.rs"))
        .expect("failed to write bindings");
}
