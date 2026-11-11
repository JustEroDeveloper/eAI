use crate::account::Account;
use crate::data::*;
use crate::draw::*;
use crate::markdown;
use crate::{App, Panel, SearchRole, SettingsTab};
use chrono::{Local, NaiveDate, Timelike};
use eframe::egui::{
    self, Align, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Frame, Id, Key,
    Label, Layout, Margin, Modifiers, Painter, Pos2, Rect, RichText, Rounding, ScrollArea, Sense,
    Stroke, TextEdit, TextureHandle, Ui, Vec2, Visuals,
};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use uuid::Uuid;

const GREETINGS: [&str; 5] = [
    "What's on your mind",
    "Where should we start",
    "What are we building today",
    "Ask me anything",
    "What can I help you figure out",
];

const CHIPS: [(&str, &str); 4] = [
    ("Brainstorm", "Help me brainstorm ideas for "),
    ("Summarize text", "Summarize the following: "),
    ("Write code", "Write a function that "),
    ("Explain a concept", "Explain like I'm five: "),
];

const TABS: [(SettingsTab, &str); 9] = [
    (SettingsTab::General, "General"),
    (SettingsTab::Agents, "Agents"),
    (SettingsTab::Models, "Models"),
    (SettingsTab::Api, "API"),
    (SettingsTab::Ollama, "Ollama"),
    (SettingsTab::Search, "Search"),
    (SettingsTab::Instructions, "Instructions"),
    (SettingsTab::Icon, "App Icon"),
    (SettingsTab::Data, "Data"),
];

const CHAT_WIDTH: f32 = 720.0;
const REPLY_BG: Color32 = Color32::from_rgb(26, 26, 30);
const PILL_BG: Color32 = Color32::from_rgb(34, 34, 40);

// ---------- Setup ----------

pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let regular = [
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
    ];
    let bold_files = [
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "C:\\Windows\\Fonts\\segoeuib.ttf",
    ];

    let base = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut reg_list = base;
    if let Some(bytes) = regular.iter().find_map(|p| std::fs::read(p).ok()) {
        fonts
            .font_data
            .insert("ui-regular".into(), FontData::from_owned(bytes));
        reg_list.insert(0, "ui-regular".into());
    }

    // Extra fallback fonts with broader Unicode coverage, so model output that
    // contains symbols or emoji does not render as empty boxes.
    let fallback_candidates = [
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Apple Symbols.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Monaco.ttf",
        "C:\\\\Windows\\\\Fonts\\\\arialuni.ttf",
        "C:\\\\Windows\\\\Fonts\\\\seguiemj.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    ];
    for (idx, path) in fallback_candidates.iter().enumerate() {
        if let Ok(bytes) = std::fs::read(path) {
            let name = format!("fallback-{idx}");
            fonts
                .font_data
                .insert(name.clone(), FontData::from_owned(bytes));
            reg_list.push(name);
        }
    }

    let mut bold_list = reg_list.clone();
    if let Some(bytes) = bold_files.iter().find_map(|p| std::fs::read(p).ok()) {
        fonts
            .font_data
            .insert("ui-bold".into(), FontData::from_owned(bytes));
        bold_list.insert(0, "ui-bold".into());
    }
    fonts.families.insert(FontFamily::Proportional, reg_list);
    fonts
        .families
        .insert(FontFamily::Name("Bold".into()), bold_list);
    ctx.set_fonts(fonts);
}

pub fn visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.panel_fill = WINDOW_BG;
    v.window_fill = CARD;
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(10.0);
    v.extreme_bg_color = Color32::from_rgb(28, 28, 30);
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT;
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.rounding = Rounding::same(8.0);
    }
    v.widgets.inactive.weak_bg_fill = CARD_HI;
    v.widgets.inactive.bg_fill = CARD_HI;
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(70, 70, 74);
    v.widgets.hovered.bg_fill = Color32::from_rgb(70, 70, 74);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(84, 84, 88);
    v.widgets.active.bg_fill = Color32::from_rgb(84, 84, 88);
    v
}

pub fn load_avatar(ctx: &egui::Context) -> Option<TextureHandle> {
    let img = image::open(avatar_path()).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture("user_avatar", color, Default::default()))
}

// ---------- Small helpers ----------

fn semi(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Bold".into()))
}

fn reg(size: f32) -> FontId {
    FontId::proportional(size)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

/// Gradient text that drifts through the blue, purple, pink colours over time.
fn gradient_title(ui: &mut Ui, text: &str, size: f32, time: f32) {
    let galley = ui.fonts(|f| f.layout_job(gradient_job(text, size, time)));
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, TEXT);
}

fn avatar_at(ui: &mut Ui, tex: Option<&TextureHandle>, rect: Rect, initial: &str) {
    match tex {
        Some(t) => {
            ui.put(
                rect,
                egui::Image::new(t)
                    .fit_to_exact_size(rect.size())
                    .rounding(rect.width() / 2.0),
            );
        }
        None => avatar_circle(ui.painter(), rect.center(), rect.width(), initial),
    }
}

fn icon_button(ui: &mut Ui, draw_fn: fn(&Painter, Pos2, Color32), tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool(resp.id, resp.hovered());
    if hover > 0.0 {
        ui.painter().rect_filled(
            rect,
            Rounding::same(7.0),
            Color32::from_white_alpha((22.0 * hover) as u8),
        );
    }
    draw_fn(ui.painter(), rect.center(), mix(TEXT_SEC, TEXT, hover));
    resp.on_hover_text(tip).clicked()
}

/// Full-width sidebar row with optional icon and a smooth hover fade.
fn sidebar_row(
    ui: &mut Ui,
    label: &str,
    icon: Option<fn(&Painter, Pos2, Color32)>,
    selected: bool,
) -> egui::Response {
    let (full, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool(resp.id, resp.hovered());
    let rect = full.shrink2(Vec2::new(8.0, 0.0));
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, Rounding::same(7.0), select());
    } else if hover > 0.0 {
        p.rect_filled(
            rect,
            Rounding::same(7.0),
            Color32::from_white_alpha((12.0 * hover) as u8),
        );
    }
    let mut x = rect.left() + 12.0;
    if let Some(f) = icon {
        f(p, Pos2::new(x + 7.0, rect.center().y), TEXT_SEC);
        x += 26.0;
    }
    p.text(
        Pos2::new(x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        reg(13.0),
        if selected { TEXT } else { TEXT_SEC },
    );
    resp
}

fn switch(ui: &mut Ui, on: &mut bool) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(38.0, 22.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
    }
    let t = ui.ctx().animate_bool(resp.id, *on);
    let p = ui.painter();
    p.rect_filled(
        rect,
        Rounding::same(11.0),
        mix(Color32::from_gray(70), ACCENT, t),
    );
    p.circle_filled(
        Pos2::new(rect.left() + 11.0 + t * 16.0, rect.center().y),
        9.0,
        Color32::WHITE,
    );
}

fn shortcut(key: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("⌘{key}")
    } else {
        format!("Ctrl+{key}")
    }
}

fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(text).font(semi(11.0)).color(TEXT_SEC));
    ui.add_space(4.0);
}

fn card(ui: &mut Ui, f: impl FnOnce(&mut Ui)) {
    Frame::none()
        .fill(CARD)
        .rounding(Rounding::same(14.0))
        .inner_margin(Margin::symmetric(18.0, 12.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            f(ui);
        });
    ui.add_space(10.0);
}

fn settings_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(label)
            .fill(Color32::from_rgb(43, 44, 49))
            .stroke(st(1.0, Color32::from_rgb(65, 66, 73))),
    )
}

fn row(ui: &mut Ui, label: &str, f: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| f(ui));
    });
}

fn slot_editor(ui: &mut Ui, slots: &mut Vec<String>, active: &mut usize, new_value: &mut String) {
    let mut remove = None;
    for i in 0..slots.len() {
        let label = slots[i].clone();
        ui.horizontal(|ui| {
            if ui.radio(*active == i, label.clone()).clicked() {
                *active = i;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if slots.len() > 1 && settings_button(ui, "Remove").clicked() {
                    remove = Some(i);
                }
            });
        });
    }
    if let Some(i) = remove {
        slots.remove(i);
        if *active >= slots.len() {
            *active = slots.len().saturating_sub(1);
        }
    }
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(new_value)
                .hint_text("http://host:port")
                .desired_width(260.0),
        );
        if settings_button(ui, "Add").clicked() && !new_value.trim().is_empty() {
            slots.push(new_value.trim().to_string());
            new_value.clear();
        }
    });
}

// ---------- Profiles ----------

fn primary_button(ui: &mut Ui, label: &str) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool(resp.id, resp.hovered());
    ui.painter().rect_filled(
        rect,
        Rounding::same(9.0),
        mix(ACCENT, Color32::from_rgb(74, 160, 255), hover),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        semi(14.0),
        Color32::WHITE,
    );
    resp.clicked()
}

fn text_link(ui: &mut Ui, label: &str) -> bool {
    ui.add(Label::new(RichText::new(label).size(12.0).color(ACCENT)).sense(Sense::click()))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn field_label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(semi(10.0)).color(TEXT_SEC));
}

fn auth_error(ui: &mut Ui, message: &str) {
    if message.is_empty() {
        return;
    }
    Frame::none()
        .fill(Color32::from_rgb(60, 30, 33))
        .stroke(st(1.0, Color32::from_rgb(120, 52, 58)))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(message)
                    .size(11.5)
                    .color(Color32::from_rgb(255, 150, 150)),
            );
        });
    ui.add_space(8.0);
}

