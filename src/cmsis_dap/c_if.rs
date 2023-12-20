/// Bindings for the CMSIS_5 DAP
use rp_pico as bsp;

use bsp::hal::pac::syscfg::DBGFORCE;

extern "C" {
    pub fn DAP_ProcessCommand(request: *const u8, response: *mut u8) -> u32;
    pub fn DAP_ExecuteCommand(request: *const u8, response: *mut u8) -> u32;
    pub fn DAP_Setup();
}

/// Wraps the DAP_ProcessCommand which executes one command
pub fn dap_process_command(request: &[u8], response: &mut [u8]) -> (u16, u16) {
    let req_resp = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };
    ((req_resp >> 16) as u16, req_resp as u16)
}

/// Wraps the DAP_ExecuteCommand which executes one or more commands
pub fn dap_execute_command(request: &[u8], response: &mut [u8]) -> (u16, u16) {
    let req_resp = unsafe { DAP_ExecuteCommand(request.as_ptr(), response.as_mut_ptr()) };
    ((req_resp >> 16) as u16, req_resp as u16)
}

/// Wraps the DAP_Setup, taking in DBGFORCE to tell that the C code is using that
pub fn dap_setup(_dbgforce: &DBGFORCE) {
    unsafe {
        DAP_Setup();
    }
}
