use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

pub const APP_NAME: &str = "eAI Lite";
pub const DEFAULT_OLLAMA: &str = "http://127.0.0.1:11434";
pub const DEFAULT_SEARX: &str = "http://127.0.0.1:8080";

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub user_name: String,
    pub model: String,
    pub thinking: bool,
    pub max_tokens: u32,
    pub instructions: String,
    pub web_search: bool,
    pub censorless: bool,
    pub api_provider: String,
    pub api_base: String,
    pub api_key: String,
    pub ollama_slots: Vec<String>,
    pub ollama_active: usize,
    pub searx_slots: Vec<String>,
    pub searx_active: usize,
    pub model_names: HashMap<String, String>,
    pub agents: Vec<Agent>,
    pub active_agent: Option<Uuid>,
    pub agent_mode: bool,
    pub groups: Vec<AgentGroup>,
    pub active_group: Option<Uuid>,
    pub workspace_root: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub purpose: String,
    pub instructions: String,
    pub memory: String,
    pub model: String,
    pub web_search: bool,
    pub thinking: bool,
    pub max_tokens: u32,
    pub max_actions: u32,
    pub file_access: bool,
    pub command_access: bool,
    pub desktop_access: bool,
    pub vision_capable: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct AgentGroup {
    pub id: Uuid,
    pub name: String,
    pub agent_ids: Vec<Uuid>,
}

impl Default for AgentGroup {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New group".into(),
            agent_ids: Vec::new(),
        }
    }
}

impl Default for Agent {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New agent".into(),
            purpose: String::new(),
            instructions: String::new(),
            memory: String::new(),
            model: String::new(),
            web_search: true,
            thinking: true,
            max_tokens: 4096,
            max_actions: 20,
            file_access: true,
            command_access: true,
            desktop_access: true,
            vision_capable: false,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            user_name: String::new(),
            model: "llama3.2:latest".into(),
            thinking: true,
            max_tokens: 4096,
            instructions: String::new(),
            web_search: true,
            censorless: false,
            api_provider: "ollama".into(),
            api_base: "https://api.openai.com/v1".into(),
            api_key: String::new(),
            ollama_slots: vec![DEFAULT_OLLAMA.into()],
            ollama_active: 0,
            searx_slots: vec![DEFAULT_SEARX.into()],
            searx_active: 0,
            model_names: HashMap::new(),
            agents: Vec::new(),
            active_agent: None,
            agent_mode: false,
            groups: Vec::new(),
            active_group: None,
            workspace_root: env!("CARGO_MANIFEST_DIR").into(),
        }
    }
}

