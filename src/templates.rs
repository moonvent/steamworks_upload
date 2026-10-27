use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use thirtyfour::prelude::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Templates {
    pub version: u32,
    pub name: String,
    pub page: Page,
    pub tabs: BTreeMap<String, Tab>,
    pub fields: BTreeMap<String, Field>,
    pub localization: Localization,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub origin: String,
    pub path_prefix: String,
    pub form_selector: String,
    pub save_path_prefix: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tab {
    pub nav: String,
    pub content: String,
    pub save: Option<String>,
    #[serde(default)]
    pub read_only: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub tab: String,
    pub selector: String,
    pub tag: String,
    pub input_type: Option<String>,
    pub expected_name: Option<String>,
    pub expected_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Localization {
    pub short_value: String,
    pub about_value: String,
    pub current_language: String,
}

pub fn parse(text: &str) -> Result<Templates> {
    let t: Templates = toml::from_str(text).context("Invalid page templates")?;
    ensure!(t.version == 1, "Unsupported template version {}", t.version);
    ensure!(
        t.page.origin == "https://partner.steamgames.com"
            && t.page.path_prefix == "/admin/game/edit/"
            && t.page.save_path_prefix == "/admin/game/save/",
        "Untrusted page origin or form route in template"
    );
    for f in t.fields.values() {
        ensure!(t.tabs.contains_key(&f.tab), "Field refers to unknown tab");
    }
    Ok(t)
}
pub fn load() -> Result<Templates> {
    if let Some(path) = std::env::var_os("STEAMPROJECT_TEMPLATES") {
        return parse(
            &std::fs::read_to_string(path).context("Cannot read STEAMPROJECT_TEMPLATES")?,
        );
    }
    // Checkout files can be edited without rebuilding; installed binaries carry a fallback.
    let local = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/store.toml");
    if local.is_file() {
        return parse(&std::fs::read_to_string(local)?);
    }
    parse(include_str!("../templates/store.toml"))
}
pub async fn page(driver: &WebDriver, app: u32) -> Result<()> {
    let t = load()?;
    let result: Result<()> = async {
        let url = driver.current_url().await?;
        ensure!(
            url.origin().ascii_serialization() == t.page.origin
                && url.path().starts_with(&t.page.path_prefix),
            "Template mismatch: not the expected Steamworks editor page"
        );
        let form = unique(driver, &t.page.form_selector).await?;
        let action = form
            .attr("action")
            .await?
            .context("Template mismatch: form has no action")?;
        let action = url.join(&action)?;
        ensure!(
            action.origin().ascii_serialization() == t.page.origin
                && action.path().starts_with(&t.page.save_path_prefix),
            "Template mismatch: unexpected Save endpoint"
        );
        let page_id = url
            .path()
            .trim_start_matches(&t.page.path_prefix)
            .trim_matches('/');
        let save_id = action
            .path()
            .trim_start_matches(&t.page.save_path_prefix)
            .trim_matches('/');
        ensure!(
            !page_id.is_empty()
                && page_id.bytes().all(|c| c.is_ascii_digit())
                && page_id == save_id,
            "Template mismatch: editor and Save endpoint IDs differ"
        );
        let actual = driver.execute("return window.g_AppId;", vec![]).await?;
        ensure!(
            actual.json().as_u64() == Some(app.into()),
            "Wrong AppID: expected {app}, got {}",
            actual.json()
        );
        for name in ["basic", "description"] {
            let tab = t.tabs.get(name).context("Missing core tab template")?;
            unique(driver, &tab.nav).await?;
            unique(driver, &tab.content).await?;
        }
        Ok(())
    }
    .await;
    crate::recording::event(
        "template.page",
        json!({"version":t.version,"template":t.name,"valid":result.is_ok()}),
    )?;
    result
}
pub async fn unique(driver: &WebDriver, selector: &str) -> Result<WebElement> {
    let elements = driver.find_all(By::Css(selector)).await?;
    ensure!(
        elements.len() == 1,
        "Template mismatch: expected one element for {selector}, found {}",
        elements.len()
    );
    Ok(elements[0].clone())
}
pub async fn field(driver: &WebDriver, key: &str) -> Result<WebElement> {
    let t = load()?;
    let f = t
        .fields
        .get(key)
        .with_context(|| format!("No field template for {key}"))?;
    let tab = t.tabs.get(&f.tab).context("Unknown field tab")?;
    let result=driver.execute(r#"
        const [container,selector,tag,type,name,id]=arguments;
        const roots=document.querySelectorAll(container);
        if(roots.length!==1)throw new Error('Template mismatch: field container');
        const found=roots[0].querySelectorAll(selector);
        if(found.length!==1)throw new Error('Template mismatch: field is missing or ambiguous');
        const e=found[0];
        if(e.tagName.toLowerCase()!==tag||(type&&e.type!==type)||(name&&e.name!==name)||(id&&e.id!==id))throw new Error('Template mismatch: field semantics changed');
        return e;
    "#,vec![json!(tab.content),json!(f.selector),json!(f.tag),json!(f.input_type),json!(f.expected_name),json!(f.expected_id)]).await;
    crate::recording::event(
        "template.field",
        json!({"field":key,"version":t.version,"valid":result.is_ok()}),
    )?;
    Ok(result?.element()?)
}
pub async fn localized(
    driver: &WebDriver,
    language: &str,
) -> Result<(WebElement, WebElement, WebElement)> {
    ensure!(
        !language.is_empty() && language.bytes().all(|c| c.is_ascii_lowercase()),
        "Invalid language"
    );
    let t = load()?;
    let short = unique(
        driver,
        &t.localization.short_value.replace("{language}", language),
    )
    .await?;
    let about = unique(
        driver,
        &t.localization.about_value.replace("{language}", language),
    )
    .await?;
    for e in [&short, &about] {
        ensure!(
            e.tag_name().await? == "input" && e.attr("type").await?.as_deref() == Some("hidden"),
            "Template mismatch: localized backing field changed"
        );
    }
    let current = unique(driver, &t.localization.current_language).await?;
    Ok((short, about, current))
}
pub async fn save_button(driver: &WebDriver, tab: &str) -> Result<WebElement> {
    let t = load()?;
    let tab = t.tabs.get(tab).context("No tab template")?;
    ensure!(!tab.read_only, "Template forbids writes on this tab");
    let selector = tab
        .save
        .as_ref()
        .context("This tab has no reviewed Save recipe")?;
    let root = unique(driver, &tab.content).await?;
    let elements = root.find_all(By::Css(selector)).await?;
    let mut visible = Vec::new();
    for e in elements {
        if e.is_displayed().await? {
            visible.push(e);
        }
    }
    ensure!(
        visible.len() == 1,
        "Template mismatch: Save button missing or ambiguous"
    );
    let e = &visible[0];
    ensure!(
        e.tag_name().await? == "input" && e.attr("type").await?.as_deref() == Some("submit"),
        "Template mismatch: Save control changed"
    );
    let label = e.value().await?.unwrap_or_default();
    ensure!(
        matches!(label.as_str(), "Save" | "Save Changes"),
        "Unexpected submission control"
    );
    Ok(e.clone())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_redirected_form_origin() {
        let changed = include_str!("../templates/store.toml")
            .replace("https://partner.steamgames.com", "https://example.invalid");
        assert!(parse(&changed).is_err());
    }
    #[test]
    fn rejects_unknown_template_version() {
        let changed =
            include_str!("../templates/store.toml").replacen("version = 1", "version = 99", 1);
        assert!(parse(&changed).is_err());
    }
}

pub fn reference_key<'a>(t: &'a Templates, tab: &str, reference: &str) -> Option<&'a str> {
    let (kind, key) = reference.split_once(':')?;
    t.fields
        .iter()
        .find(|(_, f)| {
            f.tab == tab
                && match kind {
                    "id" => f.expected_id.as_deref() == Some(key),
                    "name" => f.expected_name.as_deref() == Some(key),
                    _ => false,
                }
        })
        .map(|(k, _)| k.as_str())
}
pub async fn by_reference(driver: &WebDriver, tab: &str, reference: &str) -> Result<WebElement> {
    let t = load()?;
    let key = reference_key(&t, tab, reference).context(
        "No reviewed field template for this reference; inspect and add a recipe before editing",
    )?;
    field(driver, key).await
}
