//! Explicit offline 9B personal-memory integration; never part of normal CI.
use aevra_core::{db::Database, memory, model::Engine, prompts, types::*};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("personal.sqlite3");
    let mut db = Database::open(&path)?;
    let mut engine = Engine::new(root.join("src-tauri/resources"));
    let client = engine.start_with_device(ComputeDevice::Gpu).await?;
    println!("{} · {}", engine.status().model, engine.status().detail);
    let settings = AppSettings {
        temperature: 0.0,
        max_tokens: 512,
        ..AppSettings::default()
    };
    for text in [
        "Almanca seviyem A2.",
        "Almanca seviyem artık B1.",
        "Daha kısa anlat.",
        "Lütfen kısa yanıt ver.",
        "Cevabı kısa tut.",
    ] {
        let chat = db.create(Mode::Chat)?;
        let message = uuid::Uuid::new_v4().to_string();
        db.begin_turn(&chat.id, &message, text)?;
        let source = memory::eligible(text, Mode::Chat, false).unwrap();
        db.enqueue_memory(&chat.id, &message, &source)?;
        let known = db.memory_state()?.items;
        let candidates = memory::extract(
            &client,
            &source,
            &known,
            &settings,
            CancellationToken::new(),
        )
        .await?;
        println!("{text} -> {} aday", candidates.len());
        for c in &candidates {
            println!("  {}={} ({})", c.key, c.value, c.certainty);
        }
        let revision = db.memory_revision()?;
        db.remember_candidates(revision, &chat.id, &message, &source, candidates)?;
        db.finish_memory_job(&message)?;
        if text == "Almanca seviyem A2." {
            assert!(db
                .memory_state()?
                .items
                .iter()
                .any(|m| m.key == "language_level_de" && m.value == "A2"));
        }
    }
    let records = db.memory_state()?.items;
    assert!(records
        .iter()
        .any(|m| m.key == "language_level_de" && m.value == "B1"));
    assert!(records
        .iter()
        .any(|m| m.key == "response_length" && m.certainty == "inferred"));
    drop(db);
    let mut db = Database::open(&path)?;
    let records = db.memory_state()?.items;
    let request = GenerateRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: "new-chat".into(),
        text: "Almanca seviyem neydi? Tek cümleyle söyle.".into(),
        target_language: None,
        attachment_ids: vec![],
    };
    let mut messages = prompts::build(Mode::Chat, &request, &[], &[])?;
    messages[0]
        .content
        .push_str(&memory::context(&records, &request.text, Mode::Chat));
    let answer = client
        .generate_with_settings(messages, CancellationToken::new(), &settings, |_| {})
        .await?;
    println!("Yeni sohbet, geçmiş olmadan: {answer}");
    assert!(answer.contains("B1"));
    db.delete_memory("language_level_de")?;
    assert!(!memory::context(
        &db.memory_state()?.items,
        "Almanca seviyem neydi?",
        Mode::Chat
    )
    .contains("B1"));
    assert!(db
        .memory_state()?
        .blocked_keys
        .contains(&"language_level_de".into()));
    engine.stop();
    println!("Gerçek 9B: otomatik çıkarım, düzeltme, üç konuşmada tercih, yeniden açılma, yeni sohbetten hatırlama ve unutma geçti.");
    Ok(())
}
