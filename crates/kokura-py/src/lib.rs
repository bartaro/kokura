//! Python bindings for KOKURA through PyO3.
//!
//! Expose ROM loading, frame execution, input, saved state, screenshots and
//! diagnostic data to Python while delegating emulation to kokura-core.

use image::{ImageBuffer, Rgb};
use kokura_core::{state::MachineState, types::HardwareMode, Machine};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::fs;
use std::io::Write;
use zip::{write::FileOptions, ZipWriter};

#[pyclass]
pub struct Emulator {
    inner: Machine,
}

#[pymethods]
impl Emulator {
    #[staticmethod]
    #[pyo3(signature = (path, hardware=None))]
    // Read the ROM file, resolve the optional hardware mode and create a fresh machine.
    pub fn from_rom(path: String, hardware: Option<String>) -> PyResult<Self> {
        let rom = fs::read(&path).map_err(to_py_err)?;
        let mode = parse_mode(hardware)?;
        let mut inner = Machine::new();
        match mode {
            Some(mode) => inner
                .load_rom_with_mode(rom, Some(mode))
                .map_err(to_py_err)?,
            None => inner.load_rom(rom).map_err(to_py_err)?,
        }
        Ok(Self { inner })
    }

    // Execute one instruction and return the resulting serialized machine state.
    pub fn step_instruction(&mut self) -> PyResult<String> {
        self.inner.step_instruction().map_err(to_py_err)?;
        self.state_json()
    }

    // Advance up to count instructions; an error leaves any completed steps applied.
    pub fn step_instructions(&mut self, count: u64) -> PyResult<String> {
        for _ in 0..count {
            self.inner.step_instruction().map_err(to_py_err)?;
        }
        self.state_json()
    }

    // Advance through one machine frame, forwarding execution failures to Python.
    pub fn step_frame(&mut self) -> PyResult<()> {
        self.inner.run_frame().map_err(to_py_err)
    }

    // Run frames sequentially; completed frames are retained if a later frame fails.
    pub fn step_frames(&mut self, frames: u64) -> PyResult<()> {
        for _ in 0..frames {
            self.inner.run_frame().map_err(to_py_err)?;
        }
        Ok(())
    }

    #[pyo3(signature = (pc, max_instructions=None))]
    // Check the address before each step, without a ROM-bank condition. The returned
    // state has no separate flag distinguishing a match from instruction-budget exhaustion.
    pub fn run_until_pc(&mut self, pc: u16, max_instructions: Option<u64>) -> PyResult<String> {
        let max = max_instructions.unwrap_or(1_000_000);
        for _ in 0..max {
            if self.inner.cpu.pc == pc {
                return self.state_json();
            }
            self.inner.step_instruction().map_err(to_py_err)?;
        }
        self.state_json()
    }

    // Advance to an absolute machine frame number; an already reached target is a no-op.
    pub fn run_until_frame(&mut self, frame: u64) -> PyResult<String> {
        while self.inner.clocks.frames < frame {
            self.inner.run_frame().map_err(to_py_err)?;
        }
        self.state_json()
    }

    #[pyo3(signature = (event_type, max_frames=300))]
    // Enable event capture and check the entire retained history after each frame.
    // Old matching events can satisfy the request; capture stays enabled and history is not drained.
    pub fn run_until_diagnostic(
        &mut self,
        event_type: String,
        max_frames: u64,
    ) -> PyResult<String> {
        self.inner.set_diagnostic_events_enabled(true);
        for _ in 0..max_frames {
            self.inner.run_frame().map_err(to_py_err)?;
            if diagnostic_match(self.inner.diagnostic_events(), &event_type) {
                return self.poll_diagnostics();
            }
        }
        self.poll_diagnostics()
    }

    // Apply the eight-button mask through the machine joypad input path.
    pub fn set_input(&mut self, mask: u8) {
        self.inner.set_joypad_mask(mask);
    }

    // Read through the CPU bus, including mapped-device read side effects.
    pub fn read_cpu(&mut self, addr: u16) -> u8 {
        self.inner.read8(addr)
    }

    // Observe the mapped byte through the side-effect-free debugger read path.
    pub fn peek_cpu(&self, addr: u16) -> u8 {
        self.inner.peek8(addr)
    }

