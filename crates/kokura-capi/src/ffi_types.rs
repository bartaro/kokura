use std::ffi::c_void;

#[repr(C)]
pub struct KokuraCoreHandle {
    _private: [u8; 0],
}

#[repr(C)]
pub struct KokuraDebugSessionHandle {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct KokuraStepResult {
    pub cycles: u32,
    pub frame_completed: bool,
    pub current_rom_bank: u16,
    pub current_ram_bank: u16,
    pub is_cgb_mode: bool,
    pub is_double_speed: bool,
    pub framebuffer_ptr: *const u8,
    pub framebuffer_len: usize,
    pub reserved: *const c_void,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct KokuraDebugRunResult {
    pub frames_requested: u64,
    pub frames_executed: u64,
    pub stopped_by_debugger: bool,
    pub has_stop_reason: bool,
    pub halted_on_unsupported_opcode: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct KokuraCpuSnapshot {
    pub pc: u16,
    pub sp: u16,
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub ime: bool,
    pub halted: bool,
    pub halt_bug: bool,
    pub stopped: bool,
    pub current_rom_bank: u16,
    pub current_ram_bank: u16,
    pub cycles: u64,
    pub frames: u64,
    pub is_cgb_mode: bool,
    pub is_double_speed: bool,
}
