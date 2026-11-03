// VietFlow Core - Look-back Buffer for Multi-Word Disambiguation
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::autoswitch::{lookup_en_score, lookup_vn_score, to_telex_normalized_key, AS_LOOKBACK_MARGIN};

#[derive(Debug, Clone, Default)]
pub struct FinishedWord {
    pub raw: String,
    pub display: String,
    pub is_ambiguous: bool,
    pub en_score: Option<i8>,
    pub vn_score: Option<i8>,
    pub restored_to_english: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LookbackBuffer {
    pub words: [Option<FinishedWord>; 2],
    pub count: usize,
}

impl LookbackBuffer {
    pub fn new() -> Self {
        Self {
            words: [None, None],
            count: 0,
        }
    }

    pub fn clear(&mut self) {
        self.words = [None, None];
        self.count = 0;
    }

    /// Push a newly finalized word followed by a single space into the lookback buffer.
    pub fn push_word(&mut self, raw: &str, display: &str, restored_to_english: bool) {
        if raw.is_empty() {
            return;
        }

        let en_score = lookup_en_score(&raw.to_ascii_lowercase());
        let vn_key = to_telex_normalized_key(display);
        let vn_score = vn_key.as_deref().and_then(lookup_vn_score);

        let is_ambiguous = en_score.is_some() && vn_score.is_some();

        let fw = FinishedWord {
            raw: raw.to_string(),
            display: display.to_string(),
            is_ambiguous,
            en_score,
            vn_score,
            restored_to_english,
        };

        self.words[1] = self.words[0].take();
        self.words[0] = Some(fw);
        if self.count < 2 {
            self.count += 1;
        }
    }

    /// Returns the immediate previous word's display string (for bigram calculation).
    pub fn prev_display(&self) -> Option<&str> {
        self.words[0].as_ref().map(|w| w.display.as_str())
    }

    /// Checks if a confident English word can flip ambiguous words preceding it.
    /// E.g. "thí makes" -> "makes" is pure English, preceding is "thí" (raw "this").
    /// Returns Some((words_to_delete_count, restored_text_prefix)) if look-back should fire.
    pub fn check_lookback_restore_for_english(&self, curr_raw: &str) -> Option<(usize, String)> {
        // Current word must be a known English word
        let _curr_en = lookup_en_score(&curr_raw.to_ascii_lowercase())?;

        // Check the immediate previous word
        let prev = self.words[0].as_ref()?;
        if !prev.is_ambiguous {
            return None;
        }

        // Acronyms (2+ capitals) don't flip the word before them (D42)
        let capital_count = prev.raw.chars().filter(|c| c.is_ascii_uppercase()).count();
        if capital_count >= 2 {
            return None;
        }

        let pen = prev.en_score.unwrap_or(-128);
        let pvn = prev.vn_score.unwrap_or(-128);

        // Rule D13: Preceding word is ambiguous and its English score is at least
        // AS_LOOKBACK_MARGIN (15 points ~ 30x) higher than its Vietnamese reading.
        // E.g.: "this" (en=39) vs "thí" (vn=18) -> diff = 21 >= 15!
        if pen as i16 - pvn as i16 >= AS_LOOKBACK_MARGIN as i16 {
            // How many chars to backspace: previous display length + 1 (the space)
            let backspace_count = prev.display.chars().count() + 1;
            let mut replacement = String::with_capacity(prev.raw.len() + 1);
            replacement.push_str(&prev.raw);
            replacement.push(' ');
            return Some((backspace_count, replacement));
        }

        None
    }
}

