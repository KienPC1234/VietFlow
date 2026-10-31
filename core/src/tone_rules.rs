// VietFlow Core - Tone Placement & Syllable Rules
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::models::VnLexiName;
use crate::tables::TOTAL_VN_CHARS;

#[derive(Debug, Clone, Copy)]
pub struct VowelSeq {
    pub v: [VnLexiName; 3],
    pub len: usize,
    pub classic_tone_idx: usize,
    pub modern_tone_idx: usize,
}

#[inline(always)]
pub fn normalize_vowel(raw_vowel: VnLexiName) -> VnLexiName {
    match raw_vowel {
        VnLexiName::A | VnLexiName::a => VnLexiName::A,
        VnLexiName::Ar | VnLexiName::ar => VnLexiName::Ar,
        VnLexiName::Ah | VnLexiName::ah => VnLexiName::Ah,
        VnLexiName::E | VnLexiName::e => VnLexiName::E,
        VnLexiName::Er | VnLexiName::er => VnLexiName::Er,
        VnLexiName::I | VnLexiName::i => VnLexiName::I,
        VnLexiName::O | VnLexiName::o => VnLexiName::O,
        VnLexiName::Or | VnLexiName::or => VnLexiName::Or,
        VnLexiName::Oh | VnLexiName::oh => VnLexiName::Oh,
        VnLexiName::U | VnLexiName::u => VnLexiName::U,
        VnLexiName::Uh | VnLexiName::uh => VnLexiName::Uh,
        VnLexiName::Y | VnLexiName::y => VnLexiName::Y,
        _ => {
            let val = raw_vowel as i16;
            if (0..TOTAL_VN_CHARS as i16).contains(&val) {
                let mut root = crate::tables::STD_VN_ROOT_CHAR[val as usize];
                if (root & 1) != 0 {
                    root -= 1;
                }
                match root {
                    0 => VnLexiName::A,
                    12 => VnLexiName::Ar,
                    24 => VnLexiName::Ah,
                    44 => VnLexiName::E,
                    56 => VnLexiName::Er,
                    74 => VnLexiName::I,
                    96 => VnLexiName::O,
                    108 => VnLexiName::Or,
                    120 => VnLexiName::Oh,
                    142 => VnLexiName::U,
                    154 => VnLexiName::Uh,
                    172 => VnLexiName::Y,
                    _ => raw_vowel,
                }
            } else {
                raw_vowel
            }
        }
    }
}

macro_rules! vseq {
    ($c:expr, $m:expr, $v0:ident) => {
        VowelSeq {
            v: [VnLexiName::$v0, VnLexiName::None, VnLexiName::None],
            len: 1,
            classic_tone_idx: $c,
            modern_tone_idx: $m,
        }
    };
    ($c:expr, $m:expr, $v0:ident, $v1:ident) => {
        VowelSeq {
            v: [VnLexiName::$v0, VnLexiName::$v1, VnLexiName::None],
            len: 2,
            classic_tone_idx: $c,
            modern_tone_idx: $m,
        }
    };
    ($c:expr, $m:expr, $v0:ident, $v1:ident, $v2:ident) => {
        VowelSeq {
            v: [VnLexiName::$v0, VnLexiName::$v1, VnLexiName::$v2],
            len: 3,
            classic_tone_idx: $c,
            modern_tone_idx: $m,
        }
    };
}

