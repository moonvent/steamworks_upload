use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use thirtyfour::{components::SelectElement, prelude::*};
#[path = "technical_icons.rs"]
mod icons;
pub use icons::upload_icon;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Stat {
    pub api_name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub display_name: String,
    #[serde(default)]
    pub permission: u8,
    #[serde(default)]
    pub increment_only: bool,
    #[serde(default)]
    pub aggregated: bool,
    #[serde(default)]
    pub default: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub max_change: Option<f64>,
    pub window: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub stat: String,
    pub min: f64,
    pub max: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Achievement {
    pub api_name: String,
    pub display_name: BTreeMap<String, String>,
    pub description: BTreeMap<String, String>,
    #[serde(default)]
    pub permission: u8,
    #[serde(default)]
    pub hidden: bool,
    pub progress: Option<Progress>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "section", content = "definition", rename_all = "lowercase")]
pub enum Definition {
    Stats(Stat),
    Achievements(Achievement),
}
impl Definition {
    fn section(&self) -> &'static str {
        match self {
            Self::Stats(_) => "stats",
            Self::Achievements(_) => "achievements",
        }
    }
    fn name(&self) -> &str {
        match self {
            Self::Stats(s) => &s.api_name,
            Self::Achievements(a) => &a.api_name,
        }
    }
}
pub struct Plan {
    app: u32,
    target: Definition,
    before: Value,
    pub attempted: bool,
}
fn api_name(s: &str) -> Result<()> {
    ensure!(
        !s.is_empty()
            && s.len() <= 128
            && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'),
        "API name must contain 1..128 ASCII letters, digits or underscores"
    );
    Ok(())
}
fn number(n: f64, integer: bool) -> Result<()> {
    ensure!(
        n.is_finite() && n.abs() <= f32::MAX as f64,
        "Value is outside the Steam FLOAT range"
    );
    if integer {
        ensure!(
            n.fract() == 0.0 && n >= i32::MIN as f64 && n <= i32::MAX as f64,
            "INT value must be a signed 32-bit integer"
        );
    }
    Ok(())
}
impl Stat {
    pub fn validate(&self) -> Result<()> {
        api_name(&self.api_name)?;
        ensure!(
            matches!(self.kind.as_str(), "INT" | "FLOAT" | "AVGRATE"),
            "Unknown stat type"
        );
        ensure!(
            self.permission <= 2,
            "Permission must be 0 (Client), 1 (GS) or 2 (Official GS)"
        );
        ensure!(
            !self.display_name.trim().is_empty(),
            "Stat display name is required"
        );
        for n in [
            Some(self.default),
            self.min,
            self.max,
            self.max_change,
            self.window,
        ]
        .into_iter()
        .flatten()
        {
            number(n, self.kind == "INT")?;
        }
        ensure!(
            self.min.is_none_or(|v| self.default >= v)
                && self.max.is_none_or(|v| self.default <= v),
            "Default is outside stat bounds"
        );
        ensure!(
            self.max_change.is_none_or(|v| v >= 0.0),
            "Max change must be nonnegative"
        );
        if self.kind == "AVGRATE" {
            ensure!(
                self.window.is_some_and(|v| v > 0.0)
                    && !self.increment_only
                    && !self.aggregated
                    && self.max_change.is_none(),
                "AVGRATE needs a positive window and does not support increment_only, aggregated or max_change"
            );
        } else {
            ensure!(
                self.window.is_none(),
                "Window is only supported for AVGRATE"
            );
        }
        Ok(())
    }
}
impl Achievement {
    pub fn validate(&self) -> Result<()> {
        api_name(&self.api_name)?;
        ensure!(self.permission <= 2, "Invalid achievement permission");
        for map in [&self.display_name, &self.description] {
            ensure!(!map.is_empty(), "Localized achievement text is required");
            for (lang, text) in map {
                ensure!(
                    !lang.is_empty()
                        && lang.bytes().all(|c| c.is_ascii_lowercase() || c == b'_')
                        && lang != "token",
                    "Invalid language code"
                );
                ensure!(
                    !text.trim().is_empty(),
                    "Achievement text must not be blank"
                );
            }
        }
        if let Some(p) = &self.progress {
            api_name(&p.stat)?;
            number(p.min, false)?;
            number(p.max, false)?;
            ensure!(p.max > p.min, "Progress max must exceed min");
        }
        Ok(())
    }
}

