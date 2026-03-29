use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Top-level CLI for chatter.
#[derive(Parser, Debug)]
#[command(
    name = "chatter",
    version,
    about = "Text-to-speech from the terminal, powered by Qwen3-TTS",
    long_about = None,
    propagate_version = true
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

/// Global flags available to every subcommand.
#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Enable verbose output
    #[arg(long, global = true, env = "CHATTER_VERBOSE")]
    pub verbose: bool,

    /// Language for TTS output
    #[arg(long, global = true, value_enum, default_value_t = Language::Auto)]
    pub language: Language,
}

/// Supported languages for Qwen3-TTS.
#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum Language {
    Auto,
    Chinese,
    English,
    Japanese,
    Korean,
    French,
    German,
    Spanish,
    Portuguese,
    Russian,
    Italian,
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
    Doctor(DoctorArgs),
}

/// Arguments for the design subcommand.
#[derive(Args, Debug)]
pub struct DesignArgs {
    /// Natural language description of the desired voice
    pub description: String,

    /// Name for the voice profile
    #[arg(long)]
    pub name: Option<String>,
}

/// Arguments for the clone subcommand.
#[derive(Args, Debug)]
pub struct CloneArgs {
    /// Path to the reference audio file
    pub audio_file: PathBuf,

    /// Name for the voice profile
    #[arg(long)]
    pub name: Option<String>,
}

/// Arguments for the generate subcommand.
#[derive(Args, Debug)]
#[command(
    override_usage = "chatter generate [TEXT] --profile <PROFILE> [--file <FILE>] [OPTIONS]\n       Supported file types: PDF, DOCX, TXT, Markdown"
)]
pub struct GenerateArgs {
    /// Text to synthesize (or use --file for documents)
    pub text: Option<String>,

    /// Input file (PDF, DOCX, TXT, or Markdown) — chunks by paragraph, combines with silence gaps
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Voice profile to use
    #[arg(short, long)]
    pub profile: String,

    /// Output file path (defaults to <profile-name>-<timestamp>.mp3)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Skip playing audio after generation
    #[arg(long)]
    pub no_play: bool,

    /// Split output into separate files per chunk (e.g., output-001.mp3)
    #[arg(long)]
    pub split: bool,

    /// Slower, more deliberate speech with longer pauses at punctuation
    #[arg(long)]
    pub slow: bool,

    /// Preprocess text: insert pause markers at punctuation for more natural pacing
    #[arg(long)]
    pub natural_pace: bool,
}

/// Profile management subcommands.
#[derive(Subcommand, Debug)]
pub enum ProfilesCommands {
    /// List all saved voice profiles
    List,
    /// Show details of a voice profile
    Show {
        /// Name of the profile to show
        name: String,
    },
    /// Delete a voice profile
    Delete {
        /// Name of the profile to delete
        name: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

/// Arguments for the doctor subcommand.
#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Auto-fix issues (download models, etc.)
    #[arg(long)]
    pub fix: bool,
}

/// Model management subcommands.
#[derive(Subcommand, Debug)]
pub enum ModelCommands {
    /// Download all 1.7B model variants
    Download,
    /// List downloaded models and their disk usage
    List,
    /// Remove all downloaded 1.7B model variants
    Remove,
}
