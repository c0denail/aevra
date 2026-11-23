//! Bounded, local-only readers. Archives are inspected in memory, never extracted to disk.
use crate::{Error, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{codecs::jpeg::JpegEncoder, DynamicImage, ImageReader};
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
};
mod docx;
mod legacy_word;

pub const MAX_FILES: usize = 4;
pub const MAX_IMAGES: usize = 128;
pub const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_TEXT: usize = 1_000_000;
const MAX_ZIP_BYTES: u64 = 64 * 1024 * 1024;
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "csv", "tsv", "json", "jsonl", "xml", "yaml", "yml", "toml", "ini",
    "log", "rs", "py", "js", "jsx", "ts", "tsx", "html", "css", "scss", "sql", "sh", "ps1", "c",
    "h", "cpp", "hpp", "cs", "java", "kt", "go", "rb", "php", "swift", "vue", "svelte",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextChunk {
    pub label: String,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttachmentImage {
    pub label: String,
    pub data_url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub bytes: u64,
    pub pages: usize,
    pub entries: usize,
    pub warnings: Vec<String>,
    pub chunks: Vec<TextChunk>,
    pub images: Vec<AttachmentImage>,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Validation(message.into())
}
fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}
fn supported(name: &str) -> bool {
    TEXT_EXTENSIONS.contains(&extension(name).as_str())
        || matches!(
            extension(name).as_str(),
            "pdf" | "doc" | "docx" | "png" | "jpg" | "jpeg" | "webp"
        )
}

pub fn read_attachment(path: &Path, resources: &Path) -> Result<Attachment> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("Yalnızca normal dosyalar eklenebilir."));
    }
    if file.metadata()?.len() > MAX_FILE_BYTES {
        return Err(invalid("Bir dosya en fazla 20 MB olabilir."));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(invalid("Dosya boyutu sınırı aşıldı."));
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Dosya")
        .chars()
        .take(180)
        .collect::<String>();
    let ext = extension(&name);
    let mut attachment = Attachment {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        kind: ext.clone(),
        bytes: bytes.len() as u64,
        pages: 0,
        entries: 1,
        warnings: vec![],
        chunks: vec![],
        images: vec![],
    };
    if ext == "zip" {
        read_zip(&bytes, &mut attachment, resources)?;
    } else {
        read_content(&name, &bytes, &mut attachment, resources)?;
    }
    if attachment.chunks.is_empty() && attachment.images.is_empty() {
        return Err(invalid("Dosyada okunabilir içerik bulunamadı."));
    }
    validate_batch(std::slice::from_ref(&attachment))?;
    Ok(attachment)
}

pub fn validate_batch(files: &[Attachment]) -> Result<()> {
    if files
        .iter()
        .flat_map(|f| f.images.iter())
        .map(|i| i.data_url.len())
        .sum::<usize>()
        > 16 * 1024 * 1024
    {
        return Err(invalid(
            "Hazırlanan görseller 16 MB sınırını aşıyor. PDF'yi daha küçük parçalara bölün.",
        ));
    }
    if files
        .iter()
        .map(|f| &f.id)
        .collect::<std::collections::HashSet<_>>()
        .len()
        != files.len()
    {
        return Err(invalid("Aynı dosya eki birden fazla kullanılamaz."));
    }
    if files.len() > MAX_FILES || files.iter().map(|f| f.images.len()).sum::<usize>() > MAX_IMAGES {
        return Err(invalid(
            "Bir mesajda en fazla 4 dosya ve toplam 128 görsel/PDF sayfası kullanılabilir.",
        ));
    }
    for file in files {
        if uuid::Uuid::parse_str(&file.id).is_err()
            || file.name.len() > 720
            || file.kind.len() > 20
            || file.bytes > MAX_FILE_BYTES
            || file.pages > 100
            || file.entries > 64
            || file.chunks.len() > 1024
            || file.warnings.len() > 65
            || file.warnings.iter().any(|w| w.len() > 1000)
            || file
                .chunks
                .iter()
                .map(|c| c.text.chars().count())
                .sum::<usize>()
                > MAX_TEXT
            || file.chunks.iter().any(|c| c.label.len() > 800)
        {
            return Err(invalid("Geçersiz veya çok büyük dosya içeriği."));
        }
        for image in &file.images {
            if image.label.len() > 800 || image.data_url.len() > 3_000_000 {
                return Err(invalid("Görsel boyutu sınırı aşıldı."));
            }
            let bytes = STANDARD
                .decode(
                    image
                        .data_url
                        .strip_prefix("data:image/jpeg;base64,")
                        .ok_or_else(|| invalid("Geçersiz görsel biçimi."))?,
                )
                .map_err(|_| invalid("Görsel verisi okunamadı."))?;
            let reader = ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Jpeg);
            let (w, h) = reader
                .into_dimensions()
                .map_err(|_| invalid("Görsel verisi bozuk."))?;
            if w == 0 || h == 0 || w > 1536 || h > 1536 {
                return Err(invalid("Görsel çözünürlüğü sınırı aşıldı."));
            }
        }
    }
    Ok(())
}

