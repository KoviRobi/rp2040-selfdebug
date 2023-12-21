# rp-selfdebug

This wraps the CMSIS DAP code in a rust wrapper, and then uses that with the pi
pico's dbgforce ability to debug core0 from core1 or vice versa.

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
used cmake and didn't need CrossWorks.

And of course [rust-dap] should also get a mention, which I looked into using,
but I had problems using it, I suspect the reason was due to it not handling
error cases correctly and putting `0xFF` into the USB bulk response packet,
just propagating `Result` out using `?` in the `process` method.

[pico-debug]: https://github.com/majbthrd/pico-debug
[picoprobe]: https://github.com/majbthrd/pico-debug
[rust-dap]: https://github.com/ciniml/rust-dap

## Configuring
I couldn't figure out a good way to make the configuration in
[CMSIS_Config/DAP_config.h](CMSIS_Config/DAP_config.h) parameterized over
integers (rather than just a couple of feature flags), so feel free to just
use this library as a reference/inspiration to copy from rather than using it
as a library via cargo, if you need to change frequencies etc. Maybe link back to
here so it's easier for others to figure out how to do the same for their code
base, but I don't mind and am definitely not the first to do this anyway.
