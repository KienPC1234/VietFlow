// VietFlow Core - English & Programming Keyword List
// Licensed under GNU General Public License v3.0 (GPLv3)

pub static ENGLISH_KEYWORDS: &[&str] = &[
    "access", "address", "alex", "alias", "almost", "apex", "async", "atlas", "await", "axis",
    "basis", "bonus", "book", "bool", "boot", "branch", "break", "business", "button", "byte",
    "campus", "canvas", "center", "char", "checkout", "circus", "class", "click", "code", "collect",
    "commit", "complex", "config", "connect", "content", "context", "continue", "controller", "cook", "cool",
    "correct", "create", "crisis", "cross", "custom", "customer", "customize", "customs", "debug", "deep",
    "default", "delete", "detect", "direct", "display", "docker", "document", "double", "driver", "east",
    "effect", "enter", "event", "exist", "export", "express", "false", "feed", "feel", "feet",
    "fetch", "filter", "first", "fitness", "float", "floor", "focus", "food", "footer", "free",
    "github", "golang", "good", "green", "guess", "header", "hook", "import", "impress", "index",
    "inject", "insert", "inspect", "issue", "issues", "item", "keep", "kotlin", "latex", "letter",
    "linux", "look", "loop", "master", "matrix", "meet", "merge", "message", "minus", "model",
    "museum", "music", "musician", "need", "next", "null", "object", "override", "package", "paris",
    "password", "pixel", "pool", "posix", "prefix", "press", "private", "problem", "process", "product",
    "project", "protect", "protected", "public", "pull", "push", "python", "radius", "react", "readonly",
    "reddit", "reflect", "reject", "relax", "remix", "request", "requests", "require", "requires", "resource",
    "resources", "response", "responses", "result", "results", "resume", "return", "root", "rust", "screen",
    "script", "sealed", "seed", "select", "server", "service", "speed", "spring", "static", "status",
    "steel", "stress", "strict", "string", "struct", "style", "submit", "subscribe", "success", "suffix",
    "sugar", "suggest", "summary", "summer", "super", "support", "surface", "survey", "survive", "suspect",
    "sweet", "swift", "switch", "syntax", "system", "target", "text", "thesis", "throw", "tool",
    "true", "twist", "type", "unix", "update", "useful", "useless", "user", "username", "users",
    "versus", "vertex", "view", "virus", "void", "water", "weed", "week", "while", "window",
    "witness", "wood", "wool", "worst", "yield", "zoom",
];

/// Everyday English words that are also valid unmarked Vietnamese syllables
/// live here only as *custom* keywords. They are intentionally not enabled by
/// default: typing "test"/"reset"/"me" can be a valid Vietnamese keystroke
/// sequence, so forcing English would make Vietnamese words impossible to type.

pub fn is_english_keyword(s: &str) -> bool {
    ENGLISH_KEYWORDS.binary_search(&s).is_ok()
}

/// Sorted, de-duplicated list used by the settings UI to edit defaults.
pub fn default_english_keywords() -> Vec<&'static str> {
    let mut words: Vec<&'static str> = ENGLISH_KEYWORDS.iter().copied().collect();
    words.sort_unstable();
    words.dedup();
    words
}

use crate::models::VnLexiName;

pub fn is_foreign_initial(raw_char: char, lexi: VnLexiName, foreign_initials: &[char]) -> bool {
    if matches!(lexi, VnLexiName::uh | VnLexiName::Uh) {
        return false;
    }
    let lower = raw_char.to_ascii_lowercase();
    foreign_initials.contains(&lower)
}
