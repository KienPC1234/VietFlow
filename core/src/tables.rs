// VietFlow Core - Tables and Mappings
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::models::VnLexiName;

pub const TOTAL_VN_CHARS: usize = 213;
pub const TOTAL_ALPHA_VN_CHARS: usize = 186;

pub static UNICODE_TABLE: [char; TOTAL_VN_CHARS] = [
    '\u{0041}', '\u{0061}', '\u{00c1}', '\u{00e1}', '\u{00c0}', '\u{00e0}', '\u{1ea2}', '\u{1ea3}', '\u{00c3}', '\u{00e3}', '\u{1ea0}', '\u{1ea1}', // a
    '\u{00c2}', '\u{00e2}', '\u{1ea4}', '\u{1ea5}', '\u{1ea6}', '\u{1ea7}', '\u{1ea8}', '\u{1ea9}', '\u{1eaa}', '\u{1eab}', '\u{1eac}', '\u{1ead}', // a^
    '\u{0102}', '\u{0103}', '\u{1eae}', '\u{1eaf}', '\u{1eb0}', '\u{1eb1}', '\u{1eb2}', '\u{1eb3}', '\u{1eb4}', '\u{1eb5}', '\u{1eb6}', '\u{1eb7}', // a(
    '\u{0042}', '\u{0062}', '\u{0043}', '\u{0063}', '\u{0044}', '\u{0064}',                                                                 // B b C c D d
    '\u{0110}', '\u{0111}',                                                                                                         // DD, dd
    '\u{0045}', '\u{0065}', '\u{00c9}', '\u{00e9}', '\u{00c8}', '\u{00e8}', '\u{1eba}', '\u{1ebb}', '\u{1ebc}', '\u{1ebd}', '\u{1eb8}', '\u{1eb9}', // e
    '\u{00ca}', '\u{00ea}', '\u{1ebe}', '\u{1ebf}', '\u{1ec0}', '\u{1ec1}', '\u{1ec2}', '\u{1ec3}', '\u{1ec4}', '\u{1ec5}', '\u{1ec6}', '\u{1ec7}', // e^
    '\u{0046}', '\u{0066}', '\u{0047}', '\u{0067}', '\u{0048}', '\u{0068}',                                                                 // F f G g H h
    '\u{0049}', '\u{0069}', '\u{00cd}', '\u{00ed}', '\u{00cc}', '\u{00ec}', '\u{1ec8}', '\u{1ec9}', '\u{0128}', '\u{0129}', '\u{1eca}', '\u{1ecb}', // i
    '\u{004a}', '\u{006a}', '\u{004b}', '\u{006b}', '\u{004c}', '\u{006c}', '\u{004d}', '\u{006d}', '\u{004e}', '\u{006e}',                         // J j K k L l M m N n
    '\u{004f}', '\u{006f}', '\u{00d3}', '\u{00f3}', '\u{00d2}', '\u{00f2}', '\u{1ece}', '\u{1ecf}', '\u{00d5}', '\u{00f5}', '\u{1ecc}', '\u{1ecd}', // o
    '\u{00d4}', '\u{00f4}', '\u{1ed0}', '\u{1ed1}', '\u{1ed2}', '\u{1ed3}', '\u{1ed4}', '\u{1ed5}', '\u{1ed6}', '\u{1ed7}', '\u{1ed8}', '\u{1ed9}', // o^
    '\u{01a0}', '\u{01a1}', '\u{1eda}', '\u{1edb}', '\u{1edc}', '\u{1edd}', '\u{1ede}', '\u{1edf}', '\u{1ee0}', '\u{1ee1}', '\u{1ee2}', '\u{1ee3}', // o+
    '\u{0050}', '\u{0070}', '\u{0051}', '\u{0071}', '\u{0052}', '\u{0072}', '\u{0053}', '\u{0073}', '\u{0054}', '\u{0074}',                         // P p Q q R r S s T t
    '\u{0055}', '\u{0075}', '\u{00da}', '\u{00fa}', '\u{00d9}', '\u{00f9}', '\u{1ee6}', '\u{1ee7}', '\u{0168}', '\u{0169}', '\u{1ee4}', '\u{1ee5}', // u
    '\u{01af}', '\u{01b0}', '\u{1ee8}', '\u{1ee9}', '\u{1eea}', '\u{1eeb}', '\u{1eec}', '\u{1eed}', '\u{1eee}', '\u{1eef}', '\u{1ef0}', '\u{1ef1}', // u+
    '\u{0056}', '\u{0076}', '\u{0057}', '\u{0077}', '\u{0058}', '\u{0078}',                                                                 // V v W w X x
    '\u{0059}', '\u{0079}', '\u{00dd}', '\u{00fd}', '\u{1ef2}', '\u{1ef3}', '\u{1ef6}', '\u{1ef7}', '\u{1ef8}', '\u{1ef9}', '\u{1ef4}', '\u{1ef5}', // y
    '\u{005a}', '\u{007a}',                                                                                                         // Z z
    // Symbols that have different code points in Unicode and Western charsets
    '\u{20AC}', '\u{20A1}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{02C6}',
    '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{017D}', '\u{2018}', '\u{2019}', '\u{201C}',
    '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
    '\u{0153}', '\u{017E}', '\u{0178}'
];

