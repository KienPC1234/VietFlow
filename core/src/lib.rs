// VietFlow Core - Ultra-High Performance Vietnamese Input Engine
// Licensed under GNU General Public License v3.0 (GPLv3)

pub mod c_abi;
pub mod charset;
pub mod converter;
pub mod english;
pub mod engine;
pub mod input_proc;
pub mod macro_table;
pub mod models;
pub mod tables;
pub mod tone_rules;

pub use charset::encode_slot;
pub use converter::{convert_charset, strip_accents};
pub use engine::{Slot, VietFlowEngine};
pub use macro_table::MacroTable;
pub use models::{CharsetId, EngineOptions, InputMethod, VnLexiName, VnLexiTone};

#[cfg(test)]
mod tests {
    use super::*;

    fn process_string(engine: &mut VietFlowEngine, input: &str) -> String {
        let mut screen: Vec<char> = Vec::new();
        for c in input.chars() {
            let (backspaces, out) = engine.process_key(c);
            for _ in 0..backspaces {
                screen.pop();
            }
            for oc in out.chars() {
                screen.push(oc);
            }
        }
        screen.into_iter().collect()
    }

    #[test]
    fn test_telex_basics() {
        let mut engine = VietFlowEngine::default();
        let cases = [
            ("as", "á"),
            ("af", "à"),
            ("ar", "ả"),
            ("ax", "ã"),
            ("aj", "ạ"),
            ("aa", "â"),
            ("aas", "ấ"),
            ("aaf", "ầ"),
            ("aar", "ẩ"),
            ("aax", "ẫ"),
            ("aaj", "ậ"),
            ("ee", "ê"),
            ("ees", "ế"),
            ("eef", "ề"),
            ("oo", "ô"),
            ("oos", "ố"),
            ("aw", "ă"),
            ("aws", "ắ"),
            ("ow", "ơ"),
            ("ows", "ớ"),
            ("uw", "ư"),
            ("uws", "ứ"),
            ("w", "ư"),
            ("ws", "ứ"),
            ("dd", "đ"),
            ("dda", "đa"),
            ("ddas", "đá"),
            ("vieetj", "việt"),
            ("nam", "nam"),
            ("tieengs", "tiếng"),
            ("dduowngf", "đường"),
            ("nguowif", "người"),
            ("hoangf", "hoàng"),
            ("thuowng", "thương"),
            ("nguyeexn", "nguyễn"),
            ("chuyeenj", "chuyện"),
            ("truowfng", "trường"),
            ("Tieengs Vieetj laf tieengs mej dder.", "Tiếng Việt là tiếng mẹ đẻ."),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed for input: {input}");
        }
    }

    #[test]
    fn test_telex_undo() {
        let mut engine = VietFlowEngine::default();
        let cases = [
            ("ass", "as"),
            ("aff", "af"),
            ("arr", "ar"),
            ("aaa", "aa"),
            ("eee", "ee"),
            ("ooo", "oo"),
            ("aww", "aw"),
            ("oww", "ow"),
            ("uww", "uw"),
            ("ddd", "dd"),
            ("asz", "a"),
            ("vieetjz", "viêt"),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed undo for: {input}");
        }
    }

    #[test]
    fn test_telex_brackets() {
        let mut engine = VietFlowEngine::default();
        assert_eq!(process_string(&mut engine, "["), "ơ");
        engine.reset();
        assert_eq!(process_string(&mut engine, "]"), "ư");
        engine.reset();
        assert_eq!(process_string(&mut engine, "{"), "Ơ");
        engine.reset();
        assert_eq!(process_string(&mut engine, "}"), "Ư");
    }

