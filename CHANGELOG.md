# Changelog

## [0.2.9] - 2025-12-08

### 🚀 Features

- *(sync)* Preserve recurring task exceptions written by other clients
- *(gui)* Click on tag jumps to it
- *(gui)* Help on hover

### ⚡ Performance

- *(sync)* Optimize VTODO parsing and exception preservation to speedup startup from empty cache

### 🎨 Styling

- *(gui)* Always align tags to the right and try to share a line with the title
- *(gui)* Switch calendar highlight from blue to amber
- *(gui)* (deterministically) randomize tag color
- *(gui)* Switch Calendars/Tags header from blue to amber
- *(gui)* Move logo/icon to the sidebar when space permits

### ⚙️ Miscellaneous Tasks

- Switch to iced 0.14.0 (dev->release)
- *(forgejo)* Build once for different Linux releases
## [0.2.8] - 2025-12-08

### 🚀 Features

- *(ui)* Implement smart tag navigation, search result jumping, and implicit tag matching
- *(sync)* Implement safe 3-way merge for 412 conflicts to reduce duplicate tasks
- *(core)* Safe unmapped property handling
- *(gui)* Implement optimistic cache loading for instant startup

### 🐛 Bug Fixes

- *(gui)* Reset child creation mode when unlinking/canceling the parent reference
- *(tui)* Use default color for default text for white bg terminals compatibility
- *(core)* Optimize unmapped property parsing and ensure backward compatibility

### 📚 Documentation

- *(readme)* Mentian Mint

### 🎨 Styling

- *(gui)* "select" active task

### ⚙️ Miscellaneous Tasks

- Lint
- Update CHANGELOG
- Lint
- Release cfait version 0.2.8
## [0.2.7] - 2025-12-06

### 🚀 Features

- *(ui)* Display active task count next to each tag in GUI and TUI sidebars
- *(core)* Implement Start Date (DTSTART) with smart input parsing, sorting, and recurrence compatibility
- *(tui)* Implement PageUp/PageDown scrolling for sidebar lists
- *(tui)* Re-use '*' keybinding to clear all selected tags in tags view
- *(gui)* Add 'Clear All Tags' button to sidebar
- *(GUI)* Add help screen, use icons for help and settings
- *(gui)* Implement custom draggable and resizable client-side decorations
- *(gui)* Make the entire window header draggable

### 🐛 Bug Fixes

- *(tui)* Enable cursor movement in task creation input field
- *(gui)* Swap delete/cancel icon positions and adjust icons padding to prevent cropping

### 🚜 Refactor

- *(gui)* Decompose monolithic update logic into domain-specific modules
- *(gui)* Upgrade to iced 0.14-dev for native window resizing support

### 📚 Documentation

- Add icon to README

### 🎨 Styling

- *(tui)* Improve highlight contrast and right-align tags for readability
- Update logo (nerd-fonts cat -> Font Awesome, CC-BY-SA-4.0, license in LICENSES/nerd-fonts)
- Cleanup new logo
- Fix cropped cat outline
- *(gui)* Use ghost buttons for task actions and highlight destructive operations
- *(gui)* Add padding right of scroll bar to separate it from resizing
- *(gui)* Reduce vertical spacing between header and task list
- *(gui)* Reduce spacing between input bar and 1st task

### ⚙️ Miscellaneous Tasks

- *(release)* Update changelog for v0.2.6
- *(release)* Update screenshots for v0.2.6
- Release cfait version 0.2.6
- Fix Cargo.toml (too many keywords)
- Release cfait version 0.2.6
- Lint
- *(release)* Update readme and changelog for 0.2.7
- Release cfait version 0.2.7
## [0.2.5] - 2025-12-04

### 🚀 Features

- *(workflow)* Streamline child task creation from parent in GUI and TUI
- *(tui)* Add toggleable, dynamic, and comprehensive help screen
- *(ui)* Implement auto-jump to new tasks (TUI & GUI) and better scrollable logic
- *(GUI)* Tab between fields in the settings window

