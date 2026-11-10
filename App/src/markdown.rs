use crate::draw::{grad_static, ACCENT, CARD, TEXT, TEXT_SEC};
use eframe::egui::{
    self,
    text::{LayoutJob, TextFormat},
    Color32, FontFamily, FontId, Frame, Margin, Rounding, ScrollArea, Sense, Ui, Vec2,
};

pub enum Block {
    Text(String),
    Code(String, String),
    Table(Vec<String>, Vec<Vec<String>>),
}

pub fn render(ui: &mut Ui, raw: &str, salt: u64) {
    for (i, block) in parse(raw).into_iter().enumerate() {
        match block {
            Block::Text(t) => prose(ui, &t),
            Block::Code(code, lang) => code_block(ui, &code, &lang, salt ^ i as u64),
            Block::Table(h, rows) => table(ui, &h, &rows, salt ^ i as u64),
        }
    }
}

fn parse(raw: &str) -> Vec<Block> {
    let lines: Vec<&str> = raw.lines().collect();
    let mut blocks = Vec::new();
    let mut prose: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if let Some(lang) = t.strip_prefix("```") {
            if !prose.is_empty() {
                blocks.push(Block::Text(prose.join("\n")));
                prose.clear();
            }
            let mut code = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                code.push(lines[i]);
                i += 1;
            }
            let lang = lang.trim();
            let lang = if lang.is_empty() { "code" } else { lang };
            blocks.push(Block::Code(code.join("\n"), lang.to_string()));
            i += 1;
            continue;
        }
        if is_table_row(t) && i + 1 < lines.len() && is_separator(lines[i + 1]) {
            if !prose.is_empty() {
                blocks.push(Block::Text(prose.join("\n")));
                prose.clear();
            }
            let header = split_row(t);
            let mut rows = Vec::new();
            i += 2;
            while i < lines.len() && is_table_row(lines[i].trim()) {
                rows.push(split_row(lines[i].trim()));
                i += 1;
            }
            blocks.push(Block::Table(header, rows));
            continue;
        }
        prose.push(lines[i]);
        i += 1;
    }
    if !prose.is_empty() {
        blocks.push(Block::Text(prose.join("\n")));
    }
    blocks
}

fn is_table_row(s: &str) -> bool {
    s.contains('|') && s.matches('|').count() >= 2
}

fn is_separator(s: &str) -> bool {
    let t = s.trim();
    t.contains('-') && t.contains('|') && t.chars().all(|c| "|-: ".contains(c))
}

fn split_row(s: &str) -> Vec<String> {
    let t = s.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(|c| c.trim().to_string()).collect()
}

/// Removes leftover markdown marks (`*` and `#`) from the edges of a heading.
fn clean_heading(s: &str) -> &str {
    s.trim().trim_matches(|c| c == '*' || c == '#').trim()
}

/// `#`, `##`, `###`, `####` headings. Returns (level, clean text).
fn hash_heading(t: &str) -> Option<(usize, &str)> {
    let level = t.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let text = clean_heading(&t[level..]);
    if text.is_empty() {
        return None;
    }
    Some((level, text))
}

/// A line that is only bold, like `**1. UI Capabilities**`, is treated as a heading.
fn bold_heading(t: &str) -> Option<&str> {
    let inner = t.strip_prefix("**")?.strip_suffix("**")?;
    if inner.is_empty() || inner.contains("**") {
        return None;
    }
    Some(inner.trim())
}

fn prose(ui: &mut Ui, text: &str) {
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            ui.add_space(6.0);
        } else if let Some((level, r)) = hash_heading(t) {
            let size = match level {
                1 => 22.0,
                2 => 18.0,
                3 => 16.0,
                _ => 15.0,
            };
            heading(ui, r, size);
        } else if let Some(r) = bold_heading(t) {
            heading(ui, r, 16.0);
        } else if t == "---" || t == "***" {
            ui.separator();
        } else if let Some(r) = t.strip_prefix("> ") {
            ui.horizontal_top(|ui| {
                let (bar, _) = ui.allocate_exact_size(Vec2::new(3.0, 18.0), Sense::hover());
                ui.painter().rect_filled(bar, Rounding::same(1.5), ACCENT);
                ui.vertical(|ui| {
                    ui.label(job_text(r, 14.0, false, TEXT_SEC));
                });
            });
        } else if let Some(r) = bullet(t) {
            ui.horizontal_top(|ui| {
                let (d, _) = ui.allocate_exact_size(Vec2::new(16.0, 20.0), Sense::hover());
                ui.painter()
                    .circle_filled(egui::pos2(d.left() + 5.0, d.center().y), 2.8, TEXT_SEC);
                ui.vertical(|ui| {
                    ui.label(job_text(r, 14.0, false, TEXT));
                });
            });
        } else if let Some((n, r)) = numbered(t) {
            ui.horizontal_top(|ui| {
                ui.label(job_text(n, 14.0, true, TEXT_SEC));
                ui.vertical(|ui| {
                    ui.label(job_text(r, 14.0, false, TEXT));
                });
            });
        } else {
            ui.label(job_text(t, 14.0, false, TEXT));
        }
    }
}

