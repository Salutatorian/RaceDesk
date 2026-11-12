# RaceDesk — approved direction, 13 November 2026

A live, movable, resizable Windows desktop widget over the existing wallpaper. Rust owns the native UI, persisted filters, countdowns, notifications and startup controls. A short-lived Scrapling helper refreshes the public planner's embedded schedule and content catalog. No account credentials are required. Local Windows time is used for display; scheduling and saved alert identities use UTC.

Match the source's type, licence, official, setup, owned car/track, and favourite series/car/track filters. Include local content management, week navigation and next-start ordering. Save preferences immediately with atomic replacement and keep the last valid schedule if refresh fails. Alerts may follow a series or target a single start, with configurable lead time; avoid duplicates across restarts. Windows sleep cannot deliver an alert while asleep; resume must recalculate.

Use an Explorer desktop host for the widget and detach into a normal window for settings; provide a tray recovery menu. Keep startup opt-in and expose a per-user uninstaller. Test data extraction against the live bundle, race boundary calculations, filtering, persistence and deduplication. Build the Windows executable, package the collector, launch and inspect the actual desktop UI. Explicitly report any unverified Explorer behavior.
