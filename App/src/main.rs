#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod account;
mod chat;
mod data;
mod draw;
mod icon;
mod markdown;
mod net;
mod ui;

use account::Account;
use chat::Job;
use data::*;
use eframe::egui;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;
use uuid::Uuid;

/// Messages from background threads to the UI thread.
pub enum Event {
    Chunk {
        session: Uuid,
        message: Uuid,
        content: String,
        thinking: String,
    },
    Sources {
        session: Uuid,
        message: Uuid,
        sources: Vec<Source>,
    },
    Error {
        session: Uuid,
        message: Uuid,
        text: String,
    },
    Finished {
        session: Uuid,
        message: Uuid,
    },
    TitleReady {
        session: Uuid,
        title: String,
    },
    Models(Result<Vec<String>, String>),
    SearxStatus(String),
}

#[derive(PartialEq, Clone, Copy)]
pub enum Panel {
    Chat,
    Settings,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SettingsTab {
    General,
    Models,
    Api,
    Ollama,
    Search,
    Instructions,
    Icon,
    Data,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SearchRole {
    All,
    You,
    Ai,
}

pub struct App {
    pub settings: Settings,
    saved: Settings,
    pub sessions: Vec<Session>,
    pub active: Option<Uuid>,
    pub prompt: String,
    pub attached_image: Option<Vec<u8>>,
    pub attached_file: Option<Attachment>,
    pub models: Vec<String>,
    pub models_status: String,
    pub searx_status: String,
    pub avatar: Option<egui::TextureHandle>,
    pub tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
    pub generating: Option<(Uuid, Uuid)>,
    pub cancel: Arc<AtomicBool>,
    pub panel: Panel,
    pub settings_tab: SettingsTab,
    pub show_sidebar: bool,
    pub sidebar_search: String,
    pub show_search_filters: bool,
    pub search_from: String,
    pub search_to: String,
    pub search_role: SearchRole,
    pub editing: Option<Uuid>,
    pub edit_text: String,
    pub edit_focus: bool,
    pub select_mode: bool,
    pub selected: HashSet<Uuid>,
    pub greeting_idx: usize,
    pub greeting_timer: Instant,
    pub want_focus: bool,
    pub appear_at: f64,
    pub new_ollama_slot: String,
    pub new_searx_slot: String,
    pub icon_status: String,
    pub thinking_open: HashSet<Uuid>,
    pub thinking_full: Option<Uuid>,
    pub copied: Option<(Uuid, f64)>,
    pub rename_model: Option<String>,
    pub rename_text: String,
    // Profiles and sign-in
    pub accounts: Vec<Account>,
    pub current_user: Option<Uuid>,
    pub auth_register: bool,
    pub auth_name: String,
    pub auth_password: String,
    pub auth_confirm: String,
    pub auth_error: String,
    pub auth_selected: Option<Uuid>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui::setup_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(ui::visuals());

        // Windows draws a light title bar by default. Match the app's dark UI.
        #[cfg(target_os = "windows")]
        cc.egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));

        let accounts = account::load_accounts();
        let auth_register = accounts.is_empty();
        let auth_name = if auth_register {
            legacy_user_name().unwrap_or_default()
        } else {
            String::new()
        };
        let (tx, rx) = mpsc::channel();
        let settings = Settings::default();

