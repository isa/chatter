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

    /// Model size to use
    #[arg(long, global = true, value_enum, default_value_t = ModelSize::B1_7)]
    pub model_size: ModelSize,
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

/// Available model sizes.
#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum ModelSize {
    #[value(name = "0.6b")]
    B0_6,
    #[value(name = "1.7b")]
    B1_7,
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
pub struct GenerateArgs {
    /// Text to synthesize (reads from stdin if omitted)
    pub text: Option<String>,

    /// Input file (PDF, TXT, or Markdown)
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Voice profile to use
    #[arg(short, long)]
    pub profile: String,

    /// Output file path (defaults to output.mp3)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
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

/// Model management subcommands.
#[derive(Subcommand, Debug)]
pub enum ModelCommands {
    /// Download a model variant
    Download {
        /// Model size to download
        #[arg(value_enum, default_value_t = ModelSize::B1_7)]
        size: ModelSize,
    },
    /// List downloaded models and their disk usage
    List,
    /// Remove a downloaded model
    Remove {
        /// Model size to remove
        #[arg(value_enum)]
        size: ModelSize,
    },
}
