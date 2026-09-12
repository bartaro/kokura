"""Stdlib-only ctypes bridge for the KOKURA C API."""

from __future__ import annotations

import ctypes as _ct
import json as _json
import os as _os
import struct as _struct
import sys as _sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterable, Optional


class KokuraError(RuntimeError):
    pass


@dataclass
class StepResult:
    cycles: int
    frame_completed: bool
    current_rom_bank: int
    current_ram_bank: int
    is_cgb_mode: bool
    is_double_speed: bool
    framebuffer_len: int


@dataclass
class DebugRunResult:
    frames_requested: int
    frames_executed: int
    stopped_by_debugger: bool
    has_stop_reason: bool
    halted_on_unsupported_opcode: bool


@dataclass
class RegisterSnapshot:
    pc: int
    sp: int
    af: int
    bc: int
    de: int
    hl: int
    ime: bool
    halted: bool
    halt_bug: bool
    stopped: bool
    current_rom_bank: int
    current_ram_bank: int
    cycles: int
    frames: int
    is_cgb_mode: bool
    is_double_speed: bool

    @property
    def a(self) -> int:
        return (self.af >> 8) & 0xFF

    @property
    def f(self) -> int:
        return self.af & 0xFF

    @property
    def b(self) -> int:
        return (self.bc >> 8) & 0xFF

    @property
    def c(self) -> int:
        return self.bc & 0xFF

    @property
    def d(self) -> int:
        return (self.de >> 8) & 0xFF

    @property
    def e(self) -> int:
        return self.de & 0xFF

    @property
    def h(self) -> int:
        return (self.hl >> 8) & 0xFF

    @property
    def l(self) -> int:
        return self.hl & 0xFF


class _CStepResult(_ct.Structure):
    _fields_ = [
        ("cycles", _ct.c_uint32),
        ("frame_completed", _ct.c_bool),
        ("current_rom_bank", _ct.c_uint16),
        ("current_ram_bank", _ct.c_uint16),
        ("is_cgb_mode", _ct.c_bool),
        ("is_double_speed", _ct.c_bool),
        ("framebuffer_ptr", _ct.POINTER(_ct.c_uint8)),
        ("framebuffer_len", _ct.c_size_t),
        ("reserved", _ct.c_void_p),
    ]


class _CDebugRunResult(_ct.Structure):
    _fields_ = [
        ("frames_requested", _ct.c_uint64),
        ("frames_executed", _ct.c_uint64),
        ("stopped_by_debugger", _ct.c_bool),
        ("has_stop_reason", _ct.c_bool),
        ("halted_on_unsupported_opcode", _ct.c_bool),
    ]


class _CCpuSnapshot(_ct.Structure):
    _fields_ = [
        ("pc", _ct.c_uint16),
        ("sp", _ct.c_uint16),
        ("af", _ct.c_uint16),
        ("bc", _ct.c_uint16),
        ("de", _ct.c_uint16),
        ("hl", _ct.c_uint16),
        ("ime", _ct.c_bool),
        ("halted", _ct.c_bool),
        ("halt_bug", _ct.c_bool),
        ("stopped", _ct.c_bool),
        ("current_rom_bank", _ct.c_uint16),
        ("current_ram_bank", _ct.c_uint16),
        ("cycles", _ct.c_uint64),
        ("frames", _ct.c_uint64),
        ("is_cgb_mode", _ct.c_bool),
        ("is_double_speed", _ct.c_bool),
    ]


_DEBUG_CALLBACK = _ct.CFUNCTYPE(None, _ct.c_char_p, _ct.c_char_p, _ct.c_void_p)


def _default_library_candidates() -> Iterable[Path]:
    if getattr(_sys, "frozen", False):
        exe_dir = Path(_sys.executable).resolve().parent
        yield exe_dir / "kokura_capi.dll"
        yield exe_dir / "libkokura_capi.so"
        yield exe_dir / "libkokura_capi.dylib"

        meipass = getattr(_sys, "_MEIPASS", None)
        if meipass:
            bundle_dir = Path(meipass)
            yield bundle_dir / "kokura_capi.dll"
            yield bundle_dir / "libkokura_capi.so"
            yield bundle_dir / "libkokura_capi.dylib"

    cwd = Path.cwd()
    for rel in [
        "target/release/libkokura_capi.so",
        "target/release/kokura_capi.dll",
        "target/release/libkokura_capi.dylib",
        "target/debug/libkokura_capi.so",
        "target/debug/kokura_capi.dll",
        "target/debug/libkokura_capi.dylib",
    ]:
        yield cwd / rel


def load_library(path: Optional[str | _os.PathLike[str]] = None) -> _ct.CDLL:
    if path is not None:
        return _ct.CDLL(str(path))
    for candidate in _default_library_candidates():
        if candidate.exists():
            return _ct.CDLL(str(candidate))
    raise KokuraError(
        "Could not locate kokura-capi shared library. Pass an explicit path or build the library first."
    )


def _buffer_from_bytes(data: bytes | bytearray | memoryview) -> tuple[_ct.Array[Any], int]:
    blob = bytes(data)
    return (_ct.c_uint8 * len(blob)).from_buffer_copy(blob), len(blob)


def _string_arg(text: str) -> _ct.c_char_p:
    return _ct.c_char_p(text.encode("utf-8"))


def _json_arg(value: Any) -> _ct.c_char_p:
    return _string_arg(_json.dumps(value))


def _snapshot_from_c(value: _CCpuSnapshot) -> RegisterSnapshot:
    return RegisterSnapshot(
        pc=int(value.pc),
        sp=int(value.sp),
        af=int(value.af),
        bc=int(value.bc),
        de=int(value.de),
        hl=int(value.hl),
        ime=bool(value.ime),
        halted=bool(value.halted),
        halt_bug=bool(value.halt_bug),
        stopped=bool(value.stopped),
        current_rom_bank=int(value.current_rom_bank),
        current_ram_bank=int(value.current_ram_bank),
        cycles=int(value.cycles),
        frames=int(value.frames),
        is_cgb_mode=bool(value.is_cgb_mode),
        is_double_speed=bool(value.is_double_speed),
    )


def _framebuffer_intensity_max(framebuffer: bytes) -> int:
    return 31 if any(pixel > 3 for pixel in framebuffer) else 3


