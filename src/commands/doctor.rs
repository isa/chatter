use owo_colors::{OwoColorize, Stream};
use pyo3::prelude::*;

use crate::bridge::doctor::get_system_info;
use crate::bridge::runtime::ComputeBackend;
use crate::cli::GlobalArgs;
use crate::ui;

pub fn run(global: &GlobalArgs) -> anyhow::Result<()> {
    // Header
    let header = "Chatter Environment Check"
        .if_supports_color(Stream::Stdout, |t| t.bold().to_string())
        .to_string();
    println!("\n{header}\n");

    let info = get_system_info();
    let mut passes = 0u32;
    let mut fails = 0u32;

    // Python Runtime
    match &info.python_version {
        Some(version) => {
            ui::doctor_pass("Python", version);
            passes += 1;
        }
        None => {
            ui::doctor_fail("Python", "not found \u{2014} install Python 3.12+");
            fails += 1;
        }
    }

    // qwen-tts Package
    match &info.qwen_tts_version {
        Some(version) => {
            ui::doctor_pass("qwen-tts", version);
            passes += 1;
        }
        None => {
            ui::doctor_fail("qwen-tts", "not installed \u{2014} run: pip install qwen-tts");
            fails += 1;
        }
    }

    // PyTorch
    match &info.torch_version {
        Some(version) => {
            ui::doctor_pass("PyTorch", version);
            passes += 1;
        }
        None => {
            ui::doctor_fail(
                "PyTorch",
                "not installed \u{2014} installed automatically with qwen-tts",
            );
            fails += 1;
        }
    }

    // Compute Backend
    match &info.backend {
        Some(ComputeBackend::Cuda { name, vram_bytes }) => {
            let vram_gb = *vram_bytes as f64 / 1e9;
            ui::doctor_pass("GPU", &format!("{name} ({vram_gb:.1} GB VRAM)"));
            passes += 1;
        }
        Some(ComputeBackend::Mlx { memory_bytes }) => {
            let mem_gb = *memory_bytes as f64 / 1e9;
            ui::doctor_pass(
                "GPU",
                &format!("Apple Silicon via MLX ({mem_gb:.1} GB)"),
            );
            passes += 1;
        }
        Some(ComputeBackend::Mps) => {
            ui::doctor_warn(
                "GPU",
                "Apple Silicon via MPS (MLX recommended for better performance)",
            );
            passes += 1; // MPS works, just not optimal
        }
        Some(ComputeBackend::Cpu) => {
            ui::doctor_fail(
                "GPU",
                "no GPU detected \u{2014} chatter requires Apple Silicon or CUDA GPU",
            );
            fails += 1;
        }
        None => {
            ui::doctor_fail("GPU", "detection failed");
            fails += 1;
        }
    }

    // Model Cache / Disk
    if let Some(path) = &info.hf_cache_path {
        if let Some(size_gb) = info.hf_cache_size_gb {
            if size_gb > 0.01 {
                println!(
                    "  {} Model cache: {path} ({size_gb:.1} GB)",
                    " ".if_supports_color(Stream::Stdout, |_t| " ".to_string())
                );
            }
        }
    }

    match info.disk_free_gb {
        Some(free) if free < 10.0 => {
            ui::doctor_warn(
                "Disk",
                &format!("{free:.1} GB free \u{2014} models need 3-7 GB each"),
            );
        }
        Some(free) => {
            ui::doctor_pass("Disk", &format!("{free:.1} GB free"));
            passes += 1;
        }
        None => {
            ui::doctor_warn("Disk", "could not determine free space");
        }
    }

    // Summary
    println!();
    if fails == 0 {
        let msg = "All checks passed! Ready for TTS."
            .if_supports_color(Stream::Stdout, |t| t.green().to_string())
            .to_string();
        println!("{msg}");
    } else {
        let msg = format!("{fails} issue(s) found. Fix the items marked \u{2717} above.");
        let colored_msg = msg
            .if_supports_color(Stream::Stdout, |t| t.red().to_string())
            .to_string();
        println!("{colored_msg}");
    }

    // Verbose output
    if global.verbose {
        println!("\n--- Verbose Diagnostics ---");
        if let Some(path) = &info.hf_cache_path {
            println!("HF cache path: {path}");
        }
        if let Some(backend) = &info.backend {
            println!("Backend details: {backend:?}");
        }
        // Print Python sys.path (first 3 entries)
        pyo3::Python::attach(|py| {
            if let Ok(sys) = py.import("sys") {
                if let Ok(path) = sys.getattr("path") {
                    if let Ok(path_list) = path.extract::<Vec<String>>() {
                        println!("Python sys.path (first 3):");
                        for p in path_list.iter().take(3) {
                            println!("  {p}");
                        }
                    }
                }
            }
        });
    }

    println!();
    Ok(())
}
