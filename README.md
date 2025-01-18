# rp-selfdebug

This wraps the CMSIS DAP code in a rust wrapper, and then uses that with the Pi
Pico's `dbgforce` ability to debug core0 from core1 or vice versa.

Make sure you have cloned this repo using submodules.

I created this because I wanted to be able to debug my keyboard firmware
running USB code without having a debugger attached. The firmware doesn't need
two cores but it does need to use the USB code, so just using [pico-debug]
wasn't an option.

## Prior art

The most obvious inspiration for this is [pico-debug] which would have been a
good option but the repository was archived and if I was going to use USB, I
thought I might as well use it in Rust since that's what the rest of the
keyboard firmware was written in.

I have also been looking at [picoprobe] which was easier to compile as it just
used CMake and didn't need CrossWorks.

And of course [rust-dap] should also get a mention, which I looked into using,
but I had problems using it, I suspect the reason was due to it not handling
error cases correctly and putting `0xFF` into the USB bulk response packet,
just propagating `Result` out using `?` in the `process` method.

[pico-debug]: https://github.com/majbthrd/pico-debug
[picoprobe]: https://github.com/majbthrd/pico-debug
[rust-dap]: https://github.com/ciniml/rust-dap

## Example

Flash and start OpenOCD

```sh
cargo run --example pico_blinky
openocd -f interface/cmsis-dap.cfg -c 'set USE_CORE 0' -f target/rp2040.cfg
```

