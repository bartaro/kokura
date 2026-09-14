use std::{
    ffi::{c_char, CStr},
    ptr,
};

use kokura_core::{state::MachineState, Machine};

use crate::{
    ffi_types::{KokuraCoreHandle, KokuraCpuSnapshot, KokuraStepResult},
    strings::into_c_string_ptr,
};

// All non-null handles must be live allocations from this library. Calls
// borrow the machine exclusively, even for getters: serialize access and do
// not use a handle after destruction. Raw pointer validity is caller-owned.
const KOKURA_CAPI_VERSION: &str = "kokura-capi-v1";

// Treat null as absent; otherwise borrow the Machine behind the opaque
// handle. The caller must guarantee provenance, liveness and exclusive access
// for the entire borrow. Non-null pointers are not validated by this cast.
fn handle_from_ptr<'a>(handle: *mut KokuraCoreHandle) -> Option<&'a mut Machine> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *(handle as *mut Machine) })
    }
}

// Copy registers, paired values, banks, clocks and execution/mode flags
// into the C-layout snapshot without advancing emulation.
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
// Allocate a new machine and transfer the opaque handle to the caller;
// release it once with kokura_core_destroy from the same library.
pub extern "C" fn kokura_core_create() -> *mut KokuraCoreHandle {
    let machine = Box::new(Machine::new());
    Box::into_raw(machine) as *mut KokuraCoreHandle
}

#[no_mangle]
// Drop the live machine allocation, accepting null. A non-null handle
// must originate from this library and must not be borrowed or destroyed twice.
pub unsafe extern "C" fn kokura_core_destroy(handle: *mut KokuraCoreHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut Machine);
    }
}

#[no_mangle]
// Copy rom_len readable bytes into owned storage and load the cartridge.
// Reject null/empty input or load errors with false; detailed errors are not
// returned by this core-only API.
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
// Advance one machine step and optionally fill a writable result object.
// The returned framebuffer is borrowed; inspect/copy it before mutating or
// destroying the machine. Failure does not promise an unchanged machine.
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
// Run one frame and return whether execution completed without an error.
pub unsafe extern "C" fn kokura_core_run_frame(handle: *mut KokuraCoreHandle) -> bool {
    let Some(machine) = handle_from_ptr(handle) else {
        return false;
    };
    machine.run_frame().is_ok()
}

#[no_mangle]
// Run up to the requested frames, returning false at the first error.
// Earlier progress remains applied; a valid handle with zero frames succeeds.
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
// Apply the host input mask through the machine-level input path,
// including its modeled edge handling. Null handles return false.
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
// Borrow the byte framebuffer, or return null for a null handle.
// Do not free it; copy/read it before any machine mutation or destruction.
pub unsafe extern "C" fn kokura_core_framebuffer_ptr(handle: *mut KokuraCoreHandle) -> *const u8 {
    let Some(machine) = handle_from_ptr(handle) else {
        return ptr::null();
    };
    machine.framebuffer().as_ptr()
}

#[no_mangle]
// Return the byte framebuffer element count, or zero for a null handle.
pub unsafe extern "C" fn kokura_core_framebuffer_len(handle: *mut KokuraCoreHandle) -> usize {
    handle_from_ptr(handle)
        .map(|machine| machine.framebuffer().len())
        .unwrap_or(0)
}

#[no_mangle]
// Borrow the RGB555 word framebuffer, or return null for a null handle.
// The machine retains ownership; consume it before mutation or destruction.
pub unsafe extern "C" fn kokura_core_framebuffer_rgb555_ptr(
    handle: *mut KokuraCoreHandle,
) -> *const u16 {
    let Some(machine) = handle_from_ptr(handle) else {
        return ptr::null();
    };
    machine.framebuffer_rgb555().as_ptr()
}

#[no_mangle]
// Return the number of u16 RGB555 elements, not the byte size; null is zero.
pub unsafe extern "C" fn kokura_core_framebuffer_rgb555_len(
    handle: *mut KokuraCoreHandle,
) -> usize {
    handle_from_ptr(handle)
        .map(|machine| machine.framebuffer_rgb555().len())
        .unwrap_or(0)
}

