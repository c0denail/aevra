use super::*;
use crate::math::{self, MathAsset, Part, Rendered};

enum RunKind {
    Text(String),
    Formula(usize),
}
struct Run {
    kind: RunKind,
    width: f32,
}
struct Row {
    runs: Vec<Run>,
    size: f32,
    ascent: f32,
    descent: f32,
    gap: f32,
    center: bool,
    indent: f32,
}
impl Row {
    fn new(size: f32, code: bool) -> Self {
        Self {
            runs: vec![],
            size,
            ascent: size,
            descent: size * 0.25,
            gap: 4.0,
            center: false,
            indent: if code { 8.0 } else { 0.0 },
        }
    }
    fn width(&self) -> f32 {
        self.runs.iter().map(|r| r.width).sum()
    }
    fn height(&self) -> f32 {
        self.ascent + self.descent + self.gap
    }
}
fn text_run(row: &mut Row, text: &str, w: f32) {
    if let Some(Run {
        kind: RunKind::Text(previous),
        width,
    }) = row.runs.last_mut()
    {
        previous.push_str(text);
        *width += w;
    } else {
        row.runs.push(Run {
            kind: RunKind::Text(text.into()),
            width: w,
        });
    }
}
fn normalize(text: &str, font: &ttf_parser::Face<'_>) -> String {
    text.chars()
        .filter(|c| {
            (!c.is_control() || *c == '\n' || *c == '\t') && !('\u{e000}'..='\u{e003}').contains(c)
        })
        .map(|c| {
            if c == '\n' || c == '\t' || font.glyph_index(c).is_some() {
                c.to_string()
            } else {
                format!("[U+{:04X}]", c as u32)
            }
        })
        .collect()
}

