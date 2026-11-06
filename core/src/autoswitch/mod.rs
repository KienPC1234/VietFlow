// VietFlow Core - AutoSwitch Engine (Unigrams, Bigrams, Frequency Scoring)
// Licensed under GNU General Public License v3.0 (GPLv3)

pub mod bigrams;
pub mod data;
pub mod lookback;

pub use bigrams::*;
pub use data::*;
pub use lookback::*;

pub const AS_MISSING: i8 = -128;
pub const AS_VN_FLOOR: i8 = 10;
pub const AS_EN_FLOOR: i8 = 10;
pub const AS_PREFER: i8 = 10;
pub const AS_VN_EXTREME: i8 = 99;
pub const AS_TIE_PREV: i8 = 5;
pub const AS_PAIR_OVERRIDES_D22: i8 = 10;
pub const AS_LOOKBACK_MARGIN: i8 = 15;

#[inline]
fn get_entry<'a>(raw: &'a [u8], offsets: &[u32], idx: usize) -> &'a [u8] {
    let start = offsets[idx] as usize;
    let mut end = start;
    while end < raw.len() && raw[end] != 0 {
        end += 1;
    }
    &raw[start..end]
}

/// Look up the log-frequency score of a normalized Vietnamese syllable key.
/// Returns score in range [-120, 120] (where 10 points = 10x frequency), or None if not found.
pub fn lookup_vn_score(telex_key: &str) -> Option<i8> {
    if telex_key.is_empty() || telex_key.len() > 16 {
        return None;
    }
    let target = telex_key.as_bytes();
    let mut lo = 0usize;
    let mut hi = data::VN_COUNT.saturating_sub(1);

    while lo <= hi {
        let mid = (lo + hi) / 2;
        let entry = get_entry(data::VN_WORDS_RAW, &data::VN_OFFSETS, mid);
        match entry.cmp(target) {
            std::cmp::Ordering::Equal => return Some(data::VN_SCORES[mid]),
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => {
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            }
        }
    }
    None
}

/// Look up the log-frequency score of a raw lowercase English word.
/// Returns score in range [-120, 120] (where 10 points = 10x frequency), or None if not found.
pub fn lookup_en_score(raw_word: &str) -> Option<i8> {
    if raw_word.len() < 2 || raw_word.len() > 32 {
        return None;
    }
    let target = raw_word.as_bytes();
    let mut lo = 0usize;
    let mut hi = data::EN_COUNT.saturating_sub(1);

    while lo <= hi {
        let mid = (lo + hi) / 2;
        let entry = get_entry(data::EN_WORDS_RAW, &data::EN_OFFSETS, mid);
        match entry.cmp(target) {
            std::cmp::Ordering::Equal => return Some(data::EN_SCORES[mid]),
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => {
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            }
        }
    }
    None
}

/// Binary search in Bigram table for the frequency count of (prev_word, curr_word).
pub fn lookup_bigram_count(prev: &str, curr: &str) -> usize {
    if prev.is_empty() || curr.is_empty() {
        return 0;
    }
    let mut key_buf = [0u8; 64];
    let prev_bytes = prev.as_bytes();
    let curr_bytes = curr.as_bytes();
    let total_len = prev_bytes.len() + 1 + curr_bytes.len();
    if total_len > key_buf.len() {
        return 0;
    }
    key_buf[..prev_bytes.len()].copy_from_slice(prev_bytes);
    key_buf[prev_bytes.len()] = b'\t';
    key_buf[prev_bytes.len() + 1..total_len].copy_from_slice(curr_bytes);
    let target = &key_buf[..total_len];

    let mut lo = 0usize;
    let mut hi = bigrams::BIGRAM_COUNT.saturating_sub(1);

    while lo <= hi {
        let mid = (lo + hi) / 2;
        let entry = get_entry(bigrams::BIGRAM_KEYS_RAW, &bigrams::BIGRAM_OFFSETS, mid);
        match entry.cmp(target) {
            std::cmp::Ordering::Equal => {
                let log_count = bigrams::BIGRAM_LOG_COUNTS[mid] as f64;
                // Count = 2^(log_count / 8.0)
                return (log_count / 8.0).exp2().round() as usize;
            }
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => {
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            }
        }
    }
    0
}

