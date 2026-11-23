use crate::{Error, Result};
use std::io::{Cursor, Read};
fn invalid() -> Error {
    Error::Validation(
        "Bu Word dosyası okunamadı. Şifresiz Word 97–2003 .doc veya .docx kullanın.".into(),
    )
}
fn u16_at(bytes: &[u8], at: usize) -> Result<u16> {
    let b = bytes.get(at..at + 2).ok_or_else(invalid)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}
fn u32_at(bytes: &[u8], at: usize) -> Result<u32> {
    let b = bytes.get(at..at + 4).ok_or_else(invalid)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
pub fn text(bytes: &[u8]) -> Result<String> {
    let mut file = cfb::CompoundFile::open(Cursor::new(bytes)).map_err(|_| invalid())?;
    let mut document = Vec::new();
    file.open_stream("/WordDocument")
        .map_err(|_| invalid())?
        .take(super::MAX_FILE_BYTES + 1)
        .read_to_end(&mut document)?;
    if document.len() as u64 > super::MAX_FILE_BYTES
        || u16_at(&document, 0)? != 0xa5ec
        || u16_at(&document, 2)? < 0x00c1
    {
        return Err(invalid());
    }
    let flags = u16_at(&document, 10)?;
    if flags & 0x8100 != 0 {
        return Err(Error::Validation(
            "Şifreli Word dosyaları desteklenmiyor.".into(),
        ));
    }
    let mut at = 34 + u16_at(&document, 32)? as usize * 2;
    let longs = u16_at(&document, at)? as usize;
    at += 2;
    let main_chars = u32_at(&document, at + 12)? as usize;
    at += longs * 4;
    let pairs = u16_at(&document, at)? as usize;
    at += 2;
    if pairs <= 33 {
        return Err(invalid());
    }
    let offset = u32_at(&document, at + 33 * 8)? as usize;
    let size = u32_at(&document, at + 33 * 8 + 4)? as usize;
    let mut table = Vec::new();
    file.open_stream(if flags & 0x0200 != 0 {
        "/1Table"
    } else {
        "/0Table"
    })
    .map_err(|_| invalid())?
    .take(super::MAX_FILE_BYTES + 1)
    .read_to_end(&mut table)?;
    let clx = table
        .get(offset..offset.checked_add(size).ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    let mut at = 0;
    while clx.get(at) == Some(&1) {
        at += 3 + u16_at(clx, at + 1)? as usize;
    }
    if clx.get(at) != Some(&2) {
        return Err(invalid());
    }
    let len = u32_at(clx, at + 1)? as usize;
    let pieces = clx.get(at + 5..at + 5 + len).ok_or_else(invalid)?;
    if len < 4 || !(len - 4).is_multiple_of(12) {
        return Err(invalid());
    }
    let count = (len - 4) / 12;
    if count > 65536 {
        return Err(invalid());
    }
    let mut result = String::new();
    for i in 0..count {
        let start = u32_at(pieces, i * 4)? as usize;
        let end = u32_at(pieces, (i + 1) * 4)? as usize;
        if start >= main_chars {
            break;
        }
        if end < start {
            return Err(invalid());
        }
        let length = end.min(main_chars) - start;
        let fc = u32_at(pieces, (count + 1) * 4 + i * 8 + 2)?;
        let compressed = fc & 0x40000000 != 0;
        let offset = (fc & 0x3fffffff) as usize / if compressed { 2 } else { 1 };
        let length_bytes = length
            .checked_mul(if compressed { 1 } else { 2 })
            .ok_or_else(invalid)?;
        let data = document
            .get(offset..offset.checked_add(length_bytes).ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        let text = if compressed {
            encoding_rs::WINDOWS_1252
                .decode_without_bom_handling(data)
                .0
                .into_owned()
        } else {
            String::from_utf16(
                &data
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| u16::from_le_bytes(*b))
                    .collect::<Vec<_>>(),
            )
            .map_err(|_| invalid())?
        };
        result.push_str(&text);
        if result.chars().count() > super::MAX_TEXT {
            return Err(Error::Validation(
                "Word metni 1 milyon karakter sınırını aşıyor.".into(),
            ));
        }
    }
    Ok(result
        .chars()
        .filter_map(|c| match c {
            '\r' | '\u{b}' => Some('\n'),
            '\u{7}' => Some('\t'),
            c if !c.is_control() || c == '\n' || c == '\t' => Some(c),
            _ => None,
        })
        .collect())
}
