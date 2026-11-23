use crate::{
    attachments::{self, Attachment},
    model::ModelClient,
    prompts::{self, ChatMessage},
    types::AppSettings,
    Error, Result,
};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct TranslationPart {
    pub label: String,
    pub text: String,
    pub image: Option<String>,
}
pub fn parts(files: &[Attachment]) -> Result<Vec<TranslationPart>> {
    attachments::validate_batch(files)?;
    if files
        .iter()
        .flat_map(|f| &f.warnings)
        .any(|w| w.starts_with("Metin sınırı") || w.starts_with("Taranmış sayfa sınırı"))
    {
        return Err(Error::Validation("Belgenin bir bölümü dosya sınırını aşıyor. Tam çeviri için belgeyi daha küçük parçalara bölün.".into()));
    }
    let mut parts = Vec::new();
    for file in files {
        let mut batch = file
            .chunks
            .iter()
            .map(|c| TranslationPart {
                label: c.label.clone(),
                text: c.text.clone(),
                image: if file.kind == "pdf" {
                    file.images
                        .iter()
                        .find(|i| page(&i.label) == page(&c.label))
                        .map(|i| i.data_url.clone())
                } else {
                    None
                },
            })
            .chain(
                file.images
                    .iter()
                    .filter(|i| {
                        file.kind != "pdf"
                            || !file.chunks.iter().any(|c| page(&c.label) == page(&i.label))
                    })
                    .map(|i| TranslationPart {
                        label: i.label.clone(),
                        text: String::new(),
                        image: Some(i.data_url.clone()),
                    }),
            )
            .collect::<Vec<_>>();
        if file.kind == "pdf" {
            batch.sort_by_key(|p| page(&p.label));
        }
        parts.extend(batch);
    }
    if parts.is_empty() {
        return Err(Error::Validation(
            "Çevrilebilecek belge içeriği yok.".into(),
        ));
    }
    Ok(parts)
}
fn page(label: &str) -> usize {
    label
        .rsplit_once(" · sayfa ")
        .and_then(|(_, s)| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

pub async fn translate_files(
    client: &ModelClient,
    files: &[Attachment],
    language: &str,
    settings: &AppSettings,
    cancel: CancellationToken,
    mut delta: impl FnMut(String),
    mut status: impl FnMut(String),
) -> Result<String> {
    let tasks = parts(files)?;
    let total = tasks.len();
    let mut output = String::new();
    let mut previous_label = String::new();
    let mut previous_source = String::new();
    let mut previous_translation = String::new();
    let mut translation_settings = settings.clone();
    translation_settings.temperature = settings.temperature.min(0.3);
    for (i, part) in tasks.into_iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        status(format!("Belge çevriliyor · {}/{total}", i + 1));
        let label = part
            .label
            .rsplit_once(" · bölüm ")
            .map(|(a, _)| a)
            .unwrap_or(&part.label);
        let separator = if label != previous_label {
            format!("\n\n## {label}\n\n")
        } else {
            "\n".into()
        };
        previous_label = label.into();
        output.push_str(&separator);
        delta(separator);
        let mut system = prompts::translation_system(language)?;
        system.push_str(" The source is one sequential part of an attached document. Translate every sentence; do not summarize or omit the ending. Output only its translated content. For an image, read and translate the visible text; never invent unreadable text.");
        system.push_str(" Preserve mathematical meaning and write formulas using LaTeX: $...$ inline or $$...$$ on a separate line. Never spell formulas as Unicode code points. If both text and an image are supplied, translate only the supplied text portion and use the image to recover its equations; do not repeat other page content.");
        let mut messages = vec![ChatMessage {
            role: "system".into(),
            content: system,
            images: vec![],
        }];
        if part.image.is_none() && !previous_source.is_empty() {
            messages[0].content.push_str(" Earlier turns contain already translated context. Translate only the final user message; do not repeat earlier content.");
            messages.push(ChatMessage {
                role: "user".into(),
                content: previous_source.clone(),
                images: vec![],
            });
            messages.push(ChatMessage {
                role: "assistant".into(),
                content: previous_translation.clone(),
                images: vec![],
            });
        }
        let current_source = part.text.clone();
        messages.push(ChatMessage {
            role: "user".into(),
            content: if part.image.is_some() && part.text.is_empty() {
                "Translate all readable text in the attached page image into the target language."
                    .into()
            } else {
                part.text
            },
            images: part.image.into_iter().collect(),
        });
        let answer = client
            .generate_with_progress(
                messages,
                cancel.clone(),
                &translation_settings,
                &mut delta,
                &mut status,
            )
            .await?;
        output.push_str(&answer);
        previous_source = current_source
            .chars()
            .rev()
            .take(500)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        previous_translation = answer
            .chars()
            .rev()
            .take(700)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if output.len() > 4_000_000 {
            return Err(Error::OutputLimit);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachments::{AttachmentImage, TextChunk};
    #[test]
    fn all_document_parts_keep_page_order_without_search_filter() {
        let file = Attachment {
            id: uuid::Uuid::new_v4().to_string(),
            name: "test.pdf".into(),
            kind: "pdf".into(),
            bytes: 100,
            pages: 12,
            entries: 1,
            warnings: vec![],
            chunks: vec![
                TextChunk {
                    label: "test.pdf · sayfa 2 · bölüm 1".into(),
                    text: "Two".into(),
                },
                TextChunk {
                    label: "test.pdf · sayfa 12 · bölüm 1".into(),
                    text: "Twelve".into(),
                },
            ],
            images: vec![],
        };
        let mut file = file;
        use base64::Engine as _;
        let mut jpeg = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(8, 8)
            .write_to(&mut jpeg, image::ImageFormat::Jpeg)
            .unwrap();
        let data = format!(
            "data:image/jpeg;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(jpeg.into_inner())
        );
        file.images = (3..=7)
            .map(|n| AttachmentImage {
                label: format!("test.pdf · sayfa {n}"),
                data_url: data.clone(),
            })
            .collect();
        file.images.push(AttachmentImage {
            label: "test.pdf · sayfa 2".into(),
            data_url: data,
        });
        let p = parts(&[file]).unwrap();
        assert_eq!(p.len(), 7);
        assert!(p[0].image.is_some());
        assert_eq!(p[0].text, "Two");
        assert_eq!(p[6].text, "Twelve");
        assert_eq!(page("test.pdf · sayfa 9"), 9);
    }
}