fn account_row(ui: &mut Ui, account: &Account, selected: bool) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 50.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool(resp.id, resp.hovered());
    let p = ui.painter();
    if selected || hover > 0.0 {
        p.rect_filled(
            rect,
            Rounding::same(10.0),
            if selected {
                select()
            } else {
                Color32::from_white_alpha((10.0 * hover) as u8)
            },
        );
    }
    let center = Pos2::new(rect.left() + 28.0, rect.center().y);
    avatar_circle(p, center, 34.0, &initial_of(&account.name));
    p.text(
        Pos2::new(rect.left() + 54.0, rect.center().y - 8.0),
        Align2::LEFT_CENTER,
        &account.name,
        semi(13.0),
        TEXT,
    );
    let sub = if account.has_password() {
        "Password protected"
    } else {
        "Click to sign in"
    };
    p.text(
        Pos2::new(rect.left() + 54.0, rect.center().y + 9.0),
        Align2::LEFT_CENTER,
        sub,
        reg(10.5),
        TEXT_SEC,
    );
    if account.has_password() {
        p.text(
            Pos2::new(rect.right() - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            "•••",
            reg(11.0),
            TEXT_TER,
        );
    }
    resp
}

fn auth_screen(app: &mut App, ctx: &egui::Context) {
    ctx.request_repaint_after(Duration::from_millis(60));
    let mut do_login: Option<Uuid> = None;
    let mut do_register = false;
    let mut go_login = false;
    let mut go_register = false;

    egui::CentralPanel::default()
        .frame(Frame::none().fill(WINDOW_BG))
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let t = ui.input(|i| i.time) as f32;
                        ui.add_space(56.0);

                        let pulse = (t * 2.0).sin() * 0.5 + 0.5;
                        let (r, _) = ui.allocate_exact_size(Vec2::new(64.0, 56.0), Sense::hover());
                        sparkles(
                            ui.painter(),
                            r.center(),
                            18.0,
                            0.55 + 0.45 * pulse,
                            None,
                            pulse,
                        );
                        ui.add_space(10.0);
                        gradient_title(ui, APP_NAME, 26.0, t);
                        ui.add_space(6.0);
                        let subtitle = if app.auth_register {
                            "Create a local profile to get started."
                        } else if app.accounts.len() == 1 && app.accounts[0].has_password() {
                            "Enter your password to continue."
                        } else if app.accounts.len() == 1 {
                            "Continue to your profile."
                        } else {
                            "Choose your profile to continue."
                        };
                        ui.label(RichText::new(subtitle).size(13.0).color(TEXT_SEC));
                        ui.add_space(24.0);

                        let card_w = 400.0_f32.min((ui.available_width() - 32.0).max(280.0));
                        Frame::none()
                            .fill(CARD)
                            .stroke(st(1.0, Color32::from_white_alpha(18)))
                            .rounding(Rounding::same(16.0))
                            .inner_margin(Margin::symmetric(24.0, 22.0))
                            .show(ui, |ui| {
                                ui.set_width(card_w - 48.0);
                                if app.auth_register {
                                    auth_register_form(app, ui, &mut do_register, &mut go_login);
                                } else {
                                    auth_login_form(app, ui, &mut do_login, &mut go_register);
                                }
                            });

                        ui.add_space(18.0);
                        ui.label(
                            RichText::new(format!(
                                "eAI Lite {} · everything stays on this device",
                                env!("CARGO_PKG_VERSION")
                            ))
                            .size(10.0)
                            .color(TEXT_TER),
                        );
                        ui.add_space(40.0);
                    });
                });
        });

    if go_login {
        app.auth_register = false;
        app.auth_selected = None;
        app.auth_password.clear();
        app.auth_confirm.clear();
        app.auth_error.clear();
    }
    if go_register {
        app.auth_register = true;
        app.auth_selected = None;
        app.auth_password.clear();
        app.auth_confirm.clear();
        app.auth_error.clear();
    }

    if do_register {
        if let Err(err) = app.register(ctx) {
            app.auth_error = err;
        }
    }

    if let Some(id) = do_login {
        let ok = app
            .accounts
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.verify(&app.auth_password))
            .unwrap_or(false);
        if ok {
            app.sign_in(id, ctx);
        } else {
            app.auth_error = "Wrong password. Please try again.".into();
            app.auth_password.clear();
        }
    }
}

fn auth_register_form(app: &mut App, ui: &mut Ui, submit: &mut bool, go_login: &mut bool) {
    ui.style_mut().visuals.extreme_bg_color = Color32::from_rgb(29, 29, 31);
    ui.spacing_mut().item_spacing.y = 8.0;
    let enter = ui.input(|i| i.key_pressed(Key::Enter));

    auth_error(ui, &app.auth_error);

    field_label(ui, "YOUR NAME");
    let name = ui.add(
        TextEdit::singleline(&mut app.auth_name)
            .hint_text("What should I call you?")
            .desired_width(f32::INFINITY)
            .margin(Vec2::new(10.0, 8.0)),
    );
    if app.want_focus {
        name.request_focus();
        app.want_focus = false;
    }

    ui.add_space(10.0);
    field_label(ui, "PASSWORD (OPTIONAL)");
    ui.add(
        TextEdit::singleline(&mut app.auth_password)
            .hint_text("Leave blank for no password")
            .password(true)
            .desired_width(f32::INFINITY)
            .margin(Vec2::new(10.0, 8.0)),
    );
    if !app.auth_password.is_empty() {
        ui.add_space(6.0);
        ui.add(
            TextEdit::singleline(&mut app.auth_confirm)
                .hint_text("Confirm password")
                .password(true)
                .desired_width(f32::INFINITY)
                .margin(Vec2::new(10.0, 8.0)),
        );
    } else {
        ui.label(
            RichText::new("A password is only needed if others use this computer.")
                .size(10.0)
                .color(TEXT_TER),
        );
    }

    ui.add_space(14.0);
    if primary_button(ui, "Create profile") {
        *submit = true;
    }
    if enter {
        *submit = true;
    }

    if !app.accounts.is_empty() {
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            if text_link(ui, "I already have a profile") {
                *go_login = true;
            }
        });
    }
}

fn auth_login_form(app: &mut App, ui: &mut Ui, submit: &mut Option<Uuid>, go_register: &mut bool) {
    ui.style_mut().visuals.extreme_bg_color = Color32::from_rgb(29, 29, 31);
    ui.spacing_mut().item_spacing.y = 8.0;
    let enter = ui.input(|i| i.key_pressed(Key::Enter));
    let accounts = app.accounts.clone();

    auth_error(ui, &app.auth_error);
    if accounts.len() > 1 {
        field_label(ui, "PROFILES");
        for account in &accounts {
            let selected = app.auth_selected == Some(account.id);
            if account_row(ui, account, selected).clicked() {
                if account.has_password() {
                    app.auth_selected = Some(account.id);
                    app.auth_password.clear();
                    app.auth_error.clear();
                } else {
                    *submit = Some(account.id);
                }
            }
        }
    }

    let selected = if accounts.len() == 1 {
        accounts.first()
    } else {
        app.auth_selected
            .and_then(|id| accounts.iter().find(|a| a.id == id))
    };
    if let Some(account) = selected {
        if account.has_password() {
            if accounts.len() > 1 {
                ui.add_space(10.0);
                field_label(ui, "PASSWORD");
            }
            let password = ui.add(
                TextEdit::singleline(&mut app.auth_password)
                    .hint_text("Enter your password")
                    .password(true)
                    .desired_width(f32::INFINITY)
                    .margin(Vec2::new(10.0, 8.0)),
            );
            if app.want_focus {
                password.request_focus();
                app.want_focus = false;
            }
            ui.add_space(14.0);
            if primary_button(ui, "Sign in") {
                *submit = Some(account.id);
            }
            if enter {
                *submit = Some(account.id);
            }
        } else if accounts.len() == 1 && primary_button(ui, "Continue") {
            *submit = Some(account.id);
        }
    }

    ui.add_space(12.0);
    ui.vertical_centered(|ui| {
        if text_link(ui, "Create a new profile") {
            *go_register = true;
        }
    });
}

// ---------- Frame ----------

pub fn render(app: &mut App, ctx: &egui::Context) {
    if !app.signed_in() {
        auth_screen(app, ctx);
        return;
    }

    let busy = app.generating.is_some();
    ctx.request_repaint_after(Duration::from_millis(if busy { 33 } else { 60 }));
    shortcuts(app, ctx);

    if app.panel == Panel::Settings {
        settings_page(app, ctx);
    } else {
        let viewport_width = ctx.input(|i| {
            i.viewport()
                .inner_rect
                .unwrap_or_else(|| i.screen_rect())
                .width()
        });
        let input_gutter = if viewport_width < 1200.0 { 36.0 } else { 24.0 };

        if app.show_sidebar {
            egui::SidePanel::left("sidebar")
                .resizable(false)
                .exact_width(260.0)
                .frame(Frame::none().fill(SIDEBAR_BG))
                .show(ctx, |ui| sidebar(app, ui));
        } else {
            egui::SidePanel::left("sidebar_rail")
                .resizable(false)
                .exact_width(64.0)
                .show_separator_line(false)
                .frame(Frame::none().fill(SIDEBAR_BG))
                .show(ctx, |ui| sidebar_rail(app, ui));
        }

        egui::CentralPanel::default()
            .frame(Frame::none().fill(WINDOW_BG))
            .show(ctx, |ui| {
                if app.active_session().is_some() {
                    chat_view(app, ui);
                } else {
                    welcome(app, ui);
                }
            });

        let sidebar_width = if app.show_sidebar { 260.0 } else { 64.0 };
        let main_width = (viewport_width - sidebar_width).max(0.0);
        egui::Area::new(Id::new("input_overlay"))
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(sidebar_width * 0.5, -8.0))
            .show(ctx, |ui| {
                ui.set_min_width(main_width);
                ui.set_max_width(main_width);
                Frame::none()
                    .fill(Color32::TRANSPARENT)
                    .inner_margin(Margin {
                        left: input_gutter,
                        right: input_gutter,
                        top: 6.0,
                        bottom: 16.0,
                    })
                    .show(ui, |ui| input_bar(app, ui));
            });

        thinking_window(app, ctx);
    }
}