pub(super) fn create(
    title: &str,
    text: &str,
    status: &str,
    sources: &[Source],
    resources: &Path,
    assets: &[MathAsset],
) -> Result<Vec<u8>> {
    if text.len() > 4_000_000
        || assets.len() > 200
        || assets.iter().map(|a| a.svg.len()).sum::<usize>() > 20_000_000
    {
        return Err(Error::Validation(
            "Formüllü PDF boyut sınırını aşıyor.".into(),
        ));
    }
    model::verify_bundled_file(resources, "fonts/NotoSans-Regular.ttf")?;
    let font_bytes = std::fs::read(resources.join("fonts/NotoSans-Regular.ttf"))?;
    let face = ttf_parser::Face::parse(&font_bytes, 0).map_err(|_| failure())?;
    let mut rendered: Vec<Rendered> = Vec::new();
    let mut tex = Vec::new();
    let mut canonical = String::new();
    for part in math::parts(text) {
        match part {
            Part::Text(t) => canonical.push_str(&normalize(&t, &face)),
            Part::Formula {
                tex: formula,
                display,
            } => {
                let asset=assets.iter().find(|a|a.tex==formula&&a.display==display).ok_or_else(||Error::Validation("PDF formülleri hazırlanamadı. Güncel uygulamadaki PDF düğmesiyle yeniden deneyin.".into()))?;
                let index = rendered.len();
                rendered.push(math::render(asset, if display { 14.0 } else { 11.5 })?);
                tex.push(formula);
                if display {
                    canonical.push_str(&format!("\n\u{e002}{index}\u{e003}\n"));
                } else {
                    canonical.push_str(&format!("\u{e000}{index}\u{e001}"));
                }
            }
        }
    }
    if !sources.is_empty() {
        canonical.push_str("\n\n# Kaynaklar\n");
        for s in sources {
            canonical.push_str(&normalize(
                &format!("[{}] {}\n{}\n", s.id, s.title, s.url),
                &face,
            ));
        }
    }
    let mut rows = Vec::new();
    let mut code = false;
    for raw in canonical.lines() {
        if raw.trim_start().starts_with("```") {
            code = !code;
            continue;
        }
        if let Some(index) = raw
            .strip_prefix('\u{e002}')
            .and_then(|s| s.strip_suffix('\u{e003}'))
            .and_then(|s| s.parse::<usize>().ok())
        {
            let formula = &rendered[index];
            rows.push(Row {
                runs: vec![Run {
                    kind: RunKind::Formula(index),
                    width: formula.width,
                }],
                size: 11.5,
                ascent: formula.ascent,
                descent: formula.height - formula.ascent,
                gap: 10.0,
                center: true,
                indent: 0.0,
            });
            continue;
        }
        let heading = !code && raw.starts_with('#');
        let clean = if heading {
            raw.trim_start_matches('#').trim().to_string()
        } else if code {
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
        let mut row = Row::new(size, code);
        let mut i = 0;
        while i < clean.len() {
            let rest = &clean[i..];
            if rest.starts_with('\u{e000}') {
                let end = rest.find('\u{e001}').ok_or_else(failure)?;
                let index = rest['\u{e000}'.len_utf8()..end]
                    .parse::<usize>()
                    .map_err(|_| failure())?;
                let formula = &rendered[index];
                if row.width() + formula.width > 490.0 && !row.runs.is_empty() {
                    rows.push(row);
                    row = Row::new(size, code);
                }
                row.ascent = row.ascent.max(formula.ascent);
                row.descent = row.descent.max(formula.height - formula.ascent);
                row.runs.push(Run {
                    kind: RunKind::Formula(index),
                    width: formula.width,
                });
                i += end + '\u{e001}'.len_utf8();
                continue;
            }
            let end = rest.find('\u{e000}').unwrap_or(rest.len());
            let text = &rest[..end];
            for word in text.split_inclusive(char::is_whitespace) {
                let w = width(word, size, &face);
                if row.width() + w > 490.0 && !row.runs.is_empty() {
                    rows.push(row);
                    row = Row::new(size, code);
                }
                if w <= 490.0 {
                    text_run(&mut row, word, w);
                } else {
                    for c in word.chars() {
                        let piece = c.to_string();
                        let w = width(&piece, size, &face);
                        if row.width() + w > 490.0 && !row.runs.is_empty() {
                            rows.push(row);
                            row = Row::new(size, code);
                        }
                        text_run(&mut row, &piece, w);
                    }
                }
            }
            i += end;
        }
        if heading {
            row.gap = 10.0;
        }
        rows.push(row);
    }
    let mut pages: Vec<Vec<Row>> = vec![vec![]];
    let mut top = if status == "complete" { 735.0 } else { 700.0 };
    for row in rows {
        if top - row.height() < 55.0 {
            if pages.len() >= 500 {
                return Err(Error::Validation("PDF 500 sayfa sınırını aşıyor.".into()));
            }
            pages.push(vec![]);
            top = 780.0;
        }
        top -= row.height();
        pages.last_mut().unwrap().push(row);
    }
    let mut document = attachments::pdfium(resources)?
        .create_new_pdf()
        .map_err(|_| failure())?;
    let font = document
        .fonts_mut()
        .load_true_type_from_bytes(&font_bytes, true)
        .map_err(|_| failure())?;
    let total = pages.len();
    for (index, rows) in pages.into_iter().enumerate() {
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
            .map_err(|_| failure())?;
        if index == 0 {
            for (i, line) in wrap(
                &normalize(&title.chars().take(160).collect::<String>(), &face),
                17.0,
                &face,
            )
            .into_iter()
            .take(2)
            .enumerate()
            {
                page.objects_mut()
                    .create_text_object(
                        PdfPoints::new(50.0),
                        PdfPoints::new(776.0 - i as f32 * 22.0),
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
            }
        }
        let mut top = if index == 0 {
            if status == "complete" {
                735.0
            } else {
                700.0
            }
        } else {
            780.0
        };
        for row in rows {
            let baseline = top - row.ascent;
            let mut x = 50.0
                + row.indent
                + if row.center {
                    (490.0 - row.width()) / 2.0
                } else {
                    0.0
                };
            for run in row.runs {
                match run.kind {
                    RunKind::Text(text) => {
                        if !text.is_empty() {
                            page.objects_mut()
                                .create_text_object(
                                    PdfPoints::new(x),
                                    PdfPoints::new(baseline),
                                    text,
                                    font,
                                    PdfPoints::new(row.size),
                                )
                                .map_err(|_| failure())?;
                        }
                    }
                    RunKind::Formula(id) => {
                        let f = &rendered[id];
                        page.objects_mut()
                            .create_image_object(
                                PdfPoints::new(x),
                                PdfPoints::new(baseline - (f.height - f.ascent)),
                                &f.image,
                                Some(PdfPoints::new(f.width)),
                                Some(PdfPoints::new(f.height)),
                            )
                            .map_err(|_| failure())?;
                        let mut hidden = page
                            .objects_mut()
                            .create_text_object(
                                PdfPoints::new(x),
                                PdfPoints::new(baseline),
                                normalize(&format!("${}$", tex[id]), &face),
                                font,
                                PdfPoints::new(0.2),
                            )
                            .map_err(|_| failure())?;
                        hidden
                            .as_text_object_mut()
                            .ok_or_else(failure)?
                            .set_render_mode(PdfPageTextRenderMode::Invisible)
                            .map_err(|_| failure())?;
                    }
                }
                x += run.width;
            }
            top -= row.ascent + row.descent + row.gap;
        }
        page.objects_mut()
            .create_text_object(
                PdfPoints::new(50.0),
                PdfPoints::new(30.0),
                format!("Aevra | {} / {total}", index + 1),
                font,
                PdfPoints::new(8.5),
            )
            .map_err(|_| failure())?;
    }
    document.save_to_bytes().map_err(|_| failure())
}
