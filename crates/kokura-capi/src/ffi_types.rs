use std::ffi::c_void;

#[repr(C)]
// Opaque C handle marker. Actual allocations contain Machine and must
// be released through the matching core API, not by allocating this type.
pub struct KokuraCoreHandle {
    _private: [u8; 0],
}

#[repr(C)]
// Opaque debug-session handle marker; never substitute a core handle.
pub struct KokuraDebugSessionHandle {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
// C-layout step result with a borrowed byte-framebuffer pointer.
// framebuffer_len counts bytes; reserved is currently null.
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
// Describe the requested run and its net frame-count increase. Debugger
// stops and unsupported-opcode termination are reported separately from
// the function return value; rewind can reduce the net frame count.
pub struct KokuraDebugRunResult {
    pub frames_requested: u64,
    pub frames_executed: u64,
    pub stopped_by_debugger: bool,
    pub has_stop_reason: bool,
    pub halted_on_unsupported_opcode: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
// C-layout value snapshot of CPU/register pairs, execution flags, banks
// and clocks. Copying it does not retain a borrow of machine storage.
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