fn shortcuts(app: &mut App, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::N)) {
        app.new_chat();
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Comma)) {
        app.panel = Panel::Settings;
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::B)) {
        app.show_sidebar = !app.show_sidebar;
    }
}

// ---------- Sidebar ----------

fn sidebar(app: &mut App, ui: &mut Ui) {
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        let width = (ui.available_width() - 48.0).max(80.0);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, Rounding::same(7.0), Color32::from_white_alpha(14));
        }
        icon_plus(
            ui.painter(),
            Pos2::new(rect.left() + 12.0, rect.center().y),
            TEXT_SEC,
        );
        ui.painter().text(
            Pos2::new(rect.left() + 27.0, rect.center().y),
            Align2::LEFT_CENTER,
            "New Chat",
            reg(13.0),
            TEXT_SEC,
        );
        if response.clicked() {
            app.new_chat();
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(6.0);
            if icon_button(
                ui,
                icon_sidebar,
                &format!("Hide sidebar ({})", shortcut("B")),
            ) {
                app.show_sidebar = false;
            }
        });
    });
    ui.add_space(10.0);

    let mut filter_anchor = None;
    let mut filter_opened = false;
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        let button_size = Vec2::splat(28.0);
        let search_width = (ui.available_width() - button_size.x - 12.0).max(80.0);
        Frame::none()
            .fill(INPUT_BG)
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(10.0, 5.0))
            .show(ui, |ui| {
                ui.set_width(search_width - 16.0);
                ui.horizontal(|ui| {
                    let (icon_rect, _) =
                        ui.allocate_exact_size(Vec2::new(18.0, 22.0), Sense::hover());
                    icon_search(ui.painter(), icon_rect.center(), TEXT_SEC);
                    ui.add(
                        TextEdit::singleline(&mut app.sidebar_search)
                            .id(Id::new("sidebar_chat_search"))
                            .hint_text("Search chats")
                            .frame(false)
                            .margin(Vec2::ZERO)
                            .desired_width(ui.available_width() - 2.0),
                    );
                });
            });
        ui.add_space(6.0);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(6.0);
            let (rect, response) = ui.allocate_exact_size(button_size, Sense::click());
            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
            let hover = ui.ctx().animate_bool(response.id, response.hovered());
            let filters_active = app.search_role != SearchRole::All
                || !app.search_from.trim().is_empty()
                || !app.search_to.trim().is_empty();
            if filters_active || app.show_search_filters || hover > 0.0 {
                ui.painter().rect_filled(
                    rect.shrink(1.0),
                    Rounding::same(5.0),
                    Color32::from_white_alpha(if filters_active || app.show_search_filters {
                        18
                    } else {
                        (12.0 * hover) as u8
                    }),
                );
            }
            icon_filter(
                ui.painter(),
                rect.center(),
                if filters_active { ACCENT } else { TEXT_SEC },
            );
            if response.on_hover_text("Filter chats").clicked() {
                app.show_search_filters = !app.show_search_filters;
                filter_opened = app.show_search_filters;
            }
            filter_anchor = Some(rect);
        });
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(RichText::new("Recents").size(11.0).color(TEXT_SEC));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(14.0);
            let label = if app.select_mode { "Done" } else { "Select" };
            if ui
                .add(
                    Label::new(RichText::new(label).size(11.0).color(ACCENT)).sense(Sense::click()),
                )
                .clicked()
            {
                app.select_mode = !app.select_mode;
                app.selected.clear();
            }
        });
    });
    ui.add_space(4.0);

    if app.select_mode {
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            let n = app.selected.len();
            let del = egui::Button::new(RichText::new(format!("Delete ({n})")).color(RED));
            if ui.add_enabled(n > 0, del).clicked() {
                let ids = app.selected.clone();
                app.delete_ids(&ids);
                app.selected.clear();
                app.select_mode = false;
            }
            if ui.button("Select All").clicked() {
                app.selected = app.sessions.iter().map(|s| s.id).collect();
            }
        });
    }

    egui::TopBottomPanel::bottom("sidebar_footer")
        .frame(Frame::none())
        .show_inside(ui, |ui| footer(app, ui));

    egui::CentralPanel::default()
        .frame(Frame::none())
        .show_inside(ui, |ui| session_list(app, ui));

    if app.show_search_filters {
        if let Some(anchor) = filter_anchor {
            search_filter_popup(app, ui.ctx(), anchor, filter_opened);
        }
    }
}

fn search_filter_popup(app: &mut App, ctx: &egui::Context, anchor: Rect, opened_now: bool) {
    let screen = ctx.input(|i| i.viewport().inner_rect.unwrap_or_else(|| i.screen_rect()));
    let width = 300.0_f32.min(screen.width() - 24.0).max(220.0);
    let position = Pos2::new(
        anchor
            .left()
            .min(screen.right() - width - 12.0)
            .max(screen.left() + 12.0),
        anchor.bottom() + 8.0,
    );
    let mut close = false;
    let mut popup_rect = Rect::NOTHING;

    egui::Area::new(Id::new("sidebar_search_filter_popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .show(ctx, |ui| {
            let frame = Frame::none()
                .fill(Color32::from_rgb(35, 35, 38))
                .rounding(Rounding::same(12.0))
                .stroke(st(1.0, Color32::from_white_alpha(18)))
                .inner_margin(Margin::same(16.0));
            let response = frame.show(ui, |ui| {
                ui.set_width(width - 32.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Filters").font(semi(15.0)).color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, icon_close, "Close filters") {
                            close = true;
                        }
                    });
                });
                ui.add_space(14.0);

                ui.label(
                    RichText::new("MESSAGE AUTHOR")
                        .font(semi(10.0))
                        .color(TEXT_SEC),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let options = [
                        (SearchRole::All, "All messages"),
                        (SearchRole::You, "You"),
                        (SearchRole::Ai, "AI"),
                    ];
                    let option_width = (ui.available_width() - 12.0) / 3.0;
                    for (role, label) in options {
                        let selected = app.search_role == role;
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(option_width, 30.0), Sense::click());
                        let hover = ui.ctx().animate_bool(response.id, response.hovered());
                        if selected || hover > 0.0 {
                            ui.painter().rect_filled(
                                rect,
                                Rounding::same(6.0),
                                if selected {
                                    Color32::from_white_alpha(24)
                                } else {
                                    Color32::from_white_alpha((12.0 * hover) as u8)
                                },
                            );
                        }
                        ui.painter().text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            label,
                            reg(12.0),
                            if selected { TEXT } else { TEXT_SEC },
                        );
                        if response.clicked() {
                            app.search_role = role;
                        }
                    }
                });
                ui.add_space(14.0);

                ui.label(RichText::new("DATE RANGE").font(semi(10.0)).color(TEXT_SEC));
                ui.add_space(6.0);
                let field_width = (ui.available_width() - 12.0) / 2.0;
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("From").size(11.0).color(TEXT_SEC));
                        ui.add(
                            TextEdit::singleline(&mut app.search_from)
                                .id(Id::new("filter_date_from"))
                                .hint_text("MM/DD/YY")
                                .desired_width(field_width),
                        );
                    });
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("To").size(11.0).color(TEXT_SEC));
                        ui.add(
                            TextEdit::singleline(&mut app.search_to)
                                .id(Id::new("filter_date_to"))
                                .hint_text("MM/DD/YY")
                                .desired_width(field_width),
                        );
                    });
                });
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            Label::new(RichText::new("Clear all").size(11.0).color(TEXT_SEC))
                                .sense(Sense::click()),
                        )
                        .clicked()
                    {
                        app.search_role = SearchRole::All;
                        app.search_from.clear();
                        app.search_to.clear();
                    }
                });
            });
            popup_rect = response.response.rect;
        });

    if !opened_now
        && ctx.input(|i| i.pointer.any_click())
        && ctx
            .input(|i| i.pointer.interact_pos())
            .map_or(false, |pos| {
                !popup_rect.contains(pos) && !anchor.contains(pos)
            })
    {
        close = true;
    }
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        close = true;
    }
    if close {
        app.show_search_filters = false;
    }
}

fn sidebar_rail(app: &mut App, ui: &mut Ui) {
    egui::TopBottomPanel::bottom("sidebar_rail_footer")
        .frame(Frame::none())
        .show_inside(ui, |ui| {
            ui.vertical_centered(|ui| {
                if icon_button(ui, icon_gear, "Settings") {
                    app.panel = Panel::Settings;
                }
                ui.add_space(10.0);
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                let initial = initial_of(&app.settings.user_name);
                avatar_at(ui, app.avatar.as_ref(), rect, &initial);
                if response.on_hover_text("Account and settings").clicked() {
                    app.panel = Panel::Settings;
                }
                ui.add_space(12.0);
            });
        });

    egui::CentralPanel::default()
        .frame(Frame::none())
        .show_inside(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(12.0);
                if icon_button(ui, icon_sidebar, "Expand sidebar") {
                    app.show_sidebar = true;
                }
                ui.add_space(12.0);
                if icon_button(ui, icon_compose, "New chat") {
                    app.new_chat();
                }
                ui.add_space(8.0);
                if icon_button(ui, icon_list, "Show recent chats") {
                    app.show_sidebar = true;
                }
            });
        });
}

