#ifndef KOKURA_CAPI_H
#define KOKURA_CAPI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Handles are opaque, library-owned allocations. Serialize all access,
// including getters, and never reuse a destroyed handle. Input pointers must
// reference readable storage and output pointers sufficiently sized writable
// storage for the duration of the call. No pointer provenance check is provided.
typedef struct KokuraCoreHandle KokuraCoreHandle;
typedef struct KokuraDebugSessionHandle KokuraDebugSessionHandle;

typedef struct KokuraStepResult {
    uint32_t cycles;
    bool frame_completed;
    uint16_t current_rom_bank;
    uint16_t current_ram_bank;
    bool is_cgb_mode;
    bool is_double_speed;
    const uint8_t* framebuffer_ptr;
    size_t framebuffer_len;
    const void* reserved;
} KokuraStepResult;

typedef struct KokuraDebugRunResult {
    uint64_t frames_requested;
    uint64_t frames_executed;
    bool stopped_by_debugger;
    bool has_stop_reason;
    bool halted_on_unsupported_opcode;
} KokuraDebugRunResult;

typedef struct KokuraCpuSnapshot {
    uint16_t pc;
    uint16_t sp;
    uint16_t af;
    uint16_t bc;
    uint16_t de;
    uint16_t hl;
    bool ime;
    bool halted;
    bool halt_bug;
    bool stopped;
    uint16_t current_rom_bank;
    uint16_t current_ram_bank;
    uint64_t cycles;
    uint64_t frames;
    bool is_cgb_mode;
    bool is_double_speed;
} KokuraCpuSnapshot;

// Callbacks run synchronously while the session is borrowed. Do not reenter
// or destroy that session, and do not unwind through the C ABI. kind/payload_json
// are borrowed only until return: copy them if needed and never free them.
// Keep the callback and caller-owned user_data alive until unregistered.
typedef void (*KokuraDebugCallbackFn)(
    const char* kind,
    const char* payload_json,
    void* user_data
);

