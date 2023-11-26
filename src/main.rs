#![no_std]
#![no_main]

use bsp::entry;
use rp_pico as bsp;

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

#[entry]
fn main() -> ! {
    println!("Hello, world!");

    loop {}
}