pub static TCVN3_TABLE: [u8; TOTAL_VN_CHARS] = [
    0x41, 0x61, 0xB8, 0xB8, 0xB5, 0xB5, 0xB6, 0xB6, 0xB7, 0xB7, 0xB9, 0xB9, 0xA2, 0xA9, 0xCA, 0xCA, 0xC7, 0xC7, 0xC8, 0xC8, 0xC9, 0xC9, 0xCB, 0xCB, 0xA1, 0xA8, 0xBE, 0xBE, 0xBB, 0xBB, 0xBC, 0xBC, 0xBD, 0xBD, 0xC6, 0xC6, 0x42, 0x62, 0x43, 0x63, 0x44, 0x64, 0xA7, 0xAE, 0x45, 0x65, 0xD0, 0xD0, 0xCC, 0xCC, 0xCE, 0xCE, 0xCF, 0xCF, 0xD1, 0xD1, 0xA3, 0xAA, 0xD5, 0xD5, 0xD2, 0xD2, 0xD3, 0xD3, 0xD4, 0xD4, 0xD6, 0xD6, 0x46, 0x66, 0x47, 0x67, 0x48, 0x68, 0x49, 0x69, 0xDD, 0xDD, 0xD7, 0xD7, 0xD8, 0xD8, 0xDC, 0xDC, 0xDE, 0xDE, 0x4A, 0x6A, 0x4B, 0x6B, 0x4C, 0x6C, 0x4D, 0x6D, 0x4E, 0x6E, 0x4F, 0x6F, 0xE3, 0xE3, 0xDF, 0xDF, 0xE1, 0xE1, 0xE2, 0xE2, 0xE4, 0xE4, 0xA4, 0xAB, 0xE8, 0xE8, 0xE5, 0xE5, 0xE6, 0xE6, 0xE7, 0xE7, 0xE9, 0xE9, 0xA5, 0xAC, 0xED, 0xED, 0xEA, 0xEA, 0xEB, 0xEB, 0xEC, 0xEC, 0xEE, 0xEE, 0x50, 0x70, 0x51, 0x71, 0x52, 0x72, 0x53, 0x73, 0x54, 0x74, 0x55, 0x75, 0xF3, 0xF3, 0xEF, 0xEF, 0xF1, 0xF1, 0xF2, 0xF2, 0xF4, 0xF4, 0xA6, 0xAD, 0xF8, 0xF8, 0xF5, 0xF5, 0xF6, 0xF6, 0xF7, 0xF7, 0xF9, 0xF9, 0x56, 0x76, 0x57, 0x77, 0x58, 0x78, 0x59, 0x79, 0xFD, 0xFD, 0xFA, 0xFA, 0xFB, 0xFB, 0xFC, 0xFC, 0xFE, 0xFE, 0x5A, 0x7A, 0x80, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8C, 0x8E, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C, 0x9E, 0x9F
];

