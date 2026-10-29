// VietFlow Core - Charset Converter & Accent Stripping Toolkit
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::charset::encode_slot;
use crate::models::{CharsetId, VnLexiName, VnLexiTone};
use crate::tables::{STD_VN_ROOT_CHAR, TCVN3_TABLE, TOTAL_VN_CHARS, UNICODE_TABLE, VNI_WIN_TABLE};

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
            CharsetId::Tcvn3 => {
                let b = c as u32;
                let idx = if b <= 255 {
                    TCVN3_TABLE.iter().position(|&x| x == b as u8)
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
                    let idx = VNI_WIN_TABLE.iter().position(|&x| (x as u32 & 0xFF) == single && (x >> 8) == 0 && single > 127);
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
                let lexi: VnLexiName = unsafe { std::mem::transmute(idx as i16) };
                let is_upper = (idx & 1) == 0;
                let enc = encode_slot(lexi, VnLexiTone::None, is_upper, raw_chars[k], target);
                for &ec in enc.as_slice() {
                    result.push(ec);
                }
                continue;
            }
        }
        result.push(raw_chars[k]);
    }

    result
}
