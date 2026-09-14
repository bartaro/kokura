use std::{
    ffi::{c_char, c_void, CStr, CString},
    ptr,
};

use kokura_bridge::symbols::SymbolTable;
use kokura_core::{state::MachineState, Machine};
use kokura_debug::{DebugEvent, DebugSession, ReplayControlSet, StopConditionSet};
use serde::{Deserialize, Serialize};

use crate::{
    bridge::{apply_source_map_text, apply_symbol_map_text, parse_symbol_table_json},
    ffi_types::{KokuraCpuSnapshot, KokuraDebugRunResult, KokuraDebugSessionHandle},
    strings::into_c_string_ptr,
};

// All C entry points require a live matching handle and serialized
// access, including getters. Callback invocation retains the exclusive
// session borrow: calling back into this handle would violate that contract.
struct DebugSessionHandleImpl {
    session: DebugSession,
    symbol_table: SymbolTable,
    callback_config: DebugCallbackConfig,
    callback: Option<DebugCallbackRegistration>,
    callback_event_cursor: usize,
    last_error: Option<String>,
}

// Borrowed callback arguments are valid only until the callback returns;
// user_data belongs to the caller and is never released by this library.
type KokuraDebugCallbackFn =
    unsafe extern "C" fn(kind: *const c_char, payload_json: *const c_char, user_data: *mut c_void);

#[derive(Debug, Clone)]
struct DebugCallbackRegistration {
    callback: KokuraDebugCallbackFn,
    user_data: *mut c_void,
    config: DebugCallbackConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DebugCallbackConfig {
    #[serde(default)]
    frame_interval: Option<u64>,
    #[serde(default)]
    event_types: Vec<String>,
    #[serde(default = "default_emit_stop_callback")]
    emit_stop: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DebugCallbackFramePayload {
    frame: u64,
    local_frame: u64,
    cycle: u64,
    pc: u16,
    rom_bank: u16,
    frame_hash: u32,
}

#[derive(Debug, Clone, Serialize)]
struct DebugCallbackEventPayload {
    event_type: String,
    event: DebugEvent,
}

#[derive(Debug, Clone, Serialize)]
struct DebugCallbackStopPayload<'a> {
    frames_requested: u64,
    frames_executed: u64,
    halted_on_unsupported_opcode: bool,
    stop_reason: &'a kokura_debug::StopReason,
}

// Enable stop notifications when serde omits that configuration field.
fn default_emit_stop_callback() -> bool {
    true
}

impl Default for DebugCallbackConfig {
    // Disable periodic frame and selected-event callbacks while enabling
    // stop callbacks; an actual callback function must still be registered.
    fn default() -> Self {
        Self {
            frame_interval: None,
            event_types: Vec::new(),
            emit_stop: true,
        }
    }
}

impl DebugCallbackConfig {
    // Reject a zero frame interval, normalize event aliases and sort/dedup
    // the resulting labels. Unknown labels are retained rather than rejected.
    fn validate(mut self) -> Result<Self, String> {
        if self.frame_interval == Some(0) {
            return Err("callback frame_interval must be >= 1 when provided".to_string());
        }
        self.event_types = self
            .event_types
            .into_iter()
            .map(|value| normalize_event_type_label(&value))
            .collect();
        self.event_types.sort();
        self.event_types.dedup();
        Ok(self)
    }

    // Match an already normalized label exactly against the configured list;
    // an empty list selects no events and is not a wildcard.
    fn wants_event_type(&self, event_type: &str) -> bool {
        self.event_types.iter().any(|value| value == event_type)
    }
}

// Borrow the opaque live debug handle exclusively, treating null as absent.
// Caller-provided provenance, lifetime and synchronization make this cast valid.
fn handle_from_ptr<'a>(
    handle: *mut KokuraDebugSessionHandle,
) -> Option<&'a mut DebugSessionHandleImpl> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *(handle as *mut DebugSessionHandleImpl) })
    }
}

// Store an owned error message and return the caller-selected failure value.
fn set_last_error<T>(
    handle: &mut DebugSessionHandleImpl,
    message: impl Into<String>,
    fallback: T,
) -> T {
    handle.last_error = Some(message.into());
    fallback
}

// Discard the previous diagnostic message before a new fallible operation.
fn clear_last_error(handle: &mut DebugSessionHandleImpl) {
    handle.last_error = None;
}

// Reject null or invalid UTF-8 and borrow the C string contents. The
// caller must supply readable NUL-terminated storage valid for the whole borrow.
fn cstr_to_str<'a>(ptr: *const c_char) -> Result<&'a str, String> {
    if ptr.is_null() {
        return Err("received null C string pointer".to_string());
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|err| format!("received invalid UTF-8 string: {err}"))
}

// Install a cloned combined table into the session or clear session
// metadata when every local table collection is empty.
fn rebuild_symbol_table(handle: &mut DebugSessionHandleImpl) {
    if handle.symbol_table.is_empty() {
        handle.session.clear_symbol_table();
    } else {
        handle.session.set_symbol_table(handle.symbol_table.clone());
    }
}

// Copy CPU/register pairs, clocks, bank labels and mode flags into
// the shared C-layout value structure.
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
// Allocate a new machine/debug session with empty metadata, no callback
// and no stored error; release it once with the matching session destructor.
pub extern "C" fn kokura_debug_session_create() -> *mut KokuraDebugSessionHandle {
    let handle = Box::new(DebugSessionHandleImpl {
        session: DebugSession::new(Machine::new()),
        symbol_table: SymbolTable::default(),
        callback_config: DebugCallbackConfig::default(),
        callback: None,
        callback_event_cursor: 0,
        last_error: None,
    });
    Box::into_raw(handle) as *mut KokuraDebugSessionHandle
}

#[no_mangle]
// Destroy a live session allocation, accepting null. The caller must
// ensure it is not in use, including by a callback, and destroy it only once.
pub unsafe extern "C" fn kokura_debug_session_destroy(handle: *mut KokuraDebugSessionHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut DebugSessionHandleImpl);
    }
}

