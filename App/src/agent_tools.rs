use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct ToolResult {
    pub text: String,
    pub image: Option<Vec<u8>>,
    pub sources: Vec<crate::data::Source>,
}

pub fn run(
    name: &str,
    args: &Value,
    root: &Path,
    searx: &str,
    cancel: &Arc<AtomicBool>,
) -> Result<ToolResult, String> {
    let text = |text: String| {
        Ok(ToolResult {
            text,
            image: None,
            sources: Vec::new(),
        })
    };
    match name {
        "list_files" => {
            let dir = resolve(root, args["path"].as_str().unwrap_or("."), false)?;
            let mut names = Vec::new();
            for entry in fs::read_dir(dir).map_err(|e| e.to_string())?.take(200) {
                let entry = entry.map_err(|e| e.to_string())?;
                let kind = if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                    "dir"
                } else {
                    "file"
                };
                names.push(format!("{kind} {}", entry.file_name().to_string_lossy()));
            }
            text(names.join("\n"))
        }
        "read_file" => {
            let path = resolve(root, required(args, "path")?, false)?;
            let data = fs::read(&path).map_err(|e| e.to_string())?;
            if data.len() > 1_000_000 {
                return Err("File exceeds the 1 MB read limit".into());
            }
            text(String::from_utf8_lossy(&data).into_owned())
        }
        "write_file" => {
            let path = resolve(root, required(args, "path")?, true)?;
            let content = required(args, "content")?;
            if content.len() > 2_000_000 {
                return Err("File exceeds the 2 MB write limit".into());
            }
            fs::write(&path, content).map_err(|e| e.to_string())?;
            text(format!(
                "Wrote {} bytes to {}",
                content.len(),
                path.display()
            ))
        }
        "create_directory" => {
            let path = resolve(root, required(args, "path")?, true)?;
            fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            text(format!("Created {}", path.display()))
        }
        "delete_file" => {
            let path = resolve(root, required(args, "path")?, false)?;
            if path.is_dir() {
                return Err("Deleting directories is not supported".into());
            }
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            text(format!("Deleted {}", path.display()))
        }
        "run_command" => run_command(required(args, "command")?, root, cancel),
        "web_search" => {
            let query = required(args, "query")?;
            let results = crate::net::web_search(searx, query, 5)?;
            Ok(ToolResult {
                text: results
                    .iter()
                    .enumerate()
                    .map(|(i, r)| format!("[{}] {}\n{}\n{}", i + 1, r.title, r.url, r.snippet))
                    .collect::<Vec<_>>()
                    .join("\n\n"),
                image: None,
                sources: results,
            })
        }
        "web_open" => {
            let url = required(args, "url")?;
            let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;
            if parsed.scheme() != "https" && parsed.scheme() != "http" {
                return Err("Only HTTP and HTTPS pages can be opened".into());
            }
            if let Some(host) = parsed.host_str() {
                let host_lower = host.to_ascii_lowercase();
                if host_lower == "localhost"
                    || host_lower.ends_with(".localhost")
                    || host_lower.ends_with(".local")
                {
                    return Err("Local network hosts are not available to the web tool".into());
                }
                if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                    let private = match ip {
                        std::net::IpAddr::V4(v4) => {
                            v4.is_private()
                                || v4.is_loopback()
                                || v4.is_link_local()
                                || v4.is_unspecified()
                                || v4.is_broadcast()
                        }
                        std::net::IpAddr::V6(v6) => {
                            v6.is_loopback()
                                || v6.is_unspecified()
                                || v6.is_unique_local()
                                || v6.is_unicast_link_local()
                        }
                    };
                    if private {
                        return Err(
                            "Local network IP addresses are not available to the web tool".into(),
                        );
                    }
                }
            }
            let response = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(25))
                .user_agent("eAI-Lite/1.0")
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|e| e.to_string())?
                .get(parsed)
                .send()
                .map_err(|e| format!("Could not open page: {e}"))?;
            let status = response.status();
            if !status.is_success() {
                return Err(format!("Page returned HTTP {status}"));
            }
            use std::io::Read;
            let mut bytes = Vec::new();
            response
                .take(2_000_000)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            let body = String::from_utf8_lossy(&bytes);
            let text = html_to_text(&body);
            let snippet = text.chars().take(240).collect::<String>();
            Ok(ToolResult {
                text: text.chars().take(30_000).collect(),
                image: None,
                sources: vec![crate::data::Source {
                    title: url.to_string(),
                    url: url.to_string(),
                    snippet,
                }],
            })
        }
        "desktop_screenshot" => desktop::screenshot(),
        "desktop_wait" => desktop_wait(args["milliseconds"].as_u64().unwrap_or(500) as u32, cancel),
        "desktop_click" => desktop::click(
            args["x"].as_i64().ok_or("x coordinate required")? as i32,
            args["y"].as_i64().ok_or("y coordinate required")? as i32,
        ),
        "desktop_scroll" => desktop::scroll(
            args["x"].as_i64().unwrap_or(0) as i32,
            args["y"].as_i64().unwrap_or(0) as i32,
            args["delta"].as_i64().ok_or("delta required")? as i32,
        ),
        "desktop_type" => desktop::type_text(required(args, "text")?),
        "desktop_key" => desktop::press_key(required(args, "key")?),
        "desktop_hotkey" => {
            let keys = args["keys"]
                .as_array()
                .ok_or("keys array required")?
                .iter()
                .map(|k| {
                    k.as_str()
                        .map(str::to_string)
                        .ok_or("key names must be strings".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            desktop::hotkey(&keys)
        }
        _ => Err(format!("Tool is not available: {name}")),
    }
}

fn desktop_wait(milliseconds: u32, cancel: &Arc<AtomicBool>) -> Result<ToolResult, String> {
    let duration = Duration::from_millis(milliseconds.clamp(50, 10_000) as u64);
    let start = Instant::now();
    while start.elapsed() < duration {
        if cancel.load(Ordering::Relaxed) {
            return Err("Desktop wait canceled".into());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(ToolResult {
        text: "Wait finished".into(),
        image: None,
        sources: Vec::new(),
    })
}

fn required<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Missing argument: {key}"))
}

fn html_to_text(html: &str) -> String {
    use std::sync::OnceLock;
    static SCRIPT: OnceLock<regex::Regex> = OnceLock::new();
    static NOSCRIPT: OnceLock<regex::Regex> = OnceLock::new();
    static STYLE: OnceLock<regex::Regex> = OnceLock::new();
    static TAGS: OnceLock<regex::Regex> = OnceLock::new();
    let no_script = SCRIPT
        .get_or_init(|| regex::Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap())
        .replace_all(html, " ");
    let no_script = NOSCRIPT
        .get_or_init(|| regex::Regex::new(r"(?is)<noscript[^>]*>.*?</noscript>").unwrap())
        .replace_all(&no_script, " ");
    let no_style = STYLE
        .get_or_init(|| regex::Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap())
        .replace_all(&no_script, " ");
    let plain = TAGS
        .get_or_init(|| regex::Regex::new(r"(?s)<[^>]+>").unwrap())
        .replace_all(&no_style, " ");
    plain
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn resolve(root: &Path, raw: &str, allow_new: bool) -> Result<PathBuf, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("Invalid workspace folder: {e}"))?;
    let candidate = root.join(raw);
    if candidate.exists() {
        let resolved = candidate.canonicalize().map_err(|e| e.to_string())?;
        return resolved
            .starts_with(&root)
            .then_some(resolved)
            .ok_or_else(|| "Path is outside the configured workspace".into());
    }
    if !allow_new {
        return Err("Path does not exist".into());
    }

    let relative = Path::new(raw);
    if relative
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(
            "New paths must be relative and cannot contain parent or root components".into(),
        );
    }
    let mut ancestor = candidate.parent().ok_or("Invalid path")?;
    let mut missing = Vec::new();
    while !ancestor.exists() {
        missing.push(ancestor.file_name().ok_or("Invalid path")?.to_os_string());
        ancestor = ancestor.parent().ok_or("Invalid path")?;
    }
    let mut resolved = ancestor.canonicalize().map_err(|e| e.to_string())?;
    if !resolved.starts_with(&root) {
        return Err("Path is outside the configured workspace".into());
    }
    for part in missing.into_iter().rev() {
        resolved.push(part);
    }
    resolved.push(relative.file_name().ok_or("Invalid path")?);
    Ok(resolved)
}