/// Calculate Bigram vote between Vietnamese reading and English reading following prev word.
/// Returns positive if Vietnamese is more likely, negative if English is more likely.
pub fn bigram_vote(prev_surface: &str, vn_surface: &str, en_surface: &str) -> i8 {
    let cv = lookup_bigram_count(prev_surface, vn_surface);
    let ce = lookup_bigram_count(prev_surface, en_surface);
    if cv + ce < 3 {
        return 0;
    }
    let ratio = (cv as f64 + 0.5) / (ce as f64 + 0.5);
    let vote = 10.0 * ratio.log10() * 2.0; // Weight = 2.0 (AS_BIGRAM_W)
    vote.clamp(-30.0, 30.0).round() as i8
}

/// Convert a composed Vietnamese syllable or string into normalized ASCII Telex key.
/// E.g.: "trường" -> "truwowngf", "tiếng" -> "tieengs", "hoà" -> "hoaf", "hòa" -> "hoaf", "được" -> "dduowcj".
pub fn to_telex_normalized_key(word: &str) -> Option<String> {
    if word.is_empty() || word.len() > 32 {
        return None;
    }
    let mut out = String::with_capacity(word.len() + 2);
    let mut tone_char: Option<char> = None;

    for c in word.chars() {
        let lower = c.to_ascii_lowercase();
        // Check precomposed accents
        match lower {
            'a' | 'b' | 'c' | 'e' | 'g' | 'h' | 'i' | 'k' | 'l' | 'm' |
            'n' | 'o' | 'p' | 'q' | 'r' | 's' | 't' | 'u' | 'v' | 'x' | 'y' => {
                out.push(lower);
            }
            'á' => { out.push('a'); set_tone(&mut tone_char, 's')?; }
            'à' => { out.push('a'); set_tone(&mut tone_char, 'f')?; }
            'ả' => { out.push('a'); set_tone(&mut tone_char, 'r')?; }
            'ã' => { out.push('a'); set_tone(&mut tone_char, 'x')?; }
            'ạ' => { out.push('a'); set_tone(&mut tone_char, 'j')?; }

            'â' => { out.push_str("aa"); }
            'ấ' => { out.push_str("aa"); set_tone(&mut tone_char, 's')?; }
            'ầ' => { out.push_str("aa"); set_tone(&mut tone_char, 'f')?; }
            'ẩ' => { out.push_str("aa"); set_tone(&mut tone_char, 'r')?; }
            'ẫ' => { out.push_str("aa"); set_tone(&mut tone_char, 'x')?; }
            'ậ' => { out.push_str("aa"); set_tone(&mut tone_char, 'j')?; }

            'ă' => { out.push_str("aw"); }
            'ắ' => { out.push_str("aw"); set_tone(&mut tone_char, 's')?; }
            'ằ' => { out.push_str("aw"); set_tone(&mut tone_char, 'f')?; }
            'ẳ' => { out.push_str("aw"); set_tone(&mut tone_char, 'r')?; }
            'ẵ' => { out.push_str("aw"); set_tone(&mut tone_char, 'x')?; }
            'ặ' => { out.push_str("aw"); set_tone(&mut tone_char, 'j')?; }

            'é' => { out.push('e'); set_tone(&mut tone_char, 's')?; }
            'è' => { out.push('e'); set_tone(&mut tone_char, 'f')?; }
            'ẻ' => { out.push('e'); set_tone(&mut tone_char, 'r')?; }
            'ẽ' => { out.push('e'); set_tone(&mut tone_char, 'x')?; }
            'ẹ' => { out.push('e'); set_tone(&mut tone_char, 'j')?; }

            'ê' => { out.push_str("ee"); }
            'ế' => { out.push_str("ee"); set_tone(&mut tone_char, 's')?; }
            'ề' => { out.push_str("ee"); set_tone(&mut tone_char, 'f')?; }
            'ể' => { out.push_str("ee"); set_tone(&mut tone_char, 'r')?; }
            'ễ' => { out.push_str("ee"); set_tone(&mut tone_char, 'x')?; }
            'ệ' => { out.push_str("ee"); set_tone(&mut tone_char, 'j')?; }

            'í' => { out.push('i'); set_tone(&mut tone_char, 's')?; }
            'ì' => { out.push('i'); set_tone(&mut tone_char, 'f')?; }
            'ỉ' => { out.push('i'); set_tone(&mut tone_char, 'r')?; }
            'ĩ' => { out.push('i'); set_tone(&mut tone_char, 'x')?; }
            'ị' => { out.push('i'); set_tone(&mut tone_char, 'j')?; }

            'ó' => { out.push('o'); set_tone(&mut tone_char, 's')?; }
            'ò' => { out.push('o'); set_tone(&mut tone_char, 'f')?; }
            'ỏ' => { out.push('o'); set_tone(&mut tone_char, 'r')?; }
            'õ' => { out.push('o'); set_tone(&mut tone_char, 'x')?; }
            'ọ' => { out.push('o'); set_tone(&mut tone_char, 'j')?; }

            'ô' => { out.push_str("oo"); }
            'ố' => { out.push_str("oo"); set_tone(&mut tone_char, 's')?; }
            'ồ' => { out.push_str("oo"); set_tone(&mut tone_char, 'f')?; }
            'ổ' => { out.push_str("oo"); set_tone(&mut tone_char, 'r')?; }
            'ỗ' => { out.push_str("oo"); set_tone(&mut tone_char, 'x')?; }
            'ộ' => { out.push_str("oo"); set_tone(&mut tone_char, 'j')?; }

            'ơ' => { out.push_str("ow"); }
            'ớ' => { out.push_str("ow"); set_tone(&mut tone_char, 's')?; }
            'ờ' => { out.push_str("ow"); set_tone(&mut tone_char, 'f')?; }
            'ở' => { out.push_str("ow"); set_tone(&mut tone_char, 'r')?; }
            'ỡ' => { out.push_str("ow"); set_tone(&mut tone_char, 'x')?; }
            'ợ' => { out.push_str("ow"); set_tone(&mut tone_char, 'j')?; }

            'ú' => { out.push('u'); set_tone(&mut tone_char, 's')?; }
            'ù' => { out.push('u'); set_tone(&mut tone_char, 'f')?; }
            'ủ' => { out.push('u'); set_tone(&mut tone_char, 'r')?; }
            'ũ' => { out.push('u'); set_tone(&mut tone_char, 'x')?; }
            'ụ' => { out.push('u'); set_tone(&mut tone_char, 'j')?; }

            'ư' => { out.push_str("uw"); }
            'ứ' => { out.push_str("uw"); set_tone(&mut tone_char, 's')?; }
            'ừ' => { out.push_str("uw"); set_tone(&mut tone_char, 'f')?; }
            'ử' => { out.push_str("uw"); set_tone(&mut tone_char, 'r')?; }
            'ữ' => { out.push_str("uw"); set_tone(&mut tone_char, 'x')?; }
            'ự' => { out.push_str("uw"); set_tone(&mut tone_char, 'j')?; }

            'ý' => { out.push('y'); set_tone(&mut tone_char, 's')?; }
            'ỳ' => { out.push('y'); set_tone(&mut tone_char, 'f')?; }
            'ỷ' => { out.push('y'); set_tone(&mut tone_char, 'r')?; }
            'ỹ' => { out.push('y'); set_tone(&mut tone_char, 'x')?; }
            'ỵ' => { out.push('y'); set_tone(&mut tone_char, 'j')?; }

            'd' => { out.push('d'); }
            'đ' => { out.push_str("dd"); }
            _ => return None,
        }
    }

    if let Some(t) = tone_char {
        out.push(t);
    }
    Some(out)
}

