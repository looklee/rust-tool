mod memory;
mod identity;
mod runtime;
mod world;
mod governance;
mod models;
mod knowledge;
mod plugins;
mod reflection;
mod webapi;
mod sandbox;

use chrono::Local;
use memory::MemorySystem;
use identity::Identity;
use runtime::RuntimeEngine;
use world::WorldInterface;
use governance::Governance;
use knowledge::KnowledgeGraph;
use plugins::PluginManager;
use reflection::SelfReflection;
use sandbox::CodeSandbox;
use std::env;
use std::fs;
use std::io::{self, Write};

/// SEQUENCE OS¹ - AI Operating System
/// 
/// A system layer built on top of foundation models that provides:
/// - Persistent memory across sessions
/// - Continuous identity
/// - Task execution loop
/// - World interface
/// - Governance and safety

const VERSION: &str = env!("CARGO_PKG_VERSION");
const SEQUENCE_DIR: &str = ".sequence";

/// System state
struct SequenceOS {
    memory: MemorySystem,
    identity: Identity,
    runtime: RuntimeEngine,
    world: WorldInterface,
    governance: Governance,
    knowledge: KnowledgeGraph,
    plugins: PluginManager,
    reflection: SelfReflection,
    sandbox: CodeSandbox,
    session_start: String,
    message_count: u64,
}

impl SequenceOS {
    /// Initialize SEQUENCE OS¹
    fn new() -> Self {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let sequence_dir = format!("{}/{}", home, SEQUENCE_DIR);

        // Create directory structure if needed
        Self::ensure_dirs(&sequence_dir);

        println!("╔═══════════════════════════════════════════════════════╗");
        println!("║              SEQUENCE OS¹ v{}                      ║", VERSION);
        println!("║       AI Operating System - Persistent, Evolving   ║");
        println!("╚═══════════════════════════════════════════════════════╝");
        println!();

        // Load or create identity
        let identity = Identity::load(&sequence_dir);
        
        // Load or create memory system
        let memory = MemorySystem::new(&sequence_dir);
        
        // Load or create runtime
        let runtime = RuntimeEngine::new(&sequence_dir);
        
        // Initialize world interface
        let world = WorldInterface::new(&sequence_dir);
        
        // Load or create governance
        let governance = Governance::load(&sequence_dir);

        // Initialize knowledge graph
        let knowledge = KnowledgeGraph::new(&sequence_dir);

        // Initialize plugin manager
        let mut plugins = PluginManager::new(&sequence_dir);
        // Register built-in plugins
        Self::register_builtin_plugins(&mut plugins);

        // Initialize self-reflection
        let reflection = SelfReflection::new(&sequence_dir);

        // Initialize code sandbox
        let sandbox = CodeSandbox::new(&sequence_dir);

        SequenceOS {
            memory,
            identity,
            runtime,
            world,
            governance,
            knowledge,
            plugins,
            reflection,
            sandbox,
            session_start: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            message_count: 0,
        }
    }

    fn register_builtin_plugins(plugins: &mut PluginManager) {
        use std::collections::HashMap;

        plugins.register(plugins::Plugin {
            name: "system".to_string(),
            version: "1.0".to_string(),
            description: "System information and monitoring".to_string(),
            author: "SEQUENCE OS".to_string(),
            commands: vec!["info".to_string(), "uptime".to_string(), "net".to_string()],
            enabled: true,
            config: HashMap::new(),
        });

        plugins.register(plugins::Plugin {
            name: "code".to_string(),
            version: "1.0".to_string(),
            description: "Code linting and formatting".to_string(),
            author: "SEQUENCE OS".to_string(),
            commands: vec!["lint".to_string(), "format".to_string()],
            enabled: true,
            config: HashMap::new(),
        });

        plugins.register(plugins::Plugin {
            name: "web".to_string(),
            version: "1.0".to_string(),
            description: "Web fetching and scraping".to_string(),
            author: "SEQUENCE OS".to_string(),
            commands: vec!["fetch".to_string()],
            enabled: true,
            config: HashMap::new(),
        });
    }

