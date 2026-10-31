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
        let lower = trigger.to_lowercase();
        let template = self.macros.get(&lower)?;
        Some(self.expand_variables(template))
    }

    fn expand_variables(&self, template: &str) -> String {
        if !template.contains('@') {
            return template.to_string();
        }

        let mut result = String::with_capacity(template.len() + 16);
        let chars: Vec<char> = template.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] != '@' {
                result.push(chars[i]);
                i += 1;
                continue;
            }

            let token_start = i + 1;
            let mut token_end = token_start;
            while token_end < chars.len() && (chars[token_end].is_alphanumeric() || chars[token_end] == '_') {
                token_end += 1;
            }

            if token_end == token_start {
                result.push('@');
                i += 1;
                continue;
            }

            let token: String = chars[token_start..token_end].iter().collect();
            if let Some(val) = self.resolve_variable(&token) {
                result.push_str(&val);
            } else {
                result.push('@');
                result.push_str(&token);
            }

            i = token_end;
        }

        result
    }

    fn resolve_variable(&self, token: &str) -> Option<String> {
        match token.to_lowercase().as_str() {
            "newline" => Some("\n".to_string()),
            "tab" => Some("\t".to_string()),
            "date" => Some(format_date(&self.date_format)),
            "time" => Some(format_date(&self.time_format)),
            "datetime" | "now" => Some(format_date(&self.datetime_format)),
            "clipboard" => Some(CLIPBOARD_PLACEHOLDER.to_string()),
            _ => None,
        }
    }
}

#[cfg(windows)]
fn get_local_date_time() -> (i32, u32, u32, u32, u32, u32) {
    #[repr(C)]
    #[allow(non_camel_case_types)]
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
    #[repr(C)]
    struct Tm {
        tm_sec: i32,
        tm_min: i32,
        tm_hour: i32,
        tm_mday: i32,
        tm_mon: i32,
        tm_year: i32,
        tm_wday: i32,
        tm_yday: i32,
        tm_isdst: i32,
    }
    extern "C" {
        fn time(t: *mut i64) -> i64;
        fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
    }
    let mut t: i64 = 0;
    unsafe {
        time(&mut t);
        let mut tm: Tm = std::mem::zeroed();
        if !localtime_r(&t, &mut tm).is_null() {
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

fn format_date(fmt: &str) -> String {
    let (year, month, day, hour, minute, second) = get_local_date_time();

    let mut res = fmt.to_string();
    res = res.replace("dd", &format!("{:02}", day));
    res = res.replace("MM", &format!("{:02}", month));
    res = res.replace("yyyy", &format!("{:04}", year));
    res = res.replace("HH", &format!("{:02}", hour));
    res = res.replace("mm", &format!("{:02}", minute));
    res = res.replace("ss", &format!("{:02}", second));
    res
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
