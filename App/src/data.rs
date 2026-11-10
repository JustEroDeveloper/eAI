use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
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
    pub ollama_slots: Vec<String>,
    pub ollama_active: usize,
    pub searx_slots: Vec<String>,
    pub searx_active: usize,
    pub model_names: HashMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            user_name: "Erol".into(),
            model: "llama3.2:latest".into(),
            thinking: true,
            max_tokens: 4096,
            instructions: String::new(),
            web_search: true,
            ollama_slots: vec![DEFAULT_OLLAMA.into()],
            ollama_active: 0,
            searx_slots: vec![DEFAULT_SEARX.into()],
            searx_active: 0,
            model_names: HashMap::new(),
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Message {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: i64,
    pub thinking: String,
    pub model: String,
    pub sources: Vec<Source>,
    /// Attached image bytes (JPEG). Not saved to disk to keep conversations small.
    #[serde(skip)]
    pub image: Option<Vec<u8>>,
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
            sources: Vec::new(),
            image: None,
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
    dirs::document_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("eAI")
}

pub fn avatar_path() -> PathBuf {
    data_dir().join("avatars").join("user.png")
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