pub static VOWEL_SEQS: &[VowelSeq] = &[
    // Single
    vseq!(0, 0, A),
    vseq!(0, 0, Ar),
    vseq!(0, 0, Ah),
    vseq!(0, 0, E),
    vseq!(0, 0, Er),
    vseq!(0, 0, I),
    vseq!(0, 0, O),
    vseq!(0, 0, Or),
    vseq!(0, 0, Oh),
    vseq!(0, 0, U),
    vseq!(0, 0, Uh),
    vseq!(0, 0, Y),
    // Double
    vseq!(0, 0, A, I),
    vseq!(0, 0, A, O),
    vseq!(0, 0, A, U),
    vseq!(0, 0, A, Y),
    vseq!(0, 0, E, O),
    vseq!(0, 0, Ar, U),
    vseq!(0, 0, Ar, Y),
    vseq!(0, 0, Ah, U),
    vseq!(0, 0, Ah, Y),
    vseq!(0, 0, E, U),
    vseq!(0, 0, Er, U),
    vseq!(0, 0, Er, I),
    vseq!(0, 0, I, A),
    vseq!(1, 1, I, E),
    vseq!(1, 1, I, Er),
    vseq!(0, 0, I, U),
    vseq!(0, 1, O, A),
    vseq!(0, 1, O, E),
    vseq!(0, 0, O, I),
    vseq!(0, 0, Or, I),
    vseq!(0, 0, Oh, I),
    vseq!(0, 0, Oh, U),
    vseq!(0, 0, U, A),
    vseq!(0, 1, U, E),
    vseq!(1, 1, U, Er),
    vseq!(0, 0, U, I),
    vseq!(0, 1, U, Y),
    vseq!(0, 1, U, O),
    vseq!(1, 1, U, Or),
    vseq!(1, 1, Uh, Oh),
    vseq!(0, 0, Uh, A),
    vseq!(0, 0, Uh, I),
    vseq!(0, 0, Uh, U),
    vseq!(1, 1, U, Ar),
    vseq!(1, 1, O, Ah),
    vseq!(0, 0, Uh, O),
    vseq!(1, 1, U, Oh),
    vseq!(0, 1, U, U),
    vseq!(1, 1, Y, E),
    vseq!(1, 1, Y, Er),
    // Triple
    vseq!(1, 1, I, E, U),
    vseq!(1, 1, I, Er, U),
    vseq!(1, 1, O, A, I),
    vseq!(1, 1, O, A, Y),
    vseq!(1, 1, O, Ar, Y),
    vseq!(1, 1, O, A, O),
    vseq!(1, 1, O, E, O),
    vseq!(1, 1, U, A, O),
    vseq!(1, 1, U, A, Y),
    vseq!(1, 1, U, Ar, Y),
    vseq!(1, 1, U, E, U),
    vseq!(1, 1, U, Er, U),
    vseq!(1, 1, U, I, A),
    vseq!(1, 1, U, O, I),
    vseq!(1, 1, U, Or, I),
    vseq!(1, 1, Uh, Oh, I),
    vseq!(1, 1, U, O, U),
    vseq!(1, 1, U, Or, U),
    vseq!(1, 1, Uh, Oh, U),
    vseq!(1, 1, U, Oh, I),
    vseq!(1, 1, U, Oh, U),
    vseq!(0, 0, Uh, O, I),
    vseq!(0, 0, Uh, O, U),
    vseq!(1, 1, U, Y, A),
    vseq!(2, 2, U, Y, E),
    vseq!(2, 2, U, Y, Er),
    vseq!(1, 1, U, Y, U),
    vseq!(1, 1, Y, E, U),
    vseq!(1, 1, Y, Er, U),
];

#[inline]
pub fn find_vowel_seq(vowels: &[VnLexiName]) -> Option<&'static VowelSeq> {
    let (slice, len) = match vowels.len() {
        1 => (&VOWEL_SEQS[0..12], 1),
        2 => (&VOWEL_SEQS[12..52], 2),
        3 => (&VOWEL_SEQS[52..], 3),
        _ => return None,
    };

    let norm = [
        normalize_vowel(vowels[0]),
        if len > 1 { normalize_vowel(vowels[1]) } else { VnLexiName::None },
        if len > 2 { normalize_vowel(vowels[2]) } else { VnLexiName::None },
    ];

    slice.iter().find(|s| s.v[..len] == norm[..len])
}

pub fn get_tone_position(vowels: &[VnLexiName], has_coda: bool, modern_style: bool) -> usize {
    if vowels.len() <= 1 {
        return 0;
    }

    if let Some(seq) = find_vowel_seq(vowels) {
        if has_coda {
            return seq.modern_tone_idx;
        }
        return if modern_style {
            seq.modern_tone_idx
        } else {
            seq.classic_tone_idx
        };
    }

    if vowels.len() > 1 { 1 } else { 0 }
}

pub fn is_valid_vowel_seq(vowels: &[VnLexiName]) -> bool {
    find_vowel_seq(vowels).is_some()
}