#[inline]
fn set_tone(tone_char: &mut Option<char>, tone: char) -> Option<()> {
    if tone_char.is_some() {
        return None; // Double tone in one syllable is invalid
    }
    *tone_char = Some(tone);
    Some(())
}

/// Common Vietnamese youth slang, abbreviations, teencode, and informal chat particles
/// that should never trigger an auto-switch to English or be restored as English words.
pub fn is_vietnamese_abbrev_or_slang(word: &str) -> bool {
    matches!(
        word,
        // Confirmations & responses
        "ok" | "oke" | "oki" | "okie" | "okee" | "okela" | "okla" | "okiii" | "okiie" | "okii"
            | "uk" | "uh" | "uhm" | "um" | "umm" | "umk" | "ukm" | "ummm" | "uhmm"
            | "ye" | "yes" | "yep" | "yeah" | "r" | "roi" | "rùi" | "rui" | "ròii" | "roii"
            | "nè" | "ne" | "nha" | "nho" | "nhoá" | "nhoa" | "nhó" | "nhe" | "nhen" | "nhé"
            | "thui" | "thôi" | "thoy" | "ha" | "hả" | "hẻ" | "hở" | "hé" | "hén"
            | "lun" | "luôn" | "luon" | "wa" | "wá" | "quá" | "qua"
            | "z" | "zậy" | "zay" | "dậy" | "vại" | "vạy" | "zạ" | "dạ"
            | "j" | "ji" | "sao" | "saoz" | "saoo"

        // Negations & queries
        | "k" | "ko" | "hok" | "hông" | "hong" | "hem" | "khum" | "khom" | "kô" | "kro"
        | "chx" | "chua" | "kdc" | "dou" | "đou" | "dau" | "bn" | "nhiu"
        | "tsao" | "ts" | "ntn" | "tn"

        // Pronouns & relationships
        | "ae" | "ace" | "mk" | "m" | "t" | "ck" | "vk" | "ny" | "cr" | "ex"
        | "bb" | "b" | "c" | "e" | "a" | "mn" | "mng" | "ng" | "ngta" | "ngkhac"
        | "bame" | "bm" | "ad" | "mod"

        // Verbs & common chat actions
        | "dc" | "đc" | "vs" | "cx" | "ht" | "nn" | "cb" | "bt" | "tg" | "stt"
        | "ib" | "inb" | "ibx" | "rep" | "repp" | "cmt" | "bl" | "sub" | "fl" | "fol"
        | "nch" | "nc" | "lq" | "lquan" | "qt" | "qtam" | "tl" | "lh" | "ktra" | "kt"
        | "bik" | "bit" | "bít" | "thik" | "thix" | "thík" | "di" | "đy" | "zui" | "tym" | "kb"
        | "phot" | "flex" | "chill" | "slay" | "non" | "pro"

        // Interjections, laughs & teencode
        | "haizz" | "haiz" | "hazz" | "hix" | "hic" | "huhu" | "hehe" | "hihi" | "hoho"
        | "kaka" | "kkk" | "kk" | "mlem" | "ultr" | "gato" | "xu" | "keo"
        | "cl" | "vl" | "vcl" | "vkl" | "vđ" | "vch" | "vcc" | "vler" | "vloz" | "vlon"
        | "clgt" | "cmm" | "cmn" | "cmnr" | "dm" | "đm" | "dkm" | "đkm" | "dcm" | "đcm"

        // Dates, tech & social
        | "hnay" | "hn" | "bjo" | "bh" | "t7" | "cn" | "ksao" | "kbh"
        | "fb" | "zl" | "ig" | "tt" | "ytb" | "acc" | "pass" | "otp"
        | "link" | "web" | "app" | "bot" | "game" | "team" | "room" | "lag" | "ping"
        | "tks" | "thx" | "pls" | "plz" | "sr" | "sry"
        | "test" | "tests" | "code" | "case" | "fix" | "bug" | "bugs" | "log" | "logs"
        | "dev" | "check" | "file" | "load" | "view" | "set" | "get" | "post" | "run"
        | "task" | "tasks" | "live" | "call" | "chat" | "mail" | "server" | "client"
        | "data" | "demo" | "tool" | "tools" | "page" | "site" | "shop" | "deal"
        | "order" | "bill" | "ship" | "sale" | "top" | "rank" | "rate" | "review"
        | "clip" | "video" | "sound" | "audio" | "trend" | "fan" | "idol" | "show"
        | "stream" | "drop" | "feed" | "story" | "vibe" | "mood" | "safe" | "fake"
        | "auth" | "token" | "key" | "id" | "user" | "admin" | "pin" | "slot"
    )
}

