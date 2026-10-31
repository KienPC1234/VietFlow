// VietFlow Core - Charset Converter & Accent Stripping Toolkit
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::charset::encode_composite_index;
use crate::models::CharsetId;
use crate::tables::{
    IS_VN_VOWEL, STD_VN_ROOT_CHAR, TCVN3_TABLE, TOTAL_VN_CHARS, UNICODE_TABLE, VNI_WIN_TABLE,
};

const fn build_tcvn3_inverse() -> [u8; 256] {
    let mut table = [0xFF; 256];
    let mut i = 0;
    while i < TOTAL_VN_CHARS {
        let b = TCVN3_TABLE[i];
        if table[b as usize] == 0xFF || (i & 1 != 0) {
            table[b as usize] = i as u8;
        }
        i += 1;
    }
    table
}

const fn build_ascii_to_unicode_index() -> [u8; 128] {
    let mut table = [0xFF; 128];
    let mut i = 0;
    while i < TOTAL_VN_CHARS {
        let ch = UNICODE_TABLE[i];
        if (ch as u32) < 128 {
            table[ch as usize] = i as u8;
        }
        i += 1;
    }
    table
}

const fn build_unicode_sorted() -> [(char, u8); TOTAL_VN_CHARS] {
    let mut arr = [('\0', 0u8); TOTAL_VN_CHARS];
    let mut i = 0;
    while i < TOTAL_VN_CHARS {
        arr[i] = (UNICODE_TABLE[i], i as u8);
        i += 1;
    }
    let mut i = 1;
    while i < TOTAL_VN_CHARS {
        let key = arr[i];
        let mut j = i;
        while j > 0 && (arr[j - 1].0 as u32) > (key.0 as u32) {
            arr[j] = arr[j - 1];
            j -= 1;
        }
        arr[j] = key;
        i += 1;
    }
    arr
}

static INVERSE_TCVN3: [u8; 256] = build_tcvn3_inverse();
static ASCII_TO_UNICODE_INDEX: [u8; 128] = build_ascii_to_unicode_index();
static UNICODE_SORTED: [(char, u8); TOTAL_VN_CHARS] = build_unicode_sorted();

const fn build_vni_single_inverse() -> [u8; 256] {
    let mut table = [0xFF; 256];
    let mut i = 0;
    while i < TOTAL_VN_CHARS {
        let word = VNI_WIN_TABLE[i];
        if (word >> 8) == 0 && (word & 0xFF) > 127 {
            let b = (word & 0xFF) as usize;
            if table[b] == 0xFF {
                table[b] = i as u8;
            }
        }
        i += 1;
    }
    table
}

const fn count_vni_pairs() -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < TOTAL_VN_CHARS {
        if (VNI_WIN_TABLE[i] >> 8) != 0 {
            count += 1;
        }
        i += 1;
    }
    count
}

const VNI_PAIR_COUNT: usize = count_vni_pairs();

const fn build_vni_pair_sorted() -> [(u16, u8); VNI_PAIR_COUNT] {
    let mut arr = [(0u16, 0u8); VNI_PAIR_COUNT];
    let mut i = 0;
    let mut idx = 0;
    while i < TOTAL_VN_CHARS {
        let word = VNI_WIN_TABLE[i];
        if (word >> 8) != 0 {
            arr[idx] = (word, i as u8);
            idx += 1;
        }
        i += 1;
    }
    let mut i = 1;
    while i < VNI_PAIR_COUNT {
        let key = arr[i];
        let mut j = i;
        while j > 0 && arr[j - 1].0 > key.0 {
            arr[j] = arr[j - 1];
            j -= 1;
        }
        arr[j] = key;
        i += 1;
    }
    arr
}

static INVERSE_VNI_SINGLE: [u8; 256] = build_vni_single_inverse();
static VNI_PAIRS_SORTED: [(u16, u8); VNI_PAIR_COUNT] = build_vni_pair_sorted();

