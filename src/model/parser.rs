// File: src/model/parser.rs
// Handles smart text input parsing
use crate::model::item::Task;
use chrono::{DateTime, Local, NaiveDate, Utc};
use std::collections::HashMap;

impl Task {
    pub fn apply_smart_input(&mut self, input: &str, aliases: &HashMap<String, Vec<String>>) {
        let mut summary_words = Vec::new();
        // Reset fields
        self.priority = 0;
        self.due = None;
        self.dtstart = None;
        self.rrule = None;
        self.estimated_duration = None;
        self.categories.clear();

        let tokens: Vec<&str> = input.split_whitespace().collect();
        let mut i = 0;

        while i < tokens.len() {
            let word = tokens[i];

            // 1. Priority (!1 - !9)
            if word.starts_with('!')
                && let Ok(p) = word[1..].parse::<u8>()
                && (1..=9).contains(&p)
            {
                self.priority = p;
                i += 1;
                continue;
            }

            // 2. Duration (est:30m, ~30m)
            if let Some(val) = word.strip_prefix("est:").or_else(|| word.strip_prefix('~'))
                && let Some(m) = parse_duration(val)
            {
                self.estimated_duration = Some(m);
                i += 1;
                continue;
            }

            // 3. Tags (#tag)
            if let Some(stripped) = word.strip_prefix('#') {
                let cat = stripped.to_string();
                if !cat.is_empty() {
                    if !self.categories.contains(&cat) {
                        self.categories.push(cat.clone());
                    }

                    // Apply aliases recursively (e.g. #a:b -> check alias for #a:b, then #a)
                    let mut search = cat.as_str();
                    loop {
                        if let Some(expanded_tags) = aliases.get(search) {
                            for extra_tag in expanded_tags {
                                if !self.categories.contains(extra_tag) {
                                    self.categories.push(extra_tag.clone());
                                }
                            }
                        }
                        // Move up hierarchy
                        if let Some(idx) = search.rfind(':') {
                            search = &search[..idx];
                        } else {
                            break;
                        }
                    }

                    i += 1;
                    continue;
                }
            }

            // 4. Recurrence (rec:weekly, @weekly)
            if let Some(val) = word.strip_prefix("rec:").or_else(|| word.strip_prefix('@'))
                && let Some(rrule) = parse_recurrence(val)
            {
                self.rrule = Some(rrule);
                i += 1;
                continue;
            }
            // If not a recurrence keyword, it might be a date using '@' synonym, allow fallthrough

            // 5. Explicit Recurrence with interval (rec:every 2 days)
            // Or synonym (@every 2 days)
            if (word == "rec:every" || word == "@every") && i + 2 < tokens.len() {
                let amount_str = tokens[i + 1];
                let unit_str = tokens[i + 2];
                if let Ok(interval) = amount_str.parse::<u32>() {
                    let freq = parse_freq_unit(unit_str);
                    if !freq.is_empty() {
                        self.rrule = Some(format!("FREQ={};INTERVAL={}", freq, interval));
                        i += 3;
                        continue;
                    }
                }
            }

            // 6. Due Date (due:2025-01-01, @2025-01-01)
            if let Some(val) = word.strip_prefix("due:").or_else(|| word.strip_prefix('@'))
                && let Some(dt) = parse_smart_date(val, true)
            {
                // true = end of day
                self.due = Some(dt);
                i += 1;
                continue;
            }

            // 7. Start Date (start:2025-01-01, ^2025-01-01)
            if let Some(val) = word
                .strip_prefix("start:")
                .or_else(|| word.strip_prefix('^'))
                && let Some(dt) = parse_smart_date(val, false)
            {
                // false = start of day
                self.dtstart = Some(dt);
                i += 1;
                continue;
            }

            // Fallback: Add to summary
            summary_words.push(word);
            i += 1;
        }
        self.summary = summary_words.join(" ");
    }

    pub fn to_smart_string(&self) -> String {
        let mut s = self.summary.clone();

        // Priority: !1
        if self.priority > 0 {
            s.push_str(&format!(" !{}", self.priority));
        }

        // Start: ^YYYY-MM-DD
        if let Some(start) = self.dtstart {
            s.push_str(&format!(" ^{}", start.format("%Y-%m-%d")));
        }

        // Due: @YYYY-MM-DD
        if let Some(d) = self.due {
            s.push_str(&format!(" @{}", d.format("%Y-%m-%d")));
        }

        // Duration: ~30m
        if let Some(mins) = self.estimated_duration {
            let dur_str = if mins >= 525600 {
                format!("~{}y", mins / 525600)
            } else if mins >= 43200 {
                format!("~{}mo", mins / 43200)
            } else if mins >= 10080 {
                format!("~{}w", mins / 10080)
            } else if mins >= 1440 {
                format!("~{}d", mins / 1440)
            } else if mins >= 60 {
                format!("~{}h", mins / 60)
            } else {
                format!("~{}m", mins)
            };
            s.push_str(&format!(" {}", dur_str));
        }

        // Recurrence: @weekly or @every ...
        if let Some(r) = &self.rrule {
            if r == "FREQ=DAILY" {
                s.push_str(" @daily");
            } else if r == "FREQ=WEEKLY" {
                s.push_str(" @weekly");
            } else if r == "FREQ=MONTHLY" {
                s.push_str(" @monthly");
            } else if r == "FREQ=YEARLY" {
                s.push_str(" @yearly");
            } else if let Some(simple) = reconstruct_simple_rrule(r) {
                s.push_str(&format!(" {}", simple));
            } else {
                s.push_str(" rec:custom"); // Fallback for complex RRULEs
            }
        }

        // Tags: #tag
        for cat in &self.categories {
            s.push_str(&format!(" #{}", cat));
        }
        s
    }
}

