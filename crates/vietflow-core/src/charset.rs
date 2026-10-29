// VietFlow Core - Charset Encoders (Unicode NFC, Composite NFD, TCVN3, VNI-Windows)
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::models::{CharsetId, VnLexiName, VnLexiTone};
use crate::tables::{IS_VN_VOWEL, TOTAL_VN_CHARS, TCVN3_TABLE, UNICODE_TABLE, VNI_WIN_TABLE};

#[derive(Debug, Clone, Copy)]
pub struct EncodedChar {
    pub chars: [char; 2],
    pub len: usize,
}

impl EncodedChar {
    #[inline(always)]
    pub const fn single(c: char) -> Self {
        Self {
            chars: [c, '\0'],
            len: 1,
        }
    }

    #[inline(always)]
    pub const fn double(c0: char, c1: char) -> Self {
        Self {
            chars: [c0, c1],
            len: 2,
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

fn encode_composite(
    lexi: VnLexiName,
    tone: VnLexiTone,
    is_upper: bool,
    raw_char: char,
) -> EncodedChar {
    let base_idx = match to_table_index(lexi, VnLexiTone::None, is_upper) {
        Some(i) => i,
        None => return EncodedChar::single(raw_char),
    };

    let base_char = UNICODE_TABLE[base_idx];
    let tone_mark = match tone {
        VnLexiTone::None => '\0',
        VnLexiTone::Acute => '\u{0301}', // Sắc
        VnLexiTone::Grave => '\u{0300}', // Huyền
        VnLexiTone::Hook => '\u{0309}',  // Hỏi
        VnLexiTone::Tilde => '\u{0303}', // Ngã
        VnLexiTone::Dot => '\u{0323}',   // Nặng
    };

    if tone_mark != '\0' {
        EncodedChar::double(base_char, tone_mark)
    } else {
        EncodedChar::single(base_char)
    }
}
