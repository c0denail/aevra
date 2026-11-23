use aevra_core::{
    attachments::{self, Attachment},
    db::Database,
    model::{Engine, ModelClient, MODEL_NAME},
    prompts, research,
    types::*,
    Error,
};
use std::{collections::HashSet, io::Read, path::PathBuf, sync::Mutex};
use tauri::{ipc::Channel, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex as AsyncMutex;
use tokio_util::sync::CancellationToken;
mod browser;
mod memory_service;

struct ActiveRequest {
    id: String,
    cancel: CancellationToken,
}
#[derive(Default)]
struct RequestControl {
    active: Option<ActiveRequest>,
    early_cancel: HashSet<String>,
}
struct AppState {
    db: Mutex<Database>,
    engine: AsyncMutex<Engine>,
    generation: AsyncMutex<()>,
    requests: Mutex<RequestControl>,
    status: Mutex<EngineStatus>,
    shutdown: CancellationToken,
    data_dir: PathBuf,
    resources: PathBuf,
    pending_attachments: Mutex<Vec<Attachment>>,
    attachment_selection: AsyncMutex<()>,
    memory_work: Mutex<Option<CancellationToken>>,
}
type CommandResult<T> = std::result::Result<T, String>;
async fn foreground_guard(state: &AppState) -> CommandResult<tokio::sync::MutexGuard<'_, ()>> {
    memory_service::cancel_work(state);
    tokio::time::timeout(std::time::Duration::from_secs(2), state.generation.lock())
        .await
        .map_err(|_| "Önce devam eden yanıtı durdurun.".into())
}

fn update_status(state: &AppState, status: EngineStatus) {
    if let Ok(mut current) = state.status.lock() {
        *current = status;
    }
}
async fn ensure_model(state: &AppState) -> aevra_core::Result<ModelClient> {
    let preference = state
        .db
        .lock()
        .map_err(|_| Error::Validation("Veritabanı kilitli.".into()))?
        .settings()?
        .compute_device;
    let mut engine = state.engine.lock().await;
    if engine.status().state != "ready" || engine.requested_device() != preference {
        update_status(
            state,
            EngineStatus {
                state: "starting".into(),
                detail: "Aevra hazırlanıyor…".into(),
                model: MODEL_NAME.into(),
                requested_device: preference,
                ..EngineStatus::default()
            },
        );
    }
    let result = tokio::select! {
        _ = state.shutdown.cancelled() => { engine.stop(); Err(Error::Cancelled) },
        result = engine.start_with_device(preference) => result,
    };
    match &result {
        Ok(_) => update_status(state, engine.status()),
        Err(error) => update_status(
            state,
            EngineStatus {
                state: "error".into(),
                detail: error.to_string(),
                model: MODEL_NAME.into(),
                requested_device: preference,
                ..EngineStatus::default()
            },
        ),
    }
    result
}

