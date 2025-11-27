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
    pub selected_categories: &'a HashSet<String>,
    pub match_all_categories: bool,
    pub search_term: &'a str,
    pub hide_completed_global: bool,
    pub cutoff_date: Option<DateTime<Utc>>,
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
    ) -> Vec<String> {
        let mut set = HashSet::new();
        let mut has_uncategorized = false;

        for tasks in self.calendars.values() {
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
            if let Some(tasks) = self.calendars.get(href) {
                raw_tasks.extend(tasks.clone());
            }
        } else {
            for tasks in self.calendars.values() {
                raw_tasks.extend(tasks.clone());
            }
        }

        let filtered: Vec<Task> = raw_tasks
            .into_iter()
            .filter(|t| {
                // Treat Cancelled same as Completed for hiding purposes
                if t.status.is_done() && options.hide_completed_global {
                    return false;
                }
                // Removed the old 'hide_completed_in_tags' check logic

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

                if !options.search_term.is_empty()
                    && !t
                        .summary
                        .to_lowercase()
                        .contains(&options.search_term.to_lowercase())
                {
                    return false;
                }

                true
            })
            .collect();

        Task::organize_hierarchy(filtered, options.cutoff_date)
    }

    // Changed return type to check if "Done" (Completed OR Cancelled)
    pub fn is_task_done(&self, uid: &str) -> Option<bool> {
        for tasks in self.calendars.values() {
            if let Some(t) = tasks.iter().find(|t| t.uid == uid) {
                return Some(t.status.is_done());
            }
        }
        None
    }

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
    use crate::model::Task;
    use std::collections::{HashMap, HashSet};

    // Helper to create a dummy task
    fn make_task(uid: &str, summary: &str, cal: &str, cats: Vec<&str>, completed: bool) -> Task {
        let aliases = HashMap::new();
        let mut t = Task::new(summary, &aliases);
        t.uid = uid.to_string();
        t.calendar_href = cal.to_string();
        t.categories = cats.iter().map(|s| s.to_string()).collect();
        t.completed = completed;
        t
    }

    #[test]
    fn test_store_filter_calendar_isolation() {
        let mut store = TaskStore::new();

        let t1 = make_task("1", "Work Task", "cal_work", vec![], false);
        let t2 = make_task("2", "Home Task", "cal_home", vec![], false);

        store.insert("cal_work".into(), vec![t1]);
        store.insert("cal_home".into(), vec![t2]);

        // Filter for Work
        let res = store.filter(FilterOptions {
            active_cal_href: Some("cal_work"),
            selected_categories: &HashSet::new(),
            match_all_categories: false,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
        });
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].summary, "Work Task");

        // Filter for Global (None)
        let res_global = store.filter(FilterOptions {
            active_cal_href: None,
            selected_categories: &HashSet::new(),
            match_all_categories: false,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
        });
        assert_eq!(res_global.len(), 2);
    }

    #[test]
    fn test_store_filter_categories_or() {
        let mut store = TaskStore::new();
        let t1 = make_task("1", "A", "c", vec!["urgent"], false);
        let t2 = make_task("2", "B", "c", vec!["later"], false);
        let t3 = make_task("3", "C", "c", vec!["urgent", "later"], false);
        let t4 = make_task("4", "D", "c", vec![], false); // No tags

        store.insert("c".into(), vec![t1, t2, t3, t4]);

        let mut selected = HashSet::new();
        selected.insert("urgent".to_string());
        selected.insert("later".to_string());

        // OR Logic: Should get A, B, C
        let res = store.filter(FilterOptions {
            active_cal_href: None,
            selected_categories: &selected,
            match_all_categories: false,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
        });
        assert_eq!(res.len(), 3);
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

        // AND Logic: Should only get C
        let res = store.filter(FilterOptions {
            active_cal_href: None,
            selected_categories: &selected,
            match_all_categories: true,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
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

        // 1. Show All
        let res = store.filter(FilterOptions {
            active_cal_href: None,
            selected_categories: &HashSet::new(),
            match_all_categories: false,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
        });
        assert_eq!(res.len(), 2);

        // 2. Hide Completed Globally
        let res_hidden = store.filter(FilterOptions {
            active_cal_href: None,
            selected_categories: &HashSet::new(),
            match_all_categories: false,
            search_term: "",
            hide_completed_global: true,
            cutoff_date: None,
        });
        assert_eq!(res_hidden.len(), 1);
        assert_eq!(res_hidden[0].summary, "Active");
    }

    #[test]
    fn test_visibility_completed_in_tags_view() {
        let mut store = TaskStore::new();
        let t1 = make_task("1", "Active", "c", vec![], false);
        let t2 = make_task("2", "Done", "c", vec![], true);
        store.insert("c".into(), vec![t1, t2]);

        // Calendar View: hide_completed_in_tags should NOT affect it
        let res_cal = store.filter(FilterOptions {
            active_cal_href: Some("c"),
            selected_categories: &HashSet::new(),
            match_all_categories: false,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
        });
        assert_eq!(res_cal.len(), 2);

        // Global/Tag View: with the new settings, completed tasks are only hidden
        // when hide_completed_global is true. So with hide_completed_global=false
        // we still expect both tasks to be visible.
        let res_global = store.filter(FilterOptions {
            active_cal_href: None,
            selected_categories: &HashSet::new(),
            match_all_categories: false,
            search_term: "",
            hide_completed_global: false,
            cutoff_date: None,
        });
        assert_eq!(res_global.len(), 2);
    }
}
