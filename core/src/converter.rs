// VietFlow Core - Charset Converter & Accent Stripping Toolkit
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::models::{CharsetId, VnLexiTone};
use crate::tables::{
    IS_VN_VOWEL, STD_VN_ROOT_CHAR, TCVN3_TABLE, TOTAL_VN_CHARS, UNICODE_TABLE, VNI_WIN_TABLE,
};

pub fn strip_accents(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        if let Some(idx) = UNICODE_TABLE.iter().position(|&x| x == c) {
            let root = STD_VN_ROOT_CHAR[idx] as usize;
            if root < TOTAL_VN_CHARS {
                let rc = match UNICODE_TABLE[root] {
                    'Â' | 'Ă' => 'A',
                    'â' | 'ă' => 'a',
                    'Ê' => 'E',
                    'ê' => 'e',
                    'Ô' | 'Ơ' => 'O',
                    'ô' | 'ơ' => 'o',
                    'Ư' => 'U',
                    'ư' => 'u',
                    'Đ' => 'D',
                    'đ' => 'd',
                    other => other,
                };
                result.push(rc);
                continue;
            }
        }
        result.push(c);
    }
    result
}

/// Decode raw bytes in a legacy charset into Unicode text. This is the inverse
/// of the byte-oriented encoder below and is used by the clipboard converter,
/// where TCVN3/VNI content is not valid UTF-8.
fn decode_source_bytes(input: &[u8], source: CharsetId) -> String {
    match source {
        CharsetId::Tcvn3 => {
            let mut out = String::with_capacity(input.len());
            for &byte in input {
                if let Some(mut idx) = TCVN3_TABLE.iter().position(|&x| x == byte) {
                    if idx & 1 == 0
                        && idx + 1 < TOTAL_VN_CHARS
                        && TCVN3_TABLE[idx + 1] == byte
                    {
                        idx += 1;
                    }
                    out.push(UNICODE_TABLE[idx]);
                } else {
                    out.push(char::from(byte));
                }
            }
            out
        }
        CharsetId::VniWindows => {
            let mut out = String::with_capacity(input.len());
            let mut i = 0;
            while i < input.len() {
                let byte = input[i];
                if i + 1 < input.len() {
                    let pair = ((input[i + 1] as u16) << 8) | (byte as u16);
                    if let Some(idx) = VNI_WIN_TABLE.iter().position(|&x| x == pair) {
                        out.push(UNICODE_TABLE[idx]);
                        i += 2;
                        continue;
                    }
                }
                if let Some(idx) = VNI_WIN_TABLE
                    .iter()
                    .position(|&x| (x & 0xFF) == byte as u16 && (x >> 8) == 0 && byte > 127)
                {
                    out.push(UNICODE_TABLE[idx]);
                } else {
                    out.push(char::from(byte));
                }
                i += 1;
            }
            out
        }
        _ => String::from_utf8_lossy(input).into_owned(),
    }
}

