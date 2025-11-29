# Changelog

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