pub static VNI_WIN_TABLE: [u16; TOTAL_VN_CHARS] = [
    0x0041, 0x0061, 0xD941, 0xF961, 0xD841, 0xF861, 0xDB41, 0xFB61, 0xD541, 0xF561, 0xCF41, 0xEF61, 0xC241, 0xE261, 0xC141, 0xE161, 0xC041, 0xE061, 0xC541, 0xE561, 0xC341, 0xE361, 0xC441, 0xE461, 0xCA41, 0xEA61, 0xC941, 0xE961, 0xC841, 0xE861, 0xDA41, 0xFA61, 0xDC41, 0xFC61, 0xCB41, 0xEB61, 0x0042, 0x0062, 0x0043, 0x0063, 0x0044, 0x0064, 0x00D1, 0x00F1, 0x0045, 0x0065, 0xD945, 0xF965, 0xD845, 0xF865, 0xDB45, 0xFB65, 0xD545, 0xF565, 0xCF45, 0xEF65, 0xC245, 0xE265, 0xC145, 0xE165, 0xC045, 0xE065, 0xC545, 0xE565, 0xC345, 0xE365, 0xC445, 0xE465, 0x0046, 0x0066, 0x0047, 0x0067, 0x0048, 0x0068, 0x0049, 0x0069, 0x00CD, 0x00ED, 0x00CC, 0x00EC, 0x00C6, 0x00E6, 0x00D3, 0x00F3, 0x00D2, 0x00F2, 0x004A, 0x006A, 0x004B, 0x006B, 0x004C, 0x006C, 0x004D, 0x006D, 0x004E, 0x006E, 0x004F, 0x006F, 0xD94F, 0xF96F, 0xD84F, 0xF86F, 0xDB4F, 0xFB6F, 0xD54F, 0xF56F, 0xCF4F, 0xEF6F, 0xC24F, 0xE26F, 0xC14F, 0xE16F, 0xC04F, 0xE06F, 0xC54F, 0xE56F, 0xC34F, 0xE36F, 0xC44F, 0xE46F, 0x00D4, 0x00F4, 0xD9D4, 0xF9F4, 0xD8D4, 0xF8F4, 0xDBD4, 0xFBF4, 0xD5D4, 0xF5F4, 0xCFD4, 0xEFF4, 0x0050, 0x0070, 0x0051, 0x0071, 0x0052, 0x0072, 0x0053, 0x0073, 0x0054, 0x0074, 0x0055, 0x0075, 0xD955, 0xF975, 0xD855, 0xF875, 0xDB55, 0xFB75, 0xD555, 0xF575, 0xCF55, 0xEF75, 0x00D6, 0x00F6, 0xD9D6, 0xF9F6, 0xD8D6, 0xF8F6, 0xDBD6, 0xFBF6, 0xD5D6, 0xF5F6, 0xCFD6, 0xEFF6, 0x0056, 0x0076, 0x0057, 0x0077, 0x0058, 0x0078, 0x0059, 0x0079, 0xD959, 0xF979, 0xD859, 0xF879, 0xDB59, 0xFB79, 0xD559, 0xF579, 0x00CE, 0x00EE, 0x005A, 0x007A, 0x0080, 0x0082, 0x0083, 0x0084, 0x0085, 0x0086, 0x0087, 0x0088, 0x0089, 0x008A, 0x008B, 0x008C, 0x008E, 0x0091, 0x0092, 0x0093, 0x0094, 0x0095, 0x0096, 0x0097, 0x0098, 0x0099, 0x009A, 0x009B, 0x009C, 0x009E, 0x009F
];

