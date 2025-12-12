// File: src/model/adapter.rs
use crate::model::item::{RawProperty, Task, TaskStatus};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use icalendar::{Calendar, CalendarComponent, Component, Todo, TodoStatus};
use rrule::RRuleSet;
use std::str::FromStr;
use uuid::Uuid;

const HANDLED_KEYS: &[&str] = &[
    "UID",
    "SUMMARY",
    "DESCRIPTION",
    "STATUS",
    "PRIORITY",
    "DUE",
    "DTSTART",
    "RRULE",
    "DURATION",
    "X-ESTIMATED-DURATION",
    "CATEGORIES",
    "RELATED-TO",
    "DTSTAMP",
    "CREATED",
    "LAST-MODIFIED",
    "SEQUENCE",
    "PRODID",
    "VERSION",
    "CALSCALE",
];

impl Task {
    pub fn respawn(&self) -> Option<Task> {
        let rule_str = self.rrule.as_ref()?;
        let seed_date = self.dtstart.or(self.due)?;

        let dtstart_str = seed_date.format("%Y%m%dT%H%M%SZ").to_string();
        let rrule_string = format!("DTSTART:{}\nRRULE:{}", dtstart_str, rule_str);

        if let Ok(rrule_set) = RRuleSet::from_str(&rrule_string) {
            let result = rrule_set.all(2);
            let dates = result.dates;
            if dates.len() > 1 {
                let next_occurrence = dates[1];
                let next_start = Utc.from_utc_datetime(&next_occurrence.naive_utc());

                let mut next_task = self.clone();
                next_task.uid = Uuid::new_v4().to_string();
                next_task.href = String::new();
                next_task.etag = String::new();
                next_task.status = TaskStatus::NeedsAction;
                next_task.dependencies.clear();

                if self.dtstart.is_some() {
                    next_task.dtstart = Some(next_start);
                }

                if let Some(old_due) = self.due {
                    let duration = old_due - seed_date;
                    next_task.due = Some(next_start + duration);
                }

                return Some(next_task);
            }
        }
        None
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

        fn format_iso_duration(mins: u32) -> String {
            if mins.is_multiple_of(24 * 60) {
                format!("P{}D", mins / (24 * 60))
            } else if mins.is_multiple_of(60) {
                format!("PT{}H", mins / 60)
            } else {
                format!("PT{}M", mins)
            }
        }

        if let Some(dt) = self.dtstart {
            let formatted = dt.format("%Y%m%dT%H%M%SZ").to_string();
            todo.add_property("DTSTART", &formatted);
        }

        if let Some(dt) = self.due {
            let formatted = dt.format("%Y%m%dT%H%M%SZ").to_string();
            todo.add_property("DUE", &formatted);
            if let Some(mins) = self.estimated_duration {
                let val = format_iso_duration(mins);
                todo.add_property("X-ESTIMATED-DURATION", &val);
            }
        } else if let Some(mins) = self.estimated_duration {
            let val = format_iso_duration(mins);
            todo.add_property("DURATION", &val);
        }
        if self.priority > 0 {
            todo.priority(self.priority.into());
        }
        if let Some(rrule) = &self.rrule {
            todo.add_property("RRULE", rrule.as_str());
        }

        // --- HIERARCHY & DEPENDENCIES ---
        if let Some(p_uid) = &self.parent_uid {
            let prop = icalendar::Property::new("RELATED-TO", p_uid.as_str());
            todo.append_multi_property(prop);
        }

        for dep_uid in &self.dependencies {
            let mut prop = icalendar::Property::new("RELATED-TO", dep_uid);
            prop.add_parameter("RELTYPE", "DEPENDS-ON");
            todo.append_multi_property(prop);
        }

        // --- WRITE BACK UNMAPPED PROPERTIES ---
        for raw in &self.unmapped_properties {
            let mut prop = icalendar::Property::new(&raw.key, &raw.value);
            for (k, v) in &raw.params {
                prop.add_parameter(k, v);
            }
            todo.append_multi_property(prop);
        }

        let mut calendar = Calendar::new();
        calendar.push(todo);
        let mut ics = calendar.to_string();

        // 1. Manual injection of CATEGORIES
        if !self.categories.is_empty() {
            let escaped_cats: Vec<String> = self
                .categories
                .iter()
                .map(|c| c.replace(',', "\\,"))
                .collect();
            let cat_line = format!("CATEGORIES:{}", escaped_cats.join(","));

            if let Some(idx) = ics.rfind("END:VTODO") {
                let (start, end) = ics.split_at(idx);
                let mut buffer = String::with_capacity(ics.len() + cat_line.len() + 2);
                buffer.push_str(start);
                buffer.push_str(&cat_line);
                buffer.push_str("\r\n");
                buffer.push_str(end);
                ics = buffer;
            }
        }

        // 2. Inject Raw Components (Exceptions, Timezones, etc.)
        if !self.raw_components.is_empty() {
            let trimmed = ics.trim_end();
            if let Some(idx) = trimmed.rfind("END:VCALENDAR") {
                let (start, end) = trimmed.split_at(idx);

                let extra_len: usize = self.raw_components.iter().map(|s| s.len() + 2).sum();
                let mut buffer = String::with_capacity(trimmed.len() + extra_len);

                buffer.push_str(start);
                for raw in &self.raw_components {
                    buffer.push_str(raw);
                    if !raw.ends_with("\r\n") && !raw.ends_with('\n') {
                        buffer.push_str("\r\n");
                    }
                }
                buffer.push_str(end);
                ics = buffer;
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

        let mut master_todo: Option<&Todo> = None;
        let mut raw_components: Vec<String> = Vec::with_capacity(calendar.components.len());

        for component in &calendar.components {
            match component {
                CalendarComponent::Todo(t) => {
                    let is_exception = t.properties().contains_key("RECURRENCE-ID");

                    if is_exception {
                        raw_components.push(t.to_string());
                    } else if master_todo.is_none() {
                        master_todo = Some(t);
                    } else {
                        raw_components.push(t.to_string());
                    }
                }
                CalendarComponent::Event(e) => raw_components.push(e.to_string()),
                CalendarComponent::Venue(v) => raw_components.push(v.to_string()),
                _ => {}
            }
        }

        let todo = match master_todo {
            Some(t) => t,
            None => return Err("No Master VTODO found in ICS".to_string()),
        };

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

        let parse_date_prop = |val: &str| -> Option<DateTime<Utc>> {
            if val.len() == 8 {
                NaiveDate::parse_from_str(val, "%Y%m%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
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
        };

        let due = todo.properties().get("DUE").and_then(|p| {
            let val = p.value();
            if val.len() == 8 {
                NaiveDate::parse_from_str(val, "%Y%m%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(23, 59, 59))
                    .map(|d| d.and_utc())
            } else {
                parse_date_prop(val)
            }
        });

        let dtstart = todo
            .properties()
            .get("DTSTART")
            .and_then(|p| parse_date_prop(p.value()));

        let rrule = todo
            .properties()
            .get("RRULE")
            .map(|p| p.value().to_string());

        let parse_dur = |val: &str| -> Option<u32> {
            let mut minutes = 0;
            let mut num_buf = String::new();
            let mut in_time = false;
            for c in val.chars() {
                if c == 'T' {
                    in_time = true;
                } else if c.is_numeric() {
                    num_buf.push(c);
                } else if !num_buf.is_empty() {
                    let n = num_buf.parse::<u32>().unwrap_or(0);
                    match c {
                        'D' => minutes += n * 24 * 60,
                        'H' => {
                            if in_time {
                                minutes += n * 60
                            }
                        }
                        'M' => {
                            if in_time {
                                minutes += n
                            }
                        }
                        'W' => minutes += n * 7 * 24 * 60,
                        _ => {}
                    }
                    num_buf.clear();
                }
            }
            if minutes > 0 { Some(minutes) } else { None }
        };

        let mut estimated_duration = todo
            .properties()
            .get("X-ESTIMATED-DURATION")
            .and_then(|p| parse_dur(p.value()));

        if estimated_duration.is_none() {
            estimated_duration = todo
                .properties()
                .get("DURATION")
                .and_then(|p| parse_dur(p.value()));
        }

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

        // --- OPTIMIZED RELATION EXTRACTION (MANUAL PARSE) ---
        // Use manual parsing to avoid issues where icalendar library overwrites duplicate keys
        // (e.g. RELATED-TO) when they are not explicitly handled as multi-properties.
        let (parent_uid, dependencies) = parse_related_to_manually(raw_ics);

        // --- CAPTURE UNMAPPED PROPERTIES ---
        let mut unmapped_properties = Vec::new();

        let to_raw = |prop: &icalendar::Property| -> RawProperty {
            let mut params = Vec::new();
            for (k, param) in prop.params().iter() {
                params.push((k.clone(), param.value().to_string()));
            }
            if !params.is_empty() {
                params.sort_unstable();
            }

            RawProperty {
                key: prop.key().to_string(),
                value: prop.value().to_string(),
                params,
            }
        };

        // Use case-insensitive check for handled keys
        for (key, prop) in todo.properties() {
            if !HANDLED_KEYS.contains(&key.to_uppercase().as_str()) {
                unmapped_properties.push(to_raw(prop));
            }
        }
        for (key, props) in todo.multi_properties() {
            if !HANDLED_KEYS.contains(&key.to_uppercase().as_str()) {
                for prop in props {
                    unmapped_properties.push(to_raw(prop));
                }
            }
        }

        if !unmapped_properties.is_empty() {
            unmapped_properties
                .sort_unstable_by(|a, b| a.key.cmp(&b.key).then(a.value.cmp(&b.value)));
        }

        Ok(Task {
            uid,
            summary,
            description,
            status,
            estimated_duration,
            due,
            dtstart,
            priority,
            parent_uid,
            dependencies,
            etag,
            href,
            calendar_href,
            categories,
            depth: 0,
            rrule,
            unmapped_properties,
            raw_components,
        })
    }
}

/// Helper: Manually parse RELATED-TO from raw ICS string.
/// This handles unfolding lines and ensures we catch ALL occurrences,
/// bypassing potential overwrites in the icalendar parser.
fn parse_related_to_manually(raw_ics: &str) -> (Option<String>, Vec<String>) {
    let mut parent = None;
    let mut deps = Vec::new();
    let mut current_line = String::new();

    let process_line = |line: &str, p: &mut Option<String>, d: &mut Vec<String>| {
        if line.to_uppercase().starts_with("RELATED-TO")
            && let Some((params_part, value)) = line.split_once(':') {
                let params_upper = params_part.to_uppercase();
                // Naive check usually sufficient for RELTYPE=DEPENDS-ON
                let is_dependency = params_upper.contains("RELTYPE=DEPENDS-ON");
                let val = value.trim().to_string();
                if is_dependency {
                    if !d.contains(&val) {
                        d.push(val);
                    }
                } else {
                    *p = Some(val);
                }
            }
    };

    for raw_line in raw_ics.lines() {
        // Handle folding: Lines starting with whitespace are continuations
        if raw_line.starts_with(' ') || raw_line.starts_with('\t') {
            current_line.push_str(raw_line.trim_start());
        } else {
            if !current_line.is_empty() {
                process_line(&current_line, &mut parent, &mut deps);
            }
            current_line = raw_line.to_string();
        }
    }
    if !current_line.is_empty() {
        process_line(&current_line, &mut parent, &mut deps);
    }

    (parent, deps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationships_parsing_duplicate_protection() {
        // This validates the bug fix where having a dependency caused the parent relationship
        // to be lost (or vice versa) because the library overwrote the duplicate key.
        // We now handle it manually.
        let ics = "BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
UID:child-task-uid
SUMMARY:Child Task
RELATED-TO:parent-task-uid
RELATED-TO;RELTYPE=DEPENDS-ON:blocker-task-uid
END:VTODO
END:VCALENDAR";

        let task = Task::from_ics(
            ics,
            "etag".to_string(),
            "/href".to_string(),
            "/cal/".to_string(),
        )
        .expect("Failed to parse ICS");

        assert_eq!(
            task.parent_uid,
            Some("parent-task-uid".to_string()),
            "Parent UID should be preserved"
        );
        assert_eq!(
            task.dependencies,
            vec!["blocker-task-uid".to_string()],
            "Dependency should be preserved"
        );
    }

    #[test]
    fn test_ghost_properties_exclusion_case_insensitive() {
        // Validates that properties with different casing (e.g. Related-To vs RELATED-TO)
        // are correctly identified as handled and NOT added to unmapped_properties.
        // If they were added to unmapped, they would be duplicated upon saving.
        let ics = "BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
UID:ghost-test
SUMMARY:Ghost Test
Related-To:parent-uid
Status:NEEDS-ACTION
END:VTODO
END:VCALENDAR";

        let task = Task::from_ics(
            ics,
            "etag".to_string(),
            "/href".to_string(),
            "/cal/".to_string(),
        )
        .expect("Failed to parse ICS");

        assert_eq!(task.parent_uid, Some("parent-uid".to_string()));
        assert!(
            task.unmapped_properties.is_empty(),
            "Should not have unmapped properties for handled keys (even mixed case)"
        );
    }

    #[test]
    fn test_manual_parsing_line_folding() {
        // Validates that the manual parser handles line folding (continuation lines)
        let ics = "BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
UID:folded
SUMMARY:Folded
RELATED-TO;RELTYPE=DEP
 ENDS-ON:long-dependency-uid
END:VTODO
END:VCALENDAR";

        let task = Task::from_ics(
            ics,
            "etag".to_string(),
            "/href".to_string(),
            "/cal/".to_string(),
        )
        .expect("Failed to parse ICS");

        assert_eq!(
            task.dependencies,
            vec!["long-dependency-uid".to_string()],
            "Folded lines should be unwrapped correctly"
        );
    }
}
