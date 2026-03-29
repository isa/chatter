use std::fs;
use std::io::{self, Write};

use owo_colors::{OwoColorize, Stream, Style};

use crate::cli::{GlobalArgs, ProfilesCommands};
use crate::profile::storage;
use crate::ui;

pub fn run(command: ProfilesCommands, global: &GlobalArgs) -> anyhow::Result<()> {
    match command {
        ProfilesCommands::List => run_list(global),
        ProfilesCommands::Show { name } => run_show(&name, global),
        ProfilesCommands::Delete { name, yes } => run_delete(&name, yes, global),
    }
}

/// Display a formatted table of all saved voice profiles.
fn run_list(_global: &GlobalArgs) -> anyhow::Result<()> {
    let profiles = storage::list_profiles()?;

    if profiles.is_empty() {
        println!("No voice profiles found. Create one with `chatter design` or `chatter clone`.");
        return Ok(());
    }

    // Calculate column widths
    let mut name_w = 20usize;
    let mut type_w = 9usize;
    let mut lang_w = 10usize;
    let created_w = 10usize;

    for p in &profiles {
        name_w = name_w.max(p.profile.name.len());
        type_w = type_w.max(p.profile.profile_type.to_string().len());
        lang_w = lang_w.max(p.profile.language.len());
    }

    let header_style = Style::new().bold().underline();
    let name_style = Style::new().cyan();
    let dim_style = Style::new().dimmed();

    // Header
    println!(
        "{:<name_w$}  {:<type_w$}  {:<lang_w$}  {:<created_w$}",
        "Name".if_supports_color(Stream::Stdout, |t| t.style(header_style)),
        "Type".if_supports_color(Stream::Stdout, |t| t.style(header_style)),
        "Language".if_supports_color(Stream::Stdout, |t| t.style(header_style)),
        "Created".if_supports_color(Stream::Stdout, |t| t.style(header_style)),
        name_w = name_w,
        type_w = type_w,
        lang_w = lang_w,
        created_w = created_w,
    );

    // Rows
    for p in &profiles {
        let created_short = format_date(&p.profile.created);
        let name_colored = p.profile.name
            .if_supports_color(Stream::Stdout, |t| t.style(name_style))
            .to_string();
        let type_dim = p.profile.profile_type.to_string()
            .if_supports_color(Stream::Stdout, |t| t.style(dim_style))
            .to_string();
        let lang_dim = p.profile.language
            .if_supports_color(Stream::Stdout, |t| t.style(dim_style))
            .to_string();
        // Pad manually since ANSI codes break format width
        let name_pad = name_w.saturating_sub(p.profile.name.len());
        let type_pad = type_w.saturating_sub(p.profile.profile_type.to_string().len());
        let lang_pad = lang_w.saturating_sub(p.profile.language.len());
        println!(
            "{}{:pad_n$}  {}{:pad_t$}  {}{:pad_l$}  {}",
            name_colored, "", type_dim, "", lang_dim, "", created_short,
            pad_n = name_pad, pad_t = type_pad, pad_l = lang_pad,
        );
    }

    Ok(())
}

/// Display full details of a voice profile.
fn run_show(name: &str, global: &GlobalArgs) -> anyhow::Result<()> {
    let profile = match storage::load_profile(name) {
        Ok(p) => p,
        Err(e) => {
            ui::print_error(
                &format!("Profile '{name}' not found."),
                Some(&format!("{e:#}")),
                global.verbose,
            );
            return Err(e);
        }
    };

    let bold_style = Style::new().bold();

    // Header
    println!(
        "{} {}",
        "Profile:".if_supports_color(Stream::Stdout, |t| t.style(bold_style)),
        profile.profile.name
    );
    println!("Type: {}", profile.profile.profile_type);
    println!("Language: {}", profile.profile.language);

    if let Some(ref desc) = profile.profile.description {
        println!("Description: {desc}");
    }
    if let Some(ref source) = profile.profile.source_audio {
        println!("Source: {source}");
    }

    println!("Model: {}", profile.profile.model_variant);
    println!("Created: {}", profile.profile.created);

    // Files section
    let profile_dir = storage::profile_dir(&profile.profile.name)?;
    println!();
    println!(
        "{}",
        "Files:".if_supports_color(Stream::Stdout, |t| t.style(bold_style))
    );

    let metadata_path = profile_dir.join("profile.toml");
    print_file_info("  Metadata", &metadata_path);

    let sample_path = profile_dir.join("sample.mp3");
    print_file_info("  Sample", &sample_path);

    // Voice data: could be voice_prompt.bin or ref_audio.wav
    let prompt_path = profile_dir.join("voice_prompt.bin");
    let ref_audio_path = profile_dir.join("ref_audio.wav");
    if prompt_path.exists() {
        print_file_info("  Voice data", &prompt_path);
    } else if ref_audio_path.exists() {
        print_file_info("  Voice data", &ref_audio_path);
    }

    Ok(())
}

/// Delete a voice profile with optional confirmation.
fn run_delete(name: &str, yes: bool, global: &GlobalArgs) -> anyhow::Result<()> {
    let profile_dir = storage::profile_dir(name)?;
    if !profile_dir.exists() {
        ui::print_error(
            &format!("Profile '{name}' not found."),
            None,
            global.verbose,
        );
        anyhow::bail!("Profile '{name}' not found");
    }

    if !yes {
        eprint!("Delete profile '{name}'? [y/N] ");
        io::stderr().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }

    fs::remove_dir_all(&profile_dir)?;
    eprintln!("Profile '{name}' deleted.");
    Ok(())
}

/// Format an ISO 8601 date string to YYYY-MM-DD.
fn format_date(iso: &str) -> String {
    // Take first 10 chars (YYYY-MM-DD) from ISO 8601
    if iso.len() >= 10 {
        iso[..10].to_string()
    } else {
        iso.to_string()
    }
}

/// Print file path and size if it exists.
fn print_file_info(label: &str, path: &std::path::Path) {
    if path.exists() {
        let size = fs::metadata(path)
            .map(|m| format_size(m.len()))
            .unwrap_or_else(|_| "?".to_string());
        println!("{label}: {} ({size})", path.display());
    } else {
        let dim_style = Style::new().dimmed();
        println!(
            "{label}: {}",
            "(missing)".if_supports_color(Stream::Stdout, |t| t.style(dim_style))
        );
    }
}

/// Format byte size to human-readable string.
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
