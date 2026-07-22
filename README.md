# Lytt

Lytt is a small project-aware localhost port inspector.

It answers the boring question that keeps interrupting dev work:

> What is using this port, and which project did it come from?

`lytt` means "listen" in Norwegian. The tool lists listening TCP ports, maps them
back to processes, guesses the owning project from the process working directory,
and gives you useful actions like opening or stopping the service.

## Status

Early Linux-first scaffold. The current implementation reads `/proc`, so macOS
and Windows support are not in place yet.

## Usage

```bash
lytt
lytt 5173
lytt --json
lytt open 5173
lytt kill 3000
lytt kill 3000 --force
lytt watch
```

Example output:

```text
PORT    PROTO   PID      COMMAND            PROJECT      URL
5173    tcp     18422    vite               skald        http://127.0.0.1:5173
3000    tcp     19001    next               fattern      http://localhost:3000
5432    tcp     882      postgres           -            http://127.0.0.1:5432
```

## What It Does

- lists listening TCP ports
- maps socket inodes to owning PIDs
- reads process command and working directory
- detects likely project roots from `.git`, `package.json`, `Cargo.toml`,
  `go.mod`, or `pyproject.toml`
- opens a port in the browser
- stops the process that owns a port
- prints JSON for scripting

## Install

For now, build from source:

```bash
cargo install --git https://github.com/vardirhq/lytt
```

When the crate is published:

```bash
cargo install lytt
```

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- 5173
```

## Philosophy

Lytt should stay small, local, and obvious. It should not become a process
manager, Docker dashboard, or Electron app. The useful bit is the sentence:

> Port 5173 belongs to Vite in `~/code/skald`.

Everything else should serve that.