Start GDB, optionally adding the boot ROM symbols if you want. The
`interactive-mode off` makes GDB not stop and ask if you want to add the symbol
file. The `tar ext :3333` is shorthand for `target extended-remote
localhost:3333`, to connect to the OpenOCD session. Optionally make sure you
can read all of the memory (it sometimes help, but might make operations take
longer as it requires GDB to try and read inaccessible memory, so YMMV, but the
memory map [see `info mem`] doesn't have all the peripherals). And save
history, optional but useful.

```sh
gdb \
  -q \
  target/thumbv6m-none-eabi/debug/examples/pico_blinky \
  -ex 'tar ext :3333' \
  -ex 'set mem inaccessible-by-default off' \
  -ex 'set interactive-mode off' \
  -ex 'add-symbol-file ~/programming/rp2040/pico-bootrom/build/bootrom.elf' \
  -ex 'set interactive-mode on' \
  -ex 'set history save on'
```

Note, you can also put the `-ex` arguments into a `.gdbinit` file.

Then you can set a breakpoint, for example during the blink task:

```gdb
(gdb) l pico_blinky::core1_task
135             delay.delay_ms(1000);
136             watchdog.feed();
137         }
138     }
139
140     fn core1_task(sys_freq: u32, pins: bsp::hal::gpio::Pins) -> ! {
141         let pac = unsafe { pac::Peripherals::steal() };
142         let core = unsafe { pac::CorePeripherals::steal() };
143
144         let mut sio = Sio::new(pac.SIO);
(gdb)
145         let mut led_pin = pins.gpio25.into_push_pull_output();
146         let mut delay = cortex_m::delay::Delay::new(core.SYST, sys_freq);
147         loop {
148             if sio.fifo.is_write_ready() {
149                 sio.fifo.write(b'a' as u32);
150             }
151             delay.delay_ms(500);
152             led_pin.toggle().unwrap();
153         }
154     }
(gdb) b 152
Breakpoint 1 at 0x10004ace: file examples/pico_blinky.rs, line 152.
Note: automatically using hardware breakpoints for read-only addresses.
(gdb) c
Continuing.

Breakpoint 1, pico_blinky::core1_task (sys_freq=125000000) at examples/pico_blinky.rs:152
152             led_pin.toggle().unwrap();
(gdb)
Continuing.

Breakpoint 1, pico_blinky::core1_task (sys_freq=125000000) at examples/pico_blinky.rs:152
152             led_pin.toggle().unwrap();
```

## Reboot to USB boot

In the blinky example, you can reset to USB flash bootloader, to program, with

```sh
picocom --baud 115200 /dev/serial/by-id/usb-KoviRobi_CMSIS-DAP_Test-if00
```

(or other serial terminal) and then pressing `r`.

In the minimal example, you have to manually do the reboot to USB boot using
GDB:

1. Print well-known addresses in the boot ROM (see the rp2040-datasheet 2.8.3
   Bootrom Contents)

   ```gdb
   (gdb) x/3hx 0x14
   0x14 <_well_known>:     0x007a  0x00c4  0x001d
   ```

2. Find the address of `_reset_to_usb_boot` (see the rp2040-datasheet 2.8.3.1.5
   Miscellaneous Functions)

   ```gdb
   (gdb) find /b 0x007a, 0x00c4, 'U', 'B'
   0x9a <function_table+32>
   1 pattern found.
   ```

3. Print the value of the function table

   ```gdb
   (gdb) x/2hx 0x9a
   0x9a <function_table+32>:       0x4255  0x2591
   ```

4. Go to the `_reset_to_usb_boot` function with `gpio_activity_pin_mask` of pin
   25 (LED on RP2040) and `disable_interface_mask` of zero.

   ```gdb
   (gdb) p $pc = 0x2591
   $3 = ()
   (gdb) p $r0 = 1<<25
   $4 = ()
   (gdb) p $r1 = 0
   $5 = ()
   (gdb) c
   Continuing.

   Program stopped.
   reset_usb_boot (_usb_activity_gpio_pin_mask=33554432,_disable_interface_mask=0) at /home/rmk35/programming/rp2040/pico-bootrom/bootrom/bootrom_main.c:220
   220         watchdog_hw->scratch[0] = _usb_activity_gpio_pin_mask;

   ```

   (Note, no need to hit continue again.)

## Configuring

You can configure the following parameters of CMSIS DAP:

- `CPU_CLK` (default 120000000 [120MHz])
- `DAP_DEFAULT_SWJ_CLOCK` (default 24000000 [24MHz])
- `DAP_PACKET_COUNT` (default 8)
by editing your `.cargo/config.toml`, to include:

```
rustflags = [
  // ...
  "--cfg", "CMSIS_DAP_CPU_CLOCK=\"120000000\"",
  "--cfg", "CMSIS_DAP_DEFAULT_SWJ_CLOCK=\"24000000\"",
  "--cfg", "CMSIS_DAP_PACKET_COUNT=\"8\"",
  // ...
]
```

If you want to configure something else (see the file
[CMSIS_Config/DAP_config.h](CMSIS_Config/DAP_config.h) for more configuration
options), feel free to make a PR, or just use this repository as an example.

## Known problems

### `rust-lld: error: undefined symbol: __gnu_thumb1_case_uhi`

The issue here is that with the `-Os` optimisation GCC is using a "compact
switch" which requires helper functions/tables
(<https://chromium.googlesource.com/chromiumos/platform/ec/+/refs/heads/master/core/cortex-m0/thumb_case.S>),
but `libgcc.a` isn't getting linked. To combat this, the `libgcc.a` directory
is added to the linker search paths (see [build.rs](build.rs)) but you have to
add `-lgcc` to your linker, e.g. put the following into `.cargo/config.toml`

```diff
@@ -1,11 +1,12 @@
 [target.'cfg(all(target_arch = "arm", target_os = "none"))']
 # runner = "elf2uf2-rs -d"
 runner = "probe-rs run --chip RP2040 --probe 2E8A:000C"

 rustflags = [
   "-C", "link-arg=-Tlink.x",
   "-C", "link-arg=-Tdefmt.x",
+  "-C", "link-args=-lgcc"
 ]

 [build]
 target = "thumbv6m-none-eabi"
```

## TODOs

- Test flashing core-1 apps. This might require compiling two separate
  binaries, the CMSIS-DAP one first, and then the core 1 application, linked
  using the `-R`/`--just-symbols` flag pointing to the CMSIS-DAP application's
  ELF.

  Or maybe just relocating the USB DAP to RAM?