#[no_mangle]
// Copy readable ROM bytes into a new machine/session. Preserve local
// symbol metadata and callback registration/configuration, but replace prior
// session execution/replay/stop state only after ROM loading succeeds.
pub unsafe extern "C" fn kokura_debug_session_load_rom(
    handle: *mut KokuraDebugSessionHandle,
    rom_ptr: *const u8,
    rom_len: usize,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    if rom_ptr.is_null() || rom_len == 0 {
        return set_last_error(
            handle,
            "ROM pointer must be non-null and ROM length must be non-zero",
            false,
        );
    }
    let rom = std::slice::from_raw_parts(rom_ptr, rom_len).to_vec();
    let mut machine = Machine::new();
    if let Err(err) = machine.load_rom(rom) {
        return set_last_error(
            handle,
            format!("failed to load ROM into debug session: {err}"),
            false,
        );
    }
    let mut session = DebugSession::new(machine);
    if !handle.symbol_table.is_empty() {
        session.set_symbol_table(handle.symbol_table.clone());
    }
    handle.session = session;
    handle.callback_event_cursor = 0;
    true
}

#[no_mangle]
// Read a NUL-terminated UTF-8 map string, replace symbol spans and
// reinstall combined metadata. Invalid text encoding sets last_error;
// malformed individual map rows follow the permissive parser behavior.
pub unsafe extern "C" fn kokura_debug_session_load_symbol_map_text(
    handle: *mut KokuraDebugSessionHandle,
    map_text: *const c_char,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    let text = match cstr_to_str(map_text) {
        Ok(text) => text,
        Err(err) => return set_last_error(handle, err, false),
    };
    apply_symbol_map_text(&mut handle.symbol_table, text);
    rebuild_symbol_table(handle);
    true
}

#[no_mangle]
// Replace source positions from a readable UTF-8 C string and rebuild
// session metadata, retaining the other local table collections.
pub unsafe extern "C" fn kokura_debug_session_load_source_map_text(
    handle: *mut KokuraDebugSessionHandle,
    source_map_text: *const c_char,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    let text = match cstr_to_str(source_map_text) {
        Ok(text) => text,
        Err(err) => return set_last_error(handle, err, false),
    };
    apply_source_map_text(&mut handle.symbol_table, text);
    rebuild_symbol_table(handle);
    true
}

#[no_mangle]
// Decode and replace all local symbol metadata, then install it into
// the session; decoding errors leave the old table and set last_error.
pub unsafe extern "C" fn kokura_debug_session_load_symbol_table_json(
    handle: *mut KokuraDebugSessionHandle,
    symbol_table_json: *const c_char,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    let text = match cstr_to_str(symbol_table_json) {
        Ok(text) => text,
        Err(err) => return set_last_error(handle, err, false),
    };
    match parse_symbol_table_json(text) {
        Ok(symbol_table) => {
            handle.symbol_table = symbol_table;
            rebuild_symbol_table(handle);
            true
        }
        Err(err) => set_last_error(handle, err, false),
    }
}

#[no_mangle]
// Clear both the retained table and session symbol metadata.
pub unsafe extern "C" fn kokura_debug_session_clear_symbol_table(
    handle: *mut KokuraDebugSessionHandle,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    handle.symbol_table = SymbolTable::default();
    handle.session.clear_symbol_table();
    true
}

#[no_mangle]
// Decode UTF-8 stop-condition JSON and pass it to session validation.
// Store parse/validation failures in last_error and return false.
pub unsafe extern "C" fn kokura_debug_session_set_stop_conditions_json(
    handle: *mut KokuraDebugSessionHandle,
    stop_conditions_json: *const c_char,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    let text = match cstr_to_str(stop_conditions_json) {
        Ok(text) => text,
        Err(err) => return set_last_error(handle, err, false),
    };
    let parsed = match serde_json::from_str::<StopConditionSet>(text) {
        Ok(value) => value,
        Err(err) => {
            return set_last_error(
                handle,
                format!("failed to parse StopConditionSet JSON: {err}"),
                false,
            )
        }
    };
    match handle.session.set_stop_conditions(parsed) {
        Ok(()) => true,
        Err(err) => set_last_error(
            handle,
            format!("failed to apply stop conditions: {err}"),
            false,
        ),
    }
}

#[no_mangle]
// Clear the session stop-condition set through its dedicated method.
pub unsafe extern "C" fn kokura_debug_session_clear_stop_conditions(
    handle: *mut KokuraDebugSessionHandle,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    handle.session.clear_stop_conditions();
    true
}

#[no_mangle]
// Decode replay configuration and delegate validation/application to
// the session, returning false with last_error on failure.
pub unsafe extern "C" fn kokura_debug_session_set_replay_control_json(
    handle: *mut KokuraDebugSessionHandle,
    replay_control_json: *const c_char,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    let text = match cstr_to_str(replay_control_json) {
        Ok(text) => text,
        Err(err) => return set_last_error(handle, err, false),
    };
    let parsed = match serde_json::from_str::<ReplayControlSet>(text) {
        Ok(value) => value,
        Err(err) => {
            return set_last_error(
                handle,
                format!("failed to parse ReplayControlSet JSON: {err}"),
                false,
            )
        }
    };
    match handle.session.set_replay_control(parsed) {
        Ok(()) => true,
        Err(err) => set_last_error(
            handle,
            format!("failed to apply replay control: {err}"),
            false,
        ),
    }
}

#[no_mangle]
// Apply default disabled replay settings through normal session
// configuration handling, reporting any failure through last_error.
pub unsafe extern "C" fn kokura_debug_session_clear_replay_control(
    handle: *mut KokuraDebugSessionHandle,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    match handle
        .session
        .set_replay_control(ReplayControlSet::default())
    {
        Ok(()) => true,
        Err(err) => set_last_error(
            handle,
            format!("failed to clear replay control: {err}"),
            false,
        ),
    }
}