pub static STD_VN_ROOT_CHAR: [u8; TOTAL_VN_CHARS] = [
    0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1,
    12, 13, 12, 13, 12, 13, 12, 13, 12, 13, 12, 13,
    24, 25, 24, 25, 24, 25, 24, 25, 24, 25, 24, 25,
    36, 37, 38, 39, 40, 41, 42, 43,
    44, 45, 44, 45, 44, 45, 44, 45, 44, 45, 44, 45,
    56, 57, 56, 57, 56, 57, 56, 57, 56, 57, 56, 57,
    68, 69, 70, 71, 72, 73,
    74, 75, 74, 75, 74, 75, 74, 75, 74, 75, 74, 75,
    86, 87, 88, 89, 90, 91, 92, 93, 94, 95,
    96, 97, 96, 97, 96, 97, 96, 97, 96, 97, 96, 97,
    108, 109, 108, 109, 108, 109, 108, 109, 108, 109, 108, 109,
    120, 121, 120, 121, 120, 121, 120, 121, 120, 121, 120, 121,
    132, 133, 134, 135, 136, 137, 138, 139, 140, 141,
    142, 143, 142, 143, 142, 143, 142, 143, 142, 143, 142, 143,
    154, 155, 154, 155, 154, 155, 154, 155, 154, 155, 154, 155,
    166, 167, 168, 169, 170, 171,
    172, 173, 172, 173, 172, 173, 172, 173, 172, 173, 172, 173,
    184, 185,
    186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198,
    199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212
];

pub static IS_VN_VOWEL: [bool; TOTAL_VN_CHARS] = {
    let mut v = [false; TOTAL_VN_CHARS];
    let vowels = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, // a
        12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, // ar (â)
        24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, // ah (ă)
        44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, // e
        56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, // er (ê)
        74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, // i
        96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, // o
        108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, // or (ô)
        120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, // oh (ơ)
        142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, // u
        154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, // uh (ư)
        172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, // y
    ];
    let mut i = 0;
    while i < vowels.len() {
        v[vowels[i]] = true;
        i += 1;
    }
    v
};

#[inline(always)]
pub fn iso_to_vn_lexi(c: char) -> VnLexiName {
    match c {
        'a' => VnLexiName::a,
        'A' => VnLexiName::A,
        'b' => VnLexiName::b,
        'B' => VnLexiName::B,
        'c' => VnLexiName::c,
        'C' => VnLexiName::C,
        'd' => VnLexiName::d,
        'D' => VnLexiName::D,
        'e' => VnLexiName::e,
        'E' => VnLexiName::E,
        'f' => VnLexiName::f,
        'F' => VnLexiName::F,
        'g' => VnLexiName::g,
        'G' => VnLexiName::G,
        'h' => VnLexiName::h,
        'H' => VnLexiName::H,
        'i' => VnLexiName::i,
        'I' => VnLexiName::I,
        'j' => VnLexiName::j,
        'J' => VnLexiName::J,
        'k' => VnLexiName::k,
        'K' => VnLexiName::K,
        'l' => VnLexiName::l,
        'L' => VnLexiName::L,
        'm' => VnLexiName::m,
        'M' => VnLexiName::M,
        'n' => VnLexiName::n,
        'N' => VnLexiName::N,
        'o' => VnLexiName::o,
        'O' => VnLexiName::O,
        'p' => VnLexiName::p,
        'P' => VnLexiName::P,
        'q' => VnLexiName::q,
        'Q' => VnLexiName::Q,
        'r' => VnLexiName::r,
        'R' => VnLexiName::R,
        's' => VnLexiName::s,
        'S' => VnLexiName::S,
        't' => VnLexiName::t,
        'T' => VnLexiName::T,
        'u' => VnLexiName::u,
        'U' => VnLexiName::U,
        'v' => VnLexiName::v,
        'V' => VnLexiName::V,
        'w' => VnLexiName::w,
        'W' => VnLexiName::W,
        'x' => VnLexiName::x,
        'X' => VnLexiName::X,
        'y' => VnLexiName::y,
        'Y' => VnLexiName::Y,
        'z' => VnLexiName::z,
        'Z' => VnLexiName::Z,
        _ => VnLexiName::NonVnChar,
    }
}
