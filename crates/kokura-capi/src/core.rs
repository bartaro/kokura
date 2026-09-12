use std::{
    ffi::{c_char, CStr},
    ptr,
};

use kokura_core::{state::MachineState, Machine};

use crate::{
    ffi_types::{KokuraCoreHandle, KokuraCpuSnapshot, KokuraStepResult},
    strings::into_c_string_ptr,
};

const KOKURA_CAPI_VERSION: &str = "kokura-capi-v1";

fn handle_from_ptr<'a>(handle: *mut KokuraCoreHandle) -> Option<&'a mut Machine> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *(handle as *mut Machine) })
    }
}

fn cpu_snapshot(machine: &Machine) -> KokuraCpuSnapshot {
    KokuraCpuSnapshot {
        pc: machine.cpu.pc,
        sp: machine.cpu.sp,
        af: ((machine.cpu.a as u16) << 8) | machine.cpu.f.0 as u16,
        bc: ((machine.cpu.b as u16) << 8) | machine.cpu.c as u16,
        de: ((machine.cpu.d as u16) << 8) | machine.cpu.e as u16,
        hl: machine.cpu.hl(),
        ime: machine.cpu.ime,
        halted: machine.cpu.halted,
        halt_bug: machine.cpu.halt_bug,
        stopped: machine.cpu.stopped,
        current_rom_bank: machine.current_rom_bank(),
        current_ram_bank: machine.current_ram_bank(),
        cycles: machine.clocks.cycles,
        frames: machine.clocks.frames,
        is_cgb_mode: matches!(machine.mode, kokura_core::types::HardwareMode::Cgb),
        is_double_speed: machine.cgb_double_speed,
    }
}

