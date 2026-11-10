use eframe::egui;
use image::{codecs::ico::IcoEncoder, ExtendedColorType, RgbaImage};

/// Colours of the logo gradient, from top-left to bottom-right.
const STOPS: [(f32, [f32; 3]); 4] = [
    (0.0, [79.0, 128.0, 255.0]),
    (0.45, [153.0, 77.0, 230.0]),
    (0.8, [230.0, 98.0, 160.0]),
    (1.0, [240.0, 110.0, 150.0]),
];

fn grad(t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    for w in STOPS.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        if t <= t1 {
            let f = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
            return [
                c0[0] + (c1[0] - c0[0]) * f,
                c0[1] + (c1[1] - c0[1]) * f,
                c0[2] + (c1[2] - c0[2]) * f,
            ];
        }
    }
    STOPS[3].1
}

/// Concave four-point sparkle: |x|^p + |y|^p <= 1 with p < 1.
fn sparkle_hit(px: f32, py: f32, r: f32) -> bool {
    let x = px.abs() / r;
    let y = py.abs() / r;
    if x > 1.0 || y > 1.0 {
        return false;
    }
    x.powf(0.55) + y.powf(0.55) <= 1.0
}

/// Draws the logo at any size. Coordinates are scaled from a 1.0 canvas, so the
/// same design works for 16 px and 1024 px. The squircle corners are drawn with
/// a signed distance function, so edges stay smooth at every size.
pub fn render_rgba(n: u32) -> RgbaImage {
    let nf = n as f32;
    let mut img = RgbaImage::new(n, n);
    let radius = nf * 0.2237;
    let half = nf / 2.0;

    // Sparkle centres and arm lengths, as fractions of the canvas.
    let sparkles = [
        (0.537f32, 0.585f32, 0.205f32), // large, lower right
        (0.362f32, 0.450f32, 0.100f32), // medium, left
        (0.482f32, 0.262f32, 0.060f32), // small, top
    ];

    for y in 0..n {
        for x in 0..n {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            let dx = (fx - half).abs() - (half - radius);
            let dy = (fy - half).abs() - (half - radius);
            let d = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - radius;
            let cover = (0.5 - d).clamp(0.0, 1.0);
            if cover <= 0.0 {
                continue;
            }

            let t = (fx + fy) / (2.0 * nf);
            let g = grad(t);
            let hit = sparkles
                .iter()
                .any(|&(cx, cy, r)| sparkle_hit(fx - cx * nf, fy - cy * nf, r * nf));
            let c = if hit { [255.0, 255.0, 255.0] } else { g };

            img.put_pixel(
                x,
                y,
                image::Rgba([c[0] as u8, c[1] as u8, c[2] as u8, (cover * 255.0) as u8]),
            );
        }
    }
    img
}

/// Icon for the window title bar and taskbar. Rendered at 256 px.
pub fn app_icon() -> egui::IconData {
    let img = render_rgba(256);
    egui::IconData {
        rgba: img.into_raw(),
        width: 256,
        height: 256,
    }
}

/// Writes a multi-size Windows .ico (16 to 256 px) into Documents/eAI/AppIcon.
/// Each size is drawn fresh from the logo code, so small sizes stay sharp.
pub fn export_ico() -> Result<std::path::PathBuf, String> {
    let root = crate::data::base_dir().join("AppIcon");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let path = root.join("eAI-Lite.ico");

    let mut frames = Vec::new();
    for s in [16u32, 20, 24, 32, 40, 48, 64, 128, 256] {
        let img = render_rgba(s);
        let frame =
            image::codecs::ico::IcoFrame::as_png(img.as_raw(), s, s, ExtendedColorType::Rgba8)
                .map_err(|e| e.to_string())?;
        frames.push(frame);
    }

    let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    IcoEncoder::new(std::io::BufWriter::new(file))
        .encode_images(&frames)
        .map_err(|e| e.to_string())?;
    Ok(root)
}

/// Writes a macOS .icns-compatible PNG set (16 to 1024 px) into Documents/eAI/AppIcon/mac.
/// Open the folder on a Mac and use Icon Composer or an .iconset tool to build the .icns.
/// Writes a macOS .icns-compatible PNG set (16 to 1024 px) into Documents/eAI/AppIcon/mac.
/// Open the folder on a Mac and use Icon Composer or an .iconset tool to build the .icns.
/// Writes a macOS .icns-compatible PNG set (16 to 1024 px) into Documents/eAI/AppIcon/mac.
/// Open the folder on a Mac and use Icon Composer or an .iconset tool to build the .icns.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn export_mac_pngs() -> Result<std::path::PathBuf, String> {
    let root = crate::data::base_dir().join("AppIcon").join("mac");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    for s in [16u32, 32, 64, 128, 256, 512, 1024] {
        render_rgba(s)
            .save(root.join(format!("icon_{s}x{s}.png")))
            .map_err(|e| e.to_string())?;
    }
    Ok(root)
}
