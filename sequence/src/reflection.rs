use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// A self-reflection entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub id: String,
    pub timestamp: String,
    pub category: String,
    pub observation: String,
    pub insight: String,
    pub action_items: Vec<String>,
}

/// Self-Reflection System - periodic self-analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfReflection {
    pub reflections: Vec<Reflection>,
    pub patterns: HashMap<String, Vec<String>>,
    pub goals: Vec<String>,
    pub dir: String,
}

impl SelfReflection {
    pub fn new(dir: &str) -> Self {
        let path = format!("{}/reflections.json", dir);
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(reflection) = serde_json::from_str::<SelfReflection>(&content) {
                return reflection;
            }
        }

        SelfReflection {
            reflections: Vec::new(),
            patterns: HashMap::new(),
            goals: vec![
                "Improve tool coverage".to_string(),
                "Reduce code duplication".to_string(),
                "Increase test coverage".to_string(),
                "Optimize performance".to_string(),
            ],
            dir: dir.to_string(),
        }
    }

    /// Record a reflection
    pub fn record(&mut self, category: &str, observation: &str, insight: &str, actions: Vec<String>) {
        use chrono::Utc;
        let now = Utc::now();
        
        let reflection = Reflection {
            id: format!("{:x}", now.timestamp_nanos_opt().unwrap_or_default() as u64),
            timestamp: now.to_rfc3339(),
            category: category.to_string(),
            observation: observation.to_string(),
            insight: insight.to_string(),
            action_items: actions,
        };

        self.reflections.push(reflection);

        // Track patterns
        self.patterns
            .entry(category.to_string())
            .or_insert_with(Vec::new)
            .push(observation.to_string());

        // Keep only last 100 reflections
        if self.reflections.len() > 100 {
            self.reflections.drain(..self.reflections.len() - 100);
        }
    }

    /// Analyze patterns in reflections
    pub fn analyze_patterns(&self) -> String {
        let mut output = "🔍 Reflection Analysis:\n\n".to_string();

        output.push_str(&format!("  Total reflections: {}\n\n", self.reflections.len()));

        // Category distribution
        output.push_str("📊 Category Distribution:\n");
        let mut cat_counts: HashMap<&str, usize> = HashMap::new();
        for r in &self.reflections {
            *cat_counts.entry(&r.category).or_insert(0) += 1;
        }
        for (cat, count) in &cat_counts {
            output.push_str(&format!("  - {}: {}\n", cat, count));
        }

        // Patterns
        output.push_str("\n🔗 Identified Patterns:\n");
        for (category, observations) in &self.patterns {
            if observations.len() > 2 {
                output.push_str(&format!(
                    "  [{}] {} occurrences\n",
                    category,
                    observations.len()
                ));
                for obs in observations.iter().take(3) {
                    output.push_str(&format!("    - {}\n", obs));
                }
            }
        }

        // Goals progress
        output.push_str("\n🎯 Goals:\n");
        for goal in &self.goals {
            output.push_str(&format!("  - {}\n", goal));
        }

        // Recent reflections
        output.push_str("\n📝 Recent Reflections:\n");
        let recent = self.reflections.iter().rev().take(5);
        for r in recent {
            output.push_str(&format!(
                "  [{}] {} - {}\n",
                r.category,
                r.observation.chars().take(60).collect::<String>(),
                r.timestamp.chars().take(19).collect::<String>()
            ));
        }

        output
    }

    /// Get reflections by category
    pub fn by_category(&self, category: &str) -> Vec<&Reflection> {
        self.reflections
            .iter()
            .filter(|r| r.category == category)
            .collect()
    }

    /// Add a goal
    pub fn add_goal(&mut self, goal: &str) {
        if !self.goals.contains(&goal.to_string()) {
            self.goals.push(goal.to_string());
        }
    }

    /// Remove a goal
    pub fn remove_goal(&mut self, goal: &str) {
        self.goals.retain(|g| g != goal);
    }

    /// Save reflections to disk
    pub fn save(&self) {
        let path = format!("{}/reflections.json", self.dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_reflection() {
        let mut reflection = SelfReflection::new("/tmp");
        reflection.record(
            "performance",
            "Tool X is slow",
            "Need to optimize",
            vec!["Profile the code".to_string()],
        );
        assert_eq!(reflection.reflections.len(), 1);
    }

    #[test]
    fn test_analyze_patterns() {
        let mut reflection = SelfReflection::new("/tmp");
        reflection.record("test", "obs1", "insight1", vec![]);
        reflection.record("test", "obs2", "insight2", vec![]);
        let analysis = reflection.analyze_patterns();
        assert!(analysis.contains("test"));
        assert!(analysis.contains("Total reflections"));
    }

    #[test]
    fn test_by_category() {
        let mut reflection = SelfReflection::new("/tmp");
        reflection.record("cat1", "obs1", "insight1", vec![]);
        reflection.record("cat2", "obs2", "insight2", vec![]);
        reflection.record("cat1", "obs3", "insight3", vec![]);
        let cat1 = reflection.by_category("cat1");
        assert_eq!(cat1.len(), 2);
    }

    #[test]
    fn test_goals() {
        let mut reflection = SelfReflection::new("/tmp");
        let initial = reflection.goals.len();
        reflection.add_goal("new goal");
        assert_eq!(reflection.goals.len(), initial + 1);
        reflection.remove_goal("new goal");
        assert_eq!(reflection.goals.len(), initial);
    }
}
