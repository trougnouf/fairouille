use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use icalendar::{Calendar, CalendarComponent, Component, Todo, TodoStatus};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Task {
    pub uid: String,
    pub summary: String,
    pub description: String, // <--- NEW FIELD
    pub completed: bool,
    pub due: Option<DateTime<Utc>>,
    pub priority: u8,
    pub parent_uid: Option<String>,
    pub etag: String,
    pub href: String,
    pub depth: usize,
}

impl Task {
    // ... apply_smart_input (unchanged) ...
    pub fn apply_smart_input(&mut self, input: &str) {
        let mut summary_words = Vec::new();
        self.priority = 0;
        self.due = None;

        for word in input.split_whitespace() {
            if word.starts_with('!') {
                if let Ok(p) = word[1..].parse::<u8>() {
                    if p >= 1 && p <= 9 {
                        self.priority = p;
                        continue;
                    }
                }
            }
            if word.starts_with('@') {
                let date_str = &word[1..];
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    self.due = Some(date.and_hms_opt(23, 59, 59).unwrap().and_utc());
                    continue;
                }
                let now = Local::now().date_naive();
                if date_str == "today" {
                    self.due = Some(now.and_hms_opt(23, 59, 59).unwrap().and_utc());
                    continue;
                }
                if date_str == "tomorrow" {
                    let tomorrow = now + chrono::Duration::days(1);
                    self.due = Some(tomorrow.and_hms_opt(23, 59, 59).unwrap().and_utc());
                    continue;
                }
            }
            summary_words.push(word);
        }
        self.summary = summary_words.join(" ");
    }

    // ... to_smart_string (unchanged) ...
    pub fn to_smart_string(&self) -> String {
        let mut s = self.summary.clone();
        if self.priority > 0 {
            s.push_str(&format!(" !{}", self.priority));
        }
        if let Some(d) = self.due {
            s.push_str(&format!(" @{}", d.format("%Y-%m-%d")));
        }
        s
    }

    pub fn new(input: &str) -> Self {
        let mut task = Self {
            uid: Uuid::new_v4().to_string(),
            summary: String::new(),
            description: String::new(), // <--- Init Empty
            completed: false,
            due: None,
            priority: 0,
            parent_uid: None,
            etag: String::new(),
            href: String::new(),
            depth: 0,
        };
        task.apply_smart_input(input);
        task
    }

    // ... organize_hierarchy (unchanged) ...
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
        // --- SAVE DESCRIPTION ---
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

        let mut calendar = Calendar::new();
        calendar.push(todo);
        calendar.to_string()
    }

    pub fn from_ics(raw_ics: &str, etag: String, href: String) -> Result<Self, String> {
        let calendar: Calendar = raw_ics
            .parse()
            .map_err(|e| format!("Failed to parse ICS: {}", e))?;

        let todo = calendar
            .components
            .iter()
            .find_map(|c| match c {
                CalendarComponent::Todo(t) => Some(t),
                _ => None,
            })
            .ok_or("No VTODO found in ICS")?;

        let summary = todo.get_summary().unwrap_or("No Title").to_string();
        // --- LOAD DESCRIPTION ---
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
            } else if val.ends_with('Z') {
                NaiveDateTime::parse_from_str(val, "%Y%m%dT%H%M%SZ")
                    .ok()
                    .map(|d| Utc.from_utc_datetime(&d))
            } else {
                NaiveDateTime::parse_from_str(val, "%Y%m%dT%H%M%S")
                    .ok()
                    .map(|d| Utc.from_utc_datetime(&d))
            }
        });

        let parent_uid = todo
            .properties()
            .get("RELATED-TO")
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
        })
    }
}

// ... Ord/PartialOrd (unchanged) ...
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
