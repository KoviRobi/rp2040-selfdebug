#![no_std]
#![no_main]

use rp_pico as bsp;

use bsp::entry;
use bsp::hal::pac::interrupt;
use bsp::hal::{clocks::init_clocks_and_plls, pac, usb::UsbBus, watchdog::Watchdog};

// USB Device support
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::*;

/// The USB Device Driver (shared with the interrupt).
static mut USB_DEVICE: Option<UsbDevice<UsbBus>> = None;

/// The USB Bus Driver (shared with the interrupt).
static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

use panic_probe as _;

use rp_selfdebug::{dap_execute_command, dap_setup, CmsisDap};

/// The USB CMSIS-DAP Device Driver (shared with the interrupt).
static mut USB_DAP: Option<CmsisDap<UsbBus, 64>> = None;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
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

    let usb_dap = CmsisDap::new(bus_ref);
    unsafe {
        USB_DAP = Some(usb_dap);
    }

    let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x04b4, 0xf138))
        .manufacturer("KoviRobi")
        .product("CMSIS-DAP")
        .serial_number("Test")
        .device_class(2)
        .composite_with_iads()
        .max_packet_size_0(64)
        .build();

    unsafe {
        USB_DEVICE = Some(usb_dev);
    };

    dap_setup(&pac.SYSCFG.dbgforce);

    // Enable the USB interrupt
    unsafe {
        pac::NVIC::unmask(bsp::hal::pac::Interrupt::USBCTRL_IRQ);
    };

    loop {
        cortex_m::asm::wfe();
    }
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    let usb_dev = unsafe { USB_DEVICE.as_mut().unwrap() };
    let dap = unsafe { USB_DAP.as_mut().unwrap() };

    if usb_dev.poll(&mut [dap]) {
        let mut buf = [0u8; 64];

        match dap.read(&mut buf) {
            Err(_e) => { /* Do nothing */ }
            Ok(0) => { /* Do nothing */ }
            Ok(_) => {
                let mut out = [0; 64];
                let (_in_size, out_size) = dap_execute_command(&buf, &mut out);
                let _ = dap.write(&out[..out_size as usize]);
            }
        }
    }
}
