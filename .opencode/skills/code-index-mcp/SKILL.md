---
name: code-index-mcp
description: "Search and analyze the Moneta codebase with the active code-index MCP server. Use this first for repository understanding, planning, review, and indexed code exploration."
allowed-tools:
  - code-index:set_project_path
  - code-index:refresh_index
  - code-index:build_deep_index
  - code-index:get_settings_info
  - code-index:search_code_advanced
  - code-index:find_files
  - code-index:get_file_summary
  - code-index:get_file_watcher_status
  - code-index:configure_file_watcher
  - code-index:refresh_search_tools
disable-model-invocation: false
user-invocable: true
---

# Code Index MCP Skill

> **IMPORTANT: Use `code-index` first**. Before broad direct searching for comprehension, planning, review, or debugging tasks, use the active `code-index` MCP server to narrow the search space. Then verify important results against the live repo before editing.

Use this skill for codebase understanding, architectural research, feature planning, debugging, code review, and documentation work in Moneta.

## When to use

- When asked to explain how a feature or module works
- When planning or designing changes
- When reviewing code for bugs, regressions, or missing tests
- When researching unfamiliar parts of the repo
- Before implementing a feature to find likely entry points
- When debugging and tracing callers, symbols, or related files
- When writing or updating `.agent/` documentation

## Moneta MCP-first workflow

### Step 1: Confirm project state

This repo launches `code-index` with `--project-path /home/westbam/Development/moneta`, so the project path should already be set.

Use these tools first:
- `code-index:get_settings_info`
- `code-index:get_file_watcher_status`

If startup state looks wrong or stale:
- run `code-index:set_project_path` with `/home/westbam/Development/moneta`
- run `code-index:refresh_index`
- run `code-index:refresh_search_tools`

### Step 2: Use the right tool

| Task | Tool | Example |
|------|------|---------|
| Find files by path or extension | `code-index:find_files` | `pattern="moneta-ui/src/**/*.tsx"` |
| Search code text or literals | `code-index:search_code_advanced` | `pattern="MonetaTransactionBuilder"` |
| Search with regex | `code-index:search_code_advanced` | `pattern="class\\s+\\w+Service", regex=true, file_pattern="*.kt"` |
| Get file-level structure | `code-index:get_file_summary` | `file_path="moneta-server/src/.../UserController.kt"` |
| Build symbol metadata | `code-index:build_deep_index` | Run before symbol-heavy analysis |

### Step 3: Deep index when needed

The initial project setup builds a shallow index for fast discovery.

Run `code-index:build_deep_index` when you need:
- function or class summaries
- symbol-aware file analysis
- deeper structural review of Kotlin, TypeScript, Java, or Rust files

### Step 4: Verify against live source

`code-index` results are index-backed and can lag behind the current working tree.

Before editing or making strong claims about behavior:
- read the current file from disk
- confirm the indexed result still matches the live implementation
- if needed, run `code-index:refresh_index` or `code-index:build_deep_index`

## Watcher guidance

This Moneta setup enables watcher support immediately.

Use:
- `code-index:get_file_watcher_status` to confirm watcher health
- `code-index:configure_file_watcher` to adjust watcher settings if needed

If the watcher becomes unstable:
- report it explicitly
- fall back to `code-index:refresh_index`
- keep using direct file reads for final verification

## Operating rules

- Prefer `code-index` before broad grep/file sweeps for comprehension tasks
- Use the narrowest tool that answers the question
- Keep direct file reads as the source of truth before editing
- Run `build_deep_index` only when you need symbol-level analysis
- If watcher health degrades, continue with manual refresh instead of blocking work
- If `code-index` is unavailable, say so and fall back to direct repo inspection

## References

- `opencode.json` - active opencode MCP configuration for this repo
- `.agent/readme.md` - Moneta documentation index
- `https://github.com/johnhuang316/code-index-mcp` - upstream server documentation
