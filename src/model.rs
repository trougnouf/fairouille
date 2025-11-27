use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use icalendar::{Calendar, CalendarComponent, Component, Todo, TodoStatus};
use rrule::RRuleSet;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    NeedsAction,
    InProcess,
    Completed,
    Cancelled,
}

impl TaskStatus {
    pub fn is_done(&self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub status: TaskStatus,
    pub due: Option<DateTime<Utc>>,
    pub priority: u8,
    pub parent_uid: Option<String>,
    pub dependencies: Vec<String>,
    pub etag: String,
    pub href: String,
    pub calendar_href: String,
    pub categories: Vec<String>,
    pub depth: usize,
    pub rrule: Option<String>,
}

impl Task {
    pub fn apply_smart_input(&mut self, input: &str, aliases: &HashMap<String, Vec<String>>) {
        let mut summary_words = Vec::new();
        self.priority = 0;
        self.due = None;
        self.rrule = None;
        self.categories.clear();

        let mut tokens = input.split_whitespace().peekable();

        while let Some(word) = tokens.next() {
            if word.starts_with('!')
                && let Ok(p) = word[1..].parse::<u8>()
                && (1..=9).contains(&p)
            {
                self.priority = p;
                continue;
            }

            // 2. Categories (#tag)
            if let Some(stripped) = word.strip_prefix('#') {
                let cat = stripped.to_string();
                if !cat.is_empty() {
                    if !self.categories.contains(&cat) {
                        self.categories.push(cat.clone());
                    }
                    if let Some(expanded_tags) = aliases.get(&cat) {
                        for extra_tag in expanded_tags {
                            if !self.categories.contains(extra_tag) {
                                self.categories.push(extra_tag.clone());
                            }
                        }
                    }
                    continue;
                }
            }

            if word == "@daily" {
                self.rrule = Some("FREQ=DAILY".to_string());
                continue;
            }
            if word == "@weekly" {
                self.rrule = Some("FREQ=WEEKLY".to_string());
                continue;
            }
            if word == "@monthly" {
                self.rrule = Some("FREQ=MONTHLY".to_string());
                continue;
            }
            if word == "@yearly" {
                self.rrule = Some("FREQ=YEARLY".to_string());
                continue;
            }

            if word == "@every" {
                if let Some(next_token) = tokens.peek()
                    && let Ok(interval) = next_token.parse::<u32>()
                {
                    tokens.next();
                    if let Some(unit_token) = tokens.peek() {
                        let unit = unit_token.to_lowercase();
                        let freq = if unit.starts_with("day") {
                            "DAILY"
                        } else if unit.starts_with("week") {
                            "WEEKLY"
                        } else if unit.starts_with("month") {
                            "MONTHLY"
                        } else if unit.starts_with("year") {
                            "YEARLY"
                        } else {
                            ""
                        };

                        if !freq.is_empty() {
                            tokens.next();
                            self.rrule = Some(format!("FREQ={};INTERVAL={}", freq, interval));
                            continue;
                        }
                    }
                }
                summary_words.push(word);
                continue;
            }

            if let Some(val) = word.strip_prefix('@') {
                if let Ok(date) = NaiveDate::parse_from_str(val, "%Y-%m-%d")
                    && let Some(dt) = date.and_hms_opt(23, 59, 59)
                {
                    self.due = Some(dt.and_utc());
                    continue;
                }
                let now = Local::now().date_naive();
                if val == "today"
                    && let Some(dt) = now.and_hms_opt(23, 59, 59)
                {
                    self.due = Some(dt.and_utc());
                    continue;
                }
                if val == "tomorrow" {
                    let d = now + chrono::Duration::days(1);
                    if let Some(dt) = d.and_hms_opt(23, 59, 59) {
                        self.due = Some(dt.and_utc());
                        continue;
                    }
                }
                if val == "next"
                    && let Some(unit_token) = tokens.peek()
                {
                    let unit = unit_token.to_lowercase();
                    let mut offset = 0;
                    if unit.starts_with("week") {
                        offset = 7;
                    } else if unit.starts_with("month") {
                        offset = 30;
                    } else if unit.starts_with("year") {
                        offset = 365;
                    }

                    if offset > 0 {
                        tokens.next();
                        let d = now + chrono::Duration::days(offset);
                        if let Some(dt) = d.and_hms_opt(23, 59, 59) {
                            self.due = Some(dt.and_utc());
                            continue;
                        }
                    }
                }
            }
            summary_words.push(word);
        }
        self.summary = summary_words.join(" ");
    }

