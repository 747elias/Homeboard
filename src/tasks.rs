use anyhow::{bail, Context, Result};
use chrono::{Datelike, Days, Months, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TaskData {
    pub version: u32,
    pub revision: u64,
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub views: Vec<SavedView>,
}
impl Default for TaskData {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            tasks: vec![],
            projects: vec![
                Project {
                    id: "personal".into(),
                    name: "Persönlich".into(),
                    color: "#8bdfc5".into(),
                },
                Project {
                    id: "work".into(),
                    name: "Arbeit".into(),
                    color: "#a7b5ff".into(),
                },
            ],
            views: vec![],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: u8,
    pub due: Option<String>,
    pub start: Option<String>,
    pub project: String,
    pub tags: Vec<String>,
    pub subtasks: Vec<Subtask>,
    pub notes: Vec<Note>,
    pub links: Vec<String>,
    pub starred: bool,
    pub archived: bool,
    pub recurrence: String,
    pub estimate_minutes: u32,
    pub spent_minutes: u32,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub order: i64,
}
impl Default for Task {
    fn default() -> Self {
        let stamp = Utc::now().to_rfc3339();
        Self {
            id: String::new(),
            title: String::new(),
            description: String::new(),
            status: "todo".into(),
            priority: 0,
            due: None,
            start: None,
            project: String::new(),
            tags: vec![],
            subtasks: vec![],
            notes: vec![],
            links: vec![],
            starred: false,
            archived: false,
            recurrence: "none".into(),
            estimate_minutes: 0,
            spent_minutes: 0,
            created_at: stamp.clone(),
            updated_at: stamp,
            completed_at: None,
            order: 0,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subtask {
    pub id: String,
    pub title: String,
    pub done: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub created_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedView {
    pub id: String,
    pub name: String,
    pub layout: String,
    pub filter: Filter,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Filter {
    pub search: String,
    pub status: String,
    pub priority: String,
    pub project: String,
    pub tag: String,
    pub when: String,
    pub favorites: bool,
    pub show_completed: bool,
    pub group: String,
    pub sort: String,
    pub direction: String,
}
impl Default for Filter {
    fn default() -> Self {
        Self {
            search: String::new(),
            status: String::new(),
            priority: String::new(),
            project: String::new(),
            tag: String::new(),
            when: String::new(),
            favorites: false,
            show_completed: false,
            group: "none".into(),
            sort: "order".into(),
            direction: "asc".into(),
        }
    }
}
const LAYOUTS: &[&str] = &[
    "table", "list", "kanban", "calendar", "week", "agenda", "timeline", "matrix", "projects",
    "stats", "archive",
];
fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn date(value: &Option<String>) -> Result<Option<NaiveDate>> {
    value
        .as_ref()
        .map(|s| {
            if s.len() != 10 {
                bail!("Ungültiges Datum");
            }
            let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")?;
            if !(1900..=2200).contains(&d.year()) {
                bail!("Datum ausserhalb 1900–2200");
            }
            Ok(d)
        })
        .transpose()
}
fn timestamp(value: &str) -> bool {
    value.len() <= 40 && chrono::DateTime::parse_from_rfc3339(value).is_ok()
}
impl TaskData {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1
            || self.tasks.len() > 2000
            || self.projects.len() > 100
            || self.views.len() > 100
        {
            bail!("Ungültige Aufgabendatei oder Grössenlimit erreicht");
        }
        let mut projects = HashSet::new();
        for p in &self.projects {
            if !valid_id(&p.id)
                || !projects.insert(&p.id)
                || p.name.trim().is_empty()
                || p.name.len() > 100
                || p.color.len() != 7
                || !p.color.starts_with('#')
                || !p.color[1..].bytes().all(|b| b.is_ascii_hexdigit())
            {
                bail!("Ungültiges Projekt");
            }
        }
        let mut ids = HashSet::new();
        for t in &self.tasks {
            if !valid_id(&t.id)
                || !ids.insert(&t.id)
                || t.title.trim().is_empty()
                || t.title.len() > 500
                || t.description.len() > 20_000
                || !matches!(t.status.as_str(), "todo" | "doing" | "waiting" | "done")
                || t.priority > 3
                || !matches!(
                    t.recurrence.as_str(),
                    "none" | "daily" | "weekdays" | "weekly" | "monthly"
                )
                || (!t.project.is_empty() && !projects.contains(&t.project))
                || t.tags.len() > 20
                || t.tags.iter().any(|v| v.trim().is_empty() || v.len() > 60)
                || t.subtasks.len() > 100
                || t.notes.len() > 50
                || t.links.len() > 20
                || t.estimate_minutes > 1_000_000
                || t.spent_minutes > 1_000_000
                || !timestamp(&t.created_at)
                || !timestamp(&t.updated_at)
                || t.completed_at.as_deref().is_some_and(|v| !timestamp(v))
            {
                bail!("Ungültige Aufgabe");
            }
            let (start, due) = (date(&t.start)?, date(&t.due)?);
            if matches!((start,due), (Some(s),Some(d)) if s > d) {
                bail!("Startdatum liegt nach Fälligkeit");
            }
            let mut child_ids = HashSet::new();
            for s in &t.subtasks {
                if !valid_id(&s.id)
                    || !child_ids.insert(&s.id)
                    || s.title.trim().is_empty()
                    || s.title.len() > 500
                {
                    bail!("Ungültige Unteraufgabe");
                }
            }
            child_ids.clear();
            for n in &t.notes {
                if !valid_id(&n.id)
                    || !child_ids.insert(&n.id)
                    || n.text.trim().is_empty()
                    || n.text.len() > 5000
                    || !timestamp(&n.created_at)
                {
                    bail!("Ungültige Notiz");
                }
            }
            for link in &t.links {
                if link.is_empty() {
                    bail!("Leerer Link");
                }
                crate::config::validate_url(link)?;
            }
        }
        ids.clear();
        for v in &self.views {
            let f = &v.filter;
            if !valid_id(&v.id)
                || !ids.insert(&v.id)
                || v.name.trim().is_empty()
                || v.name.len() > 100
                || !LAYOUTS.contains(&v.layout.as_str())
                || f.search.len() > 500
                || f.tag.len() > 60
                || f.project.len() > 80
                || !matches!(
                    f.status.as_str(),
                    "" | "todo" | "doing" | "waiting" | "done"
                )
                || !matches!(f.priority.as_str(), "" | "0" | "1" | "2" | "3")
                || !matches!(f.when.as_str(), "" | "today" | "overdue" | "week" | "none")
                || !matches!(
                    f.group.as_str(),
                    "none" | "status" | "project" | "priority" | "due"
                )
                || !matches!(
                    f.sort.as_str(),
                    "order"
                        | "title"
                        | "due"
                        | "priority"
                        | "created_at"
                        | "updated_at"
                        | "project"
                )
                || !matches!(f.direction.as_str(), "asc" | "desc")
            {
                bail!("Ungültige gespeicherte Ansicht");
            }
        }
        if serde_json::to_vec(self)?.len() > 1_000_000 {
            bail!("Aufgabendaten dürfen maximal 1 MB gross sein");
        }
        Ok(())
    }
    pub fn apply_transitions(&mut self, previous: &Self, today: NaiveDate) -> Result<()> {
        let stamp = Utc::now().to_rfc3339();
        let mut recurring = vec![];
        for task in &mut self.tasks {
            let old = previous.tasks.iter().find(|t| t.id == task.id);
            let just_completed = task.status == "done" && old.is_some_and(|t| t.status != "done");
            if just_completed {
                task.completed_at = Some(stamp.clone());
                if task.recurrence != "none" && !task.archived {
                    let base = date(&task.due)?.unwrap_or(today).max(today);
                    let next_date = match task.recurrence.as_str() {
                        "weekly" => base.checked_add_days(Days::new(7)),
                        "monthly" => base.checked_add_months(Months::new(1)),
                        "weekdays" => {
                            let mut d = base.checked_add_days(Days::new(1));
                            while d.is_some_and(|v| v.weekday().number_from_monday() > 5) {
                                d = d.and_then(|v| v.checked_add_days(Days::new(1)));
                            }
                            d
                        }
                        _ => base.checked_add_days(Days::new(1)),
                    }
                    .context("Folgetermin ausserhalb des Datumsbereichs")?;
                    let mut next = task.clone();
                    next.id = format!("r-{}", &crate::auth::token()[..24]);
                    next.status = "todo".into();
                    next.completed_at = None;
                    next.created_at = stamp.clone();
                    next.updated_at = stamp.clone();
                    next.due = Some(next_date.to_string());
                    next.start = date(&task.start)?
                        .and_then(|s| {
                            s.checked_add_days(Days::new((next_date - base).num_days() as u64))
                        })
                        .map(|s| s.to_string());
                    next.spent_minutes = 0;
                    next.notes.clear();
                    for s in &mut next.subtasks {
                        s.done = false;
                    }
                    recurring.push(next);
                }
            } else if task.status != "done" {
                task.completed_at = None;
            }
            if let Some(old) = old {
                task.created_at = old.created_at.clone();
                if serde_json::to_vec(task)? != serde_json::to_vec(old)? {
                    task.updated_at = stamp.clone();
                }
            }
        }
        self.tasks.extend(recurring);
        self.validate()
    }
}
pub fn load(dir: &Path) -> Result<TaskData> {
    let path = dir.join("tasks.json");
    if !path.exists() {
        crate::config::atomic_write(&path, &serde_json::to_vec_pretty(&TaskData::default())?)?;
    }
    let data: TaskData = serde_json::from_slice(&fs::read(path)?)
        .context("tasks.json ist beschädigt; Sicherung wiederherstellen")?;
    data.validate()?;
    Ok(data)
}
pub fn save(dir: &Path, data: &TaskData) -> Result<()> {
    data.validate()?;
    let backup = dir.join("backups/tasks");
    fs::create_dir_all(&backup)?;
    let path = dir.join("tasks.json");
    if path.exists() {
        fs::copy(
            &path,
            backup.join(format!(
                "tasks-{}.json",
                Utc::now().format("%Y%m%dT%H%M%S%.9f")
            )),
        )?;
    }
    crate::config::atomic_write(&path, &serde_json::to_vec_pretty(data)?)?;
    let mut files: Vec<_> = fs::read_dir(backup)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|s| s == "json"))
        .collect();
    files.sort();
    let count = files.len().saturating_sub(10);
    for f in files.into_iter().take(count) {
        if fs::remove_file(f).is_err() {
            tracing::warn!("tasks_backup_cleanup_failed");
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> TaskData {
        let mut d = TaskData::default();
        d.tasks.push(Task {
            id: "t-1".into(),
            title: "Planen".into(),
            project: "personal".into(),
            ..Task::default()
        });
        d
    }
    #[test]
    fn persistence_backups_and_invalid_dates() {
        let dir = tempfile::tempdir().unwrap();
        let mut d = sample();
        for _ in 0..12 {
            save(dir.path(), &d).unwrap();
        }
        assert_eq!(load(dir.path()).unwrap().tasks[0].title, "Planen");
        assert_eq!(
            fs::read_dir(dir.path().join("backups/tasks"))
                .unwrap()
                .count(),
            10
        );
        d.tasks[0].due = Some("2026-02-30".into());
        assert!(d.validate().is_err());
        d.tasks[0].due = None;
        d.tasks[0].project = "missing".into();
        assert!(d.validate().is_err());
    }
    #[test]
    fn recurring_weekdays_and_month_end_and_idempotence() {
        let mut old = sample();
        old.tasks[0].due = Some("2026-10-23".into());
        old.tasks[0].recurrence = "weekdays".into();
        let mut next = old.clone();
        next.tasks[0].status = "done".into();
        next.apply_transitions(&old, NaiveDate::from_ymd_opt(2026, 10, 23).unwrap())
            .unwrap();
        assert_eq!(next.tasks.len(), 2);
        assert_eq!(next.tasks[1].due.as_deref(), Some("2026-10-26"));
        let previous = next.clone();
        next.apply_transitions(&previous, NaiveDate::from_ymd_opt(2026, 10, 23).unwrap())
            .unwrap();
        assert_eq!(next.tasks.len(), 2);
        let mut old = sample();
        old.tasks[0].due = Some("2027-01-31".into());
        old.tasks[0].recurrence = "monthly".into();
        let mut next = old.clone();
        next.tasks[0].status = "done".into();
        next.apply_transitions(&old, NaiveDate::from_ymd_opt(2027, 1, 31).unwrap())
            .unwrap();
        assert_eq!(next.tasks[1].due.as_deref(), Some("2027-02-28"));
    }
    #[test]
    fn duplicate_ids_and_javascript_links_rejected() {
        let mut d = sample();
        d.tasks.push(d.tasks[0].clone());
        assert!(d.validate().is_err());
        d.tasks.pop();
        d.tasks[0].links.push("javascript:alert(1)".into());
        assert!(d.validate().is_err());
    }
}
