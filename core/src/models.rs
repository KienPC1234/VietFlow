#![allow(non_camel_case_types)]
// VietFlow Core - Ultra-High Performance Vietnamese Input Engine
// Licensed under GNU General Public License v3.0 (GPLv3)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InputMethod {
    Telex = 0,
    SimpleTelex = 1,
    Vni = 2,
    Viqr = 3,
    UserCustom = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CharsetId {
    Unicode = 0,
    UnicodeComposite = 1,
    Tcvn3 = 2,
    VniWindows = 3,
    Viqr = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VnLexiTone {
    None = 0,
    Acute = 1, // Sắc
    Grave = 2, // Huyền
    Hook = 3,  // Hỏi
    Tilde = 4, // Ngã
    Dot = 5,   // Nặng
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ToneKeyType {
    None = 0,
    Acute,
    Grave,
    Hook,
    Tilde,
    Dot,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModifyKeyType {
    None = 0,
    Circumflex, // Â, Ê, Ô
    Breve,      // Ă
    Horn,       // Ơ, Ư
    DMark,      // Đ
    HookAll,    // W key: ă, ơ, ư
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CharType {
    Letter,
    Digit,
    ToneOrAction,
    WordBreak,
    Punctuation,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WordForm {
    NonVn = 0,
    Empty = 1,
    C = 2,
    V = 3,
    CV = 4,
    VC = 5,
    CVC = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum VnLexiName {
    None = -2,
    NonVnChar = -1,

    // A and variations with tones 0..5 (A, á, à, ả, ã, ạ)
    A = 0, a = 1, A1 = 2, a1 = 3, A2 = 4, a2 = 5, A3 = 6, a3 = 7, A4 = 8, a4 = 9, A5 = 10, a5 = 11,

    // Â (roof / circumflex)
    Ar = 12, ar = 13, Ar1 = 14, ar1 = 15, Ar2 = 16, ar2 = 17, Ar3 = 18, ar3 = 19, Ar4 = 20, ar4 = 21, Ar5 = 22, ar5 = 23,

    // Ă (breve / bowl)
    Ah = 24, ah = 25, Ah1 = 26, ah1 = 27, Ah2 = 28, ah2 = 29, Ah3 = 30, ah3 = 31, Ah4 = 32, ah4 = 33, Ah5 = 34, ah5 = 35,

    // B, C, D, Đ
    B = 36, b = 37, C = 38, c = 39,
    D = 40, d = 41, DD = 42, dd = 43,

    // E
    E = 44, e = 45, E1 = 46, e1 = 47, E2 = 48, e2 = 49, E3 = 50, e3 = 51, E4 = 52, e4 = 53, E5 = 54, e5 = 55,

    // Ê (roof)
    Er = 56, er = 57, Er1 = 58, er1 = 59, Er2 = 60, er2 = 61, Er3 = 62, er3 = 63, Er4 = 64, er4 = 65, Er5 = 66, er5 = 67,

    // F, G, H
    F = 68, f = 69, G = 70, g = 71, H = 72, h = 73,

    // I
    I = 74, i = 75, I1 = 76, i1 = 77, I2 = 78, i2 = 79, I3 = 80, i3 = 81, I4 = 82, i4 = 83, I5 = 84, i5 = 85,

    // J, K, L, M, N
    J = 86, j = 87, K = 88, k = 89, L = 90, l = 91, M = 92, m = 93, N = 94, n = 95,

    // O
    O = 96, o = 97, O1 = 98, o1 = 99, O2 = 100, o2 = 101, O3 = 102, o3 = 103, O4 = 104, o4 = 105, O5 = 106, o5 = 107,

    // Ô (roof)
    Or = 108, or = 109, Or1 = 110, or1 = 111, Or2 = 112, or2 = 113, Or3 = 114, or3 = 115, Or4 = 116, or4 = 117, Or5 = 118, or5 = 119,

    // Ơ (horn / hook)
    Oh = 120, oh = 121, Oh1 = 122, oh1 = 123, Oh2 = 124, oh2 = 125, Oh3 = 126, oh3 = 127, Oh4 = 128, oh4 = 129, Oh5 = 130, oh5 = 131,

    // P, Q, R, S, T
    P = 132, p = 133, Q = 134, q = 135, R = 136, r = 137, S = 138, s = 139, T = 140, t = 141,

    // U
    U = 142, u = 143, U1 = 144, u1 = 145, U2 = 146, u2 = 147, U3 = 148, u3 = 149, U4 = 150, u4 = 151, U5 = 152, u5 = 153,

    // Ư (horn / hook)
    Uh = 154, uh = 155, Uh1 = 156, uh1 = 157, Uh2 = 158, uh2 = 159, Uh3 = 160, uh3 = 161, Uh4 = 162, uh4 = 163, Uh5 = 164, uh5 = 165,

    // V, W, X
    V = 166, v = 167, W = 168, w = 169, X = 170, x = 171,

    // Y
    Y = 172, y = 173, Y1 = 174, y1 = 175, Y2 = 176, y2 = 177, Y3 = 178, y3 = 179, Y4 = 180, y4 = 181, Y5 = 182, y5 = 183,

    // Z
    Z = 184, z = 185,
}

impl VnLexiName {
    #[inline(always)]
    pub fn to_lower(self) -> Self {
        let val = self as i16;
        if val < 0 {
            self
        } else if (val & 1) == 0 {
            unsafe { std::mem::transmute(val + 1) }
        } else {
            self
        }
    }

    #[inline(always)]
    pub fn to_upper(self) -> Self {
        let val = self as i16;
        if val < 0 {
            self
        } else if (val & 1) != 0 {
            unsafe { std::mem::transmute(val - 1) }
        } else {
            self
        }
    }

    #[inline(always)]
    pub fn is_vowel(self) -> bool {
        let lexi_lower = self.to_lower();
        matches!(
            lexi_lower,
            VnLexiName::a
                | VnLexiName::ar
                | VnLexiName::ah
                | VnLexiName::e
                | VnLexiName::er
                | VnLexiName::i
                | VnLexiName::o
                | VnLexiName::or
                | VnLexiName::oh
                | VnLexiName::u
                | VnLexiName::uh
                | VnLexiName::y
        )
    }
}

#[derive(Debug, Clone)]
pub struct EngineOptions {
    pub method: InputMethod,
    pub free_marking: bool,
    pub modern_style: bool,
    pub spell_check: bool,
    pub auto_non_vn_restore: bool,
    pub fix_shift_lag: bool,
    pub auto_capitalize: bool,
    pub consonant_shortcuts: bool,
    pub enable_macro: bool,
    pub allow_macro_when_disabled: bool,
    pub use_clipboard_for_unicode: bool,
    pub foreign_initials: Vec<char>,
    pub custom_english_keywords: Vec<String>,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            method: InputMethod::Telex,
            free_marking: true,
            modern_style: false,
            spell_check: true,
            auto_non_vn_restore: true,
            fix_shift_lag: true,
            auto_capitalize: false,
            consonant_shortcuts: false,
            enable_macro: false,
            allow_macro_when_disabled: false,
            use_clipboard_for_unicode: false,
            foreign_initials: vec!['f', 'j', 'w', 'z'],
            custom_english_keywords: Vec::new(),
        }
    }
}
