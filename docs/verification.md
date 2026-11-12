# Verification — 13 November 2026

## Version 0.1.1 branding

The original mint racing flag was exported as SVG, PNG, 16–256 px ICO and embedded RGBA window pixels. The simplified primary SVG passed the logo-design asset audit without warnings. Seven Rust tests, five Python tests, strict Clippy and a Windows release build passed. PE inspection confirmed icon and icon-group resources. The installed executable reports version 0.1.1; its extracted Windows icon was visually inspected. Native installation created a desktop shortcut and updated the Start menu shortcut, both explicitly pointing to the installed branded ICO. The running window also exposes an icon handle. Version 0.1.1 preserves the existing schedule and preferences.

## Widget dragging and saved position

After the user reported continued difficulty, the small title-only native drag control was replaced by a wide 42-pixel-tall bar labeled "Drag here to move", with a grab cursor and direct pointer-relative window positioning. Reordering below applications is suspended while interacting. Real mouse tests dragged the title by (150,60), blank header space by (-110,90), and the hint label by (60,-70), with the exact expected displacement in each case. A close/reopen preserved the final position. Seven Rust tests and strict Clippy passed, followed by a release build. The installed app and Downloads installer/source/checksums were replaced with this build.

An actual mouse drag moved the desktop widget from (196,168) to (304,231). The saved preferences contained (304,231), but the initial restart test restored an older eframe native position. The app now explicitly applies its own saved geometry after native window creation. A repeated real drag/close/reopen test preserved (304,231) correctly. Six Rust tests, strict Clippy checks and the release build passed after the fix. The standalone installer and source archive were refreshed.

## Standalone EXE distribution

The native Inno Setup installer was tested on this Windows session: exit code 0, expected per-user installation, native EXE uninstaller registered in Windows Apps, and existing preferences preserved byte-for-byte. The installed collector fetched the live schedule successfully without depending on the downloaded installer or a system Python installation.

The native uninstaller was also tested with RaceDesk running. It closed the app, exited successfully, removed the app and its registration, and retained preferences. RaceDesk was then reinstalled from the final standalone installer in Downloads.

Performed on the user's Windows x64 session:

- Six Rust tests passed: repeating start rounding and week-end boundary, explicit starts, ownership intersection, preferences/reminder deduplication serialization, replacing an existing settings file, and the live Mazda schedule fixture.
- Three Python collector tests passed: safe JSON literal extraction, missing data rejection, and nonliteral rejection.
- `cargo clippy --all-targets -- -D warnings` passed. `cargo build --release` passed.
- The packaged Scrapling executable fetched the live source without separately installed Python: 154 series, 193 cars, 153 tracks. Runtime model produced 1,999 race weeks and 137 active series at test time.
- The actual Explorer-owned widget rendered over the desktop; a screenshot was inspected. The Settings window was also rendered and inspected. The first child-window attachment approach was replaced because it did not render correctly on this Windows 11 build.
- Windows UI Automation located actual controls. Clicking the startup checkbox created the correct quoted executable path in the user's Run registry key; clicking again removed it.
- Clicking the test-reminder button returned no Windows notification error. Banner/sound presentation remains subject to Windows notification settings; an audible banner was not independently confirmed.
- Clicking the owned-track filter changed the saved JSON. The app was fully closed, installed and restarted; the filter remained enabled. The pre-test preferences were restored afterward.
- The per-user installer created the expected app folder, Start menu shortcut and Windows uninstall entry. Running the installed uninstaller removed all three and preserved preferences. The app was then reinstalled and launched in desktop mode.
- Startup remained disabled. No races were left selected by testing.

Windows reboot/sign-in itself, a real upcoming race notification, Explorer restart recovery and monitor disconnect/reconnect have not been independently exercised. Automated tests verify core reminder timing and duplicate prevention, rather than waiting for a real race. Keep RaceDesk running and Windows awake for alerts.
