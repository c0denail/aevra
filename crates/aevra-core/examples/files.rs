//! Explicit offline integration check: actual projector, PDFium, readers, model, and database.
use aevra_core::{
    attachments::{self, Attachment},
    db::Database,
    model::Engine,
    prompts,
    types::*,
};
use image::{DynamicImage, ImageBuffer, Rgb};
use std::{
    io::{Cursor, Write},
    path::PathBuf,
};
use tokio_util::sync::CancellationToken;

fn pdf(objects: Vec<Vec<u8>>) -> Vec<u8> {
    let mut output = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        writeln!(output, "{} 0 obj", i + 1).unwrap();
        output.extend(object);
        output.extend(b"\nendobj\n");
    }
    let start = output.len();
    write!(output, "xref\n0 {}\n0000000000 65535 f \n", offsets.len()).unwrap();
    for offset in offsets.iter().skip(1) {
        writeln!(output, "{offset:010} 00000 n ").unwrap();
    }
    write!(
        output,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF",
        offsets.len()
    )
    .unwrap();
    output
}
fn stream(data: &[u8], description: &str) -> Vec<u8> {
    let mut out = format!("<< /Length {} {description} >>\nstream\n", data.len()).into_bytes();
    out.extend(data);
    out.extend(b"\nendstream");
    out
}
fn document(image: Option<&[u8]>) -> Vec<u8> {
    let (resources, commands, last) = if let Some(bytes) = image {
        ("/XObject << /Im0 5 0 R >>",b"q 300 0 0 180 0 0 cm /Im0 Do Q".as_slice(),stream(bytes,"/Type /XObject /Subtype /Image /Width 600 /Height 300 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode"))
    } else {
        (
            "/Font << /F1 5 0 R >>",
            b"BT /F1 12 Tf 20 140 Td (Aevra local document test: code 73921) Tj ET".as_slice(),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        )
    };
    pdf(vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 180] /Resources << {resources} >> /Contents 4 0 R >>").into_bytes(),stream(commands,""),last])
}
fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
async fn answer(
    engine: &mut Engine,
    file: &Attachment,
    text: &str,
    settings: &AppSettings,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = engine.start_with_device(settings.compute_device).await?;
    let request = GenerateRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: "fixture".into(),
        text: text.into(),
        target_language: (settings.default_mode == Mode::Translate)
            .then(|| settings.translation_language.clone()),
        attachment_ids: vec![file.id.clone()],
    };
    let messages = prompts::build_with_attachments(
        settings.default_mode,
        &request,
        &[],
        &[],
        settings,
        std::slice::from_ref(file),
    )?;
    Ok(client
        .generate_with_settings(messages, CancellationToken::new(), settings, |_| {})
        .await?)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let resources = root.join("src-tauri/resources");
    let temp = tempfile::tempdir()?;
    let picture = DynamicImage::ImageRgb8(ImageBuffer::from_fn(600, 300, |x, _| {
        if x < 300 {
            Rgb([255, 0, 0])
        } else {
            Rgb([0, 0, 255])
        }
    }));
    picture.save(temp.path().join("renkler.png"))?;
    let mut jpeg = Cursor::new(Vec::new());
    picture.write_to(&mut jpeg, image::ImageFormat::Jpeg)?;
    std::fs::write(
        temp.path().join("taranmis.pdf"),
        document(Some(jpeg.get_ref())),
    )?;
    std::fs::write(temp.path().join("metin.pdf"), document(None))?;
    std::fs::write(temp.path().join("belge.docx"),zip(&[("word/document.xml",br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Project code: 73921</w:t></w:r></w:p></w:body></w:document>"#)]))?;
    std::fs::write(
        temp.path().join("proje.zip"),
        zip(&[
            ("README.md", b"Project code: 73921"),
            ("src/main.rs", b"fn main() { println!(\"hello\"); }"),
        ]),
    )?;
    let mut engine = Engine::new(resources.clone());
    let settings = AppSettings {
        compute_device: ComputeDevice::Gpu,
        temperature: 0.1,
        max_tokens: 256,
        ..AppSettings::default()
    };
    let path = temp.path().join("files.db");
    let mut db = Database::open(&path)?;
    for (name, is_image) in [
        ("renkler.png", true),
        ("taranmis.pdf", true),
        ("metin.pdf", false),
        ("belge.docx", false),
        ("proje.zip", false),
    ] {
        let file = attachments::read_attachment(&temp.path().join(name), &resources)?;
        println!(
            "{name}: {} metin bölümü, {} görsel",
            file.chunks.len(),
            file.images.len()
        );
        let question = if is_image {
            "Görselin sol yarısı ve sağ yarısı hangi renkte? Yalnızca iki rengi Türkçe yaz."
        } else {
            "Belgedeki Project code veya document test code numarası nedir? Yalnızca numarayı yaz."
        };
        let output = answer(&mut engine, &file, question, &settings).await?;
        println!("GPU {name}: {output}");
        if is_image {
            let lower = output.to_lowercase();
            assert!(
                lower.contains("kırmızı") && lower.contains("mavi"),
                "Görsel renklerinin doğru okunması bekleniyor"
            );
        } else {
            assert!(output.contains("73921"), "Dosyadan gerçek kod bekleniyor");
        }
        let chat = db.create(Mode::Chat)?;
        let request = uuid::Uuid::new_v4().to_string();
        db.begin_turn_with_attachments(&chat.id, &request, question, std::slice::from_ref(&file))?;
        db.finish_turn(&chat.id, &output, vec![], "interrupted")?;
    }
    let pdf = attachments::read_attachment(&temp.path().join("metin.pdf"), &resources)?;
    let translate = AppSettings {
        default_mode: Mode::Translate,
        translation_language: "Türkçe".into(),
        ..settings.clone()
    };
    let output = answer(
        &mut engine,
        &pdf,
        "Ekli belgeyi Türkçeye çevir.",
        &translate,
    )
    .await?;
    println!("PDF çeviri: {output}");
    assert!(
        output.contains("73921")
            && (output.to_lowercase().contains("belge") || output.to_lowercase().contains("yerel"))
    );
    let picture = attachments::read_attachment(&temp.path().join("renkler.png"), &resources)?;
    let mut batch = picture.clone();
    batch.images = vec![picture.images[0].clone(); 4];
    let output = answer(
        &mut engine,
        &batch,
        "Dört görselde ortak olan iki renk nedir? Türkçe yaz.",
        &settings,
    )
    .await?;
    println!("GPU dört görsel: {output}");
    assert!(output.to_lowercase().contains("mavi"));
    let cpu = AppSettings {
        compute_device: ComputeDevice::Cpu,
        ..settings
    };
    let output = answer(
        &mut engine,
        &picture,
        "Görselde hangi iki renk var? Türkçe yaz.",
        &cpu,
    )
    .await?;
    println!("CPU görsel: {output}");
    assert!(output.to_lowercase().contains("mavi"));
    engine.stop();
    drop(db);
    let db = Database::open(&path)?;
    let chats = db.list()?;
    assert_eq!(chats.len(), 5);
    for chat in &chats {
        let messages = db.messages(&chat.id)?;
        assert_eq!(messages[0].attachments.len(), 1);
        assert_eq!(messages[1].status, "interrupted");
    }
    let backup = db.export_history()?;
    let mut imported = Database::open(&temp.path().join("import.db"))?;
    assert_eq!(imported.import_history(backup)?, 5);
    for chat in &chats {
        assert_eq!(
            imported.messages(&chat.id)?[0].attachments[0].name,
            db.messages(&chat.id)?[0].attachments[0].name
        );
        db.delete(&chat.id)?;
    }
    assert_eq!(db.counts()?, (0, 0));
    println!("Görsel + taranmış/metin PDF + DOCX + ZIP, CPU/GPU, kesilmiş geçmiş ve yedek testi başarılı.");
    Ok(())
}
