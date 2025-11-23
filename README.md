# Cfait

**Cfait** is a simple, elegant, and lightweight CalDAV task manager, written in Rust.

It features both a lightning-fast **TUI (Terminal UI)** and a modern **GUI (Graphical UI)** for desktop integration.

![Cfait GUI Screenshot](https://commons.wikimedia.org/wiki/Special:FilePath/Cfait_task_manager_v1.1.5_screenshot_(GUI).png)
*The Graphical Interface*

![Cfait TUI Screenshot](https://commons.wikimedia.org/wiki/Special:FilePath/Cfait_task_manager_v1.1.5_screenshot_(TUI).png)
*The Terminal Interface*

## Features

*   **Dual Interface:** Run it in your terminal (`cfait`) or as a windowed app (`cfait-gui`).
*   **Smart Input:** Add tasks naturally: `Buy cat food !1 @tomorrow` sets High Priority and Due Date automatically.
*   **Syncs Everywhere:** Fully compatible with standard CalDAV servers (Radicale, Nextcloud, iCloud, etc.).
*   **Hierarchy Support:** Create sub-tasks and organize nested lists easily.
*   **Multiple Calendars:** Seamlessly switch between "Work", "Personal", and other lists.
*   **Offline-First:** Optimistic UI updates mean you never wait for the server.

## Installation

### 1. Build from Source
Ensure you have Rust installed.

```bash
# Install the TUI (Default)
cargo install --path .

# Install the GUI (Optional)
cargo install --path . --bin gui --no-default-features --features gui
```

### 2. Run
```bash
# Run TUI
cfait

# Run GUI
cfait-gui
```

## Configuration

Create a config file at:
*   **Linux:** `~/.config/cfait/config.toml`
*   **Mac:** `~/Library/Application Support/com.cfait.cfait/config.toml`

```toml
url = "https://caldav.example.com/remote.php/dav/calendars/user/"
username = "myuser"
password = "mypassword"
default_calendar = "Personal" # Optional: Auto-selects this list on startup
```

## TUI Keybindings

| Context | Key | Action |
| :--- | :--- | :--- |
| **Global** | `Tab` | Switch focus (Tasks ↔ Calendars) |
| | `q` | Quit |
| **Task List** | `j` / `k` | Move Down / Up |
| | `Space` | **Toggle** Completion |
| | `a` | **Add** Task (Type name, press Enter) |
| | `e` | **Edit** Title (Shift+E for Description) |
| | `d` | **Delete** Task |
| | `/` | **Search** / Filter Tasks |
| | `+` / `-` | Increase / Decrease **Priority** |
| | `>` / `<` | **Indent** / **Outdent** (Create Sub-tasks) |
| **Sidebar** | `Enter` | Select Calendar |

## Input Syntax
When adding (`a`) or editing (`e`) a task, you can use shortcuts directly in the text:

*   `!1` to `!9`: Sets Priority (1 is High, 9 is Low).
*   `@tomorrow`, `@today`, `@next week`, `@next month`, `@next year`: Sets Due Date relative to now.
*   `@2025-12-31`: Sets specific Due Date (YYYY-MM-DD).
*   `@daily`, `@weekly`, `@monthly`, `@yearly`, `@every 4 days`, `@every 2 weeks`, etc: Sets Recurrence.

## TODO
* [ ] Browse by categories (multi-calendar)
* [ ] TUI: clickable
* [ ] CLI interface (non-interactive, e.g. --add-task or -a)
* [ ] Unit Tests
* [ ] Multi-calendar Search
* [ ] Desktop Notifications
* [ ] Add lightweight font w/ monochrome emojis (embedded font or iced_aw or ??? or use system standard font or use system standard icons)
* [ ] TUI/GUI(/CLI): multiple instances ok
* [ ] TUI: cursor when naming tasks
* [ ] TUI: list keywords when naming tasks
* [ ] switch this todo to cfait and version control it here
* [ ] move to gitlab-ci?
* [ ] publish crate

## License
GPL3
