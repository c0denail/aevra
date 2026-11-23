use super::*;
use aevra_core::memory::{self, MemoryState};
use tauri::Emitter;

pub(super) fn cancel_work(state: &AppState) {
    if let Ok(work) = state.memory_work.lock() {
        if let Some(token) = work.as_ref() {
            token.cancel();
        }
    }
}
fn changed(app: &tauri::AppHandle) {
    let _ = app.emit(
        "memory-updated",
        serde_json::json!({"batchId":"","count":0}),
    );
}
#[tauri::command]
pub(super) fn get_memories(state: State<'_, AppState>) -> CommandResult<MemoryState> {
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .memory_state()
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(super) fn edit_memory(
    key: String,
    value: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    cancel_work(&state);
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .edit_memory(&key, &value)
        .map_err(|e| e.to_string())?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub(super) fn delete_memory(
    key: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    cancel_work(&state);
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .delete_memory(&key)
        .map_err(|e| e.to_string())?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub(super) fn clear_memories(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    cancel_work(&state);
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .clear_memories()
        .map_err(|e| e.to_string())?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub(super) fn allow_memory(
    key: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    cancel_work(&state);
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .allow_memory(&key)
        .map_err(|e| e.to_string())?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub(super) fn undo_memories(
    batch_id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<usize> {
    cancel_work(&state);
    let count = state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .undo_memories(&batch_id)
        .map_err(|e| e.to_string())?;
    changed(&app);
    Ok(count)
}
#[tauri::command]
pub(super) fn set_conversation_memory(
    conversation_id: String,
    disabled: bool,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    cancel_work(&state);
    state
        .db
        .lock()
        .map_err(|_| "Veritabanı kilitli.")?
        .conversation_memory(&conversation_id, disabled)
        .map_err(|e| e.to_string())?;
    changed(&app);
    Ok(())
}

pub(super) fn spawn(app: tauri::AppHandle) {
    let state = app.state::<AppState>();
    cancel_work(&state);
    let cancel = state.shutdown.child_token();
    if let Ok(mut slot) = state.memory_work.lock() {
        *slot = Some(cancel.clone());
    }
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let result = work(&app, &state, &cancel).await;
        if result.is_err() && !cancel.is_cancelled() {
            let _=app.emit("memory-updated",serde_json::json!({"batchId":"","count":0,"error":"Kişisel hafıza güncellenemedi. Yanıt ve sohbet geçmişi korundu."}));
        }
    });
}
async fn work(
    app: &tauri::AppHandle,
    state: &AppState,
    cancel: &CancellationToken,
) -> aevra_core::Result<()> {
    let _guard =
        tokio::select! {_ = cancel.cancelled()=>return Ok(()),guard=state.generation.lock()=>guard};
    loop {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let data = {
            let db = state
                .db
                .lock()
                .map_err(|_| Error::Validation("Veritabanı kilitli.".into()))?;
            if !db.settings()?.memory_enabled {
                return Ok(());
            }
            db.next_memory_job()?
                .map(|job| -> aevra_core::Result<_> {
                    Ok((job, db.settings()?, db.memory_state()?.items))
                })
                .transpose()?
        };
        let Some(((message, conversation, source, revision), settings, known)) = data else {
            return Ok(());
        };
        let client = tokio::select! {_ = cancel.cancelled()=>return Ok(()),client=ensure_model(state)=>client?};
        let extraction = tokio::select! {_ = cancel.cancelled()=>return Ok(()),result=tokio::time::timeout(std::time::Duration::from_secs(45),memory::extract(&client,&source,&known,&settings,cancel.clone()))=>result};
        if cancel.is_cancelled() {
            return Ok(());
        }
        let mut db = state
            .db
            .lock()
            .map_err(|_| Error::Validation("Veritabanı kilitli.".into()))?;
        let result = match extraction {
            Ok(Ok(candidates)) => {
                db.remember_candidates(revision, &conversation, &message, &source, candidates)
            }
            _ => Err(Error::Validation("Hafıza çıkarımı tamamlanamadı.".into())),
        };
        db.finish_memory_job(&message)?;
        match result {
            Ok(Some(notice)) => {
                let _ = app.emit("memory-updated", notice);
            }
            Ok(None) => (),
            Err(error) => return Err(error),
        }
    }
}
