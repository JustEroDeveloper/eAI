use crate::data::{fallback_title, split_think, supports_thinking, Message};
use crate::net;
use crate::Event;
use base64::Engine;
use eframe::egui;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc};
use std::time::Duration;
use uuid::Uuid;

pub struct Job {
    pub ctx: egui::Context,
    pub tx: Sender<Event>,
    pub cancel: Arc<AtomicBool>,
    pub session: Uuid,
    pub message: Uuid,
    pub host: String,
    pub searx: String,
    pub use_search: bool,
    pub query: String,
    pub model: String,
    pub system: String,
    /// Full chat so far, ending with the new user message.
    pub history: Vec<Message>,
    pub think: bool,
    pub max_tokens: u32,
    pub needs_title: bool,
}

pub fn spawn(job: Job) {
    std::thread::spawn(move || run(job));
}

fn emit(job: &Job, ev: Event) {
    let _ = job.tx.send(ev);
    job.ctx.request_repaint();
}

fn run(job: Job) {
    let outcome = turn(&job);
    if let Err(text) = &outcome {
        emit(
            &job,
            Event::Error {
                session: job.session,
                message: job.message,
                text: text.clone(),
            },
        );
    }
    emit(
        &job,
        Event::Finished {
            session: job.session,
            message: job.message,
        },
    );
    if outcome.is_ok() && job.needs_title {
        if let Some(title) = make_title(&job) {
            emit(
                &job,
                Event::TitleReady {
                    session: job.session,
                    title,
                },
            );
        }
    }
}

fn turn(job: &Job) -> Result<(), String> {
    let last = job
        .history
        .last()
        .ok_or_else(|| "Empty conversation".to_string())?;

    if last.image.is_some() && !net::model_supports_vision(&job.host, &job.model) {
        return Err(format!(
            "{} can't read images. Pick a vision model, such as qwen2.5vl:7b, gemma3:4b, or llava. Install one with `ollama pull qwen2.5vl:7b`.",
            job.model
        ));
    }

    // Web context for the newest question.
    let mut context = String::new();
    if job.use_search {
        if let Ok(list) = net::web_search(&job.searx, &job.query, 4) {
            if !list.is_empty() {
                context = format!(
                    "Web search results for current data ({}):\n",
                    chrono::Local::now().format("%B %e, %Y")
                );
                for (i, s) in list.iter().enumerate() {
                    context.push_str(&format!(
                        "[{}] Title: {} | URL: {}\nSnippet: {}\n\n",
                        i + 1,
                        s.title,
                        s.url,
                        s.snippet
                    ));
                }
                emit(
                    job,
                    Event::Sources {
                        session: job.session,
                        message: job.message,
                        sources: list,
                    },
                );
            }
        }
    }

    let mut messages: Vec<Value> = vec![json!({ "role": "system", "content": job.system })];
    let n = job.history.len();
    for (i, m) in job.history.iter().enumerate().skip(n.saturating_sub(9)) {
        let is_last = i == n - 1;
        let mut content = m.content.clone();
        if let Some(att) = &m.attachment {
            content = format!(
                "{content}\n\nAttached file `{}`:\n```\n{}\n```",
                att.name, att.content
            );
        }
        if is_last && !context.is_empty() {
            content = format!("{context}\nQuestion: {content}");
        }
        let mut entry = json!({ "role": m.role, "content": content });
        if is_last {
            if let Some(img) = &m.image {
                entry["images"] = json!([base64::engine::general_purpose::STANDARD.encode(img)]);
            }
        }
        messages.push(entry);
    }

    stream(job, &messages)?;
    Ok(())
}

/// One streamed call to Ollama. Each delta goes to the UI as it arrives.
fn stream(job: &Job, messages: &[Value]) -> Result<(), String> {
    let mut body = json!({
        "model": job.model,
        "messages": messages,
        "stream": true,
        "keep_alive": "30m",
        "options": {
            "temperature": 0.6,
            "top_p": 0.95,
            "top_k": 20,
            "num_ctx": 8192,
            "num_predict": job.max_tokens
        }
    });
    if supports_thinking(&job.model) {
        body["think"] = json!(job.think);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(900))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{}/api/chat", job.host))
        .json(&body)
        .send()
        .map_err(|e| format!("Could not reach Ollama at {}: {e}", job.host))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().unwrap_or_default();
        let detail = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v["error"].as_str().map(String::from))
            .unwrap_or_else(|| "no details returned".into());
        return Err(format!(
            "Ollama error (HTTP {status}): {detail}. Check that the model '{}' is installed.",
            job.model
        ));
    }

    for line in BufReader::new(resp).lines() {
        if job.cancel.load(Ordering::Relaxed) {
            break;
        }
        let line = line.map_err(|e| e.to_string())?;
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let msg = &v["message"];
        let c = msg["content"].as_str().unwrap_or("").to_string();
        let t = msg["thinking"].as_str().unwrap_or("").to_string();
        if !c.is_empty() || !t.is_empty() {
            emit(
                job,
                Event::Chunk {
                    session: job.session,
                    message: job.message,
                    content: c,
                    thinking: t,
                },
            );
        }
        if v["done"].as_bool() == Some(true) {
            break;
        }
    }
    Ok(())
}

/// A short title from the first exchange. Falls back to the opening words.
fn make_title(job: &Job) -> Option<String> {
    let user = job
        .history
        .iter()
        .find(|m| m.role == "user")?
        .content
        .clone();
    let prompt: String = user.chars().take(300).collect();
    let mut body = json!({
        "model": job.model,
        "messages": [
            {
                "role": "system",
                "content": "You write short chat titles. Reply with ONLY the title: 2 to 5 words, Title Case, no quotes, no punctuation, no explanation."
            },
            {
                "role": "user",
                "content": format!("/no_think\nTitle this conversation.\nUser message:\n{prompt}")
            }
        ],
        "stream": false,
        "keep_alive": "30m",
        "options": { "temperature": 0.2, "num_predict": 32 }
    });
    if supports_thinking(&job.model) {
        body["think"] = json!(false);
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .ok()?;
    let v: Value = client
        .post(format!("{}/api/chat", job.host))
        .json(&body)
        .send()
        .ok()?
        .json()
        .ok()?;
    let raw = v["message"]["content"].as_str().unwrap_or("");
    Some(clean_title(raw).unwrap_or_else(|| fallback_title(&user)))
}

fn clean_title(raw: &str) -> Option<String> {
    let (_, content) = split_think(raw);
    let line = content.lines().map(str::trim).find(|l| !l.is_empty())?;
    let title = line
        .replace("Title:", "")
        .trim_matches(|c: char| "\"'*#.:-!?“”‘’ ".contains(c))
        .trim()
        .to_string();
    let lower = title.to_lowercase();
    const BAD: [&str; 10] = [
        "we ",
        "the user",
        "the message",
        "okay",
        "ok,",
        "sure",
        "here",
        "i ",
        "title",
        "based on",
    ];
    if BAD.iter().any(|b| lower.starts_with(*b))
        || lower.contains("conversation")
        || lower.contains("prompt")
    {
        return None;
    }
    (1..=6)
        .contains(&title.split_whitespace().count())
        .then_some(title)
}
