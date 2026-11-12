# RaceDesk implementation plan

Goal: ship a runnable Windows desktop widget using Rust and Scrapling.

Architecture: Scrapling discovers script URLs and safely parses JSON string literals from the planner's bundle. It exports validated source data and source-specific week overrides. Rust builds the current schedule, filters it, persists user choices and dispatches Windows notifications. The helper runs only during refresh, not continuously.

1. Collector (`collector/collect.py`, `tests/test_collector.py`): first exercise safe extraction, invalid schema rejection and quoted strings. Run the failing tests, implement parsing, then collect live data and check all series and content references.
2. Core (`src/model.rs`): test repeated starts, explicit starts, week boundaries, intersecting ownership filters and persisted alert IDs. Implement the typed schedule and settings store; run `cargo test`.
3. Windows integration (`src/platform.rs`): attach/detach Explorer desktop, native notifications, startup registry entry and single-instance guard. Keep OS calls behind this module.
4. Interface (`src/main.rs`): compact dark schedule widget, next race emphasis, filters/content/settings, per-series and one-start alerts, tray recovery, refresh worker and visible errors.
5. Delivery (`Install.ps1`, `Uninstall.ps1`, `README.md`): package native release executable and short-lived collector; install only to the current user's LocalAppData, register Windows uninstall entry and shortcuts. Startup remains opt-in.
6. Verify: collector tests and fresh live collection, Rust tests, release build, launch/screenshot, settings round-trip and installation/removal-path review. Record actual results in README.
