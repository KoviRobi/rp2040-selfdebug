#![no_std]
#![no_main]

use embedded_hal::digital::v2::OutputPin;

use rp_pico as bsp;

use bsp::entry;
use bsp::hal::pac::interrupt;
use bsp::hal::prelude::*;
use bsp::hal::{clocks::init_clocks_and_plls, pac, usb::UsbBus, watchdog::Watchdog, Sio};

// USB Device support
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_serial::SerialPort;

/// The USB Device Driver (shared with the interrupt).
static mut USB_DEVICE: Option<UsbDevice<UsbBus>> = None;

/// The USB Bus Driver (shared with the interrupt).
static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

/// The USB Serial Device Driver (shared with the interrupt).
static mut USB_SERIAL: Option<SerialPort<UsbBus>> = None;

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

    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    unsafe {
        USB_BUS = Some(usb_bus);
    }
    let bus_ref = unsafe { USB_BUS.as_ref().unwrap() };

    let usb_serial = SerialPort::new(bus_ref);
    unsafe {
        USB_SERIAL = Some(usb_serial);
    }

    let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x04b4, 0xf138))
        .strings(&[StringDescriptors::new(LangID::EN_US)
            .manufacturer("KoviRobi")
            .product("CMSIS-DAP")
            .serial_number("Test")])
        .unwrap()
        .device_class(2)
        .composite_with_iads()
        .max_packet_size_0(64)
        .unwrap()
        .build();

    unsafe {
        USB_DEVICE = Some(usb_dev);
    };

    cmsis_dap::dap_setup(&pac.SYSCFG.dbgforce);

    // Enable the USB interrupt
    unsafe {
        pac::NVIC::unmask(bsp::hal::pac::Interrupt::USBCTRL_IRQ);
    };
    // No more USB code after this point in main! We can do anything we want in here since USB is
    // handled in the interrupt - let's blink an LED!

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

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    let usb_dev = unsafe { USB_DEVICE.as_mut().unwrap() };
    let serial = unsafe { USB_SERIAL.as_mut().unwrap() };

    if usb_dev.poll(&mut [serial]) {
        let mut buf = [0u8; 64];

        match serial.read(&mut buf) {
            Err(_e) => {
                // Do nothing
            }
            Ok(0) => {
                // Do nothing
            }
            Ok(count) => {
                for b in buf.iter_mut().take(count) {
                    if *b == b'r' {
                        bsp::hal::rom_data::reset_to_usb_boot(1 << 25, 0);
                    }
                }
            }
        }
    }
}
