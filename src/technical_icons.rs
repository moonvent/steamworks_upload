use super::*;
use std::path::{Path, PathBuf};

fn local_path(path: &Path) -> Result<PathBuf> {
    let expanded = if let Ok(rest) = path.strip_prefix("~") {
        dirs::home_dir()
            .context("Cannot locate home directory")?
            .join(rest)
    } else {
        path.to_path_buf()
    };
    expanded
        .canonicalize()
        .context("Cannot locate achievement icon file")
}

fn cdn_url(app: u32, url: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(url)?;
    ensure!(
        url.scheme() == "https"
            && matches!(
                url.host_str(),
                Some(
                    "shared.fastly.steamstatic.com"
                        | "shared.akamai.steamstatic.com"
                        | "cdn.cloudflare.steamstatic.com"
                        | "cdn.akamai.steamstatic.com"
                )
            )
            && (url
                .path()
                .starts_with(&format!("/community_assets/images/apps/{app}/"))
                || url
                    .path()
                    .starts_with(&format!("/steamcommunity/public/images/apps/{app}/")))
            && url.username().is_empty()
            && url.password().is_none(),
        "Unrecognized achievement icon CDN URL; review recipe before downloading"
    );
    Ok(url)
}

async fn download(app: u32, url: &str, path: &Path) -> Result<()> {
    let url = cdn_url(app, url)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let mut response = client.get(url).send().await?.error_for_status()?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            bytes.len() + chunk.len() <= 10_000_000,
            "Icon download exceeds local 10 MB limit"
        );
        bytes.extend_from_slice(&chunk);
    }
    ensure!(!bytes.is_empty(), "CDN returned an empty achievement icon");
    std::fs::write(path, bytes)?;
    Ok(())
}

