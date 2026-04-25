use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// 进化模式
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum EvolveMode {
    Diagnose,    // 诊断
    Update,      // 更新
    Expand,      // 扩展
    Optimize,    // 优化
    Learn,       // 学习
    Directions,  // 进化方向
    Full,        // 完整进化
}

/// 工具状态
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolStatus {
    name: String,
    path: String,
    compiled: bool,
    last_build: Option<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
    size: u64,
}

/// 进化记录
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvolutionRecord {
    timestamp: String,
    action: String,
    status: String,
    details: String,
}

/// 用户偏好
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserPreferences {
    preferred_provider: String,
    preferred_model: String,
    frequently_used: Vec<String>,
    last_used: HashMap<String, String>,
    command_count: HashMap<String, u64>,
}

/// 进化引擎
struct EvolutionEngine {
    base_dir: String,
    tools: Vec<ToolStatus>,
    records: Vec<EvolutionRecord>,
    preferences: UserPreferences,
}

impl EvolutionEngine {
    fn new() -> Self {
        let base_dir = env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "/root".to_string());

        Self {
            base_dir,
            tools: Vec::new(),
            records: Vec::new(),
            preferences: UserPreferences {
                preferred_provider: "ollama".to_string(),
                preferred_model: "qwen2.5-coder:7b".to_string(),
                frequently_used: Vec::new(),
                last_used: HashMap::new(),
                command_count: HashMap::new(),
            },
        }
    }

    /// 加载偏好设置
    fn load_preferences(&mut self) {
        let pref_path = format!("{}/.evolve_preferences.json", self.base_dir);
        if let Ok(content) = fs::read_to_string(&pref_path) {
            if let Ok(prefs) = serde_json::from_str::<UserPreferences>(&content) {
                self.preferences = prefs;
            }
        }
    }

    /// 保存偏好设置
    fn save_preferences(&self) {
        let pref_path = format!("{}/.evolve_preferences.json", self.base_dir);
        if let Ok(json) = serde_json::to_string_pretty(&self.preferences) {
            let _ = fs::write(&pref_path, json);
        }
    }

    /// 加载进化记录
    #[allow(dead_code)]
    fn load_records(&mut self) {
        let record_path = format!("{}/.evolve_records.json", self.base_dir);
        if let Ok(content) = fs::read_to_string(&record_path) {
            if let Ok(records) = serde_json::from_str::<Vec<EvolutionRecord>>(&content) {
                self.records = records;
            }
        }
    }

    /// 保存进化记录
    fn save_records(&self) {
        let record_path = format!("{}/.evolve_records.json", self.base_dir);
        if let Ok(json) = serde_json::to_string_pretty(&self.records) {
            let _ = fs::write(&record_path, json);
        }
    }

    /// 添加进化记录
    fn add_record(&mut self, action: &str, status: &str, details: &str) {
        self.records.push(EvolutionRecord {
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            action: action.to_string(),
            status: status.to_string(),
            details: details.to_string(),
        });

        // 只保留最近 100 条记录
        if self.records.len() > 100 {
            self.records.drain(..self.records.len() - 100);
        }
    }

    /// 诊断所有工具
    fn diagnose(&mut self) {
        println!("🔍 开始诊断所有工具...");
        println!();

        let tools = [
            "cat", "head", "tail", "sort", "uniq", "wc",
            "grep", "find", "glob", "diff", "du",
            "info", "ai", "qwencode", "qwencode-git",
            "patch", "httpserver", "mysh", "rust-utils",
        ];

        let mut healthy = 0;
        let mut issues = 0;

        for tool in &tools {
            let status = self.check_tool(tool);
            if status.compiled && status.errors.is_empty() {
                println!("  ✅ {} - 正常", tool);
                healthy += 1;
            } else {
                println!("  ⚠️  {} - 有问题", tool);
                if !status.errors.is_empty() {
                    for err in &status.errors {
                        println!("      ❌ {}", err);
                    }
                }
                if !status.warnings.is_empty() {
                    for warn in &status.warnings {
                        println!("      ⚠️  {}", warn);
                    }
                }
                issues += 1;
            }
            self.tools.push(status);
        }

        println!();
        println!("📊 诊断结果:");
        println!("  健康工具: {}", healthy);
        println!("  问题工具: {}", issues);
        println!("  总计: {}", tools.len());

        self.add_record("diagnose", "completed", &format!("{} healthy, {} issues", healthy, issues));
    }

    /// 检查单个工具
    fn check_tool(&self, name: &str) -> ToolStatus {
        let tool_path = format!("{}/{}", self.base_dir, name);
        let target_path = format!("{}/target/debug/{}", tool_path, name);

        // 特殊处理 rust-utils
        let target_path = if name == "rust-utils" {
            format!("{}/target/debug/rust-utils", tool_path)
        } else {
            target_path
        };

        let compiled = Path::new(&target_path).exists();
        let size = if compiled {
            fs::metadata(&target_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        let last_build = if compiled {
            fs::metadata(&target_path)
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| format!("{:?}", t))
        } else {
            None
        };

        ToolStatus {
            name: name.to_string(),
            path: tool_path,
            compiled,
            last_build,
            warnings: Vec::new(),
            errors: Vec::new(),
            size,
        }
    }

    /// 更新所有工具
    fn update(&mut self) {
        println!("🔄 开始更新所有工具...");
        println!();

        let tools = [
            "cat", "head", "tail", "sort", "uniq", "wc",
            "grep", "find", "glob", "diff", "du",
            "info", "ai", "qwencode", "qwencode-git",
            "patch", "httpserver", "mysh", "rust-utils",
        ];

        let mut updated = 0;
        let mut failed = 0;

        for tool in &tools {
            println!("📦 更新 {}...", tool);

            let tool_path = format!("{}/{}", self.base_dir, tool);
            if !Path::new(&tool_path).exists() {
                println!("  ⏭️  跳过（目录不存在）");
                continue;
            }

            // 检查 Cargo.toml
            let cargo_path = format!("{}/Cargo.toml", tool_path);
            if !Path::new(&cargo_path).exists() {
                println!("  ⏭️  跳过（无 Cargo.toml）");
                continue;
            }

            // 编译
            let output = Command::new("cargo")
                .args(&["build", "--release"])
                .current_dir(&tool_path)
                .output();

            match output {
                Ok(result) => {
                    if result.status.success() {
                        println!("  ✅ 编译成功");
                        updated += 1;
                        self.add_record("update", "success", tool);
                    } else {
                        let stderr = String::from_utf8_lossy(&result.stderr);
                        println!("  ❌ 编译失败: {}", stderr.lines().next().unwrap_or("unknown"));
                        failed += 1;
                        self.add_record("update", "failed", &format!("{}: {}", tool, stderr.lines().next().unwrap_or("unknown")));
                    }
                }
                Err(e) => {
                    println!("  ❌ 执行失败: {}", e);
                    failed += 1;
                    self.add_record("update", "error", &format!("{}: {}", tool, e));
                }
            }
        }

        println!();
        println!("📊 更新结果:");
        println!("  成功: {}", updated);
        println!("  失败: {}", failed);

        self.add_record("update_all", "completed", &format!("{} updated, {} failed", updated, failed));
    }

    /// 扩展功能 - 根据需求生成新工具
    fn expand(&mut self) {
        println!("🌱 开始功能扩展...");
        println!();

        // 分析现有工具，找出可以扩展的方向
        let existing_tools = [
            "cat", "head", "tail", "sort", "uniq", "wc",
            "grep", "find", "glob", "diff", "du",
            "info", "ai", "qwencode", "qwencode-git",
            "patch", "httpserver", "mysh", "rust-utils",
        ];

        println!("📋 现有工具 ({} 个):", existing_tools.len());
        for tool in &existing_tools {
            println!("  - {}", tool);
        }

        println!();
        println!("💡 建议扩展方向:");
        println!("  1. 网络工具 - curl, wget, ping, traceroute");
        println!("  2. 压缩工具 - tar, zip, gzip");
        println!("  3. 文本处理 - sed, awk, tr");
        println!("  4. 系统监控 - top, ps, df");
        println!("  5. 安全工具 - chmod, chown, openssl");
        println!();
        println!("🤖 使用 AI 生成新工具:");
        println!("  ./ai.sh -p qwen -m qwen-coder-plus-latest '用 Rust 实现一个简化版的 tar 工具'");

        self.add_record("expand", "suggestions_generated", "5 expansion directions suggested");
    }

    /// 优化性能
    fn optimize(&mut self) {
        println!("⚡ 开始性能优化...");
        println!();

        let tools = [
            "cat", "head", "tail", "sort", "uniq", "wc",
            "grep", "find", "glob", "diff", "du",
            "info", "ai", "qwencode", "qwencode-git",
            "patch", "httpserver", "mysh", "rust-utils",
        ];

        let mut optimized = 0;

        for tool in &tools {
            let tool_path = format!("{}/{}", self.base_dir, tool);
            if !Path::new(&tool_path).exists() {
                continue;
            }

            // 检查是否有 Cargo.toml
            let cargo_path = format!("{}/Cargo.toml", tool_path);
            if !Path::new(&cargo_path).exists() {
                continue;
            }

            // 检查 release 版本是否存在
            let release_path = format!("{}/target/release/{}", tool_path, tool);
            let release_exists = Path::new(&release_path).exists();

            if !release_exists {
                println!("📦 编译 {} (release)...", tool);

                let output = Command::new("cargo")
                    .args(&["build", "--release"])
                    .current_dir(&tool_path)
                    .output();

                match output {
                    Ok(result) => {
                        if result.status.success() {
                            println!("  ✅ {} 优化完成", tool);
                            optimized += 1;
                        } else {
                            println!("  ⚠️  {} 编译失败", tool);
                        }
                    }
                    Err(e) => {
                        println!("  ❌ {} 执行失败: {}", tool, e);
                    }
                }
            } else {
                println!("  ✅ {} 已优化", tool);
                optimized += 1;
            }
        }

        println!();
        println!("📊 优化结果: {} 个工具已优化", optimized);

        self.add_record("optimize", "completed", &format!("{} tools optimized", optimized));
    }

    /// 学习用户偏好
    fn learn(&mut self) {
        println!("🧠 开始学习用户偏好...");
        println!();

        // 加载现有偏好
        self.load_preferences();

        // 分析使用历史
        println!("📊 当前偏好:");
        println!("  首选提供商: {}", self.preferences.preferred_provider);
        println!("  首选模型: {}", self.preferences.preferred_model);
        println!("  常用工具: {:?}", self.preferences.frequently_used);
        println!("  命令计数: {:?}", self.preferences.command_count);

        println!();
        println!("💡 建议:");
        println!("  1. 设置默认提供商: export AI_PROVIDER=qwen");
        println!("  2. 设置默认模型: export AI_QWEN_MODEL=qwen-coder-plus-latest");
        println!("  3. 使用 alias 简化命令");

        // 保存偏好
        self.save_preferences();

        self.add_record("learn", "preferences_updated", "User preferences saved");
    }

    /// 完整进化
    fn full_evolve(&mut self) {
        println!("🚀 开始完整进化...");
        println!();

        // 1. 诊断
        self.diagnose();
        println!();

        // 2. 更新
        self.update();
        println!();

        // 3. 优化
        self.optimize();
        println!();

        // 4. 扩展建议
        self.expand();
        println!();

        // 5. 学习
        self.learn();
        println!();

        // 6. 保存记录
        self.save_records();

        println!("🎉 进化完成!");
        println!();
        println!("📊 进化统计:");
        println!("  总记录: {}", self.records.len());
        println!("  工具数量: {}", self.tools.len());
    }

    /// 显示进化历史
    fn history(&self) {
        println!("📜 进化历史 (最近 20 条):");
        println!();

        let records = if self.records.len() > 20 {
            &self.records[self.records.len() - 20..]
        } else {
            &self.records
        };

        for record in records {
            println!("  [{}] {} - {} - {}",
                record.timestamp,
                record.action,
                record.status,
                record.details
            );
        }
    }

    /// 显示进化方向
    fn suggest_directions(&mut self) {
        println!("🧭 分析项目进化方向...");
        println!();

        // 定义建议的进化方向
        let directions = [
            ("网络工具", "curl, wget, ping, traceroute, ip, ss", "增强网络诊断和请求能力"),
            ("压缩工具", "tar, zip, gzip, unzip, 7z", "文件打包和压缩"),
            ("文本处理", "sed, awk, tr, col, fmt", "高级文本流处理"),
            ("系统监控", "top, ps, df, free, vmstat", "系统资源和进程监控"),
            ("安全工具", "chmod, chown, openssl, sha256sum, md5sum", "文件权限和加密校验"),
            ("代码统计", "tokei, cloc, scc", "代码行数统计和复杂度分析"),
            ("性能基准", "hyperfine, time, perf", "命令性能测试和基准对比"),
            ("JSON/CSV", "jq, xsv, csvtk", "结构化数据处理"),
            ("终端美化", "starship, zoxide, fzf, broot", "现代化终端体验"),
            ("容器/虚拟化", "docker, podman, nerdctl", "容器管理工具"),
        ];

        let mut suggested = Vec::new();

        for (category, tools, desc) in &directions {
            // 简单检查是否已有相关工具（这里假设没有，因为都是新类别）
            // 实际项目中可以检查 /root/ 下是否有对应目录
            suggested.push((category, tools, desc));
        }

        println!("📋 建议的进化方向 (按优先级排序):");
        println!();
        for (i, (cat, tools, desc)) in suggested.iter().enumerate() {
            println!("  {}. 🚀 {}", i + 1, cat);
            println!("     目标工具: {}", tools);
            println!("     价值:     {}", desc);
            println!();
        }

        println!("💡 下一步行动:");
        println!("  1. 选择最感兴趣的方向");
        println!("  2. 使用 AI 生成工具: ./ai.sh -p qwen -m qwen-coder-plus-latest '用 Rust 实现一个简化版的 {}'", suggested[0].1.split(',').next().unwrap_or("tool"));
        println!("  3. 运行 evolve full 进行完整进化");
        println!();

        self.add_record("directions", "suggestions_generated", "10 evolution directions suggested");
    }

    /// 显示帮助
    fn print_help() {
        println!(r#"
╔════════════════════════════════════════════════╗
║          自我进化引擎 (Evolve)                   ║
║        AI-Powered Self-Evolution               ║
╚════════════════════════════════════════════════╝

用法: evolve [COMMAND]

命令:
  diagnose     诊断所有工具状态
  update       更新并重新编译所有工具
  expand       分析并建议功能扩展
  optimize     优化性能（编译 release 版本）
  learn        学习并保存用户偏好
  directions   显示项目进化方向和建议
  history      查看进化历史
  full         完整进化（诊断+更新+优化+扩展+学习）
  help         显示此帮助

示例:
  evolve diagnose      # 诊断工具状态
  evolve update        # 更新所有工具
  evolve directions    # 查看进化方向
  evolve full          # 完整进化
  evolve history       # 查看历史

环境变量:
  AI_PROVIDER          默认 AI 提供商
  AI_QWEN_MODEL        默认 Qwen 模型
"#);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 || args.iter().any(|a| a == "-h" || a == "--help" || a == "help") {
        EvolutionEngine::print_help();
        return;
    }

    let command = &args[1];
    let mut engine = EvolutionEngine::new();

    match command.as_str() {
        "diagnose" => engine.diagnose(),
        "update" => engine.update(),
        "expand" => engine.expand(),
        "optimize" => engine.optimize(),
        "learn" => engine.learn(),
        "directions" | "roadmap" => engine.suggest_directions(),
        "history" => engine.history(),
        "full" => engine.full_evolve(),
        _ => {
            eprintln!("未知命令: {}", command);
            EvolutionEngine::print_help();
        }
    }
}
