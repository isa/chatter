use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::{OwoColorize, Stream};

/// Create a spinner with elapsed time display per D-07, D-08.
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg} ({elapsed})")
            .expect("valid template")
            .tick_strings(&[
                "\u{280B}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283C}", "\u{2834}",
                "\u{2826}", "\u{2827}", "\u{2807}", "\u{280F}",
            ]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a bounded progress bar for chunk synthesis per D-12.
pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} {msg} ({pos}/{len}) [{bar:30}] ({elapsed})",
        )
        .expect("valid template")
        .progress_chars("=>-"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Finish a spinner, replacing it with a green checkmark and the message.
/// Prints a newline after so subsequent output appears on its own line.
pub fn finish_spinner(pb: &ProgressBar, message: &str) {
    let check = "\u{2714}"
        .if_supports_color(Stream::Stderr, |t| t.green().to_string())
        .to_string();
    pb.finish_and_clear();
    eprintln!("{check} {message}");
}

/// Print error to stderr with colored prefix per D-05, D-06.
pub fn print_error(msg: &str, verbose_detail: Option<&str>, verbose: bool) {
    let prefix = "Error:"
        .if_supports_color(Stream::Stderr, |t| t.red().to_string())
        .to_string();
    eprintln!("{prefix} {msg}");
    if verbose {
        if let Some(detail) = verbose_detail {
            eprintln!("{detail}");
        }
    }
}

/// Print a pass item for doctor output.
pub fn doctor_pass(label: &str, detail: &str) {
    let check = "\u{2713}"
        .if_supports_color(Stream::Stdout, |t| t.green().to_string())
        .to_string();
    println!("  {check} {label}: {detail}");
}

/// Print a fail item for doctor output.
pub fn doctor_fail(label: &str, detail: &str) {
    let cross = "\u{2717}"
        .if_supports_color(Stream::Stdout, |t| t.red().to_string())
        .to_string();
    println!("  {cross} {label}: {detail}");
}

/// Print a warning item for doctor output.
pub fn doctor_warn(label: &str, detail: &str) {
    let warn = "!"
        .if_supports_color(Stream::Stdout, |t| t.yellow().to_string())
        .to_string();
    println!("  {warn} {label}: {detail}");
}

/// A row section in the summary box.
pub struct SummarySection<'a> {
    pub rows: Vec<(&'a str, String, bool)>, // (label, value, is_highlighted)
}

/// Print a bordered summary box that fits within terminal width.
/// Sections are separated by horizontal rules. Highlighted rows get cyan color.
///
/// Layout: `  │ {label:<LW}{value:<VW} │`
/// Total chrome per line: "  │ " (4) + " │" (2) = 6 chars.
/// `inner` = the character count between the two │ chars (including the spaces).
/// Print a bordered summary box that fits within terminal width.
/// Sections are separated by horizontal rules. Highlighted rows get cyan color.
///
/// Uses `console::measure_text_width` for correct unicode width (handles
/// multi-byte chars like ✔ that are 1 display column but >1 byte).
pub fn print_summary_box(title: &str, sections: &[SummarySection<'_>]) {
    use console::measure_text_width;

    let term_width = console::Term::stderr().size().1 as usize;
    // Chrome per line: "  │ " (4 left) + " │" (2 right) = 6 display columns.
    let max_content = term_width.saturating_sub(6);
    let lw: usize = 14;

    let home = std::env::var("HOME").unwrap_or_default();
    let shorten = |s: &str| -> String {
        if !home.is_empty() && s.starts_with(&home) {
            format!("~{}", &s[home.len()..])
        } else {
            s.to_string()
        }
    };

    // Compute content width from longest row, capped at terminal
    let mut cw = measure_text_width(title);
    for section in sections {
        for (_, val, _) in &section.rows {
            cw = cw.max(lw + measure_text_width(&shorten(val)));
        }
    }
    let cw = cw.min(max_content).max(30);
    let vw = cw.saturating_sub(lw);
    let bar = "\u{2500}".repeat(cw + 2);

    // Print a row: pad `content` to `cw` display columns using `visible_width`.
    let row = |content: &str, visible_width: usize| {
        let pad = cw.saturating_sub(visible_width);
        eprintln!("  \u{2502} {}{} \u{2502}", content, " ".repeat(pad));
    };

    eprintln!();
    eprintln!("  \u{256D}{bar}\u{256E}");
    row(title, measure_text_width(title));

    for section in sections {
        eprintln!("  \u{251C}{bar}\u{2524}");
        for (label, val, highlighted) in &section.rows {
            let short = shorten(val);
            let val_width = measure_text_width(&short);
            let display_val = if val_width > vw {
                // Truncate by chars (values are ASCII paths, safe to count chars)
                let limit = vw.saturating_sub(3);
                let truncated: String = short.chars().take(limit).collect();
                format!("{truncated}...")
            } else {
                short
            };
            let display_val_width = measure_text_width(&display_val);
            let label_str = format!("{:<lw$}", format!("{label}:"));
            let vis = lw + display_val_width;
            if *highlighted {
                let colored = display_val
                    .as_str()
                    .if_supports_color(Stream::Stderr, |t| t.cyan().to_string())
                    .to_string();
                row(&format!("{label_str}{colored}"), vis);
            } else {
                row(&format!("{label_str}{display_val}"), vis);
            }
        }
    }
    eprintln!("  \u{2570}{bar}\u{256F}");
    eprintln!();
}
