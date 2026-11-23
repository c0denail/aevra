//! Explicit integration smoke test. Never run automatically in offline CI.
use aevra_core::{db::Database, model::Engine, prompts, research, types::*};
use std::{io::Write, path::PathBuf};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut engine = Engine::new(root.join("src-tauri/resources"));
    println!("Paket doğrulanıyor ve gerçek model başlatılıyor…");
    let device = if std::env::args().any(|arg| arg == "--cpu") {
        ComputeDevice::Cpu
    } else {
        ComputeDevice::Auto
    };
    let client = engine.start_with_device(device).await?;
    let status = engine.status();
    println!("{} · {}", status.model, status.detail);
    if let Some(reason) = status.fallback_reason {
        println!("{reason}");
    }
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("smoke.sqlite3");
    let mut db = Database::open(&path)?;
    for (mode, text, language) in [
        (Mode::Chat, "Merhaba! Kendini tek kısa cümlede tanıt.", None),
        (
            Mode::Translate,
            "Privacy starts on your own computer.",
            Some("Türkçe"),
        ),
    ] {
        let conversation = db.create(mode)?;
        let request = GenerateRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation.id.clone(),
            text: text.into(),
            target_language: language.map(str::to_string),
            attachment_ids: vec![],
        };
        let answer = client
            .generate(
                prompts::build(mode, &request, &[], &[])?,
                CancellationToken::new(),
                |text| {
                    print!("{text}");
                    let _ = std::io::stdout().flush();
                },
            )
            .await?;
        println!();
        db.save_turn(&conversation.id, text, &answer, vec![])?;
        assert_eq!(db.messages(&conversation.id)?.len(), 2);
    }
    drop(db);
    assert_eq!(Database::open(&path)?.list()?.len(), 2);
    if std::env::args().any(|arg| arg == "--web") {
        let query = "Rust programming language official documentation";
        println!("Açık test sorgusu gönderiliyor: {query}");
        let sources = research::research(query).await?;
        assert!(!sources.is_empty());
        for source in &sources {
            println!("[{}] {}: {}", source.id, source.title, source.url);
        }
        let mut db = Database::open(&path)?;
        let conversation = db.create(Mode::Research)?;
        let request = GenerateRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation.id.clone(),
            text: "Rust resmi belgeleri ne işe yarar? Kaynak numarasıyla tek cümle yanıtla.".into(),
            target_language: None,
            attachment_ids: vec![],
        };
        let answer = client
            .generate(
                prompts::build(Mode::Research, &request, &[], &sources)?,
                CancellationToken::new(),
                |text| {
                    print!("{text}");
                    let _ = std::io::stdout().flush();
                },
            )
            .await?;
        println!();
        assert!(
            sources
                .iter()
                .any(|s| answer.contains(&format!("[{}]", s.id))),
            "Araştırma yanıtında gerçek kaynak numarası olmalı"
        );
        db.save_turn(&conversation.id, &request.text, &answer, sources)?;
        assert!(!db.messages(&conversation.id)?[1].sources.is_empty());
    }
    engine.stop();
    println!("Gerçek model ve kalıcı geçmiş denemesi başarılı.");
    Ok(())
}