fn session_list(app: &mut App, ui: &mut Ui) {
    let query = app.sidebar_search.trim().to_lowercase();
    let from = search_date_bound(&app.search_from, false);
    let before = search_date_bound(&app.search_to, true);
    let role = app.search_role;
    let filtering =
        !query.is_empty() || from.is_some() || before.is_some() || role != SearchRole::All;
    let rows: Vec<(Uuid, String)> = app
        .sessions
        .iter()
        .filter(|session| {
            !filtering
                || session.messages.iter().any(|message| {
                    let role_matches = match role {
                        SearchRole::All => message.role == "user" || message.role == "assistant",
                        SearchRole::You => message.role == "user",
                        SearchRole::Ai => message.role == "assistant",
                    };
                    let text_matches =
                        query.is_empty() || message.content.to_lowercase().contains(query.as_str());
                    let date_matches = if from.is_none() && before.is_none() {
                        true
                    } else {
                        message.created_at > 0
                            && from.map_or(true, |bound| message.created_at >= bound)
                            && before.map_or(true, |bound| message.created_at < bound)
                    };
                    role_matches && text_matches && date_matches
                })
        })
        .map(|s| (s.id, s.title.clone()))
        .collect();
    let mut open: Option<Uuid> = None;
    let mut delete: Option<Uuid> = None;

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if rows.is_empty() && filtering {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new("No chats match these filters")
                            .size(11.0)
                            .color(TEXT_SEC),
                    );
                });
            }
            for (id, title) in &rows {
                let id = *id;
                if app.editing == Some(id) {
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let te = ui.add(
                            TextEdit::singleline(&mut app.edit_text)
                                .desired_width(ui.available_width() - 12.0),
                        );
                        if app.edit_focus {
                            te.request_focus();
                            app.edit_focus = false;
                        }
                        if te.lost_focus() {
                            let t = app.edit_text.trim().to_string();
                            if !t.is_empty() {
                                if let Some(s) = app.sessions.iter_mut().find(|s| s.id == id) {
                                    s.title = t;
                                    s.auto_titled = true;
                                }
                            }
                            app.editing = None;
                            app.save_chats();
                        }
                    });
                    continue;
                }

                let selected = app.panel == Panel::Chat && app.active == Some(id);
                let resp =
                    sidebar_row(ui, &truncate(title, 30), None, selected && !app.select_mode);
                if app.select_mode {
                    let c = resp.rect.left_center();
                    let checked = app.selected.contains(&id);
                    let p = ui.painter();
                    let center = Pos2::new(c.x + 12.0, c.y);
                    if checked {
                        p.circle_filled(center, 7.0, ACCENT);
                        icon_check(p, center, Color32::WHITE);
                    } else {
                        p.circle_stroke(center, 7.0, st(1.2, TEXT_SEC));
                    }
                }
                if resp.clicked() {
                    if app.select_mode {
                        if !app.selected.remove(&id) {
                            app.selected.insert(id);
                        }
                    } else {
                        open = Some(id);
                    }
                }
                resp.context_menu(|ui| {
                    if app.select_mode {
                        return;
                    }
                    if ui.button("Rename").clicked() {
                        app.editing = Some(id);
                        app.edit_text = title.clone();
                        app.edit_focus = true;
                        ui.close_menu();
                    }
                    if ui.button("Delete Chat").clicked() {
                        delete = Some(id);
                        ui.close_menu();
                    }
                });
            }
        });

    if let Some(id) = open {
        app.select_session(id);
    }
    if let Some(id) = delete {
        let mut set = HashSet::new();
        set.insert(id);
        app.delete_ids(&set);
    }
}

fn search_date_bound(value: &str, exclusive_end: bool) -> Option<i64> {
    let date = NaiveDate::parse_from_str(value.trim(), "%m/%d/%y").ok()?;
    let date = if exclusive_end {
        date.succ_opt()?
    } else {
        date
    };
    date.and_hms_opt(0, 0, 0)?
        .and_local_timezone(Local)
        .single()
        .map(|time| time.timestamp())
}

fn footer(app: &mut App, ui: &mut Ui) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        let w = ui.available_width() - 12.0;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let hover = ui.ctx().animate_bool(resp.id, resp.hovered());
        if hover > 0.0 {
            ui.painter().rect_filled(
                rect,
                Rounding::same(9.0),
                Color32::from_white_alpha((12.0 * hover) as u8),
            );
        }
        let initial = initial_of(&app.settings.user_name);
        let avatar_rect = Rect::from_center_size(
            Pos2::new(rect.left() + 26.0, rect.center().y),
            Vec2::splat(30.0),
        );
        avatar_at(ui, app.avatar.as_ref(), avatar_rect, &initial);
        let p = ui.painter();
        p.text(
            Pos2::new(rect.left() + 52.0, rect.center().y - 8.0),
            Align2::LEFT_CENTER,
            &app.settings.user_name,
            semi(13.0),
            TEXT,
        );
        p.text(
            Pos2::new(rect.left() + 52.0, rect.center().y + 8.0),
            Align2::LEFT_CENTER,
            "Settings",
            reg(11.0),
            TEXT_SEC,
        );
        icon_gear(p, Pos2::new(rect.right() - 20.0, rect.center().y), TEXT_SEC);
        if resp.clicked() {
            app.panel = Panel::Settings;
        }
    });
    ui.add_space(10.0);
}

// ---------- Welcome ----------

fn welcome(app: &mut App, ui: &mut Ui) {
    let t = ui.input(|i| i.time) as f32;
    if app.greeting_timer.elapsed() > Duration::from_secs(6) {
        app.greeting_idx = (app.greeting_idx + 1) % GREETINGS.len();
        app.greeting_timer = Instant::now();
    }
    let mut pick: Option<&str> = None;
    let hour = chrono::Local::now().hour();
    let name = app.settings.user_name.clone();
    let subtitle = GREETINGS[app.greeting_idx];

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(CHAT_WIDTH);
                ui.add_space(110.0);

                let pulse = (t * 2.0).sin() * 0.5 + 0.5;
                let (r, _) = ui.allocate_exact_size(Vec2::new(70.0, 60.0), Sense::hover());
                sparkles(
                    ui.painter(),
                    r.center(),
                    20.0,
                    0.6 + 0.4 * pulse,
                    None,
                    pulse,
                );
                ui.add_space(12.0);

                gradient_title(
                    ui,
                    &format!("{}, {}", greeting_for_hour(hour), name),
                    30.0,
                    t,
                );
                ui.add_space(6.0);

                ui.label(RichText::new(subtitle).size(14.0).color(TEXT_SEC));
                ui.add_space(26.0);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
                    let total_w: f32 = CHIPS
                        .iter()
                        .map(|(label, _)| chip_width(ui, label))
                        .sum::<f32>()
                        + 10.0 * (CHIPS.len() as f32 - 1.0);
                    let pad = ((ui.available_width() - total_w) / 2.0).max(0.0);
                    ui.add_space(pad);
                    for (idx, (label, prompt)) in CHIPS.iter().enumerate() {
                        if chip(ui, idx, label) {
                            pick = Some(prompt);
                        }
                    }
                });
            });
        });

    if let Some(p) = pick {
        app.prompt = p.to_string();
        app.want_focus = true;
    }
}

fn chip_width(ui: &Ui, label: &str) -> f32 {
    ui.fonts(|f| f.layout_no_wrap(label.to_string(), reg(13.0), TEXT_SEC))
        .size()
        .x
        + 44.0
}

fn chip(ui: &mut Ui, idx: usize, label: &str) -> bool {
    let w = chip_width(ui, label);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 34.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui
        .ctx()
        .animate_bool(Id::new(("chip", idx)), resp.hovered());
    let rect = rect.translate(Vec2::new(0.0, -2.0 * hover));
    let p = ui.painter();
    p.rect_filled(rect, Rounding::same(17.0), mix(CARD, CARD_HI, hover));
    p.rect_stroke(
        rect,
        Rounding::same(17.0),
        st(1.0, Color32::from_white_alpha(20)),
    );
    chip_icon(
        p,
        idx,
        Pos2::new(rect.left() + 20.0, rect.center().y),
        TEXT_SEC,
    );
    p.text(
        Pos2::new(rect.left() + 34.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        reg(13.0),
        TEXT,
    );
    resp.clicked()
}

// ---------- Chat ----------

/// Chat column is 720px wide, centred in the window. Everything inside is laid out left to right.
fn chat_view(app: &mut App, ui: &mut Ui) {
    let Some(session) = app.active_session().cloned() else {
        return;
    };
    let t = ui.input(|i| i.time);
    let streaming = app.generating.map(|g| g.1);

    ScrollArea::vertical()
        .id_salt(("chat", session.id))
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail = ui.available_width();
            let side_inset = 24.0_f32.min(avail * 0.08);
            let col = (avail - side_inset * 2.0).min(CHAT_WIDTH).max(0.0);
            let pad = ((avail - col) / 2.0).max(0.0);
            ui.horizontal_top(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    ui.set_width(col);
                    ui.add_space(28.0);
                    for m in &session.messages {
                        let is_streaming = streaming == Some(m.id);
                        let fade = if is_streaming {
                            ((t - app.appear_at) / 0.35).clamp(0.0, 1.0) as f32
                        } else {
                            1.0
                        };
                        ui.scope(|ui| {
                            ui.set_opacity(fade);
                            message(app, ui, m, is_streaming);
                        });
                        ui.add_space(22.0);
                    }
                    ui.add_space(160.0);
                });
            });
        });
}