/// Helper to extract inline alias definitions from an input string.
/// Syntax: #alias=#tag1,#tag2
/// Returns:
/// 1. The cleaned input string (with definitions replaced by just the alias tag: #alias)
/// 2. A HashMap of the extracted definitions.
pub fn extract_inline_aliases(input: &str) -> (String, HashMap<String, Vec<String>>) {
    let mut cleaned_words = Vec::new();
    let mut new_aliases = HashMap::new();

    for token in input.split_whitespace() {
        if token.starts_with('#')
            && token.contains('=')
            && let Some((left, right)) = token.split_once('=')
        {
            let alias_key = left.trim_start_matches('#').to_string();
            if !alias_key.is_empty() && !right.is_empty() {
                let tags: Vec<String> = right
                    .split(',')
                    .map(|t| t.trim().trim_start_matches('#').to_string())
                    .filter(|t| !t.is_empty())
                    .collect();

                if !tags.is_empty() {
                    new_aliases.insert(alias_key.clone(), tags);
                    // Replace the definition with just the alias tag in the output string
                    cleaned_words.push(left.to_string());
                    continue;
                }
            }
        }
        cleaned_words.push(token.to_string());
    }

    (cleaned_words.join(" "), new_aliases)
}

// --- Helpers ---

fn reconstruct_simple_rrule(rrule: &str) -> Option<String> {
    // Basic parser to handle FREQ=X;INTERVAL=Y -> @every Y X(s)
    let parts: HashMap<&str, &str> = rrule.split(';').filter_map(|s| s.split_once('=')).collect();

    let freq = parts.get("FREQ")?;
    let interval = parts.get("INTERVAL").unwrap_or(&"1");

    let unit = match *freq {
        "DAILY" => "days",
        "WEEKLY" => "weeks",
        "MONTHLY" => "months",
        "YEARLY" => "years",
        _ => return None,
    };

    Some(format!("@every {} {}", interval, unit))
}

fn parse_duration(val: &str) -> Option<u32> {
    let lower = val.to_lowercase();
    if let Some(n) = lower.strip_suffix("min") {
        return n.parse::<u32>().ok();
    }
    if let Some(n) = lower.strip_suffix('m') {
        return n.parse::<u32>().ok();
    } else if let Some(n) = lower.strip_suffix('h') {
        return n.parse::<u32>().ok().map(|h| h * 60);
    } else if let Some(n) = lower.strip_suffix('d') {
        return n.parse::<u32>().ok().map(|d| d * 24 * 60);
    } else if let Some(n) = lower.strip_suffix('w') {
        return n.parse::<u32>().ok().map(|w| w * 7 * 24 * 60);
    } else if let Some(n) = lower.strip_suffix("mo") {
        return n.parse::<u32>().ok().map(|mo| mo * 30 * 24 * 60);
    } else if let Some(n) = lower.strip_suffix('y') {
        return n.parse::<u32>().ok().map(|y| y * 365 * 24 * 60);
    }
    None
}

fn parse_recurrence(val: &str) -> Option<String> {
    match val {
        "daily" => Some("FREQ=DAILY".to_string()),
        "weekly" => Some("FREQ=WEEKLY".to_string()),
        "monthly" => Some("FREQ=MONTHLY".to_string()),
        "yearly" => Some("FREQ=YEARLY".to_string()),
        _ => None,
    }
}

fn parse_freq_unit(unit: &str) -> &'static str {
    let u = unit.to_lowercase();
    if u.starts_with("day") {
        "DAILY"
    } else if u.starts_with("week") {
        "WEEKLY"
    } else if u.starts_with("month") {
        "MONTHLY"
    } else if u.starts_with("year") {
        "YEARLY"
    } else {
        ""
    }
}

fn parse_smart_date(val: &str, end_of_day: bool) -> Option<DateTime<Utc>> {
    // 1. Specific Date YYYY-MM-DD
    if let Ok(date) = NaiveDate::parse_from_str(val, "%Y-%m-%d") {
        return finalize_date(date, end_of_day);
    }

    let now = Local::now().date_naive();

    // 2. Relative Keywords
    if val == "today" {
        return finalize_date(now, end_of_day);
    }
    if val == "tomorrow" {
        return finalize_date(now + chrono::Duration::days(1), end_of_day);
    }

    // 3. "1w", "2d" offsets (from now)
    if let Some(n) = val.strip_suffix('d').and_then(|s| s.parse::<i64>().ok()) {
        return finalize_date(now + chrono::Duration::days(n), end_of_day);
    }
    if let Some(n) = val.strip_suffix('w').and_then(|s| s.parse::<i64>().ok()) {
        return finalize_date(now + chrono::Duration::days(n * 7), end_of_day);
    }
    if let Some(n) = val.strip_suffix("mo").and_then(|s| s.parse::<i64>().ok()) {
        return finalize_date(now + chrono::Duration::days(n * 30), end_of_day);
    }
    if let Some(n) = val.strip_suffix('y').and_then(|s| s.parse::<i64>().ok()) {
        return finalize_date(now + chrono::Duration::days(n * 365), end_of_day);
    }

    None
}

fn finalize_date(d: NaiveDate, end_of_day: bool) -> Option<DateTime<Utc>> {
    let t = if end_of_day {
        d.and_hms_opt(23, 59, 59)?
    } else {
        d.and_hms_opt(0, 0, 0)?
    };
    Some(t.and_utc())
}