/// Headings: a fixed (static) blue, purple, pink gradient across the letters. No animation.
fn heading(ui: &mut Ui, text: &str, size: f32) {
    ui.add_space(6.0);
    let galley = ui.fonts(|f| f.layout_job(gradient_heading(text, size)));
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, TEXT);
}

fn gradient_heading(text: &str, size: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let n = (text.chars().count().max(2) - 1) as f32;
    for (i, c) in text.chars().enumerate() {
        job.append(
            &c.to_string(),
            0.0,
            TextFormat {
                font_id: FontId::new(size, FontFamily::Name("Bold".into())),
                color: grad_static(i as f32 / n),
                ..Default::default()
            },
        );
    }
    job
}

fn bullet(t: &str) -> Option<&str> {
    t.strip_prefix("- ")
        .or_else(|| t.strip_prefix("* "))
        .or_else(|| t.strip_prefix("• "))
}

fn numbered(t: &str) -> Option<(&str, &str)> {
    let d = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if d == 0 || d > 3 {
        return None;
    }
    let rest = &t[d..];
    if rest.starts_with(". ") || rest.starts_with(") ") {
        Some((&t[..d + 1], &rest[2..]))
    } else {
        None
    }
}

/// Inline formatting: **bold**, `code`, and [n] citations in blue.
pub fn job_text(text: &str, size: f32, strong: bool, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut buf = String::new();
    let mut bold = false;
    let mut code = false;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if !code && c == '*' && chars.get(i + 1) == Some(&'*') {
            flush(&mut job, &mut buf, size, bold, code, strong, color);
            bold = !bold;
            i += 2;
            continue;
        }
        if c == '`' {
            flush(&mut job, &mut buf, size, bold, code, strong, color);
            code = !code;
            i += 1;
            continue;
        }
        if !code && c == '[' {
            let mut j = i + 1;
            let mut num = String::new();
            while j < chars.len() && chars[j].is_ascii_digit() && num.len() < 2 {
                num.push(chars[j]);
                j += 1;
            }
            if !num.is_empty() && chars.get(j) == Some(&']') {
                flush(&mut job, &mut buf, size, bold, code, strong, color);
                job.append(
                    &format!("[{num}]"),
                    0.0,
                    TextFormat {
                        font_id: FontId::new(size - 2.0, FontFamily::Proportional),
                        color: ACCENT,
                        ..Default::default()
                    },
                );
                i = j + 1;
                continue;
            }
        }
        buf.push(c);
        i += 1;
    }
    flush(&mut job, &mut buf, size, bold, code, strong, color);
    job
}

#[allow(clippy::too_many_arguments)]
fn flush(
    job: &mut LayoutJob,
    buf: &mut String,
    size: f32,
    bold: bool,
    code: bool,
    strong: bool,
    color: Color32,
) {
    if buf.is_empty() {
        return;
    }
    let format = if code {
        TextFormat {
            font_id: FontId::monospace(size - 1.0),
            color: Color32::from_rgb(255, 190, 150),
            background: Color32::from_white_alpha(18),
            ..Default::default()
        }
    } else {
        let font = if bold || strong {
            FontId::new(size, FontFamily::Name("Bold".into()))
        } else {
            FontId::proportional(size)
        };
        TextFormat {
            font_id: font,
            color,
            ..Default::default()
        }
    };
    job.append(buf, 0.0, format);
    buf.clear();
}

fn code_block(ui: &mut Ui, code: &str, lang: &str, salt: u64) {
    Frame::none()
        .fill(Color32::from_rgb(22, 22, 26))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(lang.to_uppercase())
                        .size(10.0)
                        .color(TEXT_SEC)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Copy").clicked() {
                        ui.ctx().copy_text(code.to_string());
                    }
                });
            });
            ui.add_space(4.0);
            ScrollArea::horizontal()
                .id_salt(("code", salt))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(code.trim_end())
                            .monospace()
                            .size(13.0)
                            .color(Color32::from_rgb(215, 225, 255)),
                    );
                });
        });
    ui.add_space(6.0);
}

fn table(ui: &mut Ui, header: &[String], rows: &[Vec<String>], salt: u64) {
    Frame::none()
        .fill(CARD)
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ScrollArea::horizontal()
                .id_salt(("table", salt))
                .show(ui, |ui| {
                    egui::Grid::new(("grid", salt))
                        .striped(true)
                        .spacing([18.0, 6.0])
                        .show(ui, |ui| {
                            for h in header {
                                ui.label(job_text(h, 13.0, true, TEXT));
                            }
                            ui.end_row();
                            for r in rows {
                                for i in 0..header.len() {
                                    let cell = r.get(i).map(|s| s.as_str()).unwrap_or("");
                                    ui.label(job_text(cell, 13.0, false, TEXT));
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    ui.add_space(6.0);
}