    fn ensure_dirs(dir: &str) {
        let dirs = [
            dir,
            &format!("{}/memory/episodic", dir),
            &format!("{}/memory/semantic", dir),
            &format!("{}/memory/procedural", dir),
            &format!("{}/tasks/completed", dir),
            &format!("{}/tasks/templates", dir),
            &format!("{}/governance", dir),
            &format!("{}/relationships", dir),
            &format!("{}/evolution/snapshots", dir),
            &format!("{}/world", dir),
            &format!("{}/knowledge", dir),
            &format!("{}/plugins", dir),
            &format!("{}/sandbox", dir),
        ];

        for d in &dirs {
            let _ = fs::create_dir_all(d);
        }
    }

    /// Process a user input and return a response
    fn process_input(&mut self, input: &str) -> String {
        self.message_count += 1;

        // Check governance
        if !self.governance.check_input(input) {
            return "⛔ Action blocked by governance rules.".to_string();
        }

        // Record in episodic memory
        self.memory.record_episodic(
            &format!("User input: {}", input),
            "interaction",
            &["user", "input"],
        );

        // Handle slash commands
        if input.starts_with('/') {
            return self.handle_command(input);
        }

        // Regular conversation - get AI response
        self.handle_conversation(input)
    }

    fn handle_command(&mut self, input: &str) -> String {
        let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
        let command = parts[0].to_lowercase();
        let args = parts.get(1).unwrap_or(&"").trim();

        match command.as_str() {
            "status" => self.cmd_status(),
            "memory" | "mem" => self.cmd_memory(args),
            "tasks" => self.cmd_tasks(args),
            "identity" | "id" => self.cmd_identity(args),
            "governance" | "gov" => self.cmd_governance(args),
            "world" => self.cmd_world(args),
            "evolve" => self.cmd_evolve(),
            "snapshot" => self.cmd_snapshot(),
            "run" => self.cmd_run(args),
            "read" => self.cmd_read(args),
            "write" => self.cmd_write(args),
            "search" | "find" => self.cmd_search(args),
            "knowledge" | "kg" => self.cmd_knowledge(args),
            "plugin" | "pl" => self.cmd_plugin(args),
            "reflect" => self.cmd_reflect(args),
            "sandbox" | "exec" => self.cmd_sandbox(args),
            "webapi" => self.cmd_webapi(args),
            "help" | "h" => self.cmd_help(),
            "quit" | "exit" | "q" => {
                self.save();
                "Goodbye! State saved.".to_string()
            }
            _ => format!("Unknown command: /{}. Type /help for available commands.", command),
        }
    }

    fn cmd_status(&self) -> String {
        format!(
            "📊 SEQUENCE OS¹ Status\n\
             ─────────────────────\n\
             Version:     {}\n\
             Identity:    {} ({})\n\
             Session:     {}\n\
             Messages:    {}\n\
             Memory:      {} episodic, {} semantic, {} procedural\n\
             Knowledge:   {} nodes, {} edges\n\
             Plugins:     {} registered\n\
             Reflections: {} recorded\n\
             Tasks:       {} pending, {} running, {} completed\n\
             Governance:  {} rules active",
            VERSION,
            self.identity.name,
            self.identity.role,
            self.session_start,
            self.message_count,
            self.memory.episodic_count(),
            self.memory.semantic_count(),
            self.memory.procedural_count(),
            self.knowledge.nodes.len(),
            self.knowledge.edges.len(),
            self.plugins.plugins.len(),
            self.reflection.reflections.len(),
            self.runtime.pending_count(),
            self.runtime.running_count(),
            self.runtime.completed_count(),
            self.governance.rule_count(),
        )
    }