/// Encode Unicode text as raw bytes for legacy charsets. Unicode and Unicode
/// Composite remain UTF-8; TCVN3 and VNI Windows return the bytes applications
/// using those charsets actually expect.
pub fn convert_charset_bytes(input: &[u8], source: CharsetId, target: CharsetId) -> Vec<u8> {
    let unicode = decode_source_bytes(input, source);
    match target {
        CharsetId::Tcvn3 => {
            let mut out = Vec::with_capacity(unicode.len());
            for c in unicode.chars() {
                if let Some(idx) = UNICODE_TABLE.iter().position(|&x| x == c) {
                    if idx < TOTAL_VN_CHARS {
                        out.push(TCVN3_TABLE[idx]);
                        continue;
                    }
                }
                if (c as u32) <= 0x7F {
                    out.push(c as u8);
                } else {
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
            }
            out
        }
        CharsetId::VniWindows => {
            let mut out = Vec::with_capacity(unicode.len() * 2);
            for c in unicode.chars() {
                if let Some(idx) = UNICODE_TABLE.iter().position(|&x| x == c) {
                    if idx < TOTAL_VN_CHARS {
                        let word = VNI_WIN_TABLE[idx];
                        out.push((word & 0xFF) as u8);
                        if (word >> 8) != 0 {
                            out.push((word >> 8) as u8);
                        }
                        continue;
                    }
                }
                if (c as u32) <= 0x7F {
                    out.push(c as u8);
                } else {
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
            }
            out
        }
        _ => convert_charset(&unicode, CharsetId::Unicode, target).into_bytes(),
    }
}

pub fn convert_charset(text: &str, source: CharsetId, target: CharsetId) -> String {
    if text.is_empty() || source == target {
        return text.to_string();
    }

    // Step 1: Decode source text to standard Unicode Table index (0..TOTAL_VN_CHARS) or raw char
    let mut indices: Vec<Option<usize>> = Vec::with_capacity(text.len());
    let mut raw_chars: Vec<char> = Vec::with_capacity(text.len());

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match source {
            CharsetId::Unicode => {
                let idx = UNICODE_TABLE.iter().position(|&x| x == c);
                indices.push(idx);
                raw_chars.push(c);
                i += 1;
            }
            CharsetId::UnicodeComposite => {
                let base_idx = UNICODE_TABLE.iter().position(|&x| x == c);
                if let Some(b_idx) = base_idx {
                    if b_idx < TOTAL_VN_CHARS && IS_VN_VOWEL[b_idx] {
                        let root = STD_VN_ROOT_CHAR[b_idx] as usize;
                        let mut tone_val = 0;
                        if i + 1 < chars.len() {
                            tone_val = match chars[i + 1] {
                                '\u{0301}' => 1, // Sắc
                                '\u{0300}' => 2, // Huyền
                                '\u{0309}' => 3, // Hỏi
                                '\u{0303}' => 4, // Ngã
                                '\u{0323}' => 5, // Nặng
                                _ => 0,
                            };
                        }
                        if tone_val > 0 {
                            let final_idx = root + tone_val * 2;
                            indices.push(Some(final_idx));
                            raw_chars.push(UNICODE_TABLE[final_idx]);
                            i += 2;
                            continue;
                        }
                    }
                }
                indices.push(base_idx);
                raw_chars.push(c);
                i += 1;
            }
            CharsetId::Tcvn3 => {
                let b = c as u32;
                let idx = if b <= 255 {
                    TCVN3_TABLE.iter().position(|&x| x == b as u8).map(|idx| {
                        // TCVN3 shares one byte for upper/lower accented vowels.
                        // Prefer the lowercase slot (odd index) when duplicated.
                        if idx & 1 == 0
                            && idx + 1 < TOTAL_VN_CHARS
                            && TCVN3_TABLE[idx + 1] == TCVN3_TABLE[idx]
                        {
                            idx + 1
                        } else {
                            idx
                        }
                    })
                } else {
                    None
                };
                indices.push(idx);
                raw_chars.push(c);
                i += 1;
            }
            CharsetId::VniWindows => {
                // Check if pair with next char
                let mut matched = false;
                if i + 1 < chars.len() {
                    let c1 = chars[i + 1];
                    let pair = ((c1 as u32 & 0xFF) << 8) | (c as u32 & 0xFF);
                    if let Some(idx) = VNI_WIN_TABLE.iter().position(|&x| x as u32 == pair) {
                        indices.push(Some(idx));
                        raw_chars.push(UNICODE_TABLE[idx]);
                        i += 2;
                        matched = true;
                    }
                }
                if !matched {
                    let single = c as u32 & 0xFF;
                    let idx = VNI_WIN_TABLE.iter().position(|&x| {
                        (x as u32 & 0xFF) == single && (x >> 8) == 0 && single > 127
                    });
                    indices.push(idx);
                    raw_chars.push(c);
                    i += 1;
                }
            }
            _ => {
                let idx = UNICODE_TABLE.iter().position(|&x| x == c);
                indices.push(idx);
                raw_chars.push(c);
                i += 1;
            }
        }
    }

    // Step 2: Encode to target
    let mut result = String::with_capacity(text.len() * 2);
    for k in 0..indices.len() {
        if let Some(idx) = indices[k] {
            if idx < TOTAL_VN_CHARS {
                match target {
                    CharsetId::Unicode | CharsetId::Viqr => {
                        result.push(UNICODE_TABLE[idx]);
                    }
                    CharsetId::Tcvn3 => {
                        result.push(TCVN3_TABLE[idx] as char);
                    }
                    CharsetId::VniWindows => {
                        let w = VNI_WIN_TABLE[idx];
                        let c0 = (w & 0xFF) as u8 as char;
                        let c1 = (w >> 8) as u8;
                        result.push(c0);
                        if c1 != 0 {
                            result.push(c1 as char);
                        }
                    }
                    CharsetId::UnicodeComposite => {
                        let root = STD_VN_ROOT_CHAR[idx] as usize;
                        let tone_val = (idx - root) / 2;
                        let tone = match tone_val {
                            1 => VnLexiTone::Acute,
                            2 => VnLexiTone::Grave,
                            3 => VnLexiTone::Hook,
                            4 => VnLexiTone::Tilde,
                            5 => VnLexiTone::Dot,
                            _ => VnLexiTone::None,
                        };
                        let base_char = UNICODE_TABLE[root];
                        result.push(base_char);
                        let tone_mark = match tone {
                            VnLexiTone::None => '\0',
                            VnLexiTone::Acute => '\u{0301}',
                            VnLexiTone::Grave => '\u{0300}',
                            VnLexiTone::Hook => '\u{0309}',
                            VnLexiTone::Tilde => '\u{0303}',
                            VnLexiTone::Dot => '\u{0323}',
                        };
                        if tone_mark != '\0' {
                            result.push(tone_mark);
                        }
                    }
                }
                continue;
            }
        }
        result.push(raw_chars[k]);
    }

    result
}
