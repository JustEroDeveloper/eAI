use eframe::egui::{
    text::{LayoutJob, TextFormat},
    Align2, Color32, FontFamily, FontId, Mesh, Painter, Pos2, Rect, Rounding, Shape, Stroke, Vec2,
};
use std::f32::consts::TAU;

pub const ACCENT: Color32 = Color32::from_rgb(79, 140, 255);
pub const WINDOW_BG: Color32 = Color32::from_rgb(16, 16, 18);
pub const SIDEBAR_BG: Color32 = Color32::from_rgb(22, 22, 26);
pub const CARD: Color32 = Color32::from_rgb(28, 28, 32);
pub const CARD_HI: Color32 = Color32::from_rgb(40, 40, 46);
pub const INPUT_BG: Color32 = Color32::from_rgb(28, 28, 32);
pub const TEXT: Color32 = Color32::from_rgb(245, 245, 247);
pub const TEXT_SEC: Color32 = Color32::from_rgb(152, 152, 157);
pub const TEXT_TER: Color32 = Color32::from_rgb(99, 99, 102);
pub const RED: Color32 = Color32::from_rgb(255, 69, 58);
pub const GREEN: Color32 = Color32::from_rgb(48, 209, 88);

pub fn select() -> Color32 {
    Color32::from_rgba_unmultiplied(10, 132, 255, 70)
}

const STOPS: [Color32; 4] = [
    Color32::from_rgb(64, 128, 255),
    Color32::from_rgb(153, 77, 230),
    Color32::from_rgb(242, 102, 153),
    Color32::from_rgb(64, 128, 255),
];

pub fn st(width: f32, color: Color32) -> Stroke {
    Stroke::new(width, color)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

/// Looping blue, purple, pink gradient. `t` wraps every 1.0.
pub fn grad_at(t: f32) -> Color32 {
    let x = t.rem_euclid(1.0) * 3.0;
    let i = (x.floor() as usize).min(2);
    let f = x - i as f32;
    let s = f * f * (3.0 - 2.0 * f);
    mix(STOPS[i], STOPS[i + 1], s)
}

/// Gradient clamped to 0..1.
pub fn grad_static(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        mix(STOPS[0], STOPS[1], t * 2.0)
    } else {
        mix(STOPS[1], STOPS[2], (t - 0.5) * 2.0)
    }
}

/// Text where each letter drifts through the gradient.
pub fn gradient_job(text: &str, size: f32, time: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let n = (text.chars().count().max(2) - 1) as f32;
    for (i, c) in text.chars().enumerate() {
        job.append(
            &c.to_string(),
            0.0,
            TextFormat {
                font_id: FontId::new(size, FontFamily::Name("Bold".into())),
                color: grad_at(i as f32 / n - time / 4.0),
                ..Default::default()
            },
        );
    }
    job
}

fn star(p: &Painter, center: Pos2, r: f32, color_at: &dyn Fn(Pos2) -> Color32) {
    let n = 64usize;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color_at(center));
    for i in 0..n {
        let a = i as f32 / n as f32 * TAU;
        let pt = Pos2::new(
            center.x + r * a.cos().powi(3),
            center.y - r * a.sin().powi(3),
        );
        mesh.colored_vertex(pt, color_at(pt));
    }
    for i in 0..n {
        mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    p.add(Shape::mesh(mesh));
}

/// The SF Symbols "sparkles" glyph: one large four-point star and two smaller ones.
pub fn sparkles(p: &Painter, c: Pos2, u: f32, alpha: f32, solid: Option<Color32>, pulse: f32) {
    let u = u * (1.0 + 0.05 * pulse);
    let color_at = move |pt: Pos2| -> Color32 {
        let col = match solid {
            Some(s) => s,
            None => grad_static(0.5 + ((pt.x - c.x) + (pt.y - c.y)) / (u * 5.0)),
        };
        col.gamma_multiply(alpha)
    };
    for (dx, dy, k) in [
        (0.09f32, 0.30f32, 1.0f32),
        (-0.67, -0.46, 0.41),
        (-0.09, -1.0, 0.24),
    ] {
        star(p, Pos2::new(c.x + dx * u, c.y + dy * u), u * k, &color_at);
    }
}

