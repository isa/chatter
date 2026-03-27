# CLI Patterns Research: clap 4.x, indicatif, owo-colors

**Researched:** 2026-03-27
**Domain:** Rust CLI development — argument parsing, progress UI, colored output
**Confidence:** HIGH (clap, indicatif core patterns), MEDIUM (owo-colors NO_COLOR details)

---

## 1. Clap 4.x Derive API

### 1.1 Top-level CLI structure with subcommands

```rust
use clap::{Parser, Subcommand, Args, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "chatter",
    version,
    about = "Text-to-speech from the terminal",
    long_about = None,
    propagate_version = true,
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

/// Global flags available to every subcommand
#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Enable verbose output
    #[arg(long, global = true, env = "CHATTER_VERBOSE")]
    pub verbose: bool,

    /// Language for TTS output
    #[arg(long, global = true, value_enum, default_value_t = Language::Auto)]
    pub language: Language,

    /// Model size to use
    #[arg(long, global = true, value_enum, default_value_t = ModelSize::B1_7)]
    pub model_size: ModelSize,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Design a voice profile from a natural language description
    Design(DesignArgs),
    /// Clone a voice from an audio sample
    Clone(CloneArgs),
    /// Generate speech from text or a document
    Generate(GenerateArgs),
    /// Manage voice profiles
    Profiles {
        #[command(subcommand)]
        command: ProfilesCommands,
    },
    /// Manage TTS models
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },
    /// Check environment, dependencies, and GPU
    Doctor,
}
```

### 1.2 Nested subcommands (`model download`, `model list`, `profiles list`)

```rust
#[derive(Subcommand, Debug)]
pub enum ModelCommands {
    /// Download a model variant
    Download {
        #[arg(value_enum, default_value_t = super::ModelSize::B1_7)]
        size: super::ModelSize,
    },
    /// List downloaded models and their disk usage
    List,
    /// Remove a downloaded model
    Remove {
        #[arg(value_enum)]
        size: super::ModelSize,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProfilesCommands {
    /// List all saved voice profiles
    List,
    /// Show details of a voice profile
    Show { name: String },
    /// Delete a voice profile
    Delete {
        name: String,
        #[arg(short = 'y', long)]
        yes: bool,
    },
}
```

### 1.3 Enum validation for `--language` and `--model-size`

```rust
#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum Language {
    Auto, Chinese, English, Japanese, Korean,
    French, German, Spanish, Portuguese, Russian, Italian,
}

#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum ModelSize {
    #[value(name = "0.6b")]
    B0_6,
    #[value(name = "1.7b")]
    B1_7,
}
```

**Key detail:** `#[value(name = "0.6b")]` overrides the CLI-facing string so users type `--model-size 0.6b` instead of `--model-size b0-6`.

### 1.4 Global flags pitfall

`global = true` must be set on each individual arg inside the flattened struct, not on the `#[command(flatten)]` annotation itself.

---

## 2. indicatif 0.18.x Patterns

### 2.1 Spinner with elapsed time

```rust
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub fn create_loading_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg} ({elapsed})")
            .expect("valid template")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}
```

Output: `⠸ Loading Qwen3-TTS 1.7B... (12s)`

### 2.2 Use with blocking PyO3 calls

`enable_steady_tick` spawns a background thread that redraws independently. The spinner animates even while the main thread is blocked on a synchronous PyO3 call.

**Do NOT** use manual tick loops — they won't work because the blocking call prevents the loop from running.

### 2.3 finish methods

| Method | Behavior | Use when |
|--------|----------|----------|
| `finish_with_message(msg)` | Replaces spinner with final message, keeps elapsed | Showing completion status |
| `finish_and_clear()` | Removes the spinner entirely | Spinner was transient noise |
| `abandon_with_message(msg)` | Like finish_with_message but uses "abandoned" style | Errors / early exit |

---

## 3. owo-colors 4.x Patterns

### 3.1 Basic usage

```rust
use owo_colors::OwoColorize;

println!("{}", "Error:".red().bold());
println!("{}", "Success:".green());
println!("{}", "Warning:".yellow());
```

### 3.2 Respecting NO_COLOR

```rust
use owo_colors::{OwoColorize, Stream};

println!(
    "{}",
    "Error".if_supports_color(Stream::Stdout, |text| text.red().bold())
);
```

`if_supports_color` checks NO_COLOR env var, FORCE_COLOR, and TTY detection.

Requires `supports-colors` feature in Cargo.toml:
```toml
owo-colors = { version = "4", features = ["supports-colors"] }
```

---

## 4. Stub Subcommand Pattern

Define the command struct fully (appears in `--help`), handler prints "coming soon":

```rust
pub fn run(_args: CloneArgs) -> anyhow::Result<()> {
    eprintln!(
        "{} Voice cloning is not yet implemented.",
        "Note:".if_supports_color(Stream::Stderr, |t| t.yellow().bold())
    );
    eprintln!("This feature is planned for Phase 2.");
    Ok(())
}
```

---

## 5. Cargo.toml for Phase 1

```toml
[dependencies]
clap = { version = "4.5", features = ["derive", "env"] }
indicatif = "0.18.4"
owo-colors = { version = "4", features = ["supports-colors"] }
anyhow = "1"
thiserror = "2"
```

---

## 6. Common Pitfalls

1. **`global = true` placement** — must be on each `#[arg(...)]`, not on `#[command(flatten)]`
2. **Spinner not animating** — use `enable_steady_tick`, not manual tick loops
3. **ValueEnum digit names** — use `#[value(name = "0.6b")]` since Rust identifiers can't start with digits
4. **owo-colors on pipes** — always use `if_supports_color` to avoid ANSI codes in piped output
