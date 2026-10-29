use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use thirtyfour::prelude::*;

pub async fn identity(driver: &WebDriver, app: u32) -> Result<()> {
    let recipes = crate::templates::load()?;
    let recipe = recipes.tag_wizard.context("Missing Tag Wizard recipe")?;
    let url = driver.current_url().await?;
    ensure!(
        url.origin().ascii_serialization() == recipe.origin
            && url.path() == format!("{}{app}", recipe.path_prefix),
        "Open this app's Tag Wizard manually before using tags"
    );
    let result = driver.execute(
        "const a=document.querySelectorAll(arguments[0]); return a.length===1 && a[0].innerText.includes('('+arguments[1]+')') && !Array.from(document.querySelectorAll('input[type=password]')).some(e=>e.getClientRects().length);",
        vec![json!(recipe.identity), json!(app)],
    ).await?;
    ensure!(
        result.json() == &json!(true),
        "Tag Wizard identity mismatch"
    );
    Ok(())
}

pub async fn read(driver: &WebDriver, app: u32) -> Result<Value> {
    identity(driver, app).await?;
    let recipe = crate::templates::load()?
        .tag_wizard
        .context("Missing Tag Wizard recipe")?;
    let result = driver
        .execute(
            include_str!("tags_read.js"),
            vec![json!(recipe), json!(app)],
        )
        .await?;
    Ok(result.json().clone())
}

pub async fn smoke(driver: &WebDriver) -> Result<()> {
    driver
        .goto("data:text/html,<title>Tag fixture</title><div id='choices'></div>")
        .await?;
    let recipe = crate::templates::load()?
        .tag_wizard
        .context("Missing Tag Wizard recipe")?;
    let data = json!({"appid":42,"devTags":[{"tagid":19,"name":"Action","weight":60}],
        "communityTags":[],"tagNames":{"19":"Action","21":"Adventure"},
        "sessionid":"fixture-private-value"});
    driver.execute(
        "const s=document.createElement('script');s.type='application/json';s.textContent='initSurvey('+JSON.stringify(arguments[0])+');';document.body.append(s);",
        vec![data],
    ).await?;
    let result = driver
        .execute(include_str!("tags_read.js"), vec![json!(recipe), json!(42)])
        .await?;
    ensure!(
        result.json()["developer_tags"][0]["name"] == "Action"
            && result.json()["catalog"]
                .as_array()
                .is_some_and(|a| a.len() == 2)
            && !result.json().to_string().contains("fixture-private-value"),
        "Tag fixture read or privacy failed"
    );
    ensure!(
        driver
            .execute(include_str!("tags_read.js"), vec![json!(recipe), json!(43)])
            .await
            .is_err(),
        "Tag reader accepted another app"
    );
    driver
        .execute(
            "document.querySelector('script').textContent='initSurvey({});';",
            vec![],
        )
        .await?;
    ensure!(
        driver
            .execute(include_str!("tags_read.js"), vec![json!(recipe), json!(42)])
            .await
            .is_err(),
        "Tag reader accepted a changed schema"
    );
    Ok(())
}
