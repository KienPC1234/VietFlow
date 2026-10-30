// VietFlow Core - Input Processor
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::models::{CharType, CustomKeymap, InputMethod, ModifyKeyType, ToneKeyType, VnLexiTone};

#[inline(always)]
pub fn to_lower_ascii(c: char) -> char {
    if c.is_ascii_uppercase() {
        ((c as u8) + 32) as char
    } else {
        c
    }
}

#[inline(always)]
pub fn is_ascii_upper(c: char) -> bool {
    c.is_ascii_uppercase()
}

#[inline(always)]
pub fn is_letter_fast(c: char) -> bool {
    c.is_ascii_alphabetic() || c.is_alphabetic()
}

#[inline(always)]
pub fn is_word_break_fast(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\r'
            | '\n'
            | '.'
            | ','
            | ';'
            | ':'
            | '!'
            | '?'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '"'
            | '\''
            | '<'
            | '>'
            | '='
            | '+'
            | '-'
            | '*'
            | '/'
            | '\\'
            | '|'
            | '@'
            | '#'
            | '$'
            | '%'
            | '^'
            | '&'
            | '~'
            | '`'
    )
}

#[inline]
fn key_matches(c: char, mapping: &str) -> bool {
    if mapping.is_empty() {
        return false;
    }
    let lower = to_lower_ascii(c);
    mapping.chars().any(|m| to_lower_ascii(m) == lower)
}

pub fn get_char_type(c: char, method: InputMethod, custom: &CustomKeymap) -> CharType {
    if method == InputMethod::UserCustom {
        if custom.is_action_key(c) {
            return CharType::ToneOrAction;
        }
        if c.is_ascii_alphabetic() {
            return CharType::Letter;
        }
        if c.is_ascii_digit() {
            return CharType::Digit;
        }
        if c == ' ' || c == '\t' {
            return CharType::WordBreak;
        }
        if c == '\r' || c == '\n' {
            return CharType::Reset;
        }
        if is_word_break_fast(c) {
            return CharType::WordBreak;
        }
        return CharType::Reset;
    }

    if c.is_ascii_alphabetic() {
        if matches!(
            method,
            InputMethod::Telex
                | InputMethod::SimpleTelex
                | InputMethod::TelexSimple2
                | InputMethod::TelexVni
        ) {
            let lower = to_lower_ascii(c);
            let is_action = match method {
                InputMethod::TelexSimple2 => matches!(lower, 's' | 'f' | 'r' | 'x' | 'j' | 'w'),
                _ => matches!(lower, 's' | 'f' | 'r' | 'x' | 'j' | 'z' | 'w'),
            };
            if is_action {
                return CharType::ToneOrAction;
            }
        }
        return CharType::Letter;
    }

    if c.is_ascii_digit() {
        if matches!(method, InputMethod::Vni | InputMethod::TelexVni) {
            return CharType::ToneOrAction;
        }
        return CharType::Digit;
    }

    if matches!(method, InputMethod::Viqr)
        && matches!(c, '\'' | '`' | '?' | '~' | '.' | '0' | '^' | '+' | '*' | '(')
    {
        return CharType::ToneOrAction;
    }

    if c == ' ' || c == '\t' {
        return CharType::WordBreak;
    }
    if c == '\r' || c == '\n' {
        return CharType::Reset;
    }

    if is_word_break_fast(c) {
        return CharType::WordBreak;
    }

    CharType::Reset
}

