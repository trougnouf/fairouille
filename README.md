

# Cfait
> Take control of your TODO list

**Cfait** is a powerful, simple, elegant, and lightweight CalDAV tasks manager, written in Rust.

It features both an efficient **TUI (Terminal UI)** and a modern **GUI (Graphical UI)** for desktop integration.

![logo](https://commons.wikimedia.org/wiki/Special:FilePath/Cfait_icon_v2.svg)
> The icon

![Cfait GUI Screenshot](https://commons.wikimedia.org/wiki/Special:FilePath/Cfait_task_manager_v0.2.7_screenshot_(GUI).png)
> The Graphical Interface in v0.2.7 <small>([history](https://commons.wikimedia.org/wiki/Category:Screenshots_of_Cfait_(GUI)))</small>

![Cfait TUI Screenshot](https://commons.wikimedia.org/wiki/Special:FilePath/Cfait_task_manager_v0.2.6_screenshot_(TUI).png)
> The Terminal Interface in v0.2.6 <small>([history](https://commons.wikimedia.org/wiki/Category:Screenshots_of_Cfait_(TUI)))</small>


## Features

*   **Dual Interface:** Run it in your terminal (`cfait`) or as a windowed app (`cfait-gui`).
*   **Smart Input:** Add tasks naturally: `Buy cat food !1 @tomorrow ~15m` sets Priority, Due Date, and Duration automatically.
*   **GTD Workflow:** Mark tasks as **In Process** (`>`), **Cancelled** (`x`), or **Done**.
*   **Duration Estimation:** Estimate time (`~2h`) and filter tasks by duration (`~<30m`).
*   **Syncs Everywhere:** Fully compatible with standard CalDAV servers (Radicale, Nextcloud, iCloud, etc.).
*   **Tag Support:** Organize tasks across all calendars using tags (e.g., `#woodworking`, `#project_potato`).
*   **Tag Aliases:** Define shortcuts (e.g., `#groceries`) that automatically expand into multiple tags (e.g., `#groceries`, `#shopping`, `#home`).
*   **Dependencies:** Link tasks using RFC 9253 (Blocked By) logic.
*   **Hierarchy Support:** Create sub-tasks directly from parents and organize nested lists easily.
*   **Multiple Calendars:** Seamlessly switch between "Work", "Personal", and other lists, or move tasks between them.
*   **Offline & Local First:** Optimistic UI updates mean you never wait for the server. Possibility to use the app immediately without a server; a persistent "Local" calendar stores its tasks on disk.
*   **Easy Migration:** When ready, export all tasks from the Local calendar to a CalDAV server with a single click (or keypress).
*   **Sane sorting:** Tasks are sorted by due date, then undated tasks are ordered by priority.


## Installation

### A. Pre-built Packages

Binaries are built on different environments to ensure maximum compatibility.
*   **Codeberg Releases** (Recommended)**:** Built on Arch Linux. Includes native Arch packages, cross-compiled Windows binaries, and Linux binaries.
*   **GitHub Releases:** Built on Ubuntu 24.04 and Windows.

If a binary from one source doesn't work for you, try the other.

*   **Arch Linux:**
    *   Option 1 (AUR): Build from source using your favorite helper:
        ```bash
        yay -S cfait      # Stable release
        # or
        yay -S cfait-git  # Latest git version
        ```
    *   Option 2 (Pre-built): Download the native `.pkg.tar.zst` from the [**Codeberg Releases**](https://codeberg.org/trougnouf/cfait/releases) and install it:
        ```bash
        sudo pacman -U cfait-*.pkg.tar.zst
        ```

*   **Debian / Ubuntu:**
    *   Download the `.deb` file from [**Codeberg**](https://codeberg.org/trougnouf/cfait/releases) (Built on Arch) or (if you encounter library errors, e.g. `glibc`) from [**GitHub**](https://github.com/trougnouf/cfait/releases) (Built on Ubuntu 24.04).
    *   Install:
        ```bash
        sudo dpkg -i cfait_*.deb
        ```

*   **Windows:**
    *   Download the `.zip` archive from [**Codeberg**](https://codeberg.org/trougnouf/cfait/releases) (Cross-compiled via MinGW) or [**GitHub**](https://github.com/trougnouf/cfait/releases) (Native build).
    *   Extract it and run `cfait.exe` (TUI) or `cfait-gui.exe` (GUI).

*   **Other Linux:**
    *   Download the generic `cfait-linux-*.tar.gz` archive from either release page.
    *   Extract and place the binaries in your `$PATH`.

### B. From Crates.io (via Cargo)

If you have Rust installed, you can install Cfait directly from crates.io.

```bash
# Install both TUI and GUI
cargo install cfait --features gui

# Or, install only the TUI
cargo install cfait
```

### C. For Development

If you want to contribute to Cfait, clone the repository and build it locally:

```bash
git clone https://codeberg.org/trougnouf/cfait.git
cd cfait

# Run the TUI
cargo run

# Run the GUI
cargo run --bin gui --no-default-features --features gui
```


### Run
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
url = "https://localhost:5232/trougnouf/"
username = "myuser"
password = "mypassword"

# Security: Allow self-signed certificates
# Default: false
allow_insecure_certs = true 

default_calendar = "Personal" # Optional: Auto-selects this list on startup

# Optional: Disable calendars you don't want to see (e.g., those without VTASKS capability).
# Use the full calendar href, not the display name.
#disabled_calendars = [
#    "/trougnouf/1355814b-9f29-792d-6dba-f6c671304517/",
#    "/trougnouf/36df9c8c-98e8-a920-7866-7b9d39bd8a24/",
#]

# Hide completed tasks in all views
hide_completed = false
# Hide tags from the sidebar if they contain NO active tasks
# When true, tags that have only completed tasks will be hidden from the Tags view
hide_fully_completed_tags = true

# Sorting: Tasks due more than X months away are sorted by priority only (not date)
# Default: 6
sort_cutoff_months = 6

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
**Sidebar (Cals)** | `Enter` | **Set Target** (Add to view) |
| | `Right` | **Focus** (Set Target + Hide others) |
  | | `Space` | **Toggle Visibility** (Show/Hide layer) |
| | `*` | **Toggle All** (Show all / Hide others) |
| **Sidebar (Tags)** | `Enter` | Toggle Tag Filter |
| | `m` | Toggle Tag Match Mode (AND / OR) |
| **Task List** | `j` / `k` | Move Down / Up |
| | `Space` | **Toggle** Completion |
| | `s` | **Start / Pause** (Mark In-Process) |
| | `x` | **Cancel** Task |
| | `a` | **Add** Task (Type name, press Enter) |
| | `C` | **Create Child** (Create new task linked as child of current, Shift+c) |
| | `e` | **Edit** Task Title |
| | `E` | **Edit** Task Description (Shift+e) |
| | `d` | **Delete** Task |
| | `M` | **Move** Task to another calendar (Shift+m) |
| | `y` | **Yank** (Copy ID for linking) |
| | `b` | **Block** (Mark current task as blocked by Yanked task) |
| | `c` | **Child** (Mark current task as child of Yanked task) |
| | `r` | **Refresh** (Force sync) |
| | `X` | **Export** (Migrate all tasks from Local to remote, Shift+x) |
| | `H` | Toggle **Hide Completed** tasks |
| | `/` | **Search** / Filter Tasks |
| | `+` / `-` | Increase / Decrease **Priority** |
| | `>` / `<` | **Indent** / **Outdent** (Visual Sub-tasks depth) |
| **Sidebar** | `Enter` | Select Calendar / Toggle Tag |
| | `1` | Switch to **Calendars** View |
| | `2` | Switch to **Tags** View |
| | `m` | Toggle Tag Match Mode (AND / OR) |

## Input Syntax
When adding (`a`) or editing (`e`) a task, you can use shortcuts directly in the text:

*   `!1` to `!9`: Sets **Priority** (1 is High, 9 is Low).
*   `due:DATE` or `@DATE`: Sets **Due Date**.
    *   Formats: `2025-12-31`, `today`, `tomorrow`, `1w` (1 week), `2d` (2 days).
*   `start:DATE` or `^DATE`: Sets **Start Date**.
    *   Tasks with a future start date are pushed to the bottom of the list ("Scheduled").
*   `est:DURATION` or `~DURATION`: Sets **Estimated Duration** (e.g., `~30m`, `~1h`).
    *   Also supports `~30min`.
*   `rec:RECURRENCE`: Sets **Recurrence** (e.g., `rec:weekly`, `rec:daily`).
    *   Also supports interval syntax: `rec:every 2 weeks`.
*   `#tag`: Adds a **Tag** / Category.
    *   **Aliases:** If you have configured aliases (e.g., `groceries = ["home"]`), typing `#groceries` will automatically apply `#groceries` AND `#home`.

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

**Example:** `~<20m !<4 #gardening` finds quick, high-priority, gardening tasks.

## Sorting
Tasks are sorted by:
1.  **Status**: In Process > Needs Action > Completed.
2.  **Scheduling**: Tasks with a **Start Date** in the future are pushed to the bottom.
3.  **Due Date**: Overdue and upcoming tasks appear first.
4.  **Priority**: Higher priority (`!1`) first.

## License
GPL3

## Mirrors

Commits are pushed to the following repositories. The automated build pipelines differ slightly:

*   **[Codeberg](https://codeberg.org/trougnouf/cfait)**
    *   **CI:** Runs lint and tests on every commit.
    *   **Environment:** Builds run on **Arch Linux**.
    *   **Artifacts:** Native Arch package (`.pkg.tar.zst`), Cross-compiled Windows build (MinGW), Cross-packaged Debian build, PKGBUILD.
    *   **Deployment:** Automatically pushes updates to the [AUR](https://aur.archlinux.org/packages/cfait).

*   **[GitHub](https://github.com/trougnouf/cfait)**
    *   **CI:** Runs tests on release.
    *   **Environment:** Builds run on **Ubuntu 24.04** and **Windows Server**.
    *   **Artifacts:** Native Debian/Ubuntu package, Native Windows build, PKGBUILD.
*   **[GitLab](https://gitlab.com/trougnouf/cfait)**
