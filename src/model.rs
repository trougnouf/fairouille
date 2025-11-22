use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use icalendar::{Calendar, CalendarComponent, Component, Todo, TodoStatus};
use rrule::RRuleSet;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use uuid::Uuid; // <--- Added

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)] // <--- Added Derives
pub struct Task {
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub completed: bool,
    pub due: Option<DateTime<Utc>>,
    pub priority: u8,
    pub parent_uid: Option<String>,
    pub etag: String,
    pub href: String,
    pub depth: usize,
    pub rrule: Option<String>,
}

impl Task {
    pub fn apply_smart_input(&mut self, input: &str) {
        let mut summary_words = Vec::new();
        self.priority = 0;
        self.due = None;
        self.rrule = None;

        let mut tokens = input.split_whitespace().peekable();

        while let Some(word) = tokens.next() {
            if word.starts_with('!') {
                if let Ok(p) = word[1..].parse::<u8>() {
                    if p >= 1 && p <= 9 {
                        self.priority = p;
                        continue;
                    }
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
                if let Some(next_token) = tokens.peek() {
                    if let Ok(interval) = next_token.parse::<u32>() {
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
                }
                summary_words.push(word);
                continue;
            }

            if word.starts_with('@') {
                let val = &word[1..];
                if let Ok(date) = NaiveDate::parse_from_str(val, "%Y-%m-%d") {
                    self.due = Some(date.and_hms_opt(23, 59, 59).unwrap().and_utc());
                    continue;
                }
                let now = Local::now().date_naive();
                if val == "today" {
                    self.due = Some(now.and_hms_opt(23, 59, 59).unwrap().and_utc());
                    continue;
                }
                if val == "tomorrow" {
                    let d = now + chrono::Duration::days(1);
                    self.due = Some(d.and_hms_opt(23, 59, 59).unwrap().and_utc());
                    continue;
                }
                if val == "next" {
                    if let Some(unit_token) = tokens.peek() {
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
                            self.due = Some(d.and_hms_opt(23, 59, 59).unwrap().and_utc());
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
        s
    }

    pub fn new(input: &str) -> Self {
        let mut task = Self {
            uid: Uuid::new_v4().to_string(),
            summary: String::new(),
            description: String::new(),
            completed: false,
            due: None,
            priority: 0,
            parent_uid: None,
            etag: String::new(),
            href: String::new(),
            depth: 0,
            rrule: None,
        };
        task.apply_smart_input(input);
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
                next_task.completed = false;
                next_task.due = Some(Utc.from_utc_datetime(&next_due.naive_utc()));
                return Some(next_task);
            }
        }
        None
    }

    pub fn organize_hierarchy(mut tasks: Vec<Task>) -> Vec<Task> {
        let present_uids: HashSet<String> = tasks.iter().map(|t| t.uid.clone()).collect();
        let mut children_map: HashMap<String, Vec<Task>> = HashMap::new();
        let mut roots: Vec<Task> = Vec::new();

        tasks.sort();

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

        if self.completed {
            todo.status(TodoStatus::Completed);
        } else {
            todo.status(TodoStatus::NeedsAction);
        }

        if let Some(dt) = self.due {
            let formatted = dt.format("%Y%m%dT%H%M%SZ").to_string();
            todo.add_property("DUE", &formatted);
        }
        if self.priority > 0 {
            todo.priority(self.priority.into());
        }
        if let Some(p_uid) = &self.parent_uid {
            todo.add_property("RELATED-TO", p_uid.as_str());
        }
        if let Some(rrule) = &self.rrule {
            todo.add_property("RRULE", rrule.as_str());
        }

        let mut calendar = Calendar::new();
        calendar.push(todo);
        calendar.to_string()
    }

    pub fn from_ics(raw_ics: &str, etag: String, href: String) -> Result<Self, String> {
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
        let completed = todo
            .properties()
            .get("STATUS")
            .map(|p| p.value().trim().to_uppercase() == "COMPLETED")
            .unwrap_or(false);
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
                    .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc())
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
        let parent_uid = todo
            .properties()
            .get("RELATED-TO")
            .map(|p| p.value().to_string());
        let rrule = todo
            .properties()
            .get("RRULE")
            .map(|p| p.value().to_string());

        Ok(Task {
            uid,
            summary,
            description,
            completed,
            due,
            priority,
            parent_uid,
            etag,
            href,
            depth: 0,
            rrule,
        })
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.completed != other.completed {
            return self.completed.cmp(&other.completed);
        }
        match (self.due, other.due) {
            (Some(d1), Some(d2)) => {
                if d1 != d2 {
                    return d1.cmp(&d2);
                }
            }
            (Some(_), None) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Greater,
            (None, None) => {}
        }
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
        self.summary.cmp(&other.summary)
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
    use chrono::{Duration, TimeZone};

    #[test]
    fn test_smart_input_priority() {
        let t = Task::new("Buy Milk !1");
        assert_eq!(t.summary, "Buy Milk");
        assert_eq!(t.priority, 1);
        let t2 = Task::new("Low priority !9");
        assert_eq!(t2.priority, 9);
    }

    #[test]
    fn test_smart_input_date_absolute() {
        let t = Task::new("Tax Day @2025-04-15");
        assert_eq!(t.summary, "Tax Day");
        let expected = Utc.with_ymd_and_hms(2025, 4, 15, 23, 59, 59).unwrap();
        assert_eq!(t.due, Some(expected));
    }

    #[test]
    fn test_smart_input_recurrence() {
        let t = Task::new("Gym @daily");
        assert_eq!(t.rrule, Some("FREQ=DAILY".to_string()));
        let t2 = Task::new("Meeting @weekly");
        assert_eq!(t2.rrule, Some("FREQ=WEEKLY".to_string()));
        let t3 = Task::new("Review @every 2 weeks");
        assert_eq!(t3.rrule, Some("FREQ=WEEKLY;INTERVAL=2".to_string()));
    }

    #[test]
    fn test_recurrence_respawn() {
        let yesterday = Utc::now() - Duration::days(1);
        let mut t = Task::new("Daily Task");
        t.due = Some(yesterday);
        t.rrule = Some("FREQ=DAILY".to_string());
        let next = t.respawn().expect("Should respawn");
        assert!(next.due > t.due);
        assert_eq!(next.summary, "Daily Task");
        assert_eq!(next.completed, false);
        assert_ne!(next.uid, t.uid);
    }

    #[test]
    fn test_hierarchy_sorting() {
        let mut t1 = Task::new("Child");
        let mut t2 = Task::new("Root");
        let mut t3 = Task::new("Grandchild");
        t1.uid = "child".to_string();
        t2.uid = "root".to_string();
        t3.uid = "grand".to_string();
        t1.parent_uid = Some("root".to_string());
        t3.parent_uid = Some("child".to_string());
        let raw = vec![t3.clone(), t2.clone(), t1.clone()];
        let organized = Task::organize_hierarchy(raw);
        assert_eq!(organized[0].uid, "root");
        assert_eq!(organized[0].depth, 0);
        assert_eq!(organized[1].uid, "child");
        assert_eq!(organized[1].depth, 1);
        assert_eq!(organized[2].uid, "grand");
        assert_eq!(organized[2].depth, 2);
    }

    #[test]
    fn test_to_smart_string() {
        let mut t = Task::new("Test");
        t.priority = 1;
        t.due = Some(Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap());
        t.rrule = Some("FREQ=WEEKLY".to_string());
        let s = t.to_smart_string();
        assert!(s.contains("Test"));
        assert!(s.contains("!1"));
        assert!(s.contains("@2025-12-31"));
        assert!(s.contains("@weekly"));
    }
}