### 🚜 Refactor

- Split model, client, and gui view into granular submodules
- Modularize TUI logic into network actor and event handlers
- *(core)* Decouple search matching logic from store to model domain

### 🎨 Styling

- *(GUI)* Allow main content area to expand with window width

### ⚙️ Miscellaneous Tasks

- *(release)* Update changelog and screenshots for v0.2.5
- Release cfait version 0.2.5
## [0.2.4] - 2025-12-03

### 🚀 Features

- *(core)* Implement robust file locking with fs2, atomic journal processing, and isolated tests to prevent data corruption
- *(ui)* Enable multiline task descriptions in GUI and TUI (Alt+Enter), fix visual corruption in TUI, and propagate sync errors

### 📚 Documentation

- Move main mirror from github to codeberg

### ⚙️ Miscellaneous Tasks

- Add Codeberg Actions for testing and release builds
- Add Rust toolchain to Codeberg
- Lint
- Use lld linker to fix OOM errors and install clippy component
- Reduce memory usage by not compiling cargo-deb, 2-threads
- Self-hosted runner
- *(release)* Add cmake and nasm to fix windows cross-compilation and fix shell script syntax
- *(release)* Update changelog for v0.2.4
- Release cfait version 0.2.4
## [0.2.3] - 2025-12-01

### 🚀 Features

- *(core)* Implement layered calendars, disabled state, and robust tui visibility toggles

### 🐛 Bug Fixes

- *(gui)* Add close button to error banner and clear on success
- *(sync)* Implement safe conflict resolution (copy on 412), atomic file writes, and atomic move operations
- *(gui)* Preserve active calendar on refresh, always inject local calendar, and show duration metadata for untagged tasks
- *(model)* Treat no priority as implied normal priority (5) for sorting

### 🚜 Refactor

- *(sync)* Implement CTag caching, optimize fetch, and fix journal atomicity bugs

### ⚡ Performance

- *(net)* Constrain concurrent calendar fetches to 4 to prevent server overload
- *(core)* Implement bounded concurrency and delta sync for task fetching

### ⚙️ Miscellaneous Tasks

- Add licenses
- Lint
- *(release)* Update changelog
- Release cfait version 0.2.3
## [0.2.2] - 2025-11-29

### 🐛 Bug Fixes

- *(sync)* Handle 412 Precondition Failed by refreshing ETag and retrying

### 🎨 Styling

- [GUI] align tags with titles

### ⚙️ Miscellaneous Tasks

- Lint
- Update changelog
- Release cfait version 0.2.2
## [0.2.1] - 2025-11-29

### 🚀 Features

- *(security)* Implement secure TLS with insecure toggle and improve connection UX
- *(config)* Add setting to hide specific calendars from view
- *(core)* Implement moving tasks between calendars in GUI and TUI
- *(core)* Introduce a local-only calendar with an option to migrate tasks to a CalDAV server
- *(journaling)* Implement offline task queue and UI indicators
- *(gui)* Embed Symbols Nerd Font, iconify UI and compact task-row layout

### 📚 Documentation

- Replace #urgent with !<4 in Advanced search example
- Installation instructions (Arch, deb, Windows, generic-Linux, Rust crate)
- Update README

### 🎨 Styling

- *(gui)* Overhaul task row with a space-saving layout

### ⚙️ Miscellaneous Tasks

- Lint
- Auto-add changelog to release notes
- Release cfait version 0.2.1
## [0.2.0] - 2025-11-27

### 💼 Other

- Initial implementation of ongoing & canceled tasks (Need custom checkboxes)
- Custom checkmark icons (V,>,X)
- Implement GTD workflow and advanced search parser
- Lint
- Lint
- More linting and update screenshots for next release

### 🎨 UI/UX Improvements

- [TUI] Show [E]dit description in help bar
- [TUI] Refresh on error and add refresh key
- [GUI] add remove dependency button(s) in the task description

### ⚙️ Miscellaneous Tasks

