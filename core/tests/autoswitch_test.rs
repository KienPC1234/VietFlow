// VietFlow Core - AutoSwitch Comprehensive Integration Tests
// Licensed under GNU General Public License v3.0 (GPLv3)

use vietflow_core::{CharsetId, EngineOptions, InputMethod, VietFlowEngine};

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
fn test_autoswitch_lookback_english_sentence() {
    let mut engine = VietFlowEngine::new(
        EngineOptions {
            method: InputMethod::Telex,
            auto_switch_by_typing: true,
            ..Default::default()
        },
        CharsetId::Unicode,
    );

    // "this makes sense "
    // "this" typed in VN mode alone produces "thí".
    // When "makes " follows, lookback detects "thí makes" -> restores "this makes ".
    let out = process_string(&mut engine, "this makes sense ");
    assert_eq!(out, "this makes sense ");
}

#[test]
fn test_autoswitch_bigram_bus_vs_bu() {
    let mut engine = VietFlowEngine::new(
        EngineOptions {
            method: InputMethod::Telex,
            auto_switch_by_typing: true,
            ..Default::default()
        },
        CharsetId::Unicode,
    );

    // "xe bus " -> bigram (xe, bus) heavily favors "bus" over "bú"
    let out_bus = process_string(&mut engine, "xe bus ");
    assert_eq!(out_bus, "xe bus ");

    engine.reset();
    // "cho con bus " -> bigram (con, bú) heavily favors "bú" over "bus"
    let out_bu = process_string(&mut engine, "cho con bus ");
    assert_eq!(out_bu, "cho con bú ");
}

#[test]
fn test_vietnamese_youth_abbreviations_stay_in_vietnamese_mode() {
    let mut engine = VietFlowEngine::new(
        EngineOptions {
            method: InputMethod::Telex,
            auto_switch_by_typing: true,
            ..Default::default()
        },
        CharsetId::Unicode,
    );

    // "ok bạn ơi " -> typing "ok " must stay in VN mode without triggering auto-switch to English
    for c in "ok ".chars() {
        engine.process_key(c);
    }
    assert_eq!(engine.auto_switch_requested, None, "'ok ' must not switch to English");

    for c in "ht ".chars() {
        engine.process_key(c);
    }
    assert_eq!(engine.auto_switch_requested, None, "'ht ' must not switch to English");

    for c in "nn ".chars() {
        engine.process_key(c);
    }
    assert_eq!(engine.auto_switch_requested, None, "'nn ' must not switch to English");

    for c in "dc ".chars() {
        engine.process_key(c);
    }
    assert_eq!(engine.auto_switch_requested, None, "'dc ' must not switch to English");

    for c in "vs ".chars() {
        engine.process_key(c);
    }
    assert_eq!(engine.auto_switch_requested, None, "'vs ' must not switch to English");

    for c in "ae ".chars() {
        engine.process_key(c);
    }
    assert_eq!(engine.auto_switch_requested, None, "'ae ' must not switch to English");

    // Expanded slang & youth abbreviations test suite
    let slang_words = [
        "oke", "oki", "uk", "uh", "wa", "thui", "lun", "khum", "hok", "haizz",
        "mlem", "ultr", "gato", "xu", "ib", "rep", "tym", "ny", "ck", "vk",
        "game", "link", "web", "app", "bot", "lag", "ping", "zui", "bik"
    ];

    for slang in &slang_words {
        engine.reset();
        let mut test_input = slang.to_string();
        test_input.push(' ');
        for c in test_input.chars() {
            engine.process_key(c);
        }
        assert_eq!(
            engine.auto_switch_requested,
            None,
            "Slang '{slang}' must stay in Vietnamese mode without switching to English"
        );
    }
}

