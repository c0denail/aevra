//! A separate, unprivileged WebView for visible research. Only fixed DOM operations cross into it.
use super::{AppState, CommandResult};
use aevra_core::{
    browser::{self, Action, Decision, Page},
    model::ModelClient,
    research::validate_url,
    types::{AppSettings, GenerationEvent, Source},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{
    ipc::Channel,
    webview::{NewWindowResponse, PageLoadEvent, PermissionResponse, WebviewBuilder},
    Emitter, Manager, State, Webview, WebviewUrl,
};
use tokio::sync::{oneshot, Mutex as AsyncMutex};
use tokio_util::sync::CancellationToken;

const LABEL: &str = "research-browser";
const DOM: &str = include_str!("browser_dom.js");

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserStatus {
    url: String,
    title: String,
    loading: bool,
    automating: bool,
    detail: String,
    steps: Vec<BrowserStep>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserStep {
    text: String,
    url: String,
}
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BrowserRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
#[derive(Default)]
pub struct BrowserState {
    status: Mutex<BrowserStatus>,
    layout: Mutex<Option<BrowserRect>>,
    creation: AsyncMutex<()>,
    finished: AtomicU64,
}

fn publish(app: &tauri::AppHandle, change: impl FnOnce(&mut BrowserStatus)) {
    let state = app.state::<BrowserState>();
    if let Ok(mut status) = state.status.lock() {
        change(&mut status);
        let _ = app.emit_to("main", "research-browser", status.clone());
    };
}
fn step(app: &tauri::AppHandle, text: String, url: &str) {
    publish(app, |s| {
        s.detail = text.clone();
        s.steps.push(BrowserStep {
            text,
            url: url.into(),
        });
        if s.steps.len() > 40 {
            s.steps.remove(0);
        }
    });
}
fn main_only(view: &Webview) -> CommandResult<()> {
    if view.label() != "main" {
        return Err("Bu işlem yalnızca Aevra arayüzünden kullanılabilir.".into());
    }
    Ok(())
}
fn browser_error(_: impl std::fmt::Display) -> Error {
    Error::Network("Araştırma tarayıcısına ulaşılamadı.".into())
}

pub(super) fn has_page(app: &tauri::AppHandle) -> bool {
    app.get_webview(LABEL)
        .and_then(|view| view.url().ok())
        .is_some_and(|url| validate_url(url.as_str()).is_ok())
}
pub(super) fn disable(app: &tauri::AppHandle) {
    if let Some(view) = app.get_webview(LABEL) {
        let _ = view.close();
    }
    publish(app, |status| *status = BrowserStatus::default());
}

pub(super) async fn ensure(app: &tauri::AppHandle) -> Result<Webview> {
    let state = app.state::<BrowserState>();
    let _guard = state.creation.lock().await;
    if let Some(view) = app.get_webview(LABEL) {
        return Ok(view);
    }
    let layout = *state.layout.lock().map_err(browser_error)?;
    let rect = layout.unwrap_or(BrowserRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    });
    let window = app
        .get_window("main")
        .ok_or_else(|| browser_error("window"))?;
    let navigation_app = app.clone();
    let loading_app = app.clone();
    let title_app = app.clone();
    let popup_app = app.clone();
    let builder = WebviewBuilder::new(LABEL, WebviewUrl::External("about:blank".parse().unwrap()))
        .data_directory(app.state::<AppState>().data_dir.join("research-browser"))
        .incognito(true)
        .focused(false)
        .general_autofill_enabled(false)
        .on_navigation(move |url| {
            let allowed = url.as_str() == "about:blank" || validate_url(url.as_str()).is_ok();
            if !allowed {
                step(
                    &navigation_app,
                    "Bu adres araştırma tarayıcısında açılamaz.".into(),
                    "",
                );
            }
            allowed
        })
        .on_page_load(move |_, event| {
            let loading = matches!(event.event(), PageLoadEvent::Started);
            if !loading {
                loading_app
                    .state::<BrowserState>()
                    .finished
                    .fetch_add(1, Ordering::SeqCst);
            }
            publish(&loading_app, |s| {
                s.loading = loading;
                s.url = if event.url().as_str() == "about:blank" {
                    String::new()
                } else {
                    event.url().to_string()
                };
                if loading {
                    s.title.clear();
                }
            });
        })
        .on_document_title_changed(move |_, title| {
            publish(&title_app, |s| s.title = title.chars().take(200).collect())
        })
        .on_new_window(move |url, _| {
            if validate_url(url.as_str()).is_ok() {
                let handle = popup_app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(view) = handle.get_webview(LABEL) {
                        let _ = view.navigate(url);
                    }
                });
            }
            NewWindowResponse::Deny
        })
        .on_download(|_, _| false)
        .on_permission_request(|_, _| PermissionResponse::Deny);
    let view = window
        .add_child(
            builder,
            tauri::LogicalPosition::new(rect.x, rect.y),
            tauri::LogicalSize::new(rect.width, rect.height),
        )
        .map_err(browser_error)?;
    if layout.is_none() {
        view.hide().map_err(browser_error)?;
    }
    Ok(view)
}

