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
