#![no_std]
#![no_main]

use rp_pico as bsp;

use bsp::entry;
use bsp::hal::pac;

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

pub mod cmsis_dap;

#[entry]
fn main() -> ! {
    println!("Hello, world!");

    let mut pac = pac::Peripherals::take().unwrap();

    cmsis_dap::dap_setup(&pac.SYSCFG.dbgforce);

    loop {}
}