#[tauri::command]
async fn open_source(
    url: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    webview: tauri::Webview,
) -> CommandResult<()> {
    research::validate_url(&url).map_err(|e| e.to_string())?;
    let saved = state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .has_source(&url)
        .map_err(|e| e.to_string())?;
    if !saved {
        return Err("Kaynak, yanıt kaydedildikten sonra açılabilir.".into());
    }
    browser::browser_control(browser::ManualAction::Navigate { url }, app, webview).await
}
#[tauri::command]
fn list_conversations(state: State<'_, AppState>) -> CommandResult<Vec<Conversation>> {
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .list()
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn create_conversation(
    mode: Mode,
    memory_disabled: Option<bool>,
    state: State<'_, AppState>,
) -> CommandResult<Conversation> {
    let db = state.db.lock().map_err(|_| "Veritabanı kilitli.")?;
    let item = db.create(mode).map_err(|e| e.to_string())?;
    if memory_disabled.unwrap_or(false) {
        db.conversation_memory(&item.id, true)
            .map_err(|e| e.to_string())?;
    }
    db.get(&item.id).map_err(|e| e.to_string())
}
#[tauri::command]
fn get_messages(
    conversation_id: String,
    state: State<'_, AppState>,
) -> CommandResult<Vec<Message>> {
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .messages(&conversation_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn delete_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    let _guard = foreground_guard(&state).await?;
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .delete(&conversation_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn archive_conversation(
    conversation_id: String,
    archived: bool,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    let _guard = foreground_guard(&state).await?;
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .archive(&conversation_id, archived)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn archive_all(state: State<'_, AppState>) -> CommandResult<()> {
    let _guard = foreground_guard(&state).await?;
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .archive_all()
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn clear_history(state: State<'_, AppState>) -> CommandResult<()> {
    let _guard = foreground_guard(&state).await?;
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .clear_history()
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> CommandResult<AppSettings> {
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .settings()
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn save_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> CommandResult<()> {
    memory_service::cancel_work(&state);
    settings.validate().map_err(|e| e.to_string())?;
    let previous = state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .settings()
        .map_err(|e| e.to_string())?;
    if !settings.web_research_enabled {
        browser::disable(&app);
    }
    if settings.compute_device == previous.compute_device {
        return state
            .db
            .lock()
            .map_err(|_| "Veritabanı kilitli.")?
            .save_settings(&settings)
            .map_err(|e| e.to_string());
    }
    let _guard = tokio::time::timeout(std::time::Duration::from_secs(2), state.generation.lock())
        .await
        .map_err(|_| "Donanımı değiştirmeden önce devam eden yanıtı durdurun.")?;
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .save_settings(&settings)
        .map_err(|e| e.to_string())?;
    if let Err(error) = ensure_model(&state).await {
        state
            .db
            .lock()
            .map_err(|_| "Veritabanı kilitli.")?
            .save_settings(&previous)
            .map_err(|e| e.to_string())?;
        let restored = ensure_model(&state).await;
        return Err(if restored.is_ok() {
            format!("{error} Önceki donanım seçimi korundu.")
        } else {
            format!("{error} Önceki model de yeniden başlatılamadı.")
        });
    }
    Ok(())
}
#[tauri::command]
async fn engine_status(state: State<'_, AppState>) -> CommandResult<EngineStatus> {
    if let Ok(mut engine) = state.engine.try_lock() {
        let status = engine.status();
        let current = state.status.lock().map_err(|_| "Durum okunamadı.")?.clone();
        if current.state != "error" {
            update_status(&state, status);
        }
    }
    Ok(state.status.lock().map_err(|_| "Durum okunamadı.")?.clone())
}
#[tauri::command]
fn storage_info(state: State<'_, AppState>) -> CommandResult<StorageInfo> {
    let (conversation_count, message_count) = state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .counts()
        .map_err(|e| e.to_string())?;
    let database_bytes = ["aevra.sqlite3", "aevra.sqlite3-wal", "aevra.sqlite3-shm"]
        .iter()
        .map(|file| {
            std::fs::metadata(state.data_dir.join(file))
                .map(|m| m.len())
                .unwrap_or(0)
        })
        .sum();
    let model_bytes = std::fs::read_dir(state.resources.join("models"))
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|e| e.path().extension().is_some_and(|x| x == "gguf"))
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0);
    Ok(StorageInfo {
        data_dir: state.data_dir.display().to_string(),
        database_bytes,
        model_bytes,
        conversation_count,
        message_count,
    })
}
#[tauri::command]
fn open_data_folder(app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    app.opener()
        .open_path(state.data_dir.to_string_lossy(), None::<&str>)
        .map_err(|_| "Veri klasörü açılamadı.".into())
}
#[tauri::command]
async fn export_history(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Option<String>> {
    let backup = state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .export_history()
        .map_err(|e| e.to_string())?;
    let destination = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Sohbetleri dışa aktar")
            .add_filter("Aevra yedeği", &["json"])
            .set_file_name("Aevra-sohbetler.json")
            .blocking_save_file()
    })
    .await
    .map_err(|_| "Dosya penceresi açılamadı.")?;
    let Some(destination) = destination else {
        return Ok(None);
    };
    let destination = destination
        .into_path()
        .map_err(|_| "Dosya yolu geçersiz.")?;
    if destination
        .extension()
        .and_then(|e| e.to_str())
        .is_none_or(|e| !e.eq_ignore_ascii_case("json"))
    {
        return Err("Yedek dosyasının uzantısı .json olmalı.".into());
    }
    std::fs::write(
        &destination,
        serde_json::to_vec_pretty(&backup).map_err(|e| e.to_string())?,
    )
    .map_err(|_| "Yedek dosyası yazılamadı.")?;
    Ok(Some(destination.display().to_string()))
}
#[tauri::command]
async fn import_history(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Option<usize>> {
    let source = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Sohbet yedeğini seç")
            .add_filter("Aevra yedeği", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|_| "Dosya penceresi açılamadı.")?;
    let Some(source) = source else {
        return Ok(None);
    };
    let source = source.into_path().map_err(|_| "Dosya yolu geçersiz.")?;
    let file = std::fs::File::open(source).map_err(|_| "Yedek dosyası açılamadı.")?;
    let mut bytes = Vec::new();
    file.take(50_000_001)
        .read_to_end(&mut bytes)
        .map_err(|_| "Yedek okunamadı.")?;
    if bytes.len() > 50_000_000 {
        return Err("Yedek dosyası 50 MB sınırını aşıyor.".into());
    }
    let backup: HistoryExport =
        serde_json::from_slice(&bytes).map_err(|_| "Geçerli bir Aevra JSON yedeği seçin.")?;
    let _guard = foreground_guard(&state).await?;
    let count = state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .import_history(backup)
        .map_err(|e| e.to_string())?;
    Ok(Some(count))
}
#[tauri::command]
async fn export_message_pdf(
    conversation_id: String,
    message_id: String,
    math_assets: Vec<aevra_core::math::MathAsset>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Option<String>> {
    let (title, message) = {
        let db = state.db.lock().map_err(|_| "Veritabanı kilitli.")?;
        let chat = db.get(&conversation_id).map_err(|e| e.to_string())?;
        let message = db
            .messages(&conversation_id)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|m| m.id == message_id && m.role == "assistant" && !m.content.trim().is_empty())
            .ok_or_else(|| "Kaydedilecek yanıt bulunamadı.".to_string())?;
        (chat.title, message)
    };
    let file_name = format!(
        "{}.pdf",
        title
            .chars()
            .filter(|c| !c.is_control() && !"<>:\"/\\|?*".contains(*c))
            .take(60)
            .collect::<String>()
            .trim_end_matches([' ', '.'])
    );
    let destination = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Yanıtı PDF olarak kaydet")
            .add_filter("PDF belgesi", &["pdf"])
            .set_file_name(&file_name)
            .blocking_save_file()
    })
    .await
    .map_err(|_| "Dosya penceresi açılamadı.")?;
    let Some(destination) = destination else {
        return Ok(None);
    };
    let path = destination
        .into_path()
        .map_err(|_| "Dosya yolu geçersiz.")?;
    if !path
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("pdf"))
    {
        return Err("Dosya adının uzantısı .pdf olmalı.".into());
    }
    let resources = state.resources.clone();
    let saved = path.display().to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = aevra_core::pdf_export::create_pdf_with_math(
            &title,
            &message.content,
            &message.status,
            &message.sources,
            &resources,
            &math_assets,
        )?;
        aevra_core::pdf_export::write_atomic(&path, &bytes)
    })
    .await
    .map_err(|_| "PDF oluşturma işlemi tamamlanamadı.")?
    .map_err(|e| e.to_string())?;
    Ok(Some(saved))
}
#[tauri::command]
fn cancel_generation(request_id: String, state: State<'_, AppState>) -> CommandResult<()> {
    let mut control = state.requests.lock().map_err(|_| "İstek kilitli.")?;
    if let Some(active) = control.active.as_ref().filter(|r| r.id == request_id) {
        active.cancel.cancel();
    } else {
        if control.early_cancel.len() >= 32 {
            control.early_cancel.clear();
        }
        control.early_cancel.insert(request_id);
    }
    Ok(())
}
#[tauri::command]
async fn select_attachments(
    current_ids: Vec<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Vec<Attachment>> {
    let _selection = state
        .attachment_selection
        .try_lock()
        .map_err(|_| "Dosyalar zaten hazırlanıyor.")?;
    if current_ids.len() > attachments::MAX_FILES {
        return Err("En fazla 4 dosya eklenebilir.".into());
    }
    let mut current = {
        let pending = state
            .pending_attachments
            .lock()
            .map_err(|_| "Dosya ekleri kilitli.")?;
        current_ids
            .iter()
            .map(|id| {
                pending
                    .iter()
                    .find(|a| &a.id == id)
                    .cloned()
                    .ok_or_else(|| "Dosya eki bulunamadı. Yeniden seçin.".to_string())
            })
            .collect::<CommandResult<Vec<_>>>()?
    };
    let files = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Aevra'ya yerel dosya ekle")
            .add_filter(
                "Görsel, PDF, Word, metin, kod veya ZIP",
                &[
                    "png", "jpg", "jpeg", "webp", "pdf", "doc", "docx", "txt", "md", "csv", "json",
                    "xml", "yaml", "yml", "toml", "rs", "py", "js", "ts", "tsx", "jsx", "html",
                    "css", "sql", "c", "cpp", "h", "cs", "java", "go", "zip",
                ],
            )
            .blocking_pick_files()
    })
    .await
    .map_err(|_| "Dosya penceresi açılamadı.")?;
    let Some(files) = files else {
        return Ok(current);
    };
    if files.len() + current.len() > attachments::MAX_FILES {
        return Err("Bir mesajda en fazla 4 dosya eklenebilir.".into());
    }
    let resources = state.resources.clone();
    let read = tauri::async_runtime::spawn_blocking(move || {
        files
            .into_iter()
            .map(|f| {
                let path = f
                    .into_path()
                    .map_err(|_| Error::Validation("Dosya yolu okunamadı.".into()))?;
                attachments::read_attachment(&path, &resources)
            })
            .collect::<aevra_core::Result<Vec<_>>>()
    })
    .await
    .map_err(|_| "Dosyalar hazırlanamadı.")?
    .map_err(|e| e.to_string())?;
    current.extend(read);
    attachments::validate_batch(&current).map_err(|e| e.to_string())?;
    *state
        .pending_attachments
        .lock()
        .map_err(|_| "Dosya ekleri kilitli.")? = current.clone();
    Ok(current)
}
#[tauri::command]
fn discard_attachments(ids: Vec<String>, state: State<'_, AppState>) -> CommandResult<()> {
    state
        .pending_attachments
        .lock()
        .map_err(|_| "Dosya ekleri kilitli.")?
        .retain(|f| !ids.contains(&f.id));
    Ok(())
}
#[tauri::command]
async fn generate(
    request: GenerateRequest,
    on_event: Channel<GenerationEvent>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Message> {
    memory_service::cancel_work(&state);
    let _guard = tokio::time::timeout(std::time::Duration::from_secs(2), state.generation.lock())
        .await
        .map_err(|_| "Başka bir yanıt hazırlanıyor.")?;
    let files = {
        let pending = state
            .pending_attachments
            .lock()
            .map_err(|_| "Dosya ekleri kilitli.")?;
        request
            .attachment_ids
            .iter()
            .map(|id| {
                pending
                    .iter()
                    .find(|f| &f.id == id)
                    .cloned()
                    .ok_or_else(|| "Dosya eki bulunamadı. Dosyayı yeniden seçin.".to_string())
            })
            .collect::<CommandResult<Vec<_>>>()?
    };
    attachments::validate_batch(&files).map_err(|e| e.to_string())?;
    let (conversation, history, settings) = {
        let mut db = state.db.lock().map_err(|_| "Veritabanı kilitli.")?;
        let conversation = db
            .get(&request.conversation_id)
            .map_err(|e| e.to_string())?;
        let history = db
            .messages(&request.conversation_id)
            .map_err(|e| e.to_string())?;
        let settings = db.settings().map_err(|e| e.to_string())?;
        prompts::validate(&request, conversation.mode).map_err(|e| e.to_string())?;
        db.begin_turn_with_attachments(
            &request.conversation_id,
            &request.request_id,
            request.text.trim(),
            &files,
        )
        .map_err(|e| e.to_string())?;
        (conversation, history, settings)
    };
    state
        .pending_attachments
        .lock()
        .map_err(|_| "Dosya ekleri kilitli.")?
        .retain(|f| !request.attachment_ids.contains(&f.id));
    let cancel = CancellationToken::new();
    {
        let mut control = state.requests.lock().map_err(|_| "İstek kilitli.")?;
        if control.early_cancel.remove(&request.request_id) {
            cancel.cancel();
        }
        control.active = Some(ActiveRequest {
            id: request.request_id.clone(),
            cancel: cancel.clone(),
        });
    }
    let mut partial = String::new();
    let mut sources = Vec::new();
    let memory_source = aevra_core::memory::eligible(
        &request.text,
        conversation.mode,
        !files.is_empty()
            || history
                .iter()
                .rev()
                .find(|m| !m.attachments.is_empty())
                .is_some(),
    );
    let forget = memory_source
        .as_deref()
        .map(aevra_core::memory::forget_keys)
        .unwrap_or_default();
    if settings.memory_enabled && !conversation.memory_disabled && !forget.is_empty() {
        let mut db = state.db.lock().map_err(|_| "Veritabanı kilitli.")?;
        for key in &forget {
            db.delete_memory(key).map_err(|e| e.to_string())?;
        }
        use tauri::Emitter;
        let _ = app.emit(
            "memory-updated",
            serde_json::json!({"batchId":"","count":0}),
        );
    }
    let outcome: aevra_core::Result<()> = async {
        if cancel.is_cancelled() { return Err(Error::Cancelled); }
        let _ = on_event.send(GenerationEvent::Status { text: "Aevra hazırlanıyor…".into() });
        let client = tokio::select! { _ = cancel.cancelled() => return Err(Error::Cancelled), client = ensure_model(&state) => client? };
        if conversation.mode == Mode::Research {
            let cached=aevra_core::research_chat::cached_sources(&history,settings.research_source_limit);
            let _=on_event.send(GenerationEvent::Status{text:"Konuşmanın bağlamı değerlendiriliyor…".into()});
            let mut plan=aevra_core::research_chat::plan(&client,request.text.trim(),&history,&cached,&settings,cancel.clone()).await?;
            if settings.web_research_enabled && (aevra_core::browser::requested_url(&request.text).is_some() || (aevra_core::browser::current_page_requested(&request.text) && browser::has_page(&app))) {
                plan.search=true;
                if plan.query.is_empty(){plan.query=aevra_core::research_chat::safe_query("",&request.text,&cached);}
            }
            sources=if plan.search{
                let _ = on_event.send(GenerationEvent::Status { text:format!("Web araştırılıyor · {}",plan.query) });
                let mut fresh=Vec::new();
                let result=browser::research(&app,&client,request.text.trim(),&plan.query,&settings,cancel.clone(),&on_event,&mut fresh).await;
                sources=aevra_core::research_chat::merge_sources(fresh.clone(),cached.clone(),settings.research_source_limit);
                let _=on_event.send(GenerationEvent::Sources{sources:sources.clone()});
                result?;
                aevra_core::research_chat::merge_sources(fresh,cached,settings.research_source_limit)
            }else{let _=on_event.send(GenerationEvent::Status{text:"Önceki konuşma ve kaynaklarla yanıt hazırlanıyor…".into()});cached};
            let _ = on_event.send(GenerationEvent::Sources { sources: sources.clone() });
        }
        let _ = on_event.send(GenerationEvent::Status { text: "Aevra yanıtlıyor…".into() });
        let answer = if conversation.mode==Mode::Translate && !files.is_empty(){
            aevra_core::translation::translate_files(&client,&files,request.target_language.as_deref().unwrap_or("Türkçe"),&settings,cancel.clone(),|text|{partial.push_str(&text);if on_event.send(GenerationEvent::Delta{text}).is_err(){cancel.cancel();}},|text|{let _=on_event.send(GenerationEvent::Status{text});}).await?
        }else{
        let mut messages = prompts::build_with_attachments(conversation.mode, &request, &history, &sources, &settings, &files)?;
        if settings.memory_enabled&&!conversation.memory_disabled{
            let items=state.db.lock().map_err(|_|Error::Validation("Veritabanı kilitli.".into()))?.memory_state()?.items;
            messages[0].content.push_str(&aevra_core::memory::context(&items,&request.text,conversation.mode));
        } else if conversation.mode!=Mode::Translate { messages[0].content.push_str("\nBu sohbette kişisel hafıza erişimi kapalı. Bilgi kaydettiğini, sildiğini veya hafızadan okuduğunu iddia etme; kayıt yönetimi ayarlardaki Kişisel hafıza ekranındadır."); }
        if !forget.is_empty()&&settings.memory_enabled&&!conversation.memory_disabled{messages[0].content.push_str("\nUygulama bu mesajdaki açık unutma isteğine uyan hafıza alanlarını sildi ve otomatik yeniden öğrenmeyi engelledi. İşlemi kısaca belirt.");}
        client.generate_with_progress(messages, cancel.clone(), &settings, |text| {
            partial.push_str(&text);
            if on_event.send(GenerationEvent::Delta { text }).is_err() { cancel.cancel(); }
        },|text|{let _=on_event.send(GenerationEvent::Status{text});}).await?};
        partial = answer;
        if cancel.is_cancelled() { return Err(Error::Cancelled); }
        Ok(())
    }.await;
    let status = match &outcome {
        Ok(_) => "complete",
        Err(Error::Cancelled) => "interrupted",
        Err(Error::OutputLimit) => "limited",
        Err(_) => "failed",
    };
    let saved = state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .finish_turn(&request.conversation_id, &partial, sources, status)
        .map_err(|e| e.to_string());
    state.requests.lock().map_err(|_| "İstek kilitli.")?.active = None;
    let message = saved?;
    if outcome.is_ok() && forget.is_empty() {
        if let Some(source) = memory_source {
            state
                .db
                .lock()
                .map_err(|_| "Veritabanı kilitli.")?
                .enqueue_memory(&request.conversation_id, &request.request_id, &source)
                .map_err(|e| e.to_string())?;
        }
    }
    drop(_guard);
    memory_service::spawn(app);
    match outcome {
        Ok(_) | Err(Error::Cancelled) | Err(Error::OutputLimit) => Ok(message),
        Err(error) => Err(error.to_string()),
    }
}

pub fn run() {
    let app = tauri::Builder::default()
        .manage(browser::BrowserState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let resources = if cfg!(debug_assertions) {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
            } else {
                app.path().resource_dir()?.join("resources")
            };
            app.manage(AppState {
                db: Mutex::new(Database::open(&data.join("aevra.sqlite3"))?),
                engine: AsyncMutex::new(Engine::new(resources.clone())),
                generation: AsyncMutex::new(()),
                requests: Mutex::new(RequestControl::default()),
                status: Mutex::new(EngineStatus {
                    state: "starting".into(),
                    detail: "Aevra hazırlanıyor…".into(),
                    model: MODEL_NAME.into(),
                    ..EngineStatus::default()
                }),
                shutdown: CancellationToken::new(),
                data_dir: data,
                resources,
                pending_attachments: Mutex::new(vec![]),
                attachment_selection: AsyncMutex::new(()),
                memory_work: Mutex::new(None),
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                let _ = ensure_model(&state).await;
                memory_service::spawn(handle);
            });
            Ok(())
        })
        .invoke_handler({
            let commands: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
                open_source,
                browser::browser_layout,
                browser::browser_status,
                browser::browser_close,
                browser::browser_control,
                browser::browser_takeover,
                list_conversations,
                create_conversation,
                get_messages,
                delete_conversation,
                archive_conversation,
                archive_all,
                clear_history,
                get_settings,
                save_settings,
                engine_status,
                storage_info,
                open_data_folder,
                export_history,
                import_history,
                generate,
                cancel_generation,
                select_attachments,
                discard_attachments,
                export_message_pdf,
                memory_service::get_memories,
                memory_service::edit_memory,
                memory_service::delete_memory,
                memory_service::clear_memories,
                memory_service::allow_memory,
                memory_service::undo_memories,
                memory_service::set_conversation_memory
            ];
            move |invoke| {
                // Remote research pages never get access to application commands, including
                // custom commands that do not participate in plugin capability permissions.
                if invoke.message.webview_ref().label() != "main" {
                    invoke
                        .resolver
                        .reject("Web sayfalarının Aevra komutlarına erişimi kapalı.");
                    return true;
                }
                commands(invoke)
            }
        })
        .build(tauri::generate_context!())
        .expect("Aevra başlatılamadı");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            let state = app.state::<AppState>();
            state.shutdown.cancel();
            if let Ok(control) = state.requests.lock() {
                if let Some(active) = &control.active {
                    active.cancel.cancel();
                }
            }
            tauri::async_runtime::block_on(async {
                state.engine.lock().await.stop();
            });
        }
    });
}
