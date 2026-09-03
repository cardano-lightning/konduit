---
name: rust-deps-visualizer
description:
  "Visualize Rust project dependencies as ASCII art. Triggers on: /deps-viz,
  dependency graph, show dependencies, visualize deps, 依赖图, 依赖可视化,
  显示依赖"
argument-hint: "[--depth N] [--features]"
allowed-tools: ["Bash", "Read", "Glob"]
---

# Rust Dependencies Visualizer

Generate ASCII art visualizations of your Rust project's dependency tree.

## Usage

```
/rust-deps-visualizer [--depth N] [--features]
```

**Options:**

- `--depth N`: Limit tree depth (default: 3)
- `--features`: Show feature flags

## Output Format

### Simple Tree (Default)

```
my-project v0.1.0
├── tokio v1.49.0
│   ├── pin-project-lite v0.2.x
│   └── bytes v1.x
├── serde v1.0.x
│   └── serde_derive v1.0.x
└── anyhow v1.x
```

### Feature-Aware Tree

```
my-project v0.1.0
├── tokio v1.49.0 [rt, rt-multi-thread, macros, fs, io-util]
│   ├── pin-project-lite v0.2.x
│   └── bytes v1.x
├── serde v1.0.x [derive]
│   └── serde_derive v1.0.x (proc-macro)
└── anyhow v1.x [std]
```

## Implementation

**Step 1:** Parse Cargo.toml for direct dependencies

```bash
cargo metadata --format-version=1 --no-deps 2>/dev/null
```

**Step 2:** Get full dependency tree

```bash
cargo tree --depth=${DEPTH:-3} ${FEATURES:+--features} 2>/dev/null
```

**Step 3:** Format as ASCII art tree

Use these box-drawing characters:

- `├──` for middle items
- `└──` for last items
- `│   ` for continuation lines

## Visual Enhancements

### Dependency Categories

```
my-project v0.1.0
│
├─[Runtime]─────────────────────
│ ├── tokio v1.49.0
│ └── async-trait v0.1.x
│
├─[Serialization]───────────────
│ ├── serde v1.0.x
│ └── serde_json v1.x
│
└─[Development]─────────────────
  ├── criterion v0.5.x
  └── proptest v1.x
```

### Size Visualization (Optional)

```
my-project v0.1.0
├── tokio v1.49.0        ████████████ 2.1 MB
├── serde v1.0.x         ███████ 1.2 MB
├── regex v1.x           █████ 890 KB
└── anyhow v1.x          ██ 120 KB
                         ─────────────────
                         Total: 4.3 MB
```

## Workflow

1. Check for Cargo.toml in current directory
2. Run `cargo tree` with specified options
3. Parse output and generate ASCII visualization
4. Optionally categorize by purpose (runtime, dev, build)

## Related Skills

| When                   | See           |
| ---------------------- | ------------- |
| Crate selection advice | m11-ecosystem |
| Workspace management   | m11-ecosystem |
| Feature flag decisions | m11-ecosystem |
