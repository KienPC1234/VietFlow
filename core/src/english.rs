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

/// Everyday English words that are also valid unmarked Vietnamese syllables.
/// Kept as a separate linear list because adding them to the sorted technical
/// list would be error-prone; this is only queried at word boundaries.
pub static COMMON_ENGLISH: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "best", "box", "bring", "but", "by",
    "can", "cast", "cost", "dark", "desk", "disk", "do", "end", "fast", "fix", "for",
    "game", "get", "go", "he", "help", "hen", "her", "here", "hi", "his", "home",
    "host", "how", "i", "if", "in", "is", "it", "its", "just", "king", "last", "let",
    "life", "like", "line", "link", "list", "live", "long", "lost", "love", "mark",
    "mask", "max", "me", "men", "mix", "more", "most", "move", "must", "my", "name",
    "nest", "net", "next", "no", "not", "now", "of", "on", "one", "or", "our", "out",
    "park", "pen", "pet", "ping", "post", "rank", "red", "reset", "rest", "ring",
    "risk", "same", "say", "see", "send", "set", "she", "sing", "six", "so", "some",
    "song", "strong", "task", "tax", "ten", "test", "than", "thank", "that", "the",
    "their", "them", "then", "there", "these", "they", "thing", "think", "this",
    "time", "to", "up", "us", "want", "was", "we", "went", "were", "west", "what",
    "when", "where", "which", "who", "why", "will", "wing", "with", "word", "work",
    "world", "yes", "yet", "you", "your",
];

pub fn is_english_keyword(s: &str) -> bool {
    ENGLISH_KEYWORDS.binary_search(&s).is_ok() || COMMON_ENGLISH.contains(&s)
}

use crate::models::VnLexiName;

pub fn is_foreign_initial(raw_char: char, lexi: VnLexiName, foreign_initials: &[char]) -> bool {
    if matches!(lexi, VnLexiName::uh | VnLexiName::Uh) {
        return false;
    }
    let lower = raw_char.to_ascii_lowercase();
    foreign_initials.contains(&lower)
}