    pub fn to_smart_string(&self) -> String {
        let mut s = self.summary.clone();
        if self.priority > 0 {
            s.push_str(&format!(" !{}", self.priority));
        }
        if let Some(d) = self.due {
            s.push_str(&format!(" @{}", d.format("%Y-%m-%d")));
        }
        if let Some(r) = &self.rrule {
            if r == "FREQ=DAILY" {
                s.push_str(" @daily");
            } else if r == "FREQ=WEEKLY" {
                s.push_str(" @weekly");
            } else if r == "FREQ=MONTHLY" {
                s.push_str(" @monthly");
            } else if r == "FREQ=YEARLY" {
                s.push_str(" @yearly");
            }
        }
        for cat in &self.categories {
            s.push_str(&format!(" #{}", cat));
        }
        s
    }

    pub fn new(input: &str, aliases: &HashMap<String, Vec<String>>) -> Self {
        let mut task = Self {
            uid: Uuid::new_v4().to_string(),
            summary: String::new(),
            description: String::new(),
            status: TaskStatus::NeedsAction,
            due: None,
            priority: 0,
            parent_uid: None,
            dependencies: Vec::new(),
            etag: String::new(),
            href: String::new(),
            calendar_href: String::new(),
            categories: Vec::new(),
            depth: 0,
            rrule: None,
        };
        task.apply_smart_input(input, aliases);
        task
    }

    pub fn respawn(&self) -> Option<Task> {
        let rule_str = self.rrule.as_ref()?;
        let due_utc = self.due?;
        let dtstart = due_utc.format("%Y%m%dT%H%M%SZ").to_string();
        let rrule_string = format!("DTSTART:{}\nRRULE:{}", dtstart, rule_str);

        if let Ok(rrule_set) = RRuleSet::from_str(&rrule_string) {
            let result = rrule_set.all(2);
            let dates = result.dates;
            if dates.len() > 1 {
                let next_due = dates[1];
                let mut next_task = self.clone();
                next_task.uid = Uuid::new_v4().to_string();
                next_task.href = String::new();
                next_task.etag = String::new();
                next_task.status = TaskStatus::NeedsAction;
                next_task.due = Some(Utc.from_utc_datetime(&next_due.naive_utc()));
                next_task.dependencies.clear();
                return Some(next_task);
            }
        }
        None
    }

    pub fn compare_with_cutoff(&self, other: &Self, cutoff: Option<DateTime<Utc>>) -> Ordering {
        // 1. Sort by Status Priority
        // InProcess (0) < NeedsAction (1) < Completed (2) < Cancelled (3)
        fn status_prio(s: TaskStatus) -> u8 {
            match s {
                TaskStatus::InProcess => 0,
                TaskStatus::NeedsAction => 1,
                TaskStatus::Completed => 2,
                TaskStatus::Cancelled => 3,
            }
        }

        let s1 = status_prio(self.status);
        let s2 = status_prio(other.status);
        if s1 != s2 {
            return s1.cmp(&s2);
        }

        // Helper to check if a task is within the "Timed High Priority" window
        let is_in_window = |t: &Task| -> bool {
            match (t.due, cutoff) {
                (Some(d), Some(limit)) => d <= limit, // It has a date, and it's before the limit
                (Some(_), None) => true,              // No limit? All dates are "in window"
                (None, _) => false,                   // No date? Never in window
            }
        };

        let self_in = is_in_window(self);
        let other_in = is_in_window(other);

        match (self_in, other_in) {
            // Case A: Both are in the "Immediate Window" -> Sort by Date
            (true, true) => {
                if self.due != other.due {
                    return self.due.cmp(&other.due);
                }
            }
            // Case B: Only one is in window -> It wins
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            // Case C: Neither in window (Far future OR No Date) -> Sort by Priority
            (false, false) => {}
        }

        // Priority Sort (1 is High, 9 is Low, 0 is None/Lowest)
        // We normalize 0 to 10 for comparison so 1 < 5 < 9 < 0
        let p1 = if self.priority == 0 {
            10
        } else {
            self.priority
        };
        let p2 = if other.priority == 0 {
            10
        } else {
            other.priority
        };

        if p1 != p2 {
            return p1.cmp(&p2);
        }

        // Tie-breaker: If priorities are equal, but one has a date (even if far future),
        // stick to Date then Alphabetical.
        match (self.due, other.due) {
            (Some(d1), Some(d2)) => {
                if d1 != d2 {
                    return d1.cmp(&d2);
                }
            }
            (Some(_), None) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Greater,
            _ => {}
        }

        self.summary.cmp(&other.summary)
    }

