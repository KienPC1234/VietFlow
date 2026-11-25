// VietFlow Core - Main Input Engine
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::charset::encode_slot;
use crate::english::{is_english_keyword, is_foreign_initial};
use crate::input_proc::{
    get_char_type, get_modify_key, get_tone_key, is_ascii_upper, is_letter_fast,
    is_word_break_fast, to_lower_ascii, to_vn_lexi_tone,
};
use crate::key_text::KeyText;
use crate::macro_table::MacroTable;
use crate::models::{
    CharType, CharsetId, EngineOptions, ImeAction, InputMethod, ModifyKeyType, ToneKeyType,
    VnLexiName, VnLexiTone, WordForm,
};
use crate::tables::{iso_to_vn_lexi, IS_VN_VOWEL, TOTAL_VN_CHARS};
use crate::tone_rules::{
    can_vowel_seq_take_coda, get_tone_position, is_stop_coda, is_valid_ending_consonant,
    is_valid_initial, is_valid_initial_with_vowel, is_valid_vowel_seq, normalize_vowel,
};

#[derive(Debug, Clone, Copy)]
pub struct Slot {
    pub raw_char: char,
    pub lexi: VnLexiName,
    pub tone: VnLexiTone,
    pub char_type: CharType,
    pub is_upper: bool,
    pub modified: bool,
}

impl Slot {
    pub fn new(raw_char: char) -> Self {
        let is_upper = is_ascii_upper(raw_char);
        let lower = to_lower_ascii(raw_char);
        let lexi = iso_to_vn_lexi(lower);
        Self {
            raw_char,
            lexi: if is_upper && lexi != VnLexiName::NonVnChar {
                lexi.to_upper()
            } else {
                lexi
            },
            tone: VnLexiTone::None,
            char_type: CharType::Letter,
            is_upper,
            modified: false,
        }
    }
}

pub struct VietFlowEngine {
    pub options: EngineOptions,
    pub charset: CharsetId,
    pub macros: MacroTable,

    buffer: Vec<Slot>,
    raw_chars: Vec<char>,
    word_form: WordForm,
    current_tone: VnLexiTone,
    has_ending_consonant: bool,
    display_len: usize,

    vowel_start: usize,
    vowel_count: usize,

    display_shadow: Vec<char>,
    last_key_time_ms: u64,
    pub temp_disabled: bool,
    pub lookback: crate::autoswitch::LookbackBuffer,
    pub auto_switch_requested: Option<bool>,
    pub pending_doubled_fix: bool,
}

impl Default for VietFlowEngine {
    fn default() -> Self {
        Self::new(EngineOptions::default(), CharsetId::Unicode)
    }
}

impl VietFlowEngine {
    pub fn new(mut options: EngineOptions, charset: CharsetId) -> Self {
        options.normalize_keywords();
        Self {
            options,
            charset,
            macros: MacroTable::default(),
            buffer: Vec::with_capacity(32),
            raw_chars: Vec::with_capacity(32),
            word_form: WordForm::Empty,
            current_tone: VnLexiTone::None,
            has_ending_consonant: false,
            display_len: 0,
            vowel_start: 0,
            vowel_count: 0,
            display_shadow: Vec::with_capacity(32),
            last_key_time_ms: 0,
            temp_disabled: false,
            lookback: crate::autoswitch::LookbackBuffer::new(),
            auto_switch_requested: None,
            pending_doubled_fix: false,
        }
    }

    pub fn reset(&mut self) {
        self.reset_word();
        self.display_shadow.clear();
        self.lookback.clear();
        self.auto_switch_requested = None;
        self.pending_doubled_fix = false;
    }

    /// Returns true if the internal syllable buffer is empty (no active composition).
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Returns the exact word string currently tracked in display shadow.
    pub fn current_word(&self) -> String {
        self.display_shadow.iter().collect()
    }

    /// Returns the exact preedit text of the current word being composed without heap allocation.
    pub fn current_preedit_key_text(&self) -> KeyText {
        if !self.buffer.is_empty() {
            self.encode_range(0, self.buffer.len())
        } else {
            let mut out = KeyText::new();
            for &c in &self.display_shadow {
                out.push_char(c);
            }
            out
        }
    }

    /// Returns the exact preedit text of the current word being composed.
    /// If the syllable buffer has valid slots, it encodes them to the configured charset.
    /// Falls back to display_shadow if the syllable buffer is empty.
    pub fn current_preedit_text(&self) -> String {
        self.current_preedit_key_text().as_str().to_owned()
    }

    /// Evaluates word-break actions (macro expansion, AutoSwitch, English auto-restore) for IME preedit.
    /// Returns the finalized `KeyText` without heap allocation for normal words.
    /// Resets the engine word buffer in all cases.
    pub fn finish_composition_on_break_key_text(&mut self, _break_char: char) -> KeyText {
        self.auto_switch_requested = None;

        // 1. Check macro expansion
        if self.options.enable_macro
            && !self.macros.is_empty()
            && (!self.buffer.is_empty() || !self.raw_chars.is_empty())
        {
            if let Some(replacement) = self.lookup_raw_macro() {
                self.reset_word();
                return KeyText::from_str(&replacement);
            }
        }

        let mut raw_word = KeyText::new();
        let mut raw_lower = KeyText::new();
        for &rc in &self.raw_chars {
            raw_word.push_char(rc);
            raw_lower.push_char(to_lower_ascii(rc));
        }
        let display_word = self.current_preedit_key_text();

        // 2. Unified AutoSwitch disambiguation (unigram, bigram, doubled-key undo, camelCase/acronyms)
        if self.options.auto_switch_by_typing && !self.raw_chars.is_empty() {
            if self.pending_doubled_fix {
                self.pending_doubled_fix = false;
                if self.raw_chars.len() >= 3 {
                    let trimmed_chars = self.raw_chars.len() - 1;
                    let mut trimmed_raw = KeyText::new();
                    let mut trimmed_lower = KeyText::new();
                    for &rc in &self.raw_chars[..trimmed_chars] {
                        trimmed_raw.push_char(rc);
                        trimmed_lower.push_char(to_lower_ascii(rc));
                    }
                    let full_en = crate::autoswitch::lookup_en_score(raw_lower.as_str()).is_some();
                    let trimmed_en =
                        crate::autoswitch::lookup_en_score(trimmed_lower.as_str()).is_some();
                    if !full_en && trimmed_en {
                        self.auto_switch_requested = Some(false);
                        self.lookback
                            .push_word(trimmed_raw.as_str(), trimmed_raw.as_str(), true);
                        self.reset_word();
                        return trimmed_raw;
                    }
                }
            }

            let is_slang = crate::autoswitch::is_vietnamese_abbrev_or_slang(raw_lower.as_str());
            let prev_disp = self.lookback.prev_display();
            let should_switch = !is_slang
                && crate::autoswitch::should_restore_to_english(
                    raw_word.as_str(),
                    display_word.as_str(),
                    prev_disp,
                    self.is_valid_vietnamese_word(),
                );
            let is_english_keyword = !is_slang && self.matches_english_keyword(raw_lower.as_str());

            if should_switch || is_english_keyword {
                let is_already_raw = display_word.as_str() == raw_word.as_str();
                self.auto_switch_requested = if is_already_raw { None } else { Some(false) };
                self.lookback
                    .push_word(raw_word.as_str(), raw_word.as_str(), true);
                self.reset_word();
                return raw_word;
            }

            let is_english_word =
                !is_slang && crate::autoswitch::lookup_en_score(raw_lower.as_str()).is_some();
            self.lookback.push_word(
                raw_word.as_str(),
                display_word.as_str(),
                is_english_word && display_word.as_str() == raw_word.as_str(),
            );
        }

        // 3. Spell-check & English auto-restore on invalid Vietnamese syllable
        let is_slang = crate::autoswitch::is_vietnamese_abbrev_or_slang(raw_lower.as_str());
        let should_restore = self.options.spell_check
            && self.options.auto_non_vn_restore
            && !self.buffer.is_empty()
            && (self.matches_english_keyword(raw_lower.as_str())
                || (is_slang && display_word.as_str() != raw_word.as_str())
                || (!self.temp_disabled
                    && self.has_any_vietnamese_mark()
                    && !self.is_valid_vietnamese_word()));

        if should_restore {
            self.reset_word();
            return raw_word;
        }

        self.reset_word();
        display_word
    }

    /// Evaluates word-break actions (macro expansion, English auto-restore) for IME preedit.
    /// Returns the finalized string (expanded macro, restored raw word, or formatted composed word).
    /// Resets the engine word buffer in all cases.
    pub fn finish_composition_on_break(&mut self, break_char: char) -> String {
        self.finish_composition_on_break_key_text(break_char)
            .as_str()
            .to_owned()
    }