fn message(app: &mut App, ui: &mut Ui, m: &Message, streaming: bool) {
    if m.role == "user" {
        ui.with_layout(Layout::top_down(Align::Max), |ui| {
            Frame::none()
                .fill(ACCENT)
                .rounding(Rounding::same(20.0))
                .inner_margin(Margin::symmetric(16.0, 10.0))
                .show(ui, |ui| {
                    ui.set_max_width(CHAT_WIDTH * 0.8);
                    if m.image.is_some() {
                        ui.label(
                            RichText::new("Image attached")
                                .size(11.0)
                                .color(Color32::from_white_alpha(200)),
                        );
                    }
                    if m.attachment.is_some() {
                        let name = m.attachment.as_ref().unwrap().name.clone();
                        ui.label(
                            RichText::new(format!("File attached: {name}"))
                                .size(11.0)
                                .color(Color32::from_white_alpha(200)),
                        );
                    }
                    ui.label(RichText::new(&m.content).size(14.0).color(Color32::WHITE));
                });
        });
        return;
    }

    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
        sparkles(ui.painter(), r.center(), 6.0, 1.0, None, 0.0);
        let t = ui.input(|i| i.time) as f32;
        gradient_title(ui, m.group_speaker.as_deref().unwrap_or(APP_NAME), 13.0, t);
        if !m.model.is_empty() {
            ui.label(
                RichText::new(app.settings.display_name(&m.model))
                    .size(11.0)
                    .color(TEXT_TER),
            );
        }
    });
    if streaming && !m.progress.is_empty() {
        ui.label(RichText::new(&m.progress).size(11.0).color(TEXT_TER));
    }
    for entry in &m.tool_log {
        ui.label(RichText::new(entry).size(11.0).color(TEXT_SEC));
    }
    ui.add_space(6.0);

    let (inline_think, body) = split_think(&m.content);
    let thinking = format!("{}{}", m.thinking, inline_think);

    if !thinking.trim().is_empty() {
        let live = streaming && body.is_empty();
        let open = live || app.thinking_open.contains(&m.id);
        let title = if live {
            "Thinking…"
        } else {
            "Thought process"
        };
        let arrow = if open { "v" } else { ">" };

        // Clickable header. Uses only plain text and a sparkle shape, no special glyphs.
        let pill = ui.add(
            Label::new(
                RichText::new(format!("{title}  {arrow}"))
                    .size(12.0)
                    .color(TEXT_SEC),
            )
            .sense(Sense::click()),
        );
        if pill.clicked() && !live {
            if !app.thinking_open.remove(&m.id) {
                app.thinking_open.insert(m.id);
            }
        }

        if open {
            Frame::none()
                .fill(PILL_BG)
                .rounding(Rounding::same(10.0))
                .inner_margin(Margin::same(10.0))
                .show(ui, |ui| {
                    // While thinking, the box follows the newest text so it auto-scrolls.
                    ScrollArea::vertical()
                        .max_height(180.0)
                        .stick_to_bottom(live)
                        .id_salt(("think", m.id))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(RichText::new(&thinking).size(12.0).color(TEXT_SEC));
                        });
                });
            if !live && ui.small_button("Full view").clicked() {
                app.thinking_full = Some(m.id);
            }
        }
        ui.add_space(8.0);
    }

    if body.trim().is_empty() {
        if streaming {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Generating…").size(12.0).color(TEXT_SEC));
            });
        }
        return;
    }

    Frame::none()
        .fill(REPLY_BG)
        .rounding(Rounding::same(18.0))
        .inner_margin(Margin::symmetric(18.0, 14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            markdown::render(ui, &body, m.id.as_u128() as u64);
        });

    if !m.sources.is_empty() {
        ui.add_space(8.0);
        sources_row(ui, &m.sources);
    }

    if !streaming {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let now = ui.input(|i| i.time);
            let recent = app
                .copied
                .map_or(false, |(id, t)| id == m.id && now - t < 1.5);
            let label = if recent { "Copied" } else { "Copy" };
            if ui
                .add(
                    Label::new(RichText::new(label).size(11.0).color(TEXT_SEC))
                        .sense(Sense::click()),
                )
                .clicked()
            {
                ui.ctx().copy_text(m.content.clone());
                app.copied = Some((m.id, now));
            }
            let tokens = m.content.split_whitespace().count() * 2;
            ui.label(
                RichText::new(format!("·  ~{tokens} tokens"))
                    .size(11.0)
                    .color(TEXT_TER),
            );
        });
    }
}

fn sources_row(ui: &mut Ui, sources: &[Source]) {
    ui.horizontal_wrapped(|ui| {
        for (i, s) in sources.iter().enumerate() {
            let host = crate::net::host_of(&s.url);
            let w = ui
                .fonts(|f| f.layout_no_wrap(host.clone(), reg(11.0), TEXT))
                .size()
                .x
                + 40.0;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 22.0), Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let hover = ui.ctx().animate_bool(resp.id, resp.hovered());
            let p = ui.painter();
            let fill = mix(
                Color32::from_white_alpha(14),
                Color32::from_white_alpha(30),
                hover,
            );
            p.rect_filled(rect, Rounding::same(11.0), fill);
            let cc = Pos2::new(rect.left() + 12.0, rect.center().y);
            p.circle_filled(cc, 7.0, ACCENT);
            p.text(
                cc,
                Align2::CENTER_CENTER,
                format!("{}", i + 1),
                semi(9.0),
                Color32::WHITE,
            );
            p.text(
                Pos2::new(rect.left() + 25.0, rect.center().y),
                Align2::LEFT_CENTER,
                &host,
                reg(11.0),
                TEXT,
            );
            if resp.clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(&s.url));
            }
            resp.on_hover_text(&s.title);
        }
    });
}

fn thinking_window(app: &mut App, ctx: &egui::Context) {
    let Some(id) = app.thinking_full else { return };

    // Find the message and take both the streamed thinking and any inline <think> text.
    let (text, live) = app
        .sessions
        .iter()
        .flat_map(|s| s.messages.iter())
        .find(|m| m.id == id)
        .map(|m| {
            let (inline, _) = split_think(&m.content);
            let combined = format!("{}{}", m.thinking, inline);
            (
                combined.trim().to_string(),
                app.generating.map(|g| g.1) == Some(id),
            )
        })
        .unwrap_or_default();

    let mut open = true;
    egui::Window::new("Thought Process")
        .open(&mut open)
        .default_size([640.0, 480.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.set_min_size(Vec2::new(600.0, 420.0));
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(live)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let shown = if text.is_empty() {
                        "Waiting for the model…".to_string()
                    } else {
                        text.clone()
                    };
                    ui.label(RichText::new(shown).size(13.0).color(TEXT_SEC));
                });
        });
    if !open {
        app.thinking_full = None;
    }
}

// ---------- Input bar ----------

