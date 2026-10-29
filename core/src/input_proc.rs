// VietFlow Core - Input Processor
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::models::{CharType, InputMethod, ModifyKeyType, ToneKeyType, VnLexiTone};

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
    c.is_alphabetic()
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

pub fn get_char_type(c: char, method: InputMethod) -> CharType {
    if c.is_ascii_alphabetic() {
        if matches!(method, InputMethod::Telex | InputMethod::SimpleTelex) {
            let lower = to_lower_ascii(c);
            if matches!(lower, 's' | 'f' | 'r' | 'x' | 'j' | 'z' | 'w') {
                return CharType::ToneOrAction;
            }
        }
        return CharType::Letter;
    }

    if c.is_ascii_digit() {
        if matches!(method, InputMethod::Vni) {
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
pub fn get_tone_key(c: char, method: InputMethod) -> ToneKeyType {
    let lower = to_lower_ascii(c);
    match method {
        InputMethod::Telex | InputMethod::SimpleTelex => match lower {
            's' => ToneKeyType::Acute,
            'f' => ToneKeyType::Grave,
            'r' => ToneKeyType::Hook,
            'x' => ToneKeyType::Tilde,
            'j' => ToneKeyType::Dot,
            'z' => ToneKeyType::Remove,
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
        InputMethod::Viqr => match c {
            '\'' => ToneKeyType::Acute,
            '`' => ToneKeyType::Grave,
            '?' => ToneKeyType::Hook,
            '~' => ToneKeyType::Tilde,
            '.' => ToneKeyType::Dot,
            '0' => ToneKeyType::Remove,
            _ => ToneKeyType::None,
        },
        _ => ToneKeyType::None,
    }
}

#[inline]
pub fn get_modify_key(c: char, method: InputMethod) -> ModifyKeyType {
    let lower = to_lower_ascii(c);
    match method {
        InputMethod::Telex | InputMethod::SimpleTelex => match lower {
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
        InputMethod::Viqr => match c {
            '^' => ModifyKeyType::Circumflex,
            '+' | '*' => ModifyKeyType::Horn,
            '(' => ModifyKeyType::Breve,
            _ => ModifyKeyType::None,
        },
        _ => ModifyKeyType::None,
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