- Automate changelog with git-cliff
- *(release)* Update changelog for v"${TAG}"
- Release cfait version 0.2.0
## [0.1.9] - 2025-11-26

### 💼 Other

- Update funding sources in FUNDING.yml
- Mention sorting
- Preparing for crate release
- Preparing for crate release
- Default to True
- Set cutoff date s.t. timed tasks are not always on top (default: 6-months). Add scroll wheel in GUI settings.
- Tags were saved with comma / not fully compatible w/ other clients
- Change hide_completed_in_tags setting to hide_fully_completed_tags (i.e. hide the tags, not the tasks within)

### 🎨 UI/UX Improvements

- [GUI] remove "<" and ">" buttons (replaced w/ Link functionality)

### ⚙️ Miscellaneous Tasks

- Release cfait version 0.1.9
## [0.1.7] - 2025-11-25

### 💼 Other

- Rename GUI window
- Mention categories
- Attempt Windows build (in next release)
- Groundwork to support RFC 9253 (DEPENDS-ON) in model
- Support RFC 9253 (DEPENDS-ON) in both TUI and GUI, and improve children dependency handling
- Add options=('!lto') to Arch PKGBUILD (fix issues when lto is enabled in makepkg.conf)
- Support aliases (set in the config file and/or in the GUI settings)
- Add subtitle
- Manually allow multiple RELATED-TO fields (not supported by icalendar library)
- Add unit tests
- Release 0.1.7

### 🎨 UI/UX Improvements

- [TUI] add space after [ ]
- [TUI] support RFC 9253 (DEPENDS-ON)
- [GUI] set window name in Linux only (fix Windows build?)
## [0.1.6] - 2025-11-24

### 💼 Other

- Rm warning
- Add icon, replace milk w/ cat food
- [README] add screenshots
- [README] add screenshots
- [README] use raw Wikimedia Commons URL for screenshots; gitlab does not support redirects
- Add cfait-git Arch PKGBUILD
- Uncomment icon
- Fix missing icon in opened application
- Refactor GUI, add support for #categories
- Refactor TUI
- Add unit tests
- Fix hide completed tasks in tab view, fix GUI save settings
- Fix bug where completed tags remained selected but invisible / hiding all tasks. Add uncategorized tag
- Fix TUI build error on CI, update screenshots

### 🎨 UI/UX Improvements

- [GUI] add Tags (categories) view (pulling from all calendars), add settings to hide completed tasks
- [TUI] Browse by category/tags, restore cache
- [GUI] fix cutoff tags AND/OR text
## [0.1.5] - 2025-11-22

### 💼 Other

- Fix github release
- Optimize binary size
- Add onboarding prompt
- Rename fairouille->cfait, automate Arch Linux PKGBUILD
- Add .deb release

### 🎨 UI/UX Improvements

- [TUI] respond to --help
## [0.1.4] - 2025-11-22

### 💼 Other

- Add license file, bump version
- Add Arch Linux PKGBUILD
- Rustache -> ferouille
- Rustache -> ferouille
## [0.1.3] - 2025-11-22

### 💼 Other

- Add recurrence support, recurrence symbol, and expand relative dates
- Add unit tests to model.rs
- Add caching (fast inter-calendar switching)
## [0.1.2] - 2025-11-21

### 💼 Other

- Allow viewing + editing description in GUI + TUI
- Bump version up to 0.1.2

### 🎨 UI/UX Improvements

- [GUI] Add priority / subtask / edit / delete buttons
- [GUI] show tasks description
## [0.1.1] - 2025-11-21

### 💼 Other

- Initial commit (working TUI with create/add/complete/delete, sorted by date+priorities
- Add multiple calendars support (from the same server)
- Add README
- Rename to rustache
- Add edit support
- Support moving cursor
- Prep for GUI
- Basic GUI (single-calendar)
- Update README

### 🎨 UI/UX Improvements

- [TUI] add scrolling
- [GUI] multi-calendar support
- [GUI] sub-tasks support
- [TUI] sub-tasks support
- [GUI] search function
- [GUI] show date. Bump to version 0.1.1