    // Write through the bus so memory mapping and device register rules still apply.
    pub fn write_cpu(&mut self, addr: u16, value: u8) {
        self.inner.write8(addr, value);
    }

    // This convenience name still performs an unrestricted CPU-bus read. Supply a VRAM
    // address explicitly; the machine controls the active bank and access restrictions.
    pub fn read_vram(&mut self, addr: u16) -> u8 {
        self.inner.read8(addr)
    }

    // This delegates to the CPU bus without checking that addr belongs to VRAM.
    pub fn write_vram(&mut self, addr: u16, value: u8) {
        self.inner.write8(addr, value);
    }

    // Serialize MachineState, including cartridge data, as pretty JSON rather than a KQS file.
    pub fn state_json(&self) -> PyResult<String> {
        let state = self.inner.save_state();
        serde_json::to_string_pretty(&state).map_err(to_py_err)
    }

    // Overwrite the requested file with the same JSON machine-state representation.
    pub fn save_snapshot(&self, path: String) -> PyResult<()> {
        let state = self.inner.save_state();
        let json = serde_json::to_string_pretty(&state).map_err(to_py_err)?;
        fs::write(path, json).map_err(to_py_err)
    }

    // Keep the file-oriented alias on the same JSON save path.
    pub fn save_snapshot_file(&self, path: String) -> PyResult<()> {
        self.save_snapshot(path)
    }

    // Decode JSON before replacing machine state. This wrapper does not validate internal
    // state invariants or compare the embedded cartridge with the currently loaded ROM.
    pub fn load_snapshot(&mut self, path: String) -> PyResult<()> {
        let json = fs::read_to_string(path).map_err(to_py_err)?;
        let state: MachineState = serde_json::from_str(&json).map_err(to_py_err)?;
        self.inner.load_state(&state);
        Ok(())
    }

    // Use the same JSON loader for the explicit file-oriented alias.
    pub fn load_snapshot_file(&mut self, path: String) -> PyResult<()> {
        self.load_snapshot(path)
    }

    // Convert the current image to 160x144 RGB. The image crate selects the enabled
    // encoder from the path extension, so use .png for PNG output.
    pub fn save_png(&self, path: String) -> PyResult<()> {
        let rgb = framebuffer_to_rgb(
            self.inner.framebuffer(),
            Some(self.inner.framebuffer_rgb555()),
            self.inner.is_cgb_compat_mode(),
        );
        let image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_vec(160, 144, rgb)
            .ok_or_else(|| PyRuntimeError::new_err("failed to construct 160x144 RGB image"))?;
        image.save(path).map_err(to_py_err)
    }

    // Enable future capture and overwrite the file with existing events, one JSON value
    // per line. Export neither advances the machine nor clears the retained history.
    pub fn emit_diagnostics_jsonl(&mut self, path: String) -> PyResult<()> {
        self.inner.set_diagnostic_events_enabled(true);
        let mut out = String::new();
        for event in self.inner.diagnostic_events() {
            out.push_str(&serde_json::to_string(event).map_err(to_py_err)?);
            out.push('\n');
        }
        fs::write(path, out).map_err(to_py_err)
    }

    // Return all retained events as JSON without consuming them.
    pub fn poll_diagnostics(&self) -> PyResult<String> {
        serde_json::to_string_pretty(self.inner.diagnostic_events()).map_err(to_py_err)
    }

    // Create a deflated ZIP containing only a manifest and retained diagnostic JSONL.
    // The bundle does not include the ROM, a saved machine state or screenshots.
    pub fn save_repro_bundle(&self, path: String) -> PyResult<()> {
        let file = fs::File::create(&path).map_err(to_py_err)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", options)
            .map_err(to_py_err)?;
        zip.write_all(
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "kitaq-repro-bundle",
                "schema_version": 1,
                "producer": "kokura-py",
                "platform": "gb",
                "diagnostics": "diagnostic_events.jsonl"
            }))
            .map_err(to_py_err)?
            .as_bytes(),
        )
        .map_err(to_py_err)?;
        zip.start_file("diagnostic_events.jsonl", options)
            .map_err(to_py_err)?;
        for event in self.inner.diagnostic_events() {
            zip.write_all(serde_json::to_string(event).map_err(to_py_err)?.as_bytes())
                .map_err(to_py_err)?;
            zip.write_all(b"\n").map_err(to_py_err)?;
        }
        zip.finish().map_err(to_py_err)?;
        Ok(())
    }

    #[getter]
    // Expose the absolute frame counter from the current machine clocks.
    pub fn frame(&self) -> u64 {
        self.inner.clocks.frames
    }

    #[getter]
    // Expose the current 16-bit program counter without advancing execution.
    pub fn pc(&self) -> u16 {
        self.inner.cpu.pc
    }

    #[getter]
    // Return the cartridge controller's current ROM-bank label.
    pub fn rom_bank(&self) -> u16 {
        self.inner.cartridge.current_rom_bank()
    }
}