def _framebuffer_gray_to_u8(value: int, max_value: int) -> int:
    shade = min(int(value), max_value)
    scaled = (shade * 255 + (max_value // 2)) // max(max_value, 1)
    return max(0, 255 - scaled)


def _compat_palette_rgb(framebuffer: bytes) -> bytes:
    cgb_compat = (
        (255, 255, 214),
        (181, 230, 115),
        (82, 148, 65),
        (16, 49, 24),
    )
    out = bytearray()
    for shade in framebuffer:
        out.extend(cgb_compat[min(int(shade), 3)])
    return bytes(out)


def _rgb_from_framebuffer(
    framebuffer: bytes,
    rgb555: Optional[bytes] = None,
    cgb_compat_mode: bool = False,
) -> bytes:
    if cgb_compat_mode:
        return _compat_palette_rgb(framebuffer)
    if rgb555 is not None and len(rgb555) == len(framebuffer) * 2:
        out = bytearray()
        for (value,) in _struct.iter_unpack("<H", rgb555):
            out.extend(
                (
                    ((value & 0x1F) * 255) // 31,
                    (((value >> 5) & 0x1F) * 255) // 31,
                    (((value >> 10) & 0x1F) * 255) // 31,
                )
            )
        return bytes(out)
    max_value = _framebuffer_intensity_max(framebuffer)
    out = bytearray()
    for value in framebuffer:
        gray = _framebuffer_gray_to_u8(value, max_value)
        out.extend((gray, gray, gray))
    return bytes(out)


def _encode_ppm(
    framebuffer: bytes,
    rgb555: Optional[bytes] = None,
    cgb_compat_mode: bool = False,
) -> bytes:
    out = bytearray(b"P6\n160 144\n255\n")
    out.extend(_rgb_from_framebuffer(framebuffer, rgb555, cgb_compat_mode))
    return bytes(out)


def _encode_bmp(
    framebuffer: bytes,
    rgb555: Optional[bytes] = None,
    cgb_compat_mode: bool = False,
) -> bytes:
    width = 160
    height = 144
    header_len = 14 + 40
    row_stride = (width * 3 + 3) & ~3
    pixel_bytes = row_stride * height
    file_size = header_len + pixel_bytes
    rgb = _rgb_from_framebuffer(framebuffer, rgb555, cgb_compat_mode)

    out = bytearray()
    out.extend(b"BM")
    out.extend(int(file_size).to_bytes(4, "little"))
    out.extend((0).to_bytes(2, "little"))
    out.extend((0).to_bytes(2, "little"))
    out.extend(int(header_len).to_bytes(4, "little"))
    out.extend((40).to_bytes(4, "little"))
    out.extend(int(width).to_bytes(4, "little", signed=True))
    out.extend(int(height).to_bytes(4, "little", signed=True))
    out.extend((1).to_bytes(2, "little"))
    out.extend((24).to_bytes(2, "little"))
    out.extend((0).to_bytes(4, "little"))
    out.extend(int(pixel_bytes).to_bytes(4, "little"))
    out.extend((2835).to_bytes(4, "little"))
    out.extend((2835).to_bytes(4, "little"))
    out.extend((0).to_bytes(4, "little"))
    out.extend((0).to_bytes(4, "little"))

    padding = row_stride - width * 3
    for y in range(height - 1, -1, -1):
        base = y * width
        for x in range(width):
            rgb_base = (base + x) * 3
            out.extend((rgb[rgb_base + 2], rgb[rgb_base + 1], rgb[rgb_base]))
        out.extend(b"\x00" * padding)
    return bytes(out)


def _write_screenshot(
    path: str | _os.PathLike[str],
    framebuffer: bytes,
    rgb555: Optional[bytes] = None,
    cgb_compat_mode: bool = False,
) -> Path:
    target = Path(path)
    suffix = target.suffix.lower()
    if not suffix:
        target = target.with_suffix(".bmp")
        suffix = ".bmp"
    if suffix == ".bmp":
        payload = _encode_bmp(framebuffer, rgb555, cgb_compat_mode)
    elif suffix == ".ppm":
        payload = _encode_ppm(framebuffer, rgb555, cgb_compat_mode)
    else:
        raise KokuraError("unsupported screenshot format; use .bmp or .ppm")
    target.write_bytes(payload)
    return target


def _normalize_video_frame_range(start_frame: int, end_frame: int) -> tuple[int, int]:
    start = int(start_frame)
    end = int(end_frame)
    if start < 1 or end < 1:
        raise KokuraError("video frame numbers are 1-based and must be >= 1")
    if end < start:
        raise KokuraError("video end_frame must be >= start_frame")
    return start, end


def _framebuffer_to_grayscale_indices(framebuffer: bytes) -> bytes:
    max_value = _framebuffer_intensity_max(framebuffer)
    return bytes(_framebuffer_gray_to_u8(value, max_value) for value in framebuffer)


def _gif332_palette() -> bytes:
    palette = bytearray()
    for index in range(256):
        r = ((index >> 5) & 0x07) * 255 // 7
        g = ((index >> 2) & 0x07) * 255 // 7
        b = (index & 0x03) * 255 // 3
        palette.extend((r, g, b))
    return bytes(palette)


def _rgb_to_gif332_indices(rgb: bytes) -> bytes:
    if len(rgb) != 160 * 144 * 3:
        raise KokuraError("video frame had invalid RGB framebuffer length")
    out = bytearray(len(rgb) // 3)
    for base in range(0, len(rgb), 3):
        r = rgb[base]
        g = rgb[base + 1]
        b = rgb[base + 2]
        out[base // 3] = (
            (((r * 7) + 127) // 255) << 5
            | (((g * 7) + 127) // 255) << 2
            | (((b * 3) + 127) // 255)
        )
    return bytes(out)


def _clamp_u8(value: int) -> int:
    return 0 if value < 0 else 255 if value > 255 else value


def _rgb_to_yuv444(rgb: bytes) -> tuple[bytes, bytes, bytes]:
    if len(rgb) != 160 * 144 * 3:
        raise KokuraError("video frame had invalid RGB framebuffer length")
    y_plane = bytearray(len(rgb) // 3)
    u_plane = bytearray(len(rgb) // 3)
    v_plane = bytearray(len(rgb) // 3)
    for base in range(0, len(rgb), 3):
        idx = base // 3
        r = rgb[base]
        g = rgb[base + 1]
        b = rgb[base + 2]
        y = ((77 * r) + (150 * g) + (29 * b)) >> 8
        u = (((-43 * r) - (85 * g) + (128 * b)) >> 8) + 128
        v = (((128 * r) - (107 * g) - (21 * b)) >> 8) + 128
        y_plane[idx] = _clamp_u8(y)
        u_plane[idx] = _clamp_u8(u)
        v_plane[idx] = _clamp_u8(v)
    return bytes(y_plane), bytes(u_plane), bytes(v_plane)


def _gif_pack_subblocks(payload: bytes) -> bytes:
    out = bytearray()
    for offset in range(0, len(payload), 255):
        chunk = payload[offset : offset + 255]
        out.append(len(chunk))
        out.extend(chunk)
    out.append(0)
    return bytes(out)


def _gif_encode_indices(indices: bytes) -> bytes:
    min_code_size = 8
    clear_code = 1 << min_code_size
    end_code = clear_code + 1
    next_code = end_code + 1
    code_size = min_code_size + 1
    first_symbol = True
    packed = bytearray()
    bit_buffer = 0
    bit_count = 0

    def write_code(code: int) -> None:
        nonlocal bit_buffer, bit_count
        bit_buffer |= int(code) << bit_count
        bit_count += code_size
        while bit_count >= 8:
            packed.append(bit_buffer & 0xFF)
            bit_buffer >>= 8
            bit_count -= 8

    write_code(clear_code)
    for value in indices:
        write_code(value)
        if first_symbol:
            first_symbol = False
            continue
        next_code += 1
        if next_code >= 4096:
            write_code(clear_code)
            next_code = end_code + 1
            code_size = min_code_size + 1
            first_symbol = True
            continue
        if next_code >= (1 << code_size) and code_size < 12:
            code_size += 1

    write_code(end_code)
    if bit_count:
        packed.append(bit_buffer & 0xFF)
    return _gif_pack_subblocks(bytes(packed))


def _encode_gif_frames(rgb_frames: Iterable[bytes]) -> bytes:
    width = 160
    height = 144
    fps_num = 59_727
    fps_den = 1_000
    frames = list(rgb_frames)
    if not frames:
        raise KokuraError("video recording captured no frames")

    out = bytearray()
    out.extend(b"GIF89a")
    out.extend(int(width).to_bytes(2, "little"))
    out.extend(int(height).to_bytes(2, "little"))
    out.append(0xF7)
    out.append(0)
    out.append(0)
    out.extend(_gif332_palette())
    out.extend(b"\x21\xFF\x0BNETSCAPE2.0\x03\x01\x00\x00\x00")

    delay_remainder = 0
    for rgb in frames:
        indices = _rgb_to_gif332_indices(rgb)
        delay_remainder += 100 * fps_den
        delay = delay_remainder // fps_num
        delay_remainder %= fps_num
        if delay == 0:
            delay = 1
        out.extend(b"\x21\xF9\x04\x00")
        out.extend(int(delay).to_bytes(2, "little"))
        out.extend(b"\x00\x00")
        out.extend(b"\x2C")
        out.extend((0).to_bytes(2, "little"))
        out.extend((0).to_bytes(2, "little"))
        out.extend(int(width).to_bytes(2, "little"))
        out.extend(int(height).to_bytes(2, "little"))
        out.extend(b"\x00")
        out.append(8)
        out.extend(_gif_encode_indices(indices))

    out.append(0x3B)
    return bytes(out)


def _encode_y4m_frames(rgb_frames: Iterable[bytes]) -> bytes:
    frames = list(rgb_frames)
    if not frames:
        raise KokuraError("video recording captured no frames")
    out = bytearray(b"YUV4MPEG2 W160 H144 F59727:1000 Ip A1:1 C444\n")
    for rgb in frames:
        y_plane, u_plane, v_plane = _rgb_to_yuv444(rgb)
        out.extend(b"FRAME\n")
        out.extend(y_plane)
        out.extend(u_plane)
        out.extend(v_plane)
    return bytes(out)


def _write_video(path: str | _os.PathLike[str], rgb_frames: Iterable[bytes]) -> Path:
    target = Path(path)
    suffix = target.suffix.lower()
    if not suffix:
        target = target.with_suffix(".gif")
        suffix = ".gif"
    if suffix == ".gif":
        payload = _encode_gif_frames(rgb_frames)
    elif suffix == ".y4m":
        payload = _encode_y4m_frames(rgb_frames)
    else:
        raise KokuraError("unsupported video format; use .gif or .y4m")
    target.write_bytes(payload)
    return target


class KokuraLibrary:
    def __init__(self, path: Optional[str | _os.PathLike[str]] = None):
        self.lib = load_library(path)
        self._bind()

    def _bind(self) -> None:
        lib = self.lib
        void_p = _ct.c_void_p
        c_char_p = _ct.c_char_p

        def _bind_optional(
            name: str,
            argtypes: list[Any],
            restype: Any,
        ) -> bool:
            try:
                fn = getattr(lib, name)
            except AttributeError:
                return False
            fn.argtypes = argtypes
            fn.restype = restype
            return True

        lib.kokura_string_free.argtypes = [void_p]
        lib.kokura_string_free.restype = None
        lib.kokura_version_string.argtypes = []
        lib.kokura_version_string.restype = void_p

        lib.kokura_core_create.argtypes = []
        lib.kokura_core_create.restype = void_p
        lib.kokura_core_destroy.argtypes = [void_p]
        lib.kokura_core_destroy.restype = None
        lib.kokura_core_load_rom.argtypes = [void_p, void_p, _ct.c_size_t]
        lib.kokura_core_load_rom.restype = _ct.c_bool
        lib.kokura_core_step.argtypes = [void_p, _ct.POINTER(_CStepResult)]
        lib.kokura_core_step.restype = _ct.c_bool
        lib.kokura_core_run_frame.argtypes = [void_p]
        lib.kokura_core_run_frame.restype = _ct.c_bool
        lib.kokura_core_run_frames.argtypes = [void_p, _ct.c_uint64]
        lib.kokura_core_run_frames.restype = _ct.c_bool
        lib.kokura_core_set_joypad_mask.argtypes = [void_p, _ct.c_uint8]
        lib.kokura_core_set_joypad_mask.restype = _ct.c_bool
        lib.kokura_core_framebuffer_ptr.argtypes = [void_p]
        lib.kokura_core_framebuffer_ptr.restype = void_p
        lib.kokura_core_framebuffer_len.argtypes = [void_p]
        lib.kokura_core_framebuffer_len.restype = _ct.c_size_t
        self.has_core_color_framebuffer_api = (
            _bind_optional("kokura_core_framebuffer_rgb555_ptr", [void_p], void_p)
            and _bind_optional(
                "kokura_core_framebuffer_rgb555_len", [void_p], _ct.c_size_t
            )
        )
        self.has_core_cgb_compat_api = _bind_optional(
            "kokura_core_is_cgb_compat_mode", [void_p], _ct.c_bool
        )
        lib.kokura_core_current_rom_bank.argtypes = [void_p]
        lib.kokura_core_current_rom_bank.restype = _ct.c_uint16
        lib.kokura_core_current_ram_bank.argtypes = [void_p]
        lib.kokura_core_current_ram_bank.restype = _ct.c_uint16
        lib.kokura_core_cpu_snapshot.argtypes = [void_p, _ct.POINTER(_CCpuSnapshot)]
        lib.kokura_core_cpu_snapshot.restype = _ct.c_bool
        lib.kokura_core_peek8.argtypes = [void_p, _ct.c_uint16]
        lib.kokura_core_peek8.restype = _ct.c_uint8
        lib.kokura_core_read8.argtypes = [void_p, _ct.c_uint16]
        lib.kokura_core_read8.restype = _ct.c_uint8
        lib.kokura_core_write8.argtypes = [void_p, _ct.c_uint16, _ct.c_uint8]
        lib.kokura_core_write8.restype = _ct.c_bool
        lib.kokura_core_save_state_json.argtypes = [void_p]
        lib.kokura_core_save_state_json.restype = void_p
        lib.kokura_core_load_state_json.argtypes = [void_p, c_char_p]
        lib.kokura_core_load_state_json.restype = _ct.c_bool
        lib.kokura_core_audio_sample_rate.argtypes = [void_p]
        lib.kokura_core_audio_sample_rate.restype = _ct.c_uint32
        lib.kokura_core_audio_frames_available.argtypes = [void_p]
        lib.kokura_core_audio_frames_available.restype = _ct.c_size_t
        lib.kokura_core_audio_frames_dropped.argtypes = [void_p]
        lib.kokura_core_audio_frames_dropped.restype = _ct.c_uint64
        lib.kokura_core_audio_buffer_capacity_frames.argtypes = [void_p]
        lib.kokura_core_audio_buffer_capacity_frames.restype = _ct.c_size_t
        lib.kokura_core_set_audio_buffer_capacity_frames.argtypes = [void_p, _ct.c_size_t]
        lib.kokura_core_set_audio_buffer_capacity_frames.restype = _ct.c_bool
        lib.kokura_core_audio_copy_interleaved_i16.argtypes = [void_p, _ct.POINTER(_ct.c_int16), _ct.c_size_t]
        lib.kokura_core_audio_copy_interleaved_i16.restype = _ct.c_size_t

        lib.kokura_debug_session_create.argtypes = []
        lib.kokura_debug_session_create.restype = void_p
        lib.kokura_debug_session_destroy.argtypes = [void_p]
        lib.kokura_debug_session_destroy.restype = None
        lib.kokura_debug_session_load_rom.argtypes = [void_p, void_p, _ct.c_size_t]
        lib.kokura_debug_session_load_rom.restype = _ct.c_bool
        lib.kokura_debug_session_load_symbol_map_text.argtypes = [void_p, c_char_p]
        lib.kokura_debug_session_load_symbol_map_text.restype = _ct.c_bool
        lib.kokura_debug_session_load_source_map_text.argtypes = [void_p, c_char_p]
        lib.kokura_debug_session_load_source_map_text.restype = _ct.c_bool
        lib.kokura_debug_session_load_symbol_table_json.argtypes = [void_p, c_char_p]
        lib.kokura_debug_session_load_symbol_table_json.restype = _ct.c_bool
        lib.kokura_debug_session_clear_symbol_table.argtypes = [void_p]
        lib.kokura_debug_session_clear_symbol_table.restype = _ct.c_bool
        lib.kokura_debug_session_set_stop_conditions_json.argtypes = [void_p, c_char_p]
        lib.kokura_debug_session_set_stop_conditions_json.restype = _ct.c_bool
        lib.kokura_debug_session_clear_stop_conditions.argtypes = [void_p]
        lib.kokura_debug_session_clear_stop_conditions.restype = _ct.c_bool
        lib.kokura_debug_session_set_replay_control_json.argtypes = [void_p, c_char_p]
        lib.kokura_debug_session_set_replay_control_json.restype = _ct.c_bool
        lib.kokura_debug_session_clear_replay_control.argtypes = [void_p]
        lib.kokura_debug_session_clear_replay_control.restype = _ct.c_bool
        lib.kokura_debug_session_set_joypad_mask.argtypes = [void_p, _ct.c_uint8]
        lib.kokura_debug_session_set_joypad_mask.restype = _ct.c_bool
        lib.kokura_debug_session_set_callback.argtypes = [void_p, _DEBUG_CALLBACK, void_p]
        lib.kokura_debug_session_set_callback.restype = _ct.c_bool
        lib.kokura_debug_session_clear_callback.argtypes = [void_p]
        lib.kokura_debug_session_clear_callback.restype = _ct.c_bool
        lib.kokura_debug_session_set_callback_config_json.argtypes = [void_p, c_char_p]
        lib.kokura_debug_session_set_callback_config_json.restype = _ct.c_bool
        lib.kokura_debug_session_clear_callback_config.argtypes = [void_p]
        lib.kokura_debug_session_clear_callback_config.restype = _ct.c_bool
        lib.kokura_debug_session_run_frames.argtypes = [void_p, _ct.c_uint64, _ct.POINTER(_CDebugRunResult)]
        lib.kokura_debug_session_run_frames.restype = _ct.c_bool
        lib.kokura_debug_session_rewind_frames.argtypes = [void_p, _ct.c_uint64]
        lib.kokura_debug_session_rewind_frames.restype = _ct.c_bool
        lib.kokura_debug_session_report_json.argtypes = [void_p]
        lib.kokura_debug_session_report_json.restype = void_p
        lib.kokura_debug_session_save_state_json.argtypes = [void_p]
        lib.kokura_debug_session_save_state_json.restype = void_p
        lib.kokura_debug_session_load_state_json.argtypes = [void_p, c_char_p]
        lib.kokura_debug_session_load_state_json.restype = _ct.c_bool
        lib.kokura_debug_session_stop_reason_json.argtypes = [void_p]
        lib.kokura_debug_session_stop_reason_json.restype = void_p
        lib.kokura_debug_session_last_error.argtypes = [void_p]
        lib.kokura_debug_session_last_error.restype = void_p
        lib.kokura_debug_session_clear_last_error.argtypes = [void_p]
        lib.kokura_debug_session_clear_last_error.restype = _ct.c_bool
        lib.kokura_debug_session_framebuffer_ptr.argtypes = [void_p]
        lib.kokura_debug_session_framebuffer_ptr.restype = void_p
        lib.kokura_debug_session_framebuffer_len.argtypes = [void_p]
        lib.kokura_debug_session_framebuffer_len.restype = _ct.c_size_t
        self.has_debug_color_framebuffer_api = (
            _bind_optional(
                "kokura_debug_session_framebuffer_rgb555_ptr", [void_p], void_p
            )
            and _bind_optional(
                "kokura_debug_session_framebuffer_rgb555_len",
                [void_p],
                _ct.c_size_t,
            )
        )
        self.has_debug_cgb_compat_api = _bind_optional(
            "kokura_debug_session_is_cgb_compat_mode", [void_p], _ct.c_bool
        )
        lib.kokura_debug_session_cpu_snapshot.argtypes = [void_p, _ct.POINTER(_CCpuSnapshot)]
        lib.kokura_debug_session_cpu_snapshot.restype = _ct.c_bool
        lib.kokura_debug_session_peek8.argtypes = [void_p, _ct.c_uint16]
        lib.kokura_debug_session_peek8.restype = _ct.c_uint8
        lib.kokura_debug_session_read8.argtypes = [void_p, _ct.c_uint16]
        lib.kokura_debug_session_read8.restype = _ct.c_uint8
        lib.kokura_debug_session_write8.argtypes = [void_p, _ct.c_uint16, _ct.c_uint8]
        lib.kokura_debug_session_write8.restype = _ct.c_bool
        lib.kokura_debug_session_audio_sample_rate.argtypes = [void_p]
        lib.kokura_debug_session_audio_sample_rate.restype = _ct.c_uint32
        lib.kokura_debug_session_audio_frames_available.argtypes = [void_p]
        lib.kokura_debug_session_audio_frames_available.restype = _ct.c_size_t
        lib.kokura_debug_session_audio_frames_dropped.argtypes = [void_p]
        lib.kokura_debug_session_audio_frames_dropped.restype = _ct.c_uint64
        lib.kokura_debug_session_audio_buffer_capacity_frames.argtypes = [void_p]
        lib.kokura_debug_session_audio_buffer_capacity_frames.restype = _ct.c_size_t
        lib.kokura_debug_session_set_audio_buffer_capacity_frames.argtypes = [void_p, _ct.c_size_t]
        lib.kokura_debug_session_set_audio_buffer_capacity_frames.restype = _ct.c_bool
        lib.kokura_debug_session_audio_copy_interleaved_i16.argtypes = [void_p, _ct.POINTER(_ct.c_int16), _ct.c_size_t]
        lib.kokura_debug_session_audio_copy_interleaved_i16.restype = _ct.c_size_t

    def take_string(self, raw_ptr: int) -> Optional[str]:
        if not raw_ptr:
            return None
        try:
            return _ct.cast(raw_ptr, _ct.c_char_p).value.decode("utf-8")
        finally:
            self.lib.kokura_string_free(raw_ptr)

    def version(self) -> str:
        return self.take_string(self.lib.kokura_version_string()) or ""


class _BaseHandle:
    def __init__(self, kokura: KokuraLibrary):
        self.kokura = kokura
        self.handle: Optional[int] = None

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()

    def __del__(self) -> None:
        self.close()

    def close(self) -> None:
        raise NotImplementedError

    def _capture_video_frames(
        self,
        *,
        start_frame: int,
        end_frame: int,
        advance_one_frame: Callable[[], None],
        frame_supplier: Callable[[], bytes],
    ) -> list[bytes]:
        start, end = _normalize_video_frame_range(start_frame, end_frame)
        captured: list[bytes] = []
        for frame_index in range(1, end + 1):
            advance_one_frame()
            if frame_index >= start:
                captured.append(frame_supplier())
        return captured


class SpeedRunController:
    """Small Python-side speed controller for headless automation.

    KOKURA Core.run_frames() already runs unthrottled. This helper provides a
    convenient "logical frame" multiplier for scripts that want to switch
    between normal polling and fast-forward polling with plain Python
    conditions.
    """

    def __init__(self, runner: Any, multiplier: int = 1, *, max_multiplier: int = 20):
        self.runner = runner
        self.max_multiplier = max(1, int(max_multiplier))
        self.multiplier = 1
        self.set_multiplier(multiplier)

    def set_multiplier(self, multiplier: int) -> None:
        value = int(multiplier)
        if value < 1:
            value = 1
        if value > self.max_multiplier:
            value = self.max_multiplier
        self.multiplier = value

    def run_frames(self, logical_frames: int) -> None:
        frames = int(logical_frames)
        if frames <= 0:
            return
        self.runner.run_frames(frames * self.multiplier)

    def run_frame(self) -> None:
        self.run_frames(1)


class RewindableSpeedRunController(SpeedRunController):
    """Speed controller with explicit state checkpoints for precise polling.

    This is useful for timed automation: run a large fast-forward chunk, and if
    a watched condition changed inside that chunk, load the checkpoint and
    rerun with a smaller step so the script does not accidentally spend extra
    in-game clock time.
    """

    def checkpoint(self) -> Any:
        if not hasattr(self.runner, "save_state"):
            raise KokuraError("runner does not support save_state")
        return self.runner.save_state()

    def rewind_to(self, checkpoint: Any) -> None:
        if not hasattr(self.runner, "load_state"):
            raise KokuraError("runner does not support load_state")
        self.runner.load_state(checkpoint)

    def run_frames_checked(self, logical_frames: int) -> tuple[int, Any]:
        checkpoint = self.checkpoint()
        frames = int(logical_frames)
        if frames <= 0:
            return 0, checkpoint
        executed = frames * self.multiplier
        self.runner.run_frames(executed)
        return executed, checkpoint


class Core(_BaseHandle):
    def __init__(self, kokura: KokuraLibrary):
        super().__init__(kokura)
        self.handle = kokura.lib.kokura_core_create()
        if not self.handle:
            raise KokuraError("failed to create KOKURA core handle")

    def close(self) -> None:
        handle = self.handle
        if handle:
            self.kokura.lib.kokura_core_destroy(handle)
            self.handle = None

    def load_rom(self, rom: bytes | bytearray | memoryview) -> None:
        buf, length = _buffer_from_bytes(rom)
        ok = self.kokura.lib.kokura_core_load_rom(self.handle, _ct.cast(buf, _ct.c_void_p), length)
        if not ok:
            raise KokuraError("kokura_core_load_rom failed")

    def load_rom_path(self, path: str | _os.PathLike[str]) -> None:
        self.load_rom(Path(path).read_bytes())

    def step(self) -> StepResult:
        out = _CStepResult()
        ok = self.kokura.lib.kokura_core_step(self.handle, _ct.byref(out))
        if not ok:
            raise KokuraError("kokura_core_step failed")
        return StepResult(
            cycles=int(out.cycles),
            frame_completed=bool(out.frame_completed),
            current_rom_bank=int(out.current_rom_bank),
            current_ram_bank=int(out.current_ram_bank),
            is_cgb_mode=bool(out.is_cgb_mode),
            is_double_speed=bool(out.is_double_speed),
            framebuffer_len=int(out.framebuffer_len),
        )

    def run_frame(self) -> None:
        ok = self.kokura.lib.kokura_core_run_frame(self.handle)
        if not ok:
            raise KokuraError("kokura_core_run_frame failed")

    def run_frames(self, frames: int) -> None:
        ok = self.kokura.lib.kokura_core_run_frames(self.handle, int(frames))
        if not ok:
            raise KokuraError("kokura_core_run_frames failed")

    def set_joypad_mask(self, mask: int) -> None:
        ok = self.kokura.lib.kokura_core_set_joypad_mask(self.handle, int(mask) & 0xFF)
        if not ok:
            raise KokuraError("kokura_core_set_joypad_mask failed")

    def registers(self) -> RegisterSnapshot:
        out = _CCpuSnapshot()
        ok = self.kokura.lib.kokura_core_cpu_snapshot(self.handle, _ct.byref(out))
        if not ok:
            raise KokuraError("kokura_core_cpu_snapshot failed")
        return _snapshot_from_c(out)

    def peek8(self, addr: int) -> int:
        return int(self.kokura.lib.kokura_core_peek8(self.handle, int(addr) & 0xFFFF))

    def read8(self, addr: int) -> int:
        return int(self.kokura.lib.kokura_core_read8(self.handle, int(addr) & 0xFFFF))

    def write8(self, addr: int, value: int) -> None:
        ok = self.kokura.lib.kokura_core_write8(
            self.handle, int(addr) & 0xFFFF, int(value) & 0xFF
        )
        if not ok:
            raise KokuraError("kokura_core_write8 failed")

    def read_block(self, addr: int, size: int, *, side_effects: bool = False) -> bytes:
        reader = self.read8 if side_effects else self.peek8
        return bytes(reader(addr + offset) for offset in range(size))

    def write_block(self, addr: int, data: bytes | bytearray | memoryview) -> None:
        for offset, value in enumerate(bytes(data)):
            self.write8(addr + offset, value)

    def framebuffer_bytes(self) -> bytes:
        ptr = self.kokura.lib.kokura_core_framebuffer_ptr(self.handle)
        length = int(self.kokura.lib.kokura_core_framebuffer_len(self.handle))
        if not ptr or length <= 0:
            return b""
        return _ct.string_at(ptr, length)

    def framebuffer_rgb555_bytes(self) -> bytes:
        if not self.kokura.has_core_color_framebuffer_api:
            return b""
        ptr = self.kokura.lib.kokura_core_framebuffer_rgb555_ptr(self.handle)
        length = int(self.kokura.lib.kokura_core_framebuffer_rgb555_len(self.handle))
        if not ptr or length <= 0:
            return b""
        return _ct.string_at(ptr, length * 2)

    def framebuffer_rgb555_words(self) -> tuple[int, ...]:
        raw = self.framebuffer_rgb555_bytes()
        if not raw:
            return ()
        return tuple(value for (value,) in _struct.iter_unpack("<H", raw))

    def is_cgb_compat_mode(self) -> bool:
        if not self.kokura.has_core_cgb_compat_api:
            return False
        return bool(self.kokura.lib.kokura_core_is_cgb_compat_mode(self.handle))

    def framebuffer_rgb_bytes(self) -> bytes:
        return _rgb_from_framebuffer(
            self.framebuffer_bytes(),
            self.framebuffer_rgb555_bytes() or None,
            self.is_cgb_compat_mode(),
        )

    def save_screenshot(self, path: str | _os.PathLike[str]) -> Path:
        framebuffer = self.framebuffer_bytes()
        return _write_screenshot(
            path,
            framebuffer,
            self.framebuffer_rgb555_bytes() or None,
            self.is_cgb_compat_mode(),
        )

    def save_video(
        self,
        path: str | _os.PathLike[str],
        *,
        start_frame: int = 1,
        end_frame: int,
    ) -> Path:
        frames = self._capture_video_frames(
            start_frame=start_frame,
            end_frame=end_frame,
            advance_one_frame=self.run_frame,
            frame_supplier=self.framebuffer_rgb_bytes,
        )
        return _write_video(path, frames)

    def current_rom_bank(self) -> int:
        return int(self.kokura.lib.kokura_core_current_rom_bank(self.handle))

    def current_ram_bank(self) -> int:
        return int(self.kokura.lib.kokura_core_current_ram_bank(self.handle))

    def save_state(self) -> dict[str, Any]:
        raw = self.kokura.lib.kokura_core_save_state_json(self.handle)
        text = self.kokura.take_string(raw)
        if text is None:
            raise KokuraError("kokura_core_save_state_json returned null")
        return _json.loads(text)

    def load_state(self, state: dict[str, Any]) -> None:
        ok = self.kokura.lib.kokura_core_load_state_json(self.handle, _json_arg(state))
        if not ok:
            raise KokuraError("kokura_core_load_state_json failed")

    def audio_sample_rate(self) -> int:
        return int(self.kokura.lib.kokura_core_audio_sample_rate(self.handle))

    def audio_frames_available(self) -> int:
        return int(self.kokura.lib.kokura_core_audio_frames_available(self.handle))

    def audio_frames_dropped(self) -> int:
        return int(self.kokura.lib.kokura_core_audio_frames_dropped(self.handle))

    def audio_buffer_capacity_frames(self) -> int:
        return int(self.kokura.lib.kokura_core_audio_buffer_capacity_frames(self.handle))

    def set_audio_buffer_capacity_frames(self, capacity_frames: int) -> None:
        ok = self.kokura.lib.kokura_core_set_audio_buffer_capacity_frames(
            self.handle, int(capacity_frames)
        )
        if not ok:
            raise KokuraError("kokura_core_set_audio_buffer_capacity_frames failed")

    def drain_audio_frames(self, max_frames: Optional[int] = None) -> list[tuple[int, int]]:
        frames_to_copy = self.audio_frames_available() if max_frames is None else int(max_frames)
        if frames_to_copy <= 0:
            return []
        sample_count = frames_to_copy * 2
        buf = (_ct.c_int16 * sample_count)()
        copied = int(
            self.kokura.lib.kokura_core_audio_copy_interleaved_i16(
                self.handle, buf, frames_to_copy
            )
        )
        return [(int(buf[index * 2]), int(buf[index * 2 + 1])) for index in range(copied)]


class DebugSession(_BaseHandle):
    def __init__(self, kokura: KokuraLibrary):
        super().__init__(kokura)
        self.handle = kokura.lib.kokura_debug_session_create()
        if not self.handle:
            raise KokuraError("failed to create KOKURA debug session")
        self._callback_ref: Optional[_DEBUG_CALLBACK] = None
        self._callback_error: Optional[BaseException] = None

    def close(self) -> None:
        handle = self.handle
        if handle:
            self.kokura.lib.kokura_debug_session_destroy(handle)
            self.handle = None
        self._callback_ref = None
        self._callback_error = None

    def _raise_last_error(self, fallback: str) -> None:
        text = self.kokura.take_string(
            self.kokura.lib.kokura_debug_session_last_error(self.handle)
        )
        raise KokuraError(text or fallback)

    def _raise_pending_callback_error(self) -> None:
        error = self._callback_error
        if error is not None:
            self._callback_error = None
            raise KokuraError(f"python callback failed: {error!r}")

    def load_rom(self, rom: bytes | bytearray | memoryview) -> None:
        buf, length = _buffer_from_bytes(rom)
        ok = self.kokura.lib.kokura_debug_session_load_rom(
            self.handle, _ct.cast(buf, _ct.c_void_p), length
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_load_rom failed")

    def load_rom_path(self, path: str | _os.PathLike[str]) -> None:
        self.load_rom(Path(path).read_bytes())

    def load_symbol_map_text(self, text: str) -> None:
        ok = self.kokura.lib.kokura_debug_session_load_symbol_map_text(
            self.handle, _string_arg(text)
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_load_symbol_map_text failed")

    def load_symbol_map_path(self, path: str | _os.PathLike[str]) -> None:
        self.load_symbol_map_text(Path(path).read_text(encoding="utf-8"))

    def load_source_map_text(self, text: str) -> None:
        ok = self.kokura.lib.kokura_debug_session_load_source_map_text(
            self.handle, _string_arg(text)
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_load_source_map_text failed")

    def load_source_map_path(self, path: str | _os.PathLike[str]) -> None:
        self.load_source_map_text(Path(path).read_text(encoding="utf-8"))

    def load_symbol_table(self, value: dict[str, Any]) -> None:
        ok = self.kokura.lib.kokura_debug_session_load_symbol_table_json(
            self.handle, _json_arg(value)
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_load_symbol_table_json failed")

    def clear_symbol_table(self) -> None:
        ok = self.kokura.lib.kokura_debug_session_clear_symbol_table(self.handle)
        if not ok:
            self._raise_last_error("kokura_debug_session_clear_symbol_table failed")

    def set_stop_conditions(self, value: dict[str, Any]) -> None:
        ok = self.kokura.lib.kokura_debug_session_set_stop_conditions_json(
            self.handle, _json_arg(value)
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_set_stop_conditions_json failed")

    def clear_stop_conditions(self) -> None:
        ok = self.kokura.lib.kokura_debug_session_clear_stop_conditions(self.handle)
        if not ok:
            self._raise_last_error("kokura_debug_session_clear_stop_conditions failed")

    def set_replay_control(self, value: dict[str, Any]) -> None:
        ok = self.kokura.lib.kokura_debug_session_set_replay_control_json(
            self.handle, _json_arg(value)
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_set_replay_control_json failed")

    def clear_replay_control(self) -> None:
        ok = self.kokura.lib.kokura_debug_session_clear_replay_control(self.handle)
        if not ok:
            self._raise_last_error("kokura_debug_session_clear_replay_control failed")

    def set_joypad_mask(self, mask: int) -> None:
        ok = self.kokura.lib.kokura_debug_session_set_joypad_mask(
            self.handle, int(mask) & 0xFF
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_set_joypad_mask failed")

    def set_callback_config(
        self,
        *,
        frame_interval: Optional[int] = None,
        event_types: Optional[Iterable[str]] = None,
        emit_stop: bool = True,
    ) -> None:
        payload = {
            "emit_stop": bool(emit_stop),
            "frame_interval": None if frame_interval is None else int(frame_interval),
            "event_types": list(event_types or []),
        }
        ok = self.kokura.lib.kokura_debug_session_set_callback_config_json(
            self.handle, _json_arg(payload)
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_set_callback_config_json failed")

    def clear_callback_config(self) -> None:
        ok = self.kokura.lib.kokura_debug_session_clear_callback_config(self.handle)
        if not ok:
            self._raise_last_error("kokura_debug_session_clear_callback_config failed")

    def set_callback(
        self,
        callback: Callable[[str, Any], None],
        *,
        frame_interval: Optional[int] = None,
        event_types: Optional[Iterable[str]] = None,
        emit_stop: bool = True,
    ) -> None:
        self.set_callback_config(
            frame_interval=frame_interval,
            event_types=event_types,
            emit_stop=emit_stop,
        )

        def _dispatch(kind: bytes, payload_json: bytes, _user_data: int) -> None:
            if self._callback_error is not None:
                return
            try:
                kind_text = kind.decode("utf-8") if kind else ""
                payload = _json.loads(payload_json.decode("utf-8")) if payload_json else None
                callback(kind_text, payload)
            except BaseException as exc:
                self._callback_error = exc

        callback_ref = _DEBUG_CALLBACK(_dispatch)
        ok = self.kokura.lib.kokura_debug_session_set_callback(
            self.handle, callback_ref, None
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_set_callback failed")
        self._callback_ref = callback_ref
        self._callback_error = None

    def clear_callback(self) -> None:
        ok = self.kokura.lib.kokura_debug_session_clear_callback(self.handle)
        if not ok:
            self._raise_last_error("kokura_debug_session_clear_callback failed")
        self._callback_ref = None
        self._callback_error = None

    def run_frames(self, frames: int) -> DebugRunResult:
        out = _CDebugRunResult()
        ok = self.kokura.lib.kokura_debug_session_run_frames(
            self.handle, int(frames), _ct.byref(out)
        )
        self._raise_pending_callback_error()
        if not ok:
            self._raise_last_error("kokura_debug_session_run_frames failed")
        return DebugRunResult(
            frames_requested=int(out.frames_requested),
            frames_executed=int(out.frames_executed),
            stopped_by_debugger=bool(out.stopped_by_debugger),
            has_stop_reason=bool(out.has_stop_reason),
            halted_on_unsupported_opcode=bool(out.halted_on_unsupported_opcode),
        )

    def rewind_frames(self, frames_back: int) -> None:
        ok = self.kokura.lib.kokura_debug_session_rewind_frames(
            self.handle, int(frames_back)
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_rewind_frames failed")

    def report(self) -> dict[str, Any]:
        text = self.kokura.take_string(
            self.kokura.lib.kokura_debug_session_report_json(self.handle)
        )
        if text is None:
            self._raise_last_error("kokura_debug_session_report_json returned null")
        return _json.loads(text)

    def save_state(self) -> dict[str, Any]:
        text = self.kokura.take_string(
            self.kokura.lib.kokura_debug_session_save_state_json(self.handle)
        )
        if text is None:
            self._raise_last_error("kokura_debug_session_save_state_json returned null")
        return _json.loads(text)

    def load_state(self, state: dict[str, Any]) -> None:
        ok = self.kokura.lib.kokura_debug_session_load_state_json(
            self.handle, _json_arg(state)
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_load_state_json failed")

    def stop_reason(self) -> Optional[dict[str, Any]]:
        text = self.kokura.take_string(
            self.kokura.lib.kokura_debug_session_stop_reason_json(self.handle)
        )
        if text is None:
            return None
        return _json.loads(text)

    def last_error(self) -> Optional[str]:
        return self.kokura.take_string(
            self.kokura.lib.kokura_debug_session_last_error(self.handle)
        )

    def clear_last_error(self) -> None:
        ok = self.kokura.lib.kokura_debug_session_clear_last_error(self.handle)
        if not ok:
            raise KokuraError("kokura_debug_session_clear_last_error failed")

    def registers(self) -> RegisterSnapshot:
        out = _CCpuSnapshot()
        ok = self.kokura.lib.kokura_debug_session_cpu_snapshot(
            self.handle, _ct.byref(out)
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_cpu_snapshot failed")
        return _snapshot_from_c(out)

    def peek8(self, addr: int) -> int:
        return int(
            self.kokura.lib.kokura_debug_session_peek8(self.handle, int(addr) & 0xFFFF)
        )

    def read8(self, addr: int) -> int:
        return int(
            self.kokura.lib.kokura_debug_session_read8(self.handle, int(addr) & 0xFFFF)
        )

    def write8(self, addr: int, value: int) -> None:
        ok = self.kokura.lib.kokura_debug_session_write8(
            self.handle, int(addr) & 0xFFFF, int(value) & 0xFF
        )
        if not ok:
            self._raise_last_error("kokura_debug_session_write8 failed")

    def read_block(self, addr: int, size: int, *, side_effects: bool = False) -> bytes:
        reader = self.read8 if side_effects else self.peek8
        return bytes(reader(addr + offset) for offset in range(size))

    def write_block(self, addr: int, data: bytes | bytearray | memoryview) -> None:
        for offset, value in enumerate(bytes(data)):
            self.write8(addr + offset, value)

    def framebuffer_bytes(self) -> bytes:
        ptr = self.kokura.lib.kokura_debug_session_framebuffer_ptr(self.handle)
        length = int(self.kokura.lib.kokura_debug_session_framebuffer_len(self.handle))
        if not ptr or length <= 0:
            return b""
        return _ct.string_at(ptr, length)

    def framebuffer_rgb555_bytes(self) -> bytes:
        if not self.kokura.has_debug_color_framebuffer_api:
            return b""
        ptr = self.kokura.lib.kokura_debug_session_framebuffer_rgb555_ptr(self.handle)
        length = int(
            self.kokura.lib.kokura_debug_session_framebuffer_rgb555_len(self.handle)
        )
        if not ptr or length <= 0:
            return b""
        return _ct.string_at(ptr, length * 2)

    def framebuffer_rgb555_words(self) -> tuple[int, ...]:
        raw = self.framebuffer_rgb555_bytes()
        if not raw:
            return ()
        return tuple(value for (value,) in _struct.iter_unpack("<H", raw))

    def is_cgb_compat_mode(self) -> bool:
        if not self.kokura.has_debug_cgb_compat_api:
            return False
        return bool(
            self.kokura.lib.kokura_debug_session_is_cgb_compat_mode(self.handle)
        )

    def framebuffer_rgb_bytes(self) -> bytes:
        return _rgb_from_framebuffer(
            self.framebuffer_bytes(),
            self.framebuffer_rgb555_bytes() or None,
            self.is_cgb_compat_mode(),
        )

    def save_screenshot(self, path: str | _os.PathLike[str]) -> Path:
        framebuffer = self.framebuffer_bytes()
        return _write_screenshot(
            path,
            framebuffer,
            self.framebuffer_rgb555_bytes() or None,
            self.is_cgb_compat_mode(),
        )

    def save_video(
        self,
        path: str | _os.PathLike[str],
        *,
        start_frame: int = 1,
        end_frame: int,
    ) -> Path:
        frames = self._capture_video_frames(
            start_frame=start_frame,
            end_frame=end_frame,
            advance_one_frame=lambda: self.run_frames(1),
            frame_supplier=self.framebuffer_rgb_bytes,
        )
        return _write_video(path, frames)

    def audio_sample_rate(self) -> int:
        return int(self.kokura.lib.kokura_debug_session_audio_sample_rate(self.handle))

    def audio_frames_available(self) -> int:
        return int(
            self.kokura.lib.kokura_debug_session_audio_frames_available(self.handle)
        )

    def audio_frames_dropped(self) -> int:
        return int(
            self.kokura.lib.kokura_debug_session_audio_frames_dropped(self.handle)
        )

    def audio_buffer_capacity_frames(self) -> int:
        return int(
            self.kokura.lib.kokura_debug_session_audio_buffer_capacity_frames(self.handle)
        )

    def set_audio_buffer_capacity_frames(self, capacity_frames: int) -> None:
        ok = self.kokura.lib.kokura_debug_session_set_audio_buffer_capacity_frames(
            self.handle, int(capacity_frames)
        )
        if not ok:
            self._raise_last_error(
                "kokura_debug_session_set_audio_buffer_capacity_frames failed"
            )

    def drain_audio_frames(self, max_frames: Optional[int] = None) -> list[tuple[int, int]]:
        frames_to_copy = self.audio_frames_available() if max_frames is None else int(max_frames)
        if frames_to_copy <= 0:
            return []
        sample_count = frames_to_copy * 2
        buf = (_ct.c_int16 * sample_count)()
        copied = int(
            self.kokura.lib.kokura_debug_session_audio_copy_interleaved_i16(
                self.handle, buf, frames_to_copy
            )
        )
        return [(int(buf[index * 2]), int(buf[index * 2 + 1])) for index in range(copied)]