    /// Unified single-pass IME handler for preedit hosts (Linux Fcitx5, Windows TSF).
    /// Determines inside the core whether a key starts/updates composition, commits, or passes through,
    /// so platform hosts never need to duplicate method/VIQR key classification.
    pub fn process_ime_key(&mut self, c: char, now_ms: u64) -> (ImeAction, KeyText) {
        self.auto_switch_requested = None;
        if now_ms > 0 && self.options.typing_timeout_ms > 0 {
            if self.last_key_time_ms > 0
                && now_ms.saturating_sub(self.last_key_time_ms) > self.options.typing_timeout_ms
            {
                self.reset();
            }
            self.last_key_time_ms = now_ms;
        }

        if c == '\x08' {
            if self.buffer.is_empty() && self.raw_chars.is_empty() {
                return (ImeAction::PassThrough, KeyText::new());
            }
            let _ = self.process_backspace();
            self.display_shadow.clear();
            return (ImeAction::UpdatePreedit, self.current_preedit_key_text());
        }

        if matches!(c, '\r' | '\n' | '\t') {
            if self.buffer.is_empty() && self.raw_chars.is_empty() {
                return (ImeAction::PassThrough, KeyText::new());
            }
            let finalized = self.finish_composition_on_break_key_text(c);
            return (ImeAction::CommitAndForward, finalized);
        }

        if c == '\x1b' {
            if self.buffer.is_empty() && self.raw_chars.is_empty() {
                return (ImeAction::PassThrough, KeyText::new());
            }
            let current = self.current_preedit_key_text();
            self.reset_word();
            return (ImeAction::CommitAndConsume, current);
        }

        if self.buffer.is_empty() {
            self.temp_disabled = false;
            if self.options.method == InputMethod::Telex && matches!(c, '[' | ']' | '{' | '}') {
                let _ = self.try_bracket_shortcut(c);
                return (ImeAction::UpdatePreedit, self.current_preedit_key_text());
            }
            if self.starts_lone_w(c) {
                let _ = self.process_lone_w(c);
                return (ImeAction::UpdatePreedit, self.current_preedit_key_text());
            }
            if is_letter_fast(c) {
                let _ = self.process_first_letter(c);
                return (ImeAction::UpdatePreedit, self.current_preedit_key_text());
            }
            return (ImeAction::PassThrough, KeyText::new());
        }

        // Double-bracket undo in Telex (e.g. "[[" -> "[", "a[[" -> "a[")
        if self.options.method == InputMethod::Telex
            && !self.temp_disabled
            && matches!(c, '[' | ']' | '{' | '}')
        {
            let last_raw = self.raw_chars.last().copied();
            let last_is_bracket = self
                .buffer
                .last()
                .map(|slot| slot.raw_char == c && slot.modified)
                .unwrap_or(false);
            if last_raw == Some(c) && last_is_bracket {
                let mut out = self.encode_range(0, self.buffer.len() - 1);
                out.push_char(c);
                self.reset_word();
                return (ImeAction::CommitAndConsume, out);
            }
        }

        if self.try_process_in_word_key(c).is_some() {
            return (ImeAction::UpdatePreedit, self.current_preedit_key_text());
        }

        let mut finalized = self.finish_composition_on_break_key_text(c);
        finalized.push_char(c);
        (ImeAction::CommitAndConsume, finalized)
    }

    pub fn reset_word(&mut self) {
        self.buffer.clear();
        self.raw_chars.clear();
        self.word_form = WordForm::Empty;
        self.current_tone = VnLexiTone::None;
        self.has_ending_consonant = false;
        self.display_len = 0;
        self.vowel_start = 0;
        self.vowel_count = 0;
        self.display_shadow.clear();
        self.temp_disabled = false;
        self.pending_doubled_fix = false;
    }

    pub fn process_key(&mut self, c: char) -> (usize, KeyText) {
        self.process_key_with_time(c, 0)
    }

    pub fn process_key_with_time(&mut self, c: char, now_ms: u64) -> (usize, KeyText) {
        self.auto_switch_requested = None;
        if now_ms > 0 && self.options.typing_timeout_ms > 0 {
            if self.last_key_time_ms > 0
                && now_ms.saturating_sub(self.last_key_time_ms) > self.options.typing_timeout_ms
            {
                self.reset();
            }
            self.last_key_time_ms = now_ms;
        }

        let (raw_bk, out) = self.process_key_raw(c);
        self.diff_engine_output(raw_bk, out)
    }

    fn diff_engine_output(&mut self, raw_bk: usize, out: KeyText) -> (usize, KeyText) {
        if raw_bk == 0 && out.is_empty() {
            return (0, out);
        }

        let contains_break = out
            .chars()
            .any(|c| c.is_whitespace() || c.is_ascii_punctuation() || c.is_ascii_digit());

        let shadow_len = self.display_shadow.len();
        if contains_break || raw_bk > shadow_len {
            self.display_shadow.clear();
            if !contains_break {
                for c in out.chars() {
                    if self.display_shadow.len() < 128 {
                        self.display_shadow.push(c);
                    } else {
                        self.display_shadow.clear();
                        break;
                    }
                }
            }
            return (raw_bk, out);
        }

        let prefix_len = shadow_len - raw_bk;
        let mut common = prefix_len;
        let mut suffix_offset = 0;

        for (offset, c) in out.char_indices() {
            if common < shadow_len && self.display_shadow[common] == c {
                common += 1;
                suffix_offset = offset + c.len_utf8();
            } else {
                suffix_offset = offset;
                break;
            }
        }

        let diff_bk = shadow_len - common;
        let suffix = &out.as_str()[suffix_offset..];

        self.display_shadow.truncate(prefix_len);
        for c in out.chars() {
            if self.display_shadow.len() < 128 {
                self.display_shadow.push(c);
            } else {
                self.display_shadow.clear();
                break;
            }
        }

        (diff_bk, KeyText::from_str(suffix))
    }

    #[inline]
    fn starts_lone_w(&self, c: char) -> bool {
        matches!(
            self.options.method,
            InputMethod::Telex | InputMethod::TelexVni
        ) && matches!(c, 'w' | 'W')
    }

    fn process_key_raw(&mut self, c: char) -> (usize, KeyText) {
        if c == '\x08' {
            return self.process_backspace();
        }

        // Telex bracket shortcuts: [ = ơ, ] = ư, { = Ơ, } = Ư (Telex only)
        if self.options.method == InputMethod::Telex && !self.temp_disabled {
            if let Some(res) = self.try_bracket_shortcut(c) {
                return res;
            }
        }

        if self.buffer.is_empty() {
            self.temp_disabled = false;

            if self.starts_lone_w(c) {
                return self.process_lone_w(c);
            }

            if is_letter_fast(c) {
                return self.process_first_letter(c);
            }

            return (0, KeyText::from_char(c));
        }

        if let Some(res) = self.try_process_in_word_key(c) {
            return res;
        }

        let char_type = get_char_type(c, self.options.method);
        if matches!(char_type, CharType::WordBreak | CharType::Reset) || is_word_break_fast(c) {
            return self.process_word_break(c);
        }

        self.process_word_break(c)
    }

    /// Attempts to apply `c` to the current non-empty syllable (bracket shortcut,
    /// tone mark, vowel/consonant modifier, lone-w, or subsequent letter).
    /// Returns `None` when `c` breaks the syllable.
    fn try_process_in_word_key(&mut self, c: char) -> Option<(usize, KeyText)> {
        if self.options.method == InputMethod::Telex && !self.temp_disabled {
            if let Some(res) = self.try_bracket_shortcut(c) {
                return Some(res);
            }
        }

        // When temporary key disable is active (e.g. after tone/accent undo or foreign word restoration),
        // let all subsequent alphabetic letters pass through without forming Vietnamese marks.
        if self.temp_disabled {
            if is_letter_fast(c) {
                return Some(self.append_non_vn_letter(c));
            }
            return None;
        }

        // Try tone key
        let tone_key = get_tone_key(c, self.options.method);
        if tone_key != ToneKeyType::None {
            if let Some(res) = self.try_apply_tone(tone_key, c) {
                self.raw_chars.push(c);
                return Some(res);
            }
        }

        // Try modify key
        let modify_key = get_modify_key(c, self.options.method);
        if modify_key != ModifyKeyType::None || self.is_double_key_modify(c) {
            if let Some(res) = self.try_apply_modify(modify_key, c) {
                self.raw_chars.push(c);
                return Some(res);
            }
        }

        // Lone 'w' -> 'ư' fallback (Telex-family). Handled here (not inside
        // try_apply_modify) so raw_chars is pushed exactly once.
        {
            let lower = to_lower_ascii(c);
            let is_telex_w = matches!(
                self.options.method,
                InputMethod::Telex
                    | InputMethod::SimpleTelex
                    | InputMethod::TelexSimple2
                    | InputMethod::TelexVni
            ) && lower == 'w';
            if is_telex_w {
                let is_lone_w_method = matches!(
                    self.options.method,
                    InputMethod::Telex | InputMethod::SimpleTelex | InputMethod::TelexVni
                );
                if is_lone_w_method
                    && self.word_form != WordForm::NonVn
                    && !self.buffer.iter().any(|s| to_lower_ascii(s.raw_char) == 'w')
                    && (self.vowel_count == 0
                        || self.options.free_marking
                        || self.last_vowel_is_current())
                {
                    return Some(self.process_lone_w(c));
                }
            }
        }

        // Subsequent letters
        if is_letter_fast(c) {
            return Some(self.process_subsequent_letter(c));
        }

        None
    }

