use crate::{paths, store};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thirtyfour::{components::SelectElement, prelude::*};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    pub tab: String,
    pub field: String,
    pub before: Value,
    pub after: Value,
}

pub async fn tab(driver: &WebDriver, app: u32, name: &str) -> Result<WebElement> {
    store::identity(driver, app).await?;
    let template = crate::templates::load()?;
    let recipe = template
        .tabs
        .get(name)
        .context("No reviewed template for this tab")?;
    let nav = crate::templates::unique(driver, &recipe.nav).await?;
    let content = crate::templates::unique(driver, &recipe.content).await?;
    ensure!(
        nav.attr("id").await?.as_deref() == Some(format!("tab_{name}").as_str()),
        "Template mismatch: tab identity"
    );
    // Steamworks ignores tab clicks while XFade is running. Wait before clicking,
    // and then wait for the requested content to finish its transition.
    wait_for_tabs(driver, None).await?;
    nav.click().await?;
    wait_for_tabs(driver, Some((&recipe.nav, &recipe.content))).await?;
    crate::recording::checkpoint(driver, app, &format!("tab-{name}")).await?;
    Ok(content)
}

pub async fn fields(driver: &WebDriver, app: u32, name: &str) -> Result<Value> {
    let content = tab(driver, app, name).await?;
    let mut report = driver
        .execute(include_str!("audit.js"), vec![content.to_json()?])
        .await?
        .json()
        .clone();
    let template = crate::templates::load()?;
    for list in ["fields", "empty_fields"] {
        if let Some(fields) = report[list].as_array_mut() {
            for field in fields {
                let reviewed = field["field"].as_str().is_some_and(|reference| {
                    crate::templates::reference_key(&template, name, reference).is_some()
                });
                field["template_reviewed"] = json!(reviewed);
            }
        }
    }
    Ok(report)
}

pub async fn audit(driver: &WebDriver, app: u32) -> Result<Value> {
    store::identity(driver, app).await?;
    let tabs=driver.execute("return Array.from(document.querySelectorAll('div.tab[id^=tab_]')).map(e=>({tab:e.id.slice(4),title:e.innerText.trim()}));",vec![]).await?;
    let original = driver
        .execute(
            "return document.querySelector('div.tab.activetab')?.id.slice(4)||'basic';",
            vec![],
        )
        .await?
        .json()
        .as_str()
        .unwrap_or("basic")
        .to_string();
    let mut reports = Vec::new();
    for item in tabs.json().as_array().context("No Steamworks tabs found")? {
        let name = item["tab"].as_str().context("Invalid tab")?;
        match fields(driver, app, name).await {
            Ok(report) => reports.push(json!({"tab":name,"title":item["title"],"report":report})),
            Err(e) => reports.push(json!({"tab":name,"title":item["title"],"error":e.to_string()})),
        }
    }
    let _ = tab(driver, app, &original).await;
    let result = json!({"app_id":app,"tabs":reports,"coverage":"Store editor tabs only; required states are Steamworks markers. Empty optional fields are not release blockers. Complex widgets and technical settings may require dedicated adapters."});
    std::fs::write(
        paths::runtime()?.join("audit.json"),
        serde_json::to_string_pretty(&result)?,
    )?;
    Ok(result)
}