fn slot(list: &[String], idx: usize, default: &str) -> String {
    list.get(idx)
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

impl Settings {
    pub fn ollama_host(&self) -> String {
        slot(&self.ollama_slots, self.ollama_active, DEFAULT_OLLAMA)
    }

    pub fn searx_host(&self) -> String {
        slot(&self.searx_slots, self.searx_active, DEFAULT_SEARX)
    }

    /// Custom name if set, otherwise the raw model id.
    pub fn display_name(&self, id: &str) -> String {
        self.model_names
            .get(id)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| id.to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Source {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Attachment {
    pub name: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Message {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: i64,
    pub thinking: String,
    pub model: String,
    #[serde(skip)]
    pub progress: String,
    #[serde(default)]
    pub tool_log: Vec<String>,
    #[serde(default)]
    pub group_speaker: Option<String>,
    pub sources: Vec<Source>,
    /// Attached image bytes (JPEG). Not saved to disk to keep conversations small.
    #[serde(skip)]
    pub image: Option<Vec<u8>>,
    /// Attached text/code file. Saved with the conversation.
    #[serde(default)]
    pub attachment: Option<Attachment>,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            role: String::new(),
            content: String::new(),
            created_at: 0,
            thinking: String::new(),
            model: String::new(),
            progress: String::new(),
            tool_log: Vec::new(),
            group_speaker: None,
            sources: Vec::new(),
            image: None,
            attachment: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Session {
    pub id: Uuid,
    pub title: String,
    pub messages: Vec<Message>,
    pub auto_titled: bool,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            title: "New Chat".into(),
            messages: vec![],
            auto_titled: false,
        }
    }
}

pub fn data_dir() -> PathBuf {
    match active_profile() {
        Some(id) => profile_dir(id),
        // Nothing is read or written until a profile is signed in,
        // but this keeps every path valid just in case.
        None => base_dir().join("profiles").join("local"),
    }
}

/// Shared application folder, used for accounts and the exported app icon.
pub fn base_dir() -> PathBuf {
    dirs::document_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("eAI")
}

pub fn profile_dir(id: Uuid) -> PathBuf {
    base_dir().join("profiles").join(id.simple().to_string())
}

static ACTIVE_PROFILE: Mutex<Option<Uuid>> = Mutex::new(None);

pub fn set_active_profile(id: Option<Uuid>) {
    if let Ok(mut guard) = ACTIVE_PROFILE.lock() {
        *guard = id;
    }
}

pub fn active_profile() -> Option<Uuid> {
    ACTIVE_PROFILE.lock().ok().and_then(|guard| *guard)
}

pub fn avatar_path() -> PathBuf {
    data_dir().join("avatar.png")
}

/// Copies chats, settings and avatar from the old single-user layout into the
/// first profile. The originals are left untouched so nothing can be lost.
pub fn migrate_legacy_data(id: Uuid) {
    let root = base_dir();
    let dst = profile_dir(id);
    let _ = std::fs::create_dir_all(&dst);

    for name in ["settings.json", "conversations.json"] {
        let from = root.join(name);
        let to = dst.join(name);
        if from.exists() && !to.exists() {
            let _ = std::fs::copy(from, to);
        }
    }

    let old_avatar = root.join("avatars").join("user.png");
    let new_avatar = dst.join("avatar.png");
    if old_avatar.exists() && !new_avatar.exists() {
        let _ = std::fs::copy(old_avatar, new_avatar);
    }
}

/// Reads the name from the old single-user settings file, if there is one.
/// Used to pre-fill the first-run form instead of a hard-coded name.
pub fn legacy_user_name() -> Option<String> {
    let text = std::fs::read_to_string(base_dir().join("settings.json")).ok()?;
    let settings: Settings = serde_json::from_str(&text).ok()?;
    let name = settings.user_name.trim().to_string();
    if name.is_empty() || name == "Erol" {
        None
    } else {
        Some(name)
    }
}

pub fn load_settings() -> Settings {
    std::fs::read_to_string(data_dir().join("settings.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(s: &Settings) {
    let _ = std::fs::create_dir_all(data_dir());
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(data_dir().join("settings.json"), json);
    }
}

pub fn load_sessions() -> Vec<Session> {
    std::fs::read_to_string(data_dir().join("conversations.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_sessions(list: &[Session]) {
    let _ = std::fs::create_dir_all(data_dir());
    if let Ok(json) = serde_json::to_string_pretty(list) {
        let _ = std::fs::write(data_dir().join("conversations.json"), json);
    }
}

/// Resizes the picture to a 256px square and saves it as the user's avatar.
pub fn save_avatar_from_file(path: &std::path::Path) -> bool {
    let Ok(img) = image::open(path) else {
        return false;
    };
    let _ = std::fs::create_dir_all(avatar_path().parent().unwrap_or(&data_dir()));
    let square = img.resize_to_fill(256, 256, image::imageops::FilterType::Lanczos3);
    square.save(avatar_path()).is_ok()
}

pub fn remove_avatar() {
    let _ = std::fs::remove_file(avatar_path());
}

pub fn fallback_title(text: &str) -> String {
    let title = text
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| "\"'*#.:-!?“”‘’ ".contains(c))
        .to_string();
    if title.is_empty() {
        "New Chat".into()
    } else if title.chars().count() > 40 {
        let t: String = title.chars().take(38).collect();
        format!("{t}…")
    } else {
        title
    }
}

pub fn supports_thinking(model: &str) -> bool {
    let m = model.to_lowercase();
    m.contains("qwen3") || m.contains("deepseek-r1")
}

/// Pulls inline <think>…</think> blocks out of a reply. Returns (thinking, visible text).
pub fn split_think(s: &str) -> (String, String) {
    let mut thinking = String::new();
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 7..];
        match after.find("</think>") {
            Some(end) => {
                thinking.push_str(&after[..end]);
                rest = &after[end + 8..];
            }
            None => {
                thinking.push_str(after);
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    (thinking.trim().to_string(), out.trim().to_string())
}