/// Evaluates whether a word finished in Vietnamese mode should be restored to raw English keys.
pub fn should_restore_to_english(
    raw_word: &str,
    composed_vn_word: &str,
    prev_word: Option<&str>,
    vn_phonologically_valid: bool,
) -> bool {
    // Word length check: single letters don't auto-switch
    if raw_word.len() < 2 {
        return false;
    }

    let raw_lower = raw_word.to_ascii_lowercase();

    // Guard against Vietnamese abbreviations and youth chat slang (e.g. ok, ht, nn, dc, vs, ae)
    if is_vietnamese_abbrev_or_slang(&raw_lower) {
        return false;
    }

    // A word with no vowels (a, e, i, o, u, y) is a consonant abbreviation, not a natural English word.
    let has_vowel = raw_lower.chars().any(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y'));
    if !has_vowel {
        return false;
    }

    // Acronym guard (D23, D42): words with 2+ capitals (USA, EST, HDD, camelCase) stay as typed (restore to English)
    let capital_count = raw_word.chars().filter(|c| c.is_ascii_uppercase()).count();
    if capital_count >= 2 {
        let all_caps = capital_count == raw_word.len();
        if !all_caps {
            return true; // camelCase / PascalCase like getUserId stays as typed
        }
        // If all caps, check if it has explicit Telex markings (e.g. TOOI, VIEETJ, THUOWNGF)
        let is_telex_marked = raw_word.len() >= 4 && {
            let lower = raw_word.to_ascii_lowercase();
            let b = lower.as_bytes();
            b.windows(2).any(|w| {
                let (c1, c2) = (w[0] as char, w[1] as char);
                (c1 == c2 && matches!(c1, 'a' | 'e' | 'o' | 'd'))
                    || (c2 == 'w' && matches!(c1, 'a' | 'o' | 'u'))
            })
        };
        if !is_telex_marked {
            return true; // Acronym like USA, EST, NASA, HTML restores to English
        }
    }

    let en_score = match lookup_en_score(&raw_lower) {
        Some(s) => s,
        None => return false, // Not a known English word
    };

    let vn_key = to_telex_normalized_key(composed_vn_word);
    let vn_score = vn_key.as_deref().and_then(lookup_vn_score);

    // If it's not a valid Vietnamese word at all, it's definitely English
    let vn = match vn_score {
        Some(s) => s,
        None => return true,
    };

    // Tie-breaker with Bigram vote if prev_word exists
    let pair_vote = if let Some(prev) = prev_word {
        bigram_vote(prev, composed_vn_word, raw_word)
    } else {
        0
    };

    // If bigram pair vote strongly favors Vietnamese, keep Vietnamese
    if pair_vote >= AS_PAIR_OVERRIDES_D22 {
        return false;
    }

    // Decision scoring formula:
    // If bigram vote exists: en - vn - vote >= AS_PREFER && (vn < AS_VN_FLOOR || vote <= -AS_PREFER)
    if pair_vote != 0 {
        return (en_score as i16 - vn as i16 - pair_vote as i16 >= AS_PREFER as i16)
            && (vn < AS_VN_FLOOR || pair_vote <= -AS_PREFER);
    }

    // Standard unigram tie formula:
    // vn < AS_VN_FLOOR && en - vn > AS_PREFER || en - vn >= AS_VN_EXTREME.
    // A bare tie (margin exactly AS_PREFER) keeps Vietnamese only when the
    // composition is phonologically valid and the corpus does not contradict
    // it (vn >= 0), mirroring the E->V side where ties switch to Vietnamese.
    // E.g. "oops" (en=10, vn=0 for valid "ốp") stays Vietnamese, while
    // "loss" (invalid "los") or "sos" (vn=-10 for "só") still restore English.
    let margin = en_score as i16 - vn as i16;
    (vn < AS_VN_FLOOR && margin > AS_PREFER as i16)
        || (vn < AS_VN_FLOOR
            && margin == AS_PREFER as i16
            && (!vn_phonologically_valid || vn < 0))
        || (margin >= AS_VN_EXTREME as i16)
}

/// Evaluates whether a raw word typed in English mode should be converted to Vietnamese and switch mode.
pub fn should_switch_to_vietnamese(
    raw_word: &str,
    composed_vn_word: &str,
    prev_word: Option<&str>,
) -> bool {
    if raw_word.len() < 2 || raw_word.len() > 32 {
        return false;
    }

    // Must have transformed into a different string (i.e. contains Vietnamese marks)
    if raw_word == composed_vn_word {
        return false;
    }

    // Acronym / identifier check: words with 2+ capitals stay as typed unless all caps
    let capital_count = raw_word.chars().filter(|c| c.is_ascii_uppercase()).count();
    let all_caps = capital_count == raw_word.len();
    if capital_count >= 2 && !all_caps {
        return false;
    }

    let vn_key = match to_telex_normalized_key(composed_vn_word) {
        Some(k) => k,
        None => return false,
    };

    let vn_score = lookup_vn_score(&vn_key);
    let en_score = lookup_en_score(&raw_word.to_ascii_lowercase());

    // All-caps word (e.g. THUOWNGF, VIEETJ): converts only to common Vietnamese syllable and not an English acronym
    if all_caps && (en_score.is_some() || vn_score.unwrap_or(AS_VN_FLOOR) < 15) {
        return false;
    }

    match en_score {
        None => true, // Raw keys are not English, and composed word is valid Vietnamese! E.g. "truowfng", "vieetj", "ddej", "dduowcj"
        Some(en) => {
            let vn = match vn_score {
                Some(s) => s,
                None => return false,
            };

            let pair_vote = if let Some(prev) = prev_word {
                bigram_vote(prev, composed_vn_word, raw_word)
            } else {
                0
            };

            // Decision 22: A clearly English neighbor guards against accidental conversion
            // (e.g. "video call cos", "the theme of rust" stay English).
            let prev_is_clearly_english = if let Some(prev) = prev_word {
                let pen = lookup_en_score(&prev.to_ascii_lowercase());
                let pvn_key = to_telex_normalized_key(prev);
                let pvn = pvn_key.as_deref().and_then(lookup_vn_score);
                match (pen, pvn) {
                    (Some(pe), Some(pv)) => pe as i16 - pv as i16 >= AS_PREFER as i16,
                    (Some(_), None) => true,
                    _ => false,
                }
            } else {
                false
            };

            if prev_is_clearly_english && pair_vote < AS_PAIR_OVERRIDES_D22 {
                return false;
            }

            // Ratio >= 10x (AS_PREFER) decides (e.g. "cos" -> "có", "nos" -> "nó", "toots" -> "tốt", "sex" -> "sẽ")
            vn as i16 - en as i16 + pair_vote as i16 >= AS_PREFER as i16
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_scores() {
        assert!(lookup_en_score("more").is_some());
        assert!(lookup_en_score("makes").is_some());
        assert!(lookup_en_score("sense").is_some());
        assert!(lookup_en_score("treen").is_none());

        assert!(lookup_vn_score("treen").is_some());
        assert!(lookup_vn_score("truwowngf").is_some());
        assert!(lookup_vn_score("chaof").is_some());
        assert!(lookup_vn_score("dduwowcj").is_some());
        assert!(lookup_vn_score("lieeuj").is_some());
        assert!(lookup_vn_score("more").is_none());
    }

    #[test]
    fn test_to_telex_normalized_key() {
        assert_eq!(to_telex_normalized_key("trên").unwrap(), "treen");
        assert_eq!(to_telex_normalized_key("trường").unwrap(), "truwowngf");
        assert_eq!(to_telex_normalized_key("hoà").unwrap(), "hoaf");
        assert_eq!(to_telex_normalized_key("hòa").unwrap(), "hoaf");
        assert_eq!(to_telex_normalized_key("được").unwrap(), "dduwowcj");
        assert_eq!(to_telex_normalized_key("dậy").unwrap(), "daayj");
        assert!(lookup_vn_score("daayj").is_some());
    }

    #[test]
    fn test_tie_decisions() {
        // "more" typed in VN mode -> "mỏe" (invalid/missing VN score) -> should restore
        assert!(should_restore_to_english("more", "mỏe", None, true));

        // "this" typed in VN mode -> "thí"
        // en_score = 39, vn_score = 18. en - vn = 21 >= 10, but vn (18) >= AS_VN_FLOOR (10).
        // Without prev word hint, "thí" stays VN.
        assert!(!should_restore_to_english("this", "thí", None, true));

        // Bare-tie gate: "oops" (en=10, vn=0) keeps valid "ốp"; the same tie
        // with an engine-rejected composition still restores English.
        assert!(!should_restore_to_english("oops", "ốp", None, true));
        assert!(should_restore_to_english("oops", "ốp", None, false));
        assert!(should_restore_to_english("loss", "los", None, false));

        // "bus" vs "bú" with prev word
        // "xe bus" -> should be English
        let vote_bus = bigram_vote("xe", "bú", "bus");
        assert!(vote_bus < 0, "xe bus should favor bus");

        // "cho con bú" -> should be Vietnamese
        let vote_bu = bigram_vote("con", "bú", "bus");
        assert!(vote_bu > 0, "con bú should favor bú");
    }

    #[test]
    fn test_switch_to_vietnamese() {
        // "truowfng" -> "trường" (valid VN, invalid EN) -> should switch
        assert!(should_switch_to_vietnamese("truowfng", "trường", None));
        // "treen" -> "trên" -> should switch
        assert!(should_switch_to_vietnamese("treen", "trên", None));
        // "vieetj" -> "việt" -> should switch
        assert!(should_switch_to_vietnamese("vieetj", "việt", None));

        // "sorry" -> "sorry" (no change) -> should NOT switch
        assert!(!should_switch_to_vietnamese("sorry", "sorry", None));
        // "this" -> "thí" (valid EN with score 39 >= 10) -> should NOT switch without strong VN context
        assert!(!should_switch_to_vietnamese("this", "thí", None));
        // "user" -> "user" (valid EN) -> should NOT switch
        assert!(!should_switch_to_vietnamese("user", "user", None));
    }

    #[test]
    fn test_vietnamese_abbreviations_do_not_switch_to_english() {
        for abbrev in &[
            "ok", "oke", "oki", "ht", "nn", "dc", "vs", "mk", "ae", "ko", "k", "cx", "bt", "stt",
            "uk", "uh", "wa", "thui", "lun", "khum", "mlem", "ultr", "gato", "xu", "ib", "rep",
            "tym", "ny", "ck", "vk", "game", "link", "web", "app", "lag", "ping", "zui", "bik"
        ] {
            assert!(
                !should_restore_to_english(abbrev, abbrev, None, false),
                "Abbreviation {abbrev} must not trigger restore/switch to English"
            );
        }
    }
}