async fn control(driver: &WebDriver, app: u32, name: &str, reference: &str) -> Result<WebElement> {
    ensure!(name != "publish", "Publish is read-only in this utility");
    tab(driver, app, name).await?;
    let element = crate::templates::by_reference(driver, name, reference).await?;
    let kind = element.attr("type").await?.unwrap_or_default();
    ensure!(
        !matches!(
            kind.as_str(),
            "hidden" | "password" | "submit" | "reset" | "button"
        ),
        "Protected control"
    );
    ensure!(
        element.is_displayed().await?
            && element.is_enabled().await?
            && element.attr("readonly").await?.is_none(),
        "Field is not editable"
    );
    ensure!(
        !element
            .attr("name")
            .await?
            .unwrap_or_default()
            .ends_with("_compl"),
        "Autocomplete needs a dedicated adapter"
    );
    Ok(element)
}
async fn wait_for_tabs(driver: &WebDriver, target: Option<(&str, &str)>) -> Result<()> {
    for _ in 0..100 {
        let result=driver.execute(r#"
          const [nav,content]=arguments;
          if(Array.from(document.querySelectorAll('div.tab_content')).some(e=>e.effect&&e.effect.state!=='finished'))return false;
          if(!nav)return true;
          const n=document.querySelector(nav),c=document.querySelector(content);
          return !!n&&!!c&&n.classList.contains('activetab')&&c.classList.contains('active_content')&&!!c.getClientRects().length;
        "#,vec![json!(target.map(|t|t.0)),json!(target.map(|t|t.1))]).await?;
        if result.json().as_bool() == Some(true) {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    anyhow::bail!("Template mismatch: tab transition did not finish within 10 seconds")
}

async fn value(element: &WebElement) -> Result<Value> {
    match element.attr("type").await?.as_deref() {
        Some("checkbox" | "radio") => Ok(json!(element.is_selected().await?)),
        _ => Ok(json!(element.value().await?.unwrap_or_default())),
    }
}

pub async fn get(driver: &WebDriver, app: u32, name: &str, reference: &str) -> Result<Value> {
    let element = control(driver, app, name, reference).await?;
    ensure!(
        element.attr("type").await?.as_deref() != Some("file"),
        "Use upload for files"
    );
    value(&element).await
}

pub async fn set(
    driver: &WebDriver,
    app: u32,
    name: &str,
    reference: &str,
    target: &Value,
) -> Result<Change> {
    let element = control(driver, app, name, reference).await?;
    let before = value(&element).await?;
    let kind = element.attr("type").await?.unwrap_or_default();
    ensure!(kind != "file", "Use explicit upload command for files");
    ensure!(
        matches!(
            kind.as_str(),
            "checkbox" | "radio" | "text" | "email" | "url" | "tel" | "number" | "date" | ""
        ) || element.tag_name().await? == "textarea"
            || element.tag_name().await? == "select",
        "Unsupported field type"
    );
    let change = Change {
        tab: name.into(),
        field: reference.into(),
        before,
        after: target.clone(),
    };
    let path = paths::runtime()?.join(format!(
        "field-backup-{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::write(path, serde_json::to_string_pretty(&change)?)?;
    match kind.as_str() {
        "checkbox" | "radio" => {
            let checked = target
                .as_bool()
                .context("Checkbox/radio value must be boolean")?;
            ensure!(
                kind != "radio" || checked,
                "Select another radio option instead of clearing it"
            );
            if element.is_selected().await? != checked {
                element.click().await?;
            }
        }
        _ => {
            let text = target
                .as_str()
                .context("Text/select value must be a string")?;
            if element.tag_name().await? == "select" {
                SelectElement::new(&element)
                    .await?
                    .select_by_value(text)
                    .await?;
            } else {
                driver.execute(r#"
                  const [e,text]=arguments;
                  const p=e.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;
                  Object.getOwnPropertyDescriptor(p,'value').set.call(e,text);
                  e.dispatchEvent(new Event('input',{bubbles:true}));e.dispatchEvent(new Event('change',{bubbles:true}));
                "#,vec![element.to_json()?,json!(text)]).await?;
            }
        }
    }
    ensure!(
        value(&element).await? == *target,
        "Field did not accept the requested value; not saved"
    );
    Ok(change)
}

pub async fn save(driver: &WebDriver, app: u32, changes: &[Change]) -> Result<Value> {
    ensure!(!changes.is_empty(), "No pending fields");
    for change in changes {
        ensure!(
            get(driver, app, &change.tab, &change.field).await? == change.after,
            "Field changed since set: {}",
            change.field
        );
    }
    let last = changes.last().context("No pending fields")?;
    tab(driver, app, &last.tab).await?;
    crate::templates::save_button(driver, &last.tab)
        .await?
        .click()
        .await?;
    let url = driver.current_url().await?;
    ensure!(
        url.query_pairs()
            .any(|(k, v)| k.starts_with("SuccessMsg") && v == "Changes saved"),
        "Steamworks did not confirm Save; inspect browser before retrying"
    );
    driver.refresh().await?;
    for change in changes {
        ensure!(
            get(driver, app, &change.tab, &change.field).await? == change.after,
            "Persisted field differs: {}",
            change.field
        );
    }
    Ok(json!({"saved":true,"published":false,"verified_fields":changes.len()}))
}

pub async fn upload(
    driver: &WebDriver,
    app: u32,
    name: &str,
    reference: &str,
    path: &std::path::Path,
) -> Result<Value> {
    let path = std::fs::canonicalize(path).context("File does not exist")?;
    ensure!(path.is_file(), "Upload path must be a file");
    let element = control(driver, app, name, reference).await?;
    ensure!(
        element.attr("type").await?.as_deref() == Some("file"),
        "Not a file field"
    );
    element.send_keys(path.to_string_lossy()).await?;
    Ok(
        json!({"file_selected":true,"note":"Widget may upload immediately. Completion/category must be checked in the browser or audit; file selection is not confirmation of a completed upload."}),
    )
}

pub fn summary(report: &Value) -> String {
    let mut out = String::new();
    if let Some(tabs) = report["tabs"].as_array() {
        for tab in tabs {
            out.push_str(&format!("\n{}:\n", tab["title"].as_str().unwrap_or("Tab")));
            if let Some(error) = tab.get("error") {
                out.push_str(&format!("  Not checked: {error}\n"));
                continue;
            }
            let r = &tab["report"];
            if let Some(required) = r["required"].as_array() {
                for field in required {
                    out.push_str(&format!(
                        "  [{}] {}\n",
                        field["state"].as_str().unwrap_or("unknown"),
                        field["label"].as_str().unwrap_or("section")
                    ));
                }
            }
            if let Some(assets) = r["assets"].as_array() {
                for asset in assets {
                    out.push_str(&format!(
                        "  {}\n",
                        asset["label"].as_str().unwrap_or("assets")
                    ));
                }
            }
            out.push_str(&format!(
                "  Empty visible inputs (includes optional): {}\n",
                r["empty_fields"].as_array().map_or(0, |f| f.len())
            ));
        }
    }
    out
}