    fn try_bracket_shortcut(&mut self, c: char) -> Option<(usize, KeyText)> {
        if !matches!(c, '[' | ']' | '{' | '}') {
            return None;
        }

        // Double bracket to undo: [ -> ơ, [[ -> [, etc.
        let last_raw = self.raw_chars.last().copied();
        let last_is_bracket = self
            .buffer
            .last()
            .map(|slot| slot.raw_char == c && slot.modified)
            .unwrap_or(false);
        if last_raw == Some(c) && last_is_bracket {
            self.reset_word();
            return Some((1, KeyText::from_char(c)));
        }

        match c {
            '[' => Some(self.append_mapped_letter(c, VnLexiName::oh, false)),
            '{' => Some(self.append_mapped_letter(c, VnLexiName::Oh, true)),
            ']' => Some(self.append_mapped_letter(c, VnLexiName::uh, false)),
            '}' => Some(self.append_mapped_letter(c, VnLexiName::Uh, true)),
            _ => None,
        }
    }

    fn append_mapped_letter(&mut self, raw_key: char, lexi: VnLexiName, is_upper: bool) -> (usize, KeyText) {
        self.raw_chars.push(raw_key);

        let mut slot = Slot::new(raw_key);
        slot.lexi = lexi;
        slot.is_upper = is_upper;
        slot.modified = true;
        self.buffer.push(slot);

        if self.buffer.len() == 1 {
            let is_vowel = is_vowel_lexi(lexi);
            self.word_form = if is_vowel { WordForm::V } else { WordForm::C };
            self.has_ending_consonant = false;
            self.recalculate_vowel_state();

            let out = self.encode_slot_str(&slot);
            self.display_len = out.chars().count();
            return (0, out);
        }

        self.recalculate_vowel_state();
        let is_vowel = is_vowel_lexi(lexi);
        self.update_form_after_append(is_vowel);

        if self.current_tone != VnLexiTone::None
            && self.word_form != WordForm::NonVn
            && self.refresh_tone_position()
        {
            return self.rebuild_from_index(self.vowel_start);
        }

        let out = self.encode_slot_str(&slot);
        self.display_len += out.chars().count();
        (0, out)
    }

    fn process_first_letter(&mut self, c: char) -> (usize, KeyText) {
        self.raw_chars.push(c);

        let slot = Slot::new(c);
        let is_vowel = is_vowel_lexi(slot.lexi);
        self.buffer.push(slot);

        self.word_form = if is_vowel { WordForm::V } else { WordForm::C };
        self.has_ending_consonant = false;
        self.recalculate_vowel_state();

        let out = self.encode_slot_str(&slot);
        self.display_len = out.chars().count();
        (0, out)
    }

    fn process_lone_w(&mut self, c: char) -> (usize, KeyText) {
        self.raw_chars.push(c);
        let is_upper = is_ascii_upper(c);

        let mut slot = Slot::new(c);
        slot.lexi = if is_upper { VnLexiName::Uh } else { VnLexiName::uh };
        slot.is_upper = is_upper;
        slot.modified = true;
        // Slot is Copy: retain output state before potential buffer rebuilds.
        let pushed_slot = slot;
        self.buffer.push(slot);

        if self.buffer.len() == 1 {
            self.word_form = WordForm::V;
            self.has_ending_consonant = false;
            self.recalculate_vowel_state();

            let out = self.encode_slot_str(&pushed_slot);
            self.display_len += out.chars().count();
            return (0, out);
        }

        self.recalculate_vowel_state();
        let vowel_after_coda = self.update_form_after_append(true);

        if vowel_after_coda && self.options.spell_check && self.options.auto_non_vn_restore && self.has_any_vietnamese_mark() {
            return self.restore_raw_buffer_with_new_char(c);
        }

        if self.current_tone != VnLexiTone::None
            && self.word_form != WordForm::NonVn
            && self.refresh_tone_position()
        {
            return self.rebuild_from_index(self.vowel_start);
        }

        let out = self.encode_slot_str(&pushed_slot);
        self.display_len += out.chars().count();
        (0, out)
    }

    fn process_subsequent_letter(&mut self, c: char) -> (usize, KeyText) {
        self.raw_chars.push(c);

        let vowel_start = self.get_vowel_start();
        let is_upper = is_ascii_upper(c);
        let lower = to_lower_ascii(c);
        let lexi = iso_to_vn_lexi(lower);

        if lexi == VnLexiName::NonVnChar {
            return self.append_non_vn_letter(c);
        }

        let is_vowel = is_vowel_lexi(lexi) && self.buffer.len() >= vowel_start;

        if is_vowel && self.vowel_count > 0 && self.vowel_start < vowel_start {
            self.clear_vowels();
        }

        let slot = Slot::new(c);
        self.buffer.push(slot);

        if is_vowel {
            self.add_vowel(self.buffer.len() - 1, lexi);
            if self.buffer.len() == 2
                && to_lower_ascii(self.buffer[0].raw_char) == 'q'
                && to_lower_ascii(self.buffer[1].raw_char) == 'u'
            {
                self.clear_vowels();
            }
        }

        let vowel_after_coda = self.update_form_after_append(is_vowel);

        let mut upgrade_from = None;
        if !is_vowel && self.word_form != WordForm::NonVn && self.vowel_count == 2 {
            upgrade_from = self.try_upgrade_uo_on_coda();
        }

        if vowel_after_coda && self.options.spell_check && self.options.auto_non_vn_restore && self.has_any_vietnamese_mark() {
            return self.restore_raw_buffer_with_new_char(c);
        }

        // Phonotactic rule 1: Stop codas (c, ch, p, t) in Vietnamese can only take Acute or Dot tones.
        // If appending this coda creates an impossible tone combination (e.g. "tẽ" + "t" in "texture"),
        // immediately restore raw keystrokes to prevent visible composition corruption.
        if !is_vowel && self.options.spell_check && self.options.auto_non_vn_restore && self.has_any_vietnamese_mark() {
            let ending_start = self.vowel_start + self.vowel_count;
            if ending_start < self.buffer.len() && self.vowel_count > 0 {
                let ending_len = self.buffer.len() - ending_start;
                if ending_len <= 2 {
                    let mut ending = ['\0'; 2];
                    for (i, slot) in self.buffer[ending_start..].iter().enumerate() {
                        ending[i] = to_lower_ascii(slot.raw_char);
                    }
                    if is_stop_coda(&ending[..ending_len])
                        && matches!(
                            self.current_tone,
                            VnLexiTone::Grave | VnLexiTone::Hook | VnLexiTone::Tilde
                        )
                    {
                        return self.restore_raw_buffer_with_new_char(c);
                    }
                }
            }
        }

        // Phonotactic rule 2: In Vietnamese, vowel 'u' followed by 'e' only exists after initial 'q'
        // (forming 'qu' + 'e'). Any syllable where 'e' follows 'u' without initial 'q' (e.g. -ure in
        // "pure", "cure", "sure", "texture", "picture", "future", or -ue in "blue") is English.
        // If a Vietnamese mark is present (such as hook 'r' on 'u'), restore immediately.
        if is_vowel && self.options.spell_check && self.options.auto_non_vn_restore && self.has_any_vietnamese_mark() {
            let lower_c = to_lower_ascii(c);
            if lower_c == 'e' && self.vowel_count >= 2 && self.buffer.len() >= 2 {
                let prev_vowel_idx = self.buffer.len() - 2;
                let prev_raw = to_lower_ascii(self.buffer[prev_vowel_idx].raw_char);
                let starts_with_qu = self.buffer.len() >= 3
                    && to_lower_ascii(self.buffer[0].raw_char) == 'q'
                    && to_lower_ascii(self.buffer[1].raw_char) == 'u';
                if prev_raw == 'u' && !starts_with_qu {
                    return self.restore_raw_buffer_with_new_char(c);
                }
            }
        }

        // Auto fix Shift key sticking on 2nd letter (e.g. "VIệt" -> "Việt")
        if self.options.fix_shift_lag
            && self.buffer.len() >= 3
            && self.buffer[0].is_upper
            && self.buffer[1].is_upper
            && !is_upper
        {
            self.buffer[1].is_upper = false;
            self.buffer[1].raw_char = to_lower_ascii(self.buffer[1].raw_char);
            self.buffer[1].lexi = self.buffer[1].lexi.to_lower();
            self.refresh_tone_position();
            return self.rebuild_current_word();
        }

        if let Some(up_idx) = upgrade_from {
            self.refresh_tone_position();
            return self.rebuild_from_index(up_idx);
        }

        if self.current_tone != VnLexiTone::None
            && self.word_form != WordForm::NonVn
            && self.refresh_tone_position()
        {
            return self.rebuild_from_index(self.vowel_start);
        }

        let out = match self.buffer.last() {
            Some(last) => self.encode_slot_str(last),
            None => KeyText::new(),
        };
        self.display_len += out.chars().count();
        (0, out)
    }

    fn append_non_vn_letter(&mut self, c: char) -> (usize, KeyText) {
        self.raw_chars.push(c);
        let slot = Slot::new(c);
        self.buffer.push(slot);
        self.word_form = WordForm::NonVn;
        self.recalculate_vowel_state();

        let out = self.encode_slot_str(&slot);
        self.display_len += out.chars().count();
        (0, out)
    }

