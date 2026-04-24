use serde::{Deserialize, Serialize};
use std::fs;

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::Running => write!(f, "Running"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Failed(e) => write!(f, "Failed: {}", e),
            TaskStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Task priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}

/// A task in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub priority: Priority,
    pub status: TaskStatus,
    pub created_at: String,
    pub scheduled_for: Option<String>,
    pub completed_at: Option<String>,
    pub tags: Vec<String>,
}

impl Task {
    fn new(description: &str) -> Self {
        let now = chrono::Utc::now();
        let now_str = now.to_rfc3339();
        Task {
            id: format!("{:x}", now.timestamp_nanos_opt().unwrap_or_default() as u64),
            description: description.to_string(),
            priority: Priority::Medium,
            status: TaskStatus::Pending,
            created_at: now_str.clone(),
            scheduled_for: None,
            completed_at: None,
            tags: Vec::new(),
        }
    }
}

/// Runtime engine for task management
pub struct RuntimeEngine {
    tasks: Vec<Task>,
    data_dir: String,
}

impl RuntimeEngine {
    pub fn new(data_dir: &str) -> Self {
        let mut engine = RuntimeEngine {
            tasks: Vec::new(),
            data_dir: data_dir.to_string(),
        };

        engine.load();
        engine
    }

    fn load(&mut self) {
        let path = format!("{}/tasks/queue.json", self.data_dir);
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(tasks) = serde_json::from_str::<Vec<Task>>(&content) {
                self.tasks = tasks;
            }
        }
    }

    pub fn save(&self, data_dir: &str) -> std::io::Result<()> {
        let _ = fs::create_dir_all(format!("{}/tasks", data_dir));
        let json = serde_json::to_string_pretty(&self.tasks)?;
        fs::write(format!("{}/tasks/queue.json", data_dir), json)
    }

    /// Add a new task
    pub fn add_task(&mut self, description: &str) -> String {
        let task = Task::new(description);
        let id = task.id.clone();
        self.tasks.push(task);
        let _ = self.save(&self.data_dir);
        id
    }

    /// Complete a task by ID
    pub fn complete_task(&mut self, id: &str) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id.starts_with(id) || &t.id == id) {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(chrono::Utc::now().to_rfc3339());
            let _ = self.save(&self.data_dir);
        }
    }

    /// Mark a task as failed
    pub fn fail_task(&mut self, id: &str, reason: &str) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id.starts_with(id) || &t.id == id) {
            task.status = TaskStatus::Failed(reason.to_string());
            let _ = self.save(&self.data_dir);
        }
    }

    /// Remove a task
    pub fn remove_task(&mut self, id: &str) {
        self.tasks.retain(|t| !(t.id.starts_with(id) || &t.id == id));
        let _ = self.save(&self.data_dir);
    }

    /// Cancel a task
    pub fn cancel_task(&mut self, id: &str) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id.starts_with(id) || &t.id == id) {
            task.status = TaskStatus::Cancelled;
            let _ = self.save(&self.data_dir);
        }
    }

    /// List all tasks
    pub fn list_tasks(&self) -> &[Task] {
        &self.tasks
    }

    /// Get pending tasks
    pub fn pending_tasks(&self) -> Vec<&Task> {
        self.tasks.iter()
            .filter(|t| matches!(t.status, TaskStatus::Pending))
            .collect()
    }

    /// Get running tasks
    pub fn running_tasks(&self) -> Vec<&Task> {
        self.tasks.iter()
            .filter(|t| matches!(t.status, TaskStatus::Running))
            .collect()
    }

    /// Get completed tasks
    pub fn completed_tasks(&self) -> Vec<&Task> {
        self.tasks.iter()
            .filter(|t| matches!(t.status, TaskStatus::Completed))
            .collect()
    }

    /// Get failed tasks
    pub fn failed_tasks(&self) -> Vec<&Task> {
        self.tasks.iter()
            .filter(|t| matches!(t.status, TaskStatus::Failed(_)))
            .collect()
    }

    pub fn pending_count(&self) -> usize {
        self.pending_tasks().len()
    }

    pub fn running_count(&self) -> usize {
        self.running_tasks().len()
    }

    pub fn completed_count(&self) -> usize {
        self.completed_tasks().len()
    }

    pub fn failed_count(&self) -> usize {
        self.failed_tasks().len()
    }

    /// Start a task (mark as running)
    pub fn start_task(&mut self, id: &str) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id.starts_with(id) || &t.id == id) {
            task.status = TaskStatus::Running;
            let _ = self.save(&self.data_dir);
        }
    }
}
