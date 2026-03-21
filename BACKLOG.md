# Synapse — Backlog

Legend: `[ ]` pending · `[~]` in progress · `[x]` done · `[!]` blocked

---

## Phase 1 — Core + TUI

Goal: MCP server รับ agent ได้, TUI ใช้งานได้, Claude Code + Gemini CLI ต่อได้

### Milestone 1-A: Workspace bootstrap

- [x] **TASK-001** — Cargo workspace setup — merged into develop
  - root `Cargo.toml` with `[workspace]` + `[workspace.dependencies]`
  - `rust-toolchain.toml` pinned to stable
  - crate stubs: `shared-types`, `mcp-server`, `grpc-server`, `tui`, `agent-claude`, `agent-gemini`
  - `cargo build --workspace` ผ่าน (empty crates)

- [x] **TASK-002** — Proto + codegen pipeline — merged into develop
  - เขียน `proto/synapse.proto` ตาม spec ใน CLAUDE.md
  - `build.rs` ใน `grpc-server` และ `tui` ที่รัน `tonic-build`
  - `cargo build -p grpc-server && cargo build -p tui` ผ่าน
  - **ทำก่อน TASK-003 เพราะ grpc-server และ tui depend on generated code**

### Milestone 1-B: Infrastructure

- [x] **TASK-003** — Storage setup — merged into develop
  - Redis connection pool (`redis-rs` + connection manager) ใน `shared-types`
  - SQLite schema + `sqlx` migrations (audit_log table)
  - integration test: write/read roundtrip
  - Depends on: TASK-001

- [x] **TASK-004** — NATS setup — PR #2 merged
  - `async-nats` client wrapper ใน `shared-types`
  - publish helper: `nats.publish(subject, payload)`
  - subscribe helper: `nats.subscribe(subject)` → `Stream<Event>`
  - unit test: publish → subscribe roundtrip
  - Depends on: TASK-001

### Milestone 1-C: MCP Server

- [x] **TASK-005** — MCP server skeleton — PR #3 merged
  - axum HTTP server บน :3000
  - rmcp tool routing
  - health check endpoint `GET /health`
  - graceful shutdown (SIGTERM)
  - Depends on: TASK-003, TASK-004

- [~] **TASK-006** — MCP tools: context — PR #5 (QA approved)
  - `read_context(key)` → Redis GET
  - `write_context(key, value)` → Redis SET
  - `search_memory(query)` → Redis key scan (prefix match, Phase 2 upgrade to Qdrant)
  - unit tests: roundtrip ทุก tool
  - Depends on: TASK-005

- [ ] **TASK-007** — MCP tools: tasks
  - `list_tasks(status?, assigned_to?)` → query Redis task store
  - `update_task(id, status, notes?)` → update + publish `synapse.task.status_changed`
  - Task state machine: `pending → in_progress → in_review → done | blocked`
  - unit tests: state transition ทุก path
  - Depends on: TASK-005

- [ ] **TASK-008** — MCP tools: GitHub + escalation
  - `read_github_pr(pr_number)` → GitHub REST API
  - `comment_on_pr(pr_number, body)` → GitHub REST API
  - `post_to_slack(channel, message)` → Slack webhook
  - env vars: `GITHUB_TOKEN`, `SLACK_WEBHOOK_URL`
  - Depends on: TASK-005

### Milestone 1-D: gRPC Server

- [ ] **TASK-009** — gRPC server skeleton
  - tonic server บน :3001
  - implement `SynapseUI` service stub (return empty/unimplemented)
  - graceful shutdown
  - Depends on: TASK-002, TASK-003, TASK-004

- [ ] **TASK-010** — gRPC: queries
  - implement `ListTasks` → query Redis
  - implement `GetTask` → query Redis
  - implement `ListAgents` → query Redis (agent registry state)
  - unit tests
  - Depends on: TASK-009

- [ ] **TASK-011** — gRPC: user actions
  - implement `ApproveCheckpoint` → publish `synapse.checkpoint.approved` → NATS
  - implement `PauseAgent` → update agent state → Redis
  - implement `ResumeAgent` → update agent state → Redis
  - unit tests
  - Depends on: TASK-009

- [ ] **TASK-012** — gRPC: SubscribeEvents stream
  - subscribe `synapse.>` จาก NATS
  - map NATS message → proto `Event` oneof
  - stream ไปหา TUI client
  - integration test: publish NATS event → verify stream receives it
  - Depends on: TASK-009, TASK-010, TASK-011

### Milestone 1-E: TUI

- [ ] **TASK-013** — TUI skeleton
  - ratatui + crossterm setup
  - tonic client เชื่อมต่อ gRPC :3001
  - main event loop: `tokio::select!` keyboard + gRPC stream
  - Depends on: TASK-002

- [ ] **TASK-014** — TUI: layout
  - sidebar: task list + agent list
  - main panel: agent status + progress bar
  - log view: scrollable log stream
  - footer: keybind hints
  - Depends on: TASK-013