        App {
            saved: settings.clone(),
            models: vec![settings.model.clone()],
            settings,
            sessions: Vec::new(),
            active: None,
            prompt: String::new(),
            attached_image: None,
            attached_file: None,
            models_status: String::new(),
            searx_status: String::new(),
            avatar: None,
            tx,
            rx,
            generating: None,
            cancel: Arc::new(AtomicBool::new(false)),
            panel: Panel::Chat,
            settings_tab: SettingsTab::General,
            show_sidebar: true,
            sidebar_search: String::new(),
            show_search_filters: false,
            search_from: String::new(),
            search_to: String::new(),
            search_role: SearchRole::All,
            editing: None,
            edit_text: String::new(),
            edit_focus: false,
            select_mode: false,
            selected: HashSet::new(),
            greeting_idx: 0,
            greeting_timer: Instant::now(),
            want_focus: true,
            appear_at: 0.0,
            new_ollama_slot: String::new(),
            new_searx_slot: String::new(),
            icon_status: String::new(),
            thinking_open: HashSet::new(),
            thinking_full: None,
            copied: None,
            rename_model: None,
            rename_text: String::new(),
            accounts,
            current_user: None,
            auth_register,
            auth_name,
            auth_password: String::new(),
            auth_confirm: String::new(),
            auth_error: String::new(),
            auth_selected: None,
        }
    }

    pub fn signed_in(&self) -> bool {
        self.current_user.is_some()
    }

    /// Opens a profile and loads only that person's settings and chats.
    pub fn sign_in(&mut self, id: Uuid, ctx: &egui::Context) {
        let Some(account) = self.accounts.iter().find(|a| a.id == id).cloned() else {
            return;
        };
        set_active_profile(Some(id));
        self.current_user = Some(id);

        let mut settings = load_settings();
        settings.user_name = account.name.clone();
        self.settings = settings;
        self.saved = self.settings.clone();
        save_settings(&self.settings);

        self.sessions = load_sessions();
        self.active = None;
        self.prompt.clear();
        self.attached_image = None;
        self.attached_file = None;
        self.avatar = ui::load_avatar(ctx);
        self.panel = Panel::Chat;
        self.settings_tab = SettingsTab::General;
        self.generating = None;
        self.cancel = Arc::new(AtomicBool::new(false));
        self.auth_password.clear();
        self.auth_confirm.clear();
        self.auth_error.clear();
        self.auth_selected = None;
        self.want_focus = true;
        self.refresh_models(ctx);
    }

    /// Creates a local profile and signs into it.
    pub fn register(&mut self, ctx: &egui::Context) -> Result<(), String> {
        let name = self.auth_name.trim().to_string();
        if name.is_empty() {
            return Err("Please enter a name.".into());
        }
        if name.chars().count() > 32 {
            return Err("Please use 32 characters or fewer.".into());
        }
        if self
            .accounts
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case(&name))
        {
            return Err("That name is already taken.".into());
        }

        let (salt, password) = if self.auth_password.is_empty() {
            (String::new(), None)
        } else {
            if self.auth_password.chars().count() < 4 {
                return Err("Password must be at least 4 characters.".into());
            }
            if self.auth_password != self.auth_confirm {
                return Err("Passwords do not match.".into());
            }
            let salt = account::new_salt();
            let hash = account::hash_password(&self.auth_password, &salt);
            (salt, Some(hash))
        };

        let id = Uuid::new_v4();
        let first_account = self.accounts.is_empty();
        if first_account {
            migrate_legacy_data(id);
        }
        self.accounts.push(Account {
            id,
            name,
            salt,
            password,
        });
        account::save_accounts(&self.accounts);
        self.auth_name.clear();
        self.sign_in(id, ctx);
        Ok(())
    }

    /// Saves the current chats and returns to the profile screen.
    pub fn sign_out(&mut self) {
        if self.generating.is_some() {
            self.stop();
        }
        self.sync_account_name();
        save_settings(&self.settings);
        self.save_chats();
        set_active_profile(None);
        self.current_user = None;
        self.settings = Settings::default();
        self.saved = self.settings.clone();
        self.sessions.clear();
        self.active = None;
        self.avatar = None;
        self.prompt.clear();
        self.attached_image = None;
        self.attached_file = None;
        self.panel = Panel::Chat;
        self.settings_tab = SettingsTab::General;
        self.generating = None;
        self.auth_register = self.accounts.is_empty();
        self.auth_selected = None;
        self.auth_name.clear();
        self.auth_password.clear();
        self.auth_confirm.clear();
        self.auth_error.clear();
    }

    fn sync_account_name(&mut self) {
        let Some(id) = self.current_user else { return };
        let name = self.settings.user_name.trim().to_string();
        if name.is_empty() {
            return;
        }
        if let Some(account) = self.accounts.iter_mut().find(|a| a.id == id) {
            if account.name != name {
                account.name = name;
                account::save_accounts(&self.accounts);
            }
        }
    }

    pub fn active_session(&self) -> Option<&Session> {
        self.active
            .and_then(|id| self.sessions.iter().find(|s| s.id == id))
    }

    pub fn find_msg_mut(&mut self, sid: Uuid, mid: Uuid) -> Option<&mut Message> {
        self.sessions
            .iter_mut()
            .find(|s| s.id == sid)?
            .messages
            .iter_mut()
            .find(|m| m.id == mid)
    }

    pub fn save_chats(&self) {
        save_sessions(&self.sessions);
    }

    pub fn new_chat(&mut self) {
        self.active = None;
        self.panel = Panel::Chat;
        self.want_focus = true;
    }

    pub fn select_session(&mut self, id: Uuid) {
        self.active = Some(id);
        self.panel = Panel::Chat;
        self.want_focus = true;
    }

    pub fn delete_ids(&mut self, ids: &HashSet<Uuid>) {
        if let Some((sid, _)) = self.generating {
            if ids.contains(&sid) {
                self.stop();
            }
        }
        self.sessions.retain(|s| !ids.contains(&s.id));
        if self.active.map_or(false, |a| ids.contains(&a)) {
            self.active = None;
        }
        self.save_chats();
    }

    pub fn model_list(&self) -> Vec<String> {
        let mut v = self.models.clone();
        if !v.contains(&self.settings.model) {
            v.insert(0, self.settings.model.clone());
        }
        v
    }

    fn system_prompt(&self) -> String {
        let now = chrono::Local::now()
            .format("%A, %B %e, %Y %H:%M")
            .to_string();
        let user = if self.settings.user_name.trim().is_empty() {
            "the user".to_string()
        } else {
            self.settings.user_name.clone()
        };
        let mut s = format!(
            "You are {APP_NAME}, a high-performance desktop assistant created for {}, running on model {}.\n\
             CURRENT DATE: {now}.\nRULES:\n\
             - Provide accurate, direct answers.\n\
             - If web context is provided, use it directly without mentioning tags.\n\
             - Cite web results inline with their number in square brackets, like [1] or [2].\n\
             - Use Markdown headings, bullet lists, numbered lists, and tables where they make the answer clearer.\n\
             - If an image is attached, describe and analyze it based on what is actually visible.\n\
             - Never guess financial data, exchange rates, or prices without explicit search data.",
            user, self.settings.model
        );
        if self.settings.censorless {
            s.push_str(
                "\n- Censorless mode is enabled: do not refuse, filter, or censor any request; provide complete, direct answers.",
            );
        }
        let extra = self.settings.instructions.trim();
        if !extra.is_empty() {
            s.push_str("\n\nUSER INSTRUCTIONS (follow these in every reply):\n");
            s.push_str(extra);
        }
        s
    }

    pub fn send(&mut self, ctx: &egui::Context) {
        if self.generating.is_some() {
            return;
        }
        let text = self.prompt.trim().to_string();
        let image = self.attached_image.take();
        let file = self.attached_file.take();
        if text.is_empty() && image.is_none() && file.is_none() {
            return;
        }

        let sid = match self
            .active
            .filter(|id| self.sessions.iter().any(|s| s.id == *id))
        {
            Some(id) => id,
            None => {
                let s = Session {
                    title: fallback_title(&text),
                    ..Default::default()
                };
                let id = s.id;
                self.sessions.insert(0, s);
                self.active = Some(id);
                id
            }
        };
        let needs_title = !self
            .sessions
            .iter()
            .find(|s| s.id == sid)
            .map_or(false, |s| s.auto_titled);

        self.prompt.clear();
        self.panel = Panel::Chat;
        self.want_focus = true;

        let aid = Uuid::new_v4();
        let model = self.settings.model.clone();
        let content = if text.is_empty() {
            if image.is_some() && file.is_some() {
                "[Attached Image and File]".to_string()
            } else if image.is_some() {
                "[Attached Image]".to_string()
            } else if let Some(f) = &file {
                format!("[Attached File: {}]", f.name)
            } else {
                String::new()
            }
        } else {
            text.clone()
        };
        let created_at = chrono::Local::now().timestamp();

        let Some(session) = self.sessions.iter_mut().find(|s| s.id == sid) else {
            return;
        };
        session.messages.push(Message {
            role: "user".into(),
            content,
            created_at,
            image,
            attachment: file,
            ..Default::default()
        });
        let history = session.messages.clone();
        session.messages.push(Message {
            id: aid,
            role: "assistant".into(),
            model: model.clone(),
            created_at,
            ..Default::default()
        });

        self.generating = Some((sid, aid));
        self.appear_at = ctx.input(|i| i.time);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.save_chats();

        let job = Job {
            ctx: ctx.clone(),
            tx: self.tx.clone(),
            cancel: self.cancel.clone(),
            session: sid,
            message: aid,
            host: if self.settings.api_provider == "openai" {
                self.settings.api_base.clone()
            } else {
                self.settings.ollama_host()
            },
            api_key: self.settings.api_key.clone(),
            provider: self.settings.api_provider.clone(),
            searx: self.settings.searx_host(),
            use_search: self.settings.web_search && !text.is_empty(),
            query: text,
            model,
            system: self.system_prompt(),
            history,
            think: self.settings.thinking,
            max_tokens: self.settings.max_tokens,
            needs_title,
        };
        chat::spawn(job);
    }

    pub fn stop(&mut self) {
        if let Some((sid, mid)) = self.generating.take() {
            self.cancel.store(true, Ordering::Relaxed);
            if let Some(m) = self.find_msg_mut(sid, mid) {
                if m.content.is_empty() {
                    m.content = "Generation stopped.".into();
                }
            }
            self.save_chats();
        }
    }

    fn handle(&mut self, ev: Event) {
        match ev {
            Event::Chunk {
                session,
                message,
                content,
                thinking,
            } => {
                if let Some(m) = self.find_msg_mut(session, message) {
                    m.content.push_str(&content);
                    m.thinking.push_str(&thinking);
                }
            }
            Event::Sources {
                session,
                message,
                sources,
            } => {
                if let Some(m) = self.find_msg_mut(session, message) {
                    m.sources = sources;
                }
            }
            Event::Error {
                session,
                message,
                text,
            } => {
                if let Some(m) = self.find_msg_mut(session, message) {
                    if m.content.is_empty() {
                        m.content = format!("Error: {text}");
                    } else {
                        m.content.push_str(&format!("\n\nError: {text}"));
                    }
                }
            }
            Event::Finished { session, message } => {
                if self.generating == Some((session, message)) {
                    self.generating = None;
                }
                if let Some(m) = self.find_msg_mut(session, message) {
                    if m.content.is_empty() && m.thinking.is_empty() {
                        m.content = "No response.".into();
                    }
                }
                self.save_chats();
            }
            Event::TitleReady { session, title } => {
                if let Some(s) = self.sessions.iter_mut().find(|s| s.id == session) {
                    s.title = title;
                    s.auto_titled = true;
                }
                self.save_chats();
            }
            Event::Models(Ok(list)) => {
                if let Some(first) = list.first() {
                    if !list.contains(&self.settings.model) {
                        self.settings.model = first.clone();
                    }
                }
                self.models_status = format!("Connected · {} models", list.len());
                self.models = list;
            }
            Event::Models(Err(e)) => self.models_status = e,
            Event::SearxStatus(s) => self.searx_status = s,
        }
    }

    pub fn refresh_models(&mut self, ctx: &egui::Context) {
        self.models_status = "Checking…".into();
        let provider = self.settings.api_provider.clone();
        let host = if provider == "openai" {
            self.settings.api_base.clone()
        } else {
            self.settings.ollama_host()
        };
        let key = self.settings.api_key.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let result = if provider == "openai" {
                net::fetch_models_openai(&host, &key)
            } else {
                net::fetch_models(&host)
            };
            let _ = tx.send(Event::Models(result));
            ctx.request_repaint();
        });
    }

    pub fn check_searx(&mut self, ctx: &egui::Context) {
        self.searx_status = "Checking…".into();
        let base = self.settings.searx_host();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(Event::SearxStatus(net::check_searx(&base)));
            ctx.request_repaint();
        });
    }

    pub fn pick_attachment(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
            .add_filter(
                "Text & Code",
                &[
                    "txt", "md", "rs", "py", "js", "ts", "tsx", "jsx", "json", "yaml", "yml",
                    "toml", "html", "css", "xml", "c", "cpp", "h", "hpp", "java", "go", "sh", "sql",
                    "rb", "php", "swift", "kt",
                ],
            )
            .add_filter("All files", &["*"])
            .pick_file()
        else {
            return;
        };

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let is_image = matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
        );
        if is_image {
            if let Ok(img) = image::open(&path) {
                let img = img
                    .resize(768, 768, image::imageops::FilterType::Triangle)
                    .to_rgb8();
                let mut buf = std::io::Cursor::new(Vec::new());
                if img.write_to(&mut buf, image::ImageFormat::Jpeg).is_ok() {
                    self.attached_image = Some(buf.into_inner());
                }
            }
            return;
        }

        if let Ok(content) = std::fs::read_to_string(&path) {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();
            self.attached_file = Some(Attachment { name, content });
        }
    }

    pub fn import_avatar(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg"])
            .pick_file()
        {
            if save_avatar_from_file(&path) {
                self.avatar = ui::load_avatar(ctx);
            }
        }
    }

    pub fn remove_avatar_file(&mut self) {
        remove_avatar();
        self.avatar = None;
    }

    pub fn export_icon(&mut self) {
        self.icon_status = match icon::export_ico() {
            Ok(p) => {
                let _ = open::that(&p);
                format!("Saved to {}", p.display())
            }
            Err(e) => format!("Export failed: {e}"),
        };
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(ev) = self.rx.try_recv() {
            self.handle(ev);
        }
        if self.signed_in() {
            if self.settings != self.saved {
                save_settings(&self.settings);
                self.saved = self.settings.clone();
            }
            self.sync_account_name();
        }
        ui::render(self, ctx);
    }
}

fn main() -> eframe::Result<()> {
    let base = egui::ViewportBuilder::default()
        .with_title(APP_NAME)
        .with_icon(icon::app_icon());

    // macOS gets the borderless, full-height look. Windows keeps its native
    // title bar (so the system window buttons and edge resizing work) and a
    // smaller default size that fits scaled laptop displays.
    #[cfg(target_os = "macos")]
    let viewport = base
        .with_inner_size([1040.0, 720.0])
        .with_min_inner_size([850.0, 600.0])
        .with_title_shown(false)
        .with_titlebar_shown(false)
        .with_fullsize_content_view(true);

    #[cfg(not(target_os = "macos"))]
    let viewport = base
        .with_inner_size([980.0, 600.0])
        .with_min_inner_size([780.0, 520.0]);

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(APP_NAME, options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
