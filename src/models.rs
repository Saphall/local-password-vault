use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const HISTORY_LIMIT: usize = 5;

#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub changed: i64,
    pub password: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Entry {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub username: String,
    pub password: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub created: i64,
    pub updated: i64,
    pub history: Vec<HistoryItem>,
}

impl Entry {
    /// Replaces the password and keeps the old one in a bounded history.
    pub fn set_password(&mut self, new: String, now: i64) {
        if self.password == new {
            return;
        }
        let old = std::mem::replace(&mut self.password, new);
        self.history.insert(0, HistoryItem { changed: now, password: old });
        self.history.truncate(HISTORY_LIMIT);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Strength {
    Weak,
    Fair,
    Strong,
}

impl Strength {
    pub fn label(self) -> &'static str {
        match self {
            Strength::Weak => "Weak",
            Strength::Fair => "Fair",
            Strength::Strong => "Strong",
        }
    }

    pub fn fraction(self) -> f32 {
        match self {
            Strength::Weak => 0.33,
            Strength::Fair => 0.66,
            Strength::Strong => 1.0,
        }
    }
}

pub fn strength(password: &str) -> Strength {
    let classes = [
        password.chars().any(|c| c.is_ascii_lowercase()),
        password.chars().any(|c| c.is_ascii_uppercase()),
        password.chars().any(|c| c.is_ascii_digit()),
        password.chars().any(|c| !c.is_ascii_alphanumeric()),
    ]
    .iter()
    .filter(|&&b| b)
    .count();
    let len = password.chars().count();

    if len < 8 || classes < 2 {
        Strength::Weak
    } else if len >= 12 && classes >= 3 {
        Strength::Strong
    } else {
        Strength::Fair
    }
}

pub fn generate_password(len: usize) -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*-_=+?";
    loop {
        let pw: String = (0..len)
            .map(|_| CHARS[rand::random_range(0..CHARS.len())] as char)
            .collect();
        if len < 12 || strength(&pw) == Strength::Strong {
            return pw;
        }
    }
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// "Today" for the current UTC day, otherwise `YYYY-MM-DD`.
pub fn format_date(ts: i64, now: i64) -> String {
    let days = ts.div_euclid(86_400);
    if days == now.div_euclid(86_400) {
        return "Today".into();
    }
    let (y, m, d) = civil_from_days(days);
    format!("{y}-{m:02}-{d:02}")
}

// Howard Hinnant's days-to-civil algorithm.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_000), (2024, 10, 4));
    }

    #[test]
    fn strength_levels() {
        assert_eq!(strength("abc"), Strength::Weak);
        assert_eq!(strength("abcdefgh1"), Strength::Fair);
        assert_eq!(strength("Abcdefgh1234!"), Strength::Strong);
    }
}