#[test]
fn test_autoswitch_doubled_key_recovery() {
    let mut engine = VietFlowEngine::new(
        EngineOptions {
            method: InputMethod::Telex,
            auto_switch_by_typing: true,
            ..Default::default()
        },
        CharsetId::Unicode,
    );

    // "mixx " -> user typed double x to cancel Telex, intended English "mix"
    let out_mix = process_string(&mut engine, "mixx ");
    assert_eq!(out_mix, "mix ");

    engine.reset();
    let out_box = process_string(&mut engine, "boxx ");
    assert_eq!(out_box, "box ");
}

#[test]
fn test_autoswitch_english_mode_break_detection() {
    let mut engine = VietFlowEngine::new(
        EngineOptions {
            method: InputMethod::Telex,
            auto_switch_by_typing: true,
            ..Default::default()
        },
        CharsetId::Unicode,
    );

    // User is in English mode and types "truowfng" + ' '
    let check_vn = engine.check_english_word_at_break("truowfng", ' ');
    assert!(check_vn.is_some());
    let (bs, text) = check_vn.unwrap();
    assert_eq!(bs, 8); // "truowfng".len()
    assert_eq!(text.as_str(), "trường ");
    assert_eq!(engine.take_auto_switch_request(), Some(true));

    // English word "user" + ' ' in English mode must NOT switch
    let check_en = engine.check_english_word_at_break("user", ' ');
    assert!(check_en.is_none());

    // Gibberish "asdf" + ' ' must NOT switch
    let check_gibberish = engine.check_english_word_at_break("asdf", ' ');
    assert!(check_gibberish.is_none());

    // Context test: typing Vietnamese words after English word "alex":
    engine.push_english_word_lookback("alex");

    // "ddeej" -> "đệ " must switch
    let check_ddeej = engine.check_english_word_at_break("ddeej", ' ');
    assert!(check_ddeej.is_some());
    let (bs_ddeej, text_ddeej) = check_ddeej.unwrap();
    assert_eq!(bs_ddeej, 5);
    assert_eq!(text_ddeej.as_str(), "đệ ");

    // "ddej" -> "đẹ " must also switch
    let check_ddej = engine.check_english_word_at_break("ddej", ' ');
    assert!(check_ddej.is_some());
    let (bs_ddej, text_ddej) = check_ddej.unwrap();
    assert_eq!(bs_ddej, 4);
    assert_eq!(text_ddej.as_str(), "đẹ ");

    // Vietnamese word starting with 'd' in English mode must switch
    let check_d = engine.check_english_word_at_break("dayj", ' ');
    assert!(check_d.is_some());
    let (bs_d, text_d) = check_d.unwrap();
    assert_eq!(bs_d, 4);
    assert_eq!(text_d.as_str(), "dạy ");

    // Vietnamese word with 'đ' (dd) in English mode must switch
    let check_dd = engine.check_english_word_at_break("dduowcj", ' ');
    assert!(check_dd.is_some());
    let (bs_dd, text_dd) = check_dd.unwrap();
    assert_eq!(bs_dd, 7);
    assert_eq!(text_dd.as_str(), "được ");

    // VNI method in English mode
    let mut vni_engine = VietFlowEngine::new(
        EngineOptions {
            method: InputMethod::Vni,
            auto_switch_by_typing: true,
            ..Default::default()
        },
        CharsetId::Unicode,
    );
    let check_vni = vni_engine.check_english_word_at_break("chao2", ' ');
    assert!(check_vni.is_some());
    let (bs_vni, text_vni) = check_vni.unwrap();
    assert_eq!(bs_vni, 5);
    assert_eq!(text_vni.as_str(), "chào ");

    // Programming identifier "get_user" + ' ' in English mode must NOT switch
    let check_code = engine.check_english_word_at_break("get_user", ' ');
    assert!(check_code.is_none());
}