    pub fn organize_hierarchy(mut tasks: Vec<Task>, cutoff: Option<DateTime<Utc>>) -> Vec<Task> {
        let present_uids: HashSet<String> = tasks.iter().map(|t| t.uid.clone()).collect();
        let mut children_map: HashMap<String, Vec<Task>> = HashMap::new();
        let mut roots: Vec<Task> = Vec::new();

        tasks.sort_by(|a, b| a.compare_with_cutoff(b, cutoff));

        for mut task in tasks {
            let is_orphan = match &task.parent_uid {
                Some(p_uid) => !present_uids.contains(p_uid),
                None => true,
            };

            if is_orphan {
                if task.parent_uid.is_some() {
                    task.depth = 0;
                }
                roots.push(task);
            } else {
                let p_uid = task.parent_uid.as_ref().unwrap().clone();
                children_map.entry(p_uid).or_default().push(task);
            }
        }

        let mut result = Vec::new();
        for root in roots {
            Self::append_task_and_children(&root, &mut result, &children_map, 0);
        }
        result
    }

    fn append_task_and_children(
        task: &Task,
        result: &mut Vec<Task>,
        map: &HashMap<String, Vec<Task>>,
        depth: usize,
    ) {
        let mut t = task.clone();
        t.depth = depth;
        result.push(t);
        if let Some(children) = map.get(&task.uid) {
            for child in children {
                Self::append_task_and_children(child, result, map, depth + 1);
            }
        }
    }

    pub fn to_ics(&self) -> String {
        let mut todo = Todo::new();
        todo.uid(&self.uid);
        todo.summary(&self.summary);
        if !self.description.is_empty() {
            todo.description(&self.description);
        }
        todo.timestamp(Utc::now());

        match self.status {
            TaskStatus::NeedsAction => todo.status(TodoStatus::NeedsAction),
            TaskStatus::InProcess => todo.status(TodoStatus::InProcess),
            TaskStatus::Completed => todo.status(TodoStatus::Completed),
            TaskStatus::Cancelled => todo.status(TodoStatus::Cancelled),
        };

        if let Some(dt) = self.due {
            let formatted = dt.format("%Y%m%dT%H%M%SZ").to_string();
            todo.add_property("DUE", &formatted);
        }
        if self.priority > 0 {
            todo.priority(self.priority.into());
        }
        if let Some(rrule) = &self.rrule {
            todo.add_property("RRULE", rrule.as_str());
        }
        // NOTE: We do NOT add categories here using the library.
        // The library escapes all commas, turning "A,B" into "A\,B", which treats it as one tag.
        // We manually inject the correctly formatted line below.

        // --- HIERARCHY & DEPENDENCIES ---
        // Use append_multi_property to support multiple RELATED-TO lines.
        if let Some(p_uid) = &self.parent_uid {
            let prop = icalendar::Property::new("RELATED-TO", p_uid.as_str());
            todo.append_multi_property(prop);
        }

        for dep_uid in &self.dependencies {
            let mut prop = icalendar::Property::new("RELATED-TO", dep_uid);
            prop.add_parameter("RELTYPE", "DEPENDS-ON");
            todo.append_multi_property(prop);
        }

        let mut calendar = Calendar::new();
        calendar.push(todo);
        let mut ics = calendar.to_string();

        // Manual injection of CATEGORIES to handle comma separation correctly
        if !self.categories.is_empty() {
            // 1. Escape commas inside tag names, but join tags with raw commas
            let escaped_cats: Vec<String> = self
                .categories
                .iter()
                .map(|c| c.replace(',', "\\,"))
                .collect();
            let cat_line = format!("CATEGORIES:{}", escaped_cats.join(","));

            // 2. Insert before END:VTODO
            // We assume standard formatting where END:VTODO is at the end
            if let Some(idx) = ics.rfind("END:VTODO") {
                // Simple insertion (Note: strictly this should be folded if >75 chars,
                // but most parsers handle long lines. We keep it simple for now).
                let (start, end) = ics.split_at(idx);
                ics = format!("{}{}\r\n{}", start, cat_line, end);
            }
        }

        ics
    }

