use crate::data::Source;
use regex::Regex;
use serde_json::{json, Value};
use std::time::Duration;

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 eAI/1.0";

fn client(secs: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(secs))
        .user_agent(UA)
        .build()
        .map_err(|e| e.to_string())
}

pub fn fetch_models(host: &str) -> Result<Vec<String>, String> {
    let resp = client(5)?
        .get(format!("{host}/api/tags"))
        .send()
        .map_err(|e| format!("Not reachable: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Ollama returned HTTP {}", resp.status().as_u16()));
    }
    let v: Value = resp.json().map_err(|e| e.to_string())?;
    Ok(v["models"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| m["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default())
}

pub fn fetch_models_openai(base: &str, key: &str) -> Result<Vec<String>, String> {
    let base = base.trim_end_matches('/');
    let mut req = client(10)?.get(format!("{base}/models"));
    if !key.trim().is_empty() {
        req = req.bearer_auth(key.trim());
    }
    let resp = req.send().map_err(|e| format!("Not reachable: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("API returned HTTP {}", resp.status().as_u16()));
    }
    let v: Value = resp.json().map_err(|e| e.to_string())?;
    Ok(v["data"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| m["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default())
}

pub fn fetch_models_api(base: &str, key: &str, provider: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/models", base.trim_end_matches('/'));
    let mut req = client(15)?.get(url);
    match provider {
        "anthropic" => {
            req = req
                .header("x-api-key", key.trim())
                .header("anthropic-version", "2023-06-01");
        }
        "gemini" => {
            req = req.header("x-goog-api-key", key.trim());
        }
        _ if !key.trim().is_empty() => {
            req = req.bearer_auth(key.trim());
        }
        _ => {}
    }
    let resp = req.send().map_err(|e| format!("API not reachable: {e}"))?;
    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "API returned HTTP {}: {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ));
    }
    let v: Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid models response: {e}"))?;
    if provider == "gemini" {
        Ok(v["models"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter(|m| {
                        m["supportedGenerationMethods"]
                            .as_array()
                            .map_or(true, |methods| {
                                methods
                                    .iter()
                                    .any(|s| s.as_str() == Some("generateContent"))
                            })
                    })
                    .filter_map(|m| {
                        m["name"]
                            .as_str()
                            .map(|s| s.strip_prefix("models/").unwrap_or(s).to_string())
                    })
                    .collect()
            })
            .unwrap_or_default())
    } else {
        Ok(v["data"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|m| m["id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default())
    }
}

/// True when Ollama reports the "vision" capability for the model.
pub fn model_supports_vision(host: &str, model: &str) -> bool {
    let Ok(c) = client(5) else { return false };
    let Ok(resp) = c
        .post(format!("{host}/api/show"))
        .json(&json!({ "model": model }))
        .send()
    else {
        return false;
    };
    let Ok(v) = resp.json::<Value>() else {
        return false;
    };
    v["capabilities"]
        .as_array()
        .map(|a| a.iter().any(|c| c.as_str() == Some("vision")))
        .unwrap_or(false)
}

pub fn check_searx(base: &str) -> String {
    match searx_get(base, "test", 5) {
        Ok(_) => "Reachable".into(),
        Err(e) => e,
    }
}

fn searx_get(base: &str, q: &str, secs: u64) -> Result<reqwest::blocking::Response, String> {
    let resp = client(secs)?
        .get(format!("{base}/search"))
        .query(&[("q", q), ("format", "json"), ("language", "en")])
        .send()
        .map_err(|e| format!("SearXNG not reachable: {e}"))?;
    match resp.status().as_u16() {
        200 => Ok(resp),
        403 => Err("JSON output is disabled. Add json to search.formats in SearXNG settings.yml and restart it".into()),
        c => Err(format!("SearXNG answered with HTTP {c}")),
    }
}

/// SearXNG first, then DuckDuckGo HTML as a fallback.
pub fn web_search(searx: &str, query: &str, max: usize) -> Result<Vec<Source>, String> {
    let mut reason = "SearXNG disabled".to_string();
    if !searx.is_empty() {
        match searx_json(searx, query, max) {
            Ok(list) if !list.is_empty() => return Ok(list),
            Ok(_) => reason = "SearXNG returned no results".into(),
            Err(e) => reason = e,
        }
    }
    ddg(query, max).map_err(|e| format!("SearXNG: {reason} | DuckDuckGo: {e}"))
}

fn searx_json(base: &str, query: &str, max: usize) -> Result<Vec<Source>, String> {
    let resp = searx_get(base, query, 12)?;
    let v: Value = resp.json().map_err(|e| e.to_string())?;
    Ok(v["results"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|r| {
                    let url = r["url"].as_str()?.to_string();
                    if url.is_empty() {
                        return None;
                    }
                    Some(Source {
                        title: r["title"].as_str().unwrap_or(&url).to_string(),
                        url,
                        snippet: r["content"].as_str().unwrap_or("").to_string(),
                    })
                })
                .take(max)
                .collect()
        })
        .unwrap_or_default())
}

fn ddg(query: &str, max: usize) -> Result<Vec<Source>, String> {
    let resp = client(10)?
        .post("https://html.duckduckgo.com/html/")
        .form(&[("q", query)])
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    let html = resp.text().map_err(|e| e.to_string())?;
    let link_re = Regex::new(r#"(?s)<a[^>]*class="result__a"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#)
        .map_err(|e| e.to_string())?;
    let snip_re =
        Regex::new(r#"(?s)class="result__snippet"[^>]*>(.*?)</a>"#).map_err(|e| e.to_string())?;
    let snippets: Vec<String> = snip_re
        .captures_iter(&html)
        .map(|c| clean_html(&c[1]))
        .collect();

    let mut out = Vec::new();
    for (i, c) in link_re.captures_iter(&html).enumerate() {
        if out.len() >= max {
            break;
        }
        let Some(url) = unwrap_ddg(&c[1]) else {
            continue;
        };
        out.push(Source {
            title: clean_html(&c[2]),
            url,
            snippet: snippets.get(i).cloned().unwrap_or_default(),
        });
    }
    Ok(out)
}

fn unwrap_ddg(href: &str) -> Option<String> {
    let full = if href.starts_with("//") {
        format!("https:{href}")
    } else {
        href.to_string()
    };
    let url = reqwest::Url::parse(&full).ok()?;
    if let Some((_, v)) = url.query_pairs().find(|(k, _)| k == "uddg") {
        return Some(v.into_owned());
    }
    full.starts_with("http").then_some(full)
}

pub fn clean_html(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn host_of(url: &str) -> String {
    let no_scheme = url.split("://").nth(1).unwrap_or(url);
    no_scheme
        .split('/')
        .next()
        .unwrap_or(no_scheme)
        .trim_start_matches("www.")
        .to_string()
}
