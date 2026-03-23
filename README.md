# DockYard 🐳

**Your lightweight, Rust-powered Docker Compose CLI**

I built DockYard because I wanted a super-simple, fast, and reliable way to spin up multi-container apps without the full weight of Docker Compose. It’s written in Rust, handles dependencies smartly, respects health checks, streams colored logs, and even cleans up gracefully when you hit Ctrl+C. 

Perfect for local development, quick testing, or just when you want something that “just works” without the bloat.

---

## ✨ Features

- **Smart dependency handling** — parses your `docker-compose.yml`, builds a dependency graph using topological sort, and starts services in the exact right order.
- **Full build & image support** — works with `build: .`, `build.context`, local Dockerfiles, or pulling from Docker Hub.
- **Port mapping** — hotspot:containerport binding just like the real thing.
- **Health checks that actually wait** — supports `depends_on` + full healthcheck config (interval, timeout, retries, start_period). It polls until the service is healthy before starting dependents.
- **One default network** — automatically creates a network named after your current directory and connects everything to it. All containers live under the same “table”.
- **Beautiful colored logs** — each service gets its own color in the terminal. Streaming is async and real-time.
- **Detailed container status** — see everything: health, ports, network IPs, restart policy, OOM status, you name it.
- **Graceful shutdown** — hit Ctrl+C while starting up? It stops everything cleanly. No zombie containers.
- **Stop / Down / Restart** — simple commands to stop or tear down an entire network.
- **Zero .unwrap() drama** — proper error types and clean messages everywhere.
- **Easy install** — one-line curl script that puts the binary in your PATH.

---

## Installation

For macOS users, there's a pre-built binary you can pull straight from the repo:

```bash
curl -sSL https://raw.githubusercontent.com/Rohaan-Taneja/rust_dockeCompose_cli_app/main/install.sh | bash
```

The install script detects your OS and architecture, downloads the right binary, and saves it as `dockyard` in your PATH.

> **Note:** Currently only macOS (amd64) is supported. Builds for Linux and other architectures are on the way.

<img width="1521" height="251" alt="Screenshot 2026-03-23 at 7 48 58 PM" src="https://github.com/user-attachments/assets/929dde92-fc32-461d-baec-9590eb544c2b" />


---

## Commands

All commands follow this structure:

```
dockyard DockYard <Command> [args]
```

Here's a quick overview:

| Command | What it does |
|---|---|
| `Up <yaml_path>` | Start (or restart) all services from the compose file |
| `Down <network_id>` | Stop and remove all containers in the network |
| `Stop <network_id>` | Stop all running containers in the network |
| `Logs <container_id>` | Stream all logs from a specific container |
| `ContainerStatus <container_id>` | Show full status and details of a container |

---

## Usage

### Starting services

```bash
dockyard DockYard Up ./docker-compose.yml
```

This will:
- Validate the file path and parse the compose YAML
- Build the dependency graph and sort services in the right order
- Pull or build images as needed
- Create a shared Docker network (named after the current directory)
- Start each container, waiting for health checks where specified

Running the same command again will **restart** all the containers.


<img width="1703" height="956" alt="Screenshot 2026-03-23 at 7 49 49 PM" src="https://github.com/user-attachments/assets/02136c9c-ca38-4d01-a193-4e814d0f0598" />


<img width="1700" height="945" alt="Screenshot 2026-03-23 at 7 51 42 PM" src="https://github.com/user-attachments/assets/1c2a8689-14c0-4cbc-ba62-9ec8d13900c1" />


---

### Stopping containers

```bash
dockyard DockYard Stop <network_id>
```

Stops all running containers that belong to the network. The network ID is the same as the label assigned during `Up` — by default, it's the name of your current directory.

---

### Bringing everything down

```bash
dockyard DockYard Down <network_id>
```

Stops and removes all containers in the network.

---

### Viewing logs

```bash
dockyard DockYard Logs <container_id>
```

Streams all logs for a container from start to finish. Each service gets its own colour in the terminal so you can tell them apart at a glance.

---

### Checking container status

```bash
dockyard DockYard ContainerStatus <container_id>
```

Prints a detailed breakdown of the container, including:

```
Container Status
================

ID            : abc123...
Name          : my-service
Image         : my-image:latest
Created       : 2025-01-01T00:00:00Z
Platform      : linux/amd64

State
-----
Status        : Running
Health        : Healthy
PID           : 1234
Started At    : 2025-01-01T00:00:01Z
OOM Killed    : false
Restarting    : false
Exit Code     : 0

Process
-------
Entrypoint    : /bin/sh
Command       : -c start.sh

Network
-------
Network       : my-network
IP Address    : 172.18.0.2
Gateway       : 172.18.0.1

Ports
-----
8080/tcp -> 0.0.0.0:8080

Restart Policy
--------------
Policy        : no
Restart Count : 0
```

---

## What's supported in the compose file

### Images

```yaml
# Pull from Docker Hub or use a local image
image: postgres:15

# Build from current directory
build: .

# Build from a specific path
build:
  context: ./path/to/service
```

### Port bindings

```yaml
ports:
  - "8080:8080"   # host:container
```

### Dependencies and health checks

```yaml
depends_on:
  - db

# Or with health check conditions
depends_on:
  db:
    condition: service_healthy

healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8080/api/health/check"]
  interval: 5s
  timeout: 5s
  retries: 3
  start_period: 15s
```

DockYard will wait for a service to be fully healthy before starting anything that depends on it.

---

## How it works under the hood

### Dependency resolution

When you run `Up`, DockYard parses all the services in your compose file and builds a directed acyclic graph based on `depends_on` declarations. It then runs a topological sort to figure out the correct startup order. Services with no dependencies start first, and things fan out from there.

### Networking

Every `Up` command creates a Docker network named after your current working directory. All containers are connected to this network, so they can talk to each other by service name. The network ID doubles as the label used by `Stop` and `Down`.

### Health checks

If a service has a health check defined, DockYard will poll the container's health status after it starts and wait until it reports `healthy` before moving on to dependent services. You can configure the interval, timeout, retries, and start period — same as standard Docker health checks.

### Coloured logs

When streaming logs from multiple services, each one is printed with a distinct colour so it's easy to follow what's happening. The label includes the service name so you always know where output is coming from.

### Ctrl+C handling

If you hit `Ctrl+C` while containers are starting up, DockYard will catch the signal and stop all containers cleanly before exiting. Nothing gets left running in a half-started state.

### Error handling

The whole thing uses proper typed errors throughout — no `.unwrap()` calls scattered around. If something goes wrong, you get a clear message about what failed and where.

---

## Project structure highlights

- **Topological sort** for dependency ordering
- **Async log streaming** with per-service colour labels
- **Health check polling** with configurable retry logic
- **Graceful Ctrl+C shutdown** that stops all in-progress containers
- **Structured error types** with no panics
- **Release binary** with an install script for easy distribution

---
