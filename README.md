# DockYard 🐳

A lightweight, fast, Rust-powered alternative to Docker Compose CLI.

DockYard lets you manage multi-container applications with a tiny footprint.  
It parses your compose file, respects dependencies + health checks, starts services in the correct order, streams nicely colored logs, handles Ctrl+C gracefully, and gives detailed container status — all without the full Docker Compose overhead.

Great for local development when you want something simple, reliable, and snappy.

## ✨ Main Features

- Parses `docker-compose.yml` and builds correct startup order (topological sort)
- Respects `depends_on` + full `healthcheck` blocks — waits until dependent services are actually healthy
- Supports `build: .`, `build: context: …`, local images, and Docker Hub pulls
- Creates one default network named after your current directory
- Colored, real-time log streaming (each service in its own color)
- Graceful Ctrl+C handling — stops everything cleanly during startup
- Detailed container status view (health, ports, IPs, restart policy, OOM, etc.)
- Proper error handling — no panics, no `.unwrap()`
- One-line install script for macOS (more platforms coming)

## 🚀 Installation (macOS – amd64 right now)

Run this once:

```bash
curl -sSL https://raw.githubusercontent.com/Rohaan-Taneja/rust_dockeCompose_cli_app/main/install.sh | bash