    fn is_double_key_modify(&self, c: char) -> bool {
        if self.options.method == InputMethod::Viqr {
            return to_lower_ascii(c) == 'd';
        }
        let is_double_ok = matches!(
            self.options.method,
            InputMethod::Telex
                | InputMethod::SimpleTelex
                | InputMethod::TelexSimple2
                | InputMethod::TelexVni
        );
        if !is_double_ok {
            return false;
        }
        matches!(to_lower_ascii(c), 'a' | 'e' | 'o' | 'd')
    }

    fn try_apply_tone(&mut self, tone_key: ToneKeyType, _raw_key: char) -> Option<(usize, KeyText)> {
        let vowel_count = self.vowel_count;
        if vowel_count == 0 || self.word_form == WordForm::NonVn {
            return None;
        }

        if matches!(self.options.method, InputMethod::SimpleTelex | InputMethod::TelexSimple2)
            && to_lower_ascii(self.buffer[0].raw_char) == 'w'
        {
            return None;
        }

        if self.options.spell_check
            && is_foreign_initial(self.buffer[0].raw_char, self.buffer[0].lexi, &self.options.foreign_initials)
        {
            return None;
        }

        let first_vowel = self.vowel_start;
        let last_vowel = first_vowel + vowel_count - 1;
        let has_coda = last_vowel < self.buffer.len() - 1;

        let new_tone = to_vn_lexi_tone(tone_key);

        // Tone removal key ('z' or '0')
        if new_tone == VnLexiTone::None {
            self.current_tone = VnLexiTone::None;
            for i in 0..vowel_count {
                self.buffer[first_vowel + i].tone = VnLexiTone::None;
            }
            return Some(self.rebuild_from_index(first_vowel));
        }

        // If the syllable has a coda, validate it before allowing tone
        if has_coda {
            let coda = &self.buffer[last_vowel + 1..];
            let coda_len = coda.len();
            if coda_len <= 2 {
                let mut coda_chars = ['\0'; 2];
                for (i, slot) in coda.iter().enumerate() {
                    coda_chars[i] = to_lower_ascii(slot.raw_char);
                }
                if self.options.spell_check && !is_valid_ending_consonant(&coda_chars[..coda_len]) {
                    return None;
                }
                if is_stop_coda(&coda_chars[..coda_len])
                    && matches!(
                        new_tone,
                        VnLexiTone::Grave | VnLexiTone::Hook | VnLexiTone::Tilde
                    )
                {
                    return None;
                }
            } else if self.options.spell_check {
                return None;
            }
        }

        // Check for tone undo: restore the raw keystroke sequence.
        // Typing the tone key again cancels the tone mark, reverting the syllable
        // to its literal raw characters (e.g. "củ" + "r" -> "cur", "toán" + "s" -> "toans").
        if self.current_tone == new_tone {
            self.current_tone = VnLexiTone::None;
            self.temp_disabled = true;
            if self.options.auto_switch_by_typing {
                self.pending_doubled_fix = true;
                self.auto_switch_requested = Some(false);
            }
            let backspaces = self.display_len;
            let mut out = KeyText::new();
            for &raw in &self.raw_chars {
                out.push_char(raw);
            }

            self.buffer.clear();
            for &raw in &self.raw_chars {
                self.buffer.push(Slot::new(raw));
            }
            self.word_form = WordForm::NonVn;
            self.has_ending_consonant = false;
            self.clear_vowels();
            self.display_len = out.chars().count();
            return Some((backspaces, out));
        }

        self.current_tone = new_tone;

        let target_pos = self.tone_position_at(vowel_count, has_coda);

        for i in 0..vowel_count {
            self.buffer[first_vowel + i].tone = if i == target_pos {
                self.current_tone
            } else {
                VnLexiTone::None
            };
        }

        Some(self.rebuild_from_index(first_vowel))
    }

