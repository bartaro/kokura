#ifndef KOKURA_CAPI_H
#define KOKURA_CAPI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

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

typedef void (*KokuraDebugCallbackFn)(
    const char* kind,
    const char* payload_json,
    void* user_data
);

KokuraCoreHandle* kokura_core_create(void);
void kokura_core_destroy(KokuraCoreHandle* handle);
bool kokura_core_load_rom(KokuraCoreHandle* handle, const uint8_t* rom_ptr, size_t rom_len);
bool kokura_core_step(KokuraCoreHandle* handle, KokuraStepResult* out_result);
bool kokura_core_run_frame(KokuraCoreHandle* handle);
bool kokura_core_run_frames(KokuraCoreHandle* handle, uint64_t frames);
bool kokura_core_set_joypad_mask(KokuraCoreHandle* handle, uint8_t mask);
const uint8_t* kokura_core_framebuffer_ptr(KokuraCoreHandle* handle);
size_t kokura_core_framebuffer_len(KokuraCoreHandle* handle);
const uint16_t* kokura_core_framebuffer_rgb555_ptr(KokuraCoreHandle* handle);
size_t kokura_core_framebuffer_rgb555_len(KokuraCoreHandle* handle);
bool kokura_core_is_cgb_compat_mode(KokuraCoreHandle* handle);
uint16_t kokura_core_current_rom_bank(KokuraCoreHandle* handle);
uint16_t kokura_core_current_ram_bank(KokuraCoreHandle* handle);
bool kokura_core_cpu_snapshot(KokuraCoreHandle* handle, KokuraCpuSnapshot* out_snapshot);
uint8_t kokura_core_peek8(KokuraCoreHandle* handle, uint16_t addr);
uint8_t kokura_core_read8(KokuraCoreHandle* handle, uint16_t addr);
bool kokura_core_write8(KokuraCoreHandle* handle, uint16_t addr, uint8_t value);
uint32_t kokura_core_audio_sample_rate(KokuraCoreHandle* handle);
size_t kokura_core_audio_frames_available(KokuraCoreHandle* handle);
uint64_t kokura_core_audio_frames_dropped(KokuraCoreHandle* handle);
size_t kokura_core_audio_buffer_capacity_frames(KokuraCoreHandle* handle);
bool kokura_core_set_audio_buffer_capacity_frames(KokuraCoreHandle* handle, size_t capacity_frames);
size_t kokura_core_audio_copy_interleaved_i16(KokuraCoreHandle* handle, int16_t* dst, size_t max_frames);
char* kokura_core_save_state_json(KokuraCoreHandle* handle);
bool kokura_core_load_state_json(KokuraCoreHandle* handle, const char* state_json);

KokuraDebugSessionHandle* kokura_debug_session_create(void);
void kokura_debug_session_destroy(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_load_rom(KokuraDebugSessionHandle* handle, const uint8_t* rom_ptr, size_t rom_len);
bool kokura_debug_session_load_symbol_map_text(KokuraDebugSessionHandle* handle, const char* map_text);
bool kokura_debug_session_load_source_map_text(KokuraDebugSessionHandle* handle, const char* source_map_text);
bool kokura_debug_session_load_symbol_table_json(KokuraDebugSessionHandle* handle, const char* symbol_table_json);
bool kokura_debug_session_clear_symbol_table(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_set_stop_conditions_json(KokuraDebugSessionHandle* handle, const char* stop_conditions_json);
bool kokura_debug_session_clear_stop_conditions(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_set_replay_control_json(KokuraDebugSessionHandle* handle, const char* replay_control_json);
bool kokura_debug_session_clear_replay_control(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_set_joypad_mask(KokuraDebugSessionHandle* handle, uint8_t mask);
bool kokura_debug_session_set_callback(KokuraDebugSessionHandle* handle, KokuraDebugCallbackFn callback, void* user_data);
bool kokura_debug_session_clear_callback(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_set_callback_config_json(KokuraDebugSessionHandle* handle, const char* callback_config_json);
bool kokura_debug_session_clear_callback_config(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_run_frames(KokuraDebugSessionHandle* handle, uint64_t frames, KokuraDebugRunResult* out_result);
bool kokura_debug_session_rewind_frames(KokuraDebugSessionHandle* handle, uint64_t frames_back);
char* kokura_debug_session_report_json(KokuraDebugSessionHandle* handle);
char* kokura_debug_session_save_state_json(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_load_state_json(KokuraDebugSessionHandle* handle, const char* state_json);
char* kokura_debug_session_stop_reason_json(KokuraDebugSessionHandle* handle);
char* kokura_debug_session_last_error(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_clear_last_error(KokuraDebugSessionHandle* handle);
const uint8_t* kokura_debug_session_framebuffer_ptr(KokuraDebugSessionHandle* handle);
size_t kokura_debug_session_framebuffer_len(KokuraDebugSessionHandle* handle);
const uint16_t* kokura_debug_session_framebuffer_rgb555_ptr(KokuraDebugSessionHandle* handle);
size_t kokura_debug_session_framebuffer_rgb555_len(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_is_cgb_compat_mode(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_cpu_snapshot(KokuraDebugSessionHandle* handle, KokuraCpuSnapshot* out_snapshot);
uint8_t kokura_debug_session_peek8(KokuraDebugSessionHandle* handle, uint16_t addr);
uint8_t kokura_debug_session_read8(KokuraDebugSessionHandle* handle, uint16_t addr);
bool kokura_debug_session_write8(KokuraDebugSessionHandle* handle, uint16_t addr, uint8_t value);
uint32_t kokura_debug_session_audio_sample_rate(KokuraDebugSessionHandle* handle);
size_t kokura_debug_session_audio_frames_available(KokuraDebugSessionHandle* handle);
uint64_t kokura_debug_session_audio_frames_dropped(KokuraDebugSessionHandle* handle);
size_t kokura_debug_session_audio_buffer_capacity_frames(KokuraDebugSessionHandle* handle);
bool kokura_debug_session_set_audio_buffer_capacity_frames(KokuraDebugSessionHandle* handle, size_t capacity_frames);
size_t kokura_debug_session_audio_copy_interleaved_i16(KokuraDebugSessionHandle* handle, int16_t* dst, size_t max_frames);

char* kokura_version_string(void);
void kokura_string_free(char* ptr);

#ifdef __cplusplus
}
#endif

#endif
