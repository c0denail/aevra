//! Opt-in real WebView2 + real local 9B test. Makes public web requests only with --web.
use aevra_core::{
    db::Database,
    model::Engine,
    types::{AppSettings, GenerationEvent},
};
use std::{path::PathBuf, sync::Mutex};
use tauri::{ipc::Channel, Manager};
use tokio_util::sync::CancellationToken;
type CommandResult<T> = std::result::Result<T, String>;
struct ActiveRequest {
    cancel: CancellationToken,
}
#[derive(Default)]
struct RequestControl {
    active: Option<ActiveRequest>,
}
struct AppState {
    db: Mutex<Database>,
    data_dir: PathBuf,
    requests: Mutex<RequestControl>,
}
#[allow(dead_code)]
#[path = "../src/browser.rs"]
mod browser;

async fn checks(app: &tauri::AppHandle) -> std::result::Result<serde_json::Value, String> {
    let cancel = CancellationToken::new();
    let main = app.get_webview("main").ok_or("main view missing")?;
    let rect =
        serde_json::from_value(serde_json::json!({"x":630,"y":140,"width":550,"height":540}))
            .map_err(|e| e.to_string())?;
    browser::browser_layout(Some(rect), app.clone(), main.clone()).await?;
    let view = browser::ensure(app).await.map_err(|e| e.to_string())?;
    let mut first = browser::navigate(app, &view, "https://www.google.com/?hl=tr", &cancel)
        .await
        .map_err(|e| e.to_string())?;
    if first.blocked {
        return Err("Search provider requires human verification".into());
    }
    if first.search_fields.is_empty() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = Mutex::new(Some(tx));
        view.eval_with_callback("Array.from(document.querySelectorAll('input,textarea,[contenteditable=true]')).map(e=>({tag:e.tagName,type:e.type,name:e.name,id:e.id,disabled:e.disabled,readOnly:e.readOnly,display:getComputedStyle(e).display,visibility:getComputedStyle(e).visibility,hidden:!!e.closest('[hidden],[aria-hidden=true]'),label:e.getAttribute('aria-label'),form:e.form?{method:e.form.method,action:e.form.action,types:Array.from(e.form.querySelectorAll('input')).map(i=>i.type)}:null}))",move |s|{if let Some(tx)=tx.lock().unwrap().take(){let _=tx.send(s);}}).map_err(|e|e.to_string())?;
        let info = tokio::time::timeout(std::time::Duration::from_secs(4), rx)
            .await
            .map_err(|_| "Field inspection timed out")?
            .map_err(|e| e.to_string())?;
        return Err(format!("Search field not found in real browser: {info}"));
    }
    let field = &first.search_fields[0];
    browser::evaluate(&view,serde_json::json!({"action":"search","index":field.index,"query":"Rust ownership official documentation","expectedUrl":first.url}),&cancel).await.map_err(|e|e.to_string())?;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        if let Ok(page) = browser::snapshot(&view, &cancel).await {
            if page.url != first.url && !page.links.is_empty() {
                first = page;
                break;
            }
        }
    }
    if !first.search_results || first.links.is_empty() {
        return Err("Site search did not produce real result links".into());
    }
    // Remote WebViews must not be able to call any application command.
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Mutex::new(Some(tx));
    view.eval_with_callback("(()=>{window.__aevraSmokeIpc='pending';if(!window.__TAURI_INTERNALS__){window.__aevraSmokeIpc='denied';return true;}window.__TAURI_INTERNALS__.invoke('browser_status').then(()=>window.__aevraSmokeIpc='allowed',()=>window.__aevraSmokeIpc='denied');return true;})()",move |s|{if let Some(tx)=tx.lock().unwrap().take(){let _=tx.send(s);}}).map_err(|e|e.to_string())?;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(4), rx).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Mutex::new(Some(tx));
    view.eval_with_callback("window.__aevraSmokeIpc", move |s| {
        if let Some(tx) = tx.lock().unwrap().take() {
            let _ = tx.send(s);
        }
    })
    .map_err(|e| e.to_string())?;
    let ipc = tokio::time::timeout(std::time::Duration::from_secs(4), rx)
        .await
        .map_err(|_| "IPC test timed out")?
        .map_err(|e| e.to_string())?;
    if !ipc.contains("denied") {
        return Err(format!("Remote IPC was not denied: {ipc}"));
    }
    let mut sources = Vec::new();
    let browser_only = std::env::args().any(|arg| arg == "--browser-only");
    if browser_only {
        let link = first
            .links
            .iter()
            .find(|link| link.url.starts_with("https://doc.rust-lang.org/"))
            .or_else(|| first.links.first())
            .ok_or("No Google result link")?;
        let page = browser::navigate(app, &view, &link.url, &cancel)
            .await
            .map_err(|e| e.to_string())?;
        sources.push(
            aevra_core::browser::source(&page, 1)
                .ok_or("Google result did not yield article evidence")?,
        );
    } else {
        let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
        let mut engine = Engine::new(resources);
        let client = engine.start().await.map_err(|e| e.to_string())?;
        let channel = Channel::<GenerationEvent>::new(|_| Ok(()));
        let settings = AppSettings {
            research_source_limit: 2,
            ..AppSettings::default()
        };
        let result = browser::research(
            app,
            &client,
            "Rust ownership official documentation",
            "Rust ownership official documentation",
            &settings,
            cancel.clone(),
            &channel,
            &mut sources,
        )
        .await;
        engine.stop();
        result.map_err(|e| e.to_string())?;
    }
    if sources.is_empty() || sources.iter().any(|s| s.excerpt.chars().count() < 80) {
        return Err("No article evidence was read".into());
    }
    let stopped = CancellationToken::new();
    app.state::<AppState>().requests.lock().unwrap().active = Some(ActiveRequest {
        cancel: stopped.clone(),
    });
    let takeover = async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        browser::browser_takeover(app.clone(), main.clone())
    };
    let (navigation, takeover) = tokio::join!(
        browser::navigate(app, &view, "https://example.com/", &stopped),
        takeover
    );
    takeover?;
    app.state::<AppState>().requests.lock().unwrap().active = None;
    if !matches!(navigation, Err(aevra_core::Error::Cancelled)) {
        return Err("Cancellation not honored".into());
    }
    browser::browser_layout(None, app.clone(), main.clone()).await?;
    let status = browser::browser_status(app.state::<browser::BrowserState>(), main.clone())?;
    browser::browser_close(app.clone(), main)?;
    for _ in 0..20 {
        if app.get_webview("research-browser").is_none() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    if app.get_webview("research-browser").is_some() {
        return Err("Native browser did not close".into());
    }
    Ok(
        serde_json::json!({"ok":true,"provider":"Google","modelUsed":!browser_only,"nativeSiteSearch":true,"remoteIpcDenied":true,"cancellation":true,"closedOnLeave":true,"sources":sources,"browser":status}),
    )
}
fn main() {
    if !std::env::args().any(|arg| arg == "--web") {
        eprintln!("Explicit --web required; this test accesses public websites and starts the bundled 9B model.");
        std::process::exit(2);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let data = root.join("tmp/browser-smoke");
    std::fs::create_dir_all(&data).unwrap();
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows[0].visible = false;
    context.config_mut().app.windows[0].url =
        tauri::WebviewUrl::External("about:blank".parse().unwrap());
    let output = data.join("report.json");
    let app = tauri::Builder::default()
        .manage(browser::BrowserState::default())
        .setup(move |app| {
            app.manage(AppState {
                db: Mutex::new(Database::open(&data.join("test.sqlite3"))?),
                data_dir: data.clone(),
                requests: Mutex::new(RequestControl::default()),
            });
            let handle = app.handle().clone();
            let output = output.clone();
            tauri::async_runtime::spawn(async move {
                let result = checks(&handle).await;
                let ok = result.is_ok();
                let report =
                    result.unwrap_or_else(|error| serde_json::json!({"ok":false,"error":error}));
                std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
                println!("{report}");
                handle.exit(if ok { 0 } else { 1 });
            });
            Ok(())
        })
        .invoke_handler({
            let commands: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
                browser::browser_status,
                browser::browser_layout,
                browser::browser_control,
                browser::browser_takeover
            ];
            move |invoke| {
                if invoke.message.webview_ref().label() != "main" {
                    invoke.resolver.reject("Remote IPC denied");
                    true
                } else {
                    commands(invoke)
                }
            }
        })
        .build(context)
        .expect("Smoke application could not start");
    app.run(|_, _| {});
}
