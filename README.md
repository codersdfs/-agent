# Omega Agent

**Multi-agent AI coding assistant** — orchestrates Plan, Build, and Code Review agents through a Rust backend with Mechanized Gate enforcement, entropy garbage collection, negative knowledge feedback, and structured table memory.

Built on the principles of [Harness Engineering](https://github.com/anomalyco/harness-engineering) (OpenAI's 2026 framework for agent-scalable codebases).

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri v2 (Rust)                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │   Plan    │  │   Build   │  │  Review   │               │
│  │  Agent    │  │   Agent   │  │   Agent   │               │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘               │
│       │              │              │                      │
│  ┌────┴──────────────┴──────────────┴────┐               │
│  │           Pipeline State Machine       │  max 3 retries │
│  │           (pipeline/)                  │  score ≥ 80    │
│  └────────────────┬───────────────────────┘               │
│                   │                                        │
│  ┌────────────────┴───────────────────────┐               │
│  │         Mechanized Gate (harness/)      │               │
│  │  structural · taste · golden · repeated │               │
│  └────────────────┬───────────────────────┘               │
│                   │                                        │
│  ┌────────────────┴───────────────────────┐               │
│  │            Negative Patterns            │               │
│  │   frequency ≥ 3 → auto-promote to rule │               │
│  └────────────────────────────────────────┘               │
│                                                             │
│  ┌────────┐ ┌─────────┐ ┌──────────┐ ┌──────┐ ┌───────┐  │
│  │Providers│ │ Omega   │ │ Hermes   │ │Entropy│ │ MCP   │  │
│  │(LLM)   │ │ Tables  │ │ Memory   │ │ GC   │ │ Skills│  │
│  └────────┘ └─────────┘ └──────────┘ └──────┘ └───────┘  │
└─────────────────────────────────────────────────────────┘
                    │ Tauri IPC
┌─────────────────────────────────────────────────────────┐
│              React + Tailwind v4 Frontend                 │
│  ChatPanel · AgentPanel · PlanView · ReviewPanel          │
│  TableBrowser · Terminal · Permissions · Settings         │
└─────────────────────────────────────────────────────────┘
```

## Workspace Crates

| Crate | Path | Purpose |
|-------|------|---------|
| `omega-agent` | `src-tauri/` | Main app: Tauri commands, pipeline state machine |
| `harness` | `crates/harness/` | Mechanized Gate: rules engine, pattern matching, scoring |
| `entropy` | `crates/entropy/` | Drift scanner, domain scorer, auto-GC PR generation |
| `omega-table` | `crates/omega-table/` | `.otable` format: three-level loading (index → meta → content), LRU cache |
| `providers` | `crates/providers/` | LLM abstraction: 14 providers via unified `LlmProvider` trait |
| `memory` | `crates/memory/` | Hermes memory: session/project/user layers, SQLite + FTS5 |
| `mcp` | `crates/mcp/` | MCP client: JSON-RPC transport, skills registry |

## The Three Agents

### Plan Agent
- **Read-only** (no write/edit/bash tools)
- Reads task, produces structured plan in `.otable` format
- Uses Claude Sonnet (or configured model)

### Build Agent
- **Write access** (asks permission via frontend dialog)
- Executes plan via filesystem/bash/grep/glob commands
- Uses Claude Sonnet (or configured model)

### Code Review Agent
- **Read-only** (strongest critique)
- Reviews output against golden rules, structural/taste patterns
- Uses Claude Opus (or configured strongest model)
- Every violation includes executable tool call in error message

## Pipeline

```
Plan ──→ Build ──→ Review ──→ Gate ──→ Score ≥ 80? ──→ Done
                  ↑                            │
                  └──── max 3 retries ──────────┘
```

- **Scoring**: 100 base, -15 structural, -10 taste, -20 golden, -25 repeated
- **Pass threshold**: ≥ 80
- **Context cache**: cached until `.omega/` files change
- **Delta retry**: retries pass only diff, not full replan

## Core Concepts

### Harness Engineering (by OpenAI / Ryan Lopopolo)

1. **Repo as System of Record** — everything outside the repo is invisible to agents. Slack chats, Google Docs, tribal knowledge → must be versioned artifacts in the repo.
2. **Map, Not Manual** — `AGENTS.md` ≈ directory page (~100 lines), not encyclopedia. Progressive disclosure.
3. **Mechanical Enforcement** — docs rot, lint rules don't. Custom linter + CI = invariant guardians. Error messages embed fix instructions for agent self-correction.
4. **Agent Readability** — boring tech (stable APIs, good training coverage). Sometimes reimplement a subset rather than wrap an opaque upstream. App starts per `git worktree`.
5. **Entropy & GC** — agents replicate existing patterns (including bad ones). Golden rules encoded in repo. Scheduled background tasks scan drift.
6. **Humans Steer, Agents Execute** — scarcest resource is human attention. Problem → missing context/tool/constraint, not "try harder".

### Guides × Sensors Matrix (Fowler / Böckeler, 2026)

| | Computational (CPU) | Reasoning (LLM) |
|---|---|---|
| **Guides / Feedforward** | bootstrap scripts, OpenRewrite, LSP | AGENTS.md, Skills, architecture.md |
| **Sensors / Feedback** | linter, ArchUnit, type checks, coverage | AI code review, LLM-as-judge |

### 6D Complexity Framework (Harness Engineering)

| Dimension | Focus |
|-----------|-------|
| D1: Structural | Architecture layering, dependency direction |
| D2: Taste | Code conventions, naming, file size limits |
| D3: Golden | Non-negotiable quality invariants |
| D4: Repeated | Frequency ≥ 3 → auto-promote to linter rule |
| D5: Context | Context window optimization, compaction |
| D6: Drift | Entropy scan, GC PR generation |

### Ralph Wiggum Loop (Control Theory)

A negative-knowledge feedback loop inspired by control theory:
- Every error is logged
- At frequency ≥ 3, error is promoted to a linter rule
- Reduced feedback latency → lower entropy

## LLM Providers

14 providers through a unified `LlmProvider` trait:

| Provider | Transport |
|----------|-----------|
| Anthropic | Native SDK |
| OpenAI | Native SDK |
| Google (Gemini) | Native SDK |
| Mistral | Native SDK |
| xAI (Grok) | OpenAI-compatible |
| Cerebras | OpenAI-compatible |
| Azure OpenAI | OpenAI-compatible |
| AWS Bedrock | OpenAI-compatible |
| Hugging Face | OpenAI-compatible |
| Groq | OpenAI-compatible |
| Kimi for Coding | OpenAI-compatible |
| MiniMax | OpenAI-compatible |
| OpenRouter | OpenAI-compatible |
| Local / Custom | OpenAI-compatible endpoint |

8 providers share OpenAI-compatible transport (~1050 lines total).

## Omega Tables (`.otable`)

Three-level progressive loading:

```
.otable file
├── Level 1: Index  (schema, columns, row count, version)
├── Level 2: Meta   (description, tags, source, stats)
└── Level 3: Content (actual rows, paginated)
```

- LRU cache with TTL eviction
- FTS5 full-text search (via SQLite)
- Embedding-based semantic search (via fastembed)

## Hermes Memory

Three-layer memory system:

| Layer | Scope | Persistence |
|-------|-------|-------------|
| Session | Current session | In-memory, cleared on exit |
| Project | Current project | SQLite per-project database |
| User | Cross-project | SQLite user-wide database |

- FTS5 for full-text search
- Embedding vectors for semantic similarity
- Automatic context injection into agent prompts

## Entropy GC

- Runs daily (scheduled background task)
- Scans domains for structural drift
- Scores each domain by drift severity and priority
- Auto-generates PRs to remediate high-entropy areas

## Development

### Prerequisites

- Rust 1.77+ (nightly recommended)
- Node.js 20+
- Windows (primary target) / macOS / Linux

### Setup

```bash
# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

### Project Structure

```
omega-agent/
├── src/                          # React frontend
│   ├── components/               # UI components
│   ├── stores/                   # Zustand stores
│   ├── types/                    # TypeScript types
│   ├── hooks/                    # Custom hooks
│   └── lib/                      # Tauri IPC wrappers
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── commands/             # Tauri IPC command handlers
│   │   │   ├── chat.rs           # Chat/send message
│   │   │   ├── tools.rs          # Tool execution
│   │   │   ├── gate.rs           # Gate checks
│   │   │   ├── tables.rs         # Omega table queries
│   │   │   ├── memory.rs         # Memory store/search
│   │   │   └── mcp.rs            # MCP invoke/skills
│   │   ├── pipeline/             # Agent pipeline
│   │   │   ├── state.rs          # Pipeline state machine
│   │   │   ├── plan.rs           # Plan agent
│   │   │   ├── build.rs          # Build agent
│   │   │   └── review.rs         # Review agent
│   │   ├── main.rs               # Binary entry point
│   │   └── lib.rs                # App bootstrap
│   └── crates/
│       ├── harness/              # Mechanized Gate
│       ├── entropy/              # Entropy GC
│       ├── omega-table/          # Omega Tables
│       ├── providers/            # LLM providers
│       ├── memory/               # Hermes memory
│       └── mcp/                  # MCP client
├── package.json
├── vite.config.ts
└── tsconfig.json
```

### Commands

```bash
npm run dev          # Frontend dev server (Vite)
npm run build        # TypeScript + Vite build
npm run tauri dev    # Full Tauri dev mode (Rust + frontend)
npm run tauri build  # Production build
```

## Key Decisions

| Decision | Rationale |
|----------|-----------|
| Tauri v2 over Electron | Native Rust performance, smaller bundle, secure IPC |
| Rust over Python backend | Same language as harness, better perf for filesystem ops |
| All 3 agents LLM-reasoning | User chose flexibility over speed |
| Pipeline in Rust, not TypeScript | Harness enforcement must be in the same process |
| MCP via Rust JSON-RPC | MCP SDK is TypeScript-native, Rust implementation needed |
| Local embeddings via fastembed | No external API call, fully offline |

## License

MIT
