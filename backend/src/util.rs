use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;

static ANSI_CSI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap());
static ANSI_OSC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\].*?\x07").unwrap());
static ANSI_ESC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b.").unwrap());

pub fn strip_ansi(s: &str) -> String {
    let s = ANSI_CSI.replace_all(s, "");
    let s = ANSI_OSC.replace_all(&s, "");
    let s = ANSI_ESC.replace_all(&s, "");
    s.into_owned()
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