#[no_mangle]
// Clear the previous error and apply input through the machine-level
// path rather than writing the Joypad fields directly.
pub unsafe extern "C" fn kokura_debug_session_set_joypad_mask(
    handle: *mut KokuraDebugSessionHandle,
    mask: u8,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    handle.session.machine.set_joypad_mask(mask);
    true
}

#[no_mangle]
// Run with optional synchronous event/frame/stop callbacks and fill
// an optional writable result. Execution success can coexist with a callback
// serialization error in last_error. Frame counts are net, saturating deltas
// and may be reduced by rewind; a debugger stop need not return false.
pub unsafe extern "C" fn kokura_debug_session_run_frames(
    handle: *mut KokuraDebugSessionHandle,
    frames: u64,
    out_result: *mut KokuraDebugRunResult,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    // Use the current frame count as the baseline for the net run result;
    // callbacks may report intermediate progress before a later rewind.
    let start_frames = handle.session.machine.clocks.frames;
    let start_event_cursor = handle
        .callback_event_cursor
        .min(handle.session.event_log.len());
    let mut callback_event_cursor = start_event_cursor;
    let mut callback_error: Option<String> = None;
    let mut last_emitted_frame = start_frames;
    let ok = if let Some(callback) = handle.callback.clone() {
        let callback_config = callback.config.clone();
        handle
            .session
            .run_frames_with_callback(frames, |session, local_frame| {
                if let Err(err) = emit_new_event_callbacks(
                    &callback,
                    &callback_config,
                    &session.event_log,
                    &mut callback_event_cursor,
                ) {
                    callback_error = Some(err);
                }
                if let Some(interval) = callback_config.frame_interval {
                    let current_frame = session.machine.clocks.frames;
                    if current_frame > last_emitted_frame && local_frame % interval == 0 {
                        if let Some(frame_hash) = session.last_frame_hash {
                            let payload = DebugCallbackFramePayload {
                                frame: current_frame,
                                local_frame,
                                cycle: session.machine.clocks.cycles,
                                pc: session.machine.cpu.pc,
                                rom_bank: session.machine.current_rom_bank(),
                                frame_hash,
                            };
                            if let Err(err) = invoke_debug_callback(&callback, "frame", &payload) {
                                callback_error = Some(err);
                            }
                        }
                        last_emitted_frame = current_frame;
                    }
                }
            })
    } else {
        handle.session.run_frames(frames)
    };
    handle.callback_event_cursor = callback_event_cursor;
    // Retain callback serialization errors without turning a successful
    // session run into a false return; execution errors can replace this message.
    if let Some(err) = callback_error.take() {
        handle.last_error = Some(err);
    }
    if let Some(callback) = handle.callback.clone() {
        if let Err(err) = emit_new_event_callbacks(
            &callback,
            &callback.config,
            &handle.session.event_log,
            &mut handle.callback_event_cursor,
        ) {
            handle.last_error = Some(err);
        }
        if callback.config.emit_stop {
            if let Some(stop_reason) = handle.session.stop_reason() {
                let payload = DebugCallbackStopPayload {
                    frames_requested: frames,
                    frames_executed: handle
                        .session
                        .machine
                        .clocks
                        .frames
                        .saturating_sub(start_frames),
                    halted_on_unsupported_opcode: handle.session.halted_on_unsupported_opcode(),
                    stop_reason,
                };
                if let Err(err) = invoke_debug_callback(&callback, "stop", &payload) {
                    handle.last_error = Some(err);
                }
            }
        }
    }
    let frames_executed = handle
        .session
        .machine
        .clocks
        .frames
        .saturating_sub(start_frames);
    if !out_result.is_null() {
        *out_result = KokuraDebugRunResult {
            frames_requested: frames,
            frames_executed,
            stopped_by_debugger: handle.session.stop_reason().is_some(),
            has_stop_reason: handle.session.stop_reason().is_some(),
            halted_on_unsupported_opcode: handle.session.halted_on_unsupported_opcode(),
        };
    }
    match ok {
        Ok(()) => true,
        Err(err) => set_last_error(handle, format!("debug session run failed: {err}"), false),
    }
}

#[no_mangle]
// Request rewind through stored replay checkpoints and report a missing
// matching checkpoint via false and last_error.
pub unsafe extern "C" fn kokura_debug_session_rewind_frames(
    handle: *mut KokuraDebugSessionHandle,
    frames_back: u64,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    if handle.session.rewind_frames(frames_back) {
        true
    } else {
        set_last_error(
            handle,
            format!("failed to rewind {frames_back} frame(s): no matching replay checkpoint"),
            false,
        )
    }
}

#[no_mangle]
// Serialize the current report into an owned UTF-8 C string; null
// reports failure. Release a non-null result with kokura_string_free.
pub unsafe extern "C" fn kokura_debug_session_report_json(
    handle: *mut KokuraDebugSessionHandle,
) -> *mut c_char {
    let Some(handle) = handle_from_ptr(handle) else {
        return ptr::null_mut();
    };
    clear_last_error(handle);
    match serde_json::to_string(&handle.session.report()) {
        Ok(json) => into_c_string_ptr(json),
        Err(err) => {
            handle.last_error = Some(format!("failed to serialize debug report JSON: {err}"));
            ptr::null_mut()
        }
    }
}

#[no_mangle]
// Serialize machine state as owned JSON text, not a KQS file envelope
// or a complete debugger-session snapshot. Free the result with kokura_string_free.
pub unsafe extern "C" fn kokura_debug_session_save_state_json(
    handle: *mut KokuraDebugSessionHandle,
) -> *mut c_char {
    let Some(handle) = handle_from_ptr(handle) else {
        return ptr::null_mut();
    };
    clear_last_error(handle);
    match serde_json::to_string(&handle.session.save_state()) {
        Ok(json) => into_c_string_ptr(json),
        Err(err) => {
            handle.last_error = Some(format!(
                "failed to serialize debug session state JSON: {err}"
            ));
            ptr::null_mut()
        }
    }
}