#[no_mangle]
pub extern "C" fn kokura_core_create() -> *mut KokuraCoreHandle {
    let machine = Box::new(Machine::new());
    Box::into_raw(machine) as *mut KokuraCoreHandle
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_destroy(handle: *mut KokuraCoreHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut Machine);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_load_rom(
    handle: *mut KokuraCoreHandle,
    rom_ptr: *const u8,
    rom_len: usize,
) -> bool {
    let Some(machine) = handle_from_ptr(handle) else {
        return false;
    };
    if rom_ptr.is_null() || rom_len == 0 {
        return false;
    }
    let rom = std::slice::from_raw_parts(rom_ptr, rom_len).to_vec();
    machine.load_rom(rom).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_step(
    handle: *mut KokuraCoreHandle,
    out_result: *mut KokuraStepResult,
) -> bool {
    let Some(machine) = handle_from_ptr(handle) else {
        return false;
    };
    let Ok(step) = machine.step_instruction() else {
        return false;
    };
    if !out_result.is_null() {
        *out_result = KokuraStepResult {
            cycles: step.cycles,
            frame_completed: step.frame_completed,
            current_rom_bank: machine.current_rom_bank(),
            current_ram_bank: machine.current_ram_bank(),
            is_cgb_mode: matches!(machine.mode, kokura_core::types::HardwareMode::Cgb),
            is_double_speed: machine.cgb_double_speed,
            framebuffer_ptr: machine.framebuffer().as_ptr(),
            framebuffer_len: machine.framebuffer().len(),
            reserved: ptr::null(),
        };
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_run_frame(handle: *mut KokuraCoreHandle) -> bool {
    let Some(machine) = handle_from_ptr(handle) else {
        return false;
    };
    machine.run_frame().is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_run_frames(
    handle: *mut KokuraCoreHandle,
    frames: u64,
) -> bool {
    let Some(machine) = handle_from_ptr(handle) else {
        return false;
    };
    for _ in 0..frames {
        if machine.run_frame().is_err() {
            return false;
        }
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_set_joypad_mask(
    handle: *mut KokuraCoreHandle,
    mask: u8,
) -> bool {
    let Some(machine) = handle_from_ptr(handle) else {
        return false;
    };
    machine.set_joypad_mask(mask);
    true
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_framebuffer_ptr(handle: *mut KokuraCoreHandle) -> *const u8 {
    let Some(machine) = handle_from_ptr(handle) else {
        return ptr::null();
    };
    machine.framebuffer().as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_framebuffer_len(handle: *mut KokuraCoreHandle) -> usize {
    handle_from_ptr(handle)
        .map(|machine| machine.framebuffer().len())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_framebuffer_rgb555_ptr(
    handle: *mut KokuraCoreHandle,
) -> *const u16 {
    let Some(machine) = handle_from_ptr(handle) else {
        return ptr::null();
    };
    machine.framebuffer_rgb555().as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_framebuffer_rgb555_len(
    handle: *mut KokuraCoreHandle,
) -> usize {
    handle_from_ptr(handle)
        .map(|machine| machine.framebuffer_rgb555().len())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_is_cgb_compat_mode(handle: *mut KokuraCoreHandle) -> bool {
    handle_from_ptr(handle)
        .map(|machine| machine.is_cgb_compat_mode())
        .unwrap_or(false)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_current_rom_bank(handle: *mut KokuraCoreHandle) -> u16 {
    handle_from_ptr(handle)
        .map(|m| m.current_rom_bank())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_current_ram_bank(handle: *mut KokuraCoreHandle) -> u16 {
    handle_from_ptr(handle)
        .map(|m| m.current_ram_bank())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_cpu_snapshot(
    handle: *mut KokuraCoreHandle,
    out_snapshot: *mut KokuraCpuSnapshot,
) -> bool {
    let Some(machine) = handle_from_ptr(handle) else {
        return false;
    };
    if out_snapshot.is_null() {
        return false;
    }
    *out_snapshot = cpu_snapshot(machine);
    true
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_peek8(handle: *mut KokuraCoreHandle, addr: u16) -> u8 {
    handle_from_ptr(handle)
        .map(|m| m.peek8(addr))
        .unwrap_or(0xFF)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_read8(handle: *mut KokuraCoreHandle, addr: u16) -> u8 {
    handle_from_ptr(handle)
        .map(|m| m.read8(addr))
        .unwrap_or(0xFF)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_write8(
    handle: *mut KokuraCoreHandle,
    addr: u16,
    value: u8,
) -> bool {
    let Some(machine) = handle_from_ptr(handle) else {
        return false;
    };
    machine.write8(addr, value);
    true
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_save_state_json(handle: *mut KokuraCoreHandle) -> *mut c_char {
    let Some(machine) = handle_from_ptr(handle) else {
        return ptr::null_mut();
    };
    match serde_json::to_string(&machine.save_state()) {
        Ok(json) => into_c_string_ptr(json),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_load_state_json(
    handle: *mut KokuraCoreHandle,
    state_json: *const c_char,
) -> bool {
    let Some(machine) = handle_from_ptr(handle) else {
        return false;
    };
    if state_json.is_null() {
        return false;
    }
    let Ok(text) = CStr::from_ptr(state_json).to_str() else {
        return false;
    };
    let Ok(state) = serde_json::from_str::<MachineState>(text) else {
        return false;
    };
    machine.load_state(&state);
    true
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_audio_sample_rate(handle: *mut KokuraCoreHandle) -> u32 {
    handle_from_ptr(handle)
        .map(|m| m.audio_sample_rate())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_audio_frames_available(
    handle: *mut KokuraCoreHandle,
) -> usize {
    handle_from_ptr(handle)
        .map(|m| m.audio_frames_available())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_audio_frames_dropped(handle: *mut KokuraCoreHandle) -> u64 {
    handle_from_ptr(handle)
        .map(|m| m.audio_frames_dropped())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_audio_buffer_capacity_frames(
    handle: *mut KokuraCoreHandle,
) -> usize {
    handle_from_ptr(handle)
        .map(|m| m.audio_buffer_capacity_frames())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_set_audio_buffer_capacity_frames(
    handle: *mut KokuraCoreHandle,
    capacity_frames: usize,
) -> bool {
    let Some(machine) = handle_from_ptr(handle) else {
        return false;
    };
    machine
        .set_audio_buffer_capacity_frames(capacity_frames)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn kokura_core_audio_copy_interleaved_i16(
    handle: *mut KokuraCoreHandle,
    dst: *mut i16,
    max_frames: usize,
) -> usize {
    let Some(machine) = handle_from_ptr(handle) else {
        return 0;
    };
    if dst.is_null() || max_frames == 0 {
        return 0;
    }
    let samples = machine.drain_audio_frames_interleaved_i16(max_frames);
    if samples.is_empty() {
        return 0;
    }
    ptr::copy_nonoverlapping(samples.as_ptr(), dst, samples.len());
    samples.len() / 2
}

#[no_mangle]
pub extern "C" fn kokura_version_string() -> *mut c_char {
    into_c_string_ptr(KOKURA_CAPI_VERSION)
}
