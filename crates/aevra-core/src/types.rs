use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Chat,
    Translate,
    Research,
    Math,
    Code,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Translate => "translate",
            Self::Research => "research",
            Self::Math => "math",
            Self::Code => "code",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComputeDevice {
    #[default]
    Auto,
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub mode: Mode,
    pub updated_at: i64,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub memory_disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: usize,
    pub title: String,
    pub url: String,
    pub excerpt: String,
    pub accessed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
    pub sources: Vec<Source>,
    #[serde(default = "complete_status")]
    pub status: String,
    #[serde(default)]
    pub attachments: Vec<crate::attachments::Attachment>,
}

fn complete_status() -> String {
    "complete".into()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateRequest {
    pub request_id: String,
    pub conversation_id: String,
    pub text: String,
    pub target_language: Option<String>,
    #[serde(default)]
    pub attachment_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettings {
    pub compute_device: ComputeDevice,
    pub theme: String,
    pub default_mode: Mode,
    pub send_with_enter: bool,
    pub font_size: u8,
    pub compact_layout: bool,
    pub reduce_motion: bool,
    pub show_timestamps: bool,
    pub show_suggestions: bool,
    pub completion_sound: bool,
    pub preferred_name: String,
    pub response_style: String,
    pub custom_instructions: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub translation_language: String,
    pub web_research_enabled: bool,
    pub research_source_limit: usize,
    pub memory_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            compute_device: ComputeDevice::Auto,
            theme: "light".into(),
            default_mode: Mode::Chat,
            send_with_enter: true,
            font_size: 14,
            compact_layout: false,
            reduce_motion: false,
            show_timestamps: true,
            show_suggestions: true,
            completion_sound: false,
            preferred_name: String::new(),
            response_style: "balanced".into(),
            custom_instructions: String::new(),
            temperature: 0.7,
            max_tokens: 1024,
            translation_language: "İngilizce".into(),
            web_research_enabled: true,
            research_source_limit: 3,
            memory_enabled: true,
        }
    }
}

impl AppSettings {
    pub fn validate(&self) -> crate::Result<()> {
        if !["light", "dark"].contains(&self.theme.as_str())
            || !(13..=17).contains(&self.font_size)
            || !["concise", "balanced", "detailed"].contains(&self.response_style.as_str())
            || !self.temperature.is_finite()
            || !(0.0..=1.5).contains(&self.temperature)
            || !(256..=8192).contains(&self.max_tokens)
            || !(1..=5).contains(&self.research_source_limit)
            || self.preferred_name.chars().count() > 80
            || self.custom_instructions.chars().count() > 2000
            || !crate::prompts::LANGUAGES.contains(&self.translation_language.as_str())
        {
            return Err(crate::Error::Validation(
                "Ayar değerleri izin verilen aralığın dışında.".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConversationExport {
    pub conversation: Conversation,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryExport {
    pub format_version: u32,
    pub exported_at: i64,
    pub conversations: Vec<ConversationExport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub data_dir: String,
    pub database_bytes: u64,
    pub model_bytes: u64,
    pub conversation_count: usize,
    pub message_count: usize,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub state: String,
    pub detail: String,
    pub model: String,
    pub requested_device: ComputeDevice,
    pub backend: Option<String>,
    pub device_name: Option<String>,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GenerationEvent {
    Status { text: String },
    Delta { text: String },
    Sources { sources: Vec<Source> },
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
