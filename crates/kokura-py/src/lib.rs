//! PyO3によるPython向けKOKURAバインディング。
//!
//! PythonからROMのロード、フレーム実行、入力、状態保存、スクリーンショット、
//! 診断用データ取得を行えるようにし、内部のエミュレーションは
//! `kokura-core`へ委譲します。

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

    pub fn step_instruction(&mut self) -> PyResult<String> {
        self.inner.step_instruction().map_err(to_py_err)?;
        self.state_json()
    }

    pub fn step_instructions(&mut self, count: u64) -> PyResult<String> {
        for _ in 0..count {
            self.inner.step_instruction().map_err(to_py_err)?;
        }
        self.state_json()
    }

    pub fn step_frame(&mut self) -> PyResult<()> {
        self.inner.run_frame().map_err(to_py_err)
    }

    pub fn step_frames(&mut self, frames: u64) -> PyResult<()> {
        for _ in 0..frames {
            self.inner.run_frame().map_err(to_py_err)?;
        }
        Ok(())
    }

    #[pyo3(signature = (pc, max_instructions=None))]
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

    pub fn run_until_frame(&mut self, frame: u64) -> PyResult<String> {
        while self.inner.clocks.frames < frame {
            self.inner.run_frame().map_err(to_py_err)?;
        }
        self.state_json()
    }

    #[pyo3(signature = (event_type, max_frames=300))]
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

    pub fn set_input(&mut self, mask: u8) {
        self.inner.set_joypad_mask(mask);
    }

    pub fn read_cpu(&mut self, addr: u16) -> u8 {
        self.inner.read8(addr)
    }

    pub fn peek_cpu(&self, addr: u16) -> u8 {
        self.inner.peek8(addr)
    }

    pub fn write_cpu(&mut self, addr: u16, value: u8) {
        self.inner.write8(addr, value);
    }

    pub fn read_vram(&mut self, addr: u16) -> u8 {
        self.inner.read8(addr)
    }

    pub fn write_vram(&mut self, addr: u16, value: u8) {
        self.inner.write8(addr, value);
    }

    pub fn state_json(&self) -> PyResult<String> {
        let state = self.inner.save_state();
        serde_json::to_string_pretty(&state).map_err(to_py_err)
    }

    pub fn save_snapshot(&self, path: String) -> PyResult<()> {
        let state = self.inner.save_state();
        let json = serde_json::to_string_pretty(&state).map_err(to_py_err)?;
        fs::write(path, json).map_err(to_py_err)
    }

    pub fn save_snapshot_file(&self, path: String) -> PyResult<()> {
        self.save_snapshot(path)
    }

    pub fn load_snapshot(&mut self, path: String) -> PyResult<()> {
        let json = fs::read_to_string(path).map_err(to_py_err)?;
        let state: MachineState = serde_json::from_str(&json).map_err(to_py_err)?;
        self.inner.load_state(&state);
        Ok(())
    }

    pub fn load_snapshot_file(&mut self, path: String) -> PyResult<()> {
        self.load_snapshot(path)
    }

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

    pub fn emit_diagnostics_jsonl(&mut self, path: String) -> PyResult<()> {
        self.inner.set_diagnostic_events_enabled(true);
        let mut out = String::new();
        for event in self.inner.diagnostic_events() {
            out.push_str(&serde_json::to_string(event).map_err(to_py_err)?);
            out.push('\n');
        }
        fs::write(path, out).map_err(to_py_err)
    }

    pub fn poll_diagnostics(&self) -> PyResult<String> {
        serde_json::to_string_pretty(self.inner.diagnostic_events()).map_err(to_py_err)
    }

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
    pub fn frame(&self) -> u64 {
        self.inner.clocks.frames
    }

    #[getter]
    pub fn pc(&self) -> u16 {
        self.inner.cpu.pc
    }

    #[getter]
    pub fn rom_bank(&self) -> u16 {
        self.inner.cartridge.current_rom_bank()
    }
}

#[pyfunction]
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
fn kokura(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Emulator>()?;
    m.add_function(wrap_pyfunction!(pad_mask, m)?)?;
    Ok(())
}

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

fn to_py_err<E: std::fmt::Display>(err: E) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

fn diagnostic_match(
    events: &[kokura_core::diagnostic_events::DiagnosticEvent],
    event_type: &str,
) -> bool {
    event_type.eq_ignore_ascii_case("all") && !events.is_empty()
        || events
            .iter()
            .any(|event| event.event_type.eq_ignore_ascii_case(event_type))
}

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