pub fn gradient_disc(p: &Painter, c: Pos2, r: f32) {
    p.circle_filled(c, r, grad_static(0.5));
    let n = 48usize;
    let rr = (r - 0.75).max(1.0);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(c, grad_static(0.5));
    for i in 0..n {
        let a = i as f32 / n as f32 * TAU;
        let (s, co) = a.sin_cos();
        let t = 0.5 + 0.5 * (co + s) * std::f32::consts::FRAC_1_SQRT_2;
        mesh.colored_vertex(Pos2::new(c.x + rr * co, c.y + rr * s), grad_static(t));
    }
    for i in 0..n {
        mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    p.add(Shape::mesh(mesh));
}

pub fn avatar_circle(p: &Painter, c: Pos2, size: f32, initial: &str) {
    gradient_disc(p, c, size / 2.0);
    p.text(
        c,
        Align2::CENTER_CENTER,
        initial,
        FontId::proportional(size * 0.45),
        Color32::WHITE,
    );
}

pub fn initial_of(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into())
}

pub fn greeting_for_hour(h: u32) -> &'static str {
    match h {
        5..=11 => "Good morning",
        12..=16 => "Good afternoon",
        17..=21 => "Good evening",
        _ => "Hello",
    }
}

// ----- Icons (drawn with the painter so they stay crisp at any size) -----

pub fn icon_plus(p: &Painter, c: Pos2, col: Color32) {
    let s = 5.0;
    p.line_segment(
        [Pos2::new(c.x - s, c.y), Pos2::new(c.x + s, c.y)],
        st(1.6, col),
    );
    p.line_segment(
        [Pos2::new(c.x, c.y - s), Pos2::new(c.x, c.y + s)],
        st(1.6, col),
    );
}

pub fn icon_list(p: &Painter, c: Pos2, col: Color32) {
    let s = st(1.5, col);
    for y in [-4.0, 0.0, 4.0] {
        p.circle_filled(Pos2::new(c.x - 5.0, c.y + y), 1.0, col);
        p.line_segment(
            [Pos2::new(c.x - 1.5, c.y + y), Pos2::new(c.x + 6.0, c.y + y)],
            s,
        );
    }
}

pub fn icon_search(p: &Painter, c: Pos2, col: Color32) {
    p.circle_stroke(Pos2::new(c.x - 1.0, c.y - 1.0), 5.0, st(1.5, col));
    p.line_segment(
        [
            Pos2::new(c.x + 3.0, c.y + 3.0),
            Pos2::new(c.x + 7.0, c.y + 7.0),
        ],
        st(1.5, col),
    );
}

pub fn icon_filter(p: &Painter, c: Pos2, col: Color32) {
    let s = st(1.5, col);
    p.line_segment(
        [
            Pos2::new(c.x - 7.0, c.y - 5.0),
            Pos2::new(c.x + 7.0, c.y - 5.0),
        ],
        s,
    );
    p.line_segment(
        [Pos2::new(c.x - 7.0, c.y - 5.0), Pos2::new(c.x - 2.0, c.y)],
        s,
    );
    p.line_segment(
        [Pos2::new(c.x + 7.0, c.y - 5.0), Pos2::new(c.x + 2.0, c.y)],
        s,
    );
    p.line_segment(
        [Pos2::new(c.x - 2.0, c.y), Pos2::new(c.x - 2.0, c.y + 6.0)],
        s,
    );
    p.line_segment(
        [
            Pos2::new(c.x - 2.0, c.y + 6.0),
            Pos2::new(c.x + 2.0, c.y + 4.0),
        ],
        s,
    );
    p.line_segment(
        [Pos2::new(c.x + 2.0, c.y + 4.0), Pos2::new(c.x + 2.0, c.y)],
        s,
    );
}

pub fn icon_close(p: &Painter, c: Pos2, col: Color32) {
    p.line_segment(
        [
            Pos2::new(c.x - 4.0, c.y - 4.0),
            Pos2::new(c.x + 4.0, c.y + 4.0),
        ],
        st(1.5, col),
    );
    p.line_segment(
        [
            Pos2::new(c.x + 4.0, c.y - 4.0),
            Pos2::new(c.x - 4.0, c.y + 4.0),
        ],
        st(1.5, col),
    );
}

pub fn icon_arrow_up(p: &Painter, c: Pos2, col: Color32) {
    let s = 6.5;
    let tip = Pos2::new(c.x, c.y - s);
    p.line_segment([Pos2::new(c.x, c.y + s), tip], st(2.0, col));
    p.line_segment([tip, Pos2::new(c.x - s, c.y - 0.5)], st(2.0, col));
    p.line_segment([tip, Pos2::new(c.x + s, c.y - 0.5)], st(2.0, col));
}