#[pyfunction]
// Combine case-insensitive button names into one mask; names are not whitespace-trimmed.
pub fn pad_mask(buttons: Vec<String>) -> PyResult<u8> {
    let mut mask = 0u8;
    for button in buttons {
        mask |= match button.to_ascii_lowercase().as_str() {
            "right" => 0x01,
            "left" => 0x02,
            "up" => 0x04,
            "down" => 0x08,
            "a" => 0x10,
            "b" => 0x20,
            "select" => 0x40,
            "start" => 0x80,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown GB-compatible button: {other}"
                )))
            }
        };
    }
    Ok(mask)
}

#[pymodule]
// Register the emulator class and button-mask helper in the Python extension module.
fn kokura(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Emulator>()?;
    m.add_function(wrap_pyfunction!(pad_mask, m)?)?;
    Ok(())
}

// Accept the documented case-insensitive mode aliases; absent or auto leaves detection
// to the machine. Whitespace is significant and unknown names raise ValueError.
fn parse_mode(value: Option<String>) -> PyResult<Option<HardwareMode>> {
    match value.as_deref().map(str::to_ascii_lowercase).as_deref() {
        None | Some("auto") => Ok(None),
        Some("dmg") | Some("gb") => Ok(Some(HardwareMode::Dmg)),
        Some("cgb") | Some("gbc") => Ok(Some(HardwareMode::Cgb)),
        Some(other) => Err(PyValueError::new_err(format!(
            "unknown hardware mode: {other}"
        ))),
    }
}

// Convert a displayed Rust error to a Python RuntimeError at the binding boundary.
fn to_py_err<E: std::fmt::Display>(err: E) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

// Match an event type case-insensitively; all means any nonempty retained history.
fn diagnostic_match(
    events: &[kokura_core::diagnostic_events::DiagnosticEvent],
    event_type: &str,
) -> bool {
    event_type.eq_ignore_ascii_case("all") && !events.is_empty()
        || events
            .iter()
            .any(|event| event.event_type.eq_ignore_ascii_case(event_type))
}

// Prefer the compatibility palette, then a complete RGB555 plane, then four-shade
// grayscale. RGB555 channels are expanded from five bits with integer scaling.
fn framebuffer_to_rgb(
    framebuffer: &[u8; 160 * 144],
    rgb555: Option<&[u16]>,
    cgb_compat_mode: bool,
) -> Vec<u8> {
    if cgb_compat_mode {
        const CGB_COMPAT: [(u8, u8, u8); 4] = [
            (255, 255, 214),
            (181, 230, 115),
            (82, 148, 65),
            (16, 49, 24),
        ];
        let mut rgb = Vec::with_capacity(framebuffer.len() * 3);
        for &shade in framebuffer {
            let (r, g, b) = CGB_COMPAT[usize::from(shade.min(3))];
            rgb.extend_from_slice(&[r, g, b]);
        }
        return rgb;
    }
    if let Some(rgb555) = rgb555.filter(|rgb555| rgb555.len() == 160 * 144) {
        let mut rgb = Vec::with_capacity(rgb555.len() * 3);
        for &value in rgb555 {
            let r = ((value & 0x1F) as u32 * 255 / 31) as u8;
            let g = (((value >> 5) & 0x1F) as u32 * 255 / 31) as u8;
            let b = (((value >> 10) & 0x1F) as u32 * 255 / 31) as u8;
            rgb.extend_from_slice(&[r, g, b]);
        }
        return rgb;
    }
    let mut rgb = Vec::with_capacity(framebuffer.len() * 3);
    for &value in framebuffer {
        let gray = 255u8.saturating_sub(value.min(3) * 85);
        rgb.extend_from_slice(&[gray, gray, gray]);
    }
    rgb
}
