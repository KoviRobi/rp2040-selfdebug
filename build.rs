//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use cc;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    // Compile the CMSIS DAP code
    let includes = [
        "CMSIS_5/CMSIS/Core/Include/",
        "CMSIS_5/CMSIS/DAP/Firmware/Include/",
        "CMSIS_Config/",
        "pico-sdk/src/common/pico_base/include/",
        "pico-sdk/src/rp2_common/pico_platform/include/",
        "pico-sdk/src/rp2_common/hardware_base/include/",
        "pico-sdk/src/rp2040/hardware_regs/include/",
        "pico-sdk/src/rp2040/hardware_structs/include/",
    ];
    println!("cargo:rerun-if-changed=CMSIS_5/CMSIS/DAP/Firmware/Source/DAP.c");
    for dir in includes {
        println!("cargo:rerun-if-changed={dir}");
    }
    cc::Build::new()
        .compiler("arm-none-eabi-gcc")
        .file("CMSIS_5/CMSIS/DAP/Firmware/Source/DAP.c")
        .file("CMSIS_5/CMSIS/DAP/Firmware/Source/SW_DP.c")
        .includes(includes)
        .compile("cmsis_dap");

    println!("cargo:rustc-link-lib=cmsis_dap");
}
