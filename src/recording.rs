use anyhow::{Context, Result, ensure};
use base64::Engine;
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use thirtyfour::prelude::*;
struct State {
    dir: PathBuf,
    sequence: u64,
    items: Vec<(String, String)>,
    video: bool,
}
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
fn stamp() -> Result<u128> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos())
}
pub fn init(video: bool) -> Result<PathBuf> {
    let dir = crate::paths::runtime()?
        .join("runs")
        .join(stamp()?.to_string());
    crate::paths::private_dir(&dir)?;
    STATE
        .set(Mutex::new(State {
            dir: dir.clone(),
            sequence: 0,
            items: Vec::new(),
            video,
        }))
        .map_err(|_| anyhow::anyhow!("Recorder already initialized"))?;
    event(
        "session.started",
        json!({"screenshots":true,"video_requested":video}),
    )?;
    Ok(dir)
}
pub fn event(kind: &str, detail: Value) -> Result<()> {
    if let Some(state) = STATE.get() {
        let s = state
            .lock()
            .map_err(|_| anyhow::anyhow!("Recorder lock failed"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(s.dir.join("events.jsonl"))?;
        writeln!(
            file,
            "{}",
            json!({"time_unix_ns":stamp()?.to_string(),"event":kind,"detail":detail})
        )?;
    }
    Ok(())
}
fn reserve(label: &str, extension: &str) -> Result<Option<(PathBuf, String)>> {
    let Some(state) = STATE.get() else {
        return Ok(None);
    };
    let mut s = state
        .lock()
        .map_err(|_| anyhow::anyhow!("Recorder lock failed"))?;
    s.sequence += 1;
    let clean: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(100)
        .collect();
    let name = format!("{:04}-{clean}.{extension}", s.sequence);
    Ok(Some((s.dir.join(&name), name)))
}
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn add_item(name: String, label: &str) -> Result<()> {
    let Some(state) = STATE.get() else {
        return Ok(());
    };
    let mut s = state
        .lock()
        .map_err(|_| anyhow::anyhow!("Recorder lock failed"))?;
    s.items.push((name, label.into()));
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>Steamproject session</title><style>body{font:16px system-ui;max-width:1200px;margin:auto;background:#171a21;color:#eee}img,video{max-width:100%}figure{margin:2em 0}a{color:#8cf}</style><h1>Steamproject session</h1><p>Private local recording · <a href=events.jsonl>Step journal</a></p>",
    );
    for (name, label) in &s.items {
        html.push_str(&format!(
            "<figure><figcaption>{}</figcaption>",
            escaped(label)
        ));
        if name.ends_with(".mp4") {
            html.push_str(&format!(
                "<video controls src=\"{}\"></video>",
                escaped(name)
            ));
        } else {
            html.push_str(&format!(
                "<a href=\"{0}\"><img loading=lazy src=\"{0}\"></a>",
                escaped(name)
            ));
        }
        html.push_str("</figure>");
    }
    std::fs::write(s.dir.join("index.html"), html)?;
    Ok(())
}
async fn authenticated_editor(driver: &WebDriver, app: u32) -> bool {
    let Ok(url) = driver.current_url().await else {
        return false;
    };
    if url.scheme() != "https" || url.host_str() != Some("partner.steamgames.com") {
        return false;
    }
    if let Ok(section) = crate::technical::current_section(driver, app).await {
        return crate::technical::identity(driver, app, section)
            .await
            .is_ok();
    }
    if url.path().starts_with("/taxonomy/tagwizard/") {
        return crate::tags::identity(driver, app).await.is_ok();
    }
    if !url.path().starts_with("/admin/game/edit/") {
        return false;
    }
    driver.execute("return window.g_AppId===arguments[0] && !Array.from(document.querySelectorAll('input[type=password]')).some(e=>e.getClientRects().length);",vec![json!(app)]).await.ok().is_some_and(|r|r.json().as_bool()==Some(true))
}
pub async fn checkpoint(driver: &WebDriver, app: u32, label: &str) -> Result<()> {
    if !authenticated_editor(driver, app).await {
        event(
            "screenshot.skipped",
            json!({"step":label,"reason":"not authenticated editor"}),
        )?;
        return Ok(());
    }
    let Some((path, name)) = reserve(label, "png")? else {
        return Ok(());
    };
    driver.screenshot(&path).await?;
    add_item(name.clone(), label)?;
    event("screenshot", json!({"step":label,"file":name}))?;
    Ok(())
}
pub struct Video {
    stream: String,
    path: PathBuf,
    name: String,
}
async fn begin_video(driver: &WebDriver, label: &str) -> Result<Option<Video>> {
    let Some((path, name)) = reserve(label, "mp4")? else {
        return Ok(None);
    };
    let result = driver
        .cdp()
        .send_raw(
            "Page.startScreenRecording",
            json!({"audio":false,"frameRate":12,"maxWidth":1600,"maxHeight":1000}),
        )
        .await?;
    let stream = result["stream"]
        .as_str()
        .context("Chrome returned no recording stream")?
        .to_string();
    event("video.started", json!({"step":label}))?;
    Ok(Some(Video { stream, path, name }))
}
pub async fn start_video(driver: &WebDriver, app: u32, label: &str) -> Result<Option<Video>> {
    let enabled = STATE
        .get()
        .and_then(|s| s.lock().ok().map(|s| s.video))
        .unwrap_or(false);
    if !enabled || !authenticated_editor(driver, app).await {
        return Ok(None);
    }
    match begin_video(driver, label).await {
        Ok(v) => Ok(v),
        Err(e) => {
            event("video.unavailable", json!({"error":e.to_string()}))?;
            eprintln!("Native Chrome video unavailable; screenshots remain enabled: {e}");
            Ok(None)
        }
    }
}
pub async fn stop_video(driver: &WebDriver, video: Option<Video>) -> Result<Option<PathBuf>> {
    let Some(video) = video else { return Ok(None) };
    let stopped = driver
        .cdp()
        .send_raw("Page.stopScreenRecording", json!({}))
        .await?;
    let stream = stopped["stream"].as_str().unwrap_or(&video.stream);
    let partial = video.path.with_extension("mp4.part");
    let mut file = std::fs::File::create(&partial)?;
    let mut size = 0;
    let reading: Result<()> = async {
        loop {
            let chunk = driver
                .cdp()
                .send_raw("IO.read", json!({"handle":stream,"size":1048576}))
                .await?;
            let data = chunk["data"].as_str().unwrap_or_default();
            let bytes = if chunk["base64Encoded"].as_bool() == Some(true) {
                base64::engine::general_purpose::STANDARD.decode(data)?
            } else {
                data.as_bytes().to_vec()
            };
            size += bytes.len();
            ensure!(size < 512 * 1024 * 1024, "Recording exceeds 512 MiB");
            file.write_all(&bytes)?;
            if chunk["eof"].as_bool() == Some(true) {
                break;
            }
        }
        ensure!(size > 16, "Chrome returned an empty video");
        Ok(())
    }
    .await;
    let _ = driver
        .cdp()
        .send_raw("IO.close", json!({"handle":stream}))
        .await;
    reading?;
    drop(file);
    std::fs::rename(partial, &video.path)?;
    add_item(video.name.clone(), "Native Chrome recording")?;
    event("video.saved", json!({"file":video.name,"bytes":size}))?;
    Ok(Some(video.path))
}
pub async fn probe(driver: &WebDriver) -> Result<Value> {
    match begin_video(driver, "video-probe").await {
        Ok(video) => {
            driver.execute("document.body.style.background='#335577';document.body.append(' Recording check');",vec![]).await?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let path = stop_video(driver, video).await?;
            Ok(json!({"native_video":true,"file":path}))
        }
        Err(e) => Ok(json!({"native_video":false,"reason":e.to_string()})),
    }
}
