//! Local, paginated text PDF export. No browser, shell or external document engine.
use crate::{attachments, model, types::Source, Error, Result};
use pdfium_render::prelude::*;
use std::path::Path;
mod math_layout;
pub fn create_pdf(
    title: &str,
    text: &str,
    status: &str,
    sources: &[Source],
    resources: &Path,
) -> Result<Vec<u8>> {
    create_pdf_with_math(title, text, status, sources, resources, &[])
}
pub fn create_pdf_with_math(
    title: &str,
    text: &str,
    status: &str,
    sources: &[Source],
    resources: &Path,
    assets: &[crate::math::MathAsset],
) -> Result<Vec<u8>> {
    if crate::math::parts(text)
        .iter()
        .any(|p| matches!(p, crate::math::Part::Formula { .. }))
    {
        math_layout::create(title, text, status, sources, resources, assets)
    } else {
        create_text_pdf(title, text, status, sources, resources)
    }
}
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| Error::Validation("PDF klasörü geçersiz.".into()))?;
    let temporary = parent.join(format!(".aevra-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })()
    .map_err(Error::Io);
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[derive(Debug)]
struct Line {
    text: String,
    size: f32,
    gap: f32,
    code: bool,
}
fn failure() -> Error {
    Error::Validation("PDF oluşturulamadı. Paket PDF motorunu ve yazı tipini kontrol edin.".into())
}
fn width(text: &str, size: f32, font: &ttf_parser::Face<'_>) -> f32 {
    text.chars()
        .filter_map(|c| font.glyph_index(c).and_then(|g| font.glyph_hor_advance(g)))
        .map(|v| v as f32)
        .sum::<f32>()
        / font.units_per_em() as f32
        * size
}
fn wrap(text: &str, size: f32, font: &ttf_parser::Face<'_>) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_inclusive(char::is_whitespace) {
        if width(&(line.clone() + word), size, font) > 490.0 && !line.is_empty() {
            lines.push(line.trim_end().into());
            line.clear();
        }
        for c in word.chars() {
            let next = format!("{line}{c}");
            if width(&next, size, font) > 490.0 && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            line.push(c);
        }
    }
    lines.push(line.trim_end().into());
    lines
}
fn layout(text: &str, font: &ttf_parser::Face<'_>) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut code = false;
    for raw in text.lines() {
        if raw.trim_start().starts_with("```") {
            code = !code;
            lines.push(Line {
                text: String::new(),
                size: 9.0,
                gap: 4.0,
                code,
            });
            continue;
        }
        let heading = !code && raw.starts_with('#');
        let raw = if heading {
            raw.trim_start_matches('#').trim()
        } else {
            raw
        };
        let text = if code {
            raw.replace('\t', "    ")
        } else {
            raw.replace("**", "").replace('`', "")
        };
        let size = if heading {
            14.0
        } else if code {
            9.0
        } else {
            10.5
        };
        for part in wrap(&text, size, font) {
            lines.push(Line {
                text: part,
                size,
                gap: if heading { 9.0 } else { 4.0 },
                code,
            });
        }
    }
    lines
}
fn create_text_pdf(
    title: &str,
    text: &str,
    status: &str,
    sources: &[Source],
    resources: &Path,
) -> Result<Vec<u8>> {
    if text.trim().is_empty() || text.len() > 4_000_000 {
        return Err(Error::Validation(
            "PDF için boş olmayan, en fazla 4 MB metin gerekir.".into(),
        ));
    }
    model::verify_bundled_file(resources, "fonts/NotoSans-Regular.ttf")?;
    let font_bytes = std::fs::read(resources.join("fonts/NotoSans-Regular.ttf"))?;
    let face = ttf_parser::Face::parse(&font_bytes, 0).map_err(|_| failure())?;
    let mut missing = 0;
    let mut all = 0;
    let normalize = |s: &str, missing: &mut usize, all: &mut usize| {
        s.chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .map(|c| {
                *all += 1;
                if c == '\n' || c == '\t' || face.glyph_index(c).is_some() {
                    c.to_string()
                } else {
                    *missing += 1;
                    format!("[U+{:04X}]", c as u32)
                }
            })
            .collect::<String>()
    };
    let mut body = normalize(text, &mut missing, &mut all);
    if missing > 20 && missing * 20 > all {
        return Err(Error::Validation("Bu metnin yazı sistemi mevcut PDF yazı tipinde desteklenmiyor. Türkçe, Almanca ve diğer Latin/Kiril metinleri kaydedilebilir.".into()));
    }
    if !sources.is_empty() {
        body.push_str("\n\n# Kaynaklar\n");
        for s in sources {
            body.push_str(&format!("[{}] {}\n{}\n", s.id, s.title, s.url));
        }
    }
    if missing > 0 {
        body.push_str(
            "\n\nDesteklenmeyen bazı simgeler Unicode kodlarıyla [U+....] gösterilmiştir.",
        );
    }
    let title = normalize(&title.chars().take(160).collect::<String>(), &mut 0, &mut 0);
    let lines = layout(&body, &face);
    let mut pages: Vec<Vec<Line>> = vec![Vec::new()];
    let mut y = if status == "complete" {
        735.0f32
    } else {
        700.0f32
    };
    for line in lines {
        let height = line.size + line.gap;
        if y - height < 55.0 {
            if pages.len() >= 500 {
                return Err(Error::Validation(
                    "PDF 500 sayfa sınırını aşıyor; yanıtı parçalara bölün.".into(),
                ));
            }
            pages.push(Vec::new());
            y = 780.0;
        }
        y -= height;
        pages.last_mut().unwrap().push(line);
    }
    let pdfium = attachments::pdfium(resources)?;
    let mut document = pdfium.create_new_pdf().map_err(|_| failure())?;
    let font = document
        .fonts_mut()
        .load_true_type_from_bytes(&font_bytes, true)
        .map_err(|_| failure())?;
    let total = pages.len();
    for (index, lines) in pages.into_iter().enumerate() {
        let mut page = document
            .pages_mut()
            .create_page_at_end(PdfPagePaperSize::a4())
            .map_err(|_| failure())?;
        page.objects_mut()
            .create_text_object(
                PdfPoints::new(50.0),
                PdfPoints::new(808.0),
                "AEVRA",
                font,
                PdfPoints::new(10.0),
            )
            .map_err(|_| failure())?
            .set_fill_color(PdfColor::new(65, 88, 126, 255))
            .map_err(|_| failure())?;
        let mut y = if index == 0 { 735.0 } else { 780.0 };
        if index == 0 {
            for (line_number, line) in wrap(&title, 17.0, &face).into_iter().take(2).enumerate() {
                page.objects_mut()
                    .create_text_object(
                        PdfPoints::new(50.0),
                        PdfPoints::new(776.0 - line_number as f32 * 22.0),
                        line,
                        font,
                        PdfPoints::new(17.0),
                    )
                    .map_err(|_| failure())?;
            }
            if status != "complete" {
                page.objects_mut()
                    .create_text_object(
                        PdfPoints::new(50.0),
                        PdfPoints::new(723.0),
                        "Kısmi yanıt - içerik tamamlanmamış olabilir.",
                        font,
                        PdfPoints::new(9.0),
                    )
                    .map_err(|_| failure())?;
                y = 700.0;
            }
        }
        for line in lines {
            if !line.text.is_empty() {
                page.objects_mut()
                    .create_text_object(
                        PdfPoints::new(if line.code { 58.0 } else { 50.0 }),
                        PdfPoints::new(y),
                        line.text,
                        font,
                        PdfPoints::new(line.size),
                    )
                    .map_err(|_| failure())?
                    .set_fill_color(PdfColor::new(35, 41, 51, 255))
                    .map_err(|_| failure())?;
            }
            y -= line.size + line.gap;
        }
        page.objects_mut()
            .create_text_object(
                PdfPoints::new(50.0),
                PdfPoints::new(30.0),
                format!("Aevra  |  {} / {total}", index + 1),
                font,
                PdfPoints::new(8.5),
            )
            .map_err(|_| failure())?;
    }
    document.save_to_bytes().map_err(|_| failure())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_empty_text_before_loading_native_runtime() {
        assert!(create_pdf("Title", " ", "complete", &[], Path::new("missing")).is_err());
    }
    #[test]
    fn atomic_pdf_save_replaces_destination_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.pdf");
        std::fs::write(&path, b"old").unwrap();
        write_atomic(&path, b"%PDF-new").unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"%PDF-new");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