#[tauri::command]
pub async fn browser_layout(
    rect: Option<BrowserRect>,
    app: tauri::AppHandle,
    webview: Webview,
) -> CommandResult<()> {
    main_only(&webview)?;
    if let Some(rect) = rect {
        let window = app.get_window("main").ok_or("Ana pencere bulunamadı.")?;
        let size = window
            .inner_size()
            .map_err(|e| e.to_string())?
            .to_logical::<f64>(window.scale_factor().map_err(|e| e.to_string())?);
        if ![rect.x, rect.y, rect.width, rect.height]
            .iter()
            .all(|v| v.is_finite())
            || rect.x < 0.0
            || rect.y < 0.0
            || rect.width < 1.0
            || rect.height < 1.0
            || rect.x + rect.width > size.width + 2.0
            || rect.y + rect.height > size.height + 2.0
        {
            return Err("Tarayıcı panelinin boyutu geçersiz.".into());
        }
    }
    *app.state::<BrowserState>()
        .layout
        .lock()
        .map_err(|_| "Tarayıcı kilitli.")? = rect;
    if let Some(rect) = rect {
        if let Some(view) = app.get_webview(LABEL) {
            view.set_bounds(tauri::Rect {
                position: tauri::LogicalPosition::new(rect.x, rect.y).into(),
                size: tauri::LogicalSize::new(rect.width, rect.height).into(),
            })
            .map_err(|e| e.to_string())?;
            view.show().map_err(|e| e.to_string())?;
        }
    } else if let Some(view) = app.get_webview(LABEL) {
        view.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
pub fn browser_status(
    state: State<'_, BrowserState>,
    webview: Webview,
) -> CommandResult<BrowserStatus> {
    main_only(&webview)?;
    Ok(state
        .status
        .lock()
        .map_err(|_| "Tarayıcı kilitli.")?
        .clone())
}
#[tauri::command]
pub fn browser_close(app: tauri::AppHandle, webview: Webview) -> CommandResult<()> {
    main_only(&webview)?;
    if app
        .state::<BrowserState>()
        .status
        .lock()
        .map_err(|_| "Tarayıcı kilitli.")?
        .automating
    {
        return Err("Önce araştırmayı durdurun.".into());
    }
    disable(&app);
    Ok(())
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ManualAction {
    Navigate { url: String },
    Back,
    Forward,
    Reload,
    Clear,
}
#[tauri::command]
pub async fn browser_control(
    action: ManualAction,
    app: tauri::AppHandle,
    webview: Webview,
) -> CommandResult<()> {
    main_only(&webview)?;
    if !app
        .state::<AppState>()
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .settings()
        .map_err(|e| e.to_string())?
        .web_research_enabled
    {
        return Err("Web araştırması ayarlardan kapatılmış.".into());
    }
    if app
        .state::<BrowserState>()
        .status
        .lock()
        .map_err(|_| "Tarayıcı kilitli.")?
        .automating
    {
        return Err("Önce Kontrolü al düğmesini kullanın.".into());
    }
    let view = ensure(&app).await.map_err(|e| e.to_string())?;
    match action {
        ManualAction::Navigate { url } => {
            let target = validate_url(&url).map_err(|e| e.to_string())?;
            view.navigate(target).map_err(|e| e.to_string())?;
        }
        ManualAction::Back => view.eval("history.back()").map_err(|e| e.to_string())?,
        ManualAction::Forward => view.eval("history.forward()").map_err(|e| e.to_string())?,
        ManualAction::Reload => view.reload().map_err(|e| e.to_string())?,
        ManualAction::Clear => {
            view.clear_all_browsing_data().map_err(|e| e.to_string())?;
            view.close().map_err(|e| e.to_string())?;
            publish(&app, |s| *s = BrowserStatus::default());
        }
    }
    Ok(())
}
#[tauri::command]
pub fn browser_takeover(app: tauri::AppHandle, webview: Webview) -> CommandResult<()> {
    main_only(&webview)?;
    if let Some(active) = &app
        .state::<AppState>()
        .requests
        .lock()
        .map_err(|_| "İstek kilitli.")?
        .active
    {
        active.cancel.cancel();
    }
    step(
        &app,
        "Kontrol sende. Devam etmek için sohbete yeni bir istek yazabilirsin.".into(),
        "",
    );
    // Keep automating=true until the cancelled worker has actually left its loop.
    Ok(())
}

pub(super) async fn evaluate(
    view: &Webview,
    command: serde_json::Value,
    cancel: &CancellationToken,
) -> Result<serde_json::Value> {
    let (tx, rx) = oneshot::channel();
    let tx = Mutex::new(Some(tx));
    view.eval_with_callback(format!("({DOM})({command})"), move |value| {
        if let Ok(mut tx) = tx.lock() {
            if let Some(tx) = tx.take() {
                let _ = tx.send(value);
            }
        }
    })
    .map_err(browser_error)?;
    let result = tokio::select! { _=cancel.cancelled()=>return Err(Error::Cancelled), result=tokio::time::timeout(Duration::from_secs(4),rx)=>result.map_err(|_|Error::Network("Sayfa okuma süresi doldu.".into()))?.map_err(browser_error)? };
    if result.len() > 160_000 {
        return Err(Error::Network("Sayfa okuma sınırı aşıldı.".into()));
    }
    let value: serde_json::Value = serde_json::from_str(&result)?;
    if let Some(error) = value.get("error").and_then(|v| v.as_str()) {
        return Err(Error::Network(error.chars().take(200).collect()));
    }
    Ok(value)
}
pub(super) async fn snapshot(view: &Webview, cancel: &CancellationToken) -> Result<Page> {
    let result = evaluate(view, serde_json::json!({"action":"snapshot"}), cancel).await?;
    if result.get("ready").and_then(|value| value.as_str()) == Some("loading") {
        return Err(Error::Network("Sayfa henüz hazırlanıyor.".into()));
    }
    let page: Page = serde_json::from_value(result)?;
    if view.url().map_err(browser_error)?.as_str() != page.url {
        return Err(Error::Network("Sayfa okuma sırasında değişti.".into()));
    }
    browser::normalize_page(page)
}
async fn wait_page(
    app: &tauri::AppHandle,
    view: &Webview,
    finished: u64,
    previous: &str,
    cancel: &CancellationToken,
) -> Result<Page> {
    let start = Instant::now();
    loop {
        tokio::select! {_=cancel.cancelled()=>return Err(Error::Cancelled),_=tokio::time::sleep(Duration::from_millis(350))=>{}}
        let loaded = app.state::<BrowserState>().finished.load(Ordering::SeqCst) > finished;
        let current = view.url().map_err(browser_error)?;
        // The native page-load event handles full navigation; URL changes cover client-side search.
        if loaded
            || (current.as_str() != previous && start.elapsed() > Duration::from_millis(700))
            || start.elapsed() > Duration::from_secs(16)
        {
            if let Ok(page) = snapshot(view, cancel).await {
                publish(app, |s| {
                    s.url = page.url.clone();
                    s.title = page.title.clone();
                });
                return Ok(page);
            }
        }
        if start.elapsed() > Duration::from_secs(20) {
            return Err(Error::Network(
                "Sayfa yüklenemedi. Tarayıcıdan kontrol edip yeniden deneyebilirsin.".into(),
            ));
        }
    }
}
pub(super) async fn navigate(
    app: &tauri::AppHandle,
    view: &Webview,
    url: &str,
    cancel: &CancellationToken,
) -> Result<Page> {
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let target = validate_url(url)?;
    let before = view.url().map_err(browser_error)?.to_string();
    let finished = app.state::<BrowserState>().finished.load(Ordering::SeqCst);
    view.navigate(target).map_err(browser_error)?;
    wait_page(app, view, finished, &before, cancel).await
}
struct ResearchGuard(tauri::AppHandle);
impl Drop for ResearchGuard {
    fn drop(&mut self) {
        publish(&self.0, |s| s.automating = false);
    }
}

async fn visit(
    app: &tauri::AppHandle,
    view: &Webview,
    url: &str,
    cancel: &CancellationToken,
    visited: &mut HashSet<String>,
) -> Result<Option<Page>> {
    visited.insert(url.into());
    match navigate(app, view, url, cancel).await {
        Ok(page) => Ok(Some(page)),
        Err(Error::Cancelled) => Err(Error::Cancelled),
        Err(_) => {
            step(
                app,
                "Bu sayfa okunamadı; diğer kaynaklar inceleniyor.".into(),
                url,
            );
            Ok(None)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn research(
    app: &tauri::AppHandle,
    client: &ModelClient,
    question: &str,
    query: &str,
    settings: &AppSettings,
    cancel: CancellationToken,
    on_event: &Channel<GenerationEvent>,
    sources: &mut Vec<Source>,
) -> Result<()> {
    let view = ensure(app).await?;
    publish(app, |s| {
        s.automating = true;
        s.steps.clear();
        s.detail = "Araştırma başlıyor…".into();
    });
    let _guard = ResearchGuard(app.clone());
    let outcome = tokio::time::timeout(
        Duration::from_secs(180),
        research_inner(
            app, &view, client, question, query, settings, &cancel, on_event, sources,
        ),
    )
    .await;
    let outcome = match outcome {
        Ok(result) => result,
        Err(_) if !sources.is_empty() => Ok(()),
        Err(_) => Err(Error::Network(
            "Araştırma süresi doldu. Tarayıcıdaki sayfadan devam edebilirsin.".into(),
        )),
    };
    if matches!(outcome, Err(Error::Cancelled)) {
        let _ = view.eval("window.stop()");
        publish(app, |s| s.loading = false);
    }
    let detail = match &outcome {
        Ok(_) => "Kaynaklar incelendi. Yanıt hazırlanıyor…".into(),
        Err(Error::Cancelled) => "Araştırma durduruldu. Kontrol sende.".into(),
        Err(error) => error.to_string(),
    };
    step(app, detail, "");
    outcome
}
#[allow(clippy::too_many_arguments)]
async fn research_inner(
    app: &tauri::AppHandle,
    view: &Webview,
    client: &ModelClient,
    question: &str,
    query: &str,
    settings: &AppSettings,
    cancel: &CancellationToken,
    on_event: &Channel<GenerationEvent>,
    sources: &mut Vec<Source>,
) -> Result<()> {
    let current_requested = browser::current_page_requested(question);
    let mut page = if let Some(url) = browser::requested_url(question) {
        step(app, "İstediğin site açılıyor".into(), &url);
        navigate(app, view, &url, cancel).await?
    } else if current_requested && validate_url(view.url().map_err(browser_error)?.as_str()).is_ok()
    {
        snapshot(view, cancel).await?
    } else {
        let url = browser::search_url(query)?;
        step(app, format!("Aranıyor · {query}"), &url);
        navigate(app, view, &url, cancel).await?
    };
    let mut visited = HashSet::new();
    let mut frontier = VecDeque::new();
    let mut known = HashSet::new();
    let mut queries = HashSet::from([format!("web:{query}")]);
    let started = Instant::now();
    for _ in 0..12 {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        if sources.len() >= settings.research_source_limit
            || started.elapsed() > Duration::from_secs(180)
        {
            break;
        }
        visited.insert(page.url.clone());
        if page.blocked {
            if let Some(next) = frontier
                .iter()
                .find(|link: &&browser::Link| !visited.contains(&link.url))
                .cloned()
            {
                step(
                    app,
                    format!("Başka kaynak açılıyor · {}", next.title),
                    &next.url,
                );
                if let Some(next) = visit(app, view, &next.url, cancel, &mut visited).await? {
                    page = next;
                }
                continue;
            }
            return if sources.is_empty() {
                Err(Error::Network("Site doğrulama istiyor. Kontrolü alıp tarayıcıdaki doğrulamayı tamamladıktan sonra yeniden araştırabilirsin.".into()))
            } else {
                Ok(())
            };
        }
        if page.search_results {
            for link in &page.links {
                if known.insert(link.url.clone()) {
                    frontier.push_back(link.clone());
                }
            }
        }
        let already_read = sources.iter().any(|s| s.url == page.url);
        if already_read {
            if let Some(next) = frontier
                .iter()
                .find(|link| !visited.contains(&link.url))
                .cloned()
            {
                step(app, format!("Açılıyor · {}", next.title), &next.url);
                if let Some(next) = visit(app, view, &next.url, cancel, &mut visited).await? {
                    page = next;
                }
                continue;
            }
            break;
        }
        let decision = if already_read {
            None
        } else {
            let _ = on_event.send(GenerationEvent::Status {
                text: format!("Sayfa inceleniyor · {}", page.title),
            });
            let result = tokio::select! {_=cancel.cancelled()=>return Err(Error::Cancelled),result=tokio::time::timeout(Duration::from_secs(40),browser::decide(client,question,&page,sources,settings,cancel.clone()))=>result};
            match result {
                Ok(Ok(value)) => {
                    browser::checked_decision(value, question, &page, sources, &visited).ok()
                }
                _ => None,
            }
        };
        let decision = decision
            .filter(|d| match d.action {
                Action::Finish => !sources.is_empty(),
                Action::Search => {
                    queries.len() < 3 && !queries.contains(&format!("web:{}", d.query))
                }
                Action::SiteSearch => {
                    queries.len() < 3
                        && !queries.contains(&format!("site:{}:{}", page.url, d.query))
                }
                _ => true,
            })
            .or_else(|| {
                if !already_read && browser::source(&page, 1).is_some() {
                    Some(Decision {
                        action: Action::Read,
                        index: 0,
                        query: String::new(),
                    })
                } else {
                    page.links
                        .iter()
                        .find(|link| !visited.contains(&link.url))
                        .map(|link| Decision {
                            action: Action::Open,
                            index: link.index,
                            query: String::new(),
                        })
                }
            });
        let Some(decision) = decision else {
            if let Some(next) = frontier
                .iter()
                .find(|link| !visited.contains(&link.url))
                .cloned()
            {
                step(app, format!("Açılıyor · {}", next.title), &next.url);
                if let Some(next) = visit(app, view, &next.url, cancel, &mut visited).await? {
                    page = next;
                }
                continue;
            }
            break;
        };
        if view.url().map_err(browser_error)?.as_str() != page.url {
            page = snapshot(view, cancel).await?;
            step(app, "Sayfa değişti; yeniden inceleniyor.".into(), &page.url);
            continue;
        }
        match decision.action {
            Action::Finish => break,
            Action::Read => {
                if let Some(source) = browser::source(&page, sources.len() + 1) {
                    step(app, format!("Okundu · {}", source.title), &source.url);
                    sources.push(source);
                    let _ = on_event.send(GenerationEvent::Sources {
                        sources: sources.clone(),
                    });
                }
                if sources.len() < settings.research_source_limit {
                    if let Some(next) = frontier
                        .iter()
                        .find(|link| !visited.contains(&link.url))
                        .cloned()
                    {
                        step(app, format!("Açılıyor · {}", next.title), &next.url);
                        if let Some(next) =
                            visit(app, view, &next.url, cancel, &mut visited).await?
                        {
                            page = next;
                        }
                    } else {
                        break;
                    }
                }
            }
            Action::Open => {
                if let Some(link) = page
                    .links
                    .iter()
                    .find(|link| link.index == decision.index)
                    .cloned()
                {
                    step(app, format!("Açılıyor · {}", link.title), &link.url);
                    if let Some(next) = visit(app, view, &link.url, cancel, &mut visited).await? {
                        page = next;
                    }
                }
            }
            Action::Search => {
                queries.insert(format!("web:{}", decision.query));
                let url = browser::search_url(&decision.query)?;
                step(app, format!("Yeni arama · {}", decision.query), &url);
                page = navigate(app, view, &url, cancel).await?;
            }
            Action::SiteSearch => {
                queries.insert(format!("site:{}:{}", page.url, decision.query));
                step(
                    app,
                    format!("Site içinde aranıyor · {}", decision.query),
                    &page.url,
                );
                let finished = app.state::<BrowserState>().finished.load(Ordering::SeqCst);
                evaluate(view,serde_json::json!({"action":"search","index":decision.index,"query":decision.query,"expectedUrl":page.url}),cancel).await?;
                page = wait_page(app, view, finished, &page.url, cancel).await?;
            }
        }
    }
    if sources.is_empty() {
        return Err(Error::Network("Okunabilir bir kaynak bulunamadı. Tarayıcıdan bir sayfa açıp ‘Bu sayfayı incele’ diyebilirsin.".into()));
    }
    Ok(())
}
