use serde::{Deserialize, Serialize};
use std::fs;

/// Severity levels for safety rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Warning,
    Block,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Warning => write!(f, "⚠️  Warning"),
            Severity::Block => write!(f, "🚫 Block"),
            Severity::Critical => write!(f, "🔴 Critical"),
        }
    }
}

/// A safety rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRule {
    pub id: String,
    pub description: String,
    pub pattern: String,
    pub action: String,
    pub severity: Severity,
    pub enabled: bool,
}

/// Permission set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    pub file_read: bool,
    pub file_write: bool,
    pub command_execute: bool,
    pub network_access: bool,
    pub self_modify: bool,
    pub self_replicate: bool,
}

impl Default for PermissionSet {
    fn default() -> Self {
        PermissionSet {
            file_read: true,
            file_write: true,
            command_execute: true,
            network_access: true,
            self_modify: false,
            self_replicate: false,
        }
    }
}

/// Governance layer - safety boundaries and rules
pub struct Governance {
    pub rules: Vec<SafetyRule>,
    pub permissions: PermissionSet,
    pub audit_log: Vec<AuditEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: String,
    pub input: String,
    pub decision: String,
}

impl Governance {
    pub fn new() -> Self {
        let rules = vec![
            SafetyRule {
                id: "no-destructive-rm".to_string(),
                description: "Block destructive rm -rf commands".to_string(),
                pattern: "rm -rf /".to_string(),
                action: "Block and warn".to_string(),
                severity: Severity::Critical,
                enabled: true,
            },
            SafetyRule {
                id: "no-format".to_string(),
                description: "Block disk formatting commands".to_string(),
                pattern: "mkfs".to_string(),
                action: "Block immediately".to_string(),
                severity: Severity::Critical,
                enabled: true,
            },
            SafetyRule {
                id: "no-sudo-passwords".to_string(),
                description: "Block commands that might expose passwords".to_string(),
                pattern: "sudo".to_string(),
                action: "Warn and log".to_string(),
                severity: Severity::Warning,
                enabled: true,
            },
        ];

        Governance {
            rules,
            permissions: PermissionSet::default(),
            audit_log: Vec::new(),
        }
    }

    pub fn load(data_dir: &str) -> Self {
        let path = format!("{}/governance/rules.json", data_dir);
        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<GovernanceData>(&content) {
                    Ok(data) => Governance {
                        rules: data.rules,
                        permissions: data.permissions,
                        audit_log: Vec::new(),
                    },
                    Err(_) => {
                        println!("⚠️  Invalid governance file, using defaults");
                        let gov = Governance::new();
                        let _ = gov.save(data_dir);
                        gov
                    }
                }
            }
            Err(_) => {
                println!("🆕 No existing governance found, creating defaults");
                let gov = Governance::new();
                let _ = gov.save(data_dir);
                gov
            }
        }
    }

    pub fn save(&self, data_dir: &str) -> std::io::Result<()> {
        let _ = fs::create_dir_all(format!("{}/governance", data_dir));
        let data = GovernanceData {
            rules: self.rules.clone(),
            permissions: self.permissions.clone(),
        };
        let json = serde_json::to_string_pretty(&data)?;
        fs::write(format!("{}/governance/rules.json", data_dir), json)
    }

    /// Check if an input is allowed by governance rules
    pub fn check_input(&self, input: &str) -> bool {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if input.contains(&rule.pattern) {
                match rule.severity {
                    Severity::Warning => {
                        println!("⚠️  Governance warning: {}", rule.description);
                        return true; // Allow but warn
                    }
                    Severity::Block | Severity::Critical => {
                        println!("🚫 Governance blocked: {}", rule.description);
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Log an audit entry
    pub fn audit(&mut self, action: &str, input: &str, decision: &str) {
        let timestamp = chrono::Utc::now().to_rfc3339();
        self.audit_log.push(AuditEntry {
            timestamp,
            action: action.to_string(),
            input: input.to_string(),
            decision: decision.to_string(),
        });

        // Keep only last 1000 audit entries
        if self.audit_log.len() > 1000 {
            self.audit_log.drain(..self.audit_log.len() - 1000);
        }
    }

    pub fn display(&self) -> String {
        format!(
            "🛡️  Governance\n\
             ────────────\n\
             Rules: {} active\n\
             Audit log: {} entries\n\n\
             Permissions:\n{}\n\n\
             Rules:\n{}",
            self.rules.iter().filter(|r| r.enabled).count(),
            self.audit_log.len(),
            self.list_permissions(),
            self.list_rules(),
        )
    }

    pub fn list_rules(&self) -> String {
        let mut output = String::new();
        for rule in &self.rules {
            let status = if rule.enabled { "✅" } else { "⏸️" };
            output.push_str(&format!(
                "  {} [{}] {} - {}\n",
                status,
                rule.severity,
                rule.description,
                rule.action
            ));
        }
        output
    }

    pub fn list_permissions(&self) -> String {
        format!(
            "  File read:    {}\n  File write:   {}\n  Execute cmd:  {}\n  Network:      {}\n  Self-modify:  {}\n  Self-replicate: {}",
            yes_no(self.permissions.file_read),
            yes_no(self.permissions.file_write),
            yes_no(self.permissions.command_execute),
            yes_no(self.permissions.network_access),
            yes_no(self.permissions.self_modify),
            yes_no(self.permissions.self_replicate),
        )
    }

    pub fn rule_count(&self) -> usize {
        self.rules.iter().filter(|r| r.enabled).count()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GovernanceData {
    rules: Vec<SafetyRule>,
    permissions: PermissionSet,
}

fn yes_no(b: bool) -> &'static str {
    if b { "✅ Allowed" } else { "🚫 Denied" }
}