pub async fn identity(driver: &WebDriver, app: u32, section: &str) -> Result<()> {
    let t = crate::templates::load()?;
    let p = t
        .technical
        .get(section)
        .context("Unknown technical section")?;
    let url = driver.current_url().await?;
    ensure!(
        url.origin().ascii_serialization() == t.page.origin
            && url.path() == p.path.replace("{app}", &app.to_string()),
        "Template mismatch: wrong technical page or AppID"
    );
    let heading_root = crate::templates::unique(driver, &p.heading_root).await?;
    ensure!(
        heading_root.text().await?.contains(&p.heading),
        "Template mismatch: expected technical editor heading {}",
        p.heading
    );
    let valid = driver.execute("return typeof g_sessionID==='string' && g_sessionID.length>0 && !Array.from(document.querySelectorAll('input[type=password]')).some(e=>e.getClientRects().length)", vec![]).await?;
    ensure!(
        valid.json().as_bool() == Some(true),
        "Not an authenticated technical editor"
    );
    crate::templates::unique(driver, &p.root).await?;
    crate::templates::unique(driver, &p.table).await?;
    Ok(())
}
pub async fn page(driver: &WebDriver, app: u32, section: &str) -> Result<()> {
    let t = crate::templates::load()?;
    let p = t
        .technical
        .get(section)
        .context("Unknown technical section")?;
    let path = p.path.replace("{app}", &app.to_string());
    let current = driver.current_url().await?;
    if current.host_str() == Some("partner.steamgames.com") {
        let clean = driver
            .execute(
                "return !document.querySelector('tr.dirty,tr.selected')",
                vec![],
            )
            .await?;
        ensure!(
            clean.json().as_bool() == Some(true),
            "Finish or cancel manually opened technical records before navigating"
        );
    }
    if current.origin().ascii_serialization() != t.page.origin || current.path() != path {
        driver
            .goto(format!("{}{path}?l=english", t.page.origin))
            .await?;
    }
    for _ in 0..100 {
        let ready = driver
            .execute(
                "return document.querySelectorAll(arguments[0]).length===1",
                vec![json!(p.table)],
            )
            .await?;
        if ready.json().as_bool() == Some(true) {
            identity(driver, app, section).await?;
            crate::recording::checkpoint(driver, app, &format!("technical-{section}")).await?;
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    bail!("Template mismatch: technical editor did not load; sign in manually or review recipe")
}
async fn fetch(driver: &WebDriver, app: u32, section: &str) -> Result<Value> {
    identity(driver, app, section).await?;
    let t = crate::templates::load()?;
    let p = &t.technical[section];
    let result = driver.execute(r#"
      const path=arguments[0];
      return fetch(path,{credentials:'same-origin',headers:{Accept:'application/json'},redirect:'error'})
        .then(r=>{if(!r.ok)throw new Error('Schema read failed');return r.json()});
    "#, vec![json!(p.fetch_path.replace("{app}", &app.to_string()))]).await?;
    let raw = result.json().clone();
    let items = if section == "stats" {
        &raw
    } else {
        &raw["achievements"]
    };
    let rows = items
        .as_array()
        .context("Template mismatch: schema response is not an array")?;
    let mut names = BTreeSet::new();
    for item in rows {
        let name = raw_name(section, item)?;
        ensure!(names.insert(name), "Duplicate API name in Steam schema");
        row_id(section, item)?;
    }
    Ok(raw)
}
fn items<'a>(section: &str, raw: &'a Value) -> Result<&'a Vec<Value>> {
    (if section == "stats" {
        raw
    } else {
        &raw["achievements"]
    })
    .as_array()
    .context("Invalid schema array")
}
fn raw_name<'a>(section: &str, raw: &'a Value) -> Result<&'a str> {
    raw[if section == "stats" {
        "name"
    } else {
        "api_name"
    }]
    .as_str()
    .context("Schema record is missing its API name")
}
fn id_number(raw: &Value) -> Result<String> {
    let s = if let Some(s) = raw.as_str() {
        s.to_string()
    } else {
        raw.to_string()
    };
    ensure!(
        !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()),
        "Invalid schema record ID"
    );
    Ok(s)
}
fn row_id(section: &str, raw: &Value) -> Result<String> {
    let stat = id_number(&raw["stat_id"])?;
    Ok(if section == "stats" {
        format!("s{stat}")
    } else {
        format!("a{stat}_{}", id_number(&raw["bit_id"])?)
    })
}
fn find<'a>(raw: &'a Value, section: &str, name: &str) -> Result<Option<&'a Value>> {
    Ok(items(section, raw)?
        .iter()
        .find(|r| raw_name(section, r).ok() == Some(name)))
}
pub async fn list(driver: &WebDriver, app: u32, section: &str) -> Result<Value> {
    page(driver, app, section).await?;
    Ok(
        json!({"app_id":app,"section":section,"schema":fetch(driver,app,section).await?,"published":false,"source":"Steamworks draft configuration"}),
    )
}
async fn snapshot(driver: &WebDriver, app: u32) -> Result<Value> {
    page(driver, app, "stats").await?;
    let stats = fetch(driver, app, "stats").await?;
    page(driver, app, "achievements").await?;
    let achievements = fetch(driver, app, "achievements").await?;
    Ok(json!({"stats":stats,"achievements":achievements}))
}
fn raw_num(raw: &Value) -> Option<f64> {
    raw.as_f64().or_else(|| raw.as_str()?.parse().ok())
}
fn permission(raw: &Value) -> Option<u8> {
    raw_num(raw).and_then(|n| {
        if n.fract() == 0.0 && (0.0..=2.0).contains(&n) {
            Some(n as u8)
        } else {
            None
        }
    })
}
fn validate_target(target: &Definition, before: &Value) -> Result<()> {
    match target {
        Definition::Stats(s) => {
            s.validate()?;
            if let Some(old) = find(&before["stats"], "stats", &s.api_name)? {
                ensure!(
                    old["type"].as_str() == Some(&s.kind),
                    "Changing an existing stat type is unsupported; use a new API name"
                );
            }
            for a in items("achievements", &before["achievements"])? {
                if a["progress"]["value"]["operand1"].as_str() == Some(&s.api_name) {
                    ensure!(
                        permission(&a["permission"]) == Some(s.permission),
                        "Stat permission conflicts with a linked achievement"
                    );
                    for key in ["min_val", "max_val"] {
                        let value = raw_num(&a["progress"][key])
                            .context("Unsupported achievement progress expression")?;
                        ensure!(
                            s.min.is_none_or(|v| value >= v) && s.max.is_none_or(|v| value <= v),
                            "Stat bounds conflict with a linked achievement"
                        );
                    }
                }
            }
        }
        Definition::Achievements(a) => {
            a.validate()?;
            let old = find(&before["achievements"], "achievements", &a.api_name)?;
            if old.is_none() {
                ensure!(
                    a.display_name.contains_key("english") && a.description.contains_key("english"),
                    "New achievements require English name and description"
                );
            }
            if let Some(p) = &a.progress {
                let stat = find(&before["stats"], "stats", &p.stat)?
                    .context("Progress stat must already exist")?;
                ensure!(
                    permission(&stat["permission"]) == Some(a.permission),
                    "Progress stat and achievement permissions must match"
                );
                for n in [p.min, p.max] {
                    number(n, stat["type"] == "INT")?;
                }
                ensure!(
                    raw_num(&stat["min"]).is_none_or(|v| p.min >= v)
                        && raw_num(&stat["max"]).is_none_or(|v| p.max <= v),
                    "Progress range is outside stat bounds"
                );
            }
        }
    }
    Ok(())
}
pub async fn plan(driver: &WebDriver, app: u32, target: Definition) -> Result<(Plan, Value)> {
    match &target {
        Definition::Stats(s) => s.validate()?,
        Definition::Achievements(a) => a.validate()?,
    }
    let before = snapshot(driver, app).await?;
    validate_target(&target, &before)?;
    if let Definition::Achievements(a) = &target {
        let languages = driver
            .execute("return Object.keys(window.g_rgLanguages || {})", vec![])
            .await?;
        let supported = languages
            .json()
            .as_array()
            .context("Missing achievement language schema")?;
        for lang in a.display_name.keys().chain(a.description.keys()) {
            ensure!(
                supported.iter().any(|v| v.as_str() == Some(lang)),
                "Unsupported achievement language: {lang}"
            );
        }
    }
    let existing = find(&before[target.section()], target.section(), target.name())?;
    let changed = existing.is_none_or(|old| verify(&target, old, Some(old)).is_err());
    let preview = json!({"app_id":app,"changed":changed,"action":if existing.is_some(){"update"}else{"create"},"current":existing,"planned":target,"saved":false,"published":false,"note":"Local plan only. schema-save creates/updates one draft record. Other achievement translations and icons are preserved; stat definitions replace all supported stat settings."});
    Ok((
        Plan {
            app,
            target,
            before,
            attempted: false,
        },
        preview,
    ))
}
async fn control(driver: &WebDriver, section: &str, prefix: &str, key: &str) -> Result<WebElement> {
    let t = crate::templates::load()?;
    let p = &t.technical[section];
    let f = p
        .fields
        .get(key)
        .context("No reviewed technical field recipe")?;
    let selector = format!("{} #{prefix}_{}", p.root, f.suffix);
    let e = crate::templates::unique(driver, &selector).await?;
    ensure!(
        e.tag_name().await? == f.tag
            && (f.input_type.is_none() || e.prop("type").await? == f.input_type),
        "Template mismatch: technical field semantics changed: {section}.{key}"
    );
    Ok(e)
}
async fn fill(
    driver: &WebDriver,
    section: &str,
    prefix: &str,
    key: &str,
    value: Value,
) -> Result<()> {
    let e = control(driver, section, prefix, key).await?;
    ensure!(
        e.is_displayed().await? && e.is_enabled().await? && e.attr("readonly").await?.is_none(),
        "Technical field is not editable"
    );
    if let Some(checked) = value.as_bool() {
        if e.is_selected().await? != checked {
            e.click().await?;
        }
        ensure!(
            e.is_selected().await? == checked,
            "Technical checkbox readback failed"
        );
    } else {
        let text = value
            .as_str()
            .context("Expected a string technical field")?;
        if e.tag_name().await? == "select" {
            SelectElement::new(&e).await?.select_by_value(text).await?;
        } else {
            driver.execute("const [e,v]=arguments;Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e,v);e.dispatchEvent(new Event('input',{bubbles:true}));e.dispatchEvent(new Event('change',{bubbles:true}));", vec![e.to_json()?,json!(text)]).await?;
        }
        ensure!(
            e.value().await?.as_deref() == Some(text),
            "Technical field readback failed"
        );
    }
    Ok(())
}
fn text_number(n: Option<f64>) -> Value {
    json!(n.map(|n| n.to_string()).unwrap_or_default())
}
async fn localized(
    driver: &WebDriver,
    prefix: &str,
    key: &str,
    values: &BTreeMap<String, String>,
) -> Result<()> {
    let form = control(driver, "achievements", prefix, key).await?;
    for (lang, text) in values {
        let e = form
            .find_all(By::Css(format!("input[name='{lang}']")))
            .await?;
        ensure!(
            e.len() == 1
                && e[0].tag_name().await? == "input"
                && matches!(e[0].prop("type").await?.as_deref(), Some("text" | "hidden")),
            "Template mismatch: unsupported achievement language"
        );
        driver.execute("const [e,v]=arguments;Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e,v);e.dispatchEvent(new Event('change',{bubbles:true}));", vec![e[0].to_json()?,json!(text)]).await?;
        ensure!(
            e[0].value().await?.as_deref() == Some(text),
            "Localized achievement readback failed"
        );
    }
    Ok(())
}
async fn edit(
    driver: &WebDriver,
    app: u32,
    section: &str,
    existing: Option<&Value>,
) -> Result<String> {
    let t = crate::templates::load()?;
    let p = &t.technical[section];
    if let Some(raw) = existing {
        let row =
            crate::templates::unique(driver, &format!("{} #{}", p.root, row_id(section, raw)?))
                .await?;
        ensure!(
            !row.attr("class")
                .await?
                .unwrap_or_default()
                .contains("dirty"),
            "Manual unsaved edits detected"
        );
        // Native renderer only. Unlike the New function, Edit does not persist anything.
        driver.execute("const [fn,app,raw]=arguments;if(typeof window[fn]!=='function')throw new Error('Native editor changed');window[fn](app,raw);", vec![json!(p.edit_function),json!(app),raw.clone()]).await?;
        Ok(format!("{}{}", p.edit_prefix, &row_id(section, raw)?[1..]))
    } else {
        let mut before = BTreeSet::new();
        for row in driver
            .find_all(By::Css(format!("{} tr[id]", p.table)))
            .await?
        {
            before.insert(row.attr("id").await?.context("Schema row has no ID")?);
        }
        driver.execute("const [fn,app]=arguments;if(typeof window[fn]!=='function')throw new Error('Native create control changed');window[fn](app);",vec![json!(p.create_function),json!(app)]).await?;
        for _ in 0..100 {
            let mut added = Vec::new();
            let rows = driver
                .find_all(By::Css(format!("{} tr[id]", p.table)))
                .await?;
            let count = rows.len();
            for row in rows {
                let id = row.attr("id").await?.context("Schema row has no ID")?;
                if !before.contains(&id) {
                    added.push(id);
                }
            }
            if added.len() == 1 && count == before.len() + 1 {
                let id = &added[0];
                ensure!(
                    id.starts_with(&p.row_prefix)
                        && !id[1..].is_empty()
                        && id[1..].bytes().all(|c| c.is_ascii_digit() || c == b'_'),
                    "Invalid newly allocated record ID"
                );
                let prefix = format!("{}{}", p.edit_prefix, &id[1..]);
                // EditStat marks the real row dirty through its type handler, but
                // assigns selected to a jQuery wrapper rather than to the DOM row.
                // Identify the one added row and validate its reviewed API control.
                control(driver, section, &prefix, "api_name").await?;
                return Ok(prefix);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        bail!("Creation outcome is uncertain; inspect draft and backup before planning again")
    }
}
pub async fn save(driver: &WebDriver, plan: &mut Plan) -> Result<Value> {
    ensure!(
        !plan.attempted,
        "Previous save was attempted; inspect the draft and schema-discard before replanning. Do not retry creation blindly"
    );
    let current = snapshot(driver, plan.app).await?;
    ensure!(
        current == plan.before,
        "Draft schema changed since plan; schema-discard and plan again"
    );
    validate_target(&plan.target, &current)?;
    if let Some(old) = find(
        &current[plan.target.section()],
        plan.target.section(),
        plan.target.name(),
    )? && verify(&plan.target, old, Some(old)).is_ok()
    {
        return Ok(json!({"saved":false,"unchanged":true,"published":false}));
    }
    let section = plan.target.section();
    page(driver, plan.app, section).await?;
    let dirty = driver
        .execute(
            "return document.querySelectorAll('tr.dirty,tr.selected').length",
            vec![],
        )
        .await?;
    ensure!(
        dirty.json().as_u64() == Some(0),
        "Manual edits or an open record editor detected; finish them first"
    );
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let dir = crate::paths::root()?
        .join("apps")
        .join(plan.app.to_string())
        .join("schema-backups")
        .join(stamp.to_string());
    crate::paths::private_dir(&dir)?;
    std::fs::write(
        dir.join("before.json"),
        serde_json::to_vec_pretty(
            &json!({"app_id":plan.app,"schema":current,"planned":plan.target}),
        )?,
    )?;
    crate::recording::event("schema.save.begin", json!({"backup":dir,"section":section}))?;
    plan.attempted = true;
    let existing = find(&current[section], section, plan.target.name())?;
    let prefix = edit(driver, plan.app, section, existing).await?;
    crate::recording::checkpoint(driver, plan.app, "schema-edit").await?;
    apply_definition(driver, section, &prefix, &plan.target, existing).await?;
    identity(driver, plan.app, section).await?;
    let t = crate::templates::load()?;
    let row = format!(
        "{} #{}{}",
        t.technical[section].root,
        t.technical[section].row_prefix,
        &prefix[t.technical[section].edit_prefix.len()..]
    );
    let save_selector = format!("{row} {}", t.technical[section].save);
    let button = crate::templates::unique(driver, &save_selector).await?;
    ensure!(
        button.is_displayed().await? && button.is_enabled().await?,
        "Record Save control is unavailable"
    );
    button.click().await?;
    for _ in 0..100 {
        let pending = driver.find_all(By::Css(&save_selector)).await?;
        if pending.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    ensure!(
        driver.find_all(By::Css(&save_selector)).await?.is_empty(),
        "Steamworks did not confirm record Save; inspect before retrying"
    );
    driver.refresh().await?;
    page(driver, plan.app, section).await?;
    let saved = fetch(driver, plan.app, section).await?;
    std::fs::write(dir.join("after.json"), serde_json::to_vec_pretty(&saved)?)?;
    let record = find(&saved, section, plan.target.name())?
        .context("Saved record was not found after reload")?;
    ensure!(
        row_id(section, record)?
            == format!(
                "{}{}",
                t.technical[section].row_prefix,
                &prefix[t.technical[section].edit_prefix.len()..]
            ),
        "Saved record ID changed"
    );
    verify(&plan.target, record, existing)?;
    let unaffected = |raw: &Value| -> Result<BTreeMap<String, Value>> {
        items(section, raw)?
            .iter()
            .filter(|r| raw_name(section, r).ok() != Some(plan.target.name()))
            .map(|r| Ok((raw_name(section, r)?.to_string(), r.clone())))
            .collect()
    };
    ensure!(
        unaffected(&saved)? == unaffected(&current[section])?,
        "Another schema record changed during Save"
    );
    crate::recording::event(
        "schema.save.verified",
        json!({"section":section,"backup":dir}),
    )?;
    Ok(
        json!({"saved":true,"published":false,"verified_after_reload":true,"record":record,"backup":dir}),
    )
}
fn flag(v: &Value) -> bool {
    v.as_bool()
        .unwrap_or_else(|| raw_num(v).unwrap_or(0.0) != 0.0)
}

/// Delete exactly one explicitly named achievement through the native row control.
pub async fn delete_achievement(driver: &WebDriver, app: u32, name: &str) -> Result<Value> {
    api_name(name)?;
    let before = snapshot(driver, app).await?;
    let record = find(&before["achievements"], "achievements", name)?
        .context("Achievement does not exist; nothing was deleted")?;
    let t = crate::templates::load()?;
    let p = &t.technical["achievements"];
    let delete = p
        .delete
        .as_ref()
        .context("No reviewed achievement deletion recipe")?;
    let id = row_id("achievements", record)?;
    let row_selector = format!("{} #{id}", p.root);
    let row = crate::templates::unique(driver, &row_selector).await?;
    ensure!(
        row.text().await?.lines().next() == Some(name),
        "Template mismatch: wrong achievement row"
    );
    let button = crate::templates::unique(driver, &format!("{row_selector} {delete}")).await?;
    ensure!(
        button.is_displayed().await? && button.is_enabled().await?,
        "Achievement Delete control is unavailable"
    );
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let dir = crate::paths::root()?
        .join("apps")
        .join(app.to_string())
        .join("schema-backups")
        .join(stamp.to_string());
    crate::paths::private_dir(&dir)?;
    std::fs::write(
        dir.join("before.json"),
        serde_json::to_vec_pretty(
            &json!({"app_id":app,"schema":before,"delete_achievement":name}),
        )?,
    )?;
    crate::recording::checkpoint(driver, app, "achievement-delete-before").await?;
    crate::recording::event(
        "achievement.delete.begin",
        json!({"api_name":name,"backup":dir}),
    )?;
    identity(driver, app, "achievements").await?;
    button.click().await?;
    let expected = format!(
        "Delete achievement {name} ({}/{})?",
        id_number(&record["stat_id"])?,
        id_number(&record["bit_id"])?
    );
    let prompt = driver
        .get_alert_text()
        .await
        .context("Expected native deletion confirmation did not appear; inspect before retrying")?;
    if prompt != expected {
        driver.dismiss_alert().await?;
        bail!("Template mismatch: unexpected deletion confirmation; dismissed without accepting");
    }
    driver.accept_alert().await?;
    let mut removed = false;
    for _ in 0..100 {
        if driver.find_all(By::Css(&row_selector)).await?.is_empty() {
            removed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    ensure!(
        removed,
        "Deletion outcome is uncertain; inspect current schema and backup before retrying"
    );
    driver.refresh().await?;
    page(driver, app, "achievements").await?;
    let after = snapshot(driver, app).await?;
    std::fs::write(
        dir.join("after.json"),
        serde_json::to_vec_pretty(&json!({"app_id":app,"schema":after}))?,
    )?;
    verify_achievement_deletion(&before, &after, name)?;
    crate::recording::event(
        "achievement.delete.verified",
        json!({"api_name":name,"backup":dir}),
    )?;
    Ok(
        json!({"deleted":true,"api_name":name,"published":false,"verified_after_reload":true,"backup":dir}),
    )
}

fn verify_achievement_deletion(before: &Value, after: &Value, name: &str) -> Result<()> {
    ensure!(
        find(&before["achievements"], "achievements", name)?.is_some(),
        "Missing deletion target in backup"
    );
    let mut expected = before.clone();
    expected["achievements"]["achievements"]
        .as_array_mut()
        .context("Invalid achievement schema")?
        .retain(|r| r["api_name"].as_str() != Some(name));
    ensure!(
        &expected == after,
        "Deletion readback mismatch: target still present or another schema value changed"
    );
    Ok(())
}

fn ensure_stat_unlinked(schema: &Value, name: &str) -> Result<()> {
    let linked: Vec<_> = items("achievements", &schema["achievements"])?
        .iter()
        .filter(|a| a["progress"]["value"]["operand1"].as_str() == Some(name))
        .map(|a| a["api_name"].clone())
        .collect();
    ensure!(
        linked.is_empty(),
        "Stat is linked to achievements; remove those links explicitly first: {linked:?}"
    );
    Ok(())
}

pub async fn delete_stat(driver: &WebDriver, app: u32, name: &str) -> Result<Value> {
    api_name(name)?;
    let before = snapshot(driver, app).await?;
    let record = find(&before["stats"], "stats", name)?
        .context("Stat does not exist; nothing was deleted")?;
    ensure_stat_unlinked(&before, name)?;
    page(driver, app, "stats").await?;
    let t = crate::templates::load()?;
    let p = &t.technical["stats"];
    let selector = format!("{} #{}", p.root, row_id("stats", record)?);
    crate::templates::unique(driver, &selector).await?;
    let name_cell = crate::templates::unique(
        driver,
        &format!(
            "{selector} {}",
            p.delete_name_cell
                .as_ref()
                .context("No reviewed stat API-name cell")?
        ),
    )
    .await?;
    ensure!(
        name_cell.text().await? == name,
        "Template mismatch: wrong stat row"
    );
    let button = crate::templates::unique(
        driver,
        &format!(
            "{selector} {}",
            p.delete
                .as_ref()
                .context("No reviewed stat deletion recipe")?
        ),
    )
    .await?;
    ensure!(
        button.is_displayed().await? && button.is_enabled().await?,
        "Stat Delete control is unavailable"
    );
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let dir = crate::paths::root()?
        .join("apps")
        .join(app.to_string())
        .join("schema-backups")
        .join(stamp.to_string());
    crate::paths::private_dir(&dir)?;
    std::fs::write(
        dir.join("before.json"),
        serde_json::to_vec_pretty(&json!({"app_id":app,"schema":before,"delete_stat":name}))?,
    )?;
    crate::recording::checkpoint(driver, app, "stat-delete-before").await?;
    crate::recording::event("stat.delete.begin", json!({"api_name":name,"backup":dir}))?;
    identity(driver, app, "stats").await?;
    // Reviewed DeleteStatClosure sends deletion immediately, without a confirm dialog.
    button.click().await?;
    let mut removed = false;
    for _ in 0..100 {
        if driver.find_all(By::Css(&selector)).await?.is_empty() {
            removed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    ensure!(
        removed,
        "Stat deletion outcome is uncertain; inspect current schema and backup before retrying"
    );
    driver.refresh().await?;
    page(driver, app, "stats").await?;
    let after = snapshot(driver, app).await?;
    std::fs::write(
        dir.join("after.json"),
        serde_json::to_vec_pretty(&json!({"app_id":app,"schema":after}))?,
    )?;
    verify_stat_deletion(&before, &after, name)?;
    crate::recording::event(
        "stat.delete.verified",
        json!({"api_name":name,"backup":dir}),
    )?;
    Ok(
        json!({"deleted":true,"api_name":name,"published":false,"verified_after_reload":true,"backup":dir}),
    )
}

fn verify_stat_deletion(before: &Value, after: &Value, name: &str) -> Result<()> {
    ensure!(
        find(&before["stats"], "stats", name)?.is_some(),
        "Missing deletion target in backup"
    );
    let mut expected = before.clone();
    expected["stats"]
        .as_array_mut()
        .context("Invalid stats schema")?
        .retain(|r| r["name"].as_str() != Some(name));
    ensure!(
        &expected == after,
        "Stat deletion readback mismatch: target still present or another schema value changed"
    );
    Ok(())
}
fn localized_map(v: &Value) -> BTreeMap<String, String> {
    if let Some(s) = v.as_str() {
        BTreeMap::from([("english".into(), s.into())])
    } else {
        v.as_object()
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default()
    }
}
fn verify(target: &Definition, raw: &Value, old: Option<&Value>) -> Result<()> {
    match target {
        Definition::Stats(s) => {
            ensure!(
                raw["name"] == s.api_name
                    && raw["type"] == s.kind
                    && permission(&raw["permission"]) == Some(s.permission)
                    && (s.kind == "AVGRATE"
                        || (flag(&raw["incrementonly"]) == s.increment_only
                            && flag(&raw["aggregated"]) == s.aggregated))
                    && raw["display"]["name"] == s.display_name,
                "Saved stat identity/settings differ"
            );
            if s.kind == "AVGRATE"
                && let Some(old) = old
            {
                for key in ["incrementonly", "aggregated"] {
                    ensure!(
                        raw[key] == old[key],
                        "An inactive stat setting changed: {key}"
                    );
                }
            }
            for (key, want) in [
                ("default", Some(s.default)),
                ("min", s.min),
                ("max", s.max),
                ("maxchange", s.max_change),
                ("windowsize", s.window),
            ] {
                if (key == "windowsize" && s.kind != "AVGRATE")
                    || (key == "maxchange" && s.kind == "AVGRATE")
                {
                    if let Some(old) = old {
                        ensure!(
                            raw[key] == old[key],
                            "An inactive stat setting changed: {key}"
                        );
                    }
                    continue;
                }
                ensure!(
                    raw[key].is_null()
                        || raw[key].as_str() == Some("")
                        || raw_num(&raw[key]).is_some(),
                    "Saved stat has an invalid numeric field: {key}"
                );
                ensure!(
                    raw_num(&raw[key]) == want
                        || (key == "default"
                            && want == Some(0.0)
                            && (raw[key].is_null() || raw[key].as_str() == Some(""))),
                    "Saved stat numeric field differs: {key}"
                );
            }
        }
        Definition::Achievements(a) => {
            ensure!(
                raw["api_name"] == a.api_name
                    && permission(&raw["permission"]) == Some(a.permission)
                    && flag(&raw["hidden"]) == a.hidden,
                "Saved achievement settings differ"
            );
            for (key, patch) in [
                ("display_name", &a.display_name),
                ("description", &a.description),
            ] {
                let mut expected = old.map(|o| localized_map(&o[key])).unwrap_or_default();
                expected.extend(patch.clone());
                let mut actual = localized_map(&raw[key]);
                if !expected.contains_key("token")
                    && let Some(token) = actual.remove("token")
                {
                    let suffix = if key == "display_name" {
                        "_NAME"
                    } else {
                        "_DESC"
                    };
                    ensure!(
                        token == format!("{}{suffix}", a.api_name),
                        "Unexpected generated localization token"
                    );
                }
                ensure!(
                    actual == expected,
                    "Saved achievement text differs or another translation changed: {key}"
                );
            }
            if let Some(p) = &a.progress {
                ensure!(
                    raw["progress"]["value"]["operand1"] == p.stat
                        && raw_num(&raw["progress"]["min_val"]) == Some(p.min)
                        && raw_num(&raw["progress"]["max_val"]) == Some(p.max),
                    "Saved achievement progress differs"
                );
            } else {
                ensure!(
                    raw["progress"].is_null() || raw["progress"] == false,
                    "Unexpected saved achievement progress"
                );
            }
            if let Some(old) = old {
                for key in ["icon", "icon_gray"] {
                    ensure!(
                        raw[key] == old[key],
                        "Achievement icon changed unexpectedly"
                    );
                }
            }
        }
    }
    Ok(())
}

pub async fn inspect(driver: &WebDriver, app: u32, section: &str) -> Result<Value> {
    page(driver, app, section).await?;
    let t = crate::templates::load()?;
    let root = crate::templates::unique(driver, &t.technical[section].root).await?;
    let report = driver
        .execute(include_str!("audit.js"), vec![root.to_json()?])
        .await?;
    Ok(
        json!({"section":section,"read_only":true,"report":report.json(),"unsupported":["Achievement sets and bulk import","Clearing individual achievement icons"]}),
    )
}

pub async fn current_section(driver: &WebDriver, app: u32) -> Result<&'static str> {
    let url = driver.current_url().await?;
    for section in ["stats", "achievements"] {
        if url.path() == format!("/apps/{section}/{app}") {
            return Ok(section);
        }
    }
    bail!("Not on a supported technical page")
}

async fn apply_definition(
    driver: &WebDriver,
    section: &str,
    prefix: &str,
    target: &Definition,
    existing: Option<&Value>,
) -> Result<()> {
    match target {
        Definition::Stats(s) => {
            for (k, v) in [
                ("type", json!(s.kind)),
                ("api_name", json!(s.api_name)),
                ("permission", json!(s.permission.to_string())),
                ("default", text_number(Some(s.default))),
                ("min", text_number(s.min)),
                ("max", text_number(s.max)),
                ("display_name", json!(s.display_name)),
            ] {
                fill(driver, section, prefix, k, v).await?;
            }
            if s.kind == "AVGRATE" {
                fill(driver, section, prefix, "window", text_number(s.window)).await?;
            } else {
                for (k, v) in [
                    ("increment_only", json!(s.increment_only)),
                    ("aggregated", json!(s.aggregated)),
                    ("max_change", text_number(s.max_change)),
                ] {
                    fill(driver, section, prefix, k, v).await?;
                }
            }
        }
        Definition::Achievements(a) => {
            for _ in 0..100 {
                let e = control(driver, section, prefix, "progress_stat").await?;
                if e.text().await?.lines().next() == Some("None") {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            for (k, v) in [
                ("api_name", json!(a.api_name)),
                ("permission", json!(a.permission.to_string())),
                ("hidden", json!(a.hidden)),
                (
                    "progress_stat",
                    json!(a.progress.as_ref().map(|p| p.stat.as_str()).unwrap_or("-1")),
                ),
                (
                    "progress_min",
                    text_number(Some(a.progress.as_ref().map_or(0.0, |p| p.min))),
                ),
                (
                    "progress_max",
                    text_number(Some(a.progress.as_ref().map_or(0.0, |p| p.max))),
                ),
            ] {
                fill(driver, section, prefix, k, v).await?;
            }
            localized(driver, prefix, "display_name", &a.display_name).await?;
            localized(driver, prefix, "description", &a.description).await?;
            for (key, suffix) in [("display_name", "_NAME"), ("description", "_DESC")] {
                let token = existing
                    .and_then(|v| v[key]["token"].as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{}{suffix}", a.api_name));
                localized(
                    driver,
                    prefix,
                    key,
                    &BTreeMap::from([("token".into(), token)]),
                )
                .await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stat_deletion_checks_links_and_preserves_other_records() {
        let mut before = json!({"stats":[{"name":"stat_test"},{"name":"stat_keep"}],"achievements":{"achievements":[{"api_name":"ACH_TEST","progress":{"value":{"operand1":"stat_test"}}}]}});
        assert!(ensure_stat_unlinked(&before, "stat_test").is_err());
        before["achievements"]["achievements"][0]["progress"] = json!(false);
        assert!(ensure_stat_unlinked(&before, "stat_test").is_ok());
        let mut after = before.clone();
        after["stats"] = json!([{"name":"stat_keep"}]);
        assert!(verify_stat_deletion(&before, &after, "stat_test").is_ok());
        assert!(verify_stat_deletion(&before, &before, "stat_test").is_err());
        after["achievements"]["achievements"] = json!([]);
        assert!(verify_stat_deletion(&before, &after, "stat_test").is_err());
    }
    #[test]
    fn deletion_requires_only_the_named_achievement_to_disappear() {
        let target = json!({"api_name":"ACH_TEST","stat_id":1,"bit_id":0,"icon":"old-icon"});
        let other = json!({"api_name":"ACH_KEEP","stat_id":1,"bit_id":1,"hidden":"1"});
        let before = json!({"stats":[{"name":"stat_test","type":"FLOAT"}],"achievements":{"achievements":[target,other.clone()],"languages":{"english":true}}});
        let mut after = before.clone();
        after["achievements"]["achievements"] = json!([other]);
        assert!(verify_achievement_deletion(&before, &after, "ACH_TEST").is_ok());
        assert!(verify_achievement_deletion(&before, &before, "ACH_TEST").is_err());
        assert!(verify_achievement_deletion(&before, &after, "ACH_MISSING").is_err());
        let mut changed = after.clone();
        changed["stats"][0]["type"] = json!("INT");
        assert!(verify_achievement_deletion(&before, &changed, "ACH_TEST").is_err());
        changed = after;
        changed["achievements"]["achievements"][0]["hidden"] = json!("0");
        assert!(verify_achievement_deletion(&before, &changed, "ACH_TEST").is_err());
    }
    fn stat() -> Stat {
        serde_json::from_value(json!({"api_name":"stat_rounds","type":"INT","display_name":"Rounds","increment_only":true,"min":0,"default":0})).unwrap()
    }
    fn achievement() -> Achievement {
        serde_json::from_value(json!({"api_name":"ACH_ROUNDS","display_name":{"english":"Round Trip"},"description":{"english":"Complete ten rounds."},"progress":{"stat":"stat_rounds","min":0,"max":10}})).unwrap()
    }
    #[test]
    fn rejects_invalid_stat_ranges_and_types() {
        let mut s = stat();
        s.default = 0.5;
        assert!(s.validate().is_err());
        s.default = 0.0;
        s.max = Some(-1.0);
        assert!(s.validate().is_err());
        s.max = None;
        s.kind = "FLOAT".into();
        s.default = f64::INFINITY;
        assert!(s.validate().is_err());
        s.default = 0.0;
        s.kind = "AVGRATE".into();
        s.window = Some(10.0);
        assert!(s.validate().is_err());
        s.increment_only = false;
        assert!(s.validate().is_ok());
        s.kind = "INT".into();
        s.window = None;
        s.default = i32::MAX as f64 + 1.0;
        assert!(s.validate().is_err());
    }
    #[test]
    fn verifies_active_values_and_preserves_inactive_settings() {
        let s = Definition::Stats(stat());
        let old = json!({"name":"stat_rounds","type":"INT","permission":"0","display":{"name":"Rounds"},"incrementonly":"1","aggregated":"0","default":"0","min":"0","windowsize":"10"});
        assert!(verify(&s, &old, Some(&old)).is_ok());
        let mut changed = old.clone();
        changed["windowsize"] = json!("11");
        assert!(verify(&s, &changed, Some(&old)).is_err());
        changed = old.clone();
        changed["default"] = json!("invalid");
        assert!(verify(&s, &changed, Some(&old)).is_err());
    }
    #[test]
    fn progress_requires_existing_compatible_stat() {
        let a = Definition::Achievements(achievement());
        let mut before = json!({"stats":[],"achievements":{"achievements":[]}});
        assert!(validate_target(&a, &before).is_err());
        before["stats"] = json!([{"stat_id":"1","name":"stat_rounds","type":"INT","permission":"0","min":"0","max":"100"}]);
        assert!(validate_target(&a, &before).is_ok());
        before["stats"][0]["permission"] = json!(1);
        assert!(validate_target(&a, &before).is_err());
        before["stats"][0]["permission"] = json!(0);
        before["stats"][0]["max"] = json!(5);
        assert!(validate_target(&a, &before).is_err());
        before["stats"][0]["max"] = json!(100);
        let mut fractional = achievement();
        fractional.progress.as_mut().unwrap().max = 1.5;
        assert!(validate_target(&Definition::Achievements(fractional), &before).is_err());
    }
    #[test]
    fn stat_updates_cannot_break_linked_achievements() {
        let before = json!({"stats":[{"name":"stat_rounds","type":"INT"}],"achievements":{"achievements":[{"permission":"0","progress":{"value":{"operand1":"stat_rounds"},"min_val":"0","max_val":"10"}}]}});
        let mut s = stat();
        assert!(validate_target(&Definition::Stats(s.clone()), &before).is_ok());
        s.max = Some(5.0);
        assert!(validate_target(&Definition::Stats(s.clone()), &before).is_err());
        s.max = None;
        s.permission = 1;
        assert!(validate_target(&Definition::Stats(s), &before).is_err());
    }
    #[test]
    fn readback_preserves_other_translations_and_icons() {
        let target = Definition::Achievements(achievement());
        let old = json!({"display_name":{"english":"Old","german":"Runde"},"description":{"english":"Old","german":"Zehn Runden."},"icon":"unchanged","icon_gray":"unchanged-gray"});
        let mut saved = json!({"api_name":"ACH_ROUNDS","permission":"0","hidden":"0","display_name":{"english":"Round Trip","german":"Runde","token":"ACH_ROUNDS_NAME"},"description":{"english":"Complete ten rounds.","german":"Zehn Runden.","token":"ACH_ROUNDS_DESC"},"progress":{"value":{"operand1":"stat_rounds"},"min_val":"0","max_val":"10"},"icon":"unchanged","icon_gray":"unchanged-gray"});
        assert!(verify(&target, &saved, Some(&old)).is_ok());
        saved["description"]["german"] = json!("Changed");
        assert!(verify(&target, &saved, Some(&old)).is_err());
        saved["description"]["german"] = old["description"]["german"].clone();
        saved["icon"] = json!("changed");
        assert!(verify(&target, &saved, Some(&old)).is_err());
    }
    #[test]
    fn rejects_selector_injection_and_malformed_definitions() {
        assert!(api_name("bad'] input").is_err());
        assert!(
            serde_json::from_value::<Stat>(
                json!({"api_name":"x","type":"INT","display_name":"X","publish":true})
            )
            .is_err()
        );
        let mut a = achievement();
        a.display_name.insert("english']".into(), "bad".into());
        assert!(a.validate().is_err());
        assert!(row_id("achievements", &json!({"stat_id":"1/2","bit_id":"0"})).is_err());
    }
}

pub async fn smoke(driver: &WebDriver) -> Result<()> {
    driver.goto("data:text/html,<div id='appStatDisplay'><table id='statTable'></table></div><div id='appAchievementDisplay'><table id='achievementTable'></table></div>").await?;
    driver
        .execute(include_str!("technical_fixture.js"), vec![])
        .await?;
    let stat: Stat = serde_json::from_value(
        json!({"api_name":"stat_rounds","type":"INT","display_name":"Rounds","increment_only":true,"min":0,"default":0}),
    )?;
    let achievement: Achievement = serde_json::from_value(
        json!({"api_name":"ACH_ROUNDS","display_name":{"english":"Round Trip 🛡️"},"description":{"english":"Complete ten rounds."},"progress":{"stat":"stat_rounds","min":0,"max":10}}),
    )?;
    for target in [
        Definition::Stats(stat),
        Definition::Achievements(achievement),
    ] {
        let section = target.section();
        let prefix = edit(driver, 0, section, None).await?;
        apply_definition(driver, section, &prefix, &target, None).await?;
        let t = crate::templates::load()?;
        crate::templates::unique(
            driver,
            &format!(
                "{} {}",
                t.technical[section].root, t.technical[section].save
            ),
        )
        .await?
        .click()
        .await?;
        let stored = driver
            .execute(
                "return window.fixtureRecords[arguments[0]]",
                vec![json!(section)],
            )
            .await?
            .json()
            .clone();
        verify(&target, &stored, None)?;
        let prefix = edit(driver, 0, section, Some(&stored)).await?;
        apply_definition(driver, section, &prefix, &target, Some(&stored)).await?;
        crate::templates::unique(
            driver,
            &format!(
                "{} {}",
                t.technical[section].root, t.technical[section].save
            ),
        )
        .await?
        .click()
        .await?;
        let stored_again = driver
            .execute(
                "return window.fixtureRecords[arguments[0]]",
                vec![json!(section)],
            )
            .await?
            .json()
            .clone();
        verify(&target, &stored_again, Some(&stored))?;
    }
    ensure!(
        driver
            .execute("return window.fixtureCreates", vec![])
            .await?
            .json()
            .as_u64()
            == Some(2),
        "Editing allocated duplicate records"
    );
    // A changed control type must stop the adapter before filling it.
    icons::smoke(driver).await?;
    let raw = driver
        .execute("return window.fixtureRecords.stats", vec![])
        .await?
        .json()
        .clone();
    let prefix = edit(driver, 0, "stats", Some(&raw)).await?;
    driver
        .execute(
            "document.getElementById('stat1_statapiname').type='password'",
            vec![],
        )
        .await?;
    ensure!(
        control(driver, "stats", &prefix, "api_name").await.is_err(),
        "Technical adapter accepted changed field semantics"
    );
    Ok(())
}
