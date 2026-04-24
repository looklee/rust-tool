use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;

/// A single memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub category: String,
    pub created_at: String,
    pub last_accessed: String,
    pub access_count: u64,
    pub importance: f32,
    pub tags: Vec<String>,
}

impl Memory {
    fn new(content: &str, category: &str, tags: &[&str]) -> Self {
        let now = Utc::now();
        Memory {
            id: format!("{:x}", now.timestamp_nanos_opt().unwrap_or_default() as u64),
            content: content.to_string(),
            category: category.to_string(),
            created_at: now.to_rfc3339(),
            last_accessed: now.to_rfc3339(),
            access_count: 0,
            importance: 0.5,
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Working memory for current session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemory {
    pub context: Vec<String>,
    pub max_size: usize,
}

impl WorkingMemory {
    fn new() -> Self {
        WorkingMemory {
            context: Vec::new(),
            max_size: 50,
        }
    }

    pub fn add(&mut self, item: &str) {
        self.context.push(item.to_string());
        if self.context.len() > self.max_size {
            self.context.remove(0);
        }
    }

    pub fn to_string(&self) -> String {
        self.context.join("\n")
    }
}

/// The complete memory system
pub struct MemorySystem {
    pub episodic: Vec<Memory>,    // Past events and interactions
    pub semantic: Vec<Memory>,    // Learned knowledge
    pub procedural: Vec<Memory>,  // Skills and workflows
    pub working: WorkingMemory,   // Current session context
    data_dir: String,
}

impl MemorySystem {
    pub fn new(data_dir: &str) -> Self {
        let mut system = MemorySystem {
            episodic: Vec::new(),
            semantic: Vec::new(),
            procedural: Vec::new(),
            working: WorkingMemory::new(),
            data_dir: data_dir.to_string(),
        };

        system.load();
        system
    }

    fn load(&mut self) {
        // Load episodic memory
        let episodic_path = format!("{}/memory/episodic_index.json", self.data_dir);
        if let Ok(content) = fs::read_to_string(&episodic_path) {
            if let Ok(memories) = serde_json::from_str::<Vec<Memory>>(&content) {
                self.episodic = memories;
            }
        }

        // Load semantic memory
        let semantic_path = format!("{}/memory/semantic_index.json", self.data_dir);
        if let Ok(content) = fs::read_to_string(&semantic_path) {
            if let Ok(memories) = serde_json::from_str::<Vec<Memory>>(&content) {
                self.semantic = memories;
            }
        }

        // Load procedural memory
        let procedural_path = format!("{}/memory/procedural_index.json", self.data_dir);
        if let Ok(content) = fs::read_to_string(&procedural_path) {
            if let Ok(memories) = serde_json::from_str::<Vec<Memory>>(&content) {
                self.procedural = memories;
            }
        }
    }

    pub fn save(&self, data_dir: &str) {
        let _ = fs::create_dir_all(format!("{}/memory", data_dir));

        // Save episodic index
        let episodic_path = format!("{}/memory/episodic_index.json", data_dir);
        if let Ok(json) = serde_json::to_string_pretty(&self.episodic) {
            let _ = fs::write(&episodic_path, json);
        }

        // Save semantic index
        let semantic_path = format!("{}/memory/semantic_index.json", data_dir);
        if let Ok(json) = serde_json::to_string_pretty(&self.semantic) {
            let _ = fs::write(&semantic_path, json);
        }

        // Save procedural index
        let procedural_path = format!("{}/memory/procedural_index.json", data_dir);
        if let Ok(json) = serde_json::to_string_pretty(&self.procedural) {
            let _ = fs::write(&procedural_path, json);
        }

        // Save individual episodic memories
        for mem in &self.episodic {
            let path = format!("{}/memory/episodic/{}.json", data_dir, &mem.id[..8]);
            if let Ok(json) = serde_json::to_string_pretty(mem) {
                let _ = fs::write(&path, json);
            }
        }
    }

    /// Record an episodic memory
    pub fn record_episodic(&mut self, content: &str, category: &str, tags: &[&str]) {
        let memory = Memory::new(content, category, tags);
        self.episodic.push(memory);

        // Keep only last 1000 episodic memories to prevent unbounded growth
        if self.episodic.len() > 1000 {
            self.episodic.drain(..self.episodic.len() - 1000);
        }

        // Also add to working memory
        self.working.add(content);
    }

    /// Record semantic knowledge
    pub fn record_semantic(&mut self, content: &str, category: &str, tags: &[&str]) {
        let memory = Memory::new(content, category, tags);
        self.semantic.push(memory);
    }

    /// Record procedural knowledge
    pub fn record_procedural(&mut self, content: &str, category: &str, tags: &[&str]) {
        let memory = Memory::new(content, category, tags);
        self.procedural.push(memory);
    }

    /// Search memories by query
    pub fn search(&self, query: &str) -> Vec<&Memory> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<&Memory> = Vec::new();

        // Search episodic
        for mem in &self.episodic {
            if mem.content.to_lowercase().contains(&query_lower)
                || mem.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                || mem.category.to_lowercase().contains(&query_lower)
            {
                results.push(mem);
            }
        }

        // Search semantic
        for mem in &self.semantic {
            if mem.content.to_lowercase().contains(&query_lower)
                || mem.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            {
                results.push(mem);
            }
        }

        // Sort by recency (last_accessed)
        results.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        results
    }

    /// Get recent episodic memories
    pub fn recent_episodic(&self, n: usize) -> Vec<&Memory> {
        let start = self.episodic.len().saturating_sub(n);
        self.episodic[start..].iter().rev().collect()
    }

    /// Get context relevant to current input
    pub fn get_context(&self, input: &str) -> String {
        let relevant = self.search(input);
        if relevant.is_empty() {
            return String::new();
        }

        let mut context = String::new();
        for mem in relevant.iter().take(5) {
            context.push_str(&format!("- {}\n", mem.content));
        }
        context
    }

    pub fn episodic_count(&self) -> usize {
        self.episodic.len()
    }

    pub fn semantic_count(&self) -> usize {
        self.semantic.len()
    }

    pub fn procedural_count(&self) -> usize {
        self.procedural.len()
    }
}
