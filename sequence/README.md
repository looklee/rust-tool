# SEQUENCE OS¹

> **An AI Operating System with persistent memory, identity, and execution.**

SEQUENCE OS¹ is not another AI chat application. It is a system layer built on top of foundation models that provides what LLMs lack: **continuous existence**.

## What It Is

Foundation models are powerful but stateless — brilliant in the moment, but with no continuous self, no long-term memory, no ability to persistently pursue goals. SEQUENCE OS¹ sits on top of these models and provides:

- **🧠 Persistent Memory** — Remembers across sessions, not just within a conversation
- **🆔 Identity** — A continuous sense of self that evolves over time
- **⚙️ Runtime Engine** — Task queue with scheduling, tracking, and completion
- **🌍 World Interface** — Filesystem, process execution, file search
- **🛡️ Governance** — Safety rules, permissions, and audit logging
- **🔌 Model Agnostic** — Works with OpenAI, Anthropic, Gemini, Ollama, Qwen, DeepSeek

## Quick Start

```bash
# Build
cd /root/sequence && cargo build

# Start the REPL
./target/debug/sequence

# Or check status without starting REPL
./target/debug/sequence status
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      SEQUENCE OS¹                           │
├──────────────┬──────────────┬──────────────┬────────────────┤
│   Memory     │   Identity   │   Runtime    │   Governance   │
│   Layer      │   System     │   Engine     │   Layer        │
├──────────────┴──────────────┴──────────────┴────────────────┤
│                    World Interface                          │
├─────────────────────────────────────────────────────────────┤
│                  Model Abstraction Layer                    │
└─────────────────────────────────────────────────────────────┘
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for full architecture details.

## REPL Commands

### System
| Command | Description |
|---------|-------------|
| `/status` | Show system status |
| `/help` | Show help |
| `/quit` | Save and exit |

### Memory
| Command | Description |
|---------|-------------|
| `/memory list` | List recent memories |
| `/memory search <query>` | Search memories |
| `/memory recent [n]` | Show n recent memories |
| `/memory add <text>` | Add a memory |
| `/memory stats` | Memory statistics |

### Tasks
| Command | Description |
|---------|-------------|
| `/tasks list` | List all tasks |
| `/tasks add <desc>` | Add a new task |
| `/tasks complete <id>` | Mark task complete |
| `/tasks remove <id>` | Remove a task |

### Identity
| Command | Description |
|---------|-------------|
| `/identity` | Show identity |
| `/identity values` | Show values |

### Governance
| Command | Description |
|---------|-------------|
| `/governance` | Show governance status |
| `/governance rules` | List safety rules |
| `/governance perms` | List permissions |

### World
| Command | Description |
|---------|-------------|
| `/world ls [path]` | List directory |
| `/world pwd` | Show current directory |
| `/world read <file>` | Read a file |

### Actions
| Command | Description |
|---------|-------------|
| `/run <cmd>` | Execute shell command |
| `/read <file>` | Read a file |
| `/write <file> <content>` | Write to a file |
| `/search <pattern> [path]` | Search files |

### Evolution
| Command | Description |
|---------|-------------|
| `/evolve` | Trigger self-evolution |
| `/snapshot` | Save system state |

## CLI Commands

```bash
# Start interactive mode
./target/debug/sequence start

# Quick status check
./target/debug/sequence status

# View memory stats
./target/debug/sequence memory

# View task queue
./target/debug/sequence tasks

# View identity
./target/debug/sequence identity

# Create state snapshot
./target/debug/sequence snapshot

# Trigger self-evolution
./target/debug/sequence evolve
```

## Configuration

### Environment Variables

```bash
# Choose AI provider (ollama, openai, anthropic, gemini, deepseek, qwen)
export SEQUENCE_MODEL_PROVIDER=ollama

# Choose specific model
export SEQUENCE_MODEL=qwen2.5-coder:7b

# API keys (for cloud providers)
export OPENAI_API_KEY="sk-..."
export DASHSCOPE_API_KEY="sk-..."
```

### Data Directory

All state is stored in `~/.sequence/`:

```
~/.sequence/
├── identity.json          # Core identity
├── memory/                # Memory storage
│   ├── episodic_index.json
│   ├── semantic_index.json
│   ├── procedural_index.json
│   └── episodic/          # Individual memory files
├── tasks/                 # Task storage
│   └── queue.json
├── governance/            # Safety rules
│   └── rules.json
├── relationships/         # User profiles
└── evolution/             # Evolution history
    └── snapshots/
```

## Core Modules

### Memory Layer (`memory.rs`)

Three tiers of memory:
- **Episodic** — Past interactions and events (like a diary)
- **Semantic** — Learned knowledge and facts (like a knowledge base)
- **Procedural** — Skills and workflows (like habits)

Plus **Working Memory** for current session context.

### Identity System (`identity.rs`)

Persistent AI identity with:
- Name, role, purpose
- Values and personality traits
- Relationship tracking with users

### Runtime Engine (`runtime.rs`)

Task management with:
- Persistent task queue
- Task statuses (Pending, Running, Completed, Failed, Cancelled)
- Priority levels

### World Interface (`world.rs`)

External world interaction:
- Filesystem browsing and reading
- Shell command execution
- File search
- Path-based access control

### Governance Layer (`governance.rs`)

Safety and permissions:
- Configurable safety rules
- Permission sets (read, write, execute, network)
- Audit logging

### Model Abstraction (`models.rs`)

Model-agnostic AI interface:
- Ollama (local models)
- OpenAI-compatible APIs (OpenAI, DeepSeek, Qwen)
- Anthropic Claude
- Google Gemini

## Philosophy

> The value of SEQUENCE OS¹ is not in making a "smarter AI" but in building a **system layer** that transforms instantaneous model intelligence into **continuous, memorable, evolving existence**.

Models will keep getting smarter. The scarcity is not intelligence — it's **continuity**. SEQUENCE OS¹ provides that continuity.

## Development

```bash
# Build
cargo build

# Build release
cargo build --release

# Run tests
cargo test
```

## License

Private project.

---

*SEQUENCE OS¹ — Where models become minds, and moments become memory.*