async fn form(driver: &WebDriver, app: u32, raw: &Value, gray: bool) -> Result<WebElement> {
    let t = crate::templates::load()?;
    let p = t
        .achievement_icons
        .context("No reviewed achievement icon recipe")?;
    let id = row_id("achievements", raw)?;
    let selector = format!("{} #{id} {}", t.technical["achievements"].root, p.form);
    let forms = driver.find_all(By::Css(selector)).await?;
    ensure!(
        forms.len() == 2,
        "Template mismatch: expected exactly two achievement upload forms"
    );
    let target = if gray {
        "achievement_gray"
    } else {
        "achievement"
    };
    let mut found = None;
    for f in forms {
        let kind = f.find_all(By::Css("input[name='requestType']")).await?;
        ensure!(
            kind.len() == 1,
            "Template mismatch: upload request type is ambiguous"
        );
        if kind[0].value().await?.as_deref() == Some(target) {
            ensure!(
                found.is_none(),
                "Template mismatch: duplicate icon upload form"
            );
            found = Some(f);
        }
    }
    let f = found.context("Template mismatch: requested icon form is missing")?;
    let expected_callback = format!(
        "AchievementImageUploadCallbackClosure( {app}, {}, {}, {} )",
        id_number(&raw["stat_id"])?,
        id_number(&raw["bit_id"])?,
        gray
    );
    let valid = driver.execute(r#"
      const [f,origin,action,app,stat,bit,kind,limit,callback]=arguments;
      const one=(name,value)=>{const els=f.querySelectorAll('input[name="'+name+'"]');return els.length===1 && els[0].type==='hidden' && els[0].value===value};
      return f.action===origin+action && f.method.toLowerCase()==='post'
        && f.enctype==='multipart/form-data'
        && one('appID',String(app)) && one('statID',stat) && one('bit',bit)
        && one('requestType',kind) && one('MAX_FILE_SIZE',String(limit))
        && f.querySelectorAll('input[type="hidden"][name="sessionid"]').length===1
        && (f.getAttribute('onsubmit')||'').replace(/\s+/g,'').includes(callback.replace(/\s+/g,''));
    "#, vec![f.to_json()?,json!(t.page.origin),json!(p.action),json!(app),json!(id_number(&raw["stat_id"])?),json!(id_number(&raw["bit_id"])?),json!(target),json!(p.max_bytes),json!(expected_callback)]).await?;
    ensure!(
        valid.json().as_bool() == Some(true),
        "Template mismatch: achievement upload form semantics changed"
    );
    ensure!(
        f.find_all(By::Css(&p.file)).await?.len() == 1
            && f.find_all(By::Css(&p.submit)).await?.len() == 1,
        "Template mismatch: ambiguous icon file or Upload controls"
    );
    Ok(f)
}

fn verify(before: &Value, after: &Value, name: &str, key: &str, url: &str) -> Result<()> {
    ensure!(!url.is_empty(), "Uploaded icon has an empty persisted URL");
    let mut expected = before.clone();
    let rows = expected["achievements"]["achievements"]
        .as_array_mut()
        .context("Invalid achievement array")?;
    let record = rows
        .iter_mut()
        .find(|r| r["api_name"].as_str() == Some(name))
        .context("Missing achievement in backup")?;
    record[key] = json!(url);
    ensure!(
        expected == *after,
        "Icon readback mismatch: another field, record or stat changed"
    );
    Ok(())
}

fn upload_success(response: &Value) -> bool {
    response["success"].as_bool() == Some(true) || raw_num(&response["success"]) == Some(1.0)
}

pub async fn upload_icon(
    driver: &WebDriver,
    app: u32,
    name: &str,
    file: &Path,
    gray: bool,
    attempted: &mut bool,
) -> Result<Value> {
    api_name(name)?;
    let file = local_path(file)?;
    let t = crate::templates::load()?;
    let p = t
        .achievement_icons
        .context("No reviewed achievement icon recipe")?;
    ensure!(
        std::fs::metadata(&file)?.len() <= p.max_bytes,
        "Achievement icon exceeds native 3000000-byte file limit"
    );
    let validation = crate::achievement_icons::validate(&file)?;
    let before = snapshot(driver, app).await?;
    let raw = find(&before["achievements"], "achievements", name)?
        .context("Achievement must exist before icon upload")?;
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
            &json!({"app_id":app,"schema":before,"icon_upload":{"api_name":name,"gray":gray,"file":file,"validation":validation}}),
        )?,
    )?;
    for key in ["icon", "icon_gray"] {
        if let Some(url) = raw[key].as_str().filter(|s| !s.is_empty()) {
            download(app, url, &dir.join(format!("before-{key}.image"))).await?;
        }
    }
    // Upload the private validated copy so source-file edits cannot change the request.
    let upload_file = dir.join(if validation["format"] == "PNG" {
        "upload.png"
    } else {
        "upload.jpg"
    });
    std::fs::copy(&file, &upload_file)?;
    ensure!(
        std::fs::metadata(&upload_file)?.len() <= p.max_bytes,
        "Icon file changed during backup"
    );
    crate::achievement_icons::validate(&upload_file)?;
    *attempted = true;
    edit(driver, app, "achievements", Some(raw)).await?;
    let f = form(driver, app, raw, gray).await?;
    let input = f.find(By::Css(&p.file)).await?;
    input
        .send_keys(upload_file.to_str().context("Icon path is not UTF-8")?)
        .await?;
    identity(driver, app, "achievements").await?;
    // Observe the native upload acknowledgement while preserving its original callback.
    driver.execute(r#"
      const [fn,app,stat,bit,gray]=arguments;
      const original=window[fn];if(typeof original!=='function')throw Error('Native upload callback changed');
      window.__steamprojectIconUpload=null;
      window[fn]=function(a,s,b,g){
        if(a!==app || Number(s)!==Number(stat) || Number(b)!==Number(bit) || g!==gray)throw Error('Unexpected icon upload target');
        const callback=original.apply(this,arguments);
        window[fn]=original;
        return function(raw){
          try{const result=JSON.parse(raw);window.__steamprojectIconUpload={success:result.success,message:result.message};}
          catch(e){window.__steamprojectIconUpload={success:0,message:'Invalid native upload acknowledgement'};}
          return callback.apply(this,arguments);
        };
      };
    "#,vec![json!(p.callback),json!(app),json!(id_number(&raw["stat_id"])?),json!(id_number(&raw["bit_id"])?),json!(gray)]).await?;
    crate::recording::checkpoint(driver, app, "achievement-icon-selected").await?;
    crate::recording::event(
        "achievement.icon.upload.begin",
        json!({"api_name":name,"gray":gray,"backup":dir}),
    )?;
    f.find(By::Css(&p.submit)).await?.click().await?;
    let mut acknowledgement = Value::Null;
    for _ in 0..300 {
        acknowledgement = driver
            .execute("return window.__steamprojectIconUpload", vec![])
            .await?
            .json()
            .clone();
        if !acknowledgement.is_null() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    std::fs::write(
        dir.join("upload-response.json"),
        serde_json::to_vec_pretty(&acknowledgement)?,
    )?;
    ensure!(
        upload_success(&acknowledgement),
        "Native icon upload did not acknowledge success; inspect private backup and current schema before retrying"
    );
    let key = if gray { "icon_gray" } else { "icon" };
    let image_selector = format!(
        "{} #{}_{}",
        t.technical["achievements"].root,
        row_id("achievements", raw)?,
        key
    );
    let mut preview_url = String::new();
    let mut refreshed = false;
    for _ in 0..100 {
        let image = crate::templates::unique(driver, &image_selector).await?;
        preview_url = image.attr("src").await?.unwrap_or_default();
        let row = crate::templates::unique(
            driver,
            &format!(
                "{} #{}",
                t.technical["achievements"].root,
                row_id("achievements", raw)?
            ),
        )
        .await?;
        if row
            .attr("class")
            .await?
            .unwrap_or_default()
            .split_whitespace()
            .any(|c| c == "dirty")
            && !preview_url.is_empty()
        {
            refreshed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    ensure!(
        refreshed,
        "Native icon preview did not refresh; inspect current schema before retrying"
    );
    cdn_url(app, &preview_url)?;
    // Native Upload writes the image draft immediately; no unrelated text Save is needed.
    driver.refresh().await?;
    page(driver, app, "achievements").await?;
    let after = snapshot(driver, app).await?;
    std::fs::write(
        dir.join("after.json"),
        serde_json::to_vec_pretty(&json!({"app_id":app,"schema":after}))?,
    )?;
    verify(&before, &after, name, key, &preview_url)?;
    download(app, &preview_url, &dir.join("after-icon.image")).await?;
    crate::achievement_icons::validate(&dir.join("after-icon.image"))?;
    crate::recording::event(
        "achievement.icon.upload.verified",
        json!({"api_name":name,"gray":gray,"backup":dir}),
    )?;
    Ok(
        json!({"uploaded":true,"saved":true,"verified_after_reload":true,"published":false,"api_name":name,"gray":gray,"url":preview_url,"backup":dir,"validation":validation}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_native_boolean_and_numeric_acknowledgements() {
        assert!(upload_success(&json!({"success":true})));
        assert!(upload_success(&json!({"success":1})));
        assert!(!upload_success(&json!({"success":false})));
        assert!(!upload_success(&json!({"success":0})));
        assert!(!upload_success(&json!({"success":"true"})));
        assert!(!upload_success(&Value::Null));
    }
    #[test]
    fn upload_verification_preserves_other_icon_and_stats() {
        let before = json!({"stats":[{"name":"stat_rounds"}],"achievements":{"languages":{"english":true},"achievements":[{"api_name":"ACH_ROUNDS","icon":"old","icon_gray":"gray","hidden":"0"}]}});
        let mut after = before.clone();
        after["achievements"]["achievements"][0]["icon"] = json!("new");
        assert!(verify(&before, &after, "ACH_ROUNDS", "icon", "new").is_ok());
        assert!(verify(&before, &before, "ACH_ROUNDS", "icon", "new").is_err());
        after["achievements"]["achievements"][0]["icon_gray"] = json!("changed");
        assert!(verify(&before, &after, "ACH_ROUNDS", "icon", "new").is_err());
        assert!(
            cdn_url(
                42,
                "https://example.com/community_assets/images/apps/42/icon.jpg"
            )
            .is_err()
        );
        assert!(
            cdn_url(
                42,
                "https://shared.fastly.steamstatic.com/community_assets/images/apps/43/icon.jpg"
            )
            .is_err()
        );
    }
}

pub(super) async fn smoke(driver: &WebDriver) -> Result<()> {
    let raw = driver
        .execute("return window.fixtureRecords.achievements", vec![])
        .await?
        .json()
        .clone();
    edit(driver, 0, "achievements", Some(&raw)).await?;
    let t = crate::templates::load()?;
    let p = t.achievement_icons.context("Missing icon recipe")?;
    for gray in [false, true] {
        let f = form(driver, 0, &raw, gray).await?;
        let path = crate::paths::runtime()?.join(if gray {
            "fixture-icon-gray.png"
        } else {
            "fixture-icon.png"
        });
        image::DynamicImage::new_rgb8(64, 64).save(&path)?;
        f.find(By::Css(&p.file))
            .await?
            .send_keys(path.to_str().context("Invalid fixture path")?)
            .await?;
        let valid = driver
            .execute(
                "const f=arguments[0];return f.querySelector('input[type=file]').files.length===1",
                vec![f.to_json()?],
            )
            .await?;
        ensure!(
            valid.json().as_bool() == Some(true),
            "Fixture file selection failed"
        );
        driver
            .execute(
                "arguments[0].action='https://example.com/upload'",
                vec![f.to_json()?],
            )
            .await?;
        ensure!(
            form(driver, 0, &raw, gray).await.is_err(),
            "Icon adapter accepted an untrusted upload destination"
        );
        driver
            .execute(
                "arguments[0].action='https://partner.steamgames.com/images/uploadachievement'",
                vec![f.to_json()?],
            )
            .await?;
    }
    let f = form(driver, 0, &raw, false).await?;
    driver
        .execute(
            "arguments[0].querySelector('input[name=appID]').value='99'",
            vec![f.to_json()?],
        )
        .await?;
    ensure!(
        form(driver, 0, &raw, false).await.is_err(),
        "Icon adapter accepted a different AppID"
    );
    Ok(())
}
