// VietFlow Core - Charset Encoders (Unicode NFC, Composite NFD, TCVN3, VNI-Windows)
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::models::{CharsetId, VnLexiName, VnLexiTone};
use crate::tables::{
    IS_VN_VOWEL, STD_VN_ROOT_CHAR, TCVN3_TABLE, TOTAL_VN_CHARS, UNICODE_TABLE, VNI_WIN_TABLE,
};

#[derive(Debug, Clone, Copy)]
pub struct EncodedChar {
    pub chars: [char; 4],
    pub len: usize,
}

impl EncodedChar {
    #[inline(always)]
    pub const fn single(c: char) -> Self {
        Self {
            chars: [c, '\0', '\0', '\0'],
            len: 1,
        }
    }

    #[inline(always)]
    pub const fn double(c0: char, c1: char) -> Self {
        Self {
            chars: [c0, c1, '\0', '\0'],
            len: 2,
        }
    }

    #[inline(always)]
    pub const fn triple(c0: char, c1: char, c2: char) -> Self {
        Self {
            chars: [c0, c1, c2, '\0'],
            len: 3,
        }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[char] {
        &self.chars[..self.len]
    }
}

#[inline]
pub fn to_table_index(lexi: VnLexiName, tone: VnLexiTone, is_upper: bool) -> Option<usize> {
    if matches!(lexi, VnLexiName::NonVnChar | VnLexiName::None) {
        return None;
    }

    let mut base_val = lexi as i16;
    if base_val < 0 || base_val >= TOTAL_VN_CHARS as i16 {
        return None;
    }

    if is_upper && (base_val & 1) != 0 {
        base_val -= 1;
    } else if !is_upper && (base_val & 1) == 0 {
        base_val += 1;
    }

    let mut idx = base_val as usize;
    if idx < TOTAL_VN_CHARS && IS_VN_VOWEL[idx] {
        idx += (tone as usize) * 2;
    }

    if idx < TOTAL_VN_CHARS {
        Some(idx)
    } else {
        None
    }
}

pub fn encode_slot(
    lexi: VnLexiName,
    tone: VnLexiTone,
    is_upper: bool,
    raw_char: char,
    charset: CharsetId,
) -> EncodedChar {
    let idx_opt = to_table_index(lexi, tone, is_upper);
    let idx = match idx_opt {
        Some(i) => i,
        None => return EncodedChar::single(raw_char),
    };

    match charset {
        CharsetId::Unicode => EncodedChar::single(UNICODE_TABLE[idx]),
        CharsetId::Tcvn3 => EncodedChar::single(TCVN3_TABLE[idx] as char),
        CharsetId::VniWindows => {
            let w = VNI_WIN_TABLE[idx];
            let c0 = (w & 0xFF) as u8 as char;
            let c1 = (w >> 8) as u8;
            if c1 != 0 {
                EncodedChar::double(c0, c1 as char)
            } else {
                EncodedChar::single(c0)
            }
        }
        CharsetId::UnicodeComposite => encode_composite(lexi, tone, is_upper, raw_char),
        CharsetId::Viqr => EncodedChar::single(UNICODE_TABLE[idx]),
    }
}

/// Encode one standard Vietnamese table entry as canonical NFD.
pub fn encode_composite_index(idx: usize) -> EncodedChar {
    let root = STD_VN_ROOT_CHAR[idx] as usize;
    let tone_val = (idx - root) / 2;

    // True NFD: decompose the base diacritic (circumflex/breve/horn) as well
    // as the tone mark. The old implementation emitted a precomposed base
    // such as "ê" + U+0301, which is not canonical NFD.
    let (plain_base, modifier) = match root {
        12 | 13 => (if root & 1 == 0 { 0 } else { 1 }, '\u{0302}'), // â
        24 | 25 => (if root & 1 == 0 { 0 } else { 1 }, '\u{0306}'), // ă
        56 | 57 => (if root & 1 == 0 { 44 } else { 45 }, '\u{0302}'), // ê
        108 | 109 => (if root & 1 == 0 { 96 } else { 97 }, '\u{0302}'), // ô
        120 | 121 => (if root & 1 == 0 { 96 } else { 97 }, '\u{031B}'), // ơ
        154 | 155 => (if root & 1 == 0 { 142 } else { 143 }, '\u{031B}'), // ư
        _ => (root, '\0'),
    };

    let tone_mark = match tone_val {
        1 => '\u{0301}', // Sắc
        2 => '\u{0300}', // Huyền
        3 => '\u{0309}', // Hỏi
        4 => '\u{0303}', // Ngã
        5 => '\u{0323}', // Nặng
        _ => '\0',
    };

    let base_char = UNICODE_TABLE[plain_base];
    match (modifier != '\0', tone_mark != '\0') {
        (true, true) => EncodedChar::triple(base_char, modifier, tone_mark),
        (true, false) => EncodedChar::double(base_char, modifier),
        (false, true) => EncodedChar::double(base_char, tone_mark),
        (false, false) => EncodedChar::single(base_char),
    }
}

fn encode_composite(
    lexi: VnLexiName,
    tone: VnLexiTone,
    is_upper: bool,
    raw_char: char,
) -> EncodedChar {
    let idx = match to_table_index(lexi, tone, is_upper) {
        Some(i) => i,
        None => return EncodedChar::single(raw_char),
    };
    encode_composite_index(idx)
}
