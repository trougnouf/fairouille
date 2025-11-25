# Cfait
> Take control of your TODO list

**Cfait** is a simple, elegant, and lightweight CalDAV task manager, written in Rust.

It features both a lightning-fast **TUI (Terminal UI)** and a modern **GUI (Graphical UI)** for desktop integration.

![Cfait GUI Screenshot](https://commons.wikimedia.org/wiki/Special:FilePath/Cfait_task_manager_v0.1.6_screenshot_(GUI).png)
*The Graphical Interface in v0.1.6*

![Cfait TUI Screenshot](https://commons.wikimedia.org/wiki/Special:FilePath/Cfait_task_manager_v0.1.6_screenshot_(TUI).png)
*The Terminal Interface in v0.1.6*

## Features

*   **Dual Interface:** Run it in your terminal (`cfait`) or as a windowed app (`cfait-gui`).
*   **Smart Input:** Add tasks naturally: `Buy cat food !1 @tomorrow` sets High Priority and Due Date automatically.
*   **Syncs Everywhere:** Fully compatible with standard CalDAV servers (Radicale, Nextcloud, iCloud, etc.).
*   **Tag Support:** Organize tasks across all calendars using tags (e.g., `#work`, `#urgent`).
*   **Tag Aliases:** Define shortcuts (e.g., `#groceries`) that automatically expand into multiple tags (e.g., `#groceries`, `#shopping`, `#home`).
*   **Dependencies:** Link tasks using RFC 9253 (Blocked By) logic.
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

The GUI includes a configuration dialog which writes to the config file below.

The TUI has on onboarding dialog but it is only used to populate the `url`, `username`, and `password` fields.

Create a config file at:
*   **Linux:** `~/.config/cfait/config.toml`
*   **Mac:** `~/Library/Application Support/com.cfait.cfait/config.toml`

```toml
url = "https://caldav.example.com/remote.php/dav/calendars/user/"
username = "myuser"
password = "mypassword"
default_calendar = "Personal" # Optional: Auto-selects this list on startup

# Hide completed tasks in all views
hide_completed = false
# Hide completed tasks only when viewing Tags/Categories
hide_completed_in_tags = true

# Tag Aliases: Automatically expand one tag into multiple
[tag_aliases]
groceries = ["shopping", "home"]  # Typing #groceries will add #groceries, #shopping and #home
cfait = ["dev", "rust"]           # Typing #cfait will add #cfait, #dev and #rust
```

## TUI Keybindings

| Context | Key | Action |
| :--- | :--- | :--- |
| **Global** | `Tab` | Switch focus (Tasks ↔ Sidebar) |
| | `q` | Quit |
| **Task List** | `j` / `k` | Move Down / Up |
| | `Space` | **Toggle** Completion |
| | `a` | **Add** Task (Type name, press Enter) |
| | `e` | **Edit** Title (Shift+E for Description) |
| | `d` | **Delete** Task |
| | `y` | **Yank** (Copy ID for linking) |
| | `b` | **Block** (Mark current task as blocked by Yanked task) |
| | `H` | Toggle **Hide Completed** tasks |
| | `/` | **Search** / Filter Tasks |
| | `+` / `-` | Increase / Decrease **Priority** |
| | `>` / `<` | **Indent** / **Outdent** (Create Sub-tasks) |
| **Sidebar** | `Enter` | Select Calendar / Toggle Tag |
| | `1` | Switch to **Calendars** View |
| | `2` | Switch to **Tags** View |
| | `m` | Toggle Tag Match Mode (AND / OR) |

## Input Syntax
When adding (`a`) or editing (`e`) a task, you can use shortcuts directly in the text:

*   `!1` to `!9`: Sets Priority (1 is High, 9 is Low).
*   `@tomorrow`, `@today`, `@next week`, `@next month`, `@next year`: Sets Due Date relative to now.
*   `@2025-12-31`: Sets specific Due Date (YYYY-MM-DD).
*   `@daily`, `@weekly`, `@monthly`, `@yearly`, `@every 4 days`, `@every 2 weeks`, etc: Sets Recurrence.
*   `#tag` (e.g. `#Gardening`) to set a tag / category.
*   **Aliases:** If you have configured aliases (e.g. `groceries = ["home"]`), typing `#groceries` will automatically apply `#groceries` AND `#home`.

## License
GPL3