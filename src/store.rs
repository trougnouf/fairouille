// File: src/store.rs
use crate::cache::Cache;
use crate::model::{Task, TaskStatus};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

pub const UNCATEGORIZED_ID: &str = ":::uncategorized:::";

#[derive(Debug, Clone, Default)]
pub struct TaskStore {
    pub calendars: HashMap<String, Vec<Task>>,
    /// Reverse index: Maps Task UID -> Calendar HREF for O(1) lookups
    pub index: HashMap<String, String>,
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

    /// Bulk insert (e.g. from network load). Rebuilds index for these tasks.
    pub fn insert(&mut self, calendar_href: String, tasks: Vec<Task>) {
        for task in &tasks {
            self.index.insert(task.uid.clone(), calendar_href.clone());
        }
        self.calendars.insert(calendar_href, tasks);
    }

    /// Safe single insert that maintains the O(1) index.
    pub fn add_task(&mut self, task: Task) {
        let href = task.calendar_href.clone();
        self.index.insert(task.uid.clone(), href.clone());
        self.calendars.entry(href).or_default().push(task);
    }

    pub fn clear(&mut self) {
        self.calendars.clear();
        self.index.clear();
    }

    // --- Core Logic Helpers ---

    pub fn get_task_mut(&mut self, uid: &str) -> Option<(&mut Task, String)> {
        // 1. O(1) Lookup
        let href = self.index.get(uid)?.clone();

        // 2. Retrieve
        if let Some(tasks) = self.calendars.get_mut(&href)
            && let Some(task) = tasks.iter_mut().find(|t| t.uid == uid)
        {
            return Some((task, href));
        }

        // If we get here, the index is stale (should not happen). Clean it up.
        self.index.remove(uid);
        None
    }

    pub fn toggle_task(&mut self, uid: &str) -> Option<Task> {
        if let Some((task, _)) = self.get_task_mut(uid) {
            task.status = if task.status == TaskStatus::Completed {
                TaskStatus::NeedsAction
            } else {
                TaskStatus::Completed
            };
            return Some(task.clone());
        }
        None
    }

    pub fn set_status(&mut self, uid: &str, status: TaskStatus) -> Option<Task> {
        if let Some((task, _)) = self.get_task_mut(uid) {
            if task.status == status {
                task.status = TaskStatus::NeedsAction;
            } else {
                task.status = status;
            }
            return Some(task.clone());
        }
        None
    }

    pub fn change_priority(&mut self, uid: &str, delta: i8) -> Option<Task> {
        if let Some((task, _)) = self.get_task_mut(uid) {
            task.priority = if delta > 0 {
                match task.priority {
                    0 => 9,
                    9 => 5,
                    5 => 1,
                    1 => 1,
                    _ => 5,
                }
            } else {
                match task.priority {
                    1 => 5,
                    5 => 9,
                    9 => 0,
                    0 => 0,
                    _ => 0,
                }
            };
            return Some(task.clone());
        }
        None
    }

    pub fn delete_task(&mut self, uid: &str) -> Option<Task> {
        let href = self.index.get(uid)?.clone();

        if let Some(tasks) = self.calendars.get_mut(&href)
            && let Some(idx) = tasks.iter().position(|t| t.uid == uid)
        {
            let task = tasks.remove(idx);

            // Remove from Index
            self.index.remove(uid);

            // Sync to Cache
            let (_, token) = Cache::load(&href).unwrap_or((vec![], None));
            let _ = Cache::save(&href, tasks, token);

            return Some(task);
        }
        None
    }

    pub fn set_parent(&mut self, child_uid: &str, parent_uid: Option<String>) -> Option<Task> {
        if let Some((task, _)) = self.get_task_mut(child_uid) {
            task.parent_uid = parent_uid;
            return Some(task.clone());
        }
        None
    }

    pub fn add_dependency(&mut self, task_uid: &str, dep_uid: String) -> Option<Task> {
        if let Some((task, _)) = self.get_task_mut(task_uid)
            && !task.dependencies.contains(&dep_uid)
        {
            task.dependencies.push(dep_uid);
            return Some(task.clone());
        }
        None
    }

    pub fn remove_dependency(&mut self, task_uid: &str, dep_uid: &str) -> Option<Task> {
        if let Some((task, _)) = self.get_task_mut(task_uid)
            && let Some(pos) = task.dependencies.iter().position(|d| d == dep_uid)
        {
            task.dependencies.remove(pos);
            return Some(task.clone());
        }
        None
    }

