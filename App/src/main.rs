mod chat;
mod data;
mod draw;
mod icon;
mod markdown;
mod net;
mod ui;

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
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui::setup_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(ui::visuals());

        let settings = load_settings();
        let (tx, rx) = mpsc::channel();

        let mut app = App {
            saved: settings.clone(),
            models: vec![settings.model.clone()],
            settings,
            sessions: load_sessions(),
            active: None,
            prompt: String::new(),
            attached_image: None,
            models_status: String::new(),
            searx_status: String::new(),
            avatar: ui::load_avatar(&cc.egui_ctx),
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
        };
        app.refresh_models(&cc.egui_ctx);
        app
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
        let mut s = format!(
            "You are {APP_NAME}, a high-performance desktop assistant created for {}, running on model {}.\n\
             CURRENT DATE: {now}.\nRULES:\n\
             - Provide accurate, direct answers.\n\
             - If web context is provided, use it directly without mentioning tags.\n\
             - Cite web results inline with their number in square brackets, like [1] or [2].\n\
             - Use Markdown headings, bullet lists, numbered lists, and tables where they make the answer clearer.\n\
             - If an image is attached, describe and analyze it based on what is actually visible.\n\
             - Never guess financial data, exchange rates, or prices without explicit search data.",
            self.settings.user_name, self.settings.model
        );
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
        if text.is_empty() && image.is_none() {
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
            "[Attached Image]".to_string()
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
            host: self.settings.ollama_host(),
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
        let host = self.settings.ollama_host();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(Event::Models(net::fetch_models(&host)));
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

    pub fn pick_image(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
            .pick_file()
        else {
            return;
        };
        if let Ok(img) = image::open(&path) {
            let img = img
                .resize(768, 768, image::imageops::FilterType::Triangle)
                .to_rgb8();
            let mut buf = std::io::Cursor::new(Vec::new());
            if img.write_to(&mut buf, image::ImageFormat::Jpeg).is_ok() {
                self.attached_image = Some(buf.into_inner());
            }
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
        if self.settings != self.saved {
            save_settings(&self.settings);
            self.saved = self.settings.clone();
        }
        ui::render(self, ctx);
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 720.0])
            .with_min_inner_size([850.0, 600.0])
            .with_title(APP_NAME)
            .with_title_shown(false)
            .with_titlebar_shown(false)
            .with_fullsize_content_view(true)
            .with_icon(icon::app_icon()),
        ..Default::default()
    };
    eframe::run_native(APP_NAME, options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
