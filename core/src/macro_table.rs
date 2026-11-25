// VietFlow Core - Macro Table & Dynamic Variable Expansion
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::collections::HashMap;

/// Host-side placeholder for the clipboard. Core stays platform-neutral; the
/// Windows host replaces this sentinel with the current clipboard text before
/// sending the macro output.
pub const CLIPBOARD_PLACEHOLDER: char = '\u{E000}';

#[derive(Debug, Clone)]
pub struct MacroTable {
    macros: HashMap<String, String>,
    pub date_format: String,
    pub time_format: String,
    pub datetime_format: String,
}

impl Default for MacroTable {
    fn default() -> Self {
        let mut macros = HashMap::new();
        macros.insert("vf".to_string(), "VietFlow".to_string());
        macros.insert("vn".to_string(), "Việt Nam".to_string());
        macros.insert(
            "chxhcnvn".to_string(),
            "Cộng hòa Xã hội Chủ nghĩa Việt Nam".to_string(),
        );

        Self {
            macros,
            date_format: "dd/MM/yyyy".to_string(),
            time_format: "HH:mm".to_string(),
            datetime_format: "dd/MM/yyyy HH:mm".to_string(),
        }
    }
}

impl MacroTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, trigger: &str, replacement: &str) {
        self.macros.insert(trigger.to_lowercase(), replacement.to_string());
    }

    pub fn remove(&mut self, trigger: &str) -> bool {
        self.macros.remove(&trigger.to_lowercase()).is_some()
    }

    pub fn clear(&mut self) {
        self.macros.clear();
    }

    pub fn len(&self) -> usize {
        self.macros.len()
    }

    pub fn is_empty(&self) -> bool {
        self.macros.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.macros.iter()
    }

    pub fn get(&self, trigger: &str) -> Option<String> {
        // The engine normally passes an already-lowercase ASCII trigger. Avoid
        // allocating a second String for every word break in that common case.
        let template = if trigger
            .bytes()
            .all(|b| b.is_ascii() && !b.is_ascii_uppercase())
        {
            self.macros.get(trigger)?
        } else {
            self.macros.get(&trigger.to_lowercase())?
        };
        Some(self.expand_variables(template))
    }

    fn expand_variables(&self, template: &str) -> String {
        if !template.contains('@') {
            return template.to_string();
        }

        let mut result = String::with_capacity(template.len() + 16);
        let bytes = template.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] != b'@' {
                let ch = template[i..].chars().next().unwrap();
                result.push(ch);
                i += ch.len_utf8();
                continue;
            }

            let token_start = i + 1;
            let mut token_end = token_start;
            while token_end < bytes.len() {
                let ch = template[token_end..].chars().next().unwrap();
                if ch.is_alphanumeric() || ch == '_' {
                    token_end += ch.len_utf8();
                } else {
                    break;
                }
            }

            if token_end == token_start {
                result.push('@');
                i += 1;
                continue;
            }

            let token = &template[token_start..token_end];
            if !self.append_resolved_variable(token, &mut result) {
                result.push('@');
                result.push_str(token);
            }

            i = token_end;
        }

        result
    }

    fn append_resolved_variable(&self, token: &str, out: &mut String) -> bool {
        if token.eq_ignore_ascii_case("newline") {
            out.push('\n');
            true
        } else if token.eq_ignore_ascii_case("tab") {
            out.push('\t');
            true
        } else if token.eq_ignore_ascii_case("date") {
            format_date_into(&self.date_format, out);
            true
        } else if token.eq_ignore_ascii_case("time") {
            format_date_into(&self.time_format, out);
            true
        } else if token.eq_ignore_ascii_case("datetime") || token.eq_ignore_ascii_case("now") {
            format_date_into(&self.datetime_format, out);
            true
        } else if token.eq_ignore_ascii_case("clipboard") {
            out.push(CLIPBOARD_PLACEHOLDER);
            true
        } else {
            false
        }
    }
}