    fn try_apply_modify(
        &mut self,
        modify_key: ModifyKeyType,
        raw_key: char,
    ) -> Option<(usize, KeyText)> {
        let lower = to_lower_ascii(raw_key);

        if self.options.spell_check
            && is_foreign_initial(self.buffer[0].raw_char, self.buffer[0].lexi, &self.options.foreign_initials)
        {
            return None;
        }

        // Double key handling in Telex
        let is_double_ok = matches!(
            self.options.method,
            InputMethod::Telex
                | InputMethod::SimpleTelex
                | InputMethod::TelexSimple2
                | InputMethod::TelexVni
                | InputMethod::Viqr
        );

        if is_double_ok && self.word_form != WordForm::NonVn {
            if lower == 'd' {
                let last_idx = self.buffer.len().saturating_sub(1);
                let last_slot = &mut self.buffer[last_idx];
                if matches!(
                    last_slot.lexi,
                    VnLexiName::d | VnLexiName::D | VnLexiName::dd | VnLexiName::DD
                ) {
                    if last_slot.modified {
                        last_slot.lexi = if last_slot.is_upper { VnLexiName::D } else { VnLexiName::d };
                        last_slot.modified = false;
                        self.temp_disabled = true;
                        self.buffer.push(Slot::new(raw_key));
                        return Some(self.rebuild_current_word());
                    }

                    last_slot.lexi = if last_slot.is_upper { VnLexiName::DD } else { VnLexiName::dd };
                    last_slot.modified = true;
                    return Some(self.rebuild_current_word());
                } else if self.options.free_marking && !self.buffer.is_empty() {
                    // Free marking: 'd' typed after vowels with or without ending consonant (e.g. "di" + "d" -> "đi", "dem" + "d" -> "đem", "dang" + "d" -> "đang")
                    let first_is_d = matches!(self.buffer[0].lexi, VnLexiName::d | VnLexiName::D | VnLexiName::dd | VnLexiName::DD);
                    if first_is_d && self.vowel_count > 0 {
                        let ending_start = self.vowel_start + self.vowel_count;
                        let ending_len = self.buffer.len().saturating_sub(ending_start);
                        let coda_ok = if ending_len == 0 {
                            true
                        } else if ending_len <= 2 {
                            let mut coda = ['\0'; 2];
                            for (idx, slot) in self.buffer[ending_start..].iter().enumerate() {
                                coda[idx] = to_lower_ascii(slot.raw_char);
                            }
                            is_valid_ending_consonant(&coda[..ending_len])
                        } else {
                            false
                        };

                        let mut vowels = [VnLexiName::None; 3];
                        let v_count = self.vowel_count.min(3);
                        for (k, target_vowel) in vowels.iter_mut().enumerate().take(v_count) {
                            *target_vowel = self.buffer[self.vowel_start + k].lexi;
                        }
                        let vowel_seq_ok = is_valid_vowel_seq(&vowels[..v_count]);
                        let coda_allowed = if ending_len > 0 {
                            can_vowel_seq_take_coda(&vowels[..v_count])
                        } else {
                            true
                        };

                        if coda_ok && vowel_seq_ok && coda_allowed {
                            let slot = &mut self.buffer[0];
                            if slot.modified {
                                slot.lexi = if slot.is_upper { VnLexiName::D } else { VnLexiName::d };
                                slot.modified = false;
                                self.temp_disabled = true;
                                self.buffer.push(Slot::new(raw_key));
                                return Some(self.rebuild_current_word());
                            }
                            slot.lexi = if slot.is_upper { VnLexiName::DD } else { VnLexiName::dd };
                            slot.modified = true;
                            return Some(self.rebuild_current_word());
                        }
                    }
                }
            }

            if matches!(lower, 'a' | 'e' | 'o') && self.options.method != InputMethod::Viqr {
                let last_idx = self.buffer.len().saturating_sub(1);
                let last_slot = &mut self.buffer[last_idx];
                let last_slot_char = to_lower_ascii(last_slot.raw_char);

                if last_slot_char == lower {
                    let is_same_circumflex = last_slot.modified && matches!(
                        (lower, last_slot.lexi.to_lower()),
                        ('a', VnLexiName::ar) | ('e', VnLexiName::er) | ('o', VnLexiName::or)
                    );
                    if is_same_circumflex {
                        last_slot.lexi = iso_to_vn_lexi(last_slot_char);
                        last_slot.modified = false;
                        self.temp_disabled = true;
                        self.buffer.push(Slot::new(raw_key));
                        return Some(self.rebuild_current_word());
                    }

                    let circumflex = match lower {
                        'a' => if last_slot.is_upper { VnLexiName::Ar } else { VnLexiName::ar },
                        'e' => if last_slot.is_upper { VnLexiName::Er } else { VnLexiName::er },
                        'o' => if last_slot.is_upper { VnLexiName::Or } else { VnLexiName::or },
                        _ => last_slot.lexi,
                    };

                    last_slot.lexi = circumflex;
                    last_slot.modified = true;
                    self.refresh_tone_position();
                    if self.current_tone == VnLexiTone::None {
                        return Some(self.rebuild_from_index(last_idx));
                    }
                    return Some(self.rebuild_current_word());
                } else if self.options.free_marking && self.vowel_count > 0 {
                    // Free marking: 'a', 'e', or 'o' typed after coda or diphthongs/triphthongs (e.g. "them" + "e" -> "thêm", "lieu" + "e" -> "liêu", "ton" + "o" -> "tôn", "may" + "a" -> "mây")
                    let vowel_start = self.vowel_start;
                    let ending_start = vowel_start + self.vowel_count;
                    let ending_len = self.buffer.len().saturating_sub(ending_start);
                    if ending_len == 0 || ending_len <= 2 {
                        let mut coda = ['\0'; 2];
                        let coda_valid = if ending_len > 0 {
                            for (idx, slot) in self.buffer[ending_start..].iter().enumerate() {
                                coda[idx] = to_lower_ascii(slot.raw_char);
                            }
                            is_valid_ending_consonant(&coda[..ending_len])
                        } else {
                            true
                        };

                        if coda_valid {
                            for i in (vowel_start..ending_start).rev() {
                                let slot_char = to_lower_ascii(self.buffer[i].raw_char);
                                if slot_char == lower {
                                    let is_upper = self.buffer[i].is_upper;
                                    let is_same_circumflex = self.buffer[i].modified && matches!(
                                        (lower, self.buffer[i].lexi.to_lower()),
                                        ('a', VnLexiName::ar) | ('e', VnLexiName::er) | ('o', VnLexiName::or)
                                    );
                                    if is_same_circumflex {
                                        let slot = &mut self.buffer[i];
                                        slot.lexi = iso_to_vn_lexi(slot_char);
                                        slot.modified = false;
                                        self.temp_disabled = true;
                                        self.buffer.push(Slot::new(raw_key));
                                        return Some(self.rebuild_current_word());
                                    }

                                    let circumflex = match lower {
                                        'a' => if is_upper { VnLexiName::Ar } else { VnLexiName::ar },
                                        'e' => if is_upper { VnLexiName::Er } else { VnLexiName::er },
                                        'o' => if is_upper { VnLexiName::Or } else { VnLexiName::or },
                                        _ => self.buffer[i].lexi,
                                    };

                                    let mut candidate_vowels = [VnLexiName::None; 3];
                                    let v_count = self.vowel_count.min(3);
                                    for (k, target_vowel) in candidate_vowels.iter_mut().enumerate().take(v_count) {
                                        *target_vowel = if vowel_start + k == i {
                                            circumflex
                                        } else {
                                            self.buffer[vowel_start + k].lexi
                                        };
                                    }

                                    let seq_ok = is_valid_vowel_seq(&candidate_vowels[..v_count]);
                                    let coda_allowed = if ending_len > 0 {
                                        can_vowel_seq_take_coda(&candidate_vowels[..v_count])
                                    } else {
                                        true
                                    };

                                    if seq_ok && coda_allowed {
                                        let slot = &mut self.buffer[i];
                                        slot.lexi = circumflex;
                                        slot.modified = true;
                                        self.refresh_tone_position();
                                        return Some(self.rebuild_current_word());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 'w' key handling (Telex hook-all or custom hook-all)
        let is_telex_w = matches!(
            self.options.method,
            InputMethod::Telex
                | InputMethod::SimpleTelex
                | InputMethod::TelexSimple2
                | InputMethod::TelexVni
        ) && lower == 'w';

        if modify_key == ModifyKeyType::HookAll || is_telex_w {
            if self.vowel_count > 0 && !self.options.free_marking && !self.last_vowel_is_current() {
                return None;
            }

            if let Some(res) = self.apply_hook_all(raw_key) {
                return Some(res);
            }

            // Lone 'w' is handled in process_key_raw (outside try_apply_modify)
            // so that raw_chars is pushed exactly once (no outer double-push).
        }

        // VNI & VIQR & TelexVni number/symbol modifications
        if matches!(
            self.options.method,
            InputMethod::Vni | InputMethod::Viqr | InputMethod::TelexVni
        ) {
            if modify_key == ModifyKeyType::Horn {
                if self.vowel_count > 0 && !self.options.free_marking && !self.last_vowel_is_current() {
                    return None;
                }
                return self.apply_hook_all(raw_key);
            }

            let vowel_start = self.get_vowel_start();

            let last_idx = self.buffer.len().saturating_sub(1);
            match modify_key {
                ModifyKeyType::Circumflex => {
                    for i in (vowel_start..self.buffer.len()).rev() {
                        if !self.options.free_marking && i != last_idx {
                            return None;
                        }
                        let slot = &mut self.buffer[i];
                        let c = to_lower_ascii(slot.raw_char);
                        if matches!(c, 'a' | 'e' | 'o') {
                            if slot.modified && matches!(slot.lexi, VnLexiName::ar | VnLexiName::Ar | VnLexiName::er | VnLexiName::Er | VnLexiName::or | VnLexiName::Or) {
                                let plain = iso_to_vn_lexi(c);
                                slot.lexi = if slot.is_upper { plain.to_upper() } else { plain };
                                slot.modified = false;
                                self.temp_disabled = true;
                                self.buffer.push(Slot::new(raw_key));
                                return Some(self.rebuild_current_word());
                            }
                            slot.lexi = match c {
                                'a' => if slot.is_upper { VnLexiName::Ar } else { VnLexiName::ar },
                                'e' => if slot.is_upper { VnLexiName::Er } else { VnLexiName::er },
                                'o' => if slot.is_upper { VnLexiName::Or } else { VnLexiName::or },
                                _ => slot.lexi,
                            };
                            slot.modified = true;
                            self.refresh_tone_position();
                            return Some(self.rebuild_current_word());
                        }
                    }
                }
                ModifyKeyType::Breve => {
                    for i in (vowel_start..self.buffer.len()).rev() {
                        if !self.options.free_marking && i != last_idx {
                            return None;
                        }
                        let slot = &mut self.buffer[i];
                        if to_lower_ascii(slot.raw_char) == 'a' {
                            if slot.modified && matches!(slot.lexi, VnLexiName::ah | VnLexiName::Ah) {
                                slot.lexi = if slot.is_upper { VnLexiName::A } else { VnLexiName::a };
                                slot.modified = false;
                                self.buffer.push(Slot::new(raw_key));
                                return Some(self.rebuild_current_word());
                            }
                            slot.lexi = if slot.is_upper { VnLexiName::Ah } else { VnLexiName::ah };
                            slot.modified = true;
                            self.refresh_tone_position();
                            return Some(self.rebuild_current_word());
                        }
                    }
                }
                ModifyKeyType::DMark => {
                    for i in (0..self.buffer.len()).rev() {
                        if !self.options.free_marking && i != last_idx {
                            return None;
                        }
                        let slot = &mut self.buffer[i];
                        if to_lower_ascii(slot.raw_char) == 'd' {
                            if slot.modified && matches!(slot.lexi, VnLexiName::dd | VnLexiName::DD) {
                                slot.lexi = if slot.is_upper { VnLexiName::D } else { VnLexiName::d };
                                slot.modified = false;
                                self.temp_disabled = true;
                                self.buffer.push(Slot::new(raw_key));
                                return Some(self.rebuild_current_word());
                            }
                            slot.lexi = if slot.is_upper { VnLexiName::DD } else { VnLexiName::dd };
                            slot.modified = true;
                            return Some(self.rebuild_current_word());
                        }
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn apply_hook_all(&mut self, raw_key: char) -> Option<(usize, KeyText)> {
        let vowel_start = self.get_vowel_start();
        let mut u_idx = None;
        let mut o_idx = None;

        for (i, slot) in self.buffer.iter().enumerate().skip(vowel_start) {
            let ch = to_lower_ascii(slot.raw_char);
            if ch == 'u' {
                u_idx = Some(i);
                o_idx = None;
            } else if ch == 'o' && u_idx.is_some() && Some(i) == u_idx.map(|idx| idx + 1) {
                o_idx = Some(i);
            }
        }

        if let (Some(u), Some(o)) = (u_idx, o_idx) {
            let u_horned = matches!(self.buffer[u].lexi, VnLexiName::uh | VnLexiName::Uh);
            let o_horned = matches!(self.buffer[o].lexi, VnLexiName::oh | VnLexiName::Oh);

            if u_horned && o_horned {
                // ươ + w -> uo + w (undo)
                self.buffer[u].lexi = if self.buffer[u].is_upper { VnLexiName::U } else { VnLexiName::u };
                self.buffer[o].lexi = if self.buffer[o].is_upper { VnLexiName::O } else { VnLexiName::o };
                self.buffer[u].modified = false;
                self.buffer[o].modified = false;
                self.temp_disabled = true;
                self.refresh_tone_position();
                self.buffer.push(Slot::new(raw_key));
                return Some(self.rebuild_current_word());
            }

            if o_horned {
                // uơ + w -> uo + w (undo)
                self.buffer[o].lexi = if self.buffer[o].is_upper { VnLexiName::O } else { VnLexiName::o };
                self.buffer[o].modified = false;
                self.temp_disabled = true;
                self.refresh_tone_position();
                self.buffer.push(Slot::new(raw_key));
                return Some(self.rebuild_current_word());
            }

            if u_horned {
                // ưo + w -> ươ
                self.buffer[o].lexi = if self.buffer[o].is_upper { VnLexiName::Oh } else { VnLexiName::oh };
                self.buffer[o].modified = true;
                self.refresh_tone_position();
                return Some(self.rebuild_current_word());
            }

            // "th" + uo -> thuơ (only o is horned); otherwise uo -> ươ
            let th_special = o == self.buffer.len() - 1
                && u == 2
                && to_lower_ascii(self.buffer[0].raw_char) == 't'
                && to_lower_ascii(self.buffer[1].raw_char) == 'h'
                && !self.buffer[0].modified
                && !self.buffer[1].modified;

            self.buffer[u].lexi = if self.buffer[u].is_upper { VnLexiName::Uh } else { VnLexiName::uh };
            self.buffer[o].lexi = if self.buffer[o].is_upper { VnLexiName::Oh } else { VnLexiName::oh };
            self.buffer[u].modified = true;
            self.buffer[o].modified = true;

            if th_special {
                self.buffer[u].lexi = if self.buffer[u].is_upper { VnLexiName::U } else { VnLexiName::u };
                self.buffer[u].modified = false;
            }

            self.refresh_tone_position();
            return Some(self.rebuild_current_word());
        }

        // Check double 'w' undo: find modified vowel (last slot or with free marking)
        let last_buf_idx = self.buffer.len().saturating_sub(1);
        for i in (vowel_start..self.buffer.len()).rev() {
            let slot = &mut self.buffer[i];
            if slot.modified
                && matches!(
                    slot.lexi,
                    VnLexiName::ah
                        | VnLexiName::Ah
                        | VnLexiName::oh
                        | VnLexiName::Oh
                        | VnLexiName::uh
                        | VnLexiName::Uh
                )
            {
                if !self.options.free_marking && i != last_buf_idx {
                    break;
                }
                if to_lower_ascii(slot.raw_char) == 'w' {
                    slot.lexi = if is_ascii_upper(slot.raw_char) {
                        VnLexiName::W
                    } else {
                        VnLexiName::w
                    };
                    slot.modified = false;
                    self.word_form = WordForm::NonVn;
                    self.recalculate_vowel_state();
                    self.refresh_tone_position();
                    return Some(self.rebuild_current_word());
                }

                let plain = iso_to_vn_lexi(to_lower_ascii(slot.raw_char));
                slot.lexi = if is_ascii_upper(slot.raw_char) {
                    plain.to_upper()
                } else {
                    plain
                };
                slot.modified = false;
                self.temp_disabled = true;
                self.refresh_tone_position();
                self.buffer.push(Slot::new(raw_key));
                return Some(self.rebuild_current_word());
            }
        }

        // Modify the nearest preceding a, o or u inside the vowel nucleus
        for i in (vowel_start..self.buffer.len()).rev() {
            let slot = &mut self.buffer[i];
            let c = to_lower_ascii(slot.raw_char);

            if c == 'a' {
                if matches!(
                    self.options.method,
                    InputMethod::Telex
                        | InputMethod::SimpleTelex
                        | InputMethod::TelexSimple2
                        | InputMethod::TelexVni
                ) {
                    slot.lexi = if slot.is_upper { VnLexiName::Ah } else { VnLexiName::ah };
                } else {
                    continue;
                }
            } else if c == 'o' {
                slot.lexi = if slot.is_upper { VnLexiName::Oh } else { VnLexiName::oh };
            } else if c == 'u' {
                slot.lexi = if slot.is_upper { VnLexiName::Uh } else { VnLexiName::uh };
            } else {
                continue;
            }

            slot.modified = true;
            self.refresh_tone_position();
            return Some(self.rebuild_current_word());
        }

        None
    }

    fn process_word_break(&mut self, c: char) -> (usize, KeyText) {
        if self.options.enable_macro
            && !self.macros.is_empty()
            && (!self.buffer.is_empty() || !self.raw_chars.is_empty())
        {
            if let Some(replacement) = self.lookup_raw_macro() {
                let backspaces = self.display_len;
                let mut out = KeyText::from_str(&replacement);
                out.push_char(c);
                self.reset_word();
                return (backspaces, out);
            }
        }

        let mut raw_word = KeyText::new();
        let mut raw_lower = KeyText::new();
        for &rc in &self.raw_chars {
            raw_word.push_char(rc);
            raw_lower.push_char(to_lower_ascii(rc));
        }
        let display_word = self.current_preedit_key_text();
        let is_slang = crate::autoswitch::is_vietnamese_abbrev_or_slang(raw_lower.as_str());

        if self.options.auto_switch_by_typing && !self.raw_chars.is_empty() {
            // 1. Doubled key fix: e.g. "mixx" -> "mix"
            if self.pending_doubled_fix {
                self.pending_doubled_fix = false;
                if self.raw_chars.len() >= 3 {
                    // Char-boundary safe: raw may contain multibyte (e.g. typed đ/ơ/ư).
                    let trimmed_chars = self.raw_chars.len() - 1;
                    let mut trimmed_raw = KeyText::new();
                    let mut trimmed_lower = KeyText::new();
                    for &rc in &self.raw_chars[..trimmed_chars] {
                        trimmed_raw.push_char(rc);
                        trimmed_lower.push_char(to_lower_ascii(rc));
                    }
                    let full_en = crate::autoswitch::lookup_en_score(raw_lower.as_str()).is_some();
                    let trimmed_en =
                        crate::autoswitch::lookup_en_score(trimmed_lower.as_str()).is_some();
                    if !full_en && trimmed_en {
                        let backspaces = self.display_len;
                        let mut out = KeyText::from_str(trimmed_raw.as_str());
                        out.push_char(c);
                        self.auto_switch_requested = Some(false);
                        self.lookback
                            .push_word(trimmed_raw.as_str(), trimmed_raw.as_str(), true);
                        self.reset_word();
                        return (backspaces, out);
                    }
                }
            }

            // 2. Check AutoSwitch restore to English
            let prev_disp = self.lookback.prev_display();
            let should_switch = !is_slang
                && crate::autoswitch::should_restore_to_english(
                    raw_word.as_str(),
                    display_word.as_str(),
                    prev_disp,
                    self.is_valid_vietnamese_word(),
                );
            let is_english_keyword = !is_slang && self.matches_english_keyword(raw_lower.as_str());
            let is_english_word = !is_slang
                && (is_english_keyword
                    || should_switch
                    || crate::autoswitch::lookup_en_score(raw_lower.as_str()).is_some());

            if is_english_word {
                // 3. Look-back Multi-Word Check (e.g. "thí makes" -> "this makes")
                if self.options.can_rewrite_previous_word {
                    if let Some((extra_backspaces, restored_prefix)) = self
                        .lookback
                        .check_lookback_restore_for_english(raw_word.as_str())
                    {
                        let total_backspaces = self.display_len + extra_backspaces;
                        let mut out = restored_prefix;
                        out.push_str(raw_word.as_str());
                        out.push_char(c);
                        self.auto_switch_requested = Some(false);
                        self.lookback
                            .push_word(raw_word.as_str(), raw_word.as_str(), true);
                        self.reset_word();
                        return (total_backspaces, out);
                    }
                }

                if should_switch || is_english_keyword {
                    let is_already_raw = display_word.as_str() == raw_word.as_str();
                    if is_already_raw {
                        // Bilingual smoothness: the English word needed no visible
                        // correction (e.g. "open", "report", "hello"), so keep VN
                        // mode instead of ping-ponging V->E on every code-switched
                        // word. Only request a mode switch when we actually had to
                        // restore mangled text (display != raw).
                        self.auto_switch_requested = None;
                        self.lookback
                            .push_word(raw_word.as_str(), raw_word.as_str(), true);
                        self.reset_word();
                        return (0, KeyText::from_char(c));
                    }
                    let bk = self.display_len;
                    let mut o = raw_word;
                    o.push_char(c);
                    self.auto_switch_requested = Some(false);
                    self.lookback.push_word(
                        &o.as_str()[..o.len() - c.len_utf8()],
                        &o.as_str()[..o.len() - c.len_utf8()],
                        true,
                    );
                    self.reset_word();
                    return (bk, o);
                }
            }

            self.lookback
                .push_word(raw_word.as_str(), display_word.as_str(), false);
        }

        let should_restore = self.options.spell_check
            && self.options.auto_non_vn_restore
            && !self.buffer.is_empty()
            && (self.matches_english_keyword(raw_lower.as_str())
                || (is_slang && display_word.as_str() != raw_word.as_str())
                || (!self.temp_disabled
                    && self.has_any_vietnamese_mark()
                    && !self.is_valid_vietnamese_word()));

        if should_restore {
            // Restore raw keystrokes
            let mut match_len = 0;
            while match_len < self.buffer.len()
                && match_len < self.raw_chars.len()
                && to_lower_ascii(self.buffer[match_len].raw_char)
                    == to_lower_ascii(self.raw_chars[match_len])
                && self.buffer[match_len].tone == VnLexiTone::None
                && !self.buffer[match_len].modified
            {
                match_len += 1;
            }

            let prefix_len = self.measure_range(0, match_len);
            let backspaces = self.display_len.saturating_sub(prefix_len);

            let mut out = KeyText::new();
            for &raw in &self.raw_chars[match_len..] {
                out.push_char(raw);
            }
            out.push_char(c);

            self.reset_word();
            return (backspaces, out);
        }

        self.reset_word();
        (0, KeyText::from_char(c))
    }

    fn restore_raw_buffer_with_new_char(&mut self, _c: char) -> (usize, KeyText) {
        let prev_display_len = self.display_len;

        let mut match_len = 0;
        let buf_len = self.buffer.len();
        while match_len < buf_len - 1
            && match_len < self.raw_chars.len()
            && to_lower_ascii(self.buffer[match_len].raw_char)
                == to_lower_ascii(self.raw_chars[match_len])
            && self.buffer[match_len].tone == VnLexiTone::None
            && !self.buffer[match_len].modified
        {
            match_len += 1;
        }

        let prefix_len = self.measure_range(0, match_len);
        let backspaces = prev_display_len.saturating_sub(prefix_len);

        let mut out = KeyText::new();
        for &raw in &self.raw_chars[match_len..] {
            out.push_char(raw);
        }

        self.buffer.clear();
        for &raw in &self.raw_chars {
            let slot = Slot::new(raw);
            self.buffer.push(slot);
        }

        self.current_tone = VnLexiTone::None;
        self.word_form = WordForm::NonVn;
        self.has_ending_consonant = false;
        self.display_len = prefix_len + out.chars().count();
        self.clear_vowels();
        self.temp_disabled = true;

        (backspaces, out)
    }

    fn process_backspace(&mut self) -> (usize, KeyText) {
        if self.temp_disabled {
            self.temp_disabled = false;
            self.raw_chars = self.buffer.iter().map(|s| s.raw_char).collect();
        }

        if !self.raw_chars.is_empty() {
            self.raw_chars.pop();
        }

        if self.buffer.is_empty() {
            return (0, KeyText::new());
        }

        let word_len = self.measure_range(0, self.buffer.len());

        if self.display_len > word_len {
            let bk = self.display_len - word_len;
            self.display_len = word_len;
            return (bk, KeyText::new());
        }

        let prev_display_len = self.display_len;
        let removed_tone = self.buffer.last().map(|s| s.tone != VnLexiTone::None).unwrap_or(false);
        self.buffer.pop();

        if self.buffer.is_empty() {
            self.reset_word();
            return (prev_display_len, KeyText::new());
        }

        self.recalculate_word_form();

        if removed_tone {
            self.current_tone = VnLexiTone::None;
        } else {
            self.refresh_tone_position();
        }

        let out = self.encode_range(0, self.buffer.len());
        self.display_len = out.chars().count();
        (prev_display_len, out)
    }

    fn rebuild_current_word(&mut self) -> (usize, KeyText) {
        let bk = self.display_len;
        let out = self.encode_range(0, self.buffer.len());
        self.display_len = out.chars().count();
        (bk, out)
    }

    fn rebuild_from_index(&mut self, from: usize) -> (usize, KeyText) {
        let from_idx = from.min(self.buffer.len());
        let prefix_len = self.measure_range(0, from_idx);
        let bk = self.display_len.saturating_sub(prefix_len);
        let out = self.encode_range(from_idx, self.buffer.len());
        self.display_len = prefix_len + out.chars().count();
        (bk, out)
    }

    fn encode_range(&self, from: usize, to: usize) -> KeyText {
        let mut s = KeyText::new();
        for i in from..to {
            let slot = &self.buffer[i];
            let enc = encode_slot(slot.lexi, slot.tone, slot.is_upper, slot.raw_char, self.charset);
            for &c in enc.as_slice() {
                s.push_char(c);
            }
        }
        s
    }

    fn encode_slot_str(&self, slot: &Slot) -> KeyText {
        let enc = encode_slot(slot.lexi, slot.tone, slot.is_upper, slot.raw_char, self.charset);
        let mut s = KeyText::new();
        for &c in enc.as_slice() {
            s.push_char(c);
        }
        s
    }

    fn measure_range(&self, from: usize, to: usize) -> usize {
        let mut count = 0;
        for i in from..to {
            let slot = &self.buffer[i];
            let enc = encode_slot(slot.lexi, slot.tone, slot.is_upper, slot.raw_char, self.charset);
            count += enc.len;
        }
        count
    }

    fn get_vowel_start(&self) -> usize {
        if self.buffer.len() >= 2 {
            let c0 = to_lower_ascii(self.buffer[0].raw_char);
            let c1 = to_lower_ascii(self.buffer[1].raw_char);
            if (c0 == 'q' && c1 == 'u') || (c0 == 'g' && c1 == 'i') {
                return 2;
            }
        }
        0
    }

    fn clear_vowels(&mut self) {
        self.vowel_start = 0;
        self.vowel_count = 0;
    }

    fn add_vowel(&mut self, index: usize, _lexi: VnLexiName) {
        if self.vowel_count == 0 {
            self.vowel_start = index;
        }
        self.vowel_count += 1;
    }

    fn recalculate_vowel_state(&mut self) {
        self.clear_vowels();
        let start = self.get_vowel_start();

        for i in start..self.buffer.len() {
            if is_vowel_lexi(self.buffer[i].lexi) {
                self.add_vowel(i, self.buffer[i].lexi);
            }
        }

        if self.vowel_count == 0
            && start == 2
            && self.buffer.len() >= 2
            && to_lower_ascii(self.buffer[0].raw_char) == 'g'
            && to_lower_ascii(self.buffer[1].raw_char) == 'i'
        {
            self.add_vowel(1, self.buffer[1].lexi);
        }
    }

    fn last_vowel_is_current(&self) -> bool {
        self.vowel_count > 0 && self.vowel_start + self.vowel_count == self.buffer.len()
    }

    fn update_form_after_append(&mut self, is_vowel: bool) -> bool {
        let mut vowel_after_coda = false;

        if self.word_form != WordForm::NonVn {
            if is_vowel {
                if self.has_ending_consonant || matches!(self.word_form, WordForm::CVC | WordForm::VC) {
                    self.word_form = WordForm::NonVn;
                    vowel_after_coda = true;
                } else {
                    self.word_form = match self.word_form {
                        WordForm::C => WordForm::CV,
                        WordForm::V | WordForm::CV => self.word_form,
                        _ => WordForm::V,
                    };
                }
            } else {
                self.has_ending_consonant = matches!(self.word_form, WordForm::CV | WordForm::CVC | WordForm::VC);
                self.word_form = match self.word_form {
                    WordForm::CV => WordForm::CVC,
                    WordForm::V => WordForm::VC,
                    _ => WordForm::C,
                };
            }
        }

        vowel_after_coda
    }

    fn try_upgrade_uo_on_coda(&mut self) -> Option<usize> {
        if self.vowel_count != 2 || self.vowel_start + 2 != self.buffer.len() - 1 {
            return None;
        }

        let v0 = self.buffer[self.vowel_start].lexi;
        let v1 = self.buffer[self.vowel_start + 1].lexi;

        let u_then_o = matches!(v0, VnLexiName::Uh | VnLexiName::uh)
            && matches!(v1, VnLexiName::O | VnLexiName::o);
        let o_then_u = matches!(v0, VnLexiName::U | VnLexiName::u)
            && matches!(v1, VnLexiName::Oh | VnLexiName::oh);

        if !u_then_o && !o_then_u {
            return None;
        }

        let u_up = self.buffer[self.vowel_start].is_upper;
        let o_up = self.buffer[self.vowel_start + 1].is_upper;

        self.buffer[self.vowel_start].lexi = if u_up { VnLexiName::Uh } else { VnLexiName::uh };
        self.buffer[self.vowel_start + 1].lexi = if o_up { VnLexiName::Oh } else { VnLexiName::oh };
        self.buffer[self.vowel_start].modified = true;
        self.buffer[self.vowel_start + 1].modified = true;

        Some(self.vowel_start)
    }

    /// Tone position within the current vowel nucleus, without heap allocation.
    /// Nuclei longer than three vowels are outside Vietnamese phonotactics; the
    /// fallback matches `get_tone_position`'s behavior for unknown sequences.
    #[inline]
    fn tone_position_at(&self, count: usize, has_coda: bool) -> usize {
        if count > 3 {
            return 1;
        }
        let mut vowels = [VnLexiName::None; 3];
        for (i, vowel) in vowels.iter_mut().enumerate().take(count) {
            *vowel = self.buffer[self.vowel_start + i].lexi;
        }
        get_tone_position(&vowels[..count], has_coda, self.options.modern_style)
    }

    fn refresh_tone_position(&mut self) -> bool {
        if self.current_tone == VnLexiTone::None {
            return false;
        }

        let count = self.vowel_count;
        if count == 0 {
            self.current_tone = VnLexiTone::None;
            return true;
        }

        let has_coda = self.vowel_start + count < self.buffer.len();
        let target = self.tone_position_at(count, has_coda);

        let mut changed = false;
        for i in 0..count {
            let expected = if i == target {
                self.current_tone
            } else {
                VnLexiTone::None
            };
            if self.buffer[self.vowel_start + i].tone != expected {
                self.buffer[self.vowel_start + i].tone = expected;
                changed = true;
            }
        }

        changed
    }

    fn recalculate_word_form(&mut self) {
        self.recalculate_vowel_state();

        if self.buffer.is_empty() {
            self.word_form = WordForm::Empty;
            self.has_ending_consonant = false;
            return;
        }

        if self.vowel_count == 0 {
            self.word_form = WordForm::C;
            self.has_ending_consonant = false;
            return;
        }

        let starts_at_nucleus = self.vowel_start == self.get_vowel_start();
        let ends_at_end = self.vowel_start + self.vowel_count == self.buffer.len();

        self.word_form = if starts_at_nucleus {
            if ends_at_end { WordForm::V } else { WordForm::VC }
        } else {
            if ends_at_end { WordForm::CV } else { WordForm::CVC }
        };

        self.has_ending_consonant = !ends_at_end;
    }

    fn has_any_vietnamese_mark(&self) -> bool {
        self.buffer.iter().any(|s| s.tone != VnLexiTone::None || s.modified)
    }

    #[inline]
    fn lookup_raw_macro(&self) -> Option<String> {
        const STACK_TRIGGER_BYTES: usize = 64;
        if self.raw_chars.len() <= STACK_TRIGGER_BYTES {
            let mut trigger = [0u8; STACK_TRIGGER_BYTES];
            let mut is_ascii = true;
            for (i, &c) in self.raw_chars.iter().enumerate() {
                if c.is_ascii() {
                    trigger[i] = to_lower_ascii(c) as u8;
                } else {
                    is_ascii = false;
                    break;
                }
            }
            if is_ascii {
                if let Ok(trigger) = std::str::from_utf8(&trigger[..self.raw_chars.len()]) {
                    return self.macros.get(trigger);
                }
            }
        }

        let trigger: String = self.raw_chars.iter().map(|&c| to_lower_ascii(c)).collect();
        self.macros.get(&trigger)
    }

    #[inline]
    pub fn matches_english_keyword(&self, word: &str) -> bool {
        if word.len() > 32 {
            return false;
        }

        let is_removed = self.options
            .removed_english_keywords
            .binary_search_by(|r| r.as_str().cmp(word))
            .is_ok();

        if is_english_keyword(word) && !is_removed {
            return true;
        }

        self.options
            .custom_english_keywords
            .binary_search_by(|k| k.as_str().cmp(word))
            .is_ok()
    }

    #[inline]
    pub fn matches_raw_english_keyword(&self) -> bool {
        if self.raw_chars.is_empty() {
            return false;
        }
        if self.raw_chars.len() <= 32 {
            let mut stack_buf = [0u8; 32];
            let mut is_ascii = true;
            for (i, &c) in self.raw_chars.iter().enumerate() {
                if c.is_ascii() {
                    stack_buf[i] = to_lower_ascii(c) as u8;
                } else {
                    is_ascii = false;
                    break;
                }
            }
            if is_ascii {
                if let Ok(raw_slice) = std::str::from_utf8(&stack_buf[..self.raw_chars.len()]) {
                    return self.matches_english_keyword(raw_slice);
                }
            }
        } else {
            let raw_str: String = self.raw_chars.iter().map(|&c| to_lower_ascii(c)).collect();
            return self.matches_english_keyword(&raw_str);
        }
        false
    }

    pub fn is_valid_vietnamese_word(&self) -> bool {
        if self.buffer.is_empty() {
            return true;
        }
        if self.word_form == WordForm::NonVn {
            return false;
        }

        if is_foreign_initial(self.buffer[0].raw_char, self.buffer[0].lexi, &self.options.foreign_initials)
        {
            return false;
        }

        if self.matches_raw_english_keyword() {
            return false;
        }

        if self.vowel_count == 0 {
            return self.buffer.len() <= 3;
        }

        // Validate initial consonants
        if self.vowel_start > 0 {
            let initial_len = self.vowel_start;
            if initial_len > 3 {
                return false;
            }
            let mut initial = ['\0'; 3];
            for (i, slot) in self.buffer[..initial_len].iter().enumerate() {
                initial[i] = to_lower_ascii(slot.raw_char);
            }
            if !is_valid_initial(&initial[..initial_len]) {
                return false;
            }
            if !is_valid_initial_with_vowel(&initial[..initial_len], self.buffer[self.vowel_start].lexi) {
                return false;
            }
        }

        // Extract and validate the vowel nucleus
        let vowel_count = self.vowel_count;
        if vowel_count > 3 {
            return false;
        }
        let mut vowels = [VnLexiName::None; 3];
        for (i, vowel) in vowels.iter_mut().enumerate().take(vowel_count) {
            *vowel = self.buffer[self.vowel_start + i].lexi;
        }
        if !is_valid_vowel_seq(&vowels[..vowel_count]) {
            return false;
        }

        // In Vietnamese, 'u' followed by 'e' (e.g. -ue, -ure) is only possible
        // when preceded by initial 'q' (making 'qu' where 'qu' is initial consonant).
        // If vowels starts with [U, E] here, get_vowel_start() was 0 or 1 (not 'qu'),
        // which means this is an English word like texture, picture, blue, true, etc.
        if vowel_count >= 2
            && normalize_vowel(vowels[0]) == VnLexiName::U
            && normalize_vowel(vowels[1]) == VnLexiName::E
        {
            return false;
        }

        // Validate ending coda (if present)
        let ending_start = self.vowel_start + vowel_count;
        if ending_start < self.buffer.len() {
            let ending_len = self.buffer.len() - ending_start;
            // No Vietnamese syllable ends with more than two consonants.
            if ending_len > 2 {
                return false;
            }
            let mut ending = ['\0'; 2];
            for (i, slot) in self.buffer[ending_start..].iter().enumerate() {
                ending[i] = to_lower_ascii(slot.raw_char);
            }
            if !is_valid_ending_consonant(&ending[..ending_len]) {
                return false;
            }

            // Vowel sequence must be allowed to take a coda
            if !can_vowel_seq_take_coda(&vowels[..vowel_count]) {
                return false;
            }

            // Stop codas (c, ch, p, t) cannot take grave, hook or tilde
            if is_stop_coda(&ending[..ending_len])
                && matches!(
                    self.current_tone,
                    VnLexiTone::Grave | VnLexiTone::Hook | VnLexiTone::Tilde
                )
            {
                return false;
            }
        }

        true
    }

    pub fn take_auto_switch_request(&mut self) -> Option<bool> {
        self.auto_switch_requested.take()
    }

    pub fn push_english_word_lookback(&mut self, word: &str) {
        if self.options.auto_switch_by_typing && !word.is_empty() {
            self.lookback.push_word(word, word, true);
        }
    }

    /// Evaluates a raw word typed in English mode at word break.
    /// If it represents a valid Vietnamese word (typed with Telex/VNI keystrokes),
    /// returns Some((backspaces, replacement_text)) and sets auto_switch_requested to Some(true).
    pub fn check_english_word_at_break(
        &mut self,
        raw_word: &str,
        break_char: char,
    ) -> Option<(usize, KeyText)> {
        if !self.options.auto_switch_by_typing || raw_word.len() < 2 || raw_word.len() > 32 {
            return None;
        }

        let raw_lower = crate::autoswitch::ascii_lowercase_inline(raw_word);
        // Custom or built-in English keywords must strictly stay in English mode
        if self.matches_english_keyword(raw_lower.as_str()) {
            return None;
        }

        // Compose in-place using preallocated syllable buffers (zero heap allocation).
        self.reset_word();
        let mut broke_mid_word = false;
        for ch in raw_word.chars() {
            if self.buffer.is_empty() {
                if self.options.method == InputMethod::Telex && matches!(ch, '[' | ']' | '{' | '}')
                {
                    let _ = self.try_bracket_shortcut(ch);
                } else if self.starts_lone_w(ch) {
                    let _ = self.process_lone_w(ch);
                } else if is_letter_fast(ch) {
                    let _ = self.process_first_letter(ch);
                } else {
                    broke_mid_word = true;
                    break;
                }
            } else if self.try_process_in_word_key(ch).is_none() {
                broke_mid_word = true;
                break;
            }
        }

        if broke_mid_word || self.buffer.is_empty() {
            self.reset_word();
            return None;
        }

        let composed = self.current_preedit_key_text();
        let valid_vn = self.is_valid_vietnamese_word();
        self.reset_word();

        if composed.is_empty() || composed.as_str() == raw_word || !valid_vn {
            return None;
        }

        let prev = self.lookback.prev_display();
        if crate::autoswitch::should_switch_to_vietnamese(raw_word, composed.as_str(), prev) {
            let backspaces = raw_word.chars().count();
            self.auto_switch_requested = Some(true);
            self.lookback.push_word(raw_word, composed.as_str(), false);
            let mut out = composed;
            if break_char != '\0' {
                out.push_char(break_char);
            }
            return Some((backspaces, out));
        }

        None
    }
}

#[inline(always)]
fn is_vowel_lexi(lexi: VnLexiName) -> bool {
    let idx = lexi as i16;
    idx >= 0 && (idx as usize) < TOTAL_VN_CHARS && IS_VN_VOWEL[idx as usize]
}
