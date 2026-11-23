//! Explicit Windows/GPU integration check. Uses real local model files, no web.
use aevra_core::{db::Database, model::Engine, prompts, types::*};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut engine = Engine::new(root.join("src-tauri/resources"));
    let gpu = engine.start_with_device(ComputeDevice::Gpu).await?;
    let status = engine.status();
    assert_eq!(status.backend.as_deref(), Some("vulkan"));
    println!(
        "GPU: {}",
        status.device_name.as_deref().unwrap_or("unknown")
    );
    let dir = tempfile::tempdir()?;
    let mut db = Database::open(&dir.path().join("hardware.db"))?;
    let settings = AppSettings {
        compute_device: ComputeDevice::Gpu,
        temperature: 0.1,
        max_tokens: 512,
        ..AppSettings::default()
    };
    db.save_settings(&settings)?;
    for (mode, text) in [(Mode::Math,"2x + 5 = 17 denklemini çöz ve sonucu kontrol et."), (Mode::Code,"Rust dilinde i32 bir sayının karesini döndüren square adlı küçük bir fonksiyon yaz. Kısa bir örnek ekle.")] {
        let chat = db.create(mode)?;
        let request = GenerateRequest { request_id:uuid::Uuid::new_v4().to_string(),conversation_id:chat.id.clone(),text:text.into(),target_language:None, attachment_ids: vec![], };
        let answer = gpu.generate_with_settings(prompts::build_with_settings(mode,&request,&[],&[],&settings)?,CancellationToken::new(),&settings,|_|{}).await?;
        println!("{}: {}",mode.as_str(),answer);
        if mode == Mode::Math { assert!(answer.contains('6'),"x=6 sonucu bekleniyor"); }
        else { assert!(answer.contains("fn square") && answer.contains("```"),"Dil etiketli Rust fonksiyonu bekleniyor"); }
        db.save_turn(&chat.id,text,&answer,vec![])?;
    }
    assert_eq!(db.list()?.len(), 2);
    let cpu = engine.start_with_device(ComputeDevice::Cpu).await?;
    assert_eq!(engine.status().backend.as_deref(), Some("cpu"));
    let request = GenerateRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: "test".into(),
        text: "Hello!".into(),
        target_language: Some("Türkçe".into()),
        attachment_ids: vec![],
    };
    let answer = cpu
        .generate(
            prompts::build(Mode::Translate, &request, &[], &[])?,
            CancellationToken::new(),
            |_| {},
        )
        .await?;
    assert!(!answer.trim().is_empty());
    println!("CPU: {answer}");
    engine.start_with_device(ComputeDevice::Auto).await?;
    assert_eq!(engine.status().backend.as_deref(), Some("vulkan"));
    engine.stop();
    println!("GPU → CPU → Otomatik geçişleri, matematik, kodlama ve kayıt testi başarılı.");
    Ok(())
}
