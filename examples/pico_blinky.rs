#![no_std]
#![no_main]

use embedded_hal::digital::v2::ToggleableOutputPin;

use rp_pico as bsp;

use bsp::entry;
use bsp::hal::pac::interrupt;
use bsp::hal::prelude::*;
use bsp::hal::{
    clocks::init_clocks_and_plls,
    multicore::{Multicore, Stack},
    pac,
    usb::UsbBus,
    watchdog::Watchdog,
    Sio,
};

use bsp::hal::fugit::ExtU32;
use embedded_hal::watchdog::{Watchdog as _, WatchdogEnable as _};

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

use panic_probe as _;

use rp_selfdebug::{dap_execute_command, dap_setup, CmsisDap};

/// The USB CMSIS-DAP Device Driver (shared with the interrupt).
static mut USB_DAP: Option<CmsisDap<UsbBus, 64>> = None;

/// The core 0 multi-core FIFO/mailbox (shared with the USB interrupt)
static mut CORE0_FIFO: Option<bsp::hal::sio::SioFifo> = None;

static mut CORE1_STACK: Stack<4096> = Stack::new();

#[entry]
fn main() -> ! {
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

    let sys_freq = clocks.system_clock.freq().to_Hz();

    watchdog.start(5.secs());

    let mut sio = Sio::new(pac.SIO);

    let pins = bsp::hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    let _task = core1.spawn(unsafe { &mut CORE1_STACK.mem }, move || {
        core1_task(sys_freq, pins);
    });

    unsafe {
        CORE0_FIFO = Some(sio.fifo);
    }

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

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    loop {
        delay.delay_ms(1000);
        watchdog.feed();
    }
}

fn core1_task(sys_freq: u32, pins: bsp::hal::gpio::Pins) -> ! {
    let pac = unsafe { pac::Peripherals::steal() };
    let core = unsafe { pac::CorePeripherals::steal() };

    let mut sio = Sio::new(pac.SIO);
    let mut led_pin = pins.gpio25.into_push_pull_output();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, sys_freq);
    loop {
        if sio.fifo.is_write_ready() {
            sio.fifo.write(b'a' as u32);
        }
        delay.delay_ms(500);
        led_pin.toggle().unwrap();
    }
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    let usb_dev = unsafe { USB_DEVICE.as_mut().unwrap() };
    let serial = unsafe { USB_SERIAL.as_mut().unwrap() };
    let dap = unsafe { USB_DAP.as_mut().unwrap() };
    let fifo = unsafe { CORE0_FIFO.as_mut().unwrap() };

    if fifo.is_read_ready() {
        let _ = serial.write(b"Fifo: ");
        while let Some(data) = fifo.read() {
            let b = [data as u8];
            let _ = serial.write(&b);
        }
        let _ = serial.write(b"\r\n");
    }
    let _ = serial.flush();

    if usb_dev.poll(&mut [serial, dap]) {
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

        match dap.read(&mut buf) {
            Err(_e) => {
                // Do nothing
            }
            Ok(0) => {
                // Do nothing
            }
            Ok(_) => {
                let mut out = [0; 64];
                let (_in_size, out_size) = dap_execute_command(&buf, &mut out);
                let _ = dap.write(&out[..out_size as usize]);
            }
        }
    }
}