#[inline]
pub fn unicode_to_table_index(c: char) -> Option<usize> {
    if c.is_ascii() {
        let idx = ASCII_TO_UNICODE_INDEX[c as usize];
        if idx != 0xFF {
            return Some(idx as usize);
        }
        return None;
    }
    UNICODE_SORTED
        .binary_search_by_key(&(c as u32), |&(ch, _)| ch as u32)
        .ok()
        .map(|pos| UNICODE_SORTED[pos].1 as usize)
}

#[inline]
pub fn strip_accent_char(c: char) -> char {
    if c.is_ascii() {
        return c;
    }
    match c {
        'Á' | 'À' | 'Ả' | 'Ã' | 'Ạ' | 'Â' | 'Ấ' | 'Ầ' | 'Ẩ' | 'Ẫ' | 'Ậ' | 'Ă' | 'Ắ' | 'Ằ' | 'Ẳ' | 'Ẵ' | 'Ặ' => 'A',
        'á' | 'à' | 'ả' | 'ã' | 'ạ' | 'â' | 'ấ' | 'ầ' | 'ẩ' | 'ẫ' | 'ậ' | 'ă' | 'ắ' | 'ằ' | 'ẳ' | 'ẵ' | 'ặ' => 'a',
        'É' | 'È' | 'Ẻ' | 'Ẽ' | 'Ẹ' | 'Ê' | 'Ế' | 'Ề' | 'Ể' | 'Ễ' | 'Ệ' => 'E',
        'é' | 'è' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ế' | 'ề' | 'ể' | 'ễ' | 'ệ' => 'e',
        'Í' | 'Ì' | 'Ỉ' | 'Ĩ' | 'Ị' => 'I',
        'í' | 'ì' | 'ỉ' | 'ĩ' | 'ị' => 'i',
        'Ó' | 'Ò' | 'Ỏ' | 'Õ' | 'Ọ' | 'Ô' | 'Ố' | 'Ồ' | 'Ổ' | 'Ỗ' | 'Ộ' | 'Ơ' | 'Ớ' | 'Ờ' | 'Ở' | 'Ỡ' | 'Ợ' => 'O',
        'ó' | 'ò' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ố' | 'ồ' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ớ' | 'ờ' | 'ở' | 'ỡ' | 'ợ' => 'o',
        'Ú' | 'Ù' | 'Ủ' | 'Ũ' | 'Ụ' | 'Ư' | 'Ứ' | 'Ừ' | 'Ử' | 'Ữ' | 'Ự' => 'U',
        'ú' | 'ù' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ứ' | 'ừ' | 'ử' | 'ữ' | 'ự' => 'u',
        'Ý' | 'Ỳ' | 'Ỷ' | 'Ỹ' | 'Ỵ' => 'Y',
        'ý' | 'ỳ' | 'ỷ' | 'ỹ' | 'ỵ' => 'y',
        'Đ' => 'D',
        'đ' => 'd',
        other => other,
    }
}

/// Combining marks used by Vietnamese NFD text. They carry no base letter of
/// their own, so stripping accents must remove them instead of copying them.
fn is_vietnamese_combining_mark(c: char) -> bool {
    matches!(
        c,
        '\u{0300}' // grave
            | '\u{0301}' // acute
            | '\u{0302}' // circumflex
            | '\u{0303}' // tilde
            | '\u{0306}' // breve
            | '\u{0309}' // hook above
            | '\u{031B}' // horn
            | '\u{0323}' // dot below
            | '\u{0340}' // canonical grave
            | '\u{0341}' // canonical acute
    )
}

pub fn strip_accents(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        if is_vietnamese_combining_mark(c) {
            continue;
        }
        result.push(strip_accent_char(c));
    }
    result
}

