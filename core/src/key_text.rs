// VietFlow Core - Allocation-free key output buffer
// Licensed under GNU General Public License v3.0 (GPLv3)
//
// The engine emits at most a few UTF-8 characters on the steady-state typing
// path. Storing those characters inline avoids a heap allocation per keystroke.
// Longer results (macro expansions, restored raw words) transparently fall back
// to a heap string.

const INLINE_BYTES: usize = 64;

#[derive(Debug, Clone)]
pub struct KeyText {
    len: usize,
    inline: [u8; INLINE_BYTES],
    heap: Option<String>,
}

impl Default for KeyText {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl KeyText {
    #[inline]
    pub fn new() -> Self {
        Self {
            len: 0,
            inline: [0; INLINE_BYTES],
            heap: None,
        }
    }

    #[inline]
    pub fn from_char(c: char) -> Self {
        let mut text = Self::new();
        text.push_char(c);
        text
    }

    #[inline]
    pub fn from_str(s: &str) -> Self {
        let mut text = Self::new();
        text.push_str(s);
        text
    }

    #[inline]
    pub fn clear(&mut self) {
        self.heap = None;
        self.len = 0;
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match &self.heap {
            Some(heap) => heap.is_empty(),
            None => self.len == 0,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.heap {
            Some(heap) => heap.len(),
            None => self.len,
        }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.heap {
            Some(heap) => heap.as_bytes(),
            None => &self.inline[..self.len],
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: only valid UTF-8 produced by `push_str`/`push_char` is stored.
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }

    #[inline]
    pub fn chars(&self) -> std::str::Chars<'_> {
        self.as_str().chars()
    }

    pub fn push_char(&mut self, c: char) {
        let mut buf = [0u8; 4];
        self.push_str(c.encode_utf8(&mut buf));
    }

    pub fn push_str(&mut self, s: &str) {
        if let Some(heap) = &mut self.heap {
            heap.push_str(s);
            return;
        }

        let extra = s.len();
        if self.len + extra <= INLINE_BYTES {
            self.inline[self.len..self.len + extra].copy_from_slice(s.as_bytes());
            self.len += extra;
            return;
        }

        let mut heap = String::with_capacity(self.len + extra);
        // SAFETY: the inline prefix is valid UTF-8, see `push_str`.
        heap.push_str(unsafe { std::str::from_utf8_unchecked(&self.inline[..self.len]) });
        heap.push_str(s);
        self.heap = Some(heap);
    }
}

impl std::ops::Deref for KeyText {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for KeyText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for KeyText {
    #[inline]
    fn from(value: &str) -> Self {
        Self::from_str(value)
    }
}

impl From<char> for KeyText {
    #[inline]
    fn from(value: char) -> Self {
        Self::from_char(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_text_roundtrip() {
        let mut text = KeyText::new();
        text.push_str("Việt");
        text.push_char(' ');
        text.push_str("Nam");
        assert_eq!(text.as_str(), "Việt Nam");
        assert_eq!(text.chars().count(), 8);
        assert_eq!(text.len(), "Việt Nam".len());
    }

    #[test]
    fn spills_to_heap_when_long() {
        let long = "x".repeat(200);
        let text = KeyText::from_str(&long);
        assert_eq!(text.as_str(), long);
        assert!(text.len() > INLINE_BYTES);
    }

    #[test]
    fn unicode_boundaries_survive_spill() {
        let mut text = KeyText::from_str("ế");
        for _ in 0..40 {
            text.push_char('ự');
        }
        assert_eq!(text.chars().count(), 41);
        assert!(text.as_str().starts_with('ế'));
        assert!(text.as_str().ends_with('ự'));
    }
}
