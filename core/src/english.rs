// VietFlow Core - English & Programming Keyword List
// Licensed under GNU General Public License v3.0 (GPLv3)

pub static ENGLISH_KEYWORDS: &[&str] = &[
    "access", "address", "admin", "afk", "alert", "alex", "alias", "almost", "ambivert", "aoe",
    "apex", "api", "app", "async", "atlas", "auth", "auto", "await", "axis", "babe",
    "bae", "base", "basis", "best", "bestie", "block", "blog", "bonus", "book", "bool",
    "boot", "boss", "branch", "break", "brief", "bro", "buddy", "buff", "bug", "build",
    "business", "button", "byte", "cache", "campus", "canvas", "card", "carry", "cart", "cast",
    "center", "chat", "checkout", "chill", "chip", "circus", "class", "clean", "click", "client",
    "clone", "cloud", "clutch", "code", "collect", "combat", "comment", "commit", "complex", "config",
    "connect", "content", "context", "continue", "controller", "cook", "cool", "cooldown", "copy", "correct",
    "cost", "count", "craft", "crash", "create", "cringe", "crisis", "cross", "crush", "custom",
    "customer", "customize", "customs", "damage", "dark", "data", "date", "dead", "deadline", "deal",
    "debug", "deck", "deep", "default", "delete", "deploy", "desk", "detect", "dev", "direct",
    "disk", "display", "doc", "docker", "document", "double", "draft", "drama", "drift", "driver",
    "drop", "dude", "dust", "east", "edit", "effect", "elo", "email", "enter", "error",
    "event", "exist", "explore", "explorer", "export", "express", "extrovert", "ez", "fail", "fake",
    "false", "farm", "fast", "feed", "feedback", "feel", "feet", "fetch", "file", "filter",
    "find", "firm", "first", "fitness", "fix", "flash", "flat", "flex", "flirt", "float",
    "floor", "flop", "flow", "focus", "follow", "font", "food", "footer", "form", "fps",
    "free", "gain", "game", "gank", "gaslight", "ghost", "gift", "github", "glass", "global",
    "golang", "gold", "good", "green", "greenflag", "grid", "guess", "header", "healing", "help",
    "hide", "highkey", "hint", "hook", "host", "hype", "icon", "idea", "image", "import",
    "impress", "inbox", "index", "info", "inject", "input", "insert", "inspect", "introvert", "issue",
    "issues", "item", "join", "jump", "keep", "kick", "king", "kiss", "kotlin", "last",
    "latex", "lead", "leak", "letter", "lift", "like", "line", "link", "linux", "list",
    "load", "lock", "login", "logo", "look", "loop", "loot", "loss", "lowkey", "mail",
    "main", "mask", "master", "match", "matrix", "meet", "meeting", "menu", "merge", "mess",
    "message", "meta", "minus", "miss", "mist", "mix", "mod", "mode", "model", "mood",
    "move", "museum", "music", "musician", "must", "mvp", "name", "need", "nerf", "news",
    "next", "node", "note", "null", "object", "open", "overrated", "override", "overthinking", "package",
    "page", "paris", "pass", "password", "patch", "path", "pitch", "pixel", "plan", "play",
    "plugin", "plus", "point", "pool", "port", "posix", "post", "prefix", "press", "private",
    "problem", "process", "product", "project", "protect", "protected", "public", "pull", "push", "python",
    "query", "queue", "radius", "rank", "react", "read", "readonly", "real", "reboot", "reconnect",
    "reddit", "redflag", "reels", "reflect", "reject", "relax", "remix", "rep", "report", "request",
    "requests", "require", "requires", "resource", "resources", "response", "responses", "restart", "result",
    "results", "resume", "return", "review", "revive", "risk", "role", "root", "rule", "rust",
    "save", "scam", "scan", "screen", "script", "sealed", "seed", "select", "send", "server",
    "service", "setup", "share", "sheet", "shift", "simp", "site", "size", "skip", "slash",
    "slay", "slide", "smash", "smurf", "soft", "solo", "spam", "spawn", "speed", "spring",
    "staff", "static", "status", "steel", "story", "stress", "strict", "string", "struct", "stun",
    "style", "submit", "subscribe", "success", "suffix", "sugar", "suggest", "summary", "summer", "super",
    "support", "surface", "survey", "survive", "suspect", "sweet", "swift", "switch", "sync", "syntax",
    "system", "target", "task", "taskbar", "taskmgr", "team", "tester", "text", "thesis",
    "throw", "tilt", "time", "token", "tool", "toxic", "trash", "trend", "true", "twist",
    "type", "ult", "ulti", "underrated", "undo", "unfriend", "unit", "unix", "update", "useful",
    "useless", "user", "username", "users", "versus", "vertex", "vibe", "view", "virus", "void",
    "wait", "wake", "walk", "wall", "want", "ward", "warm", "warn", "wash", "water",
    "wave", "wear", "web", "weed", "week", "well", "west", "what", "when", "where",
    "which", "while", "wifi", "wiki", "wild", "will", "win", "wind", "window", "wing",
    "wire", "wise", "wish", "with", "witness", "wood", "wool", "word", "work", "world",
    "wrap", "write", "wrong", "yield", "zero", "zone", "zoom",
];

// Everyday English words that are also valid unmarked Vietnamese syllables
// live here only as *custom* keywords. They are intentionally not enabled by
// default: typing "test"/"reset"/"me" can be a valid Vietnamese keystroke
// sequence, so forcing English would make Vietnamese words impossible to type.

/// Returns true if the string is in the built-in default English keyword dictionary.
pub fn is_english_keyword(s: &str) -> bool {
    ENGLISH_KEYWORDS.binary_search(&s).is_ok()
}

/// Sorted, de-duplicated list used by the settings UI to edit defaults.
pub fn default_english_keywords() -> Vec<&'static str> {
    let mut words: Vec<&'static str> = ENGLISH_KEYWORDS.to_vec();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english_keywords_is_sorted_and_unique() {
        for window in ENGLISH_KEYWORDS.windows(2) {
            assert!(
                window[0] < window[1],
                "ENGLISH_KEYWORDS not sorted or has duplicate: '{}' >= '{}'",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn test_youth_and_tech_words_lookup() {
        let youth_words = [
            "afk", "buff", "nerf", "gank", "clutch", "flex", "crush", "cringe", "slay",
            "toxic", "inbox", "shift", "draft", "craft", "drift", "vibe", "tester", "pass"
        ];
        for word in youth_words {
            assert!(
                is_english_keyword(word),
                "Expected '{word}' to be found in ENGLISH_KEYWORDS"
            );
        }
    }
}