    #[test]
    fn test_vni_input() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::Vni,
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        let cases = [
            ("a1", "á"),
            ("a2", "à"),
            ("a3", "ả"),
            ("a4", "ã"),
            ("a5", "ạ"),
            ("a6", "â"),
            ("o7", "ơ"),
            ("u7", "ư"),
            ("a8", "ă"),
            ("d9", "đ"),
            ("vie6t5", "việt"),
            ("tie6ng1", "tiếng"),
            ("d9u7o7ng2", "đường"),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed VNI for: {input}");
        }
    }

    #[test]
    fn test_macro_expansion() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                enable_macro: true,
                ..Default::default()
            },
            CharsetId::Unicode,
        );
        engine.macros.insert("vf", "VietFlow");
        engine.macros.insert("vn", "Việt Nam");

        assert_eq!(process_string(&mut engine, "vf "), "VietFlow ");
        engine.reset();
        assert_eq!(process_string(&mut engine, "vn."), "Việt Nam.");
    }

    #[test]
    fn test_strip_accents() {
        assert_eq!(strip_accents("Tiếng Việt"), "Tieng Viet");
        assert_eq!(strip_accents("Đường hoa Nguyễn Huệ"), "Duong hoa Nguyen Hue");
    }

    #[test]
    fn test_backspace() {
        let mut engine = VietFlowEngine::default();
        let actual = process_string(&mut engine, "vieetj\x08");
        assert_eq!(actual, "việ");

        engine.reset();
        let actual2 = process_string(&mut engine, "vieetj\x08\x08");
        assert_eq!(actual2, "vi");
    }

    #[test]
    fn test_simple_telex() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::SimpleTelex,
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        let cases = [
            ("w", "w"),
            ("web", "web"),
            ("wiki", "wiki"),
            ("aw", "ă"),
            ("ow", "ơ"),
            ("uw", "ư"),
            ("aws", "ắ"),
            ("dduowngf", "đường"),
            ("nguowif", "người"),
            ("aa", "â"),
            ("ee", "ê"),
            ("oo", "ô"),
            ("dd", "đ"),
            ("vieetj", "việt"),
            ("nam", "nam"),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed Simple Telex for: {input}");
        }
    }

    #[test]
    fn test_complex_vowels_and_special_cases() {
        let mut engine = VietFlowEngine::default();
        let cases = [
            ("chuyeen", "chuyên"),
            ("truyeenf", "truyền"),
            ("khuyeen", "khuyên"),
            ("nguyeen", "nguyên"),
            ("huyeenj", "huyện"),
            ("thuyeenf", "thuyền"),
            ("quyeenf", "quyền"),
            ("chuyeens", "chuyến"),
            ("nguyeenz", "nguyên"),
            ("nguyeenx", "nguyễn"),
            ("hoanf", "hoàn"),
            ("toans", "toán"),
            ("khoans", "khoán"),
            ("ddoanf", "đoàn"),
            ("hoangf", "hoàng"),
            ("loangx", "loãng"),
            ("thoangs", "thoáng"),
            ("vuownf", "vườn"),
            ("vuonwf", "vườn"),
            ("luowjn", "lượn"),
            ("luonwj", "lượn"),
            ("muowjn", "mượn"),
            ("muonwj", "mượn"),
            ("truwowngf", "trường"),
            ("phuowng", "phương"),
            ("huowngs", "hướng"),
            ("nhieeuf", "nhiều"),
            ("tieeu", "tiêu"),
            ("kieeuj", "kiệu"),
            ("hieeur", "hiểu"),
            ("xoay", "xoay"),
            ("loay", "loay"),
            ("ngoays", "ngoáy"),
            ("ngoaif", "ngoài"),
            ("khoais", "khoái"),
            ("loaij", "loại"),
            ("xuaan", "xuân"),
            ("chuaanr", "chuẩn"),
            ("luaanj", "luận"),
            ("xuaats", "xuất"),
            ("luaatj", "luật"),
            // Tone alteration
            ("toansf", "toàn"),
            ("toanfr", "toản"),
            ("toansz", "toan"),
            ("toanss", "toans"),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed complex vowel for: {input}");
        }
    }

    #[test]
    fn test_english_auto_restore() {
        let mut engine = VietFlowEngine::default();
        // Typing English keyword followed by space or punctuation should restore English word
        let actual = process_string(&mut engine, "restart ");
        assert_eq!(actual, "restart ");

        engine.reset();
        let actual_code = process_string(&mut engine, "custom ");
        assert_eq!(actual_code, "custom ");
    }
}