- [ ] **TASK-015** — TUI: realtime events
  - รับ `SubscribeEvents` stream → update AppState
  - re-render เมื่อ state เปลี่ยน
  - Depends on: TASK-014, TASK-012

- [ ] **TASK-016** — TUI: user actions
  - `a` → `ApproveCheckpoint` RPC
  - `p` → `PauseAgent` RPC
  - `r` → `ResumeAgent` RPC
  - `↑↓` → navigate task list
  - `q` → quit gracefully
  - Depends on: TASK-015

### Milestone 1-F: Agent adapters (Phase 1)

- [~] **TASK-017** — shared-types: CodingAgent trait — PR #4 (QA approved)
  - `CodingAgent` trait: `id()`, `capabilities()`, `is_available()`, `execute()`
  - `AgentCapabilities` struct
  - `AgentRegistry`: `register()`, `select(task)` with fallback list
  - unit tests: registry select + fallback logic
  - Depends on: TASK-001

- [ ] **TASK-018** — agent-claude: Claude Code adapter
  - impl `CodingAgent` for Claude Code CLI
  - `is_available()` → ping MCP :3000 for registered agent
  - agent poll loop: `list_tasks` → `execute` → `update_task`
  - Depends on: TASK-017, TASK-007

- [ ] **TASK-019** — agent-gemini: Gemini CLI adapter
  - impl `CodingAgent` for Gemini CLI
  - same poll loop pattern as TASK-018
  - Depends on: TASK-017, TASK-007

### Milestone 1-G: Integration + CI

- [ ] **TASK-020** — End-to-end integration test (Docker)
  - `crates/mock-agent/` — Rust binary ที่ทำ agent loop จริงแต่ไม่เรียก LLM
    - poll `list_tasks()` → `update_task(in_progress)` → sleep 1s → `update_task(done)`
    - env: `MCP_URL`, `AGENT_ID`, `AGENT_ROLE`
  - `crates/tui/` เพิ่ม `E2E_MODE=true` — headless mode ที่ assert แทน render:
    - connect gRPC → subscribe events → assert ได้รับ `TaskStatusChanged` และ `CheckpointRequired`
    - call `ApproveCheckpoint` → assert ได้รับ `checkpoint.approved`
    - exit 0 ถ้าผ่าน, exit 1 ถ้า fail
  - `docker-compose.e2e.yml` — full stack: nats + redis + mcp-server + grpc-server + mock-agent + tui-runner
  - `Dockerfile.mcp-server`, `Dockerfile.grpc-server`, `Dockerfile.mock-agent`, `Dockerfile.tui-runner`
  - test scenario:
    1. สร้าง task ใน Redis
    2. mock-agent รับ task → update in_progress → update done
    3. tui-runner assert ได้รับ events ครบ ภายใน 30s
  - Depends on: TASK-012, TASK-016

- [ ] **TASK-021** — CI pipeline (infra — draft PR only)
  - GitHub Actions: `cargo test`, `cargo clippy`, `cargo fmt --check`
  - run on push to `main` and PRs
  - **open as draft, post to slack before merging**
  - Depends on: TASK-020

- [ ] **TASK-022** — Rustdoc + README
  - `///` doc comments ทุก public type และ function
  - README.md: quick start, architecture summary, synapse.toml example
  - `cargo doc --no-deps` ต้องผ่านโดยไม่มี warning
  - Depends on: TASK-020

---

## Phase 2 — Multi-agent + Memory

เริ่มหลังจาก Phase 1 done ทุก task

- [ ] **TASK-101** — agent-codex: Codex CLI adapter
- [ ] **TASK-102** — agent-opencode: OpenCode adapter
- [ ] **TASK-103** — synapse.toml routing config parser
  - อ่าน prefer list → `AgentRegistry` routing rules
- [ ] **TASK-104** — Qdrant integration
  - `search_memory` upgrade: embed query → Qdrant vector search
  - ผ่าน rig-core embedding pipeline
- [ ] **TASK-105** — MCP tools: spec management
  - `read_spec()` · `write_spec()` · `approve_spec()`
- [ ] **TASK-106** — Multi-agent orchestration
  - Orchestrator loop: assign tasks ตาม routing rules
  - parallel task execution สำหรับ independent tasks

---

## Decisions log

| Date | Decision |
|---|---|
| 2026-03-20 | Transport: MCP HTTP :3000 (agents), gRPC :3001 (TUI), NATS :4222 (internal) |
| 2026-03-20 | Agent dispatch: pull model — agents poll list_tasks() เอง |
| 2026-03-20 | Agent offline: skip → next in prefer list → error TUI if none |
| 2026-03-20 | MCP notifications: ไม่ใช้ — client support ไม่ดีพอ |
| 2026-03-20 | API-only agents: ตัดออก — CLI เท่านั้น |
| 2026-03-20 | Neo4j: ตัดออก — ไม่มี graph relationship |
| 2026-03-20 | Qdrant: Phase 2 เท่านั้น |
| 2026-03-20 | E2E test: Docker full stack — mock-agent แทน real LLM, tui-runner headless mode |