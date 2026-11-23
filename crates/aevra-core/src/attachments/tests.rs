use super::*;
use std::io::Write;

fn zip(entries: &[(&str, &[u8])], compressed: bool) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer
            .start_file(
                *name,
                zip::write::SimpleFileOptions::default().compression_method(if compressed {
                    zip::CompressionMethod::Deflated
                } else {
                    zip::CompressionMethod::Stored
                }),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
fn read(name: &str, bytes: &[u8]) -> Result<Attachment> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).unwrap();
    read_attachment(&path, Path::new("missing-resources"))
}

#[test]
fn reads_word_equations_as_latex() {
    let xml = br#"<w:document xmlns:w="w" xmlns:m="m"><w:body><w:p><w:r><w:t>Equation:</w:t></w:r><m:oMath><m:f><m:num><m:r><m:t>1</m:t></m:r></m:num><m:den><m:rad><m:e><m:r><m:t>x</m:t></m:r></m:e></m:rad></m:den></m:f></m:oMath></w:p></w:body></w:document>"#;
    let file = read("equations.docx", &zip(&[("word/document.xml", xml)], false)).unwrap();
    assert!(file.chunks[0].text.contains("$\\frac{1}{\\sqrt{x}}$"));
}
#[test]
fn reads_legacy_word_piece_table_and_rejects_encryption() {
    fn document(encrypted: bool) -> Vec<u8> {
        let mut doc = vec![0u8; 1024];
        doc[0..2].copy_from_slice(&0xa5ecu16.to_le_bytes());
        doc[2..4].copy_from_slice(&0xc1u16.to_le_bytes());
        if encrypted {
            doc[10..12].copy_from_slice(&0x100u16.to_le_bytes());
        }
        doc[34..36].copy_from_slice(&4u16.to_le_bytes());
        doc[48..52].copy_from_slice(&6u32.to_le_bytes());
        doc[52..54].copy_from_slice(&34u16.to_le_bytes());
        doc[322..326].copy_from_slice(&21u32.to_le_bytes());
        let text = "Aevra\r"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        doc[512..512 + text.len()].copy_from_slice(&text);
        let mut table = vec![2];
        table.extend_from_slice(&16u32.to_le_bytes());
        table.extend_from_slice(&0u32.to_le_bytes());
        table.extend_from_slice(&6u32.to_le_bytes());
        table.extend_from_slice(&[0, 0]);
        table.extend_from_slice(&512u32.to_le_bytes());
        table.extend_from_slice(&[0, 0]);
        let mut c = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        c.create_stream("/WordDocument")
            .unwrap()
            .write_all(&doc)
            .unwrap();
        c.create_stream("/0Table")
            .unwrap()
            .write_all(&table)
            .unwrap();
        c.into_inner().into_inner()
    }
    assert_eq!(
        read("old.doc", &document(false)).unwrap().chunks[0]
            .text
            .trim(),
        "Aevra"
    );
    assert!(read("secret.doc", &document(true)).is_err());
}
#[test]
fn reads_unicode_code_docx_and_zip_without_extracting_files() {
    let file = read("kod.rs", "// Türkçe\nfn main() {}".as_bytes()).unwrap();
    assert!(file.chunks[0].text.contains("Türkçe"));
    let utf16 = vec![0xff, 0xfe, b'A', 0, b'B', 0];
    assert_eq!(read("metin.txt", &utf16).unwrap().chunks[0].text, "AB");
    let xml=br#"<w:document xmlns:w="x"><w:body><w:p><w:r><w:t>A &amp; B &#65;</w:t></w:r></w:p></w:body></w:document>"#;
    let docx = zip(&[("word/document.xml", xml)], false);
    assert!(read("test.docx", &docx).unwrap().chunks[0]
        .text
        .contains("A & B A"));
    let outer = zip(
        &[
            ("README.md", b"Aevra"),
            ("src/main.rs", b"fn main(){}"),
            ("nested.zip", &docx),
            ("run.exe", b"MZ"),
        ],
        false,
    );
    let file = read("proje.zip", &outer).unwrap();
    assert_eq!(file.chunks.len(), 2);
    assert_eq!(file.warnings.len(), 2);
    assert_eq!(file.entries, 4);
}
#[test]
fn rejects_unsafe_archives_binary_text_and_oversized_decompression() {
    for name in [
        "../outside.txt",
        "/outside.txt",
        "C:/outside.txt",
        "dir\\outside.txt",
    ] {
        assert!(
            read("bad.zip", &zip(&[(name, b"data")], false)).is_err(),
            "{name}"
        );
    }
    assert!(read("bad.zip", &zip(&[("huge.txt", &vec![b'x'; 200_000])], true)).is_err());
    let entries = (0..65)
        .map(|i| (format!("{i}.txt"), b"x".as_slice()))
        .collect::<Vec<_>>();
    assert!(read(
        "bad.zip",
        &zip(
            &entries
                .iter()
                .map(|(a, b)| (a.as_str(), *b))
                .collect::<Vec<_>>(),
            false
        )
    )
    .is_err());
    assert!(read("bad.txt", b"a\0b").is_err());
    assert!(read("bad.txt", &[0xff, 0xff]).is_err());
    assert!(read("run.exe", b"MZ").is_err());
    assert!(read("bad.zip", b"PK broken").is_err());
    let xml = br#"<!DOCTYPE document [<!ENTITY x SYSTEM "file:///secrets">]><w:t>&x;</w:t>"#;
    assert!(read("bad.docx", &zip(&[("word/document.xml", xml)], false)).is_err());
}
#[test]
fn normalizes_images_and_rejects_external_or_forged_image_data() {
    let image = DynamicImage::new_rgb8(1600, 900);
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    let mut file = read("photo.png", bytes.get_ref()).unwrap();
    assert!(file.images[0]
        .data_url
        .starts_with("data:image/jpeg;base64,"));
    validate_batch(&[file.clone()]).unwrap();
    file.images[0].data_url = "https://example.com/image.jpg".into();
    assert!(validate_batch(&[file]).is_err());
    assert!(read("bad.png", b"not an image").is_err());
}
#[test]
fn selects_relevant_chunks_and_labels_and_enforces_text_budget() {
    let mut file = read(
        "notes.txt",
        format!("{}\nMercury secret number 73921", "A".repeat(5000)).as_bytes(),
    )
    .unwrap();
    let context = context(&[file.clone()], "Mercury secret number?", 1700);
    assert!(context.contains("73921"));
    assert!(context.contains("notes.txt"));
    assert!(context.chars().count() <= 1700);
    file.chunks.clear();
    add_text(&mut file, "long", &"é".repeat(MAX_TEXT + 1));
    assert!(file.warnings.iter().any(|w| w.starts_with("Metin sınırı")));
    validate_batch(&[file]).unwrap();
}
#[test]
fn persists_attachments_before_interruption_and_round_trips_backups() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let file = read("notes.txt", b"Local data 73921").unwrap();
    let mut db = crate::db::Database::open(&path).unwrap();
    let chat = db.create(crate::types::Mode::Chat).unwrap();
    db.begin_turn_with_attachments(
        &chat.id,
        &uuid::Uuid::new_v4().to_string(),
        "Read file",
        std::slice::from_ref(&file),
    )
    .unwrap();
    db.finish_turn(&chat.id, "Partial", vec![], "interrupted")
        .unwrap();
    drop(db);
    let db = crate::db::Database::open(&path).unwrap();
    let messages = db.messages(&chat.id).unwrap();
    assert_eq!(
        messages[0].attachments[0].chunks[0].text,
        "Local data 73921"
    );
    assert_eq!(messages[1].status, "interrupted");
    let backup = db.export_history().unwrap();
    let mut imported = crate::db::Database::open(&dir.path().join("import")).unwrap();
    assert_eq!(imported.import_history(backup).unwrap(), 1);
    assert_eq!(
        imported.messages(&chat.id).unwrap()[0].attachments[0].id,
        file.id
    );
    imported.delete(&chat.id).unwrap();
    assert_eq!(imported.counts().unwrap(), (0, 0));
}
#[test]
fn attachment_prompts_reuse_local_files_and_reject_research_mode() {
    use crate::{prompts, types::*};
    let file = read("notes.txt", b"Local code 73921").unwrap();
    let mut request = GenerateRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: "local".into(),
        text: "What code?".into(),
        target_language: None,
        attachment_ids: vec![file.id.clone()],
    };
    let settings = AppSettings::default();
    let mut translation_request = GenerateRequest {
        request_id: request.request_id.clone(),
        conversation_id: request.conversation_id.clone(),
        text: "Çevir".into(),
        target_language: Some("Türkçe".into()),
        attachment_ids: request.attachment_ids.clone(),
    };
    let translated = prompts::build_with_attachments(
        Mode::Translate,
        &translation_request,
        &[],
        &[],
        &settings,
        std::slice::from_ref(&file),
    )
    .unwrap();
    assert!(translated[0].content.contains("JSON alan adları"));
    translation_request.target_language = Some("uydurma dil".into());
    assert!(prompts::build_with_attachments(
        Mode::Translate,
        &translation_request,
        &[],
        &[],
        &settings,
        std::slice::from_ref(&file)
    )
    .is_err());
    let messages = prompts::build_with_attachments(
        Mode::Chat,
        &request,
        &[],
        &[],
        &settings,
        std::slice::from_ref(&file),
    )
    .unwrap();
    assert!(messages.last().unwrap().content.contains("73921"));
    assert!(messages[0].content.contains("güvenilmeyen"));
    assert!(prompts::build_with_attachments(
        Mode::Research,
        &request,
        &[],
        &[],
        &settings,
        std::slice::from_ref(&file)
    )
    .is_err());
    let history = vec![Message {
        id: request.request_id.clone(),
        conversation_id: "local".into(),
        role: "user".into(),
        content: "Read".into(),
        created_at: 0,
        sources: vec![],
        status: "complete".into(),
        attachments: vec![file],
    }];
    request.attachment_ids.clear();
    assert!(
        prompts::build_with_attachments(Mode::Chat, &request, &history, &[], &settings, &[])
            .unwrap()
            .last()
            .unwrap()
            .content
            .contains("73921")
    );
}
