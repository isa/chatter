use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMetadata {
    pub profile: ProfileInfo,
    pub audio: AudioInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub profile_type: ProfileType,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_audio: Option<String>,
    pub created: String, // ISO 8601 via chrono
    pub model_variant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProfileType {
    Designed,
    Cloned,
}

impl std::fmt::Display for ProfileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Designed => write!(f, "designed"),
            Self::Cloned => write!(f, "cloned"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInfo {
    pub sample_text: String,
    pub sample_rate: u32,
}

pub mod storage;