#[test]
fn test_programming_identifiers_and_acronyms() {
    let mut engine = VietFlowEngine::new(
        EngineOptions {
            method: InputMethod::Telex,
            auto_switch_by_typing: true,
            ..Default::default()
        },
        CharsetId::Unicode,
    );

    // Identifiers with 2+ capitals: "HTML", "USA", "getUserId" stay untouched
    assert_eq!(process_string(&mut engine, "HTML "), "HTML ");
    engine.reset();
    assert_eq!(process_string(&mut engine, "USA "), "USA ");
    engine.reset();
    assert_eq!(process_string(&mut engine, "getUserId "), "getUserId ");
}

#[test]
fn test_decision_22_frequency_ratio_and_neighbor_guard() {
    let mut engine = VietFlowEngine::new(
        EngineOptions {
            method: InputMethod::Telex,
            auto_switch_by_typing: true,
            ..Default::default()
        },
        CharsetId::Unicode,
    );

    // Standalone high-frequency Vietnamese words typed in English mode convert via ratio:
    // "cos" -> "có", "nos" -> "nó", "toots" -> "tốt", "sex" -> "sẽ"
    let check_cos = engine.check_english_word_at_break("cos", ' ');
    assert!(check_cos.is_some(), "cos should convert to có");
    let (bs, text) = check_cos.unwrap();
    assert_eq!(bs, 3);
    assert_eq!(text.as_str(), "có ");

    let check_nos = engine.check_english_word_at_break("nos", ' ');
    assert!(check_nos.is_some(), "nos should convert to nó");

    let check_toots = engine.check_english_word_at_break("toots", ' ');
    assert!(check_toots.is_some(), "toots should convert to tốt");

    let check_sex = engine.check_english_word_at_break("sex", ' ');
    assert!(check_sex.is_some(), "sex should convert to sẽ");

    // Guarded English neighbor: "video call cos"
    // After clearly English words, "cos" must stay English
    engine.reset();
    engine.push_english_word_lookback("video");
    engine.push_english_word_lookback("call");
    let check_guarded = engine.check_english_word_at_break("cos", ' ');
    assert!(check_guarded.is_none(), "video call cos must stay English");
}

#[test]
fn test_plain_english_word_switch_without_jitter() {
    let mut engine = VietFlowEngine::new(
        EngineOptions {
            method: InputMethod::Telex,
            auto_switch_by_typing: true,
            ..Default::default()
        },
        CharsetId::Unicode,
    );

    // Typing plain English word "open" in Vietnamese mode:
    // Each key emits natively. At space:
    for c in "open".chars() {
        let (bk, _) = engine.process_key(c);
        assert_eq!(bk, 0);
    }

    // When space arrives at the end of "open":
    // Because "open" is already literal on screen, zero backspaces should be sent!
    let (bk, text) = engine.process_key(' ');
    assert_eq!(bk, 0, "Plain English word must not backspace and retype");
    assert_eq!(text.as_str(), " ");
    assert_eq!(engine.take_auto_switch_request(), Some(false), "Must switch to English");
}

#[test]
fn test_custom_english_keywords_autoswitch() {
    let mut engine = VietFlowEngine::new(
        EngineOptions {
            method: InputMethod::Telex,
            auto_switch_by_typing: true,
            custom_english_keywords: vec!["grpc".to_string(), "redux".to_string(), "props".to_string()],
            ..Default::default()
        },
        CharsetId::Unicode,
    );

    // In English mode, custom keyword "redux" or "props" must never convert to Vietnamese
    let check_redux = engine.check_english_word_at_break("redux", ' ');
    assert!(check_redux.is_none(), "Custom keyword redux must stay in English mode");

    let check_props = engine.check_english_word_at_break("props", ' ');
    assert!(check_props.is_none(), "Custom keyword props must stay in English mode");

    // In Vietnamese mode, typing custom keyword "redux" should restore to raw English and switch to English mode
    engine.reset();
    let out = process_string(&mut engine, "redux ");
    assert_eq!(out, "redux ");
    assert_eq!(engine.take_auto_switch_request(), Some(false));
}

