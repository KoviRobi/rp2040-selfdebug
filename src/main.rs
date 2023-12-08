#![no_std]
#![no_main]

use embedded_hal::digital::v2::OutputPin;

use rp_pico as bsp;

use bsp::entry;
use bsp::hal::prelude::*;
use bsp::hal::{clocks::init_clocks_and_plls, pac, watchdog::Watchdog, Sio};

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

pub mod cmsis_dap;

#[entry]
fn main() -> ! {
    println!("Hello, world!");

    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    cmsis_dap::dap_setup(&pac.SYSCFG.dbgforce);

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let mut led_pin = pins.led.into_push_pull_output();

    loop {
        led_pin.set_high().unwrap();
        delay.delay_ms(500);
        led_pin.set_low().unwrap();
        delay.delay_ms(500);
        defmt::info!("tick");
    }
}
