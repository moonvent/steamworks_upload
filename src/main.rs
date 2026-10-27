mod browser;
mod paths;
mod recording;
mod store;
mod templates;
use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use thirtyfour::prelude::*;

#[derive(Parser)]
#[command(
    version,
    about = "Audit and edit Steamworks store drafts using your own browser session"
)]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    app: Option<u32>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Initialize private app config (does not contact Steam).
    Init,
    /// Print private storage locations.
    Paths,
    Check,
    Open {
        #[arg(long, default_value = "http://127.0.0.1:9515")]
        webdriver: String,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long)]
        headless: bool,
        /// Record each authenticated operation using native Chrome video.
        #[arg(long)]
        video: bool,
        /// Read JSON requests and write JSON responses, one per line.
        #[arg(long)]
        json: bool,
    },
}
#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(tag = "command", rename_all = "kebab-case", deny_unknown_fields)]
enum Request {
    Audit,
    Read,
    Pull,
    Plan,
    Fill,
    Save,
    Discard,
    Inspect,
    SelfTest,
    Help,
    Quit,
    Fields {
        tab: String,
    },
    Tab {
        tab: String,
    },
    Get {
        tab: String,
        field: String,
    },
    Set {
        tab: String,
        field: String,
        value: Value,
    },
    Upload {
        tab: String,
        field: String,
        file: PathBuf,
    },
}
#[derive(Default)]
struct Pending {
    store: Option<(store::Store, store::Snapshot)>,
    fields: Vec<browser::Change>,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(run());
    runtime.shutdown_timeout(std::time::Duration::from_millis(100));
    result
}
fn config_path(cli: &Cli) -> Result<PathBuf> {
    if let Some(path) = &cli.config {
        return Ok(path.clone());
    }
    let active = paths::root()?.join("active-app");
    let app = if let Some(app) = cli.app {
        app
    } else {
        std::fs::read_to_string(&active)
            .context("Choose an app: ./run.sh --app YOUR_APP_ID")?
            .trim()
            .parse::<u32>()
            .context("Invalid active app")?
    };
    ensure!(app > 0, "AppID must be positive");
    let path = paths::config(app)?;
    if !path.exists() {
        let initial = store::Store {
            app_id: app,
            language: "english".into(),
            name: String::new(),
            short_description: String::new(),
            description_file: "about.bbcode".into(),
        };
        std::fs::write(&path, toml::to_string_pretty(&initial)?)?;
        std::fs::write(
            path.parent()
                .context("Invalid config path")?
                .join("about.bbcode"),
            "",
        )?;
    }
    std::fs::write(active, app.to_string())?;
    Ok(path)
}
async fn run() -> Result<()> {
    let cli = Cli::parse();
    if matches!(cli.command, Command::Paths) {
        println!(
            "{}",
            json!({"private_root":paths::root()?,"runtime":paths::runtime()?})
        );
        return Ok(());
    }
    let headless = matches!(cli.command, Command::Open { headless: true, .. });
    let config_path = if headless {
        None
    } else {
        Some(config_path(&cli)?)
    };
    let config = if let Some(path) = &config_path {
        Some(store::load(path)?)
    } else {
        None
    };
    if let (Some(app), Some(config)) = (cli.app, &config) {
        ensure!(app == config.app_id, "--app and config AppID disagree");
    }
    match cli.command {
        Command::Init => println!(
            "Config: {}",
            config_path.context("Missing config")?.display()
        ),
        Command::Check => {
            let config = config.context("Missing config")?;
            store::desired(&config_path.context("Missing path")?, &config)?;
            println!("{}", json!({"valid":true,"app_id":config.app_id}));
        }
        Command::Paths => {}
        Command::Open {
            webdriver,
            profile,
            headless,
            video,
            json: json_mode,
        } => {
            let profile = profile.unwrap_or(paths::root()?.join("browser-profile"));
            paths::private_dir(&profile)?;
            let mut caps = DesiredCapabilities::chrome();
            caps.add_arg(&format!(
                "--user-data-dir={}",
                std::fs::canonicalize(profile)?.display()
            ))?;
            if headless {
                caps.add_arg("--headless=new")?;
            }
            let driver = WebDriver::new(&webdriver, caps)
                .await
                .context("Cannot connect to ChromeDriver; use ./run.sh")?;
            let capture_dir = recording::init(video)?;
            eprintln!("Private recording: {}", capture_dir.display());
            let result=async{
                if headless{return smoke(&driver,video).await;}
                let config=config.context("Missing config")?;
                let config_path=config_path.context("Missing config path")?;
                driver.goto(format!("https://partner.steamgames.com/apps/landing/{}",config.app_id)).await?;
                if let Ok(link)=driver.find(By::Css("a[href*='/admin/game/edit/']")).await{link.click().await?;}
                if json_mode{println!("{}",json!({"event":"ready","app_id":config.app_id,"config":config_path}));}
                else{println!("Steamworks: sign in, then open Edit Store Page. help lists commands. Config: {}",config_path.display());}
                let mut pending=Pending::default();
                loop{
                    if !json_mode{print!("steamworks> ");std::io::stdout().flush()?;}
                    let input=tokio::task::spawn_blocking(||{let mut line=String::new();std::io::stdin().read_line(&mut line).map(|n|(n,line))});
                    let (n,line)=tokio::select!{_=tokio::signal::ctrl_c()=>break,line=input=>line??};
                    if n==0{break;}
                    let parsed=parse_request(&line,json_mode);
                    let request=match parsed{Ok(r)=>r,Err(e)=>{respond(json_mode,Err(e));continue;}};
                    if matches!(request,Request::Quit){respond(json_mode,Ok(json!({"closed":true})));break;}
                    let action=serde_json::to_value(&request)?["command"].as_str().unwrap_or("operation").to_string();
                    recording::event("operation.begin",json!({"command":action}))?;
                    recording::checkpoint(&driver,config.app_id,&format!("{action}-before")).await?;
                    let capture=recording::start_video(&driver,config.app_id,&action).await?;
                    let current=store::load(&config_path);
                    let result=match current{
                        Ok(current) if current.app_id==config.app_id=>handle(&driver,&config_path,&current,request,&mut pending).await,
                        Ok(_)=>Err(anyhow::anyhow!("AppID changed; restart the browser for another app")),
                        Err(e)=>Err(e),
                    };
                    if let Err(error)=recording::stop_video(&driver,capture).await{recording::event("video.error",json!({"error":error.to_string()}))?;eprintln!("Video could not be finalized: {error}");}
                    let phase=if result.is_ok(){"after"}else{"error"};
                    recording::event("operation.end",json!({"command":action,"ok":result.is_ok(),"error":result.as_ref().err().map(|e|format!("{e:#}"))}))?;
                    recording::checkpoint(&driver,config.app_id,&format!("{action}-{phase}")).await?;
                    respond(json_mode,result);
                }
                Ok(())
            }.await;
            let cleanup = driver.quit().await;
            result?;
            cleanup?;
        }
    }
    Ok(())
}
fn parse_request(line: &str, json_mode: bool) -> Result<Request> {
    let line = line.trim();
    if json_mode || line.starts_with('{') {
        let value: Value = serde_json::from_str(line)?;
        let object = value.as_object().context("Request must be a JSON object")?;
        let command = object
            .get("command")
            .and_then(Value::as_str)
            .context("Request needs command")?;
        let allowed: &[&str] = match command {
            "fields" | "tab" => &["command", "tab"],
            "get" => &["command", "tab", "field"],
            "set" => &["command", "tab", "field", "value"],
            "upload" => &["command", "tab", "field", "file"],
            _ => &["command"],
        };
        ensure!(
            object.keys().all(|key| allowed.contains(&key.as_str())),
            "Unexpected request field"
        );
        return Ok(serde_json::from_value(value)?);
    }
    match line {
        "audit" => Ok(Request::Audit),
        "read" => Ok(Request::Read),
        "pull" => Ok(Request::Pull),
        "plan" => Ok(Request::Plan),
        "fill" => Ok(Request::Fill),
        "save" => Ok(Request::Save),
        "discard" => Ok(Request::Discard),
        "inspect" => Ok(Request::Inspect),
        "self-test" => Ok(Request::SelfTest),
        "quit" => Ok(Request::Quit),
        "help" | "" => Ok(Request::Help),
        _ if line.starts_with("fields ") => Ok(Request::Fields {
            tab: line[7..].trim().into(),
        }),
        _ if line.starts_with("tab ") => Ok(Request::Tab {
            tab: line[4..].trim().into(),
        }),
        _ => bail!("Unknown command; type help or send a JSON request"),
    }
}
fn respond(json_mode: bool, result: Result<Value>) {
    match result {
        Ok(value) => {
            if json_mode {
                println!("{}", json!({"ok":true,"result":value}));
            } else if value.get("tabs").is_some() {
                println!("{}", browser::summary(&value));
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value).unwrap_or_default()
                );
            }
        }
        Err(e) => {
            if json_mode {
                println!("{}", json!({"ok":false,"error":format!("{e:#}")}));
            } else {
                eprintln!("{e:#}");
            }
        }
    }
}
async fn handle(
    driver: &WebDriver,
    path: &Path,
    config: &store::Store,
    request: Request,
    pending: &mut Pending,
) -> Result<Value> {
    match request {
        Request::Help => Ok(
            json!({"commands":["audit","fields basic","tab description","read","pull","plan","fill","save","discard","inspect","self-test","quit"],"json_examples":[{"command":"fields","tab":"basic"},{"command":"get","tab":"basic","field":"name:app[name]"},{"command":"set","tab":"basic","field":"name:app[name]","value":"Your Game"}],"save":"Saves a draft only. Publish stays manual."}),
        ),
        Request::Audit => browser::audit(driver, config.app_id).await,
        Request::Fields { tab } => browser::fields(driver, config.app_id, &tab).await,
        Request::Tab { tab } => {
            browser::tab(driver, config.app_id, &tab).await?;
            Ok(json!({"tab":tab}))
        }
        Request::Get { tab, field } => {
            Ok(json!({"field":field,"value":browser::get(driver,config.app_id,&tab,&field).await?}))
        }
        Request::Set { tab, field, value } => {
            ensure!(
                pending.store.is_none(),
                "Save or discard the text patch before editing generic fields"
            );
            let mut change = browser::set(driver, config.app_id, &tab, &field, &value).await?;
            if let Some(i) = pending
                .fields
                .iter()
                .position(|c| c.tab == tab && c.field == field)
            {
                change.before = pending.fields.remove(i).before;
            }
            pending.fields.push(change);
            Ok(json!({"filled":true,"saved":false,"pending_fields":pending.fields.len()}))
        }
        Request::Upload { tab, field, file } => {
            ensure!(
                pending.store.is_none() && pending.fields.is_empty(),
                "Save or discard edits before uploading"
            );
            browser::upload(driver, config.app_id, &tab, &field, &file).await
        }
        Request::Read => Ok(serde_json::to_value(store::read(driver, config).await?)?),
        Request::Pull => {
            ensure!(
                pending.store.is_none() && pending.fields.is_empty(),
                "Save or discard pending edits first"
            );
            store::pull(path, &store::read(driver, config).await?)?;
            Ok(json!({"downloaded":true,"config":path}))
        }
        Request::Plan => {
            let local = store::desired(path, config)?;
            let remote = store::read(driver, config).await?;
            store::validate_pair(&remote, &local)?;
            Ok(
                json!({"changed":remote!=local,"current":remote,"local":local,"pending_save":pending.store.is_some()||!pending.fields.is_empty()}),
            )
        }
        Request::Fill => {
            ensure!(
                pending.fields.is_empty(),
                "Save or discard generic field edits first"
            );
            let target = store::desired(path, config)?;
            if let Some((old, _)) = &pending.store {
                ensure!(
                    old.language == config.language,
                    "Save or discard before switching language"
                );
            }
            let old = store::read(driver, config).await?;
            if old != target {
                store::backup(&old)?;
                store::fill(driver, config, &target).await?;
                pending.store = Some((config.clone(), target));
            }
            Ok(json!({"filled":pending.store.is_some(),"saved":false}))
        }
        Request::Save => {
            if let Some((previous, target)) = &pending.store {
                ensure!(
                    store::desired(path, config)? == *target,
                    "Local texts changed after fill; run fill again"
                );
                store::save(driver, previous, target).await?;
                pending.store = None;
                Ok(json!({"saved":true,"published":false}))
            } else {
                let result = browser::save(driver, config.app_id, &pending.fields).await?;
                pending.fields.clear();
                Ok(result)
            }
        }
        Request::Discard => {
            store::identity(driver, config.app_id).await?;
            driver.refresh().await?;
            *pending = Pending::default();
            Ok(json!({"discarded":true}))
        }
        Request::SelfTest => {
            ensure!(
                pending.store.is_none() && pending.fields.is_empty(),
                "Save or discard edits before self-test"
            );
            store::roundtrip(driver, config).await?;
            Ok(json!({"verified":true,"saved":false}))
        }
        Request::Inspect => {
            store::identity(driver, config.app_id).await?;
            let result=driver.execute("return Array.from(document.querySelectorAll('div.tab[id^=tab_]')).map(e=>({tab:e.id.slice(4),title:e.innerText.trim()}));",vec![]).await?;
            Ok(result.json().clone())
        }
        Request::Quit => Ok(json!({"closed":true})),
    }
}
async fn smoke(driver: &WebDriver, video: bool) -> Result<()> {
    driver.goto("data:text/html,<title>Driver check</title><section id='test'><h2>Required<span class='required_status'><span>*</span></span></h2><input id='public' value=''><input id='password' type='password' value='secret'><input type='hidden' name='sessionid' value='secret'></section>").await?;
    ensure!(
        driver.title().await? == "Driver check",
        "Unexpected browser title"
    );
    let root = driver.find(By::Id("test")).await?;
    let report = driver
        .execute(include_str!("audit.js"), vec![root.to_json()?])
        .await?;
    ensure!(
        report.json()["required"][0]["state"] == "missing",
        "Required audit failed"
    );
    let output = report.json().to_string();
    ensure!(
        !output.contains("secret") && !output.contains("sessionid") && !output.contains("password"),
        "Audit leaked sensitive controls"
    );
    driver.execute("document.body.insertAdjacentHTML('beforeend', '<div id=tab_basic_content><input name=app[name] type=password></div>');",vec![]).await?;
    ensure!(
        templates::field(driver, "name").await.is_err(),
        "Template accepted a changed input type"
    );
    driver.execute("document.querySelector('#tab_basic_content').innerHTML='<input name=app[name]><input name=app[name]>';",vec![]).await?;
    ensure!(
        templates::field(driver, "name").await.is_err(),
        "Template accepted an ambiguous field"
    );
    let recording = if video {
        recording::probe(driver).await?
    } else {
        json!({"requested":false})
    };
    println!(
        "{}",
        json!({"ok":true,"browser":true,"audit_fixture":true,"recording":recording})
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_json_fields() {
        assert!(parse_request(r#"{"command":"audit","save":true}"#, true).is_err());
    }
    #[test]
    fn preserves_unicode_json_values() {
        let r = parse_request(
            r#"{"command":"set","tab":"basic","field":"name:app[name]","value":"Игра 🛡️"}"#,
            true,
        )
        .unwrap();
        assert!(matches!(r,Request::Set{value,..}if value=="Игра 🛡️"));
    }
}
