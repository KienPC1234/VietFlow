// VietFlow Core - Ultra-High Performance Vietnamese Input Engine
// Licensed under GNU General Public License v3.0 (GPLv3)

pub mod c_abi;
pub mod charset;
pub mod constants;
pub mod converter;
pub mod english;
pub mod engine;
pub mod input_proc;
pub mod key_text;
pub mod macro_table;
pub mod models;
pub mod tables;
pub mod tone_rules;

pub use charset::encode_slot;
pub use converter::{convert_charset, convert_charset_bytes, strip_accents};
pub use constants::*;
pub use engine::{Slot, VietFlowEngine};
pub use key_text::KeyText;
pub use macro_table::MacroTable;
pub use models::{CharsetId, CustomKeymap, EngineOptions, InputMethod, VnLexiName, VnLexiTone};

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
    fn test_telex_free_marking_double_keys() {
        let mut engine = VietFlowEngine::default();
        let cases = [
            ("theme", "thêm"),
            ("deme", "dêm"),
            ("ddeme", "đêm"),
            ("tono", "tôn"),
            ("cama", "câm"),
            ("demd", "đem"),
            ("dangd", "đang"),
            ("themee", "theme"),
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
            ("owss", "ows"),
            ("uwss", "uws"),
            ("[ss", "[s"),
            ("]ss", "]s"),
            ("{ss", "{s"),
            ("}ss", "}s"),
            ("a[ss", "a[s"),
            ("ww", "w"),
            ("www", "ww"),
            ("wwww", "www"),
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
        engine.reset();
        assert_eq!(process_string(&mut engine, "[["), "[");
        engine.reset();
        assert_eq!(process_string(&mut engine, "]]"), "]");
        engine.reset();
        assert_eq!(process_string(&mut engine, "{{"), "{");
        engine.reset();
        assert_eq!(process_string(&mut engine, "}}"), "}");
        engine.reset();
        assert_eq!(process_string(&mut engine, "a[["), "a[");
        engine.reset();
        assert_eq!(process_string(&mut engine, "[s"), "ớ");
        engine.reset();
        assert_eq!(process_string(&mut engine, "[sz"), "ơ");
        engine.reset();
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
    fn test_ime_preedit_and_break_resolution() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::Telex,
                modern_style: true,
                free_marking: true,
                spell_check: true,
                auto_non_vn_restore: true,
                enable_macro: true,
                ..Default::default()
            },
            CharsetId::Unicode,
        );
        engine.macros.insert("vn", "Việt Nam");

        // 1. Valid Vietnamese syllable composition
        for c in "vieetj".chars() {
            let _ = engine.process_key(c);
        }
        assert_eq!(engine.current_preedit_text(), "việt");
        assert_eq!(engine.finish_composition_on_break(' '), None);

        // 2. English word auto-restore on break
        for c in "word".chars() {
            let _ = engine.process_key(c);
        }
        assert_eq!(engine.current_preedit_text(), "ưởd");
        assert_eq!(engine.finish_composition_on_break(' '), Some("word".to_string()));

        // 3. Macro expansion on break
        for c in "vn".chars() {
            let _ = engine.process_key(c);
        }
        assert_eq!(engine.current_preedit_text(), "vn");
        assert_eq!(engine.finish_composition_on_break(' '), Some("Việt Nam".to_string()));
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
    fn test_strip_accents_handles_nfd() {
        let nfd = "Tie\u{0302}\u{0301}ng Vie\u{0302}\u{0323}t";
        assert_eq!(strip_accents(nfd), "Tieng Viet");
        assert_eq!(
            strip_accents("đu\u{031B}o\u{031B}\u{0300}ng"),
            "duong",
            "horn + grave combining marks must be removed"
        );
    }

    #[test]
    fn test_strip_accents() {
        assert_eq!(strip_accents("Tiếng Việt"), "Tieng Viet");
        assert_eq!(strip_accents("Đường hoa Nguyễn Huệ"), "Duong hoa Nguyen Hue");
    }

    #[test]
    fn test_backspace() {
        let mut engine = VietFlowEngine::default();
        assert_eq!(engine.process_key('\x08'), (0, KeyText::new()));

        let actual = process_string(&mut engine, "vieetj\x08");
        assert_eq!(actual, "việ");

        engine.reset();
        assert_eq!(engine.process_key('\x08'), (0, KeyText::new()));
        let actual2 = process_string(&mut engine, "vieetj\x08\x08");
        assert_eq!(actual2, "vi");
    }

    #[test]
    fn test_telex_simple_2() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::TelexSimple2,
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        let cases = [
            ("w", "w"),
            ("web", "web"),
            ("win", "win"),
            ("zero", "zero"),
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
            ("toans", "toán"),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed Telex Simple 2 for: {input}");
        }
    }

    #[test]
    fn test_telex_vni_hybrid() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::TelexVni,
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        let cases = [
            // Pure Telex input
            ("toans", "toán"),
            ("vieetj", "việt"),
            ("dduowngf", "đường"),
            ("nguowif", "người"),
            // Pure VNI input
            ("toan1", "toán"),
            ("viet65", "việt"),
            ("d9u7o7ng2", "đường"),
            ("ngu7o7i2", "người"),
            ("d9a6u2", "đầu"),
            // Hybrid mixing
            ("vieet5", "việt"),
            ("dda6u2", "đầu"),
            ("dduowng2", "đường"),
            ("d9u7o7ngf", "đường"),
            ("toan1z", "toan"),
            ("toans0", "toan"),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed Telex + VNI hybrid for: {input}");
        }
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

    #[test]
    fn test_custom_english_keywords() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                custom_english_keywords: vec!["nam".to_string()],
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        for c in "nam".chars() {
            engine.process_key(c);
        }
        assert!(
            !engine.is_valid_vietnamese_word(),
            "user-added keyword must be treated as non-Vietnamese"
        );
    }

    #[test]
    fn test_custom_foreign_initials() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                foreign_initials: vec!['b'],
                ..Default::default()
            },
            CharsetId::Unicode,
        );
        assert_eq!(process_string(&mut engine, "bas"), "bas");

        let mut default_engine = VietFlowEngine::default();
        assert_eq!(process_string(&mut default_engine, "bas"), "bá");
    }

    #[test]
    fn test_convert_charset_all_codecs() {
        let text = "Tiếng Việt Nam đường hoa Nguyễn Huệ";

        // Unicode to TCVN3 and back
        let tcvn3 = convert_charset(text, CharsetId::Unicode, CharsetId::Tcvn3);
        assert_ne!(tcvn3, text);
        let back_from_tcvn3 = convert_charset(&tcvn3, CharsetId::Tcvn3, CharsetId::Unicode);
        assert_eq!(back_from_tcvn3, text);

        // Unicode to VNI Windows and back
        let vni = convert_charset(text, CharsetId::Unicode, CharsetId::VniWindows);
        assert_ne!(vni, text);
        let back_from_vni = convert_charset(&vni, CharsetId::VniWindows, CharsetId::Unicode);
        assert_eq!(back_from_vni, text);

        // Unicode to Unicode Composite (NFD) and back
        let nfd = convert_charset(text, CharsetId::Unicode, CharsetId::UnicodeComposite);
        assert_ne!(nfd, text);
        // Ensure NFD contains combining marks
        assert!(nfd.contains('\u{0301}') || nfd.contains('\u{0323}'));
        let back_from_nfd = convert_charset(&nfd, CharsetId::UnicodeComposite, CharsetId::Unicode);
        assert_eq!(back_from_nfd, text);
    }

    #[test]
    fn test_convert_full_nfd_composite() {
        let nfd = "Tie\u{0302}\u{0301}ng Vie\u{0302}\u{0323}t";
        let nfc = "Ti\u{1EBF}ng Vi\u{1EC7}t";

        assert_eq!(
            convert_charset(nfd, CharsetId::UnicodeComposite, CharsetId::Unicode),
            nfc
        );
        assert_eq!(
            convert_charset(nfc, CharsetId::Unicode, CharsetId::UnicodeComposite),
            nfd
        );
        assert_eq!(
            convert_charset(nfd, CharsetId::UnicodeComposite, CharsetId::Tcvn3),
            convert_charset(nfc, CharsetId::Unicode, CharsetId::Tcvn3)
        );
    }

    #[test]
    fn test_tone_placement_and_vowel_normalization() {
        use crate::tone_rules::{get_tone_position, normalize_vowel};

        // Ensure vowels with tones normalize to uppercase base
        assert_eq!(normalize_vowel(VnLexiName::a1), VnLexiName::A);
        assert_eq!(normalize_vowel(VnLexiName::ar2), VnLexiName::Ar);
        assert_eq!(normalize_vowel(VnLexiName::er5), VnLexiName::Er);

        // Check classic vs modern tone placement for "oa"
        let oa_vowels = [VnLexiName::O, VnLexiName::A];
        let classic_pos = get_tone_position(&oa_vowels, false, false);
        let modern_pos = get_tone_position(&oa_vowels, false, true);
        assert_eq!(classic_pos, 0, "Classic style should place tone on first vowel");
        assert_eq!(modern_pos, 1, "Modern style should place tone on second vowel");

        // When coda is present, tone always lands on the second vowel
        let coda_pos = get_tone_position(&oa_vowels, true, false);
        assert_eq!(coda_pos, 1);
    }

    #[test]
    fn test_english_synced_keywords() {
        use crate::english::is_english_keyword;

        assert!(is_english_keyword("context"));
        assert!(is_english_keyword("status"));
        assert!(is_english_keyword("create"));
        assert!(is_english_keyword("reddit"));
        assert!(is_english_keyword("custom"));
        assert!(is_english_keyword("requires"));
        assert!(!is_english_keyword("test"));
        assert!(!is_english_keyword("reset"));
        assert!(!is_english_keyword("the"));
        assert!(!is_english_keyword("tieng"));

        let defaults = crate::english::default_english_keywords();
        assert!(defaults.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(!defaults.contains(&"test"));
        assert!(!defaults.contains(&"reset"));

        let needed = unsafe { crate::c_abi::vf_english_keywords(std::ptr::null_mut(), 0) };
        assert!(needed > 0);
        let mut buffer = vec![0u8; needed];
        let written = unsafe { crate::c_abi::vf_english_keywords(buffer.as_mut_ptr(), buffer.len()) };
        assert_eq!(written, needed);
        let text = String::from_utf8(buffer).expect("UTF-8 keyword list");
        assert!(text.lines().any(|line| line == "server"));
        assert!(!text.lines().any(|line| line == "test"));
    }

    #[test]
    fn test_removed_and_custom_english_keywords() {
        // "test" is not in the conservative default list, so a custom entry is
        // needed to force English for it. This keeps Vietnamese "tét" typable.
        let options = EngineOptions {
            custom_english_keywords: vec!["test".to_string()],
            ..Default::default()
        };
        let mut engine = VietFlowEngine::new(options, CharsetId::Unicode);
        assert_eq!(process_string(&mut engine, "test "), "test ");
    }

    #[test]
    fn test_common_english_habit_words() {
        let mut engine = VietFlowEngine::default();
        let cases = [
            ("hello ", "hello "),
            ("server ", "server "),
            ("windows ", "windows "),
            ("javascript ", "javascript "),
            ("github ", "github "),
            ("file ", "file "),
            ("ko ", "ko "),
            ("dc ", "dc "),
        ];
        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed English habit word: {input}");
        }
    }

    #[test]
    fn test_mixed_english_sentences() {
        let mut engine = VietFlowEngine::default();
        let cases = [
            ("server config ", "server config "),
            ("github commit ", "github commit "),
            ("docker update ", "docker update "),
            ("file nayf ", "file này "),
            ("code bij looix ", "code bị lỗi "),
            ("push leen ", "push lên "),
            ("pull veef ", "pull về "),
        ];
        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed mixed sentence: {input}");
        }
    }

    #[test]
    fn test_native_minimal_backspaces_diffing() {
        let mut engine = VietFlowEngine::default();
        engine.process_key('t');
        engine.process_key('o');
        engine.process_key('a');
        engine.process_key('n');

        // Typing 's' should only backspace 2 chars ("an") and emit "án"
        let (bk, text) = engine.process_key('s');
        assert_eq!(bk, 2);
        assert_eq!(text.as_str(), "án");

        // Typing 'z' to remove tone should only backspace 2 chars ("án") and emit "an"
        let (bk2, text2) = engine.process_key('z');
        assert_eq!(bk2, 2);
        assert_eq!(text2.as_str(), "an");

        engine.reset();
        // Test typing timeout auto-reset
        engine.options.typing_timeout_ms = 500;
        engine.process_key_with_time('a', 1000);
        // Next key 1000ms later (> 500ms timeout)
        let (bk_timeout, text_timeout) = engine.process_key_with_time('s', 2100);
        assert_eq!(bk_timeout, 0);
        assert_eq!(text_timeout.as_str(), "s");
    }

    #[test]
    fn test_user_custom_input_method() {
        // 1. Telex preset via UserCustom
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::UserCustom,
                custom_keymap: CustomKeymap::telex(),
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        let telex_cases = [
            ("as", "á"),
            ("af", "à"),
            ("ar", "ả"),
            ("ax", "ã"),
            ("aj", "ạ"),
            ("aa", "â"),
            ("aw", "ă"),
            ("ee", "ê"),
            ("oo", "ô"),
            ("ow", "ơ"),
            ("uw", "ư"),
            ("dd", "đ"),
            ("w", "ư"),
            ("tieengs", "tiếng"),
            ("dduwowngf", "đường"),
        ];
        for (input, expected) in telex_cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed custom Telex preset for: {input}");
        }

        // 2. VNI preset via UserCustom
        let mut vni_engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::UserCustom,
                custom_keymap: CustomKeymap::vni(),
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        let vni_cases = [
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
            ("tie6ng1", "tiếng"),
            ("d9u7o7ng2", "đường"),
        ];
        for (input, expected) in vni_cases {
            vni_engine.reset();
            let actual = process_string(&mut vni_engine, input);
            assert_eq!(actual, expected, "Failed custom VNI preset for: {input}");
        }

        // 3. Fully customized keys
        let custom_keys = CustomKeymap {
            acute: "z".to_string(),
            grave: "q".to_string(),
            circumflex: "9".to_string(),
            horn: "8".to_string(),
            breve: "7".to_string(),
            d_mark: "6".to_string(),
            telex_double_keys: false,
            lone_w: false,
            ..Default::default()
        };

        let mut custom_engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::UserCustom,
                custom_keymap: custom_keys,
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        let custom_cases = [
            ("az", "á"),
            ("aq", "à"),
            ("a9", "â"),
            ("u8", "ư"),
            ("a7", "ă"),
            ("d6", "đ"),
            ("d6u8o8ngq", "đường"),
        ];
        for (input, expected) in custom_cases {
            custom_engine.reset();
            let actual = process_string(&mut custom_engine, input);
            assert_eq!(actual, expected, "Failed custom keys for: {input}");
        }

        // 4. Individual modify keys
        let individual_keys = CustomKeymap {
            acute: "s".to_string(),
            roof_a: "3".to_string(),
            roof_e: "4".to_string(),
            telex_double_keys: false,
            ..Default::default()
        };

        let mut ind_engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::UserCustom,
                custom_keymap: individual_keys,
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        ind_engine.reset();
        assert_eq!(process_string(&mut ind_engine, "a3s"), "ấ");
        ind_engine.reset();
        assert_eq!(process_string(&mut ind_engine, "e4s"), "ế");
    }

    #[test]
    fn test_configurable_auto_capitalize_chars() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                auto_capitalize: true,
                auto_capitalize_chars: "。\n".to_string(),
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        assert_eq!(process_string(&mut engine, "xin chào. bạn"), "xin chào. bạn");
        engine.reset();
        assert_eq!(process_string(&mut engine, "xin chào。bạn"), "xin chào。Bạn");
    }

    #[test]
    fn test_viqr_input() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::Viqr,
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        let cases = [
            ("toan'", "toán"),
            ("toan`", "toàn"),
            ("toan?", "toản"),
            ("toan~", "toãn"),
            ("toan.", "toạn"),
            ("a^", "â"),
            ("e^", "ê"),
            ("o^", "ô"),
            ("a(", "ă"),
            ("o+", "ơ"),
            ("u+", "ư"),
            ("dd", "đ"),
            ("ddi", "đi"),
            ("dduong+`", "đường"),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed VIQR for: {input}");
        }
    }

    #[test]
    fn test_words_ending_in_d_not_corrupted() {
        let mut engine = VietFlowEngine::default();
        let cases = [
            ("dad ", "dad "),
            ("did ", "did "),
            ("bad ", "bad "),
            ("red ", "red "),
            ("dead ", "dead "),
            ("demd", "đem"),
            ("dangd", "đang"),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed for input: {input}");
        }
    }

    #[test]
    fn test_macro_local_datetime_expansion() {
        let mut engine = VietFlowEngine::new(
            EngineOptions {
                enable_macro: true,
                ..Default::default()
            },
            CharsetId::Unicode,
        );
        engine.macros.insert("curdate", "@date");
        engine.macros.insert("curtime", "@time");

        let expanded_date = process_string(&mut engine, "curdate ");
        // Format is dd/MM/yyyy
        assert!(expanded_date.trim().len() == 10, "Expected dd/MM/yyyy, got: {expanded_date}");
        assert!(expanded_date.contains('/'));

        engine.reset();
        let expanded_time = process_string(&mut engine, "curtime ");
        // Format is HH:mm
        assert!(expanded_time.trim().len() == 5, "Expected HH:mm, got: {expanded_time}");
        assert!(expanded_time.contains(':'));
    }

    #[test]
    fn test_w_words_english_restore() {
        let mut engine = VietFlowEngine::default();
        let cases = [
            ("win ", "win "),
            ("web ", "web "),
            ("word ", "word "),
            ("with ", "with "),
            ("work ", "work "),
            ("wait ", "wait "),
            ("wifi ", "wifi "),
            ("wiki ", "wiki "),
            ("warn ", "warn "),
            ("wing ", "wing "),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed English restore for: {input}");
        }
    }

    #[test]
    fn test_diphthong_coda_phonotactic_restore() {
        let mut engine = VietFlowEngine::default();
        let cases = [
            ("main ", "main "),
            ("join ", "join "),
            ("point ", "point "),
            ("coin ", "coin "),
            ("suit ", "suit "),
            ("train ", "train "),
            ("rain ", "rain "),
            ("gain ", "gain "),
            ("pain ", "pain "),
            ("mail ", "mail "),
        ];

        for (input, expected) in cases {
            engine.reset();
            let actual = process_string(&mut engine, input);
            assert_eq!(actual, expected, "Failed phonotactic coda restore for: {input}");
        }
    }

    #[test]
    fn test_telex_and_vni_modify_undo() {
        let mut telex_engine = VietFlowEngine::default();
        let telex_cases = [
            ("ddd", "dd"),
            ("aaa", "aa"),
            ("eee", "ee"),
            ("ooo", "oo"),
            ("uoww", "uow"),
            ("baww", "baw"),
            ("bangww", "bangw"),
        ];

        for (input, expected) in telex_cases {
            telex_engine.reset();
            let actual = process_string(&mut telex_engine, input);
            assert_eq!(actual, expected, "Failed Telex undo for: {input}");
        }

        let mut vni_engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::Vni,
                ..Default::default()
            },
            CharsetId::Unicode,
        );
        let vni_cases = [
            ("a66", "a6"),
            ("a88", "a8"),
            ("d99", "d9"),
            ("ban11", "ban1"),
        ];

        for (input, expected) in vni_cases {
            vni_engine.reset();
            let actual = process_string(&mut vni_engine, input);
            assert_eq!(actual, expected, "Failed VNI undo for: {input}");
        }
    }
}
