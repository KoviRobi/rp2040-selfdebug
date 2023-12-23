//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

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

    // - Configuration for the CMSIS DAP library
    // -- Processor Clock of the Cortex-M MCU used in the Debug Unit.
    // -- This value is used to calculate the SWD/JTAG clock speed.
    // -- (Specifies the CPU Clock in Hz.)
    let cpu_clock = match env::var("CARGO_CFG_CMSIS_DAP_CPU_CLOCK") {
        Ok(s) => s
            .parse()
            .expect("Rustc cfg CMSIS_DAP_CPU_CLOCK cannot be parsed as a u32 integer"),
        Err(env::VarError::NotPresent) => 120_000_000u32,
        Err(env::VarError::NotUnicode(os_str)) => {
            panic!("Rustc cfg CMSIS_DAP_CPU_CLOCK not unicode: {:?}", os_str)
        }
    };
    // -- Default communication speed on the Debug Access Port for SWD and JTAG mode.
    // -- Used to initialize the default SWD/JTAG clock frequency.
    // -- The command \ref DAP_SWJ_Clock can be used to overwrite this default setting.
    // -- RP2040 datasheet says max 24MHz (for SYSCFG we can assume max speed)
    // -- (Default SWD/JTAG clock frequency in Hz.)
    let dap_default_swj_clock = match env::var("CARGO_CFG_CMSIS_DAP_DEFAULT_SWJ_CLOCK") {
        Ok(s) => s
            .parse()
            .expect("Rustc cfg CMSIS_DAP_DEFAULT_SWJ_CLOCK cannot be parsed as a u32 integer"),
        Err(env::VarError::NotPresent) => 24_000_000u32,
        Err(env::VarError::NotUnicode(os_str)) => {
            panic!(
                "Rustc cfg CMSIS_DAP_DEFAULT_SWJ_CLOCK not unicode: {:?}",
                os_str
            )
        }
    };
    // -- Maximum Package Buffers for Command and Response data.
    // -- This configuration settings is used to optimize the communication performance with the
    // -- debugger and depends on the USB peripheral. For devices with limited RAM or USB buffer the
    // -- setting can be reduced (valid range is 1 .. 255 inclusive).
    // -- (Specifies number of packets buffered.)
    let dap_packet_count = match env::var("CARGO_CFG_CMSIS_DAP_PACKET_COUNT") {
        Ok(s) => s
            .parse()
            .ok()
            .filter(|n| (1..=255).contains(n))
            .expect("Rustc cfg CMSIS_DAP_PACKET_COUNT cannot be parsed as a u8 integer in the range 1..=255"),
        Err(env::VarError::NotPresent) => 8u8,
        Err(env::VarError::NotUnicode(os_str)) => {
            panic!("Rustc cfg CMSIS_DAP_PACKET_COUNT not unicode: {:?}", os_str)
        }
    };

    // - Change tracking for the CMSIS DAP library
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

    // - Building the CMSIS DAP library
    cc::Build::new()
        .compiler("arm-none-eabi-gcc")
        .file("CMSIS_5/CMSIS/DAP/Firmware/Source/DAP.c")
        .file("CMSIS_5/CMSIS/DAP/Firmware/Source/SW_DP.c")
        .define("CPU_CLOCK", cpu_clock.to_string().as_ref())
        .define(
            "DAP_DEFAULT_SWJ_CLOCK",
            dap_default_swj_clock.to_string().as_ref(),
        )
        .define("DAP_PACKET_COUNT", dap_packet_count.to_string().as_ref())
        .includes(includes)
        .compile("cmsis_dap");

    println!("cargo:rustc-link-lib=cmsis_dap");
}