#[no_mangle]
// Report the machine compatibility-mode flag, using false for null.
pub unsafe extern "C" fn kokura_core_is_cgb_compat_mode(handle: *mut KokuraCoreHandle) -> bool {
    handle_from_ptr(handle)
        .map(|machine| machine.is_cgb_compat_mode())
        .unwrap_or(false)
}

#[no_mangle]
// Return the controller ROM-bank label, using zero for null. A zero
// result alone cannot distinguish a valid bank-zero selection from null.
pub unsafe extern "C" fn kokura_core_current_rom_bank(handle: *mut KokuraCoreHandle) -> u16 {
    handle_from_ptr(handle)
        .map(|m| m.current_rom_bank())
        .unwrap_or(0)
}

#[no_mangle]
// Return the controller RAM-bank label, using zero for null.
pub unsafe extern "C" fn kokura_core_current_ram_bank(handle: *mut KokuraCoreHandle) -> u16 {
    handle_from_ptr(handle)
        .map(|m| m.current_ram_bank())
        .unwrap_or(0)
}

#[no_mangle]
// Fill one valid writable snapshot object; reject null handle/output.
// The copied snapshot contains no borrowed framebuffer pointer.
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
// Inspect a mapped byte without ordinary device-read side effects.
// A null handle returns FF, which is also a possible memory value.
pub unsafe extern "C" fn kokura_core_peek8(handle: *mut KokuraCoreHandle, addr: u16) -> u8 {
    handle_from_ptr(handle)
        .map(|m| m.peek8(addr))
        .unwrap_or(0xFF)
}

#[no_mangle]
// Perform an ordinary machine bus read, including modeled side effects;
// null returns FF. Use peek8 for observational inspection.
pub unsafe extern "C" fn kokura_core_read8(handle: *mut KokuraCoreHandle, addr: u16) -> u8 {
    handle_from_ptr(handle)
        .map(|m| m.read8(addr))
        .unwrap_or(0xFF)
}

#[no_mangle]
// Perform an ordinary machine bus write; true means the call was made,
// not that hardware gates necessarily accepted or retained the byte.
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
// Serialize the MachineState payload, including cartridge storage,
// into an owned UTF-8 C string. Free it with kokura_string_free; null reports
// failure. This JSON is not the binary KQS file envelope.
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
// Decode a readable NUL-terminated UTF-8 MachineState JSON string and
// apply it. This wrapper performs no payload validation or live-ROM identity
// check after decoding; false reports null, UTF-8 or JSON decoding failure.
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
// Return output samples per second, or zero for a null handle.
pub unsafe extern "C" fn kokura_core_audio_sample_rate(handle: *mut KokuraCoreHandle) -> u32 {
    handle_from_ptr(handle)
        .map(|m| m.audio_sample_rate())
        .unwrap_or(0)
}

#[no_mangle]
// Return queued stereo frames, not interleaved sample elements; null is zero.
pub unsafe extern "C" fn kokura_core_audio_frames_available(
    handle: *mut KokuraCoreHandle,
) -> usize {
    handle_from_ptr(handle)
        .map(|m| m.audio_frames_available())
        .unwrap_or(0)
}

#[no_mangle]
// Return the accumulated dropped-audio-frame count, or zero for null.
pub unsafe extern "C" fn kokura_core_audio_frames_dropped(handle: *mut KokuraCoreHandle) -> u64 {
    handle_from_ptr(handle)
        .map(|m| m.audio_frames_dropped())
        .unwrap_or(0)
}

#[no_mangle]
// Return audio queue capacity in stereo frames, or zero for null.
pub unsafe extern "C" fn kokura_core_audio_buffer_capacity_frames(
    handle: *mut KokuraCoreHandle,
) -> usize {
    handle_from_ptr(handle)
        .map(|m| m.audio_buffer_capacity_frames())
        .unwrap_or(0)
}

#[no_mangle]
// Request an audio queue capacity and return whether the machine accepts
// it; null and invalid-capacity errors return false.
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
// Drain up to max_frames stereo frames into caller-owned storage and
// return the number drained. dst must hold 2 * max_frames i16 elements
// and must not overlap source storage; null/zero requests drain nothing.
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
// Allocate an owned C ABI version string; release it with kokura_string_free.
pub extern "C" fn kokura_version_string() -> *mut c_char {
    into_c_string_ptr(KOKURA_CAPI_VERSION)
}
