// VietFlow Core - English & Programming Keyword List
// Licensed under GNU General Public License v3.0 (GPLv3)

pub static ENGLISH_KEYWORDS: &[&str] = &[
    "access", "address", "admin", "alert", "alex", "alias", "almost", "apex", "api", "app",
    "async", "atlas", "auth", "auto", "await", "axis", "base", "basis", "blog", "bonus",
    "book", "bool", "boot", "branch", "break", "build", "business", "button", "byte", "cache",
    "campus", "canvas", "card", "cart", "center", "chat", "checkout", "chip", "circus", "class",
    "click", "client", "clone", "cloud", "code", "collect", "commit", "complex", "config", "connect",
    "content", "context", "continue", "controller", "cook", "cool", "copy", "correct", "count", "create",
    "crisis", "cross", "custom", "customer", "customize", "customs", "dark", "data", "date", "dead",
    "deal", "debug", "deck", "deep", "default", "delete", "desk", "detect", "direct", "disk",
    "display", "docker", "document", "double", "driver", "drop", "east", "edit", "effect", "email",
    "enter", "error", "event", "exist", "export", "express", "fail", "false", "feed", "feel",
    "feet", "fetch", "file", "filter", "find", "firm", "first", "fitness", "flat", "float",
    "floor", "flow", "focus", "font", "food", "footer", "form", "free", "gain", "game",
    "github", "global", "golang", "gold", "good", "green", "grid", "guess", "header", "help",
    "hide", "hint", "hook", "icon", "idea", "image", "import", "impress", "index", "info",
    "inject", "input", "insert", "inspect", "issue", "issues", "item", "join", "jump", "keep",
    "kick", "king", "kotlin", "latex", "letter", "line", "link", "linux", "load", "lock",
    "login", "logo", "look", "loop", "mail", "main", "master", "matrix", "meet", "menu",
    "merge", "message", "minus", "mode", "model", "move", "museum", "music", "musician", "name",
    "need", "news", "next", "node", "note", "null", "object", "open", "override", "package",
    "page", "paris", "pass", "password", "patch", "path", "pixel", "plan", "play", "plugin",
    "point", "pool", "port", "posix", "post", "prefix", "press", "private", "problem", "process",
    "product", "project", "protect", "protected", "public", "pull", "push", "python", "query", "queue",
    "radius", "rank", "react", "read", "readonly", "reddit", "reflect", "reject", "relax", "remix",
    "request", "requests", "require", "requires", "resource", "resources", "response", "responses", "result", "results",
    "resume", "return", "role", "root", "rule", "save", "scan", "screen", "script", "sealed",
    "seed", "select", "send", "server", "service", "share", "site", "size", "skip", "speed",
    "spring", "static", "status", "steel", "stress", "strict", "string", "struct", "style", "submit",
    "subscribe", "success", "suffix", "sugar", "suggest", "summary", "summer", "super", "support", "surface",
    "survey", "survive", "suspect", "sweet", "swift", "switch", "syntax", "system", "target", "task",
    "team", "text", "thesis", "throw", "time", "token", "tool", "true", "twist", "type",
    "unit", "unix", "update", "useful", "useless", "user", "username", "users", "versus", "vertex",
    "view", "virus", "void", "wait", "wake", "walk", "wall", "want", "ward", "warm",
    "warn", "wash", "water", "wave", "wear", "web", "weed", "week", "well", "west",
    "what", "when", "where", "which", "while", "wifi", "wiki", "wild", "will", "win",
    "wind", "window", "wing", "wire", "wise", "wish", "with", "witness", "wood", "wool",
    "word", "work", "world", "wrap", "write", "wrong", "yield", "zero", "zone", "zoom",
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