#[no_mangle]
// Decode a MachineState JSON payload, delegate application to the
// session and advance the callback cursor to its resulting log end. This
// wrapper does not validate state invariants or compare the live ROM.
pub unsafe extern "C" fn kokura_debug_session_load_state_json(
    handle: *mut KokuraDebugSessionHandle,
    state_json: *const c_char,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    let text = match cstr_to_str(state_json) {
        Ok(text) => text,
        Err(err) => return set_last_error(handle, err, false),
    };
    let state = match serde_json::from_str::<MachineState>(text) {
        Ok(state) => state,
        Err(err) => {
            return set_last_error(
                handle,
                format!("failed to parse MachineState JSON: {err}"),
                false,
            )
        }
    };
    handle.session.load_state(&state);
    handle.callback_event_cursor = handle.session.event_log.len();
    true
}

#[no_mangle]
// Serialize the optional stop reason into an owned C string. No stop
// is represented by JSON null text, distinct from a null pointer on failure.
pub unsafe extern "C" fn kokura_debug_session_stop_reason_json(
    handle: *mut KokuraDebugSessionHandle,
) -> *mut c_char {
    let Some(handle) = handle_from_ptr(handle) else {
        return ptr::null_mut();
    };
    clear_last_error(handle);
    match serde_json::to_string(&handle.session.stop_reason()) {
        Ok(json) => into_c_string_ptr(json),
        Err(err) => {
            handle.last_error = Some(format!("failed to serialize stop reason JSON: {err}"));
            ptr::null_mut()
        }
    }
}