    pub fn from_ics(
        raw_ics: &str,
        etag: String,
        href: String,
        calendar_href: String,
    ) -> Result<Self, String> {
        let calendar: Calendar = raw_ics.parse().map_err(|e| format!("Parse: {}", e))?;
        let todo = calendar
            .components
            .iter()
            .find_map(|c| match c {
                CalendarComponent::Todo(t) => Some(t),
                _ => None,
            })
            .ok_or("No VTODO")?;

        let summary = todo.get_summary().unwrap_or("No Title").to_string();
        let description = todo.get_description().unwrap_or("").to_string();
        let uid = todo.get_uid().unwrap_or_default().to_string();

        let status = if let Some(prop) = todo.properties().get("STATUS") {
            match prop.value().trim().to_uppercase().as_str() {
                "COMPLETED" => TaskStatus::Completed,
                "IN-PROCESS" => TaskStatus::InProcess,
                "CANCELLED" => TaskStatus::Cancelled,
                _ => TaskStatus::NeedsAction,
            }
        } else {
            TaskStatus::NeedsAction
        };
        let priority = todo
            .properties()
            .get("PRIORITY")
            .and_then(|p| p.value().parse::<u8>().ok())
            .unwrap_or(0);

        let due = todo.properties().get("DUE").and_then(|p| {
            let val = p.value();
            if val.len() == 8 {
                NaiveDate::parse_from_str(val, "%Y%m%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(23, 59, 59))
                    .map(|d| d.and_utc())
            } else {
                NaiveDateTime::parse_from_str(
                    val,
                    if val.ends_with('Z') {
                        "%Y%m%dT%H%M%SZ"
                    } else {
                        "%Y%m%dT%H%M%S"
                    },
                )
                .ok()
                .map(|d| Utc.from_utc_datetime(&d))
            }
        });

        let rrule = todo
            .properties()
            .get("RRULE")
            .map(|p| p.value().to_string());

        let mut categories = Vec::new();
        if let Some(multi_props) = todo.multi_properties().get("CATEGORIES") {
            for prop in multi_props {
                let parts: Vec<String> = prop
                    .value()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                categories.extend(parts);
            }
        }
        if let Some(prop) = todo.properties().get("CATEGORIES") {
            let parts: Vec<String> = prop
                .value()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            categories.extend(parts);
        }
        categories.sort();
        categories.dedup();

        // --- MANUAL RELATED-TO PARSING (Fix for library overwrite issue) ---
        let mut parent_uid = None;
        let mut dependencies = Vec::new();

        // Standard Parse (if lucky)
        let mut related_props = Vec::new();
        if let Some(multi) = todo.multi_properties().get("RELATED-TO") {
            related_props.extend(multi.iter());
        }
        if let Some(single) = todo.properties().get("RELATED-TO") {
            related_props.push(single);
        }

        // Manual Parse (Fallback for lost duplicates)
        // Unfold lines (remove CRLF+Space)
        let unfolded = raw_ics.replace("\r\n ", "").replace("\n ", "");

        for line in unfolded.lines() {
            if line.starts_with("RELATED-TO")
                && let Some((key_part, value)) = line.split_once(':')
            {
                let value = value.trim().to_string();
                let key_upper = key_part.to_uppercase();

                if key_upper.contains("RELTYPE=DEPENDS-ON") {
                    if !dependencies.contains(&value) {
                        dependencies.push(value);
                    }
                } else if !key_upper.contains("RELTYPE=") || key_upper.contains("RELTYPE=PARENT") {
                    // Only set parent if not already found (or overwrite if multiple? RFC says 1 parent)
                    parent_uid = Some(value);
                }
            }
        }

        Ok(Task {
            uid,
            summary,
            description,
            status,
            due,
            priority,
            parent_uid,
            dependencies,
            etag,
            href,
            calendar_href,
            categories,
            depth: 0,
            rrule,
        })
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        // Use the same comparison logic but without cutoff (always None)
        self.compare_with_cutoff(other, None)
    }
}
impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct CalendarListEntry {
    pub name: String,
    pub href: String,
    pub color: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_smart_input_basics() {
        let aliases = HashMap::new();
        let t = Task::new("Buy cat food !1", &aliases);
        assert_eq!(t.summary, "Buy cat food");
        assert_eq!(t.priority, 1);
    }

    #[test]
    fn test_smart_input_categories() {
        let aliases = HashMap::new();
        let t = Task::new("Project meeting #work #urgent @tomorrow", &aliases);
        assert!(t.summary.contains("Project meeting"));
        assert!(t.categories.contains(&"work".to_string()));
        assert!(t.categories.contains(&"urgent".to_string()));
        assert!(t.due.is_some());
    }

    #[test]
    fn test_smart_input_aliases() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "cfait".to_string(),
            vec!["dev".to_string(), "rust".to_string()],
        );
        let t = Task::new("Coding session #cfait", &aliases);
        assert!(t.categories.contains(&"cfait".to_string()));
        assert!(t.categories.contains(&"dev".to_string()));
        assert!(t.categories.contains(&"rust".to_string()));
    }

