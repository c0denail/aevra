use crate::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MathAsset {
    pub tex: String,
    pub display: bool,
    pub svg: String,
}
#[derive(Debug, Clone)]
pub enum Part {
    Text(String),
    Formula { tex: String, display: bool },
}
pub fn parts(source: &str) -> Vec<Part> {
    let mut out = Vec::new();
    let mut buffer = String::new();
    let mut i = 0;
    let mut fence = false;
    while i < source.len() {
        let s = &source[i..];
        if s.starts_with("```") {
            fence = !fence;
            buffer.push_str("```");
            i += 3;
            continue;
        }
        if !fence && s.starts_with('`') {
            if let Some(end) = s[1..].find('`') {
                buffer.push_str(&s[..end + 2]);
                i += end + 2;
                continue;
            }
        }
        let delimiter = if !fence && (i == 0 || source.as_bytes()[i - 1] != b'\\') {
            if s.starts_with("$$") {
                Some(("$$", "$$", true))
            } else if s.starts_with("\\[") {
                Some(("\\[", "\\]", true))
            } else if s.starts_with("\\(") {
                Some(("\\(", "\\)", false))
            } else if s.starts_with('$') {
                Some(("$", "$", false))
            } else {
                None
            }
        } else {
            None
        };
        if let Some((open, close, display)) = delimiter {
            if let Some(end) = s[open.len()..].find(close) {
                let end = end + open.len();
                let tex = &s[open.len()..end];
                if !tex.trim().is_empty() && (display || !tex.contains('\n')) {
                    if !buffer.is_empty() {
                        out.push(Part::Text(std::mem::take(&mut buffer)));
                    }
                    out.push(Part::Formula {
                        tex: tex.trim().into(),
                        display,
                    });
                    i += end + close.len();
                    continue;
                }
            }
        }
        let c = s.chars().next().unwrap();
        buffer.push(c);
        i += c.len_utf8();
    }
    if !buffer.is_empty() {
        out.push(Part::Text(buffer));
    }
    out
}
pub fn looks_mathematical(text: &str) -> bool {
    text.contains("\\frac")
        || text.contains("\\sqrt")
        || text
            .chars()
            .any(|c| "∫∑∏√∂∇∞≤≥≠±∈⊆∪∩²³₀₁₂₃αβγθλμπσφω".contains(c))
        || text.lines().any(|line| line.contains('='))
}
pub struct Rendered {
    pub image: image::DynamicImage,
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
}
pub fn render(asset: &MathAsset, font_size: f32) -> Result<Rendered> {
    if asset.tex.len() > 10_000 || asset.svg.len() > 2_000_000 {
        return Err(Error::Validation(
            "PDF formülü boyut sınırını aşıyor.".into(),
        ));
    }
    let mut xml = quick_xml::Reader::from_str(&asset.svg);
    let mut count = 0;
    let mut metrics = None;
    loop {
        use quick_xml::events::Event;
        match xml
            .read_event()
            .map_err(|_| Error::Validation("Formül SVG verisi bozuk.".into()))?
        {
            Event::Start(e) | Event::Empty(e) => {
                count += 1;
                if count > 20000 {
                    return Err(Error::Validation("Formül çok karmaşık.".into()));
                }
                let name = e.local_name();
                if ![
                    b"svg".as_slice(),
                    b"g",
                    b"defs",
                    b"path",
                    b"use",
                    b"rect",
                    b"line",
                    b"polyline",
                    b"polygon",
                    b"circle",
                    b"ellipse",
                    b"title",
                    b"desc",
                ]
                .contains(&name.as_ref())
                {
                    return Err(Error::Validation(
                        "Formülde izin verilmeyen SVG öğesi.".into(),
                    ));
                }
                for attr in e.attributes() {
                    let attr =
                        attr.map_err(|_| Error::Validation("Formül SVG niteliği bozuk.".into()))?;
                    let key = String::from_utf8_lossy(attr.key.as_ref());
                    let value = attr
                        .decode_and_unescape_value(xml.decoder())
                        .map_err(|_| Error::Validation("Formül SVG değeri bozuk.".into()))?;
                    if key.starts_with("on")
                        || (key.ends_with("href") && !value.starts_with('#'))
                        || value.to_ascii_lowercase().contains("url(")
                    {
                        return Err(Error::Validation(
                            "Formülde dış içerik kullanılamaz.".into(),
                        ));
                    }
                    if key == "viewBox" && name.as_ref() == b"svg" {
                        let values = value
                            .split(|c: char| c.is_whitespace() || c == ',')
                            .filter(|s| !s.is_empty())
                            .map(str::parse::<f32>)
                            .collect::<std::result::Result<Vec<_>, _>>()
                            .map_err(|_| Error::Validation("Formül ölçüleri bozuk.".into()))?;
                        if values.len() == 4
                            && values.iter().all(|v| v.is_finite())
                            && values[2] > 0.0
                            && values[3] > 0.0
                        {
                            metrics = Some((values[1], values[2], values[3]));
                        }
                    }
                }
            }
            Event::DocType(_) => {
                return Err(Error::Validation("Formülde DTD kullanılamaz.".into()))
            }
            Event::Eof => break,
            _ => (),
        }
    }
    let (y, w, h) =
        metrics.ok_or_else(|| Error::Validation("Formül görünüm ölçüleri yok.".into()))?;
    let natural_w = w / 1000.0 * font_size;
    let natural_h = h / 1000.0 * font_size;
    let scale = (490.0 / natural_w).min(240.0 / natural_h).min(1.0);
    let width = (natural_w * scale).max(1.0);
    let height = (natural_h * scale).max(1.0);
    let tree = resvg::usvg::Tree::from_str(&asset.svg, &resvg::usvg::Options::default())
        .map_err(|_| Error::Validation("Formül dizilemesi okunamadı.".into()))?;
    let pixel_w = (width * 4.0).ceil() as u32;
    let pixel_h = (height * 4.0).ceil() as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixel_w, pixel_h)
        .ok_or_else(|| Error::Validation("Formül belleğe sığmadı.".into()))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            pixel_w as f32 / tree.size().width(),
            pixel_h as f32 / tree.size().height(),
        ),
        &mut pixmap.as_mut(),
    );
    let bytes = pixmap
        .encode_png()
        .map_err(|_| Error::Validation("Formül görseli hazırlanamadı.".into()))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|_| Error::Validation("Formül görseli okunamadı.".into()))?;
    Ok(Rendered {
        image,
        width,
        height,
        ascent: (-y / h * height).clamp(0.0, height),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_formulas_and_skips_code() {
        let p = parts("Sonuç $x^2$ ve \\[\\frac{1}{2}\\] `a$b$`\n```rust\nlet x=\"$a$\";\n```");
        assert_eq!(
            p.iter()
                .filter(|p| matches!(p, Part::Formula { .. }))
                .count(),
            2
        );
    }
    #[test]
    fn rejects_external_svg_resources() {
        let a = MathAsset {
            tex: "x".into(),
            display: true,
            svg: "<svg viewBox=\"0 0 1000 1000\"><use href=\"https://example.com/a\"/></svg>"
                .into(),
        };
        assert!(render(&a, 12.0).is_err());
    }
}