#[cfg(windows)]
fn get_local_date_time() -> (i32, u32, u32, u32, u32, u32) {
    #[repr(C)]
    #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
    struct SYSTEMTIME {
        w_year: u16,
        w_month: u16,
        w_day_of_week: u16,
        w_day: u16,
        w_hour: u16,
        w_minute: u16,
        w_second: u16,
        w_milliseconds: u16,
    }
    extern "system" {
        fn GetLocalTime(lpSystemTime: *mut SYSTEMTIME);
    }
    let mut st = SYSTEMTIME {
        w_year: 0,
        w_month: 0,
        w_day_of_week: 0,
        w_day: 0,
        w_hour: 0,
        w_minute: 0,
        w_second: 0,
        w_milliseconds: 0,
    };
    unsafe {
        GetLocalTime(&mut st);
    }
    (
        st.w_year as i32,
        st.w_month as u32,
        st.w_day as u32,
        st.w_hour as u32,
        st.w_minute as u32,
        st.w_second as u32,
    )
}

#[cfg(not(windows))]
fn get_local_date_time() -> (i32, u32, u32, u32, u32, u32) {
    use std::ffi::{c_char, c_int, c_long, c_void};

    // Full BSD/glibc/musl `struct tm` layout including `tm_gmtoff` and `tm_zone`
    // plus trailing slack so `localtime_r` never writes past the stack frame.
    #[repr(C)]
    struct Tm {
        tm_sec: c_int,
        tm_min: c_int,
        tm_hour: c_int,
        tm_mday: c_int,
        tm_mon: c_int,
        tm_year: c_int,
        tm_wday: c_int,
        tm_yday: c_int,
        tm_isdst: c_int,
        tm_gmtoff: c_long,
        tm_zone: *const c_char,
        _reserved: [usize; 4],
    }
    extern "C" {
        fn time(t: *mut c_void) -> i64;
        fn localtime_r(timep: *const c_void, result: *mut Tm) -> *mut Tm;
    }
    // 16-byte aligned storage accommodates both 32-bit and 64-bit `time_t` ABIs.
    let mut raw_time = [0i64; 2];
    unsafe {
        time(raw_time.as_mut_ptr().cast());
        let mut tm: Tm = std::mem::zeroed();
        if !localtime_r(raw_time.as_ptr().cast(), &mut tm).is_null() {
            (
                tm.tm_year + 1900,
                (tm.tm_mon + 1) as u32,
                tm.tm_mday as u32,
                tm.tm_hour as u32,
                tm.tm_min as u32,
                tm.tm_sec as u32,
            )
        } else {
            (1970, 1, 1, 0, 0, 0)
        }
    }
}

#[inline]
fn push_two_digits(out: &mut String, val: u32) {
    let v = val % 100;
    out.push((b'0' + (v / 10) as u8) as char);
    out.push((b'0' + (v % 10) as u8) as char);
}

#[inline]
fn push_four_digits(out: &mut String, val: i32) {
    let v = val.unsigned_abs() % 10_000;
    out.push((b'0' + ((v / 1000) % 10) as u8) as char);
    out.push((b'0' + ((v / 100) % 10) as u8) as char);
    out.push((b'0' + ((v / 10) % 10) as u8) as char);
    out.push((b'0' + (v % 10) as u8) as char);
}

fn format_date_into(fmt: &str, out: &mut String) {
    let (year, month, day, hour, minute, second) = get_local_date_time();
    let bytes = fmt.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"yyyy") {
            push_four_digits(out, year);
            i += 4;
        } else if bytes[i..].starts_with(b"dd") {
            push_two_digits(out, day);
            i += 2;
        } else if bytes[i..].starts_with(b"MM") {
            push_two_digits(out, month);
            i += 2;
        } else if bytes[i..].starts_with(b"HH") {
            push_two_digits(out, hour);
            i += 2;
        } else if bytes[i..].starts_with(b"mm") {
            push_two_digits(out, minute);
            i += 2;
        } else if bytes[i..].starts_with(b"ss") {
            push_two_digits(out, second);
            i += 2;
        } else {
            let ch = fmt[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_variable_is_left_for_the_host() {
        let mut table = MacroTable::new();
        table.insert("clip", "prefix-@clipboard-suffix");
        assert_eq!(
            table.get("clip").unwrap(),
            format!("prefix-{CLIPBOARD_PLACEHOLDER}-suffix")
        );
    }
}