// Allocate a new machine and transfer the opaque handle to the caller;
// release it once with kokura_core_destroy from the same library.
KokuraCoreHandle* kokura_core_create(void);
// Drop the live machine allocation, accepting null. A non-null handle
// must originate from this library and must not be borrowed or destroyed twice.
void kokura_core_destroy(KokuraCoreHandle* handle);
// Copy rom_len readable bytes into owned storage and load the cartridge.
// Reject null/empty input or load errors with false; detailed errors are not
// returned by this core-only API.
bool kokura_core_load_rom(KokuraCoreHandle* handle, const uint8_t* rom_ptr, size_t rom_len);
// Advance one machine step and optionally fill a writable result object.
// The returned framebuffer is borrowed; inspect/copy it before mutating or
// destroying the machine. Failure does not promise an unchanged machine.
bool kokura_core_step(KokuraCoreHandle* handle, KokuraStepResult* out_result);
// Run one frame and return whether execution completed without an error.
bool kokura_core_run_frame(KokuraCoreHandle* handle);
// Run up to the requested frames, returning false at the first error.
// Earlier progress remains applied; a valid handle with zero frames succeeds.
bool kokura_core_run_frames(KokuraCoreHandle* handle, uint64_t frames);
// Apply the host input mask through the machine-level input path,
// including its modeled edge handling. Null handles return false.
bool kokura_core_set_joypad_mask(KokuraCoreHandle* handle, uint8_t mask);
// Borrow the byte framebuffer, or return null for a null handle.
// Do not free it; copy/read it before any machine mutation or destruction.
const uint8_t* kokura_core_framebuffer_ptr(KokuraCoreHandle* handle);
// Return the byte framebuffer element count, or zero for a null handle.
size_t kokura_core_framebuffer_len(KokuraCoreHandle* handle);
// Borrow the RGB555 word framebuffer, or return null for a null handle.
// The machine retains ownership; consume it before mutation or destruction.
const uint16_t* kokura_core_framebuffer_rgb555_ptr(KokuraCoreHandle* handle);
// Return the number of u16 RGB555 elements, not the byte size; null is zero.
size_t kokura_core_framebuffer_rgb555_len(KokuraCoreHandle* handle);
// Report the machine compatibility-mode flag, using false for null.
bool kokura_core_is_cgb_compat_mode(KokuraCoreHandle* handle);
// Return the controller ROM-bank label, using zero for null. A zero
// result alone cannot distinguish a valid bank-zero selection from null.
uint16_t kokura_core_current_rom_bank(KokuraCoreHandle* handle);
// Return the controller RAM-bank label, using zero for null.
uint16_t kokura_core_current_ram_bank(KokuraCoreHandle* handle);
// Fill one valid writable snapshot object; reject null handle/output.
// The copied snapshot contains no borrowed framebuffer pointer.
bool kokura_core_cpu_snapshot(KokuraCoreHandle* handle, KokuraCpuSnapshot* out_snapshot);
// Inspect a mapped byte without ordinary device-read side effects.
// A null handle returns FF, which is also a possible memory value.
uint8_t kokura_core_peek8(KokuraCoreHandle* handle, uint16_t addr);
// Perform an ordinary machine bus read, including modeled side effects;
// null returns FF. Use peek8 for observational inspection.
uint8_t kokura_core_read8(KokuraCoreHandle* handle, uint16_t addr);
// Perform an ordinary machine bus write; true means the call was made,
// not that hardware gates necessarily accepted or retained the byte.
bool kokura_core_write8(KokuraCoreHandle* handle, uint16_t addr, uint8_t value);
// Return output samples per second, or zero for a null handle.
uint32_t kokura_core_audio_sample_rate(KokuraCoreHandle* handle);
// Return queued stereo frames, not interleaved sample elements; null is zero.
size_t kokura_core_audio_frames_available(KokuraCoreHandle* handle);
// Return the accumulated dropped-audio-frame count, or zero for null.
uint64_t kokura_core_audio_frames_dropped(KokuraCoreHandle* handle);
// Return audio queue capacity in stereo frames, or zero for null.
size_t kokura_core_audio_buffer_capacity_frames(KokuraCoreHandle* handle);
// Request an audio queue capacity and return whether the machine accepts
// it; null and invalid-capacity errors return false.
bool kokura_core_set_audio_buffer_capacity_frames(KokuraCoreHandle* handle, size_t capacity_frames);
// Drain up to max_frames stereo frames into caller-owned storage and
// return the number drained. dst must hold 2 * max_frames i16 elements
// and must not overlap source storage; null/zero requests drain nothing.
size_t kokura_core_audio_copy_interleaved_i16(KokuraCoreHandle* handle, int16_t* dst, size_t max_frames);
// Serialize the MachineState payload, including cartridge storage,
// into an owned UTF-8 C string. Free it with kokura_string_free; null reports
// failure. This JSON is not the binary KQS file envelope.
char* kokura_core_save_state_json(KokuraCoreHandle* handle);
// Decode a readable NUL-terminated UTF-8 MachineState JSON string and
// apply it. This wrapper performs no payload validation or live-ROM identity
// check after decoding; false reports null, UTF-8 or JSON decoding failure.
bool kokura_core_load_state_json(KokuraCoreHandle* handle, const char* state_json);

