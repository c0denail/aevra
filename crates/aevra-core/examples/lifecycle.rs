//! Runs against the real local model; no internet requests.
use aevra_core::{db::Database, model::Engine, prompts, types::*, Error};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut engine = Engine::new(root.join("src-tauri/resources"));
    let client = engine.start().await?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("lifecycle.sqlite3");
    let mut db = Database::open(&path)?;
    let chat = db.create(Mode::Chat)?;
    let request = GenerateRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: chat.id.clone(),
        text: "Bir uzay yolculuğunu uzun bir hikâye olarak anlat.".into(),
        target_language: None,
        attachment_ids: vec![],
    };
    db.begin_turn(&chat.id, &request.request_id, &request.text)?;
    let cancel = CancellationToken::new();
    let mut partial = String::new();
    let result = client
        .generate(
            prompts::build(Mode::Chat, &request, &[], &[])?,
            cancel.clone(),
            |text| {
                partial.push_str(&text);
                if partial.chars().count() >= 20 {
                    cancel.cancel();
                }
            },
        )
        .await;
    assert!(matches!(result, Err(Error::Cancelled)));
    assert!(!partial.is_empty());
    db.finish_turn(&chat.id, &partial, vec![], "interrupted")?;
    drop(db);
    let db = Database::open(&path)?;
    let messages = db.messages(&chat.id)?;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].content, partial);
    assert_eq!(messages[1].status, "interrupted");
    assert_eq!(engine.status().state, "ready");
    let next = GenerateRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: chat.id.clone(),
        text: "Hello, how are you?".into(),
        target_language: Some("Türkçe".into()),
        attachment_ids: vec![],
    };
    let answer = client
        .generate(
            prompts::build(Mode::Translate, &next, &[], &[])?,
            CancellationToken::new(),
            |_| {},
        )
        .await?;
    assert!(!answer.trim().is_empty());
    engine.stop();
    println!("Gerçek model: durdurma, kısmi yanıtın kalıcı kaydı ve durdurma sonrası yeniden üretim başarılı.");
    Ok(())
}
