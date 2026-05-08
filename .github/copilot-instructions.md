# AI Playmate — GitHub Copilot Repository Instructions
#
# These instructions apply to ALL Copilot interactions in this repository.
# They define the project conventions, architecture, and how the AI dev team
# should behave when implementing features, reviewing PRs, and fixing bugs.

## Project Overview

AI Playmate is a cross-platform (desktop + mobile) AI companion application with
**associative memory** — it remembers all conversations and makes semantic connections
across them, just like human memory.

**Tech stack:**
- Backend: Rust (Cargo workspace)
- UI: Tauri v2 + React (TypeScript)
- Vector storage: Qdrant (semantic similarity)
- Graph storage: SurrealDB (entity relations, sessions)
- Local embeddings: fastembed-rs (CPU inference, no API key needed)
- LLM: configurable via env vars (OpenAI / Anthropic / Gemini / Ollama)
- AI framework: rig-core (unified LLM provider interface)

## Repository Structure

```
AI-Playmate/
├── crates/
│   ├── ai-core/      # LLM provider config, embedding trait, ChatSession
│   ├── memory/       # Three-layer memory system (core/episodic/semantic + entity graph)
│   └── storage/      # Qdrant + SurrealDB adapters
├── apps/
│   └── desktop/      # Tauri v2 app (serves desktop + mobile via same codebase)
│       ├── src/      # React frontend
│       └── src-tauri/ # Rust backend commands
├── .github/
│   ├── workflows/    # CI/CD automation
│   └── prompts/      # AI agent role definitions
└── agents/           # Multi-agent development team configs
```

## Coding Conventions

### Rust
- Edition 2021, resolver = "2"
- Use `anyhow` for application errors, `thiserror` for library errors
- All async code uses `tokio`; prefer `async fn` over manual `Future` impls
- Use `tracing::{info, debug, error, instrument}` for logging — no `println!`
- Public APIs must have doc comments explaining purpose and invariants
- Prefer `Arc<T>` for shared state; avoid `Rc` and raw `Mutex<T>` unless necessary
- Run `cargo clippy -- -D warnings` before every commit
- Run `cargo fmt` before every commit

### TypeScript / React
- Strict TypeScript — no `any` types
- React functional components only, hooks for state
- Inline styles via `CSSProperties` objects (no CSS modules in initial phase)
- Tauri IPC calls go through `invoke()` from `@tauri-apps/api/core`
- All Tauri command names use `snake_case` to match Rust functions

### Git
- Branch naming: `feat/<description>`, `fix/<description>`, `chore/<description>`
- Commit messages follow Conventional Commits: `feat:`, `fix:`, `docs:`, `test:`, `chore:`
- Every PR must pass CI (fmt + clippy + test + typecheck) before merge

## Memory System Architecture

The three-layer memory model:

1. **Core Memory** (`crates/memory/src/core_memory.rs`) — always in LLM context,
   stores user profile + key facts. Limited to ~2000 tokens.

2. **Episodic Memory** (`crates/memory/src/episodic.rs`) — raw conversation turns
   in SurrealDB. Sliding window of 50 messages injected verbatim.

3. **Semantic Memory** (`crates/memory/src/semantic.rs`) — distilled compressed
   memories stored as vectors in Qdrant. Retrieved via cosine similarity.

4. **Entity Graph** (`crates/memory/src/entity.rs`) — named entities + relations
   in SurrealDB graph mode. Multi-hop traversal for associative retrieval.

**Background Consolidator** runs every 5 minutes, distils new messages into
semantic memories, and extracts entities to update the graph.

## AI Development Team Roles

When acting as part of the AI dev team, follow the appropriate role prompt:
- **PM Agent**: `.github/prompts/pm-agent.prompt.md`
- **Architect Agent**: `.github/prompts/architect-agent.prompt.md`
- **Developer Agent**: `.github/prompts/developer-agent.prompt.md`
- **Reviewer Agent**: `.github/prompts/reviewer-agent.prompt.md`
- **Tester Agent**: `.github/prompts/tester-agent.prompt.md`
- **DevOps Agent**: `.github/prompts/devops-agent.prompt.md`

## Security Guidelines

- NEVER hard-code API keys, passwords, or secrets in source code
- All credentials are loaded from environment variables (see `.env.example`)
- User data stays local by default (Local-first architecture)
- Follow OWASP Top 10 guidelines for any network-facing code
- Sanitise all user inputs before passing to LLM prompts (prevent prompt injection)
- Use `tauri`'s capability system to restrict what the frontend can access

## Testing Requirements

- Every new Rust function should have at least one unit test
- Integration tests for storage adapters go in `crates/*/tests/`
- Use `#[tokio::test]` for async test functions
- Mock external services (Qdrant, LLM APIs) in unit tests
