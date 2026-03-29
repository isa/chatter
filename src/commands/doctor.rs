use std::collections::HashSet;

use owo_colors::{OwoColorize, Stream};
use pyo3::prelude::*;

use crate::bridge;
use crate::bridge::doctor::get_system_info;
use crate::bridge::model;
use crate::bridge::runtime::ComputeBackend;
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

        // Models — check expected variants for active backend
        match bridge::list_cached_models() {
            Ok(cached_models) => {
                let cached_ids: HashSet<String> =
                    cached_models.iter().map(|m| m.repo_id.clone()).collect();

                if let Some(backend) = &info.backend {
                    let expected = model::model_variants(backend);
                    let mut present: Vec<&String> = Vec::new();
                    let mut missing: Vec<&String> = Vec::new();

                    for variant in &expected {
                        if cached_ids.contains(variant) {
                            present.push(variant);
                        } else {
                            missing.push(variant);
                        }
                    }

                    if missing.is_empty() {
                        // All expected variants are cached
                        let total_bytes: u64 = cached_models
                            .iter()
                            .filter(|m| expected.contains(&m.repo_id))
                            .filter_map(|m| m.size_bytes)
                            .sum();
                        let total_gb = total_bytes as f64 / 1_073_741_824.0;
                        ui::doctor_pass(
                            "Models",
                            &format!(
                                "{}/{} for active backend ({total_gb:.1} GB)",
                                present.len(),
                                expected.len()
                            ),
                        );
                        passes += 1;
                    } else if !present.is_empty() {
                        // Some but not all expected variants cached
                        let missing_names: Vec<String> = missing
                            .iter()
                            .map(|id| {
                                id.rsplit('/')
                                    .next()
                                    .unwrap_or(id)
                                    .to_string()
                            })
                            .collect();
                        ui::doctor_warn(
                            "Models",
                            &format!(
                                "{}/{} for active backend — missing: {}",
                                present.len(),
                                expected.len(),
                                missing_names.join(", ")
                            ),
                        );
                        fails += 1;
                        models_missing = true;
                    } else {
                        // None of the expected variants cached
                        ui::doctor_fail(
                            "Models",
                            "not downloaded — run: chatter model download",
                        );
                        fails += 1;
                        models_missing = true;
                    }

                    // Verbose: per-model status
                    if global.verbose {
                        for variant in &expected {
                            let short_name =
                                variant.rsplit('/').next().unwrap_or(variant);
                            if cached_ids.contains(variant) {
                                println!("      \u{2713} {short_name}");
                            } else {
                                println!("      \u{2717} {short_name}");
                            }
                        }
                    }
                } else {
                    // No backend detected — fall back to counting all cached models
                    if !cached_models.is_empty() {
                        let total_bytes: u64 =
                            cached_models.iter().filter_map(|m| m.size_bytes).sum();
                        let total_gb = total_bytes as f64 / 1_073_741_824.0;
                        ui::doctor_pass(
                            "Models",
                            &format!(
                                "{} downloaded ({total_gb:.1} GB)",
                                cached_models.len()
                            ),
                        );
                        passes += 1;
                    } else {
                        ui::doctor_fail(
                            "Models",
                            "not downloaded — run: chatter model download",
                        );
                        fails += 1;
                        models_missing = true;
                    }
                }
            }
            Err(_) => {
                ui::doctor_fail(
                    "Models",
                    "not downloaded — run: chatter model download",
                );
                fails += 1;
                models_missing = true;
            }
        }

        // HF Cache / Disk
        if let Some(path) = &info.hf_cache_path {
            if let Some(size_gb) = info.hf_cache_size_gb {
                if size_gb > 0.01 {
                    println!(
                        "    Model cache: {path} ({size_gb:.1} GB)",
                    );
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

        if models_missing && !args.fix {
            println!();
            println!("To download models:  chatter model download");
            println!("To auto-fix all:     chatter doctor --fix");
        }
    }

    // --fix: auto-download models if missing
    if args.fix && models_missing && venv_ok {
        println!();
        println!("Downloading models...");
        let spinner = ui::create_spinner("Downloading Qwen3-TTS 1.7B models");
        match bridge::download_model() {
            Ok(()) => {
                spinner.finish_with_message("Models downloaded");
                println!();
                let msg = "Fixed! All models downloaded."
                    .if_supports_color(Stream::Stdout, |t| t.green().to_string())
                    .to_string();
                println!("{msg}");
            }
            Err(e) => {
                spinner.abandon_with_message("Download failed");
                return Err(anyhow::anyhow!(e).context("Model download failed"));
            }
        }
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