// Allocate a new machine/debug session with empty metadata, no callback
// and no stored error; release it once with the matching session destructor.
KokuraDebugSessionHandle* kokura_debug_session_create(void);
// Destroy a live session allocation, accepting null. The caller must
// ensure it is not in use, including by a callback, and destroy it only once.
void kokura_debug_session_destroy(KokuraDebugSessionHandle* handle);
// Copy readable ROM bytes into a new machine/session. Preserve local
// symbol metadata and callback registration/configuration, but replace prior
// session execution/replay/stop state only after ROM loading succeeds.
bool kokura_debug_session_load_rom(KokuraDebugSessionHandle* handle, const uint8_t* rom_ptr, size_t rom_len);
// Read a NUL-terminated UTF-8 map string, replace symbol spans and
// reinstall combined metadata. Invalid text encoding sets last_error;
// malformed individual map rows follow the permissive parser behavior.
bool kokura_debug_session_load_symbol_map_text(KokuraDebugSessionHandle* handle, const char* map_text);
// Replace source positions from a readable UTF-8 C string and rebuild
// session metadata, retaining the other local table collections.
bool kokura_debug_session_load_source_map_text(KokuraDebugSessionHandle* handle, const char* source_map_text);
// Decode and replace all local symbol metadata, then install it into
// the session; decoding errors leave the old table and set last_error.
bool kokura_debug_session_load_symbol_table_json(KokuraDebugSessionHandle* handle, const char* symbol_table_json);
// Clear both the retained table and session symbol metadata.
bool kokura_debug_session_clear_symbol_table(KokuraDebugSessionHandle* handle);
// Decode UTF-8 stop-condition JSON and pass it to session validation.
// Store parse/validation failures in last_error and return false.
bool kokura_debug_session_set_stop_conditions_json(KokuraDebugSessionHandle* handle, const char* stop_conditions_json);
// Clear the session stop-condition set through its dedicated method.
bool kokura_debug_session_clear_stop_conditions(KokuraDebugSessionHandle* handle);
// Decode replay configuration and delegate validation/application to
// the session, returning false with last_error on failure.
bool kokura_debug_session_set_replay_control_json(KokuraDebugSessionHandle* handle, const char* replay_control_json);
// Apply default disabled replay settings through normal session
// configuration handling, reporting any failure through last_error.
bool kokura_debug_session_clear_replay_control(KokuraDebugSessionHandle* handle);
// Clear the previous error and apply input through the machine-level
// path rather than writing the Joypad fields directly.
bool kokura_debug_session_set_joypad_mask(KokuraDebugSessionHandle* handle, uint8_t mask);
// Register a non-null C callback and borrowed user_data with the current
// configuration, skipping old events. Keep both callable/data valid until
// unregistration; callbacks must not reenter or destroy this borrowed session.
bool kokura_debug_session_set_callback(KokuraDebugSessionHandle* handle, KokuraDebugCallbackFn callback, void* user_data);
// Unregister the callback without freeing caller-owned user_data.
bool kokura_debug_session_clear_callback(KokuraDebugSessionHandle* handle);
// Validate/normalize callback settings and apply them to both saved
// configuration and any active registration; preserve old settings on failure.
bool kokura_debug_session_set_callback_config_json(KokuraDebugSessionHandle* handle, const char* callback_config_json);
// Restore default callback settings for future and active registrations
// without unregistering the callback itself.
bool kokura_debug_session_clear_callback_config(KokuraDebugSessionHandle* handle);
// Run with optional synchronous event/frame/stop callbacks and fill
// an optional writable result. Execution success can coexist with a callback
// serialization error in last_error. Frame counts are net, saturating deltas
// and may be reduced by rewind; a debugger stop need not return false.
bool kokura_debug_session_run_frames(KokuraDebugSessionHandle* handle, uint64_t frames, KokuraDebugRunResult* out_result);
// Request rewind through stored replay checkpoints and report a missing
// matching checkpoint via false and last_error.
bool kokura_debug_session_rewind_frames(KokuraDebugSessionHandle* handle, uint64_t frames_back);
// Serialize the current report into an owned UTF-8 C string; null
// reports failure. Release a non-null result with kokura_string_free.
char* kokura_debug_session_report_json(KokuraDebugSessionHandle* handle);
// Serialize machine state as owned JSON text, not a KQS file envelope
// or a complete debugger-session snapshot. Free the result with kokura_string_free.
char* kokura_debug_session_save_state_json(KokuraDebugSessionHandle* handle);
// Decode a MachineState JSON payload, delegate application to the
// session and advance the callback cursor to its resulting log end. This
// wrapper does not validate state invariants or compare the live ROM.
bool kokura_debug_session_load_state_json(KokuraDebugSessionHandle* handle, const char* state_json);
// Serialize the optional stop reason into an owned C string. No stop
// is represented by JSON null text, distinct from a null pointer on failure.
char* kokura_debug_session_stop_reason_json(KokuraDebugSessionHandle* handle);
// Copy the last error into an owned C string without clearing it.
// Null means no stored error or a null handle; free copied text with kokura_string_free.
char* kokura_debug_session_last_error(KokuraDebugSessionHandle* handle);
// Clear the stored diagnostic message, returning false only for null.
bool kokura_debug_session_clear_last_error(KokuraDebugSessionHandle* handle);
// Borrow byte framebuffer storage; null handles return null. Do not
// free or retain the pointer across session/machine mutation or destruction.
const uint8_t* kokura_debug_session_framebuffer_ptr(KokuraDebugSessionHandle* handle);
// Return byte-framebuffer element count, or zero for null; do not clear last_error.
size_t kokura_debug_session_framebuffer_len(KokuraDebugSessionHandle* handle);
// Borrow the RGB555 word framebuffer, or return null for null. Copy
// it before any session mutation; ownership remains with the machine.
const uint16_t* kokura_debug_session_framebuffer_rgb555_ptr(KokuraDebugSessionHandle* handle);
// Return the RGB555 u16 element count, not byte size; null returns zero.
size_t kokura_debug_session_framebuffer_rgb555_len(KokuraDebugSessionHandle* handle);
// Report compatibility mode, returning false for null without clearing last_error.
bool kokura_debug_session_is_cgb_compat_mode(KokuraDebugSessionHandle* handle);
// Clear last_error and copy CPU state into one writable output object;
// null output returns false and records a message on a valid handle.
bool kokura_debug_session_cpu_snapshot(KokuraDebugSessionHandle* handle, KokuraCpuSnapshot* out_snapshot);
// Clear last_error and inspect one byte without ordinary bus-read side
// effects. Null returns FF, which is also a valid memory value.
uint8_t kokura_debug_session_peek8(KokuraDebugSessionHandle* handle, uint16_t addr);
// Clear last_error and perform a bus read with modeled side effects;
// null returns FF. Use peek8 when inspecting rather than accessing a device.
uint8_t kokura_debug_session_read8(KokuraDebugSessionHandle* handle, uint16_t addr);
// Clear last_error and issue a machine bus write. A true result does
// not guarantee that the device accepted the value through its access gates.
bool kokura_debug_session_write8(KokuraDebugSessionHandle* handle, uint16_t addr, uint8_t value);
// Return the output sample rate or zero for null, preserving last_error.
uint32_t kokura_debug_session_audio_sample_rate(KokuraDebugSessionHandle* handle);
// Return queued stereo frames or zero for null, preserving last_error.
size_t kokura_debug_session_audio_frames_available(KokuraDebugSessionHandle* handle);
// Return the accumulated dropped-frame count or zero for null.
uint64_t kokura_debug_session_audio_frames_dropped(KokuraDebugSessionHandle* handle);
// Return audio capacity in stereo frames, or zero for null.
size_t kokura_debug_session_audio_buffer_capacity_frames(KokuraDebugSessionHandle* handle);
// Clear last_error and request a queue capacity; record a rejected
// capacity as false plus an error message.
bool kokura_debug_session_set_audio_buffer_capacity_frames(KokuraDebugSessionHandle* handle, size_t capacity_frames);
// Drain up to max_frames stereo frames into nonoverlapping caller
// storage for 2 * max_frames i16 elements. Return frames copied, or zero
// for null/zero/empty input conditions; this call preserves last_error.
size_t kokura_debug_session_audio_copy_interleaved_i16(KokuraDebugSessionHandle* handle, int16_t* dst, size_t max_frames);

// Allocate an owned C ABI version string; release it with kokura_string_free.
char* kokura_version_string(void);
// Release an owned string from this library once; null is allowed.
// Never free a borrowed callback string or use a foreign/interior pointer.
void kokura_string_free(char* ptr);

#ifdef __cplusplus
}
#endif

#endif
