use crate::model::Task;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

// Special ID for the "Uncategorized" pseudo-tag
pub const UNCATEGORIZED_ID: &str = ":::uncategorized:::";

#[derive(Debug, Clone, Default)]
pub struct TaskStore {
    pub calendars: HashMap<String, Vec<Task>>,
}

pub struct FilterOptions<'a> {
    pub active_cal_href: Option<&'a str>,
    pub hidden_calendars: &'a std::collections::HashSet<String>,
    pub selected_categories: &'a HashSet<String>,
    pub match_all_categories: bool,
    pub search_term: &'a str,
    pub hide_completed_global: bool,
    pub cutoff_date: Option<DateTime<Utc>>,
    pub min_duration: Option<u32>,
    pub max_duration: Option<u32>,
    pub include_unset_duration: bool,
}

impl TaskStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, calendar_href: String, tasks: Vec<Task>) {
        self.calendars.insert(calendar_href, tasks);
    }

    pub fn clear(&mut self) {
        self.calendars.clear();
    }

    pub fn get_all_categories(
        &self,
        hide_completed: bool,
        hide_fully_completed_tags: bool,
        forced_includes: &HashSet<String>,
        hidden_calendars: &HashSet<String>,
    ) -> Vec<String> {
        let mut set = HashSet::new();
        let mut has_uncategorized = false;

        for (href, tasks) in &self.calendars {
            if hidden_calendars.contains(href) {
                continue;
            }
            for task in tasks {
                let is_done = task.status.is_done();

                if hide_completed && is_done {
                    continue;
                }

                if !hide_completed && hide_fully_completed_tags && is_done {
                    continue;
                }

                if task.categories.is_empty() {
                    has_uncategorized = true;
                } else {
                    for cat in &task.categories {
                        set.insert(cat.clone());
                    }
                }
            }
        }

        // Ensure selected tags remain visible
        for included in forced_includes {
            if included != UNCATEGORIZED_ID {
                set.insert(included.clone());
            }
        }

        let mut list: Vec<String> = set.into_iter().collect();
        list.sort();

        if has_uncategorized || forced_includes.contains(UNCATEGORIZED_ID) {
            list.push(UNCATEGORIZED_ID.to_string());
        }

        list
    }

    pub fn filter(&self, options: FilterOptions) -> Vec<Task> {
        let mut raw_tasks = Vec::new();

        if let Some(href) = options.active_cal_href {
            // If explicit calendar selected, ignore hidden list (unless it matches)
            if !options.hidden_calendars.contains(href) {
                if let Some(tasks) = self.calendars.get(href) {
                    raw_tasks.extend(tasks.clone());
                }
            }
        } else {
            // "All Tasks" view: Skip hidden calendars
            for (href, tasks) in &self.calendars {
                if !options.hidden_calendars.contains(href) {
                    raw_tasks.extend(tasks.clone());
                }
            }
        }

        let filtered: Vec<Task> = raw_tasks
            .into_iter()
            .filter(|t| {
                // Pre-check for any status-related filter in the search term
                let search_lower = options.search_term.to_lowercase();
                let has_status_filter = search_lower.contains("is:done")
                    || search_lower.contains("is:active")
                    || search_lower.contains("is:ongoing");

                // Apply global hide setting ONLY if there's no overriding status filter in the search
                if !has_status_filter && t.status.is_done() && options.hide_completed_global {
                    return false;
                }

                // Duration Filter
                match t.estimated_duration {
                    Some(mins) => {
                        if let Some(min) = options.min_duration
                            && mins < min
                        {
                            return false;
                        }
                        if let Some(max) = options.max_duration
                            && mins > max
                        {
                            return false;
                        }
                    }
                    None => {
                        if !options.include_unset_duration {
                            return false;
                        }
                    }
                }

                if !options.selected_categories.is_empty() {
                    let filter_uncategorized =
                        options.selected_categories.contains(UNCATEGORIZED_ID);

                    if options.match_all_categories {
                        for sel in options.selected_categories {
                            if sel == UNCATEGORIZED_ID {
                                if !t.categories.is_empty() {
                                    return false;
                                }
                            } else if !t.categories.contains(sel) {
                                return false;
                            }
                        }
                    } else {
                        let mut hit = false;
                        if filter_uncategorized && t.categories.is_empty() {
                            hit = true;
                        } else {
                            for sel in options.selected_categories {
                                if sel != UNCATEGORIZED_ID && t.categories.contains(sel) {
                                    hit = true;
                                    break;
                                }
                            }
                        }
                        if !hit {
                            return false;
                        }
                    }
                }

                // Advanced Search Parsing
                if !options.search_term.is_empty() {
                    let term = options.search_term.to_lowercase();
                    let parts: Vec<&str> = term.split_whitespace().collect();
                    let mut text_match = true;

                    for part in parts {
                        // 1. Duration Filter (~30m, ~<1h, ~>2h)
                        if part.starts_with('~') {
                            let (op, val_str) = if let Some(stripped) = part.strip_prefix("~<=") {
                                ("<=", stripped)
                            } else if let Some(stripped) = part.strip_prefix("~>=") {
                                (">=", stripped)
                            } else if let Some(stripped) = part.strip_prefix("~<") {
                                ("<", stripped)
                            } else if let Some(stripped) = part.strip_prefix("~>") {
                                (">", stripped)
                            } else if let Some(stripped) = part.strip_prefix('~') {
                                ("=", stripped)
                            } else {
                                continue;
                            };

                            // Parse value
                            let mins = if let Some(n) = val_str.strip_suffix('m') {
                                n.parse::<u32>().ok()
                            } else if let Some(n) = val_str.strip_suffix('h') {
                                n.parse::<u32>().ok().map(|h| h * 60)
                            } else if let Some(n) = val_str.strip_suffix('d') {
                                n.parse::<u32>().ok().map(|d| d * 1440)
                            } else if let Some(n) = val_str.strip_suffix('w') {
                                n.parse::<u32>().ok().map(|w| w * 10080)
                            } else if let Some(n) = val_str.strip_suffix("mo") {
                                n.parse::<u32>().ok().map(|m| m * 43200)
                            } else if let Some(n) = val_str.strip_suffix('y') {
                                n.parse::<u32>().ok().map(|y| y * 525600)
                            } else {
                                None
                            };

                            if let Some(target) = mins {
                                match t.estimated_duration {
                                    Some(d) => match op {
                                        "<" => {
                                            if d >= target {
                                                return false;
                                            }
                                        }
                                        ">" => {
                                            if d <= target {
                                                return false;
                                            }
                                        }
                                        "<=" => {
                                            if d > target {
                                                return false;
                                            }
                                        }
                                        ">=" => {
                                            if d < target {
                                                return false;
                                            }
                                        }
                                        _ => {
                                            if d != target {
                                                return false;
                                            }
                                        }
                                    },
                                    None => return false,
                                }
                                continue;
                            }
                        }

                        if part.starts_with('!') {
                            let (op, val_str) = if let Some(stripped) = part.strip_prefix("!<=") {
                                ("<=", stripped)
                            } else if let Some(stripped) = part.strip_prefix("!>=") {
                                (">=", stripped)
                            } else if let Some(stripped) = part.strip_prefix("!<") {
                                ("<", stripped)
                            } else if let Some(stripped) = part.strip_prefix("!>") {
                                (">", stripped)
                            } else if let Some(stripped) = part.strip_prefix('!') {
                                ("=", stripped)
                            } else {
                                continue;
                            };

                            if let Ok(target) = val_str.parse::<u8>() {
                                let p = t.priority;
                                // Treat 0 (None) as 10 for comparison purposes?
                                // Or strict? Strict is safer. 0 is 0.
                                match op {
                                    "<" => {
                                        if p >= target {
                                            return false;
                                        }
                                    }
                                    ">" => {
                                        if p <= target {
                                            return false;
                                        }
                                    }
                                    "<=" => {
                                        if p > target {
                                            return false;
                                        }
                                    }
                                    ">=" => {
                                        if p < target {
                                            return false;
                                        }
                                    }
                                    _ => {
                                        if p != target {
                                            return false;
                                        }
                                    }
                                }
                                continue;
                            }
                        }

                        // 3. Due Date Filter (@<2025-01-01, @>today)
                        // Supports: @<YYYY-MM-DD, @>today, @tomorrow
                        if part.starts_with('@') {
                            let (op, val_str) = if let Some(stripped) = part.strip_prefix("@<=") {
                                ("<=", stripped)
                            } else if let Some(stripped) = part.strip_prefix("@>=") {
                                (">=", stripped)
                            } else if let Some(stripped) = part.strip_prefix("@<") {
                                ("<", stripped)
                            } else if let Some(stripped) = part.strip_prefix("@>") {
                                (">", stripped)
                            } else if let Some(stripped) = part.strip_prefix('@') {
                                ("=", stripped)
                            } else {
                                continue;
                            };

                            // Parse Target Date
                            let now = Utc::now().date_naive();
                            let target_date = if val_str == "today" {
                                Some(now)
                            } else if val_str == "tomorrow" {
                                Some(now + chrono::Duration::days(1))
                            } else if let Ok(date) =
                                chrono::NaiveDate::parse_from_str(val_str, "%Y-%m-%d")
                            {
                                Some(date)
                            } else {
                                // Try Relative Offsets (1d, 2w, 1mo)
                                let offset = if let Some(n) = val_str.strip_suffix('d') {
                                    n.parse::<i64>().ok()
                                } else if let Some(n) = val_str.strip_suffix('w') {
                                    n.parse::<i64>().ok().map(|w| w * 7)
                                } else if let Some(n) = val_str.strip_suffix("mo") {
                                    n.parse::<i64>().ok().map(|m| m * 30)
                                } else if let Some(n) = val_str.strip_suffix('y') {
                                    n.parse::<i64>().ok().map(|y| y * 365)
                                } else {
                                    None
                                };

                                offset.map(|days| now + chrono::Duration::days(days))
                            };

                            if let Some(target) = target_date {
                                match t.due {
                                    Some(dt) => {
                                        let t_date = dt.naive_utc().date();
                                        match op {
                                            "<" => {
                                                if t_date >= target {
                                                    return false;
                                                }
                                            }
                                            ">" => {
                                                if t_date <= target {
                                                    return false;
                                                }
                                            }
                                            "<=" => {
                                                if t_date > target {
                                                    return false;
                                                }
                                            }
                                            ">=" => {
                                                if t_date < target {
                                                    return false;
                                                }
                                            }
                                            _ => {
                                                if t_date != target {
                                                    return false;
                                                }
                                            }
                                        }
                                    }
                                    None => return false, // Hide tasks with no date if filtering by date
                                }
                                continue;
                            }
                        }

                        // 2. Tag Filter (#work)
                        if let Some(tag_query) = part.strip_prefix('#') {
                            if !t
                                .categories
                                .iter()
                                .any(|c| c.to_lowercase().contains(tag_query))
                            {
                                return false;
                            }
                            continue;
                        }

                        // 3. Status Filter (is:done, is:active)
                        if part == "is:done" {
                            if !t.status.is_done() {
                                return false;
                            }
                            continue;
                        }
                        if part == "is:ongoing" || part == "is:process" {
                            if t.status != crate::model::TaskStatus::InProcess {
                                return false;
                            }
                            continue;
                        }
                        if part == "is:active" {
                            if t.status.is_done() {
                                return false;
                            }
                            continue;
                        }

                        // 4. Standard Text Search
                        if !t.summary.to_lowercase().contains(part)
                            && !t.description.to_lowercase().contains(part)
                        {
                            text_match = false;
                        }
                    }

                    if !text_match {
                        return false;
                    }
                }

                true
            })
            .collect();

        Task::organize_hierarchy(filtered, options.cutoff_date)
    }

    pub fn is_task_done(&self, uid: &str) -> Option<bool> {
        for tasks in self.calendars.values() {
            if let Some(t) = tasks.iter().find(|t| t.uid == uid) {
                return Some(t.status.is_done());
            }
        }
        None
    }
    // Backward compat helper # TODO replace usages
    pub fn get_task_status(&self, uid: &str) -> Option<bool> {
        self.is_task_done(uid)
    }

    pub fn is_blocked(&self, task: &Task) -> bool {
        if task.dependencies.is_empty() {
            return false;
        }
        for dep_uid in &task.dependencies {
            // Blocked if the dependency exists and is NOT done
            if let Some(is_done) = self.is_task_done(dep_uid)
                && !is_done
            {
                return true;
            }
        }
        false
    }

    pub fn get_summary(&self, uid: &str) -> Option<String> {
        for tasks in self.calendars.values() {
            if let Some(t) = tasks.iter().find(|t| t.uid == uid) {
                return Some(t.summary.clone());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Task, TaskStatus};
    use chrono::{Duration, Utc};
    use std::collections::{HashMap, HashSet};

    fn make_task(uid: &str, summary: &str, cal: &str, cats: Vec<&str>, completed: bool) -> Task {
        let aliases = HashMap::new();
        let mut t = Task::new(summary, &aliases);
        t.uid = uid.to_string();
        t.calendar_href = cal.to_string();
        t.categories = cats.iter().map(|s| s.to_string()).collect();
        t.status = if completed {
            TaskStatus::Completed
        } else {
            TaskStatus::NeedsAction
        };
        t
    }

    #[test]
    fn test_store_filter_calendar_isolation() {
        let mut store = TaskStore::new();
        let t1 = make_task("1", "Work Task", "cal_work", vec![], false);
        let t2 = make_task("2", "Home Task", "cal_home", vec![], false);
        store.insert("cal_work".into(), vec![t1]);
        store.insert("cal_home".into(), vec![t2]);

        let res = store.filter(FilterOptions {
            active_cal_href: Some("cal_work"),
            selected_categories: &HashSet::new(),
            match_all_categories: false,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
            min_duration: None,
            max_duration: None,
            include_unset_duration: true,
        });
        assert_eq!(res.len(), 1);

        let res_global = store.filter(FilterOptions {
            active_cal_href: None,
            selected_categories: &HashSet::new(),
            match_all_categories: false,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
            min_duration: None,
            max_duration: None,
            include_unset_duration: true,
        });
        assert_eq!(res_global.len(), 2);
    }

    #[test]
    fn test_store_filter_categories_or() {
        let mut store = TaskStore::new();
        let t1 = make_task("1", "A", "c", vec!["urgent"], false);
        let t2 = make_task("2", "B", "c", vec!["later"], false);
        let t3 = make_task("3", "C", "c", vec!["urgent", "later"], false);
        store.insert("c".into(), vec![t1, t2, t3]);
        let mut selected = HashSet::new();
        selected.insert("urgent".to_string());

        let res = store.filter(FilterOptions {
            active_cal_href: None,
            selected_categories: &selected,
            match_all_categories: false,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
            min_duration: None,
            max_duration: None,
            include_unset_duration: true,
        });
        assert_eq!(res.len(), 2);
    }

    #[test]
    fn test_store_filter_categories_and() {
        let mut store = TaskStore::new();
        let t1 = make_task("1", "A", "c", vec!["urgent"], false);
        let t2 = make_task("2", "B", "c", vec!["later"], false);
        let t3 = make_task("3", "C", "c", vec!["urgent", "later"], false);
        store.insert("c".into(), vec![t1, t2, t3]);
        let mut selected = HashSet::new();
        selected.insert("urgent".to_string());
        selected.insert("later".to_string());

        let res = store.filter(FilterOptions {
            active_cal_href: None,
            selected_categories: &selected,
            match_all_categories: true,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
            min_duration: None,
            max_duration: None,
            include_unset_duration: true,
        });
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].summary, "C");
    }

    #[test]
    fn test_visibility_completed() {
        let mut store = TaskStore::new();
        let t1 = make_task("1", "Active", "c", vec![], false);
        let t2 = make_task("2", "Done", "c", vec![], true);
        store.insert("c".into(), vec![t1, t2]);

        let res_hidden = store.filter(FilterOptions {
            active_cal_href: None,
            selected_categories: &HashSet::new(),
            match_all_categories: false,
            search_term: "",
            hide_completed_global: true,
            cutoff_date: None,
            min_duration: None,
            max_duration: None,
            include_unset_duration: true,
        });
        assert_eq!(res_hidden.len(), 1);
    }

    #[test]
    fn test_advanced_search_filters() {
        let mut store = TaskStore::new();
        let mut t1 = make_task("1", "Quick urgent task", "c", vec!["work", "urgent"], false);
        t1.priority = 2;
        t1.estimated_duration = Some(15);
        t1.due = Some(Utc::now() + Duration::days(3));

        let mut t2 = make_task("2", "Long research task", "c", vec!["work"], false);
        t2.priority = 5;
        t2.estimated_duration = Some(120);
        t2.due = Some(Utc::now() + Duration::days(10));

        let mut t3 = make_task("3", "Review meeting notes", "c", vec!["meeting"], false);
        t3.priority = 0;
        t3.estimated_duration = Some(45);

        let mut t4 = make_task("4", "Finished old stuff", "c", vec!["work"], true);
        t4.estimated_duration = Some(60);

        store.insert(
            "c".into(),
            vec![t1.clone(), t2.clone(), t3.clone(), t4.clone()],
        );

        // --- Test with hide_completed = false ---
        let base_options = FilterOptions {
            search_term: "", // Add this
            active_cal_href: None,
            selected_categories: &HashSet::new(),
            match_all_categories: false,
            hide_completed_global: false,
            cutoff_date: None,
            min_duration: None,
            max_duration: None,
            include_unset_duration: true,
        };

        // Test tag filter: should find ALL #work tasks (active and done)
        let res_tag = store.filter(FilterOptions {
            search_term: "#work",
            ..base_options
        });
        assert_eq!(
            res_tag.len(),
            3,
            "Should find all #work tasks t1, t2, and t4 when not hiding completed"
        );

        // --- Test with hide_completed = true ---
        let hiding_options = FilterOptions {
            hide_completed_global: true,
            ..base_options
        };

        // Test tag filter: should find ONLY ACTIVE #work tasks
        let res_tag_hiding = store.filter(FilterOptions {
            search_term: "#work",
            ..hiding_options
        });
        assert_eq!(
            res_tag_hiding.len(),
            2,
            "Should find only active #work tasks t1 and t2 when hiding completed"
        );

        // Test 'is:done' override: should find ONLY DONE #work tasks, even when hiding completed
        let res_tag_override = store.filter(FilterOptions {
            search_term: "#work is:done",
            ..hiding_options
        });
        assert_eq!(
            res_tag_override.len(),
            1,
            "is:done should override hide_completed"
        );
        assert_eq!(res_tag_override[0].uid, "4");

        // Test combined query (implicitly active)
        let res_combo = store.filter(FilterOptions {
            search_term: "task #work ~>=2h",
            ..hiding_options
        });
        assert_eq!(res_combo.len(), 1);
        assert_eq!(res_combo[0].uid, "2");
    }
}