#[no_mangle]
// Copy the last error into an owned C string without clearing it.
// Null means no stored error or a null handle; free copied text with kokura_string_free.
pub unsafe extern "C" fn kokura_debug_session_last_error(
    handle: *mut KokuraDebugSessionHandle,
) -> *mut c_char {
    let Some(handle) = handle_from_ptr(handle) else {
        return ptr::null_mut();
    };
    match &handle.last_error {
        Some(err) => into_c_string_ptr(err.clone()),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
// Clear the stored diagnostic message, returning false only for null.
pub unsafe extern "C" fn kokura_debug_session_clear_last_error(
    handle: *mut KokuraDebugSessionHandle,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    true
}

#[no_mangle]
// Borrow byte framebuffer storage; null handles return null. Do not
// free or retain the pointer across session/machine mutation or destruction.
pub unsafe extern "C" fn kokura_debug_session_framebuffer_ptr(
    handle: *mut KokuraDebugSessionHandle,
) -> *const u8 {
    let Some(handle) = handle_from_ptr(handle) else {
        return ptr::null();
    };
    handle.session.machine.framebuffer().as_ptr()
}

#[no_mangle]
// Return byte-framebuffer element count, or zero for null; do not clear last_error.
pub unsafe extern "C" fn kokura_debug_session_framebuffer_len(
    handle: *mut KokuraDebugSessionHandle,
) -> usize {
    handle_from_ptr(handle)
        .map(|handle| handle.session.machine.framebuffer().len())
        .unwrap_or(0)
}

#[no_mangle]
// Borrow the RGB555 word framebuffer, or return null for null. Copy
// it before any session mutation; ownership remains with the machine.
pub unsafe extern "C" fn kokura_debug_session_framebuffer_rgb555_ptr(
    handle: *mut KokuraDebugSessionHandle,
) -> *const u16 {
    let Some(handle) = handle_from_ptr(handle) else {
        return ptr::null();
    };
    handle.session.machine.framebuffer_rgb555().as_ptr()
}

#[no_mangle]
// Return the RGB555 u16 element count, not byte size; null returns zero.
pub unsafe extern "C" fn kokura_debug_session_framebuffer_rgb555_len(
    handle: *mut KokuraDebugSessionHandle,
) -> usize {
    handle_from_ptr(handle)
        .map(|handle| handle.session.machine.framebuffer_rgb555().len())
        .unwrap_or(0)
}

#[no_mangle]
// Report compatibility mode, returning false for null without clearing last_error.
pub unsafe extern "C" fn kokura_debug_session_is_cgb_compat_mode(
    handle: *mut KokuraDebugSessionHandle,
) -> bool {
    handle_from_ptr(handle)
        .map(|handle| handle.session.machine.is_cgb_compat_mode())
        .unwrap_or(false)
}

#[no_mangle]
// Clear last_error and copy CPU state into one writable output object;
// null output returns false and records a message on a valid handle.
pub unsafe extern "C" fn kokura_debug_session_cpu_snapshot(
    handle: *mut KokuraDebugSessionHandle,
    out_snapshot: *mut KokuraCpuSnapshot,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    if out_snapshot.is_null() {
        return set_last_error(
            handle,
            "CPU snapshot output pointer must be non-null",
            false,
        );
    }
    *out_snapshot = cpu_snapshot(&handle.session.machine);
    true
}

#[no_mangle]
// Clear last_error and inspect one byte without ordinary bus-read side
// effects. Null returns FF, which is also a valid memory value.
pub unsafe extern "C" fn kokura_debug_session_peek8(
    handle: *mut KokuraDebugSessionHandle,
    addr: u16,
) -> u8 {
    let Some(handle) = handle_from_ptr(handle) else {
        return 0xFF;
    };
    clear_last_error(handle);
    handle.session.machine.peek8(addr)
}

#[no_mangle]
// Clear last_error and perform a bus read with modeled side effects;
// null returns FF. Use peek8 when inspecting rather than accessing a device.
pub unsafe extern "C" fn kokura_debug_session_read8(
    handle: *mut KokuraDebugSessionHandle,
    addr: u16,
) -> u8 {
    let Some(handle) = handle_from_ptr(handle) else {
        return 0xFF;
    };
    clear_last_error(handle);
    handle.session.machine.read8(addr)
}

#[no_mangle]
// Clear last_error and issue a machine bus write. A true result does
// not guarantee that the device accepted the value through its access gates.
pub unsafe extern "C" fn kokura_debug_session_write8(
    handle: *mut KokuraDebugSessionHandle,
    addr: u16,
    value: u8,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    handle.session.machine.write8(addr, value);
    true
}

#[no_mangle]
// Return queued stereo frames or zero for null, preserving last_error.
pub unsafe extern "C" fn kokura_debug_session_audio_frames_available(
    handle: *mut KokuraDebugSessionHandle,
) -> usize {
    handle_from_ptr(handle)
        .map(|handle| handle.session.machine.audio_frames_available())
        .unwrap_or(0)
}

#[no_mangle]
// Return the output sample rate or zero for null, preserving last_error.
pub unsafe extern "C" fn kokura_debug_session_audio_sample_rate(
    handle: *mut KokuraDebugSessionHandle,
) -> u32 {
    handle_from_ptr(handle)
        .map(|handle| handle.session.machine.audio_sample_rate())
        .unwrap_or(0)
}

#[no_mangle]
// Return the accumulated dropped-frame count or zero for null.
pub unsafe extern "C" fn kokura_debug_session_audio_frames_dropped(
    handle: *mut KokuraDebugSessionHandle,
) -> u64 {
    handle_from_ptr(handle)
        .map(|handle| handle.session.machine.audio_frames_dropped())
        .unwrap_or(0)
}

#[no_mangle]
// Return audio capacity in stereo frames, or zero for null.
pub unsafe extern "C" fn kokura_debug_session_audio_buffer_capacity_frames(
    handle: *mut KokuraDebugSessionHandle,
) -> usize {
    handle_from_ptr(handle)
        .map(|handle| handle.session.machine.audio_buffer_capacity_frames())
        .unwrap_or(0)
}

#[no_mangle]
// Clear last_error and request a queue capacity; record a rejected
// capacity as false plus an error message.
pub unsafe extern "C" fn kokura_debug_session_set_audio_buffer_capacity_frames(
    handle: *mut KokuraDebugSessionHandle,
    capacity_frames: usize,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    match handle
        .session
        .machine
        .set_audio_buffer_capacity_frames(capacity_frames)
    {
        Ok(()) => true,
        Err(err) => set_last_error(handle, err, false),
    }
}

#[no_mangle]
// Drain up to max_frames stereo frames into nonoverlapping caller
// storage for 2 * max_frames i16 elements. Return frames copied, or zero
// for null/zero/empty input conditions; this call preserves last_error.
pub unsafe extern "C" fn kokura_debug_session_audio_copy_interleaved_i16(
    handle: *mut KokuraDebugSessionHandle,
    dst: *mut i16,
    max_frames: usize,
) -> usize {
    let Some(handle) = handle_from_ptr(handle) else {
        return 0;
    };
    if dst.is_null() || max_frames == 0 {
        return 0;
    }
    let samples = handle
        .session
        .machine
        .drain_audio_frames_interleaved_i16(max_frames);
    if samples.is_empty() {
        return 0;
    }
    ptr::copy_nonoverlapping(samples.as_ptr(), dst, samples.len());
    samples.len() / 2
}

#[no_mangle]
// Register a non-null C callback and borrowed user_data with the current
// configuration, skipping old events. Keep both callable/data valid until
// unregistration; callbacks must not reenter or destroy this borrowed session.
pub unsafe extern "C" fn kokura_debug_session_set_callback(
    handle: *mut KokuraDebugSessionHandle,
    callback: Option<KokuraDebugCallbackFn>,
    user_data: *mut c_void,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    let Some(callback) = callback else {
        return set_last_error(
            handle,
            "callback function pointer must be non-null; use clear_callback to unregister",
            false,
        );
    };
    handle.callback = Some(DebugCallbackRegistration {
        callback,
        user_data,
        config: handle.callback_config.clone(),
    });
    handle.callback_event_cursor = handle.session.event_log.len();
    true
}

#[no_mangle]
// Unregister the callback without freeing caller-owned user_data.
pub unsafe extern "C" fn kokura_debug_session_clear_callback(
    handle: *mut KokuraDebugSessionHandle,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    handle.callback = None;
    true
}

#[no_mangle]
// Validate/normalize callback settings and apply them to both saved
// configuration and any active registration; preserve old settings on failure.
pub unsafe extern "C" fn kokura_debug_session_set_callback_config_json(
    handle: *mut KokuraDebugSessionHandle,
    callback_config_json: *const c_char,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    let text = match cstr_to_str(callback_config_json) {
        Ok(text) => text,
        Err(err) => return set_last_error(handle, err, false),
    };
    let parsed = match serde_json::from_str::<DebugCallbackConfig>(text) {
        Ok(value) => value,
        Err(err) => {
            return set_last_error(
                handle,
                format!("failed to parse callback config JSON: {err}"),
                false,
            )
        }
    };
    let parsed = match parsed.validate() {
        Ok(value) => value,
        Err(err) => return set_last_error(handle, err, false),
    };
    handle.callback_config = parsed.clone();
    if let Some(callback) = &mut handle.callback {
        callback.config = parsed;
    }
    true
}

#[no_mangle]
// Restore default callback settings for future and active registrations
// without unregistering the callback itself.
pub unsafe extern "C" fn kokura_debug_session_clear_callback_config(
    handle: *mut KokuraDebugSessionHandle,
) -> bool {
    let Some(handle) = handle_from_ptr(handle) else {
        return false;
    };
    clear_last_error(handle);
    handle.callback_config = DebugCallbackConfig::default();
    if let Some(callback) = &mut handle.callback {
        callback.config = DebugCallbackConfig::default();
    }
    true
}

// Visit unconsumed log events and synchronously emit selected labels,
// then advance the cursor. A serialization failure returns before cursor
// advancement, so a later attempt can revisit earlier events in that slice.
fn emit_new_event_callbacks(
    callback: &DebugCallbackRegistration,
    config: &DebugCallbackConfig,
    event_log: &[DebugEvent],
    cursor: &mut usize,
) -> Result<(), String> {
    let start = (*cursor).min(event_log.len());
    for event in &event_log[start..] {
        let event_type = normalize_event_type_from_debug_event(event);
        if config.wants_event_type(&event_type) {
            let payload = DebugCallbackEventPayload {
                event_type,
                event: event.clone(),
            };
            invoke_debug_callback(callback, "event", &payload)?;
        }
    }
    *cursor = event_log.len();
    Ok(())
}

// Create temporary NUL-terminated kind/JSON strings and call foreign
// code synchronously. Their pointers are borrowed only during this call;
// the callback must copy retained text, never free it, and must not unwind.
fn invoke_debug_callback<T: Serialize>(
    callback: &DebugCallbackRegistration,
    kind: &str,
    payload: &T,
) -> Result<(), String> {
    let kind_c =
        CString::new(kind).map_err(|_| format!("callback kind contained interior NUL: {kind}"))?;
    let payload_json = serde_json::to_string(payload)
        .map_err(|err| format!("failed to serialize callback payload JSON: {err}"))?;
    let payload_c = CString::new(payload_json)
        .map_err(|_| "callback payload JSON contained interior NUL".to_string())?;
    unsafe {
        (callback.callback)(kind_c.as_ptr(), payload_c.as_ptr(), callback.user_data);
    }
    Ok(())
}

// Map each typed event to its callback category, grouping selected
// start/end events and distinguishing HBlank from general DMA blocks.
fn normalize_event_type_from_debug_event(event: &DebugEvent) -> String {
    match event {
        DebugEvent::ScanlineAdvance { .. } => "scanline".to_string(),
        DebugEvent::PpuModeChange { .. } => "ppu_mode".to_string(),
        DebugEvent::ScanlineRender { .. } => "scanline_render".to_string(),
        DebugEvent::OamDmaStart { .. } | DebugEvent::OamDmaComplete { .. } => "oam_dma".to_string(),
        DebugEvent::GdmaStallEstimate { .. } => "gdma_stall".to_string(),
        DebugEvent::HdmaStart { .. } => "hdma_start".to_string(),
        DebugEvent::HdmaBlock { hblank_mode, .. } => if *hblank_mode {
            "hdma_block"
        } else {
            "gdma_block"
        }
        .to_string(),
        DebugEvent::HdmaComplete { hblank_mode, .. } => if *hblank_mode {
            "hdma_complete"
        } else {
            "gdma_complete"
        }
        .to_string(),
        DebugEvent::HdmaCancel { .. } => "hdma_cancel".to_string(),
        DebugEvent::HdmaDeferred { .. } => "hdma_deferred".to_string(),
        DebugEvent::HdmaWriteIgnored { .. } => "hdma_write_ignored".to_string(),
        DebugEvent::MapperControlWrite { .. } => "mapper_ctrl".to_string(),
        DebugEvent::MapperRomBankChange { .. } => "mapper_rom_bank".to_string(),
        DebugEvent::MapperRamBankChange { .. } => "mapper_ram_bank".to_string(),
        DebugEvent::ApuMasterToggle { .. } => "apu_master".to_string(),
        DebugEvent::ApuChannelTrigger { .. } => "apu_trigger".to_string(),
        DebugEvent::ApuChannelLengthExpired { .. } => "apu_length".to_string(),
        DebugEvent::ApuEnvelopeStep { .. } => "apu_env".to_string(),
        DebugEvent::ApuSweepStep { .. } => "apu_sweep".to_string(),
        DebugEvent::ApuChannelDisabled { .. } => "apu_disable".to_string(),
        DebugEvent::ApuDacStateChange { .. } => "apu_dac".to_string(),
        DebugEvent::ApuPopRisk { .. } => "apu_pop".to_string(),
        DebugEvent::ApuFrameSequencerStep { .. } => "apu_frame".to_string(),
        DebugEvent::ApuMixerControl { .. } => "apu_mix_ctrl".to_string(),
        DebugEvent::ApuMixedOutput { .. } => "apu_mix".to_string(),
        DebugEvent::ApuPcmFramesBuffered { .. } => "apu_pcm".to_string(),
        DebugEvent::ApuPcmBufferWrapped { .. } => "apu_drop".to_string(),
        DebugEvent::ApuWaveRamWrite { .. } => "wave_ram".to_string(),
        DebugEvent::ApuWaveRamAccessAliased { .. } => "apu_wave_alias".to_string(),
        DebugEvent::ApuCh3TriggerRetainsSample { .. } => "ch3_hold".to_string(),
        DebugEvent::ApuNoiseClockFrozen { .. } => "noise_lock".to_string(),
        DebugEvent::CgbModeSelected { .. } => "cgb_mode".to_string(),
        DebugEvent::CgbVramBankSwitch { .. } => "cgb_vram_bank".to_string(),
        DebugEvent::CgbWramBankSwitch { .. } => "cgb_wram_bank".to_string(),
        DebugEvent::CgbBgPaletteIndexWrite { .. } => "cgb_bgpi".to_string(),
        DebugEvent::CgbBgPaletteDataWrite { .. } => "cgb_bgpd".to_string(),
        DebugEvent::CgbObjPaletteIndexWrite { .. } => "cgb_obpi".to_string(),
        DebugEvent::CgbObjPaletteDataWrite { .. } => "cgb_obpd".to_string(),
        DebugEvent::CgbKey1Write { .. } => "key1".to_string(),
        DebugEvent::CgbSpeedSwitch { .. } => "speed_switch".to_string(),
        DebugEvent::CgbSpeedSwitchFreeze { .. } => "speed_freeze".to_string(),
        DebugEvent::LcdToggle { .. } => "lcd_toggle".to_string(),
        DebugEvent::StatWrite { .. } => "stat_write".to_string(),
        DebugEvent::LycWrite { .. } => "lyc_write".to_string(),
        DebugEvent::StatSignal { .. } => "stat".to_string(),
        DebugEvent::VblankEnter { .. } => "vblank".to_string(),
        DebugEvent::FrameComplete { .. } => "frame".to_string(),
        DebugEvent::BankSwitch { .. } => "bank".to_string(),
        DebugEvent::FarCallSuspected { .. } => "farcall".to_string(),
        DebugEvent::KitaqgbIntrinsic { .. } => "intrinsic".to_string(),
        DebugEvent::BankReturnMissing { .. } => "bank_return_missing".to_string(),
        DebugEvent::TimerInterrupt { .. } => "timer".to_string(),
        DebugEvent::TimerOverflow { .. } => "timer_overflow".to_string(),
        DebugEvent::TimerReload { .. } => "timer_reload".to_string(),
        DebugEvent::TimerControlWrite { .. } => "timer_ctrl".to_string(),
        DebugEvent::DivResetEdge { .. } => "div_reset".to_string(),
        DebugEvent::SerialTransferStart { .. } | DebugEvent::SerialTransferComplete { .. } => {
            "serial".to_string()
        }
        DebugEvent::JoypadEdge { .. } => "joypad_edge".to_string(),
        DebugEvent::JoypadRead { .. } => "joypad_read".to_string(),
        DebugEvent::JoypadSelectionWrite { .. } => "joypad_select".to_string(),
        DebugEvent::JoypadInterrupt { .. } => "joypad_irq".to_string(),
        DebugEvent::InterruptRequested { .. } => "irq_request".to_string(),
        DebugEvent::InterruptServiced { .. } => "irq_service".to_string(),
        DebugEvent::InterruptPendingBlocked { .. } => "irq_blocked".to_string(),
        DebugEvent::ExecutionStop { .. } => "stop".to_string(),
        DebugEvent::SymbolContextChange { .. } => "symbol_context".to_string(),
        DebugEvent::UnsupportedOpcode { .. } => "unsupported".to_string(),
        DebugEvent::ReplayCheckpointSaved { .. } => "replay_checkpoint".to_string(),
        DebugEvent::ReplayRewindApplied { .. } => "replay_rewind".to_string(),
        DebugEvent::ReplayDivergenceDetected { .. } => "replay_divergence".to_string(),
        DebugEvent::BankThrashSuspected { .. } => "thrash".to_string(),
    }
}

// Trim/lowercase configured labels and map recognized aliases to
// callback categories. Unknown labels remain normalized literal strings.
fn normalize_event_type_label(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "scanlineadvance" | "scanline_advance" | "scanline" => "scanline".to_string(),
        "ppumodechange" | "ppu_mode_change" | "ppumode" | "mode" | "ppu_mode" => {
            "ppu_mode".to_string()
        }
        "scanlinerender" | "scanline_render" | "render" => "scanline_render".to_string(),
        "oamdma" | "oam_dma" | "dma" => "oam_dma".to_string(),
        "gdmastall" | "gdma_stall" | "dma_stall" => "gdma_stall".to_string(),
        "hdmastart" | "hdma_start" => "hdma_start".to_string(),
        "hdmablock" | "hdma_block" => "hdma_block".to_string(),
        "gdmablock" | "gdma_block" => "gdma_block".to_string(),
        "hdmacomplete" | "hdma_complete" => "hdma_complete".to_string(),
        "gdmacomplete" | "gdma_complete" => "gdma_complete".to_string(),
        "hdmacancel" | "hdma_cancel" | "dma_cancel" => "hdma_cancel".to_string(),
        "hdmadeferred" | "hdma_deferred" | "dma_deferred" => "hdma_deferred".to_string(),
        "hdmawriteignored" | "hdma_write_ignored" | "dma_ignored" => {
            "hdma_write_ignored".to_string()
        }
        "mappercontrolwrite" | "mapper_control_write" | "mapperctrl" | "mapper_ctrl" | "mapper" => {
            "mapper_ctrl".to_string()
        }
        "mapperrombankchange" | "mapper_rom_bank_change" | "mapperrombank" | "mapper_rom_bank" => {
            "mapper_rom_bank".to_string()
        }
        "mapperrambankchange" | "mapper_ram_bank_change" | "mapperrambank" | "mapper_ram_bank" => {
            "mapper_ram_bank".to_string()
        }
        "apumaster" | "apu_master" | "nr52" => "apu_master".to_string(),
        "aputrigger" | "apu_trigger" | "channel_trigger" => "apu_trigger".to_string(),
        "apulength" | "apu_length" => "apu_length".to_string(),
        "apuenv" | "apu_env" => "apu_env".to_string(),
        "apusweep" | "apu_sweep" => "apu_sweep".to_string(),
        "apudisable" | "apu_disable" => "apu_disable".to_string(),
        "apuframe" | "apu_frame" | "frame_seq" | "framesequencer" => "apu_frame".to_string(),
        "apupcm" | "apu_pcm" | "pcm" | "audio_buffer" => "apu_pcm".to_string(),
        "apudrop" | "apu_drop" | "pcm_drop" | "audio_drop" => "apu_drop".to_string(),
        "apudac" | "apu_dac" | "dac" => "apu_dac".to_string(),
        "apupop" | "apu_pop" | "pop" => "apu_pop".to_string(),
        "cgbmode" | "cgb_mode" | "cgb" => "cgb_mode".to_string(),
        "cgbvrambank" | "cgb_vram_bank" | "vbk" => "cgb_vram_bank".to_string(),
        "cgbwrambank" | "cgb_wram_bank" | "svbk" => "cgb_wram_bank".to_string(),
        "cgbbgpi" | "cgb_bgpi" | "bgpi" => "cgb_bgpi".to_string(),
        "cgbbgpd" | "cgb_bgpd" | "bgpd" | "cgbpalette" | "cgb_palette" => "cgb_bgpd".to_string(),
        "cgbobpi" | "cgb_obpi" | "obpi" => "cgb_obpi".to_string(),
        "cgbobpd" | "cgb_obpd" | "obpd" => "cgb_obpd".to_string(),
        "key1" | "speedarm" | "speed_arm" | "cgbspeed" | "cgb_speed" => "key1".to_string(),
        "speedswitch" | "speed_switch" | "double_speed" => "speed_switch".to_string(),
        "speedfreeze" | "speed_freeze" | "stopfreeze" | "stop_freeze" => "speed_freeze".to_string(),
        "waveram" | "wave_ram" | "apu_wave" => "wave_ram".to_string(),
        "apuwavealias" | "apu_wave_alias" | "wavealias" | "wave_alias" => {
            "apu_wave_alias".to_string()
        }
        "ch3hold" | "ch3_hold" | "samplehold" | "sample_hold" => "ch3_hold".to_string(),
        "noiselock" | "noise_lock" | "noisefreeze" | "noise_freeze" => "noise_lock".to_string(),
        "lcdtoggle" | "lcd_toggle" | "lcdc_toggle" | "lcd" => "lcd_toggle".to_string(),
        "statwrite" | "stat_write" | "statcfg" => "stat_write".to_string(),
        "lycwrite" | "lyc_write" | "lyc" => "lyc_write".to_string(),
        "statsignal" | "stat_signal" | "stat" | "stat_irq" => "stat".to_string(),
        "vblankenter" | "vblank_enter" | "vblank" => "vblank".to_string(),
        "framecomplete" | "frame_complete" | "frame" => "frame".to_string(),
        "bankswitch" | "bank_switch" | "bank" => "bank".to_string(),
        "farcallsuspected" | "far_call_suspected" | "farcall" | "far_call" => "farcall".to_string(),
        "kitaqgbintrinsic" | "intrinsic" => "intrinsic".to_string(),
        "bankreturnmissing" | "bank_return_missing" => "bank_return_missing".to_string(),
        "timerinterrupt" | "timer_interrupt" | "timer" => "timer".to_string(),
        "timeroverflow" | "timer_overflow" => "timer_overflow".to_string(),
        "timerreload" | "timer_reload" => "timer_reload".to_string(),
        "timercontrolwrite" | "timer_ctrl" | "timer_control" => "timer_ctrl".to_string(),
        "divresetedge" | "div_reset" | "div" => "div_reset".to_string(),
        "serialtransferstart" | "serialtransfercomplete" | "serial" => "serial".to_string(),
        "joypadedge" | "joypad_edge" => "joypad_edge".to_string(),
        "joypadread" | "joypad_read" | "p1_read" => "joypad_read".to_string(),
        "joypadselectionwrite" | "joypad_select" | "joypad_selection" => {
            "joypad_select".to_string()
        }
        "joypadinterrupt" | "joypad_irq" => "joypad_irq".to_string(),
        "interruptrequested" | "irq_request" => "irq_request".to_string(),
        "interruptserviced" | "irq_service" => "irq_service".to_string(),
        "interruptpendingblocked" | "irq_blocked" => "irq_blocked".to_string(),
        "executionstop" | "stop" => "stop".to_string(),
        "symbolcontextchange" | "symbol_context" => "symbol_context".to_string(),
        "unsupportedopcode" | "unsupported" => "unsupported".to_string(),
        "replaycheckpointsaved" | "replay_checkpoint" => "replay_checkpoint".to_string(),
        "replayrewindapplied" | "replay_rewind" => "replay_rewind".to_string(),
        "replaydivergencedetected" | "replay_divergence" => "replay_divergence".to_string(),
        "bankthrashsuspected" | "thrash" => "thrash".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        kokura_debug_session_create, kokura_debug_session_destroy,
        kokura_debug_session_set_joypad_mask, normalize_event_type_label, DebugCallbackConfig,
    };

    #[test]
    // Check alias normalization and deduplication with a frame interval of two.
    fn callback_config_normalizes_event_types() {
        let config = DebugCallbackConfig {
            frame_interval: Some(2),
            event_types: vec![
                "PpuModeChange".to_string(),
                "ppu_mode".to_string(),
                "JoypadRead".to_string(),
            ],
            emit_stop: true,
        }
        .validate()
        .unwrap();
        assert_eq!(
            config.event_types,
            vec!["joypad_read".to_string(), "ppu_mode".to_string()]
        );
    }

    #[test]
    // Check three representative event aliases against their expected labels.
    fn callback_event_aliases_match_cli_labels() {
        assert_eq!(normalize_event_type_label("PpuModeChange"), "ppu_mode");
        assert_eq!(normalize_event_type_label("dma"), "oam_dma");
        assert_eq!(normalize_event_type_label("joypad_read"), "joypad_read");
    }

    #[test]
    // Create a C handle, set/release START and inspect the internal mask,
    // then destroy it. This test does not execute a ROM or verify an IRQ edge.
    fn debug_session_set_joypad_mask_updates_machine() {
        let handle = kokura_debug_session_create();
        assert!(!handle.is_null());
        unsafe {
            assert!(kokura_debug_session_set_joypad_mask(handle, 0x80));
            {
                let inner = super::handle_from_ptr(handle).unwrap();
                assert_eq!(inner.session.machine.joypad.mask, 0x80);
            }
            assert!(kokura_debug_session_set_joypad_mask(handle, 0x00));
            {
                let inner = super::handle_from_ptr(handle).unwrap();
                assert_eq!(inner.session.machine.joypad.mask, 0x00);
            }
            kokura_debug_session_destroy(handle);
        }
    }
}