pub fn icon_stop(p: &Painter, c: Pos2, col: Color32) {
    p.rect_filled(
        Rect::from_center_size(c, Vec2::splat(10.0)),
        Rounding::same(2.0),
        col,
    );
}

pub fn icon_sidebar(p: &Painter, c: Pos2, col: Color32) {
    let body = Rect::from_center_size(c, Vec2::new(16.0, 13.0));
    p.rect_stroke(body, Rounding::same(3.0), st(1.4, col));
    let x = body.left() + 5.5;
    p.line_segment(
        [Pos2::new(x, body.top()), Pos2::new(x, body.bottom())],
        st(1.4, col),
    );
}

pub fn icon_compose(p: &Painter, c: Pos2, col: Color32) {
    let sq = Rect::from_center_size(Pos2::new(c.x - 1.0, c.y + 1.0), Vec2::splat(12.0));
    p.rect_stroke(sq, Rounding::same(2.5), st(1.3, col));
    p.line_segment(
        [
            Pos2::new(c.x - 1.0, c.y + 1.0),
            Pos2::new(c.x + 6.0, c.y - 6.0),
        ],
        st(1.6, col),
    );
}

pub fn icon_gear(p: &Painter, c: Pos2, col: Color32) {
    p.circle_stroke(c, 4.0, st(1.4, col));
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        let (s, co) = a.sin_cos();
        p.line_segment(
            [
                Pos2::new(c.x + co * 5.5, c.y + s * 5.5),
                Pos2::new(c.x + co * 7.0, c.y + s * 7.0),
            ],
            st(1.6, col),
        );
    }
}

pub fn icon_robot(p: &Painter, c: Pos2, col: Color32) {
    let antenna_top = Pos2::new(c.x, c.y - 8.0);
    p.line_segment([Pos2::new(c.x, c.y - 5.5), antenna_top], st(1.4, col));
    p.circle_filled(antenna_top, 1.4, col);
    let head = Rect::from_center_size(Pos2::new(c.x, c.y + 0.5), Vec2::new(14.0, 11.0));
    p.rect_stroke(head, Rounding::same(3.0), st(1.5, col));
    p.circle_filled(Pos2::new(c.x - 3.2, c.y), 1.1, col);
    p.circle_filled(Pos2::new(c.x + 3.2, c.y), 1.1, col);
    p.line_segment(
        [
            Pos2::new(c.x - 2.5, c.y + 4.0),
            Pos2::new(c.x + 2.5, c.y + 4.0),
        ],
        st(1.3, col),
    );
}

pub fn icon_check(p: &Painter, c: Pos2, col: Color32) {
    p.line_segment(
        [Pos2::new(c.x - 3.5, c.y), Pos2::new(c.x - 1.0, c.y + 3.0)],
        st(1.6, col),
    );
    p.line_segment(
        [
            Pos2::new(c.x - 1.0, c.y + 3.0),
            Pos2::new(c.x + 3.5, c.y - 2.5),
        ],
        st(1.6, col),
    );
}

pub fn chip_icon(p: &Painter, kind: usize, c: Pos2, col: Color32) {
    let s = st(1.2, col);
    let pt = |dx: f32, dy: f32| Pos2::new(c.x + dx, c.y + dy);
    match kind {
        0 => {
            p.circle_stroke(pt(0.0, -1.5), 4.5, s);
            p.line_segment([pt(-2.0, 4.2), pt(2.0, 4.2)], s);
            p.line_segment([pt(-1.4, 6.2), pt(1.4, 6.2)], s);
        }
        1 => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(10.0, 13.0)),
                Rounding::same(2.0),
                s,
            );
            p.line_segment([pt(-2.5, -1.0), pt(2.5, -1.0)], s);
            p.line_segment([pt(-2.5, 2.0), pt(2.5, 2.0)], s);
        }
        2 => {
            p.line_segment([pt(-2.5, -4.0), pt(-6.0, 0.0)], s);
            p.line_segment([pt(-6.0, 0.0), pt(-2.5, 4.0)], s);
            p.line_segment([pt(2.5, -4.0), pt(6.0, 0.0)], s);
            p.line_segment([pt(6.0, 0.0), pt(2.5, 4.0)], s);
        }
        _ => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(10.0, 13.0)),
                Rounding::same(2.0),
                s,
            );
            p.line_segment([pt(-2.5, -6.5), pt(-2.5, 6.5)], s);
        }
    }
}