/// Combine a base character plus Vietnamese combining marks into one table
/// index. This accepts both ordering styles produced by NFD text:
/// `base + tone` and `base + modifier + tone`.
fn compose_composite_index(base: char, marks: &[char]) -> Option<usize> {
    let base_index = unicode_to_table_index(base)?;
    if base_index >= TOTAL_VN_CHARS {
        return None;
    }

    let root = STD_VN_ROOT_CHAR[base_index] as usize;
    let upper = (root & 1) == 0;
    let base_letter = UNICODE_TABLE[root].to_ascii_lowercase();
    let mut root_index = root;
    let mut modifier = 0u8;
    let mut tone = 0u8;

    for &mark in marks {
        match mark {
            '\u{0302}' => {
                if modifier != 0 {
                    return None;
                }
                modifier = 1; // circumflex
            }
            '\u{0306}' => {
                if modifier != 0 {
                    return None;
                }
                modifier = 2; // breve
            }
            '\u{031B}' => {
                if modifier != 0 {
                    return None;
                }
                modifier = 3; // horn
            }
            '\u{0301}' | '\u{0341}' => tone = 1,
            '\u{0300}' | '\u{0340}' => tone = 2,
            '\u{0309}' => tone = 3,
            '\u{0303}' => tone = 4,
            '\u{0323}' => tone = 5,
            _ => return None,
        }
    }

    if modifier != 0 {
        root_index = match (base_letter, modifier) {
            ('a', 1) => 12, // â
            ('a', 2) => 24, // ă
            ('e', 1) => 56, // ê
            ('o', 1) => 108, // ô
            ('o', 3) => 120, // ơ
            ('u', 3) => 154, // ư
            _ => return None,
        };
        if !upper {
            root_index += 1;
        }
    }

    if tone > 0 && !IS_VN_VOWEL[root_index] {
        return None;
    }
    Some(root_index + tone as usize * 2)
}

/// Decode raw bytes in a legacy charset into Unicode text. This is the inverse
/// of the byte-oriented encoder below and is used by the clipboard converter,
/// where TCVN3/VNI content is not valid UTF-8.
fn decode_source_bytes(input: &[u8], source: CharsetId) -> String {
    match source {
        CharsetId::Tcvn3 => {
            let mut out = String::with_capacity(input.len());
            for &byte in input {
                let idx = INVERSE_TCVN3[byte as usize];
                if idx != 0xFF {
                    out.push(UNICODE_TABLE[idx as usize]);
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
                    if let Ok(pos) = VNI_PAIRS_SORTED.binary_search_by_key(&pair, |&(p, _)| p) {
                        out.push(UNICODE_TABLE[VNI_PAIRS_SORTED[pos].1 as usize]);
                        i += 2;
                        continue;
                    }
                }
                let single_idx = INVERSE_VNI_SINGLE[byte as usize];
                if single_idx != 0xFF {
                    out.push(UNICODE_TABLE[single_idx as usize]);
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
                if let Some(idx) = unicode_to_table_index(c) {
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
                if let Some(idx) = unicode_to_table_index(c) {
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
                let idx = unicode_to_table_index(c);
                indices.push(idx);
                raw_chars.push(c);
                i += 1;
            }
            CharsetId::UnicodeComposite => {
                let mut mark_end = i + 1;
                while mark_end < chars.len()
                    && is_vietnamese_combining_mark(chars[mark_end])
                {
                    mark_end += 1;
                }

                if mark_end > i + 1 {
                    if let Some(final_idx) = compose_composite_index(c, &chars[i + 1..mark_end])
                    {
                        indices.push(Some(final_idx));
                        raw_chars.push(UNICODE_TABLE[final_idx]);
                        i = mark_end;
                        continue;
                    }
                }

                let base_idx = unicode_to_table_index(c);
                indices.push(base_idx);
                raw_chars.push(c);
                i += 1;
            }
            CharsetId::Tcvn3 => {
                let b = c as u32;
                let idx = if b <= 255 {
                    let val = INVERSE_TCVN3[b as usize];
                    if val != 0xFF {
                        Some(val as usize)
                    } else {
                        None
                    }
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
                let idx = unicode_to_table_index(c);
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
                        let encoded = encode_composite_index(idx);
                        for &c in encoded.as_slice() {
                            result.push(c);
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