fn input_bar(app: &mut App, ui: &mut Ui) {
    let generating = app.generating.is_some();
    let focus_id = Id::new("prompt_input");
    let focused = ui.ctx().memory(|m| m.has_focus(focus_id));
    let shift = ui.input(|i| i.modifiers.shift);
    let has_image = app.attached_image.is_some();
    let has_file = app.attached_file.is_some();
    let models = app.model_list();
    let selected_agent_id = app.settings.active_agent;
    let active_agent_id = if app.settings.agent_mode {
        selected_agent_id
    } else {
        None
    };
    let effective_search = active_agent_id
        .and_then(|id| app.settings.agents.iter().find(|a| a.id == id))
        .map_or(app.settings.web_search, |a| a.web_search);
    let selected_model = active_agent_id
        .and_then(|id| app.settings.agents.iter().find(|a| a.id == id))
        .map(|a| {
            if a.model.is_empty() {
                app.settings.model.clone()
            } else {
                a.model.clone()
            }
        })
        .unwrap_or_else(|| app.settings.model.clone());
    let mut send = false;
    let mut stop = false;

    if focused && !shift && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
        send = true;
    }

    ui.vertical_centered(|ui| {
        let composer_width = (ui.available_width() - 80.0).min(CHAT_WIDTH).max(0.0);
        ui.set_max_width(composer_width);
        ui.allocate_ui_with_layout(
            Vec2::new(composer_width, 16.0),
            Layout::right_to_left(Align::Center),
            |ui| {
                ui.label(
                    RichText::new(format!(
                        "{APP_NAME} can make mistakes. Verify important info."
                    ))
                    .size(10.0)
                    .color(TEXT_TER),
                );
            },
        );
        ui.add_space(4.0);
        let border = if focused {
            ACCENT
        } else {
            Color32::from_rgb(52, 117, 182)
        };
        Frame::none()
            .fill(INPUT_BG)
            .stroke(st(1.4, border))
            .rounding(Rounding::same(14.0))
            .inner_margin(Margin {
                left: 12.0,
                right: 12.0,
                top: 10.0,
                bottom: 9.0,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                if has_image {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Image attached").size(11.0).color(TEXT_SEC));
                        if ui.small_button("Remove").clicked() {
                            app.attached_image = None;
                        }
                    });
                }
                if app.attached_file.is_some() {
                    let name = app.attached_file.as_ref().unwrap().name.clone();
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("File attached: {name}"))
                                .size(11.0)
                                .color(TEXT_SEC),
                        );
                        if ui.small_button("Remove").clicked() {
                            app.attached_file = None;
                        }
                    });
                }

                ScrollArea::vertical()
                    .id_salt("prompt_scroll")
                    .max_height(140.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        let edit = ui.add(
                            TextEdit::multiline(&mut app.prompt)
                                .id(focus_id)
                                .hint_text("Type a message and press Enter to send ...")
                                .desired_rows(1)
                                .frame(false)
                                .desired_width(f32::INFINITY)
                                .font(reg(15.0))
                                .text_color(TEXT),
                        );
                        if app.want_focus {
                            edit.request_focus();
                            app.want_focus = false;
                        }
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let active_name = app
                        .settings
                        .active_group
                        .and_then(|id| app.settings.groups.iter().find(|g| g.id == id))
                        .map(|g| format!("Group: {}", g.name))
                        .or_else(|| {
                            app.settings
                                .active_agent
                                .and_then(|id| app.settings.agents.iter().find(|a| a.id == id))
                                .map(|a| a.name.clone())
                        })
                        .unwrap_or_else(|| "Choose agent".into());
                    let enabled = app.settings.agent_mode;
                    let (robot_rect, robot_response) =
                        ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
                    let robot_hover = robot_response.hovered();
                    let robot_fill = if enabled {
                        ACCENT.gamma_multiply(0.24)
                    } else if robot_hover {
                        Color32::from_white_alpha(14)
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter()
                        .rect_filled(robot_rect, Rounding::same(7.0), robot_fill);
                    icon_robot(
                        ui.painter(),
                        robot_rect.center(),
                        if enabled { ACCENT } else { TEXT_SEC },
                    );
                    if robot_response
                        .on_hover_text(if enabled {
                            "Agent Mode on — click to turn off"
                        } else {
                            "Enable Agent Mode"
                        })
                        .clicked()
                    {
                        if app.settings.agent_mode {
                            app.settings.agent_mode = false;
                        } else if let Some(group_id) = app.settings.active_group {
                            let member_count = app
                                .settings
                                .groups
                                .iter()
                                .find(|g| g.id == group_id)
                                .map(|g| {
                                    g.agent_ids
                                        .iter()
                                        .filter(|id| {
                                            app.settings.agents.iter().any(|a| a.id == **id)
                                        })
                                        .count()
                                })
                                .unwrap_or(0);
                            if member_count < 2 {
                                app.panel = Panel::Settings;
                                app.settings_tab = SettingsTab::Agents;
                            } else {
                                app.settings.agent_mode = true;
                            }
                        } else if app.settings.agents.is_empty() {
                            app.panel = Panel::Settings;
                            app.settings_tab = SettingsTab::Agents;
                        } else {
                            if app.settings.active_agent.is_none() {
                                app.settings.active_agent =
                                    app.settings.agents.first().map(|agent| agent.id);
                            }
                            app.settings.agent_mode = app.settings.active_agent.is_some();
                        }
                    }
                    if icon_button(ui, icon_plus, "Add image or file") {
                        app.pick_attachment();
                    }
                    if app.settings.agent_mode {
                        egui::ComboBox::from_id_salt("agent_picker")
                            .selected_text(RichText::new(active_name).size(12.0).color(TEXT))
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        app.settings.active_agent.is_none()
                                            && app.settings.active_group.is_none(),
                                        "Default",
                                    )
                                    .clicked()
                                {
                                    app.settings.active_agent = None;
                                    app.settings.active_group = None;
                                    app.settings.agent_mode = false;
                                }
                                for agent in &app.settings.agents {
                                    if ui
                                        .selectable_label(
                                            app.settings.active_agent == Some(agent.id),
                                            &agent.name,
                                        )
                                        .clicked()
                                    {
                                        app.settings.active_agent = Some(agent.id);
                                        app.settings.active_group = None;
                                    }
                                }
                                if !app.settings.groups.is_empty() {
                                    ui.separator();
                                    ui.label(
                                        RichText::new("Agent groups").size(11.0).color(TEXT_SEC),
                                    );
                                    for group in &app.settings.groups {
                                        let member_count = group.agent_ids.len();
                                        let label = format!("{} ({member_count})", group.name);
                                        if ui
                                            .selectable_label(
                                                app.settings.active_group == Some(group.id),
                                                label,
                                            )
                                            .clicked()
                                        {
                                            app.settings.active_group = Some(group.id);
                                            app.settings.active_agent = None;
                                            if group.agent_ids.len() < 2 {
                                                app.settings.agent_mode = false;
                                            }
                                        }
                                    }
                                }
                            });
                    }
                    let search_label = if effective_search {
                        "Web search on"
                    } else {
                        "Web search"
                    };
                    let search_color = if effective_search { ACCENT } else { TEXT_SEC };
                    if ui
                        .add(
                            Label::new(RichText::new(search_label).size(12.0).color(search_color))
                                .sense(Sense::click()),
                        )
                        .on_hover_text("Toggle web search")
                        .clicked()
                    {
                        if let Some(agent) = app
                            .settings
                            .agents
                            .iter_mut()
                            .find(|a| Some(a.id) == active_agent_id)
                        {
                            agent.web_search = !agent.web_search;
                        } else {
                            app.settings.web_search = !app.settings.web_search;
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
                        let resp = resp
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(
                                if generating
                                    && app.settings.agent_mode
                                    && app.settings.active_group.is_some()
                                {
                                    "Interrupt group discussion"
                                } else if generating {
                                    "Stop generation"
                                } else {
                                    "Send message"
                                },
                            );
                        let enabled = !app.prompt.trim().is_empty() || has_image || has_file;
                        let hover = ui.ctx().animate_bool(resp.id, resp.hovered() && enabled);
                        let radius = 15.0 * (1.0 + 0.08 * hover);
                        let p = ui.painter();
                        if generating {
                            p.circle_filled(rect.center(), 15.0, RED);
                            icon_stop(p, rect.center(), Color32::WHITE);
                            if resp.clicked() {
                                stop = true;
                            }
                        } else {
                            let fill = if enabled {
                                ACCENT
                            } else {
                                Color32::from_rgb(70, 70, 74)
                            };
                            p.circle_filled(rect.center(), radius, fill);
                            icon_arrow_up(
                                p,
                                rect.center(),
                                if enabled { Color32::WHITE } else { TEXT_TER },
                            );
                            if resp.clicked() && enabled {
                                send = true;
                            }
                        }

                        ui.add_space(8.0);

                        ui.scope(|ui| {
                            let visuals = &mut ui.style_mut().visuals.widgets;
                            for widget in [
                                &mut visuals.inactive,
                                &mut visuals.hovered,
                                &mut visuals.active,
                                &mut visuals.open,
                            ] {
                                widget.bg_fill = Color32::TRANSPARENT;
                                widget.weak_bg_fill = Color32::TRANSPARENT;
                                widget.bg_stroke = Stroke::NONE;
                            }
                            egui::ComboBox::from_id_salt("model_picker")
                                .selected_text(
                                    RichText::new(format!(
                                        "Model   {}",
                                        truncate(&app.settings.display_name(&selected_model), 22)
                                    ))
                                    .size(12.0)
                                    .color(TEXT),
                                )
                                .icon(|ui, rect, visuals, open, _| {
                                    let c = rect.center();
                                    let (y0, y1) = if open {
                                        (c.y + 1.0, c.y - 3.0)
                                    } else {
                                        (c.y - 1.0, c.y + 3.0)
                                    };
                                    let color = visuals.fg_stroke.color;
                                    ui.painter().line_segment(
                                        [Pos2::new(c.x - 3.0, y0), Pos2::new(c.x, y1)],
                                        st(1.4, color),
                                    );
                                    ui.painter().line_segment(
                                        [Pos2::new(c.x, y1), Pos2::new(c.x + 3.0, y0)],
                                        st(1.4, color),
                                    );
                                })
                                .show_ui(ui, |ui| {
                                    ui.label(
                                        RichText::new("Available models")
                                            .size(11.0)
                                            .color(TEXT_SEC),
                                    );
                                    ui.separator();
                                    for m in &models {
                                        let label = app.settings.display_name(m);
                                        if ui
                                            .selectable_label(selected_model == *m, label)
                                            .clicked()
                                        {
                                            if let Some(agent) = app
                                                .settings
                                                .agents
                                                .iter_mut()
                                                .find(|a| Some(a.id) == active_agent_id)
                                            {
                                                agent.model = m.clone();
                                            } else {
                                                app.settings.model = m.clone();
                                            }
                                        }
                                    }
                                });
                        });
                    });
                });
            });
    });

    if stop {
        app.stop();
    } else if send {
        let ctx = ui.ctx().clone();
        app.send(&ctx);
    }
}

// ---------- Settings ----------

fn settings_page(app: &mut App, ctx: &egui::Context) {
    let mut close = false;

    egui::SidePanel::left("settings_navigation")
        .resizable(false)
        .exact_width(250.0)
        .frame(Frame::none().fill(SIDEBAR_BG))
        .show(ctx, |ui| {
            ui.add_space(28.0);
            for (tab, label) in TABS {
                if settings_nav_item(ui, label, app.settings_tab == tab) {
                    app.settings_tab = tab;
                }
            }
            ui.add_space((ui.available_height() - 48.0).max(0.0));
            if settings_nav_item(ui, "Back to chat", false) {
                close = true;
            }
        });

    egui::CentralPanel::default()
        .frame(Frame::none().fill(WINDOW_BG))
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .id_salt("settings_page_content")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(36.0);
                    ui.horizontal(|ui| {
                        ui.add_space(36.0);
                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 36.0).min(980.0));
                            settings_content(app, ui);
                        });
                    });
                    ui.add_space(36.0);
                });
        });

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        close = true;
    }
    if close {
        app.panel = Panel::Chat;
        app.want_focus = true;
    }
}