fn add_text(target: &mut Attachment, label: &str, text: &str) {
    let used = target
        .chunks
        .iter()
        .map(|c| c.text.chars().count())
        .sum::<usize>();
    let remaining = MAX_TEXT.saturating_sub(used);
    let chars = text.chars().take(remaining).collect::<Vec<_>>();
    let mut start = 0;
    let mut part_number = 1;
    while start < chars.len() {
        let mut end = (start + 1600).min(chars.len());
        if end < chars.len() {
            let earliest = (end.saturating_sub(400)).max(start + 1);
            let sentence = (earliest..end).rev().find(|&i| {
                chars[i].is_whitespace() && i > 0 && matches!(chars[i - 1], '.' | '!' | '?')
            });
            let whitespace = (earliest..end).rev().find(|&i| chars[i].is_whitespace());
            if let Some(i) = sentence.or(whitespace) {
                end = i + 1;
            }
        }
        let text = chars[start..end].iter().collect::<String>();
        if !text.trim().is_empty() {
            target.chunks.push(TextChunk {
                label: format!("{label} · bölüm {part_number}"),
                text,
            });
        }
        start = end;
        part_number += 1;
    }
    if text.chars().count() > remaining
        && !target
            .warnings
            .iter()
            .any(|w| w.starts_with("Metin sınırı"))
    {
        target.warnings.push(
            "Metin sınırı: ilk 1.000.000 karakter alındı. Tam belge çevirisi için dosyayı bölün."
                .into(),
        );
    }
}
fn image_data(image: DynamicImage) -> Result<String> {
    let resized = image.thumbnail(1536, 1536).into_rgba8();
    let mut white = image::RgbaImage::from_pixel(
        resized.width(),
        resized.height(),
        image::Rgba([255, 255, 255, 255]),
    );
    image::imageops::overlay(&mut white, &resized, 0, 0);
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 85)
        .encode_image(&DynamicImage::ImageRgba8(white).into_rgb8())
        .map_err(|_| invalid("Görsel hazırlanamadı."))?;
    Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes)))
}
fn read_content(name: &str, bytes: &[u8], target: &mut Attachment, resources: &Path) -> Result<()> {
    match extension(name).as_str() {
        "png" | "jpg" | "jpeg" | "webp" => {
            if target.images.len() >= MAX_IMAGES {
                return Err(invalid(
                    "En fazla 128 görsel veya taranmış sayfa eklenebilir.",
                ));
            }
            let mut reader = ImageReader::new(Cursor::new(bytes))
                .with_guessed_format()
                .map_err(|_| invalid("Görsel biçimi okunamadı."))?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(8192);
            limits.max_image_height = Some(8192);
            limits.max_alloc = Some(128 * 1024 * 1024);
            reader.limits(limits);
            let image = reader.decode().map_err(|_| {
                invalid("Görsel bozuk veya çözünürlüğü çok büyük (en fazla 8192×8192).")
            })?;
            target.images.push(AttachmentImage {
                label: name.into(),
                data_url: image_data(image)?,
            });
        }
        "pdf" => read_pdf(name, bytes, target, resources)?,
        "docx" => read_docx(name, bytes, target)?,
        "doc" => {
            add_text(target, name, &legacy_word::text(bytes)?);
            target.warnings.push("Word 97–2003 metni alındı; gömülü nesneler, eski denklem nesneleri ve makrolar işlenmez. Formüller için DOCX/PDF tercih edin.".into());
        }
        ext if TEXT_EXTENSIONS.contains(&ext) => {
            let text = decode_text(bytes)?;
            add_text(target, name, &text);
        }
        _ => {
            return Err(invalid(
                "Desteklenmeyen dosya. PDF, DOCX, TXT, metin/kod, PNG, JPG, WebP veya ZIP seçin.",
            ))
        }
    }
    Ok(())
}
fn decode_text(bytes: &[u8]) -> Result<String> {
    let text = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if !bytes.len().is_multiple_of(2) {
            return Err(invalid("UTF-16 dosyası bozuk."));
        }
        let le = bytes[0] == 0xff;
        String::from_utf16(
            &bytes[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| {
                    if le {
                        u16::from_le_bytes([b[0], b[1]])
                    } else {
                        u16::from_be_bytes([b[0], b[1]])
                    }
                })
                .collect::<Vec<_>>(),
        )
        .map_err(|_| invalid("UTF-16 metni okunamadı."))?
    } else {
        std::str::from_utf8(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes))
            .map_err(|_| invalid("Metin UTF-8 veya UTF-16 olarak kaydedilmiş olmalı."))?
            .into()
    };
    if text.contains('\0') {
        return Err(invalid("İkili dosya metin olarak okunamaz."));
    }
    Ok(text)
}
fn archive(bytes: &[u8]) -> Result<zip::ZipArchive<Cursor<&[u8]>>> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| invalid("ZIP/DOCX arşivi okunamadı."))?;
    if zip.len() > 64 {
        return Err(invalid("Arşiv en fazla 64 kayıt içerebilir."));
    }
    let mut total = 0u64;
    for i in 0..zip.len() {
        let file = zip
            .by_index(i)
            .map_err(|_| invalid("Şifreli veya desteklenmeyen arşiv."))?;
        let name = file.name();
        if name.contains('\\')
            || name.contains(':')
            || name.starts_with('/')
            || name.split('/').any(|p| p == "..")
            || file.enclosed_name().is_none()
            || file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err(invalid(
                "Arşiv güvenli olmayan dosya yolu veya sembolik bağlantı içeriyor.",
            ));
        }
        total = total
            .checked_add(file.size())
            .ok_or_else(|| invalid("Arşiv çok büyük."))?;
        if total > MAX_ZIP_BYTES
            || file.size() > MAX_FILE_BYTES
            || file.size() > file.compressed_size().max(1).saturating_mul(200)
        {
            return Err(invalid(
                "Arşivin açılmış boyutu veya sıkıştırma oranı sınırı aşıldı.",
            ));
        }
    }
    Ok(zip)
}
fn zip_bytes(file: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(invalid("Arşiv dosyası boyut sınırını aştı."));
    }
    Ok(bytes)
}
fn read_zip(bytes: &[u8], target: &mut Attachment, resources: &Path) -> Result<()> {
    let mut zip = archive(bytes)?;
    target.entries = zip.len();
    for i in 0..zip.len() {
        let file = zip
            .by_index(i)
            .map_err(|_| invalid("Arşiv kaydı okunamadı."))?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().chars().take(180).collect::<String>();
        if !supported(&name) {
            target.warnings.push(format!(
                "Atlandı: {name} (desteklenmeyen tür; iç içe ZIP açılmaz)."
            ));
            continue;
        }
        let bytes = zip_bytes(file)?;
        read_content(
            &format!("{} / {name}", target.name),
            &bytes,
            target,
            resources,
        )?;
    }
    Ok(())
}
fn read_docx(name: &str, bytes: &[u8], target: &mut Attachment) -> Result<()> {
    let mut zip = archive(bytes)?;
    let xml = zip_bytes(
        zip.by_name("word/document.xml")
            .map_err(|_| invalid("DOCX belge metni bulunamadı."))?,
    )?;
    let text = docx::text(&xml)?;
    add_text(target, name, &text);
    target.warnings.push(
        "Word DOCX metni ve standart denklemleri alındı; gömülü görseller ve makrolar işlenmez."
            .into(),
    );
    Ok(())
}
pub fn pdfium(resources: &Path) -> Result<&'static Pdfium> {
    crate::model::verify_bundled_file(resources, "runtime/pdfium.dll")?;
    // pdfium-render 0.9 owns process-wide bindings; initialize once and reuse across reads.
    static PDFIUM: std::sync::OnceLock<std::result::Result<Pdfium, String>> =
        std::sync::OnceLock::new();
    let pdfium = PDFIUM
        .get_or_init(|| {
            Pdfium::bind_to_library(resources.join("runtime/pdfium.dll"))
                .map(Pdfium::new)
                .map_err(|_| "Paketin PDF motoru yüklenemedi.".to_string())
        })
        .as_ref()
        .map_err(|e| invalid(e.clone()))?;
    Ok(pdfium)
}
fn read_pdf(name: &str, bytes: &[u8], target: &mut Attachment, resources: &Path) -> Result<()> {
    let pdfium = pdfium(resources)?;
    let document = pdfium
        .load_pdf_from_byte_slice(bytes, None)
        .map_err(|_| invalid("PDF açılamadı. Şifreli veya bozuk olabilir."))?;
    let count = document.pages().len() as usize;
    if count > 100 || target.pages + count > 100 {
        return Err(invalid(
            "Bir ekte toplam en fazla 100 PDF sayfası işlenebilir.",
        ));
    }
    target.pages += count;
    for (i, page) in document.pages().iter().enumerate() {
        let text = page
            .text()
            .map_err(|_| invalid("PDF sayfa metni okunamadı."))?
            .all();
        let label = format!("{name} · sayfa {}", i + 1);
        let has_text = text.trim().chars().count() >= 20;
        let visual = !has_text
            || crate::math::looks_mathematical(&text)
            || page.objects().iter().any(|o| o.as_image_object().is_some());
        if has_text {
            add_text(target, &label, &text);
        }
        if visual && target.images.len() < MAX_IMAGES {
            let bitmap = page
                .render_with_config(
                    &PdfRenderConfig::new()
                        .set_target_width(1536)
                        .set_maximum_height(1536),
                )
                .map_err(|_| invalid("PDF sayfası görüntülenemedi."))?;
            target.images.push(AttachmentImage {
                label,
                data_url: image_data(
                    bitmap
                        .as_image()
                        .map_err(|_| invalid("PDF görseli hazırlanamadı."))?,
                )?,
            });
        } else if visual
            && !target
                .warnings
                .iter()
                .any(|w| w.starts_with("Taranmış sayfa sınırı"))
        {
            return Err(invalid(
                "PDF sayfaları görsel sınırını aşıyor; dosyayı bölün.",
            ));
        }
    }
    target.warnings.push("Metin, formül içeren ve görselli PDF sayfaları yerel olarak alındı. Çeviride tüm bölümler sırayla; sohbette ilgili metinler ve ilk 4 görsel sayfa kullanılır. Çok küçük yazılarda ilgili alanı ayrıca kırpıp ekleyin.".into());
    Ok(())
}

/// Select labelled excerpts with a small query-relevance score; no network or embeddings.
pub fn context(files: &[Attachment], query: &str, budget: usize) -> String {
    let terms = query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.chars().count() > 2)
        .map(String::from)
        .collect::<Vec<_>>();
    let mut candidates = files
        .iter()
        .flat_map(|f| f.chunks.iter())
        .enumerate()
        .map(|(i, c)| {
            let lower = c.text.to_lowercase();
            let score = terms.iter().filter(|t| lower.contains(t.as_str())).count();
            (score, i, c)
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(score, i, _)| (std::cmp::Reverse(*score), *i));
    let mut remaining = budget;
    let mut result = String::new();
    for (_, _, chunk) in candidates {
        if remaining < 100 {
            break;
        }
        let entry = format!("\n[{}]\n{}\n", chunk.label, chunk.text);
        let chosen = entry.chars().take(remaining).collect::<String>();
        remaining -= chosen.chars().count();
        result.push_str(&chosen);
    }
    result
}

#[cfg(test)]
mod tests;