fn run_command(command: &str, root: &Path, cancel: &Arc<AtomicBool>) -> Result<ToolResult, String> {
    let mut child = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            command,
        ])
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start PowerShell: {e}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Could not capture command output")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("Could not capture command errors")?;
    let out_thread = thread::spawn(move || {
        let b = read_bounded(stdout, 128_000);
        b
    });
    let err_thread = thread::spawn(move || {
        let b = read_bounded(stderr, 128_000);
        b
    });
    let start = Instant::now();
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            return Err("Command canceled".into());
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(120) {
            let _ = child.kill();
            return Err("Command timed out after 120 seconds".into());
        }
        thread::sleep(Duration::from_millis(100));
    };
    let stdout = String::from_utf8_lossy(&out_thread.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&err_thread.join().unwrap_or_default()).into_owned();
    Ok(ToolResult {
        text: format!(
            "exit code: {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            status.code().unwrap_or(-1)
        ),
        image: None,
        sources: Vec::new(),
    })
}

fn read_bounded<R: std::io::Read>(mut reader: R, limit: usize) -> Vec<u8> {
    let mut stored = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let remaining = limit.saturating_sub(stored.len());
                stored.extend_from_slice(&buffer[..n.min(remaining)]);
            }
        }
    }
    stored
}

#[cfg(target_os = "windows")]
mod desktop {
    use super::ToolResult;
    use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
    use std::io::Cursor;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    pub fn screenshot() -> Result<ToolResult, String> {
        unsafe {
            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);
            if width < 1 || height < 1 {
                return Err("Could not read desktop size".into());
            }
            let screen = GetDC(std::ptr::null_mut());
            if screen.is_null() {
                return Err("Could not access the desktop".into());
            }
            let memory = CreateCompatibleDC(screen);
            if memory.is_null() {
                ReleaseDC(std::ptr::null_mut(), screen);
                return Err("Could not create a desktop capture surface".into());
            }
            let bitmap = CreateCompatibleBitmap(screen, width, height);
            if bitmap.is_null() {
                DeleteDC(memory);
                ReleaseDC(std::ptr::null_mut(), screen);
                return Err("Could not capture desktop".into());
            }
            let old = SelectObject(memory, bitmap as _);
            let copied = BitBlt(memory, 0, 0, width, height, screen, 0, 0, SRCCOPY);
            let mut info = BITMAPINFO::default();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = width;
            info.bmiHeader.biHeight = -height;
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            info.bmiHeader.biCompression = BI_RGB;
            let mut pixels = vec![0u8; width as usize * height as usize * 4];
            SelectObject(memory, old);
            let read = if copied != 0 {
                GetDIBits(
                    memory,
                    bitmap,
                    0,
                    height as u32,
                    pixels.as_mut_ptr().cast(),
                    &mut info,
                    DIB_RGB_COLORS,
                )
            } else {
                0
            };
            DeleteObject(bitmap as _);
            DeleteDC(memory);
            ReleaseDC(std::ptr::null_mut(), screen);
            if read == 0 {
                return Err("Could not read desktop pixels".into());
            }
            for px in pixels.chunks_exact_mut(4) {
                px.swap(0, 2);
                px[3] = 255;
            }
            let image: ImageBuffer<Rgba<u8>, _> =
                ImageBuffer::from_raw(width as u32, height as u32, pixels)
                    .ok_or("Invalid desktop image")?;
            let mut out = Cursor::new(Vec::new());
            DynamicImage::ImageRgba8(image)
                .write_to(&mut out, ImageFormat::Jpeg)
                .map_err(|e| e.to_string())?;
            Ok(ToolResult {
                text: format!("Desktop screenshot, {width} by {height} pixels"),
                image: Some(out.into_inner()),
                sources: Vec::new(),
            })
        }
    }

    pub fn click(x: i32, y: i32) -> Result<ToolResult, String> {
        unsafe {
            let w = GetSystemMetrics(SM_CXSCREEN).max(1);
            let h = GetSystemMetrics(SM_CYSCREEN).max(1);
            if x < 0 || y < 0 || x >= w || y >= h {
                return Err("Click coordinates are outside the screen".into());
            }
            let inputs = [
                mouse_input(
                    x * 65535 / w,
                    y * 65535 / h,
                    MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                ),
                mouse_input(0, 0, MOUSEEVENTF_LEFTDOWN),
                mouse_input(0, 0, MOUSEEVENTF_LEFTUP),
            ];
            if SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            ) != inputs.len() as u32
            {
                return Err("Desktop click was blocked".into());
            }
            Ok(ToolResult {
                text: format!("Clicked at ({x}, {y})"),
                image: None,
                sources: Vec::new(),
            })
        }
    }

    pub fn scroll(x: i32, y: i32, delta: i32) -> Result<ToolResult, String> {
        unsafe {
            let w = GetSystemMetrics(SM_CXSCREEN).max(1);
            let h = GetSystemMetrics(SM_CYSCREEN).max(1);
            if x < 0 || y < 0 || x >= w || y >= h {
                return Err("Scroll coordinates are outside the screen".into());
            }
            let normalized = [
                mouse_input(
                    x * 65535 / w,
                    y * 65535 / h,
                    MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                ),
                INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            mouseData: delta as u32,
                            dwFlags: MOUSEEVENTF_WHEEL,
                            ..Default::default()
                        },
                    },
                },
            ];
            if SendInput(2, normalized.as_ptr(), std::mem::size_of::<INPUT>() as i32) != 2 {
                return Err("Desktop scroll was blocked".into());
            }
            Ok(ToolResult {
                text: format!("Scrolled at ({x}, {y})"),
                image: None,
                sources: Vec::new(),
            })
        }
    }

    pub fn type_text(text: &str) -> Result<ToolResult, String> {
        if text.len() > 4000 {
            return Err("Desktop text is limited to 4,000 bytes per action".into());
        }
        let mut inputs = Vec::new();
        for unit in text.encode_utf16() {
            inputs.push(key_input(unit, KEYEVENTF_UNICODE));
            inputs.push(key_input(unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
        }
        unsafe {
            if SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            ) != inputs.len() as u32
            {
                return Err("Desktop typing was blocked".into());
            }
        }
        Ok(ToolResult {
            text: format!("Typed {} characters", text.chars().count()),
            image: None,
            sources: Vec::new(),
        })
    }

    pub fn press_key(key: &str) -> Result<ToolResult, String> {
        let vk = key_to_vk(key)?;
        let inputs = [vk_input(vk, 0), vk_input(vk, KEYEVENTF_KEYUP)];
        unsafe {
            if SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) != 2 {
                return Err("Desktop key was blocked".into());
            }
        }
        Ok(ToolResult {
            text: format!("Pressed {key}"),
            image: None,
            sources: Vec::new(),
        })
    }

    pub fn hotkey(keys: &[String]) -> Result<ToolResult, String> {
        if keys.is_empty() || keys.len() > 4 {
            return Err("Provide one to four keys".into());
        }
        let vks = keys
            .iter()
            .map(|k| key_to_vk(k))
            .collect::<Result<Vec<_>, _>>()?;
        let mut inputs: Vec<INPUT> = vks.iter().map(|vk| vk_input(*vk, 0)).collect();
        inputs.extend(vks.iter().rev().map(|vk| vk_input(*vk, KEYEVENTF_KEYUP)));
        unsafe {
            if SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            ) != inputs.len() as u32
            {
                return Err("Desktop shortcut was blocked".into());
            }
        }
        Ok(ToolResult {
            text: format!("Pressed {}", keys.join("+")),
            image: None,
            sources: Vec::new(),
        })
    }

    fn key_to_vk(key: &str) -> Result<VIRTUAL_KEY, String> {
        Ok(match key.to_lowercase().as_str() {
            "enter" => VK_RETURN,
            "tab" => VK_TAB,
            "escape" | "esc" => VK_ESCAPE,
            "backspace" => VK_BACK,
            "delete" => VK_DELETE,
            "space" => VK_SPACE,
            "up" => VK_UP,
            "down" => VK_DOWN,
            "left" => VK_LEFT,
            "right" => VK_RIGHT,
            "home" => VK_HOME,
            "end" => VK_END,
            "ctrl" | "control" => VK_CONTROL,
            "shift" => VK_SHIFT,
            "alt" => VK_MENU,
            "win" => VK_LWIN,
            _ if key.len() == 1 => key.to_ascii_uppercase().as_bytes()[0] as u16,
            _ => return Err(format!("Unsupported key name: {key}")),
        })
    }

    unsafe fn mouse_input(dx: i32, dy: i32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        }
    }
    fn key_input(scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wScan: scan,
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        }
    }
    fn vk_input(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod desktop {
    use super::ToolResult;
    pub fn screenshot() -> Result<ToolResult, String> {
        Err("Desktop control is currently supported on Windows only".into())
    }
    pub fn click(_: i32, _: i32) -> Result<ToolResult, String> {
        Err("Desktop control is currently supported on Windows only".into())
    }
    pub fn scroll(_: i32, _: i32, _: i32) -> Result<ToolResult, String> {
        Err("Desktop control is currently supported on Windows only".into())
    }
    pub fn type_text(_: &str) -> Result<ToolResult, String> {
        Err("Desktop control is currently supported on Windows only".into())
    }
    pub fn press_key(_: &str) -> Result<ToolResult, String> {
        Err("Desktop control is currently supported on Windows only".into())
    }
    pub fn hotkey(_: &[String]) -> Result<ToolResult, String> {
        Err("Desktop control is currently supported on Windows only".into())
    }
}