fn settings_nav_item(ui: &mut Ui, label: &str, selected: bool) -> bool {
    let (row, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::click());
    let rect = row.shrink2(Vec2::new(14.0, 1.0));
    let hover = ui.ctx().animate_bool(response.id, response.hovered());
    if selected {
        ui.painter()
            .rect_filled(rect, Rounding::same(8.0), Color32::from_white_alpha(20));
    } else if hover > 0.0 {
        ui.painter().rect_filled(
            rect,
            Rounding::same(8.0),
            Color32::from_white_alpha((12.0 * hover) as u8),
        );
    }
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        reg(14.0),
        if selected { TEXT } else { TEXT_SEC },
    );
    response.clicked()
}

fn settings_content(app: &mut App, ui: &mut Ui) {
    match app.settings_tab {
        SettingsTab::Agents => {
            heading(ui, "AGENTS");
            ui.label(
                RichText::new(
                    "Create focused assistants with their own model and response settings. Choose one in the composer, then click the robot to turn on Agent Mode.",
                )
                .size(11.0)
                .color(TEXT_SEC),
            );
            ui.add_space(8.0);
            card(ui, |ui| {
                ui.label(RichText::new("Agent workspace").size(11.0).color(TEXT_SEC));
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&app.settings.workspace_root)
                            .size(11.0)
                            .color(TEXT),
                    );
                    if settings_button(ui, "Choose folder").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            app.settings.workspace_root = path.display().to_string();
                        }
                    }
                });
                ui.label(
                    RichText::new("File tools are confined here. Commands start here but can access other locations.")
                        .size(10.0)
                        .color(TEXT_TER),
                );
            });
            ui.add_space(8.0);
            if ui
                .add_sized(
                    Vec2::new(ui.available_width(), 34.0),
                    egui::Button::new("Create agent").fill(Color32::from_rgb(42, 43, 48)),
                )
                .clicked()
            {
                let agent = Agent::default();
                app.settings.active_agent = Some(agent.id);
                app.settings.agents.push(agent);
            }
            ui.add_space(8.0);
            if !app.settings.agents.is_empty() {
                let agent_tabs: Vec<(uuid::Uuid, String)> = app
                    .settings
                    .agents
                    .iter()
                    .map(|agent| (agent.id, agent.name.clone()))
                    .collect();
                ui.columns(agent_tabs.len(), |columns| {
                    for (column, (agent_id, name)) in columns.iter_mut().zip(agent_tabs.iter()) {
                        let width = column.available_width();
                        let selected = app.settings.active_agent == Some(*agent_id);
                        if column
                            .add_sized(
                                Vec2::new(width, 34.0),
                                egui::Button::new(name).fill(if selected {
                                    ACCENT.gamma_multiply(0.28)
                                } else {
                                    Color32::from_rgb(35, 36, 40)
                                }),
                            )
                            .clicked()
                        {
                            app.settings.active_agent = Some(*agent_id);
                            app.settings.active_group = None;
                        }
                    }
                });
                ui.add_space(8.0);
            }
            let selected = app.settings.active_agent;
            if let Some(index) = app
                .settings
                .agents
                .iter()
                .position(|a| Some(a.id) == selected)
            {
                let models = app.model_list();
                let mut remove = false;
                card(ui, |ui| {
                    ui.label(RichText::new("Name").size(11.0).color(TEXT_SEC));
                    ui.add(
                        TextEdit::singleline(&mut app.settings.agents[index].name)
                            .desired_width(300.0),
                    );
                    ui.add_space(6.0);
                    ui.label(RichText::new("Purpose").size(11.0).color(TEXT_SEC));
                    ui.add(
                        TextEdit::singleline(&mut app.settings.agents[index].purpose)
                            .hint_text("What this agent is for")
                            .desired_width(420.0),
                    );
                    ui.add_space(6.0);
                    ui.label(RichText::new("Instructions").size(11.0).color(TEXT_SEC));
                    ui.add(
                        TextEdit::multiline(&mut app.settings.agents[index].instructions)
                            .hint_text("How this agent should respond")
                            .desired_rows(4)
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Persistent memory")
                            .size(11.0)
                            .color(TEXT_SEC),
                    );
                    ui.add(
                        TextEdit::multiline(&mut app.settings.agents[index].memory)
                            .hint_text(
                                "Facts and preferences this agent should remember between chats",
                            )
                            .desired_rows(3)
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(6.0);
                    egui::ComboBox::from_id_salt("agent_model_setting")
                        .selected_text(if app.settings.agents[index].model.is_empty() {
                            format!(
                                "Default model ({})",
                                app.settings.display_name(&app.settings.model)
                            )
                        } else {
                            app.settings.display_name(&app.settings.agents[index].model)
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.settings.agents[index].model,
                                String::new(),
                                "Use default model",
                            );
                            for model in &models {
                                let label = app.settings.display_name(model);
                                ui.selectable_value(
                                    &mut app.settings.agents[index].model,
                                    model.clone(),
                                    label,
                                );
                            }
                        });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut app.settings.agents[index].web_search, "Web search");
                        ui.checkbox(&mut app.settings.agents[index].thinking, "Thinking");
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut app.settings.agents[index].file_access, "Project files");
                        ui.checkbox(
                            &mut app.settings.agents[index].command_access,
                            "Run commands",
                        );
                        ui.checkbox(
                            &mut app.settings.agents[index].desktop_access,
                            "Desktop control",
                        );
                    });
                    ui.checkbox(
                        &mut app.settings.agents[index].vision_capable,
                        "This model can inspect images",
                    );
                    if app.settings.agents[index].desktop_access {
                        ui.label(RichText::new("When needed, screenshots are sent to the selected model provider for screen understanding.").size(10.0).color(TEXT_TER));
                    }
                    ui.checkbox(
                        &mut app.settings.agents[index].vision_capable,
                        "Model can inspect images (for hosted providers)",
                    );
                    ui.add(
                        egui::Slider::new(&mut app.settings.agents[index].max_tokens, 256..=16384)
                            .text("Max output tokens per step"),
                    );
                    ui.add(
                        egui::Slider::new(&mut app.settings.agents[index].max_actions, 1..=40)
                            .text("Maximum tool actions per task"),
                    );
                    remove = ui
                        .add_sized(
                            Vec2::new(ui.available_width(), 32.0),
                            egui::Button::new("Delete agent").fill(Color32::from_rgb(70, 35, 39)),
                        )
                        .clicked();
                });
                if remove {
                    let removed_id = app.settings.agents[index].id;
                    app.settings.agents.remove(index);
                    app.settings.active_agent = None;
                    app.settings.agent_mode = false;
                    for group in &mut app.settings.groups {
                        group.agent_ids.retain(|id| *id != removed_id);
                    }
                }
            }

            ui.add_space(28.0);
            heading(ui, "AGENT GROUPS");
            ui.label(RichText::new("Combine agents with different roles. They can discuss, ask one another questions, and produce a shared answer.").size(11.0).color(TEXT_SEC));
            ui.add_space(8.0);
            if ui
                .add_sized(
                    Vec2::new(ui.available_width(), 34.0),
                    egui::Button::new("Create group").fill(Color32::from_rgb(42, 43, 48)),
                )
                .clicked()
            {
                let group = AgentGroup::default();
                app.settings.active_group = Some(group.id);
                app.settings.active_agent = None;
                app.settings.agent_mode = false;
                app.settings.groups.push(group);
            }
            let group_selected = app.settings.active_group;
            if let Some(index) = app
                .settings
                .groups
                .iter()
                .position(|g| Some(g.id) == group_selected)
            {
                let mut remove_group = false;
                let agents = app.settings.agents.clone();
                card(ui, |ui| {
                    ui.label(RichText::new("Group name").size(11.0).color(TEXT_SEC));
                    ui.add(
                        TextEdit::singleline(&mut app.settings.groups[index].name)
                            .desired_width(300.0),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Members (choose at least two)")
                            .size(11.0)
                            .color(TEXT_SEC),
                    );
                    for agent in &agents {
                        let mut included = app.settings.groups[index].agent_ids.contains(&agent.id);
                        if ui
                            .checkbox(&mut included, format!("{} — {}", agent.name, agent.purpose))
                            .changed()
                        {
                            let members = &mut app.settings.groups[index].agent_ids;
                            if included && !members.contains(&agent.id) {
                                members.push(agent.id);
                            }
                            if !included {
                                members.retain(|id| *id != agent.id);
                            }
                            if app.settings.agent_mode
                                && app.settings.active_group == Some(app.settings.groups[index].id)
                                && app.settings.groups[index].agent_ids.len() < 2
                            {
                                app.settings.agent_mode = false;
                            }
                        }
                    }
                    if agents.is_empty() {
                        ui.label(
                            RichText::new("Create agents above before adding group members.")
                                .size(11.0)
                                .color(TEXT_TER),
                        );
                    }
                    ui.add_space(6.0);
                    remove_group = ui
                        .add_sized(
                            Vec2::new(ui.available_width(), 32.0),
                            egui::Button::new("Delete group").fill(Color32::from_rgb(70, 35, 39)),
                        )
                        .clicked();
                });
                if remove_group {
                    app.settings.groups.remove(index);
                    app.settings.active_group = None;
                    app.settings.agent_mode = false;
                }
            }
        }
        SettingsTab::General => {
            heading(ui, "PROFILE");
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    let initial = initial_of(&app.settings.user_name);
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(64.0), Sense::hover());
                    avatar_at(ui, app.avatar.as_ref(), r, &initial);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Display name").size(10.0).color(TEXT_SEC));
                        ui.add(
                            TextEdit::singleline(&mut app.settings.user_name).desired_width(240.0),
                        );
                        ui.horizontal(|ui| {
                            if settings_button(ui, "Change Photo").clicked() {
                                let ctx = ui.ctx().clone();
                                app.import_avatar(&ctx);
                            }
                            if app.avatar.is_some() && settings_button(ui, "Remove").clicked() {
                                app.remove_avatar_file();
                            }
                        });
                    });
                });
            });

            heading(ui, "ACCOUNT");
            card(ui, |ui| {
                ui.label(
                    RichText::new(
                        "Signing out returns you to the profile picker. This profile's chats stay saved.",
                    )
                    .size(12.0)
                    .color(TEXT_SEC),
                );
                ui.add_space(8.0);
                if settings_button(ui, "Sign out").clicked() {
                    app.sign_out();
                }
            });
            if !app.signed_in() {
                return;
            }

            heading(ui, "CHAT");
            card(ui, |ui| {
                row(ui, "Web search", |ui| {
                    switch(ui, &mut app.settings.web_search)
                });
                ui.add_space(8.0);
                row(ui, "Thinking (Qwen3, DeepSeek-R1)", |ui| {
                    switch(ui, &mut app.settings.thinking)
                });
                ui.add_space(8.0);
                row(ui, "Censorless mode", |ui| {
                    switch(ui, &mut app.settings.censorless)
                });
                ui.label(
                    RichText::new("Instructs the model not to refuse or filter replies.")
                        .size(10.5)
                        .color(TEXT_SEC),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Maximum reply length").color(TEXT));
                    ui.add(
                        egui::Slider::new(&mut app.settings.max_tokens, 256..=16384)
                            .suffix(" tokens"),
                    );
                });
                ui.add_space(8.0);
                let models = app.model_list();
                row(ui, "Default model", |ui| {
                    egui::ComboBox::from_id_salt("default_model")
                        .width(200.0)
                        .selected_text(app.settings.display_name(&app.settings.model))
                        .show_ui(ui, |ui| {
                            for m in &models {
                                let label = app.settings.display_name(m);
                                ui.selectable_value(&mut app.settings.model, m.clone(), label);
                            }
                        });
                });
            });
        }

        SettingsTab::Models => {
            heading(ui, "MODELS");
            ui.label(
                RichText::new("Rename how models appear. Ollama still uses the real name. Vision models can read images.")
                    .size(11.0)
                    .color(TEXT_SEC),
            );
            ui.add_space(6.0);
            card(ui, |ui| {
                if app.models.is_empty() {
                    ui.label(
                        RichText::new("No models found. Check the Ollama address.").color(TEXT_SEC),
                    );
                }
                for m in app.models.clone() {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(app.settings.display_name(&m)).color(TEXT));
                            ui.label(RichText::new(&m).size(10.0).color(TEXT_SEC));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if app.rename_model.as_deref() == Some(m.as_str()) {
                                if settings_button(ui, "Save").clicked() {
                                    let t = app.rename_text.trim().to_string();
                                    if t.is_empty() || t == m {
                                        app.settings.model_names.remove(&m);
                                    } else {
                                        app.settings.model_names.insert(m.clone(), t);
                                    }
                                    app.rename_model = None;
                                }
                                ui.add(
                                    TextEdit::singleline(&mut app.rename_text).desired_width(160.0),
                                );
                            } else if settings_button(ui, "Rename").clicked() {
                                app.rename_text = app.settings.display_name(&m);
                                app.rename_model = Some(m.clone());
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            });
        }

        SettingsTab::Api => {
            heading(ui, "API PROVIDER");
            ui.label(
                RichText::new("Use Ollama locally, or connect an OpenAI, Anthropic, Gemini, or compatible API.")
                    .size(11.0)
                    .color(TEXT_SEC),
            );
            ui.add_space(6.0);
            card(ui, |ui| {
                row(ui, "Provider", |ui| {
                    let previous_provider = app.settings.api_provider.clone();
                    egui::ComboBox::from_id_salt("api_provider")
                        .selected_text(match app.settings.api_provider.as_str() {
                            "openai" => "OpenAI-compatible",
                            "anthropic" => "Anthropic",
                            "gemini" => "Gemini",
                            _ => "Ollama",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.settings.api_provider,
                                "ollama".into(),
                                "Ollama",
                            );
                            ui.selectable_value(
                                &mut app.settings.api_provider,
                                "anthropic".into(),
                                "Anthropic",
                            );
                            ui.selectable_value(
                                &mut app.settings.api_provider,
                                "gemini".into(),
                                "Gemini",
                            );
                            ui.selectable_value(
                                &mut app.settings.api_provider,
                                "openai".into(),
                                "OpenAI-compatible",
                            );
                        });
                    if app.settings.api_provider != previous_provider {
                        app.settings.api_base = match app.settings.api_provider.as_str() {
                            "anthropic" => "https://api.anthropic.com/v1".into(),
                            "gemini" => "https://generativelanguage.googleapis.com/v1beta".into(),
                            _ => "https://api.openai.com/v1".into(),
                        };
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Base URL").color(TEXT));
                ui.add(
                    TextEdit::singleline(&mut app.settings.api_base)
                        .desired_width(f32::INFINITY)
                        .hint_text(match app.settings.api_provider.as_str() {
                            "anthropic" => "https://api.anthropic.com/v1",
                            "gemini" => "https://generativelanguage.googleapis.com/v1beta",
                            _ => "https://api.openai.com/v1",
                        }),
                );
                ui.add_space(8.0);
                ui.label(RichText::new("API key").color(TEXT));
                ui.add(
                    TextEdit::singleline(&mut app.settings.api_key)
                        .password(true)
                        .desired_width(f32::INFINITY)
                        .hint_text("sk-..."),
                );
            });
            ui.horizontal(|ui| {
                if settings_button(ui, "Test & Refresh Models").clicked() {
                    let ctx = ui.ctx().clone();
                    app.refresh_models(&ctx);
                }
                if !app.models_status.is_empty() {
                    let ok = app.models_status.starts_with("Connected");
                    ui.label(RichText::new(&app.models_status).size(12.0).color(if ok {
                        GREEN
                    } else {
                        TEXT_SEC
                    }));
                }
            });
        }

        SettingsTab::Ollama => {
            heading(ui, "OLLAMA ADDRESS");
            card(ui, |ui| {
                slot_editor(
                    ui,
                    &mut app.settings.ollama_slots,
                    &mut app.settings.ollama_active,
                    &mut app.new_ollama_slot,
                );
            });
            ui.horizontal(|ui| {
                if settings_button(ui, "Test & Refresh Models").clicked() {
                    let ctx = ui.ctx().clone();
                    app.refresh_models(&ctx);
                }
                if !app.models_status.is_empty() {
                    let ok = app.models_status.starts_with("Connected");
                    ui.label(RichText::new(&app.models_status).size(12.0).color(if ok {
                        GREEN
                    } else {
                        TEXT_SEC
                    }));
                }
            });
        }

        SettingsTab::Search => {
            heading(ui, "SEARXNG ADDRESS");
            ui.label(
                RichText::new("Queries go to the active SearXNG first, then DuckDuckGo.")
                    .size(11.0)
                    .color(TEXT_SEC),
            );
            ui.add_space(6.0);
            card(ui, |ui| {
                slot_editor(
                    ui,
                    &mut app.settings.searx_slots,
                    &mut app.settings.searx_active,
                    &mut app.new_searx_slot,
                );
            });
            ui.horizontal(|ui| {
                if settings_button(ui, "Test Connection").clicked() {
                    let ctx = ui.ctx().clone();
                    app.check_searx(&ctx);
                }
                if !app.searx_status.is_empty() {
                    let ok = app.searx_status == "Reachable";
                    ui.label(RichText::new(&app.searx_status).size(12.0).color(if ok {
                        GREEN
                    } else {
                        TEXT_SEC
                    }));
                }
            });
        }

        SettingsTab::Instructions => {
            heading(ui, "CUSTOM INSTRUCTIONS");
            ui.label(
                RichText::new("Applied to every new reply. Saved automatically.")
                    .size(11.0)
                    .color(TEXT_SEC),
            );
            ui.add_space(6.0);
            card(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut app.settings.instructions)
                        .desired_rows(10)
                        .desired_width(f32::INFINITY)
                        .frame(false),
                );
            });
        }

        SettingsTab::Icon => {
            heading(ui, "APP ICON (WINDOWS)");
            card(ui, |ui| {
                ui.label(
                    RichText::new("Exports eAI-Lite.ico with sharp 16–256 px frames, made from your icon image, into Documents/eAI/AppIcon.")
                        .size(12.0)
                        .color(TEXT_SEC),
                );
                ui.add_space(8.0);
                if settings_button(ui, "Export Windows Icon (.ico)").clicked() {
                    app.export_icon();
                }
                if !app.icon_status.is_empty() {
                    ui.label(RichText::new(&app.icon_status).size(12.0).color(TEXT_SEC));
                }
            });
        }

        SettingsTab::Data => {
            heading(ui, "DATA");
            card(ui, |ui| {
                ui.label(
                    RichText::new("Conversations and settings are stored here.")
                        .size(12.0)
                        .color(TEXT_SEC),
                );
                ui.add_space(8.0);
                if settings_button(ui, "Open Profile Folder").clicked() {
                    let _ = std::fs::create_dir_all(data_dir());
                    let _ = open::that(data_dir());
                }
            });
        }
    }
}
