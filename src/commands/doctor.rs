use owo_colors::{OwoColorize, Stream};
use pyo3::prelude::*;

use crate::bridge;
use crate::bridge::doctor::get_system_info;
use crate::bridge::runtime::ComputeBackend;
use crate::bridge::venv::install_chatterbox_deps;
use crate::cli::{DoctorArgs, GlobalArgs};
use crate::ui;

pub fn run(args: DoctorArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let header = "Chatter Environment Check"
        .if_supports_color(Stream::Stdout, |t| t.bold().to_string())
        .to_string();
    println!("{header}\n");

    let mut passes = 0u32;
    let mut fails = 0u32;
    let mut models_missing = false;
    let mut cb_models_missing = false;

    // Venv
    let diagnosis = bridge::diagnose_venv();
    let venv_ok = matches!(diagnosis, bridge::VenvDiagnosis::Ready);
    match &diagnosis {
        bridge::VenvDiagnosis::Ready => {
            let venv_display = bridge::venv_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            ui::doctor_pass("Venv", &venv_display);
            passes += 1;
        }
        bridge::VenvDiagnosis::NotFound => {
            ui::doctor_fail(
                "Venv",
                "not found — set CHATTER_VENV or reinstall: brew reinstall chatter",
            );
            fails += 1;
        }
        bridge::VenvDiagnosis::InvalidEnvVar { value } => {
            ui::doctor_fail(
                "Venv",
                &format!(
                    "CHATTER_VENV={value} is invalid (missing bin/python) — unset it or fix the path"
                ),
            );
            fails += 1;
        }
        bridge::VenvDiagnosis::NoPython { venv_path } => {
            ui::doctor_fail(
                "Venv",
                &format!(
                    "found at {} but bin/python is missing — recreate venv",
                    venv_path.display()
                ),
            );
            fails += 1;
        }
        bridge::VenvDiagnosis::BridgeMissing { venv_path } => {
            ui::doctor_fail(
                "Venv",
                &format!(
                    "found at {} but chatter_bridge not installed — run any chatter command to auto-install",
                    venv_path.display()
                ),
            );
            fails += 1;
        }
    }

    // Only run Python-dependent checks if venv is configured
    if venv_ok {
        let spinner = ui::create_spinner("Checking environment");
        let info = get_system_info();
        spinner.finish_and_clear();

        // Python Runtime
        match &info.python_version {
            Some(version) => {
                ui::doctor_pass("Python", version);
                passes += 1;
            }
            None => {
                ui::doctor_fail("Python", "not found");
                fails += 1;
            }
        }

        // Python import sanity (NumPy/SciPy ABI issues, etc.)
        if info.python_imports_ok {
            ui::doctor_pass("Python imports", "ok");
            passes += 1;
        } else {
            let detail = info
                .python_imports_error
                .as_deref()
                .unwrap_or("failed (unknown error)");
            ui::doctor_fail(
                "Python imports",
                &format!(
                    "{detail} — try: brew reinstall chatter (or recreate venv / reinstall numpy+scipy)"
                ),
            );
            fails += 1;
        }

        // Inference package
        match &info.inference_pkg_version {
            Some(version) => {
                ui::doctor_pass(&info.inference_pkg_name, version);
                passes += 1;
            }
            None => {
                ui::doctor_fail(
                    &info.inference_pkg_name,
                    "not installed — pip install qwen-tts (or brew reinstall chatter)",
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
                ui::doctor_pass("GPU", &format!("Apple Silicon via MLX ({mem_gb:.1} GB)"));
                passes += 1;
            }
            Some(ComputeBackend::Mps) => {
                ui::doctor_warn(
                    "GPU",
                    "Apple Silicon via MPS (MLX recommended for better performance)",
                );
                passes += 1;
            }
            Some(ComputeBackend::Cpu) => {
                ui::doctor_fail(
                    "GPU",
                    "no GPU detected — chatter requires Apple Silicon or CUDA GPU",
                );
                fails += 1;
            }
            None => {
                ui::doctor_fail("GPU", "detection failed");
                fails += 1;
            }
        }

        // --- Qwen3-TTS section ---
        println!();
        println!(
            "  {}",
            "Qwen3-TTS"
                .if_supports_color(Stream::Stdout, |t| t.bold().to_string())
                .to_string()
        );

        // Qwen Models
        match bridge::list_cached_models() {
            Ok(models) if !models.is_empty() => {
                let total_bytes: u64 = models.iter().filter_map(|m| m.size_bytes).sum();
                let total_gb = total_bytes as f64 / 1_073_741_824.0;
                ui::doctor_pass(
                    "Qwen Models",
                    &format!("{} downloaded ({total_gb:.1} GB)", models.len()),
                );
                passes += 1;
            }
            _ => {
                ui::doctor_fail(
                    "Qwen Models",
                    "not downloaded — run: chatter model download",
                );
                fails += 1;
                models_missing = true;
            }
        }

        // --- ChatterBox section ---
        println!();
        println!(
            "  {}",
            "ChatterBox"
                .if_supports_color(Stream::Stdout, |t| t.bold().to_string())
                .to_string()
        );

        // ChatterBox package
        if info.chatterbox_installed {
            let version = info
                .chatterbox_pkg_version
                .as_deref()
                .unwrap_or("unknown");
            ui::doctor_pass("chatterbox-tts", version);
            passes += 1;
        } else {
            ui::doctor_warn(
                "chatterbox-tts",
                "not installed (optional — install with: chatter model download --engine chatterbox)",
            );
        }

        // ChatterBox models (only check if chatterbox is installed)
        if info.chatterbox_installed {
            match bridge::list_cached_chatterbox_models() {
                Ok(models) if !models.is_empty() => {
                    let total_bytes: u64 = models.iter().filter_map(|m| m.size_bytes).sum();
                    let total_gb = total_bytes as f64 / 1_073_741_824.0;
                    ui::doctor_pass(
                        "CB Models",
                        &format!("{} downloaded ({total_gb:.1} GB)", models.len()),
                    );
                    passes += 1;
                }
                _ => {
                    ui::doctor_warn(
                        "CB Models",
                        "not downloaded — run: chatter model download --engine chatterbox",
                    );
                    cb_models_missing = true;
                }
            }
        }

        // ChatterBox hardware compatibility note
        if info.chatterbox_installed {
            match &info.backend {
                Some(ComputeBackend::Mlx { .. }) => {
                    ui::doctor_pass("CB Hardware", "MLX (Original + Turbo via mlx-community models)");
                    passes += 1;
                }
                Some(ComputeBackend::Cuda { .. }) => {
                    ui::doctor_pass("CB Hardware", "CUDA (all variants supported)");
                    passes += 1;
                }
                Some(ComputeBackend::Mps) => {
                    ui::doctor_pass("CB Hardware", "MPS (all variants supported)");
                    passes += 1;
                }
                Some(ComputeBackend::Cpu) => {
                    ui::doctor_warn("CB Hardware", "no GPU — ChatterBox requires GPU acceleration");
                }
                None => {
                    ui::doctor_warn("CB Hardware", "could not detect hardware");
                }
            }
        }

        // HF Cache / Disk
        if let Some(path) = &info.hf_cache_path {
            if let Some(size_gb) = info.hf_cache_size_gb {
                if size_gb > 0.01 {
                    println!("    Model cache: {path} ({size_gb:.1} GB)",);
                }
            }
        }

        match info.disk_free_gb {
            Some(free) if free < 10.0 => {
                ui::doctor_warn(
                    "Disk",
                    &format!("{free:.1} GB free — models need 3-7 GB each"),
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
    }

    // Summary
    println!();
    if fails == 0 {
        let msg = format!("All {passes} checks passed! Ready for TTS.");
        let colored_msg = msg
            .if_supports_color(Stream::Stdout, |t| t.green().to_string())
            .to_string();
        println!("{colored_msg}");
    } else {
        let msg = format!("{passes} passed, {fails} failed. Fix the items marked \u{2717} above.");
        let colored_msg = msg
            .if_supports_color(Stream::Stdout, |t| t.red().to_string())
            .to_string();
        println!("{colored_msg}");

        if !args.fix && (models_missing || cb_models_missing) {
            println!();
            if models_missing {
                println!("To download Qwen models:       chatter model download");
            }
            if cb_models_missing {
                println!("To download ChatterBox models:  chatter model download --engine chatterbox");
            }
            println!("To auto-fix all:                chatter doctor --fix");
        }
    }

    // --fix: auto-download models if missing
    if args.fix && venv_ok {
        let spinner = ui::create_spinner("Checking environment");
        let info = get_system_info();
        spinner.finish_and_clear();

        // Fix Qwen models
        if models_missing {
            println!();
            println!("Downloading Qwen models...");
            let spinner = ui::create_spinner("Downloading Qwen3-TTS 1.7B models");
            match bridge::download_model(&bridge::ModelQuantization::EightBit) {
                Ok(()) => {
                    spinner.finish_with_message("Qwen models downloaded");
                }
                Err(e) => {
                    spinner.abandon_with_message("Download failed");
                    return Err(anyhow::anyhow!(e).context("Qwen model download failed"));
                }
            }
        }

        // Fix ChatterBox: install deps + download models
        // ChatterBox is optional, so errors are warnings, not hard failures.
        if cb_models_missing || !info.chatterbox_installed {
            println!();
            if !info.chatterbox_installed {
                println!("Installing ChatterBox (first time setup)...");
            } else {
                println!("Downloading ChatterBox models...");
            }

            // Install ChatterBox deps via curated install pipeline
            let spinner = ui::create_spinner("Installing ChatterBox dependencies");
            match install_chatterbox_deps() {
                Ok(()) => {
                    spinner.finish_with_message("ChatterBox dependencies installed");
                }
                Err(e) => {
                    spinner.abandon_with_message("ChatterBox install failed");
                    let warn = "Warning:"
                        .if_supports_color(Stream::Stdout, |t| t.yellow().to_string())
                        .to_string();
                    println!(
                        "{warn} ChatterBox installation failed (optional): {e}"
                    );
                    println!(
                        "You can install manually: pip install chatterbox-tts"
                    );
                }
            }

            // Always download ChatterBox models after successful install
            let spinner = ui::create_spinner("Downloading ChatterBox models");
            match bridge::model::download_model_chatterbox() {
                Ok(()) => {
                    spinner.finish_with_message("ChatterBox models downloaded");
                }
                Err(e) => {
                    spinner.abandon_with_message("ChatterBox model download failed");
                    let warn = "Warning:"
                        .if_supports_color(Stream::Stdout, |t| t.yellow().to_string())
                        .to_string();
                    println!(
                        "{warn} ChatterBox model download failed (optional): {e}"
                    );
                    println!(
                        "You can download manually: chatter model download --engine chatterbox"
                    );
                }
            }
        }

        println!();
        let msg = "Fix complete."
            .if_supports_color(Stream::Stdout, |t| t.green().to_string())
            .to_string();
        println!("{msg}");
    }

    // Verbose output
    if global.verbose {
        println!("\n--- Verbose Diagnostics ---");
        if venv_ok {
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
    }

    println!();
    Ok(())
}

