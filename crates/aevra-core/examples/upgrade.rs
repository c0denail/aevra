//! Explicit 9B / equation-PDF / conversational research regression, outside CI.
use aevra_core::{
    attachments, math::MathAsset, model::Engine, pdf_export, prompts, research_chat, types::*,
};
use std::{fs, path::PathBuf};
use tokio_util::sync::CancellationToken;
#[derive(serde::Deserialize)]
struct Sample {
    text: String,
    assets: Vec<MathAsset>,
}
fn message(role: &str, text: &str, sources: &[Source]) -> Message {
    Message {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: "upgrade".into(),
        role: role.into(),
        content: text.into(),
        created_at: 0,
        sources: sources.to_vec(),
        status: "complete".into(),
        attachments: vec![],
    }
}
fn request(text: &str) -> GenerateRequest {
    GenerateRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: "upgrade".into(),
        text: text.into(),
        target_language: None,
        attachment_ids: vec![],
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let resources = root.join("src-tauri/resources");
    let sample: Sample =
        serde_json::from_slice(&fs::read(root.join("tmp/pdfs/math-sample.json"))?)?;
    let bytes = pdf_export::create_pdf_with_math(
        "Aevra — Formül doğrulaması",
        &sample.text,
        "complete",
        &[],
        &resources,
        &sample.assets,
    )?;
    let output = root.join("output/pdf/aevra-formuller.pdf");
    fs::create_dir_all(output.parent().unwrap())?;
    pdf_export::write_atomic(&output, &bytes)?;
    let mut file = attachments::read_attachment(&output, &resources)?;
    assert!(!file.images.is_empty());
    assert!(file.chunks.iter().any(|c| c.text.contains("9264")));
    println!(
        "PDF: {} sayfa, {} metin bölümü, {} görsel; formüller ve son kontrol geri okundu.",
        file.pages,
        file.chunks.len(),
        file.images.len()
    );
    if std::env::args().any(|a| a == "--pdf-only") {
        return Ok(());
    }
    let mut engine = Engine::new(resources.clone());
    let client = engine.start_with_device(ComputeDevice::Gpu).await?;
    println!("{} · {}", engine.status().model, engine.status().detail);
    let settings = AppSettings {
        max_tokens: 512,
        temperature: 0.1,
        ..AppSettings::default()
    };
    // No hidden LaTeX/text shortcut: the 9B visual encoder must read the page.
    file.chunks.clear();
    file.images.truncate(1);
    let answer=client.generate_with_settings(prompts::build_with_attachments(Mode::Math,&request("İlk sayfadaki ikinci derece denklem örneğinin iki kökünü yaz. Kök formülünü de LaTeX ile göster. Kısaca cevapla."),&[],&[],&settings,&[file])?,CancellationToken::new(),&settings,|_|{}).await?;
    println!("PDF formülleri: {answer}");
    assert!(answer.contains('2') && answer.contains('3') && answer.contains("\\frac"));
    if std::env::args().any(|a| a == "--vision-only") {
        engine.stop();
        println!("9B yalnız sayfa görüntüsünden formülü ve kökleri okudu.");
        return Ok(());
    }
    let sources=vec![Source{id:1,title:"Rust ownership guide".into(),url:"https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html".into(),excerpt:"Ownership manages memory with rules checked by the compiler. A value has one owner; it is dropped when the owner goes out of scope.".into(),accessed_at:0}];
    let history=vec![message("user","Rust ownership nedir?",&[]),message("assistant","Rust ownership belleği derleme zamanında kontrol edilen kurallarla yönetir. Her değerin bir sahibi vardır. [1]",&sources)];
    let follow = "Bunu daha basit bir örnekle açıkla.";
    let plan = research_chat::plan(
        &client,
        follow,
        &history,
        &sources,
        &settings,
        CancellationToken::new(),
    )
    .await?;
    println!("Takip planı: search={}, query={}", plan.search, plan.query);
    assert!(!plan.search);
    let answer = client
        .generate_with_settings(
            prompts::build_with_settings(
                Mode::Research,
                &request(follow),
                &history,
                &sources,
                &settings,
            )?,
            CancellationToken::new(),
            &settings,
            |_| {},
        )
        .await?;
    println!("Bağlamlı araştırma sohbeti: {answer}");
    assert!(answer.to_lowercase().contains("sahip") || answer.contains("Rust"));
    let plan = research_chat::plan(
        &client,
        "Bunun C++ ile güncel karşılaştırmasını araştır.",
        &history,
        &sources,
        &settings,
        CancellationToken::new(),
    )
    .await?;
    println!(
        "Yeni araştırma planı: search={}, query={}",
        plan.search, plan.query
    );
    assert!(plan.search);
    assert!(plan.query.to_lowercase().contains("rust"));
    if std::env::args().any(|a| a == "--web") {
        let found = aevra_core::research::research_with_limit(&plan.query, 2).await?;
        println!("Gerçek web: {} kaynak", found.len());
        assert!(!found.is_empty());
    }
    let offline = AppSettings {
        web_research_enabled: false,
        ..settings
    };
    assert!(
        !research_chat::plan(
            &client,
            "Güncel bilgi ara",
            &history,
            &sources,
            &offline,
            CancellationToken::new()
        )
        .await?
        .search
    );
    engine.stop();
    println!("9B, formüllü PDF, bağlamlı araştırma ve çevrimdışı araştırma kapatma doğrulandı.");
    Ok(())
}
