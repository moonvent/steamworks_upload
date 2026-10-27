use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thirtyfour::components::SelectElement;
use thirtyfour::prelude::*;

const EDITOR_HOST: &str = r#"[data-featuretarget="storeadmin-app-description-editor"]"#;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
    pub app_id: u32,
    pub language: String,
    pub name: String,
    pub short_description: String,
    pub description_file: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq)]
pub struct Snapshot {
    pub app_id: u32,
    pub language: String,
    pub name: String,
    pub short_description: String,
    pub description: String,
}

impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        // Steam normalizes BBCode line endings on Save. Preserve the original
        // bytes in snapshots and backups, but accept that transport-only change.
        self.app_id == other.app_id
            && self.language == other.language
            && self.name == other.name
            && self.short_description == other.short_description
            && self.description.replace("\r\n", "\n") == other.description.replace("\r\n", "\n")
    }
}

pub fn load(path: &Path) -> Result<Store> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))?;
    let config: Store = toml::from_str(&text).context("Invalid store config")?;
    ensure!(config.app_id > 0, "app_id must be positive");
    ensure!(
        !config.language.is_empty() && config.language.bytes().all(|c| c.is_ascii_lowercase()),
        "Invalid Steam language code"
    );
    Ok(config)
}

pub fn desired(path: &Path, config: &Store) -> Result<Snapshot> {
    ensure!(
        !config.name.trim().is_empty(),
        "name is required; run pull to download existing texts first"
    );
    ensure!(
        config.short_description.encode_utf16().count() <= 300,
        "Short description exceeds 300 UTF-16 characters"
    );
    let description = std::fs::read_to_string(
        path.parent()
            .unwrap_or(Path::new("."))
            .join(&config.description_file),
    )
    .context("Cannot read description_file")?;
    Ok(Snapshot {
        app_id: config.app_id,
        language: config.language.clone(),
        name: config.name.clone(),
        short_description: config.short_description.clone(),
        description,
    })
}

pub async fn identity(driver: &WebDriver, app_id: u32) -> Result<()> {
    crate::templates::page(driver, app_id).await
}

