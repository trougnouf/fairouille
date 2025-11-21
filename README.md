# Rustache

A simple, elegant, and lightweight CalDAV task manager for the terminal, written in Rust.

## Features
- **Vim-like Navigation:** `j`/`k` to move, `d` to delete, `Space` to toggle.
- **Smart Input:** Add tasks naturally: `Buy Milk !1 @tomorrow`.
- **Syncs Everywhere:** Fully compatible with CalDAV servers (Currently tested with Radicale).
- **Offline-First UI:** Optimistic UI updates for instant feedback.
- **Multiple Calendars:** Sidebar support to switch between task lists.

## Installation

1.  **Build from source:**
    ```bash
    cargo install --path .
    ```

2.  **Run:**
    ```bash
    rustache
    ```

## Configuration

Create a config file at `~/.config/rustache/config.toml` (Linux) or `~/Library/Application Support/com.rustache.rustache/config.toml` (Mac):

```toml
url = "https://localhost:5232/trougnouf/"
username = "myuser"
password = "mypassword"
default_calendar = "todo" # Optional: Auto-selects this list on startup
```

## Keybindings

| Key | Action |
| :--- | :--- |
| `a` | **Add Task** (Type name, press Enter) |
| `Space` | **Toggle** Completion |
| `d` | **Delete** Task |
| `+` / `-` | Increase / Decrease **Priority** |
| `Tab` | Switch focus (Tasks <-> Calendars) |
| `Enter` | Select Calendar (in Sidebar) |
| `j` / `k` | Move Down / Up |
| `q` | Quit |

## Input Syntax
When adding a task (`a`), you can use shortcuts:
- `!1` sets High Priority (1-9)
- `@tomorrow`, `@today`, or `@2025-12-31` sets the Due Date.

## License
GPL3