    pub fn move_task(&mut self, uid: &str, target_href: String) -> Option<Task> {
        // delete_task handles removal from calendar, index, and cache
        let task_opt = self.delete_task(uid);

        if let Some(mut task) = task_opt {
            if task.calendar_href == target_href {
                // Edge case: Move to same calendar. Re-add.
                self.add_task(task);
                return None;
            }

            task.calendar_href = target_href.clone();

            // Add to new calendar (Updates Index automatically via add_task)
            // But we need to save cache manually since add_task doesn't save to disk by default
            // (to allow batch inserts).
            self.add_task(task.clone());

            // Update Cache for target calendar
            if let Some(target_list) = self.calendars.get(&target_href) {
                let (_, token) = Cache::load(&target_href).unwrap_or((vec![], None));
                let _ = Cache::save(&target_href, target_list, token);
            }

            return Some(task);
        }
        None
    }

    // --- Read/Filter Logic ---

    pub fn get_all_categories(
        &self,
        _hide_completed: bool,
        hide_fully_completed_tags: bool,
        forced_includes: &HashSet<String>,
        hidden_calendars: &HashSet<String>,
    ) -> Vec<(String, usize)> {
        let mut active_counts: HashMap<String, usize> = HashMap::new();
        let mut present_tags: HashSet<String> = HashSet::new();
        let mut has_uncategorized_active = false;
        let mut has_uncategorized_any = false;

        for (href, tasks) in &self.calendars {
            if hidden_calendars.contains(href) {
                continue;
            }
            for task in tasks {
                let is_active = !task.status.is_done();

                if task.categories.is_empty() {
                    has_uncategorized_any = true;
                    if is_active {
                        has_uncategorized_active = true;
                    }
                } else {
                    for cat in &task.categories {
                        present_tags.insert(cat.clone());
                        if is_active {
                            *active_counts.entry(cat.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        let mut result = Vec::new();

        for tag in present_tags {
            let count = *active_counts.get(&tag).unwrap_or(&0);
            let should_show = if hide_fully_completed_tags {
                count > 0 || forced_includes.contains(&tag)
            } else {
                true
            };

            if should_show {
                result.push((tag, count));
            }
        }

        let show_uncategorized = if hide_fully_completed_tags {
            has_uncategorized_active || forced_includes.contains(UNCATEGORIZED_ID)
        } else {
            has_uncategorized_any || forced_includes.contains(UNCATEGORIZED_ID)
        };

        if show_uncategorized {
            let count = if has_uncategorized_active {
                self.count_uncategorized_active(hidden_calendars)
            } else {
                0
            };
            result.push((UNCATEGORIZED_ID.to_string(), count));
        }

        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    fn count_uncategorized_active(&self, hidden_calendars: &HashSet<String>) -> usize {
        let mut count = 0;
        for (href, tasks) in &self.calendars {
            if hidden_calendars.contains(href) {
                continue;
            }
            for task in tasks {
                if task.categories.is_empty() && !task.status.is_done() {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn filter(&self, options: FilterOptions) -> Vec<Task> {
        let mut raw_tasks = Vec::new();

        if let Some(href) = options.active_cal_href {
            if !options.hidden_calendars.contains(href)
                && let Some(tasks) = self.calendars.get(href)
            {
                raw_tasks.extend(tasks.clone());
            }
        } else {
            for (href, tasks) in &self.calendars {
                if !options.hidden_calendars.contains(href) {
                    raw_tasks.extend(tasks.clone());
                }
            }
        }

        let filtered: Vec<Task> = raw_tasks
            .into_iter()
            .filter(|t| {
                let search_lower = options.search_term.to_lowercase();
                let has_status_filter = search_lower.contains("is:done")
                    || search_lower.contains("is:active")
                    || search_lower.contains("is:ongoing");

                if !has_status_filter && t.status.is_done() && options.hide_completed_global {
                    return false;
                }

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

                if !options.search_term.is_empty() {
                    return t.matches_search_term(options.search_term);
                }
                true
            })
            .collect();

        Task::organize_hierarchy(filtered, options.cutoff_date)
    }

    pub fn is_task_done(&self, uid: &str) -> Option<bool> {
        if let Some(href) = self.index.get(uid)
            && let Some(tasks) = self.calendars.get(href)
            && let Some(t) = tasks.iter().find(|t| t.uid == uid)
        {
            return Some(t.status.is_done());
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
            if let Some(is_done) = self.is_task_done(dep_uid)
                && !is_done
            {
                return true;
            }
        }
        false
    }

    pub fn get_summary(&self, uid: &str) -> Option<String> {
        if let Some(href) = self.index.get(uid)
            && let Some(tasks) = self.calendars.get(href)
            && let Some(t) = tasks.iter().find(|t| t.uid == uid)
        {
            return Some(t.summary.clone());
        }
        None
    }
}