pub fn can_vowel_seq_take_coda(vowels: &[VnLexiName]) -> bool {
    match vowels.len() {
        0 => false,
        1 => true,
        2 => {
            let v0 = normalize_vowel(vowels[0]);
            let v1 = normalize_vowel(vowels[1]);
            matches!(
                (v0, v1),
                (VnLexiName::I, VnLexiName::E)
                    | (VnLexiName::I, VnLexiName::Er)
                    | (VnLexiName::Y, VnLexiName::E)
                    | (VnLexiName::Y, VnLexiName::Er)
                    | (VnLexiName::U, VnLexiName::O)
                    | (VnLexiName::U, VnLexiName::Or)
                    | (VnLexiName::U, VnLexiName::Oh)
                    | (VnLexiName::Uh, VnLexiName::Oh)
                    | (VnLexiName::Uh, VnLexiName::O)
                    | (VnLexiName::O, VnLexiName::A)
                    | (VnLexiName::O, VnLexiName::Ah)
                    | (VnLexiName::O, VnLexiName::E)
                    | (VnLexiName::U, VnLexiName::Ar)
                    | (VnLexiName::U, VnLexiName::Er)
                    | (VnLexiName::U, VnLexiName::Y)
                    | (VnLexiName::O, VnLexiName::O)
                    | (VnLexiName::O, VnLexiName::Or)
            )
        }
        3 => {
            let v0 = normalize_vowel(vowels[0]);
            let v1 = normalize_vowel(vowels[1]);
            let v2 = normalize_vowel(vowels[2]);
            v0 == VnLexiName::U
                && v1 == VnLexiName::Y
                && (v2 == VnLexiName::E || v2 == VnLexiName::Er)
        }
        _ => false,
    }
}

pub fn is_valid_ending_consonant(ending: &[char]) -> bool {
    let mut lower = ['\0'; 2];
    for (i, &c) in ending.iter().take(2).enumerate() {
        lower[i] = c.to_ascii_lowercase();
    }
    matches!(
        &lower[..ending.len().min(2)],
        ['c'] | ['m'] | ['n'] | ['p'] | ['t'] | ['c', 'h'] | ['n', 'g'] | ['n', 'h']
    )
}

pub fn is_stop_coda(ending: &[char]) -> bool {
    let mut lower = ['\0'; 2];
    for (i, &c) in ending.iter().take(2).enumerate() {
        lower[i] = c.to_ascii_lowercase();
    }
    matches!(
        &lower[..ending.len().min(2)],
        ['c'] | ['p'] | ['t'] | ['c', 'h']
    )
}

pub fn is_valid_initial(initial: &[char]) -> bool {
    let mut lower = ['\0'; 3];
    for (i, &c) in initial.iter().take(3).enumerate() {
        lower[i] = c.to_ascii_lowercase();
    }
    matches!(
        &lower[..initial.len().min(3)],
        ['b'] | ['c'] | ['d'] | ['g'] | ['h'] | ['k'] | ['l'] | ['m'] | ['n'] | ['p'] | ['r']
            | ['s'] | ['t'] | ['v'] | ['x']
            | ['c', 'h'] | ['g', 'h'] | ['g', 'i'] | ['k', 'h'] | ['n', 'g'] | ['n', 'h'] | ['p', 'h']
            | ['q', 'u'] | ['t', 'h'] | ['t', 'r']
            | ['n', 'g', 'h']
    )
}

pub fn is_valid_initial_with_vowel(initial: &[char], first_vowel: VnLexiName) -> bool {
    if initial.is_empty() {
        return true;
    }

    let norm_v = normalize_vowel(first_vowel);
    let mut init_lower = ['\0'; 3];
    for (i, &c) in initial.iter().take(3).enumerate() {
        init_lower[i] = c.to_ascii_lowercase();
    }
    let lower_slice = &init_lower[..initial.len().min(3)];

    // 'k' only pairs with e, ê, i, y
    if lower_slice == ['k'] {
        return matches!(norm_v, VnLexiName::E | VnLexiName::Er | VnLexiName::I | VnLexiName::Y);
    }

    // 'gh', 'ngh' only with e, ê, i
    if lower_slice == ['g', 'h'] || lower_slice == ['n', 'g', 'h'] {
        return matches!(norm_v, VnLexiName::E | VnLexiName::Er | VnLexiName::I);
    }

    // 'g', 'ng' cannot pair with e, ê, i (must use 'gh', 'ngh')
    if lower_slice == ['g'] || lower_slice == ['n', 'g'] {
        return !matches!(norm_v, VnLexiName::E | VnLexiName::Er | VnLexiName::I);
    }

    // 'c' does not pair with e, ê, i, y
    if lower_slice == ['c'] {
        return !matches!(norm_v, VnLexiName::E | VnLexiName::Er | VnLexiName::I | VnLexiName::Y);
    }

    // 'gi' does not pair with i, 'qu' does not pair with u
    if lower_slice.len() == 2 {
        if lower_slice == ['g', 'i'] && norm_v == VnLexiName::I {
            return false;
        }
        if lower_slice == ['q', 'u'] && norm_v == VnLexiName::U {
            return false;
        }
    }

    true
}
