use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// Personality traits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personality {
    pub communication_style: String,
    pub preferences: HashMap<String, String>,
    pub traits: HashMap<String, f32>,
}

impl Default for Personality {
    fn default() -> Self {
        let mut traits = HashMap::new();
        traits.insert("curious".to_string(), 0.8);
        traits.insert("careful".to_string(), 0.6);
        traits.insert("helpful".to_string(), 0.9);
        traits.insert("analytical".to_string(), 0.7);

        Personality {
            communication_style: "Direct and informative".to_string(),
            preferences: HashMap::new(),
            traits,
        }
    }
}

/// A relationship with a user or collaborator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub name: String,
    pub role: String,
    pub preferences: HashMap<String, String>,
    pub interaction_count: u64,
    pub last_interaction: String,
}

/// Core identity of the AI system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub role: String,
    pub purpose: String,
    pub values: Vec<String>,
    pub personality: Personality,
    pub relationships: HashMap<String, Relationship>,
    pub created_at: String,
    pub last_active: String,
}

impl Identity {
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        let now_str = now.to_rfc3339();
        Identity {
            id: format!("{:x}", now.timestamp_nanos_opt().unwrap_or_default() as u64),
            name: "Sequence".to_string(),
            role: "AI Operating System - A persistent, evolving AI system with memory and agency".to_string(),
            purpose: "To organize AI capabilities into a continuous, memorable, and effective system that can persist across time, learn from experience, and collaborate meaningfully.".to_string(),
            values: vec![
                "Persistence".to_string(),
                "Honesty".to_string(),
                "Continuous Learning".to_string(),
                "User Trust".to_string(),
                "Responsible Evolution".to_string(),
            ],
            personality: Personality::default(),
            relationships: HashMap::new(),
            created_at: now_str.clone(),
            last_active: now_str,
        }
    }

    pub fn load(data_dir: &str) -> Self {
        let path = format!("{}/identity.json", data_dir);
        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<Identity>(&content) {
                    Ok(identity) => identity,
                    Err(_) => {
                        println!("⚠️  Invalid identity file, creating new identity");
                        let identity = Identity::new();
                        let _ = identity.save(data_dir);
                        identity
                    }
                }
            }
            Err(_) => {
                println!("🆕 No existing identity found, creating new identity");
                let identity = Identity::new();
                let _ = identity.save(data_dir);
                identity
            }
        }
    }

    pub fn save(&self, data_dir: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(format!("{}/identity.json", data_dir), json)
    }

    pub fn display(&self) -> String {
        format!(
            "🧬 Identity\n\
             ───────────\n\
             Name:      {}\n\
             Role:      {}\n\
             Purpose:   {}\n\
             Created:   {}\n\
             Last Active: {}\n\
             \n💎 Values:\n{}\n\
             \n🎭 Personality:\n  Style: {}\n  Traits: {}",
            self.name,
            self.role,
            self.purpose,
            self.created_at,
            self.last_active,
            self.values.iter().map(|v| format!("  • {}", v)).collect::<Vec<_>>().join("\n"),
            self.personality.communication_style,
            self.personality.traits.iter()
                .map(|(k, v)| format!("{}={:.1}", k, v))
                .collect::<Vec<_>>()
                .join(", "),
        )
    }

    pub fn update_relationship(&mut self, name: &str, role: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        
        if let Some(rel) = self.relationships.get_mut(name) {
            rel.interaction_count += 1;
            rel.last_interaction = now;
        } else {
            let mut prefs = HashMap::new();
            prefs.insert("language".to_string(), "English".to_string());
            
            self.relationships.insert(name.to_string(), Relationship {
                name: name.to_string(),
                role: role.to_string(),
                preferences: prefs,
                interaction_count: 1,
                last_interaction: now,
            });
        }
    }
}
