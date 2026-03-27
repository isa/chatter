use std::path::PathBuf;

use directories::ProjectDirs;

use super::ProfileMetadata;

/// Preview sentence for voice profile samples.
pub const PREVIEW_SENTENCE: &str = "Hello, this is a preview of your voice profile.";

/// Get the profiles directory (~/.config/chatter/profiles/)
pub fn profiles_dir() -> anyhow::Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "chatter")
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    Ok(dirs.config_dir().join("profiles"))
}

/// Save profile metadata to TOML and return the profile directory path.
pub fn save_profile(profile: &ProfileMetadata) -> anyhow::Result<PathBuf> {
    let dir = profiles_dir()?.join(&profile.profile.name);
    std::fs::create_dir_all(&dir)?;
    let toml_str = toml::to_string_pretty(profile)?;
    std::fs::write(dir.join("profile.toml"), toml_str)?;
    Ok(dir)
}

/// Load a profile by name.
pub fn load_profile(name: &str) -> anyhow::Result<ProfileMetadata> {
    let dir = profiles_dir()?.join(name);
    let content = std::fs::read_to_string(dir.join("profile.toml"))
        .map_err(|_| anyhow::anyhow!("Profile '{}' not found", name))?;
    let profile: ProfileMetadata = toml::from_str(&content)?;
    Ok(profile)
}

/// List all profile names and metadata.
pub fn list_profiles() -> anyhow::Result<Vec<ProfileMetadata>> {
    let dir = profiles_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut profiles = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let toml_path = entry.path().join("profile.toml");
            if toml_path.exists() {
                let content = std::fs::read_to_string(&toml_path)?;
                if let Ok(p) = toml::from_str::<ProfileMetadata>(&content) {
                    profiles.push(p);
                }
            }
        }
    }
    profiles.sort_by(|a, b| a.profile.name.cmp(&b.profile.name));
    Ok(profiles)
}

/// Slugify a description into a profile name.
/// Takes first `max_words` words, lowercases, keeps only alphanumeric and hyphens.
pub fn slugify(input: &str, max_words: usize) -> String {
    let slug: String = input
        .to_lowercase()
        .split_whitespace()
        .take(max_words)
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    slug.trim_matches('-').to_string()
}

/// Generate a unique profile name, appending -2, -3, etc. if collision.
pub fn unique_profile_name(base: &str) -> anyhow::Result<String> {
    let dir = profiles_dir()?;
    if !dir.join(base).exists() {
        return Ok(base.to_string());
    }
    for i in 2.. {
        let candidate = format!("{base}-{i}");
        if !dir.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    unreachable!()
}

/// Get the profile directory path for a given name.
pub fn profile_dir(name: &str) -> anyhow::Result<PathBuf> {
    Ok(profiles_dir()?.join(name))
}