    #[test]
    fn test_smart_input_recurrence() {
        let aliases = HashMap::new();
        let t = Task::new("Gym @daily", &aliases);
        assert_eq!(t.rrule, Some("FREQ=DAILY".to_string()));
        let t2 = Task::new("Review @every 2 weeks", &aliases);
        assert_eq!(t2.rrule, Some("FREQ=WEEKLY;INTERVAL=2".to_string()));
    }

    #[test]
    fn test_ical_roundtrip_dependencies() {
        let aliases = HashMap::new();
        let mut t = Task::new("Blocked Task", &aliases);
        t.dependencies.push("blocker-1-uid".to_string());
        t.dependencies.push("blocker-2-uid".to_string());
        t.parent_uid = Some("parent-uid".to_string());

        let ics = t.to_ics();
        println!("Generated ICS:\n{}", ics);

        let t2 =
            Task::from_ics(&ics, "etag".into(), "href".into(), "cal".into()).expect("Parse failed");

        assert_eq!(t2.dependencies.len(), 2);
        assert!(t2.dependencies.contains(&"blocker-1-uid".to_string()));
        assert!(t2.dependencies.contains(&"blocker-2-uid".to_string()));
        assert_eq!(t2.parent_uid, Some("parent-uid".to_string()));
    }

    #[test]
    fn test_respawn_daily_logic() {
        let aliases = HashMap::new();
        let mut t = Task::new("Daily Grind", &aliases);
        let start_date = Utc.with_ymd_and_hms(2025, 1, 1, 9, 0, 0).unwrap();
        t.due = Some(start_date);
        t.rrule = Some("FREQ=DAILY".to_string());
        let next_task = t.respawn().expect("Should generate next task");
        let expected = Utc.with_ymd_and_hms(2025, 1, 2, 9, 0, 0).unwrap();
        assert_eq!(next_task.due, Some(expected));
        assert!(next_task.dependencies.is_empty());
    }

    #[test]
    fn test_respawn_weekly_logic() {
        let aliases = HashMap::new();
        let mut t = Task::new("Weekly Meeting", &aliases);
        let start_date = Utc.with_ymd_and_hms(2025, 1, 1, 9, 0, 0).unwrap();
        t.due = Some(start_date);
        t.rrule = Some("FREQ=WEEKLY".to_string());
        let next_task = t.respawn().expect("Should generate next task");
        let expected = Utc.with_ymd_and_hms(2025, 1, 8, 9, 0, 0).unwrap();
        assert_eq!(next_task.due, Some(expected));
    }

    #[test]
    fn test_hierarchy_sorting() {
        let aliases = HashMap::new();
        let mut t1 = Task::new("Child", &aliases);
        let mut t2 = Task::new("Root", &aliases);
        let mut t3 = Task::new("Grandchild", &aliases);
        t1.uid = "child".to_string();
        t2.uid = "root".to_string();
        t3.uid = "grand".to_string();
        t1.parent_uid = Some("root".to_string());
        t3.parent_uid = Some("child".to_string());
        let raw = vec![t3.clone(), t2.clone(), t1.clone()];
        let organized = Task::organize_hierarchy(raw, None);
        assert_eq!(organized[0].uid, "root");
        assert_eq!(organized[0].depth, 0);
        assert_eq!(organized[1].uid, "child");
        assert_eq!(organized[1].depth, 1);
        assert_eq!(organized[2].uid, "grand");
        assert_eq!(organized[2].depth, 2);
    }

    #[test]
    fn test_to_ics_categories_format() {
        let aliases = HashMap::new();
        let mut t = Task::new("Tag Test", &aliases);
        // Add multiple tags
        t.categories = vec!["Work".to_string(), "Urgent".to_string()];

        let ics = t.to_ics();
        println!("ICS Output:\n{}", ics);

        // Expect single line with comma separator (NOT escaped)
        assert!(ics.contains("CATEGORIES:Urgent,Work") || ics.contains("CATEGORIES:Work,Urgent"));
        // Ensure no backslash before the comma between tags
        assert!(!ics.contains("Work\\,Urgent"));
    }

    #[test]
    fn test_to_ics_categories_with_internal_comma() {
        let aliases = HashMap::new();
        let mut t = Task::new("Comma Test", &aliases);
        // Tag with a comma in the name
        t.categories = vec!["City, Country".to_string(), "Travel".to_string()];

        let ics = t.to_ics();

        // Expect the comma INSIDE the tag to be escaped, but the separator NOT to be
        // e.g. CATEGORIES:City\, Country,Travel
        assert!(ics.contains("City\\, Country"));
        assert!(ics.contains("City\\, Country,Travel") || ics.contains("Travel,City\\, Country"));
    }
}
