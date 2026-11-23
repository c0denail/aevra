//! Explicit local model regression check for German, continuation, whole-document translation and PDF export.
use aevra_core::{attachments, model::Engine, pdf_export, prompts, translation, types::*};
use pdfium_render::prelude::*;
use std::{fs, path::PathBuf};
use tokio_util::sync::CancellationToken;

fn request(text: &str, language: &str) -> GenerateRequest {
    GenerateRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: "regression".into(),
        text: text.into(),
        target_language: Some(language.into()),
        attachment_ids: vec![],
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let resources = root.join("src-tauri/resources");
    let temp = tempfile::tempdir()?;
    let mut engine = Engine::new(resources.clone());
    let client = engine.start_with_device(ComputeDevice::Gpu).await?;
    let settings = AppSettings {
        compute_device: ComputeDevice::Gpu,
        temperature: 0.1,
        max_tokens: 256,
        ..AppSettings::default()
    };
    let german = client
        .generate_with_settings(
            prompts::build(
                Mode::Translate,
                &request(
                    "Good morning. My name is Anna. Thank you very much.",
                    "Almanca",
                ),
                &[],
                &[],
            )?,
            CancellationToken::new(),
            &settings,
            |_| {},
        )
        .await?;
    println!("Almanca: {german}");
    assert!(
        german.contains("Anna")
            && (german.contains("Morgen") || german.contains("Guten"))
            && (german.contains("Dank") || german.contains("dank"))
    );
    let turkish = client
        .generate_with_settings(
            prompts::build(
                Mode::Translate,
                &request(
                    "Guten Morgen. Ich wohne in Berlin und lerne Türkisch.",
                    "Türkçe",
                ),
                &[],
                &[],
            )?,
            CancellationToken::new(),
            &settings,
            |_| {},
        )
        .await?;
    println!("Almanca → Türkçe: {turkish}");
    assert!(turkish.contains("Berlin") && turkish.contains("Türk"));
    let long=(1..=35).map(|n|format!("{n}. The local assistant keeps this document on your computer and does not send its content to a remote service.\n")).collect::<String>()+"Reference code: AEVRA-END-41862";
    let mut continuations = 0;
    let answer = client
        .generate_with_progress(
            prompts::build(Mode::Translate, &request(&long, "Almanca"), &[], &[])?,
            CancellationToken::new(),
            &settings,
            |_| {},
            |_| continuations += 1,
        )
        .await?;
    println!(
        "Uzun Almanca: {} karakter, {continuations} otomatik devam; son: {}",
        answer.chars().count(),
        answer
            .chars()
            .rev()
            .take(120)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>()
    );
    assert!(continuations > 0);
    assert!(answer.contains("41862"));
    let source=(1..=30).map(|n|format!("## Document section {n}\nThis is a local document translation test. Every section must be read in its original order. The application preserves the document on this computer. The translation must include every section and must not summarize the source. This paragraph is deliberately repeated to verify processing beyond the old excerpt limit. Section number {n} remains visible in the result.\n\n")).collect::<String>()+"Final reference code: AEVRA-PDF-END-73921.";
    let bytes = pdf_export::create_pdf(
        "Whole document translation input",
        &source,
        "complete",
        &[],
        &resources,
    )?;
    let path = temp.path().join("whole.pdf");
    fs::write(&path, bytes)?;
    let file = attachments::read_attachment(&path, &resources)?;
    let source_length = file
        .chunks
        .iter()
        .map(|c| c.text.chars().count())
        .sum::<usize>();
    assert!(source_length > 8000);
    println!(
        "PDF: {} sayfa, {} bölüm, {source_length} karakter",
        file.pages,
        file.chunks.len()
    );
    let mut progress = Vec::new();
    let translated = translation::translate_files(
        &client,
        &[file],
        "Almanca",
        &AppSettings {
            max_tokens: 1024,
            ..settings.clone()
        },
        CancellationToken::new(),
        |_| {},
        |s| {
            println!("{s}");
            progress.push(s)
        },
    )
    .await?;
    assert!(translated.contains("73921"));
    assert!(translated.contains("30"));
    println!(
        "Tam PDF Almanca çeviri: {} karakter; son kimlik korundu",
        translated.chars().count()
    );
    let output = root.join("output/pdf");
    let preview = root.join("tmp/pdfs");
    fs::create_dir_all(&output)?;
    fs::create_dir_all(&preview)?;
    let content=format!("# Türkçe ve Almanca\nÇığ, öğüt, şeker, İstanbul, ıslak ve üzüm.\nÜberprüfung, Größe, Straße, Äpfel, Öl und süß.\n\n## Almanca çeviri\n{german}\n\n## Uzun yanıtın son bölümü\n{}\n\n## Kod örneği\n```rust\nfn square(n: i32) -> Option<i32> {{\n    n.checked_mul(n)\n}}\n```\n\n## Tam PDF çevirisi\n{translated}",answer.chars().rev().take(600).collect::<Vec<_>>().into_iter().rev().collect::<String>());
    let bytes = pdf_export::create_pdf(
        "Aevra - Çeviri ve PDF doğrulaması",
        &content,
        "complete",
        &[],
        &resources,
    )?;
    pdf_export::write_atomic(&output.join("aevra-ornek.pdf"), &bytes)?;
    let pdfium = attachments::pdfium(&resources)?;
    let document = pdfium.load_pdf_from_byte_slice(&bytes, None)?;
    let count = document.pages().len();
    let mut extracted = String::new();
    for (index, page) in document.pages().iter().enumerate() {
        extracted.push_str(&page.text()?.all());
        page.render_with_config(&PdfRenderConfig::new().set_target_width(1000))?
            .as_image()?
            .save(preview.join(format!("aevra-page-{}.png", index + 1)))?;
    }
    for word in ["İstanbul", "Straße", "73921"] {
        assert!(extracted.contains(word), "PDF metni: {word}");
    }
    println!("PDF üretimi: {count} sayfa; Türkçe/Almanca karakterler ve belge sonu geri okundu.");
    engine.stop();
    Ok(())
}