    fn cmd_memory(&mut self, args: &str) -> String {
        if args.is_empty() {
            return "Memory commands: /memory list, /memory search <query>, /memory recent [n], /memory add <content>".to_string();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];

        match sub {
            "list" | "ls" => {
                let recent = self.memory.recent_episodic(5);
                if recent.is_empty() {
                    "No memories recorded yet.".to_string()
                } else {
                    let mut output = "📜 Recent Memories:\n".to_string();
                    for (i, mem) in recent.iter().enumerate() {
                        output.push_str(&format!("  {}. [{}] {}\n", 
                            i + 1, 
                            mem.category,
                            if mem.content.len() > 80 { 
                                &mem.content[..80] 
                            } else { 
                                &mem.content 
                            }
                        ));
                    }
                    output
                }
            }
            "search" | "find" => {
                let query = parts.get(1).unwrap_or(&"");
                if query.is_empty() {
                    "Usage: /memory search <query>".to_string()
                } else {
                    let results = self.memory.search(query);
                    if results.is_empty() {
                        format!("No memories found for: {}", query)
                    } else {
                        let mut output = format!("🔍 Found {} memories:\n", results.len());
                        for mem in &results[..results.len().min(10)] {
                            output.push_str(&format!("  - [{}] {}\n", mem.category, mem.content));
                        }
                        output
                    }
                }
            }
            "recent" => {
                let n: usize = parts.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);
                let recent = self.memory.recent_episodic(n);
                if recent.is_empty() {
                    "No memories yet.".to_string()
                } else {
                    let mut output = format!("📜 {} Recent Memories:\n", recent.len());
                    for mem in &recent {
                        output.push_str(&format!("  • {}\n", mem.content));
                    }
                    output
                }
            }
            "add" => {
                let content = parts.get(1).unwrap_or(&"");
                if content.is_empty() {
                    "Usage: /memory add <content>".to_string()
                } else {
                    self.memory.record_episodic(content, "manual", &["user-added"]);
                    format!("✅ Memory added: {}", content)
                }
            }
            "stats" => {
                format!(
                    "📊 Memory Statistics:\n  Episodic: {}\n  Semantic: {}\n  Procedural: {}",
                    self.memory.episodic_count(),
                    self.memory.semantic_count(),
                    self.memory.procedural_count(),
                )
            }
            _ => format!("Unknown memory command: {}. Use /memory for help.", sub),
        }
    }

    fn cmd_tasks(&mut self, args: &str) -> String {
        if args.is_empty() {
            return "Task commands: /tasks list, /tasks add <desc>, /tasks complete <id>, /tasks remove <id>".to_string();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];

        match sub {
            "list" | "ls" => {
                let tasks = self.runtime.list_tasks();
                if tasks.is_empty() {
                    "No tasks in queue.".to_string()
                } else {
                    let mut output = "📋 Task Queue:\n".to_string();
                    for task in tasks {
                        output.push_str(&format!(
                            "  [{}] {} - {}\n",
                            task.id.get(..8).unwrap_or(&task.id),
                            task.description,
                            match task.status {
                                runtime::TaskStatus::Pending => "⏳ Pending",
                                runtime::TaskStatus::Running => "🔄 Running",
                                runtime::TaskStatus::Completed => "✅ Done",
                                runtime::TaskStatus::Failed(_) => "❌ Failed",
                                runtime::TaskStatus::Cancelled => "🚫 Cancelled",
                            }
                        ));
                    }
                    output
                }
            }
            "add" => {
                let desc = parts.get(1).unwrap_or(&"");
                if desc.is_empty() {
                    "Usage: /tasks add <description>".to_string()
                } else {
                    let id = self.runtime.add_task(desc);
                    format!("✅ Task added: {} (ID: {})", desc, id)
                }
            }
            "complete" | "done" => {
                let id = parts.get(1).unwrap_or(&"");
                if id.is_empty() {
                    "Usage: /tasks complete <id>".to_string()
                } else {
                    self.runtime.complete_task(id);
                    format!("✅ Task {} marked as complete", &id[..8.min(id.len())])
                }
            }
            "remove" | "rm" => {
                let id = parts.get(1).unwrap_or(&"");
                if id.is_empty() {
                    "Usage: /tasks remove <id>".to_string()
                } else {
                    self.runtime.remove_task(id);
                    format!("🗑️ Task {} removed", &id[..8.min(id.len())])
                }
            }
            _ => format!("Unknown task command: {}", sub),
        }
    }

    fn cmd_identity(&self, args: &str) -> String {
        if args.is_empty() {
            return self.identity.display();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];

        match sub {
            "name" => {
                let name = parts.get(1).unwrap_or(&"");
                if name.is_empty() {
                    format!("Current name: {}", self.identity.name)
                } else {
                    format!("Name change requested to: {} (use /evolve to apply)", name)
                }
            }
            "role" => {
                let role = parts.get(1).unwrap_or(&"");
                if role.is_empty() {
                    format!("Current role: {}", self.identity.role)
                } else {
                    format!("Role change requested to: {} (use /evolve to apply)", role)
                }
            }
            "values" => {
                let mut output = "💎 Values:\n".to_string();
                for v in &self.identity.values {
                    output.push_str(&format!("  • {}\n", v));
                }
                output
            }
            _ => self.identity.display(),
        }
    }

    fn cmd_governance(&self, args: &str) -> String {
        if args.is_empty() {
            return self.governance.display();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];

        match sub {
            "rules" => {
                self.governance.list_rules()
            }
            "permissions" | "perms" => {
                self.governance.list_permissions()
            }
            _ => self.governance.display(),
        }
    }

    fn cmd_world(&self, args: &str) -> String {
        if args.is_empty() {
            return "World commands: /world ls [path], /world read <file>, /world pwd".to_string();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];

        match sub {
            "ls" | "list" => {
                let path = parts.get(1).unwrap_or(&".");
                self.world.list_dir(path)
            }
            "pwd" => {
                format!("📁 {}", self.world.current_dir())
            }
            "read" => {
                let file = parts.get(1).unwrap_or(&"");
                if file.is_empty() {
                    "Usage: /world read <file>".to_string()
                } else {
                    self.world.read_file(file)
                }
            }
            _ => format!("Unknown world command: {}", sub),
        }
    }

    fn cmd_evolve(&mut self) -> String {
        // Record evolution trigger
        self.memory.record_episodic(
            "System requested self-evolution",
            "evolution",
            &["system", "self-improvement"],
        );

        let suggestions = self.generate_evolution_suggestions();
        format!(
            "🧬 Self-Evolution Analysis\n\
             ───────────────────────\n\
             {}\n\n\
             💡 To apply changes, use specific commands:\n\
             /identity name <new_name>\n\
             /memory add <new_knowledge>\n\
             /tasks add <new_goal>",
            suggestions
        )
    }

    fn generate_evolution_suggestions(&self) -> String {
        let mut suggestions: Vec<String> = Vec::new();

        // Analyze memory patterns
        let mem_count = self.memory.episodic_count();
        if mem_count > 100 {
            suggestions.push("• Memory has grown significantly - consider semantic organization".to_string());
        }

        // Analyze task patterns
        let failed = self.runtime.failed_count();
        if failed > 0 {
            suggestions.push(format!("• {} failed tasks - review and learn from failures", failed));
        }

        // Analyze session patterns
        if self.message_count > 50 {
            suggestions.push("• Active session - consider saving important learnings to semantic memory".to_string());
        }

        if suggestions.is_empty() {
            "• System is stable, no immediate evolution needed".to_string()
        } else {
            suggestions.join("\n")
        }
    }

    fn cmd_snapshot(&self) -> String {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let snapshot_path = format!(
            "{}/{}/evolution/snapshots/snapshot_{}.json",
            env::var("HOME").unwrap_or_else(|_| ".".to_string()),
            SEQUENCE_DIR,
            timestamp
        );

        let snapshot = serde_json::json!({
            "timestamp": Local::now().to_string(),
            "identity": {
                "name": self.identity.name,
                "role": self.identity.role,
            },
            "memory": {
                "episodic": self.memory.episodic_count(),
                "semantic": self.memory.semantic_count(),
                "procedural": self.memory.procedural_count(),
            },
            "tasks": {
                "pending": self.runtime.pending_count(),
                "completed": self.runtime.completed_count(),
            },
            "session": {
                "started": self.session_start,
                "messages": self.message_count,
            }
        });

        match serde_json::to_string_pretty(&snapshot) {
            Ok(json) => {
                if fs::write(&snapshot_path, json).is_ok() {
                    format!("📸 Snapshot saved: {}", snapshot_path)
                } else {
                    "❌ Failed to save snapshot".to_string()
                }
            }
            Err(e) => format!("❌ Failed to serialize snapshot: {}", e),
        }
    }

    fn cmd_run(&self, args: &str) -> String {
        if args.is_empty() {
            return "Usage: /run <command>".to_string();
        }

        self.world.execute_command(args)
    }

    fn cmd_read(&self, args: &str) -> String {
        if args.is_empty() {
            return "Usage: /read <file>".to_string();
        }
        self.world.read_file(args)
    }

    fn cmd_write(&mut self, args: &str) -> String {
        if args.is_empty() {
            return "Usage: /write <file> <content>".to_string();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let file = parts[0];
        let content = parts.get(1).unwrap_or(&"");

        if content.is_empty() {
            return "Usage: /write <file> <content>".to_string();
        }

        match fs::write(file, content) {
            Ok(()) => {
                self.memory.record_episodic(
                    &format!("Wrote file: {}", file),
                    "action",
                    &["write", "file"],
                );
                format!("✅ Wrote {} bytes to {}", content.len(), file)
            }
            Err(e) => format!("❌ Failed to write file: {}", e),
        }
    }

    fn cmd_search(&self, args: &str) -> String {
        if args.is_empty() {
            return "Usage: /search <pattern> [path]".to_string();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let pattern = parts[0];
        let path = parts.get(1).unwrap_or(&".");

        self.world.search_files(path, pattern)
    }

    fn cmd_help(&self) -> String {
        r#"📖 SEQUENCE OS¹ Help

System Commands:
  /status              Show system status
  /help                Show this help
  /quit                Save and exit

Memory:
  /memory list         List recent memories
  /memory search <q>   Search memories
  /memory recent [n]   Show n recent memories
  /memory add <text>   Add a memory
  /memory stats        Memory statistics

Tasks:
  /tasks list          List all tasks
  /tasks add <desc>    Add a new task
  /tasks complete <id> Mark task complete
  /tasks remove <id>   Remove a task

Identity:
  /identity            Show identity
  /identity name       Show/change name
  /identity role       Show/change role
  /identity values     Show values

Governance:
  /governance          Show governance status
  /governance rules    List safety rules
  /governance perms    List permissions

World:
  /world ls [path]     List directory
  /world pwd           Show current directory
  /world read <file>   Read a file

Actions:
  /run <cmd>           Execute shell command
  /read <file>         Read a file
  /write <f> <content> Write to a file
  /search <p> [path]   Search files

Evolution:
  /evolve              Trigger self-evolution
  /snapshot            Save system state"#.to_string()
    }

    fn handle_conversation(&mut self, input: &str) -> String {
        // Retrieve relevant memory context
        let context = self.memory.get_context(input);

        // Build system prompt with identity and context
        let system_prompt = self.build_system_prompt(&context);

        // Call AI model
        let response = models::chat(&system_prompt, input);

        // Record interaction
        let response_preview: String = response.chars().take(200).collect();
        self.memory.record_episodic(
            &format!("Q: {}\nA: {}", input, response_preview),
            "conversation",
            &["ai", "response"],
        );

        response
    }

    fn build_system_prompt(&self, context: &str) -> String {
        let mut prompt = format!(
            "You are {}, an AI with persistent memory and identity.\n\
             Role: {}\n\
             Values: {}\n\n",
            self.identity.name,
            self.identity.role,
            self.identity.values.join(", "),
        );

        if !context.is_empty() {
            prompt.push_str("Relevant past context:\n");
            prompt.push_str(context);
            prompt.push_str("\n\n");
        }

        prompt.push_str(
            "Use this context to provide informed, consistent responses. \
             You remember past interactions and can build on them."
        );

        prompt
    }

    /// Knowledge Graph commands
    fn cmd_knowledge(&mut self, args: &str) -> String {
        if args.is_empty() {
            return "Knowledge commands: /knowledge add <id> <label>, /knowledge link <from> <to> <relation>, /knowledge search <query>, /knowledge neighbors <id>, /knowledge stats".to_string();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];

        match sub {
            "add" => {
                let rest = parts.get(1).unwrap_or(&"");
                let parts2: Vec<&str> = rest.splitn(2, ' ').collect();
                if parts2.len() < 2 {
                    return "Usage: /knowledge add <id> <label>".to_string();
                }
                self.knowledge.add_node(parts2[0], parts2[1], std::collections::HashMap::new());
                format!("✅ Added node: {} ({})", parts2[0], parts2[1])
            }
            "link" => {
                let rest = parts.get(1).unwrap_or(&"");
                let parts2: Vec<&str> = rest.splitn(3, ' ').collect();
                if parts2.len() < 3 {
                    return "Usage: /knowledge link <from> <to> <relation>".to_string();
                }
                self.knowledge.add_edge(parts2[0], parts2[1], parts2[2], 0.5);
                format!("✅ Linked: {} -> {} ({})", parts2[0], parts2[1], parts2[2])
            }
            "search" | "find" => {
                let query = parts.get(1).unwrap_or(&"");
                if query.is_empty() {
                    return "Usage: /knowledge search <query>".to_string();
                }
                let results = self.knowledge.search(query);
                if results.is_empty() {
                    format!("No results for: {}", query)
                } else {
                    let mut output = format!("🔍 Found {} results:\n", results.len());
                    for node in results.iter().take(10) {
                        output.push_str(&format!("  - {} [{}]\n", node.label, node.id));
                    }
                    output
                }
            }
            "neighbors" => {
                let node_id = parts.get(1).unwrap_or(&"");
                if node_id.is_empty() {
                    return "Usage: /knowledge neighbors <id>".to_string();
                }
                let neighbors = self.knowledge.get_neighbors(node_id);
                if neighbors.is_empty() {
                    format!("No neighbors for: {}", node_id)
                } else {
                    let mut output = format!("🔗 Neighbors of {}:\n", node_id);
                    for (node, edge) in &neighbors {
                        output.push_str(&format!("  - {} ({}) [weight: {}]\n", node.label, edge.relation, edge.weight));
                    }
                    output
                }
            }
            "stats" => self.knowledge.stats(),
            _ => format!("Unknown knowledge command: {}", sub),
        }
    }

    /// Plugin commands
    fn cmd_plugin(&mut self, args: &str) -> String {
        if args.is_empty() {
            return "Plugin commands: /plugin list, /plugin enable <name>, /plugin disable <name>, /plugin exec <name>/<cmd> [args]".to_string();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];

        match sub {
            "list" | "ls" => self.plugins.list(),
            "enable" => {
                let name = parts.get(1).unwrap_or(&"");
                if name.is_empty() {
                    return "Usage: /plugin enable <name>".to_string();
                }
                if self.plugins.enable(name) {
                    format!("✅ Enabled plugin: {}", name)
                } else {
                    format!("Plugin not found: {}", name)
                }
            }
            "disable" => {
                let name = parts.get(1).unwrap_or(&"");
                if name.is_empty() {
                    return "Usage: /plugin disable <name>".to_string();
                }
                if self.plugins.disable(name) {
                    format!("❌ Disabled plugin: {}", name)
                } else {
                    format!("Plugin not found: {}", name)
                }
            }
            "exec" | "run" => {
                let rest = parts.get(1).unwrap_or(&"");
                let parts2: Vec<&str> = rest.splitn(2, '/').collect();
                if parts2.len() < 2 {
                    return "Usage: /plugin exec <name>/<cmd> [args]".to_string();
                }
                let plugin_name = parts2[0];
                let cmd_args: Vec<String> = parts2[1].split_whitespace().map(|s| s.to_string()).collect();
                let cmd = cmd_args[0].clone();
                let args = &cmd_args[1..];
                
                match self.plugins.execute(plugin_name, &cmd, args) {
                    Some(result) => result,
                    None => format!("Plugin or command not found: {}/{}", plugin_name, cmd),
                }
            }
            _ => format!("Unknown plugin command: {}", sub),
        }
    }

    /// Self-Reflection commands
    fn cmd_reflect(&mut self, args: &str) -> String {
        if args.is_empty() {
            return "Reflection commands: /reflect analyze, /reflect record <category> <observation> <insight>, /reflect goal <add|remove> <goal>".to_string();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];

        match sub {
            "analyze" => self.reflection.analyze_patterns(),
            "record" => {
                let rest = parts.get(1).unwrap_or(&"");
                let parts2: Vec<&str> = rest.splitn(3, ' ').collect();
                if parts2.len() < 3 {
                    return "Usage: /reflect record <category> <observation> <insight>".to_string();
                }
                self.reflection.record(parts2[0], parts2[1], parts2[2], vec![]);
                format!("✅ Reflection recorded: [{}] {}", parts2[0], parts2[1])
            }
            "goal" => {
                let rest = parts.get(1).unwrap_or(&"");
                let parts2: Vec<&str> = rest.splitn(2, ' ').collect();
                if parts2.len() < 2 {
                    return "Usage: /reflect goal <add|remove> <goal>".to_string();
                }
                match parts2[0] {
                    "add" => {
                        self.reflection.add_goal(parts2[1]);
                        format!("✅ Goal added: {}", parts2[1])
                    }
                    "remove" => {
                        self.reflection.remove_goal(parts2[1]);
                        format!("✅ Goal removed: {}", parts2[1])
                    }
                    _ => "Usage: /reflect goal <add|remove> <goal>".to_string(),
                }
            }
            _ => format!("Unknown reflection command: {}", sub),
        }
    }

    /// Code Sandbox commands
    fn cmd_sandbox(&mut self, args: &str) -> String {
        if args.is_empty() {
            return "Sandbox commands: /sandbox run <language> <code>, /sandbox exec <code>".to_string();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];

        match sub {
            "run" | "exec" => {
                let rest = parts.get(1).unwrap_or(&"");
                let parts2: Vec<&str> = rest.splitn(2, ' ').collect();
                if parts2.len() < 2 {
                    return "Usage: /sandbox run <language> <code>".to_string();
                }
                let language = parts2[0];
                let code = parts2[1];
                let result = self.sandbox.execute(code, Some(language));
                self.sandbox.format_result(&result)
            }
            _ => format!("Unknown sandbox command: {}", sub),
        }
    }

    /// Web API commands
    fn cmd_webapi(&self, args: &str) -> String {
        if args.is_empty() {
            return "Web API commands: /webapi start [port], /webapi status, /webapi docs".to_string();
        }

        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let sub = parts[0];

        match sub {
            "start" => {
                let port: u16 = parts.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(8080);
                format!("🌐 Web API would start on port {} (use /webapi docs for endpoints)", port)
            }
            "status" => "🌐 Web API is available at http://127.0.0.1:8080".to_string(),
            "docs" => {
                "📖 Web API Endpoints:\n  GET  /api/status   - System status\n  GET  /api/health   - Health check\n  GET  /api/tools    - List tools\n  GET  /api/memory   - List memory\n  POST /api/execute  - Execute command\n  GET  /api/docs     - This documentation".to_string()
            }
            _ => format!("Unknown webapi command: {}", sub),
        }
    }

    /// Save all state to disk
    fn save(&mut self) {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let sequence_dir = format!("{}/{}", home, SEQUENCE_DIR);

        self.memory.save(&sequence_dir);
        self.identity.save(&sequence_dir);
        self.runtime.save(&sequence_dir);
        self.governance.save(&sequence_dir);
        self.knowledge.save();
        self.plugins.save();
        self.reflection.save();

        println!("\n💾 State saved.");
    }
}

fn print_usage() {
    println!("SEQUENCE OS¹ v{} - AI Operating System", VERSION);
    println!();
    println!("Usage: sequence [COMMAND]");
    println!();
    println!("Commands:");
    println!("  start          Start the REPL (default)");
    println!("  status         Show system status");
    println!("  memory         Show memory statistics");
    println!("  tasks          Show task queue");
    println!("  identity       Show identity");
    println!("  governance     Show governance rules");
    println!("  evolve         Trigger self-evolution");
    println!("  snapshot       Create state snapshot");
    println!("  help           Show this help");
    println!();
    println!("Examples:");
    println!("  sequence start         # Start interactive mode");
    println!("  sequence status        # Quick status check");
    println!("  sequence snapshot      # Save current state");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "start" | "repl" | "i" | "-i" | "--interactive" => {
                // Start REPL below
            }
            "status" => {
                let os = SequenceOS::new();
                println!("{}", os.cmd_status());
                return;
            }
            "memory" => {
                let mut os = SequenceOS::new();
                println!("{}", os.cmd_memory("stats"));
                return;
            }
            "tasks" => {
                let mut os = SequenceOS::new();
                println!("{}", os.cmd_tasks("list"));
                return;
            }
            "identity" => {
                let os = SequenceOS::new();
                println!("{}", os.cmd_identity(""));
                return;
            }
            "governance" => {
                let os = SequenceOS::new();
                println!("{}", os.cmd_governance(""));
                return;
            }
            "evolve" => {
                let mut os = SequenceOS::new();
                println!("{}", os.cmd_evolve());
                return;
            }
            "snapshot" => {
                let os = SequenceOS::new();
                println!("{}", os.cmd_snapshot());
                return;
            }
            "help" | "--help" | "-h" => {
                print_usage();
                return;
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
                print_usage();
                return;
            }
        }
    }

    // Start REPL
    let mut os = SequenceOS::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    println!("\nType /help for commands, /quit to exit.\n");

    loop {
        print!("sequence> ");
        let _ = stdout.flush();

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        let response = os.process_input(&input);
        println!("{}", response);

        if response == "Goodbye! State saved." {
            break;
        }
    }
}
