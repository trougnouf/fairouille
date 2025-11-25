use crate::model::Task;
use std::collections::{HashMap, HashSet};

// Special ID for the "Uncategorized" pseudo-tag
pub const UNCATEGORIZED_ID: &str = ":::uncategorized:::";

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

    pub fn get_all_categories(
        &self,
        hide_completed: bool,
        forced_includes: &HashSet<String>, // Fix for vanishing selected tags
    ) -> Vec<String> {
        let mut set = HashSet::new();
        let mut has_uncategorized = false;

        for tasks in self.calendars.values() {
            for task in tasks {
                if hide_completed && task.completed {
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

        // 1. Ensure selected tags remain visible (Fixes the bug)
        for included in forced_includes {
            // Don't add the special ID here, we handle it below
            if included != UNCATEGORIZED_ID {
                set.insert(included.clone());
            }
        }

        let mut list: Vec<String> = set.into_iter().collect();
        list.sort();

        // 2. Append "Uncategorized" at the end if needed
        // It shows if we found uncategorized tasks OR if it is currently selected
        if has_uncategorized || forced_includes.contains(UNCATEGORIZED_ID) {
            list.push(UNCATEGORIZED_ID.to_string());
        }

        list
    }

    pub fn filter(
        &self,
        active_cal_href: Option<&str>,
        selected_categories: &HashSet<String>,
        match_all_categories: bool,
        search_term: &str,
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
                if t.completed {
                    if hide_completed_global {
                        return false;
                    }
                    if is_category_mode && hide_completed_in_tags {
                        return false;
                    }
                }

                if !selected_categories.is_empty() {
                    // Check if we are filtering for "Uncategorized"
                    let filter_uncategorized = selected_categories.contains(UNCATEGORIZED_ID);

                    if match_all_categories {
                        // AND Logic
                        for sel in selected_categories {
                            if sel == UNCATEGORIZED_ID {
                                if !t.categories.is_empty() {
                                    return false;
                                }
                            } else if !t.categories.contains(sel) {
                                return false;
                            }
                        }
                    } else {
                        // OR Logic
                        let mut hit = false;
                        // Special case: if searching for Uncategorized, match tasks with no tags
                        if filter_uncategorized && t.categories.is_empty() {
                            hit = true;
                        } else {
                            for sel in selected_categories {
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

    pub fn get_task_status(&self, uid: &str) -> Option<bool> {
        for tasks in self.calendars.values() {
            if let Some(t) = tasks.iter().find(|t| t.uid == uid) {
                return Some(t.completed);
            }
        }
        None // Task not found (maybe deleted?)
    }

    pub fn is_blocked(&self, task: &Task) -> bool {
        if task.dependencies.is_empty() {
            return false;
        }
        for dep_uid in &task.dependencies {
            // If we can't find the dependency, assume it's not blocking (or external)
            // If found and NOT completed, then we are blocked.
            if let Some(completed) = self.get_task_status(dep_uid) {
                if !completed {
                    return true;
                }
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
    // Updated to pass empty aliases to Task::new
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
