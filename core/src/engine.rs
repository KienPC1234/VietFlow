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
    CharType, CharsetId, EngineOptions, InputMethod, ModifyKeyType, ToneKeyType, VnLexiName,
    VnLexiTone, WordForm,
};
use crate::tables::{iso_to_vn_lexi, IS_VN_VOWEL, TOTAL_VN_CHARS};
use crate::tone_rules::{
    get_tone_position, is_stop_coda, is_valid_ending_consonant, is_valid_initial_with_vowel,
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
    capitalize_next: bool,
}

impl Default for VietFlowEngine {
    fn default() -> Self {
        Self::new(EngineOptions::default(), CharsetId::Unicode)
    }
}

impl VietFlowEngine {
    pub fn new(options: EngineOptions, charset: CharsetId) -> Self {
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
            capitalize_next: false,
        }
    }

    pub fn reset(&mut self) {
        self.reset_word();
        self.capitalize_next = false;
    }

    fn reset_word(&mut self) {
        self.buffer.clear();
        self.raw_chars.clear();
        self.word_form = WordForm::Empty;
        self.current_tone = VnLexiTone::None;
        self.has_ending_consonant = false;
        self.display_len = 0;
        self.vowel_start = 0;
        self.vowel_count = 0;
    }

    pub fn process_key(&mut self, c: char) -> (usize, KeyText) {
        if c == '\x08' {
            return self.process_backspace();
        }

        // Telex bracket shortcuts: [ = ơ, ] = ư, { = Ơ, } = Ư (Telex only)
        if self.options.method == InputMethod::Telex {
            if let Some(res) = self.try_bracket_shortcut(c) {
                return res;
            }
        }

        if self.buffer.is_empty() {
            if self.options.consonant_shortcuts && matches!(c, 'f' | 'F' | 'j' | 'J' | 'w' | 'W') {
                return self.process_first_letter(c);
            }

            if matches!(
                self.options.method,
                InputMethod::Telex | InputMethod::TelexVni
            ) && matches!(c, 'w' | 'W')
            {
                return self.process_lone_w(c);
            }

            if is_letter_fast(c) {
                return self.process_first_letter(c);
            }

            if self.options.auto_capitalize && matches!(c, '.' | '!' | '?' | '\r' | '\n') {
                self.capitalize_next = true;
            }

            return (0, KeyText::from_char(c));
        }

        // Try tone key
        let tone_key = get_tone_key(c, self.options.method);
        if tone_key != ToneKeyType::None {
            if let Some(res) = self.try_apply_tone(tone_key, c) {
                self.raw_chars.push(c);
                return res;
            }
        }

        // Try modify key
        let modify_key = get_modify_key(c, self.options.method);
        if modify_key != ModifyKeyType::None || self.is_double_key_modify(c) {
            if let Some(res) = self.try_apply_modify(modify_key, c) {
                self.raw_chars.push(c);
                return res;
            }
        }

        // Subsequent letters
        if is_letter_fast(c) {
            return self.process_subsequent_letter(c);
        }

        // Word breaks
        let char_type = get_char_type(c, self.options.method);
        if matches!(char_type, CharType::WordBreak | CharType::Reset) || is_word_break_fast(c) {
            return self.process_word_break(c);
        }

        self.process_word_break(c)
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

    fn append_mapped_letter(&mut self, raw_key: char, lexi: VnLexiName, mut is_upper: bool) -> (usize, KeyText) {
        if self.buffer.is_empty() && self.capitalize_next && self.options.auto_capitalize {
            is_upper = true;
            self.capitalize_next = false;
        }

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

        if self.current_tone != VnLexiTone::None && self.word_form != WordForm::NonVn {
            if self.refresh_tone_position() {
                return self.rebuild_from_index(self.vowel_start);
            }
        }

        let out = self.encode_slot_str(&slot);
        self.display_len += out.chars().count();
        (0, out)
    }

    fn process_first_letter(&mut self, mut c: char) -> (usize, KeyText) {
        if self.capitalize_next && self.options.auto_capitalize && c.is_alphabetic() {
            c = c.to_ascii_uppercase();
            self.capitalize_next = false;
        }

        self.raw_chars.push(c);
        let is_upper = is_ascii_upper(c);
        let lower = to_lower_ascii(c);

        if self.options.consonant_shortcuts {
            if lower == 'f' {
                return self.rebuild_shortcut(VnLexiName::p, VnLexiName::h, is_upper);
            }
            if lower == 'j' {
                return self.rebuild_shortcut(VnLexiName::g, VnLexiName::i, is_upper);
            }
            if lower == 'w' {
                return self.rebuild_shortcut(VnLexiName::q, VnLexiName::u, is_upper);
            }
        }

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

    fn rebuild_shortcut(&mut self, first: VnLexiName, second: VnLexiName, is_upper: bool) -> (usize, KeyText) {
        let mut slot0 = Slot::new(if is_upper { 'P' } else { 'p' });
        slot0.lexi = if is_upper { first.to_upper() } else { first };
        slot0.is_upper = is_upper;
        self.buffer.push(slot0);

        let mut slot1 = Slot::new('h');
        slot1.lexi = second;
        slot1.is_upper = false;
        self.buffer.push(slot1);

        self.word_form = WordForm::C;
        self.has_ending_consonant = false;
        self.recalculate_vowel_state();

        self.rebuild_current_word()
    }

    fn process_lone_w(&mut self, c: char) -> (usize, KeyText) {
        self.raw_chars.push(c);
        let is_upper = is_ascii_upper(c);

        let mut slot = Slot::new(c);
        slot.lexi = if is_upper { VnLexiName::Uh } else { VnLexiName::uh };
        slot.is_upper = is_upper;
        slot.modified = true;
        self.buffer.push(slot);

        self.word_form = WordForm::V;
        self.has_ending_consonant = false;
        self.recalculate_vowel_state();

        let out = self.encode_slot_str(&slot);
        self.display_len = out.chars().count();
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

        // Auto fix Shift key sticking on 2nd letter (e.g. "VIệt" -> "Việt")
        if self.options.fix_shift_lag && self.buffer.len() >= 3 {
            if self.buffer[0].is_upper && self.buffer[1].is_upper && !is_upper {
                self.buffer[1].is_upper = false;
                self.buffer[1].raw_char = to_lower_ascii(self.buffer[1].raw_char);
                self.buffer[1].lexi = self.buffer[1].lexi.to_lower();
                self.refresh_tone_position();
                return self.rebuild_current_word();
            }
        }

        if let Some(up_idx) = upgrade_from {
            self.refresh_tone_position();
            return self.rebuild_from_index(up_idx);
        }

        if self.current_tone != VnLexiTone::None && self.word_form != WordForm::NonVn {
            if self.refresh_tone_position() {
                return self.rebuild_from_index(self.vowel_start);
            }
        }

        let out = self.encode_slot_str(&self.buffer.last().unwrap());
        self.display_len += out.chars().count();
        (0, out)
    }

    fn append_non_vn_letter(&mut self, c: char) -> (usize, KeyText) {
        let slot = Slot::new(c);
        self.buffer.push(slot);
        self.word_form = WordForm::NonVn;
        self.recalculate_vowel_state();

        let out = self.encode_slot_str(&slot);
        self.display_len += out.chars().count();
        (0, out)
    }

    fn is_double_key_modify(&self, c: char) -> bool {
        if !matches!(
            self.options.method,
            InputMethod::Telex
                | InputMethod::SimpleTelex
                | InputMethod::TelexSimple2
                | InputMethod::TelexVni
        ) {
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
            && !self.options.consonant_shortcuts
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

        // Stop codas (c, ch, p, t) never take huyền/hỏi/ngã
        if has_coda {
            let coda = &self.buffer[last_vowel + 1..];
            let coda_len = coda.len();
            if coda_len <= 2 {
                let mut coda_chars = ['\0'; 2];
                for (i, slot) in coda.iter().enumerate() {
                    coda_chars[i] = to_lower_ascii(slot.raw_char);
                }
                if is_stop_coda(&coda_chars[..coda_len])
                    && matches!(
                        new_tone,
                        VnLexiTone::Grave | VnLexiTone::Hook | VnLexiTone::Tilde
                    )
                {
                    return None;
                }
            }
        }

        // Check for tone undo: restore the raw keystroke sequence up to (but
        // not including) the repeated tone key. This keeps the original `ow`,
        // `uw`, `[`, `]`, `{`, `}` forms instead of leaving `ơs`/`ưs`.
        if self.current_tone == new_tone {
            self.current_tone = VnLexiTone::None;
            let backspaces = self.display_len;
            let mut out = KeyText::new();
            for &raw in &self.raw_chars {
                out.push_char(raw);
            }
            self.buffer.clear();
            for &raw in &self.raw_chars {
                self.buffer.push(Slot::new(raw));
            }
            self.recalculate_word_form();
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
            && !self.options.consonant_shortcuts
            && is_foreign_initial(self.buffer[0].raw_char, self.buffer[0].lexi, &self.options.foreign_initials)
        {
            return None;
        }

        // Double key handling in Telex
        if matches!(
            self.options.method,
            InputMethod::Telex
                | InputMethod::SimpleTelex
                | InputMethod::TelexSimple2
                | InputMethod::TelexVni
        ) && self.word_form != WordForm::NonVn
        {
            if lower == 'd' {
                let last_idx = self.buffer.len() - 1;
                let slot = &mut self.buffer[last_idx];
                if matches!(
                    slot.lexi,
                    VnLexiName::d | VnLexiName::D | VnLexiName::dd | VnLexiName::DD
                ) {
                    if slot.modified {
                        slot.lexi = if slot.is_upper { VnLexiName::D } else { VnLexiName::d };
                        slot.modified = false;
                        let (bk, mut s) = self.rebuild_current_word();
                        s.push_char(raw_key);
                        self.display_len += 1;
                        return Some((bk, s));
                    }

                    slot.lexi = if slot.is_upper { VnLexiName::DD } else { VnLexiName::dd };
                    slot.modified = true;
                    return Some(self.rebuild_current_word());
                }
            }

            if matches!(lower, 'a' | 'e' | 'o') {
                let last_idx = self.buffer.len() - 1;
                let slot = &mut self.buffer[last_idx];
                let slot_char = to_lower_ascii(slot.raw_char);

                if slot_char == lower {
                    if slot.modified {
                        slot.lexi = iso_to_vn_lexi(slot_char);
                        slot.modified = false;
                        let (bk, mut s) = self.rebuild_current_word();
                        s.push_char(raw_key);
                        self.display_len += 1;
                        return Some((bk, s));
                    }

                    let circumflex = match lower {
                        'a' => if slot.is_upper { VnLexiName::Ar } else { VnLexiName::ar },
                        'e' => if slot.is_upper { VnLexiName::Er } else { VnLexiName::er },
                        'o' => if slot.is_upper { VnLexiName::Or } else { VnLexiName::or },
                        _ => slot.lexi,
                    };

                    slot.lexi = circumflex;
                    slot.modified = true;
                    return Some(self.rebuild_current_word());
                }
            }
        }

        // 'w' key handling (Telex hook-all)
        if modify_key == ModifyKeyType::HookAll
            || (matches!(
                self.options.method,
                InputMethod::Telex
                    | InputMethod::SimpleTelex
                    | InputMethod::TelexSimple2
                    | InputMethod::TelexVni
            ) && lower == 'w')
        {
            if self.vowel_count > 0 && !self.options.free_marking && !self.last_vowel_is_current() {
                return None;
            }

            if let Some(res) = self.apply_hook_all(raw_key) {
                return Some(res);
            }

            // Lone 'w' becomes 'ư' in Telex, SimpleTelex, and TelexVni, but NOT in TelexSimple2
            // or if the word already has a non-vn form or literal 'w'
            if matches!(
                self.options.method,
                InputMethod::Telex | InputMethod::SimpleTelex | InputMethod::TelexVni
            ) && self.word_form != WordForm::NonVn
                && !self.buffer.iter().any(|s| to_lower_ascii(s.raw_char) == 'w')
            {
                return Some(self.process_lone_w(raw_key));
            }
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

        for i in vowel_start..self.buffer.len() {
            let ch = to_lower_ascii(self.buffer[i].raw_char);
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
                // ươ + w -> uo (undo)
                self.buffer[u].lexi = if self.buffer[u].is_upper { VnLexiName::U } else { VnLexiName::u };
                self.buffer[o].lexi = if self.buffer[o].is_upper { VnLexiName::O } else { VnLexiName::o };
                self.buffer[u].modified = false;
                self.buffer[o].modified = false;
                self.refresh_tone_position();
                let (bk, mut s) = self.rebuild_current_word();
                s.push_char(raw_key);
                self.display_len += 1;
                return Some((bk, s));
            }

            if o_horned {
                // uơ + w -> uo (undo)
                self.buffer[o].lexi = if self.buffer[o].is_upper { VnLexiName::O } else { VnLexiName::o };
                self.buffer[o].modified = false;
                self.refresh_tone_position();
                let (bk, mut s) = self.rebuild_current_word();
                s.push_char(raw_key);
                self.display_len += 1;
                return Some((bk, s));
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

        // Check double 'w' undo: only on immediately preceding modified vowel
        let last_idx = self.buffer.len() - 1;
        let last_slot = &mut self.buffer[last_idx];
        if last_slot.modified
            && matches!(
                last_slot.lexi,
                VnLexiName::ah
                    | VnLexiName::Ah
                    | VnLexiName::oh
                    | VnLexiName::Oh
                    | VnLexiName::uh
                    | VnLexiName::Uh
            )
        {
            if to_lower_ascii(last_slot.raw_char) == 'w' {
                last_slot.lexi = if is_ascii_upper(last_slot.raw_char) {
                    VnLexiName::W
                } else {
                    VnLexiName::w
                };
                last_slot.modified = false;
                self.word_form = WordForm::NonVn;
                self.recalculate_vowel_state();
                self.refresh_tone_position();
                return Some(self.rebuild_current_word());
            }

            let plain = iso_to_vn_lexi(to_lower_ascii(last_slot.raw_char));
            last_slot.lexi = if is_ascii_upper(last_slot.raw_char) {
                plain.to_upper()
            } else {
                plain
            };
            last_slot.modified = false;
            self.refresh_tone_position();
            let (bk, mut s) = self.rebuild_current_word();
            s.push_char(raw_key);
            self.display_len += 1;
            return Some((bk, s));
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
        if self.options.auto_capitalize && matches!(c, '.' | '!' | '?' | '\r' | '\n') {
            self.capitalize_next = true;
        }

        if self.options.enable_macro
            && !self.macros.is_empty()
            && (!self.buffer.is_empty() || !self.raw_chars.is_empty())
        {
            let trigger: String = self.raw_chars.iter().collect();
            if let Some(replacement) = self.macros.get(&trigger) {
                let backspaces = self.display_len;
                let mut out = KeyText::from_str(&replacement);
                out.push_char(c);
                self.reset_word();
                return (backspaces, out);
            }
        }

        if !self.buffer.is_empty()
            && self.options.spell_check
            && self.options.auto_non_vn_restore
            && self.has_any_vietnamese_mark()
            && !self.is_valid_vietnamese_word()
        {
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
            for i in match_len..self.raw_chars.len() {
                out.push_char(self.raw_chars[i]);
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
        for i in match_len..self.raw_chars.len() {
            out.push_char(self.raw_chars[i]);
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

        (backspaces, out)
    }

    fn process_backspace(&mut self) -> (usize, KeyText) {
        if !self.raw_chars.is_empty() {
            self.raw_chars.pop();
        }

        if self.buffer.is_empty() {
            return (1, KeyText::new());
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
    fn matches_english_keyword(&self, word: &str) -> bool {
        is_english_keyword(word)
            || self
                .options
                .custom_english_keywords
                .iter()
                .any(|keyword| keyword == word)
    }

    pub fn is_valid_vietnamese_word(&self) -> bool {
        if self.buffer.is_empty() {
            return true;
        }
        if self.word_form == WordForm::NonVn {
            return false;
        }

        if !self.options.consonant_shortcuts
            && is_foreign_initial(self.buffer[0].raw_char, self.buffer[0].lexi, &self.options.foreign_initials)
        {
            return false;
        }

        // Only compare the raw key sequence. Comparing the buffer base would
        // treat Vietnamese words such as "mẹ" (raw "mej", base "me") as the
        // English word "me" and skip the tone restoration.
        let raw_str: String = self.raw_chars.iter().map(|&c| to_lower_ascii(c)).collect();
        if self.matches_english_keyword(&raw_str) {
            return false;
        }

        if self.vowel_count == 0 {
            return true;
        }

        if self.vowel_count == 1 && self.vowel_start > 0 {
            let initial_len = self.vowel_start;
            if initial_len <= 3 {
                let mut initial = ['\0'; 3];
                for (i, slot) in self.buffer[..initial_len].iter().enumerate() {
                    initial[i] = to_lower_ascii(slot.raw_char);
                }
                if !is_valid_initial_with_vowel(&initial[..initial_len], self.buffer[self.vowel_start].lexi) {
                    return false;
                }
            }
        }

        let ending_start = self.vowel_start + self.vowel_count;
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
        }

        true
    }
}

#[inline(always)]
fn is_vowel_lexi(lexi: VnLexiName) -> bool {
    let idx = lexi as i16;
    idx >= 0 && (idx as usize) < TOTAL_VN_CHARS && IS_VN_VOWEL[idx as usize]
}
