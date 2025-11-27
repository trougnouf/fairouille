# Cfait
> Take control of your TODO list

**Cfait** is a simple, elegant, and lightweight CalDAV task manager, written in Rust.

It features both an efficient **TUI (Terminal UI)** and a modern **GUI (Graphical UI)** for desktop integration.

![Cfait GUI Screenshot](https://commons.wikimedia.org/wiki/Special:FilePath/Cfait_task_manager_v0.2.0_screenshot_(GUI).png)
> The Graphical Interface in v0.1.9

![Cfait TUI Screenshot](https://commons.wikimedia.org/wiki/Special:FilePath/Cfait_task_manager_v0.2.0_screenshot_(TUI).png)
> The Terminal Interface in v0.1.9

## Features

*   **Dual Interface:** Run it in your terminal (`cfait`) or as a windowed app (`cfait-gui`).
*   **Smart Input:** Add tasks naturally: `Buy cat food !1 @tomorrow ~15m` sets Priority, Due Date, and Duration automatically.
*   **GTD Workflow:** Mark tasks as **In Process** (`>`), **Cancelled** (`x`), or **Done**.
*   **Duration Estimation:** Estimate time (`~2h`) and filter tasks by duration (`~<30m`).
*   **Syncs Everywhere:** Fully compatible with standard CalDAV servers (Radicale, Nextcloud, iCloud, etc.).
*   **Tag Support:** Organize tasks across all calendars using tags (e.g., `#woodworking`, `#project_potato`).
*   **Tag Aliases:** Define shortcuts (e.g., `#groceries`) that automatically expand into multiple tags (e.g., `#groceries`, `#shopping`, `#home`).
*   **Dependencies:** Link tasks using RFC 9253 (Blocked By) logic.
*   **Hierarchy Support:** Create sub-tasks and organize nested lists easily.
*   **Multiple Calendars:** Seamlessly switch between "Work", "Personal", and other lists.
*   **Offline-First:** Optimistic UI updates mean you never wait for the server.
*   **Sane sorting:** Tasks are sorted by due date, then undated tasks are ordered by priority.

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
# Hide tags from the sidebar if they contain NO active tasks
# When true, tags that have only completed tasks will be hidden from the Tags view
hide_fully_completed_tags = true

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
| | `s` | **Start / Pause** (Mark In-Process) |
| | `x` | **Cancel** Task |
| | `a` | **Add** Task (Type name, press Enter) |
| | `e` | **Edit** Task Title |
| | `E` | **Edit** Task Description (Shift+e) |
| | `d` | **Delete** Task |
| | `y` | **Yank** (Copy ID for linking) |
| | `b` | **Block** (Mark current task as blocked by Yanked task) |
| | `r` | **Refresh** (Force sync) |
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
*   `~30m`, `~1h`, `~2d`: Sets **Estimated Duration**.
*   `@daily`, `@weekly`, `@monthly`, `@yearly`, `@every 4 days`, `@every 2 weeks`, etc: Sets Recurrence.
*   `#tag` (e.g. `#Gardening`) to set a tag / category.
    *   **Aliases:** If you have configured aliases (e.g. `groceries = ["home"]`), typing `#groceries` will automatically apply `#groceries` AND `#home`.

## Advanced Search
The search bar (in both GUI and TUI) supports powerful filtering syntax:

*   `text`: Matches title or description.
*   `#tag`: Filters by tag (e.g. `#work`).
*   `~<30m`: Duration less than 30 mins.
*   `~>=1h`: Duration greater or equal to 1 hour.
*   `!<3`: Priority higher than 3 (1 or 2).
*   `!>=5`: Priority 5 or lower.
*   `@<2025-01-01`: Due before specific date.
*   `@<1w`: Due within 1 week from today.
*   `@>=2d`: Due at least 2 days from today.
*   `is:done`: Show only completed/cancelled tasks.
*   `is:ongoing`: Show only ongoing (started) tasks.
*   `is:active`: Show only active (not completed/cancelled) tasks.

**Example:** `~<=15m !<4 is:active` finds quick, high-priority, active tasks.

## License
GPL3