pub async fn editor(driver: &WebDriver, config: &Store) -> Result<()> {
    identity(driver, config.app_id).await?;
    crate::browser::tab(driver, config.app_id, "description").await?;
    let select = crate::templates::field(driver, "language").await?;
    SelectElement::new(&select)
        .await?
        .select_by_value(&config.language)
        .await?;
    // The site's own language-change handler updates both PHP and React editors.
    wait(driver, r#"return document.getElementById('app_content_short_description__currentlanguage')?.value === arguments[0];"#, &config.language).await?;
    let host = about_host(driver).await?;
    let toggle = host.find(By::Css("[role='checkbox']")).await?;
    if toggle.attr("aria-checked").await?.as_deref() == Some("true") {
        toggle.click().await?;
    }
    host.query(By::Css("textarea")).first().await?;
    // React and PHP controls must agree on the selected language.
    let ready = driver.execute(r#"
        const h=Array.from(document.querySelectorAll(arguments[1])).find(e=>JSON.parse(e.dataset.props).editorType==='aboutthegame');
        const label = h?.querySelector('[role="combobox"]')?.innerText.trim();
        const option = document.querySelector('#app_content_short_description__select')?.selectedOptions[0]?.text;
        return label === option;
    "#, vec![serde_json::json!(config.language), serde_json::json!(EDITOR_HOST)]).await?;
    ensure!(
        ready.json().as_bool() == Some(true),
        "Description editor language has not synchronized; try again"
    );
    Ok(())
}

async fn about_host(driver: &WebDriver) -> Result<WebElement> {
    crate::templates::field(driver, "about").await
}

async fn wait(driver: &WebDriver, script: &str, arg: &str) -> Result<()> {
    for _ in 0..100 {
        if driver
            .execute(script, vec![serde_json::json!(arg)])
            .await?
            .json()
            .as_bool()
            == Some(true)
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    bail!("Steamworks did not synchronize the editor within 10 seconds")
}

pub async fn read(driver: &WebDriver, config: &Store) -> Result<Snapshot> {
    editor(driver, config).await?;
    let (short, about, current) = crate::templates::localized(driver, &config.language).await?;
    ensure!(
        current.value().await?.as_deref() == Some(config.language.as_str()),
        "Template mismatch: wrong language"
    );
    let name = crate::templates::field(driver, "name").await?;
    Ok(Snapshot {
        app_id: config.app_id,
        language: config.language.clone(),
        name: name.value().await?.unwrap_or_default(),
        short_description: short.value().await?.unwrap_or_default(),
        description: about.value().await?.unwrap_or_default(),
    })
}

pub fn validate_pair(old: &Snapshot, new: &Snapshot) -> Result<()> {
    ensure!(
        old.app_id == new.app_id && old.language == new.language,
        "App or language mismatch"
    );
    Ok(())
}

fn stamp() -> Result<String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
}

pub fn backup(snapshot: &Snapshot) -> Result<PathBuf> {
    let dir = crate::paths::runtime()?.join("backups").join(stamp()?);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("store.json"),
        serde_json::to_string_pretty(snapshot)?,
    )?;
    eprintln!("Backup: {}", dir.display());
    Ok(dir)
}

pub fn pull(path: &Path, snapshot: &Snapshot) -> Result<()> {
    let dir = backup(snapshot)?;
    if path.exists() {
        std::fs::copy(path, dir.join("previous-config.toml"))?;
    }
    let parent = path.parent().unwrap_or(Path::new("."));
    let relative = PathBuf::from("store-data")
        .join(snapshot.app_id.to_string())
        .join(format!("{}.{}.bbcode", snapshot.language, stamp()?));
    let description_path = parent.join(&relative);
    std::fs::create_dir_all(
        description_path
            .parent()
            .context("Invalid description path")?,
    )?;
    std::fs::write(&description_path, &snapshot.description)?;
    let config = Store {
        app_id: snapshot.app_id,
        language: snapshot.language.clone(),
        name: snapshot.name.clone(),
        short_description: snapshot.short_description.clone(),
        description_file: relative,
    };
    std::fs::write(path, toml::to_string_pretty(&config)?)?;
    eprintln!(
        "Downloaded current texts to {} and {}",
        path.display(),
        description_path.display()
    );
    Ok(())
}

async fn set_text(driver: &WebDriver, element: &WebElement, text: &str) -> Result<()> {
    if element.value().await?.as_deref() == Some(text) {
        return Ok(());
    }
    ensure!(
        element.is_enabled().await? && element.attr("readonly").await?.is_none(),
        "Template field is not editable"
    );
    element.scroll_into_view().await?;

    // Native setter bypasses React's value tracker, so its input handler sees the change.
    driver.execute(r#"
        const [e,text]=arguments;
        const prototype=e.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;
        Object.getOwnPropertyDescriptor(prototype,'value').set.call(e,text);
        e.dispatchEvent(new Event('input',{bubbles:true}));
        e.dispatchEvent(new Event('change',{bubbles:true}));
    "#,vec![element.to_json()?,serde_json::json!(text)]).await?;
    Ok(())
}

pub async fn fill(driver: &WebDriver, config: &Store, target: &Snapshot) -> Result<()> {
    identity(driver, target.app_id).await?;
    ensure!(
        config.app_id == target.app_id && config.language == target.language,
        "Target/config mismatch"
    );
    editor(driver, config).await?;
    crate::browser::tab(driver, config.app_id, "basic").await?;
    set_text(
        driver,
        &crate::templates::field(driver, "name").await?,
        &target.name,
    )
    .await?;
    crate::browser::tab(driver, config.app_id, "description").await?;
    set_text(
        driver,
        &crate::templates::field(driver, "short_description").await?,
        &target.short_description,
    )
    .await?;
    let host = about_host(driver).await?;
    set_text(
        driver,
        &host.find(By::Css("textarea")).await?,
        &target.description,
    )
    .await?;
    for _ in 0..100 {
        if read(driver, config).await? == *target {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    bail!("Form values did not synchronize; draft was not saved")
}

pub async fn save(driver: &WebDriver, config: &Store, target: &Snapshot) -> Result<()> {
    ensure!(
        read(driver, config).await? == *target,
        "Form changed since fill; run plan and fill again"
    );
    // Standard Save button only. This is distinct from the Publish tab.
    crate::templates::save_button(driver, "description")
        .await?
        .click()
        .await?;

    for _ in 0..200 {
        let url = driver.current_url().await?;
        if url
            .query_pairs()
            .any(|(k, v)| k.starts_with("SuccessMsg") && v == "Changes saved")
            && identity(driver, config.app_id).await.is_ok()
        {
            driver.refresh().await?;
            identity(driver, config.app_id).await?;
            ensure!(
                read(driver, config).await? == *target,
                "Saved response does not match local texts; inspect Steamworks manually"
            );
            eprintln!("Draft saved and verified. Public store page was not published.");
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    bail!("Steamworks did not confirm saving; check the browser before retrying")
}

pub async fn roundtrip(driver: &WebDriver, config: &Store) -> Result<()> {
    let old = read(driver, config).await?;
    backup(&old)?;
    let mut probe = old.clone();
    probe.name.push_str("_rust_probe");
    probe.short_description = "Rust temporary input check 🛡️".into();
    probe.description.push_str("\nRust input check 🛡️");
    let result = fill(driver, config, &probe).await;
    let restore = fill(driver, config, &old).await;
    restore.context("Could not restore browser form after input check; do not save the form")?;
    result?;
    ensure!(read(driver, config).await? == old, "Restore failed");
    eprintln!("Unicode/BBCode editor input verified and restored. No Save request sent.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_saved_crlf_without_hiding_content_changes() {
        let original = Snapshot {
            app_id: 1,
            language: "english".into(),
            name: "Example".into(),
            short_description: "Example description".into(),
            description: "[h2]Title[/h2]\n\n[p]Text[/p]\n".into(),
        };
        let mut saved = original.clone();
        saved.description = saved.description.replace('\n', "\r\n");
        assert_eq!(original, saved);
        assert!(saved.description.contains("\r\n"));
        for changed in [
            Snapshot {
                app_id: 2,
                ..saved.clone()
            },
            Snapshot {
                language: "french".into(),
                ..saved.clone()
            },
            Snapshot {
                name: "Changed".into(),
                ..saved.clone()
            },
            Snapshot {
                short_description: "Changed".into(),
                ..saved.clone()
            },
            Snapshot {
                description: saved.description.replace("Text", "Changed"),
                ..saved.clone()
            },
            Snapshot {
                description: saved.description.replace("\r\n\r\n", "\r\n"),
                ..saved.clone()
            },
            Snapshot {
                description: saved.description.trim_end().into(),
                ..saved.clone()
            },
        ] {
            assert_ne!(original, changed);
        }
    }
    #[test]
    fn refuses_cross_app_plan() {
        let a = Snapshot {
            app_id: 1,
            language: "english".into(),
            name: "A".into(),
            short_description: "".into(),
            description: "".into(),
        };
        let mut b = a.clone();
        b.app_id = 2;
        assert!(validate_pair(&a, &b).is_err());
    }
    #[test]
    fn enforces_utf16_length_for_non_bmp_text() {
        let config = Store {
            app_id: 1,
            language: "english".into(),
            name: "A".into(),
            short_description: "🛡".repeat(151),
            description_file: "does-not-exist".into(),
        };
        let error = desired(Path::new("store.toml"), &config).unwrap_err();
        assert!(error.to_string().contains("exceeds 300"));
    }
}