#[inline]
pub fn get_tone_key(c: char, method: InputMethod, custom: &CustomKeymap) -> ToneKeyType {
    let lower = to_lower_ascii(c);
    match method {
        InputMethod::UserCustom => {
            if key_matches(c, &custom.acute) {
                ToneKeyType::Acute
            } else if key_matches(c, &custom.grave) {
                ToneKeyType::Grave
            } else if key_matches(c, &custom.hook) {
                ToneKeyType::Hook
            } else if key_matches(c, &custom.tilde) {
                ToneKeyType::Tilde
            } else if key_matches(c, &custom.dot) {
                ToneKeyType::Dot
            } else if key_matches(c, &custom.remove_tone) {
                ToneKeyType::Remove
            } else {
                ToneKeyType::None
            }
        }
        InputMethod::Telex | InputMethod::SimpleTelex => match lower {
            's' => ToneKeyType::Acute,
            'f' => ToneKeyType::Grave,
            'r' => ToneKeyType::Hook,
            'x' => ToneKeyType::Tilde,
            'j' => ToneKeyType::Dot,
            'z' => ToneKeyType::Remove,
            _ => ToneKeyType::None,
        },
        InputMethod::TelexSimple2 => match lower {
            's' => ToneKeyType::Acute,
            'f' => ToneKeyType::Grave,
            'r' => ToneKeyType::Hook,
            'x' => ToneKeyType::Tilde,
            'j' => ToneKeyType::Dot,
            _ => ToneKeyType::None,
        },
        InputMethod::Vni => match c {
            '1' => ToneKeyType::Acute,
            '2' => ToneKeyType::Grave,
            '3' => ToneKeyType::Hook,
            '4' => ToneKeyType::Tilde,
            '5' => ToneKeyType::Dot,
            '0' => ToneKeyType::Remove,
            _ => ToneKeyType::None,
        },
        InputMethod::TelexVni => match lower {
            's' => ToneKeyType::Acute,
            'f' => ToneKeyType::Grave,
            'r' => ToneKeyType::Hook,
            'x' => ToneKeyType::Tilde,
            'j' => ToneKeyType::Dot,
            'z' => ToneKeyType::Remove,
            _ => match c {
                '1' => ToneKeyType::Acute,
                '2' => ToneKeyType::Grave,
                '3' => ToneKeyType::Hook,
                '4' => ToneKeyType::Tilde,
                '5' => ToneKeyType::Dot,
                '0' => ToneKeyType::Remove,
                _ => ToneKeyType::None,
            },
        },
        InputMethod::Viqr => match c {
            '\'' => ToneKeyType::Acute,
            '`' => ToneKeyType::Grave,
            '?' => ToneKeyType::Hook,
            '~' => ToneKeyType::Tilde,
            '.' => ToneKeyType::Dot,
            '0' => ToneKeyType::Remove,
            _ => ToneKeyType::None,
        },
    }
}

#[inline]
pub fn get_modify_key(c: char, method: InputMethod, custom: &CustomKeymap) -> ModifyKeyType {
    let lower = to_lower_ascii(c);
    match method {
        InputMethod::UserCustom => {
            if key_matches(c, &custom.hook_all) {
                ModifyKeyType::HookAll
            } else if key_matches(c, &custom.circumflex) {
                ModifyKeyType::Circumflex
            } else if key_matches(c, &custom.horn) {
                ModifyKeyType::Horn
            } else if key_matches(c, &custom.breve) {
                ModifyKeyType::Breve
            } else if key_matches(c, &custom.d_mark) {
                ModifyKeyType::DMark
            } else if key_matches(c, &custom.roof_a) {
                ModifyKeyType::RoofA
            } else if key_matches(c, &custom.roof_e) {
                ModifyKeyType::RoofE
            } else if key_matches(c, &custom.roof_o) {
                ModifyKeyType::RoofO
            } else if key_matches(c, &custom.horn_o) {
                ModifyKeyType::HornO
            } else if key_matches(c, &custom.horn_u) {
                ModifyKeyType::HornU
            } else if key_matches(c, &custom.breve_a) {
                ModifyKeyType::BreveA
            } else if key_matches(c, &custom.d_lower) {
                ModifyKeyType::DLower
            } else {
                ModifyKeyType::None
            }
        }
        InputMethod::Telex | InputMethod::SimpleTelex | InputMethod::TelexSimple2 => match lower {
            'w' => ModifyKeyType::HookAll,
            _ => ModifyKeyType::None,
        },
        InputMethod::Vni => match c {
            '6' => ModifyKeyType::Circumflex,
            '7' => ModifyKeyType::Horn,
            '8' => ModifyKeyType::Breve,
            '9' => ModifyKeyType::DMark,
            _ => ModifyKeyType::None,
        },
        InputMethod::TelexVni => {
            if lower == 'w' {
                ModifyKeyType::HookAll
            } else {
                match c {
                    '6' => ModifyKeyType::Circumflex,
                    '7' => ModifyKeyType::Horn,
                    '8' => ModifyKeyType::Breve,
                    '9' => ModifyKeyType::DMark,
                    _ => ModifyKeyType::None,
                }
            }
        }
        InputMethod::Viqr => match c {
            '^' => ModifyKeyType::Circumflex,
            '+' | '*' => ModifyKeyType::Horn,
            '(' => ModifyKeyType::Breve,
            _ => ModifyKeyType::None,
        },
    }
}

#[inline(always)]
pub fn to_vn_lexi_tone(tone: ToneKeyType) -> VnLexiTone {
    match tone {
        ToneKeyType::Acute => VnLexiTone::Acute,
        ToneKeyType::Grave => VnLexiTone::Grave,
        ToneKeyType::Hook => VnLexiTone::Hook,
        ToneKeyType::Tilde => VnLexiTone::Tilde,
        ToneKeyType::Dot => VnLexiTone::Dot,
        ToneKeyType::Remove | ToneKeyType::None => VnLexiTone::None,
    }
}
