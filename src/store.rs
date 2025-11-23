use crate::model::Task;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct TaskStore {
    pub calendars: HashMap<String, Vec<Task>>,
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

    pub fn get_all_categories(&self) -> Vec<String> {
        let mut set = HashSet::new();
        for tasks in self.calendars.values() {
            for task in tasks {
                for cat in &task.categories {
                    set.insert(cat.clone());
                }
            }
        }
        let mut list: Vec<String> = set.into_iter().collect();
        list.sort();
        list
    }

    pub fn filter(
        &self,
        active_cal_href: Option<&str>,
        selected_categories: &HashSet<String>,
        match_all_categories: bool,
        search_term: &str,
        // NEW ARGS
        hide_completed_global: bool,
        hide_completed_in_tags: bool,
    ) -> Vec<Task> {
        let mut raw_tasks = Vec::new();
        let is_category_mode = active_cal_href.is_none();

        if let Some(href) = active_cal_href {
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
                // VISIBILITY FILTER
                if t.completed {
                    if hide_completed_global {
                        return false;
                    }
                    if is_category_mode && hide_completed_in_tags {
                        return false;
                    }
                }

                if !selected_categories.is_empty() {
                    if match_all_categories {
                        for sel in selected_categories {
                            if !t.categories.contains(sel) {
                                return false;
                            }
                        }
                    } else {
                        let mut hit = false;
                        for sel in selected_categories {
                            if t.categories.contains(sel) {
                                hit = true;
                                break;
                            }
                        }
                        if !hit {
                            return false;
                        }
                    }
                }

                if !search_term.is_empty() {
                    if !t
                        .summary
                        .to_lowercase()
                        .contains(&search_term.to_lowercase())
                    {
                        return false;
                    }
                }

                true
            })
            .collect();

        Task::organize_hierarchy(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Task;

    // Helper to create a dummy task
    fn make_task(uid: &str, summary: &str, cal: &str, cats: Vec<&str>, completed: bool) -> Task {
        let mut t = Task::new(summary);
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
        let res = store.filter(Some("cal_work"), &HashSet::new(), false, "", false, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].summary, "Work Task");

        // Filter for Global (None)
        let res_global = store.filter(None, &HashSet::new(), false, "", false, false);
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
        let res = store.filter(None, &selected, false, "", false, false);
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
        let res = store.filter(None, &selected, true, "", false, false);
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
        let res = store.filter(None, &HashSet::new(), false, "", false, false);
        assert_eq!(res.len(), 2);

        // 2. Hide Completed Globally
        let res_hidden = store.filter(None, &HashSet::new(), false, "", true, false);
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
        let res_cal = store.filter(Some("c"), &HashSet::new(), false, "", false, true);
        assert_eq!(res_cal.len(), 2);

        // Global/Tag View: hide_completed_in_tags SHOULD affect it
        let res_global = store.filter(None, &HashSet::new(), false, "", false, true);
        assert_eq!(res_global.len(), 1);
        assert_eq!(res_global[0].summary, "Active");
    }
}
