#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod platform;
use eframe::egui::{self, Color32, RichText};
use racedesk::model::*;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    collections::BTreeSet,
    os::windows::process::CommandExt,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

const ACCENT: Color32 = Color32::from_rgb(110, 231, 183);
const MUTED: Color32 = Color32::from_rgb(156, 169, 184);
fn toggle<T: Ord + Clone>(set: &mut BTreeSet<T>, item: T, on: bool) {
    if on {
        set.insert(item);
    } else {
        set.remove(&item);
    }
}
fn check_set(ui: &mut egui::Ui, set: &mut BTreeSet<String>, value: &str) {
    let mut on = set.contains(value);
    if ui.checkbox(&mut on, value).changed() {
        toggle(set, value.to_string(), on);
    }
}
fn check_id(ui: &mut egui::Ui, set: &mut BTreeSet<i64>, id: i64, label: &str) {
    let mut on = set.contains(&id);
    if ui.checkbox(&mut on, label).changed() {
        toggle(set, id, on);
    }
}
fn local(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%a %d %b · %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}
fn remaining(t: i64) -> String {
    let s = (t - now()).max(0);
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m {:02}s", s / 60, s % 60)
    } else {
        format!("{}h {:02}m", s / 3600, (s % 3600) / 60)
    }
}
struct Tray {
    _icon: TrayIcon,
    settings: MenuItem,
    widget: MenuItem,
    quit: MenuItem,
}
impl Tray {
    fn new() -> Result<Self, String> {
        let menu = Menu::new();
        let settings = MenuItem::new("Settings and filters", true, None);
        let widget = MenuItem::new("Show desktop widget", true, None);
        let quit = MenuItem::new("Quit RaceDesk", true, None);
        menu.append_items(&[&settings, &widget, &quit])
            .map_err(|e| e.to_string())?;
        let mut rgba = vec![0u8; 32 * 32 * 4];
        for y in 0..32 {
            for x in 0..32 {
                let i = (y * 32 + x) * 4;
                let flag = (7..25).contains(&x) && (6..20).contains(&y);
                let color = if flag && (x / 4 + y / 4) % 2 == 0 {
                    [110, 231, 183, 255]
                } else {
                    [21, 29, 40, 255]
                };
                rgba[i..i + 4].copy_from_slice(&color);
            }
        }
        let icon = Icon::from_rgba(rgba, 32, 32).map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("RaceDesk — right-click for settings")
            .with_icon(icon)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            _icon: tray,
            settings,
            widget,
            quit,
        })
    }
}
struct App {
    settings: Settings,
    data: Data,
    path: PathBuf,
    saved: String,
    error: String,
    page: usize,
    expanded: bool,
    week_offset: i64,
    content_search: String,
    tray: Option<Tray>,
    desktop: platform::Desktop,
    notifier: platform::Notifier,
    refresh: Option<mpsc::Receiver<Result<Data, String>>>,
    last_refresh: Instant,
    first: Instant,
    last_tick: Instant,
    last_notification: i64,
    uninstall_confirm: bool,
    initial_geometry_applied: bool,
    window_drag: Option<platform::WindowDrag>,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10., 8.);
        style.visuals.panel_fill = Color32::from_rgb(17, 23, 32);
        style.visuals.selection.bg_fill = Color32::from_rgb(38, 95, 77);
        cc.egui_ctx.set_style(style);
        let path = data_dir().join("settings.json");
        let mut error = String::new();
        let mut settings = match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                error = format!(
                    "Settings could not be read: {e}. Original saved as settings.invalid.json."
                );
                let _ = std::fs::copy(&path, path.with_file_name("settings.invalid.json"));
                Settings::default()
            }),
            Err(_) => Settings::default(),
        };
        let cache = data_dir().join("schedule.json");
        let data = std::fs::read_to_string(&cache)
            .ok()
            .and_then(|s| Data::parse(&s).ok())
            .unwrap_or_else(|| {
                Data::parse(include_str!("../schedule.json")).expect("Validated bundled schedule")
            });
        if !settings.initialized {
            settings.initialize_content(&data);
        }
        let tray = match Tray::new() {
            Ok(t) => Some(t),
            Err(e) => {
                error = format!("Tray unavailable: {e}");
                None
            }
        };
        let mut app = Self {
            settings,
            data,
            path,
            saved: String::new(),
            error,
            page: 0,
            expanded: false,
            week_offset: 0,
            content_search: String::new(),
            tray,
            desktop: platform::Desktop::default(),
            notifier: platform::Notifier::default(),
            refresh: None,
            last_refresh: Instant::now(),
            first: Instant::now(),
            last_tick: Instant::now() - Duration::from_secs(2),
            last_notification: 0,
            uninstall_confirm: false,
            initial_geometry_applied: false,
            window_drag: None,
        };
        if std::env::args().any(|x| x == "--settings") {
            app.expanded = true;
            app.page = 3;
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(900., 700.)));
        }
        app.persist();
        app.start_refresh();
        app
    }
    fn persist(&mut self) {
        let serialized = serde_json::to_string(&self.settings).unwrap_or_default();
        if serialized != self.saved {
            match save_settings(&self.path, &self.settings) {
                Ok(()) => self.saved = serialized,
                Err(e) => self.error = format!("Could not save preferences: {e}"),
            }
        }
    }
    fn start_refresh(&mut self) {
        if self.refresh.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.refresh = Some(rx);
        self.last_refresh = Instant::now();
        std::thread::spawn(move || {
            let result = (|| -> Result<Data, String> {
                std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
                let dir = std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .parent()
                    .ok_or("No executable directory")?
                    .to_path_buf();
                let helper = dir.join("collector").join("racedesk-collector.exe");
                let mut command = if helper.exists() {
                    std::process::Command::new(helper)
                } else {
                    let script = dir.join("collector").join("collect.py");
                    if !script.exists() {
                        return Err(
                            "Collector is missing. Keep the collector folder beside RaceDesk.exe."
                                .into(),
                        );
                    }
                    let mut c = std::process::Command::new("python");
                    c.arg(script);
                    c
                };
                let staging = data_dir().join("refresh.json");
                let mut child = command
                    .arg(&staging)
                    .creation_flags(0x08000000)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| format!("Cannot start Scrapling collector: {e}"))?;
                let started = Instant::now();
                loop {
                    if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                        break;
                    }
                    if started.elapsed() > Duration::from_secs(150) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err("Refresh timed out. Last schedule remains available.".into());
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
                let output = child.wait_with_output().map_err(|e| e.to_string())?;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr)
                        .chars()
                        .rev()
                        .take(900)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect());
                }
                let s = std::fs::read_to_string(&staging).map_err(|e| e.to_string())?;
                let data = Data::parse(&s)?;
                std::fs::rename(staging, data_dir().join("schedule.json"))
                    .map_err(|e| e.to_string())?;
                Ok(data)
            })();
            let _ = tx.send(result);
        });
    }
    fn set_expanded(&mut self, ctx: &egui::Context, on: bool) {
        self.expanded = on;
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(on));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(if on {
            egui::vec2(900., 700.)
        } else {
            egui::vec2(self.settings.widget_size[0], self.settings.widget_size[1])
        }));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        if on {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }
    fn tick(&mut self, ctx: &egui::Context, hwnd: windows_sys::Win32::Foundation::HWND) {
        if self.last_tick.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last_tick = Instant::now();
        if !self.expanded && self.first.elapsed() > Duration::from_secs(2) {
            ctx.input(|input| {
                if let Some(rect) = input.viewport().outer_rect {
                    self.settings.widget_position = Some([rect.min.x, rect.min.y]);
                }
                if let Some(rect) = input.viewport().inner_rect {
                    self.settings.widget_size = [rect.width().max(360.), rect.height().max(350.)];
                }
            });
        }
        let wanted = self.settings.desktop && !self.expanded;
        if self.first.elapsed() > Duration::from_millis(700)
            && (wanted != self.desktop.attached || (wanted && !self.desktop.alive()))
        {
            if let Err(e) = self.desktop.set(hwnd, wanted) {
                self.error = e;
                self.settings.desktop = false;
            }
        }
        if self.window_drag.is_none() && !ctx.input(|i| i.pointer.any_down()) {
            self.desktop.keep_below_apps(hwnd);
        }
        if let Some(rx) = &self.refresh {
            match rx.try_recv() {
                Ok(Ok(data)) => {
                    self.settings.initialize_content(&data);
                    self.data = data;
                    self.refresh = None;
                    self.error.clear();
                }
                Ok(Err(e)) => {
                    self.error = format!("Refresh failed: {e}");
                    self.refresh = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.error = "Collector stopped unexpectedly".into();
                    self.refresh = None;
                }
                Err(_) => {}
            }
        }
        if self.last_refresh.elapsed() > Duration::from_secs(6 * 3600) {
            self.start_refresh();
        }
        let time = now();
        // Space notifications to avoid one Windows balloon overwriting another.
        if time - self.last_notification >= 8 {
            let due = self
                .data
                .races
                .iter()
                .filter_map(|r| r.next(time).map(|t| (r, t)))
                .filter(|(r, t)| {
                    self.settings
                        .alert_due(r.series_id, *t, time, self.settings.lead_minutes)
                })
                .min_by_key(|(_, t)| *t);
            if let Some((race, start)) = due {
                let title = format!("{} — {} min", race.series, ((start - time) + 59) / 60);
                let body = format!(
                    "{}\n{}\n{} · {}",
                    race.track,
                    local(start),
                    race.cars,
                    race.length
                );
                match self.notifier.show(hwnd, &title, &body, self.settings.sound) {
                    Ok(()) => {
                        let key = format!("{}:{start}", race.series_id);
                        self.settings.delivered.insert(key.clone());
                        self.settings.alert_once.remove(&key);
                        self.last_notification = time;
                    }
                    Err(e) => self.error = e,
                }
            }
        }
        self.settings.delivered.retain(|s| {
            s.rsplit(':')
                .next()
                .and_then(|x| x.parse::<i64>().ok())
                .is_some_and(|t| t > time - 86400 * 14)
        });
        self.persist();
        let pointer = ctx.input(|i| i.pointer.latest_pos().map(|p| [p.x, p.y]));
        let diagnostic = serde_json::json!({"desktop_attached":self.desktop.attached,"desktop_host":self.desktop.host as usize,"widget_hwnd":hwnd as usize,"expanded":self.expanded,"error":self.error,"races":self.data.races.len(),"updated":time,"pointer":pointer});
        let _ = std::fs::write(data_dir().join("status.json"), diagnostic.to_string());
        ctx.request_repaint_after(Duration::from_secs(1));
    }
    fn filters(&mut self, ui: &mut egui::Ui) {
        ui.heading("Find your races");
        ui.add(
            egui::TextEdit::singleline(&mut self.settings.filters.search)
                .hint_text("Search series, cars or tracks")
                .desired_width(f32::INFINITY),
        );
        ui.label(RichText::new("RACE TYPE").small().color(MUTED));
        ui.horizontal_wrapped(|ui| {
            for t in TYPES {
                check_set(ui, &mut self.settings.filters.types, t);
            }
        });
        ui.label(RichText::new("LICENCE").small().color(MUTED));
        ui.horizontal_wrapped(|ui| {
            for l in LICENCES {
                check_set(ui, &mut self.settings.filters.licences, l);
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.settings.filters.official[0], "Unofficial");
            ui.checkbox(&mut self.settings.filters.official[1], "Official");
            ui.checkbox(&mut self.settings.filters.fixed[0], "Open setup");
            ui.checkbox(&mut self.settings.filters.fixed[1], "Fixed setup");
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.settings.filters.owned_cars, "Owned cars only");
            ui.checkbox(&mut self.settings.filters.owned_tracks, "Owned tracks only");
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(
                &mut self.settings.filters.favourite_series,
                "Favourite series only",
            );
            ui.checkbox(
                &mut self.settings.filters.favourite_cars,
                "Favourite cars only",
            );
            ui.checkbox(
                &mut self.settings.filters.favourite_tracks,
                "Favourite tracks only",
            );
        });
        if ui.button("Reset filters").clicked() {
            self.settings.filters = Filters::default();
        }
        ui.label(
            RichText::new(
                "Changes save automatically. Manage ownership and favourites in My content.",
            )
            .small()
            .color(MUTED),
        );
    }
    fn schedule(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.small_button("‹").clicked() {
                self.week_offset -= 1;
            }
            if ui
                .small_button(if self.week_offset == 0 {
                    "This week"
                } else {
                    "Back to now"
                })
                .clicked()
            {
                self.week_offset = 0;
            }
            if ui.small_button("›").clicked() {
                self.week_offset += 1;
            }
            ui.label(
                RichText::new(local(now() + self.week_offset * 604800))
                    .small()
                    .color(MUTED),
            );
        });
        let time = now();
        let selected = time + self.week_offset * 604800;
        let mut rows: Vec<_> = self
            .data
            .races
            .iter()
            .filter(|r| selected >= r.begin && selected < r.end && self.settings.matches(r))
            .map(|r| {
                (
                    r.clone(),
                    r.next(if self.week_offset == 0 {
                        time
                    } else {
                        selected.max(r.begin)
                    }),
                )
            })
            .collect();
        rows.sort_by_key(|(_, start)| start.unwrap_or(i64::MAX));
        ui.label(
            RichText::new(format!("{} series · Windows local time", rows.len()))
                .small()
                .color(MUTED),
        );
        if rows.is_empty() {
            ui.add_space(24.);
            ui.heading("No races match");
            ui.label("Try changing filters or choosing another week.");
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("schedule")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, (race, start)) in rows.iter().enumerate() {
                    let highlighted = index == 0 && start.is_some() && self.week_offset == 0;
                    egui::Frame::new()
                        .fill(if highlighted {
                            Color32::from_rgb(24, 44, 42)
                        } else {
                            Color32::from_rgb(26, 34, 46)
                        })
                        .inner_margin(12.)
                        .corner_radius(10.)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{}  ·  {}  ·  {}",
                                        race.licence,
                                        race.kind,
                                        if race.fixed { "Fixed" } else { "Open" }
                                    ))
                                    .small()
                                    .color(if highlighted { ACCENT } else { MUTED }),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let favourite = self
                                            .settings
                                            .favourite_series
                                            .contains(&race.series_id);
                                        if ui
                                            .small_button(if favourite { "★" } else { "☆" })
                                            .on_hover_text("Favourite series")
                                            .clicked()
                                        {
                                            toggle(
                                                &mut self.settings.favourite_series,
                                                race.series_id,
                                                !favourite,
                                            );
                                        }
                                    },
                                );
                            });
                            ui.label(RichText::new(&race.series).size(17.).strong());
                            ui.label(&race.track);
                            ui.label(
                                RichText::new(format!(
                                    "{} · {} · Week {}",
                                    race.cars,
                                    race.length,
                                    race.week + 1
                                ))
                                .small()
                                .color(MUTED),
                            );
                            if let Some(start) = start {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(local(*start)).color(ACCENT));
                                    if self.week_offset == 0 {
                                        ui.label(
                                            RichText::new(format!("in {}", remaining(*start)))
                                                .strong(),
                                        );
                                    }
                                });
                                let key = format!("{}:{start}", race.series_id);
                                let mut once = self.settings.alert_once.contains(&key);
                                let mut series =
                                    self.settings.alert_series.contains(&race.series_id);
                                ui.horizontal_wrapped(|ui| {
                                    if ui.checkbox(&mut once, "Remind this start").changed() {
                                        toggle(&mut self.settings.alert_once, key, once);
                                    }
                                    if ui.checkbox(&mut series, "Every race").changed() {
                                        toggle(
                                            &mut self.settings.alert_series,
                                            race.series_id,
                                            series,
                                        );
                                    }
                                });
                            } else {
                                ui.label(
                                    RichText::new("No remaining start in this week").color(MUTED),
                                );
                            }
                        });
                    ui.add_space(6.);
                }
            });
    }
    fn content(&mut self, ui: &mut egui::Ui) {
        ui.heading("My content");
        ui.label("Free subscription content is included automatically. Tick the paid content you own; star your favourites.");
        ui.add(
            egui::TextEdit::singleline(&mut self.content_search).hint_text("Find a car or track"),
        );
        let query = self.content_search.to_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (label, items, owned, favourites) in [
                (
                    "Cars",
                    &self.data.cars,
                    &mut self.settings.owned_cars,
                    &mut self.settings.favourite_cars,
                ),
                (
                    "Tracks",
                    &self.data.tracks,
                    &mut self.settings.owned_tracks,
                    &mut self.settings.favourite_tracks,
                ),
            ] {
                ui.heading(label);
                for item in items
                    .iter()
                    .filter(|i| i.name.to_lowercase().contains(&query))
                {
                    ui.horizontal(|ui| {
                        let fav = favourites.contains(&item.id);
                        if ui.small_button(if fav { "★" } else { "☆" }).clicked() {
                            toggle(favourites, item.id, !fav);
                        }
                        if item.free {
                            ui.add_enabled(
                                false,
                                egui::Checkbox::new(&mut true, format!("{} · included", item.name)),
                            );
                        } else {
                            check_id(ui, owned, item.id, &item.name);
                        }
                    });
                }
            }
        });
    }
    fn preferences(&mut self, ui: &mut egui::Ui, hwnd: windows_sys::Win32::Foundation::HWND) {
        ui.heading("Make it yours");
        ui.checkbox(
            &mut self.settings.desktop,
            "Keep the widget on my desktop, behind normal windows",
        );
        ui.label(
            RichText::new("Settings opens as a normal window. Return to widget to reattach it.")
                .color(MUTED),
        );
        let mut startup = platform::startup_enabled();
        if ui
            .checkbox(&mut startup, "Open RaceDesk when I sign in to Windows")
            .changed()
        {
            if let Err(e) = platform::set_startup(startup) {
                self.error = e;
            }
        }
        ui.add_space(12.);
        ui.heading("Race reminders");
        ui.checkbox(&mut self.settings.alerts_enabled, "Enable reminders");
        ui.checkbox(&mut self.settings.sound, "Play notification sound");
        ui.add(
            egui::Slider::new(&mut self.settings.lead_minutes, 1..=120)
                .text("minutes before the start"),
        );
        ui.label("Select “Remind this start” or “Every race” on a series card. Alerts continue regardless of your current filters.");
        ui.label(RichText::new("RaceDesk must be running and Windows awake. Focus Assist and Windows notification settings can silence reminders.").small().color(MUTED));
        if ui.button("Send a test reminder").clicked() {
            if let Err(e) = self.notifier.show(
                hwnd,
                "RaceDesk test",
                "Your race reminders are ready. Choose a race on the schedule.",
                self.settings.sound,
            ) {
                self.error = e;
            }
        }
        ui.add_space(10.);
        ui.label(format!(
            "{} series alerts · {} individual starts",
            self.settings.alert_series.len(),
            self.settings.alert_once.len()
        ));
        if ui.button("Clear all selected reminders").clicked() {
            self.settings.alert_series.clear();
            self.settings.alert_once.clear();
        }
        ui.separator();
        ui.heading("Data and installation");
        ui.label(format!(
            "Last successful collection: {}",
            parse_time(&self.data.fetched)
                .map(local)
                .unwrap_or_else(|| self.data.fetched.clone())
        ));
        ui.label("Scrapling refreshes on launch and every 6 hours. Cached data works offline.");
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.refresh.is_none(), egui::Button::new("Refresh now"))
                .clicked()
            {
                self.start_refresh();
            }
            if ui.button("Open saved settings folder").clicked() {
                if let Err(e) = platform::open_path(&data_dir()) {
                    self.error = e;
                }
            }
        });
        ui.hyperlink_to(
            "Source: iRacing Week Planner",
            "https://iracing-week-planner.tmo.lol/",
        );
        ui.add_space(10.);
        if ui.button("Uninstall RaceDesk…").clicked() {
            self.uninstall_confirm = true;
        }
        if self.uninstall_confirm {
            ui.label("Remove this installation and its startup entry? Your saved preferences will be kept.");
            ui.horizontal(|ui|{if ui.button("Uninstall").clicked(){
                let exe=std::env::current_exe().unwrap();let script=exe.parent().unwrap().join("Uninstall.ps1");
                if !script.exists(){self.error="This is a development build. Run Uninstall.ps1 from the installed RaceDesk folder.".into();}else{
                    match std::process::Command::new("powershell.exe").args(["-NoProfile","-ExecutionPolicy","Bypass","-File"]).arg(script).creation_flags(0x08000000).spawn(){Ok(_)=>ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),Err(e)=>self.error=e.to_string()}
                }
            }
            if ui.button("Cancel").clicked(){self.uninstall_confirm=false;}});
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let hwnd = match frame.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(h)) => h.hwnd.get() as windows_sys::Win32::Foundation::HWND,
            _ => return,
        };
        // eframe may load older native window placement even with persist_window
        // disabled. Apply our own saved geometry after native window creation.
        if !self.initial_geometry_applied {
            if let Some([x, y]) = self.settings.widget_position {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
            }
            if !self.expanded {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    self.settings.widget_size[0],
                    self.settings.widget_size[1],
                )));
            }
            self.initial_geometry_applied = true;
        }
        self.tick(ctx, hwnd);
        if let Some(drag) = &self.window_drag {
            if ctx.input(|i| i.pointer.primary_down()) {
                if let Some([x, y]) = drag.update(hwnd) {
                    let scale = ctx.native_pixels_per_point().unwrap_or(1.);
                    self.settings.widget_position = Some([x as f32 / scale, y as f32 / scale]);
                }
                ctx.request_repaint();
            } else {
                self.window_drag = None;
                self.persist();
            }
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(tray) = &self.tray {
                if event.id == *tray.quit.id() {
                    self.persist();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                } else if event.id == *tray.settings.id() {
                    self.set_expanded(ctx, true);
                } else if event.id == *tray.widget.id() {
                    self.set_expanded(ctx, false);
                }
            }
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(17, 23, 32))
                    .inner_margin(16.),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (handle_rect, drag) = ui.allocate_exact_size(
                        egui::vec2((ui.available_width() - 110.).max(180.), 42.),
                        egui::Sense::click_and_drag(),
                    );
                    ui.painter().text(
                        handle_rect.left_top(),
                        egui::Align2::LEFT_TOP,
                        "RACE / DESK",
                        egui::FontId::proportional(19.),
                        ACCENT,
                    );
                    ui.painter().text(
                        handle_rect.left_top() + egui::vec2(0., 25.),
                        egui::Align2::LEFT_TOP,
                        "Drag here to move",
                        egui::FontId::proportional(12.),
                        MUTED,
                    );
                    let drag = drag
                        .on_hover_cursor(egui::CursorIcon::Grab)
                        .on_hover_text("Hold and drag anywhere on this bar to move the widget");
                    if drag.is_pointer_button_down_on() && self.window_drag.is_none() {
                        self.window_drag = platform::WindowDrag::begin(hwnd);
                        ctx.request_repaint();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("×")
                            .on_hover_text("Quit RaceDesk and stop alerts")
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui
                            .small_button(if self.expanded { "Widget" } else { "Settings" })
                            .clicked()
                        {
                            self.set_expanded(ctx, !self.expanded);
                        }
                    });
                });
                if self.expanded {
                    ui.horizontal(|ui| {
                        for (i, name) in ["Schedule", "Filters", "My content", "Settings"]
                            .iter()
                            .enumerate()
                        {
                            ui.selectable_value(&mut self.page, i, *name);
                        }
                    });
                    ui.separator();
                }
                if !self.error.is_empty() {
                    ui.colored_label(Color32::from_rgb(255, 183, 112), &self.error);
                }
                if self.refresh.is_some() {
                    ui.label(RichText::new("Refreshing schedule…").small().color(MUTED));
                }
                if parse_time(&self.data.fetched).is_some_and(|t| now() - t > 48 * 3600) {
                    ui.colored_label(
                        Color32::from_rgb(255, 183, 112),
                        "Cached schedule is over 48 hours old. Refresh to check for changes.",
                    );
                }
                if !self.expanded {
                    ui.label(
                        RichText::new("Your next grid. Your local time.")
                            .small()
                            .color(MUTED),
                    );
                }
                if !self.expanded || self.page == 0 {
                    self.schedule(ui);
                } else if self.page == 1 {
                    self.filters(ui);
                } else if self.page == 2 {
                    self.content(ui);
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| self.preferences(ui, hwnd));
                }
            });
        if !self.expanded {
            let rect = ctx.screen_rect();
            egui::Area::new("resize-grip".into())
                .fixed_pos(egui::pos2(rect.right() - 24., rect.bottom() - 24.))
                .show(ctx, |ui| {
                    let grip = ui
                        .add(
                            egui::Label::new(RichText::new("↘").color(MUTED))
                                .sense(egui::Sense::drag()),
                        )
                        .on_hover_text("Drag to resize");
                    if grip.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(
                            egui::ResizeDirection::SouthEast,
                        ));
                    }
                });
        }
        self.persist();
        ctx.request_repaint_after(Duration::from_secs(1));
    }
    fn save(&mut self, _: &mut dyn eframe::Storage) {
        self.persist();
    }
}
fn main() -> eframe::Result {
    if std::env::args().any(|a| a == "--check") {
        let data = Data::parse(include_str!("../schedule.json")).expect("schedule");
        let active = data
            .races
            .iter()
            .filter(|r| r.begin <= now() && r.end > now())
            .count();
        let next = data
            .races
            .iter()
            .filter(|r| r.begin <= now() && r.end > now() && r.next(now()).is_some())
            .count();
        std::fs::write(std::env::current_exe().unwrap().with_file_name("check.json"),serde_json::json!({"weeks":data.races.len(),"active_series":active,"with_upcoming_start":next,"data_dir":data_dir()}).to_string()).unwrap();
        return Ok(());
    }
    let Some(_instance) = platform::Instance::acquire() else {
        return Ok(());
    };
    let expanded = std::env::args().any(|x| x == "--settings");
    let geometry: Settings = std::fs::read_to_string(data_dir().join("settings.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let size = [
        geometry.widget_size[0].clamp(360., 1600.),
        geometry.widget_size[1].clamp(350., 1600.),
    ];
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("RaceDesk")
        .with_icon(std::sync::Arc::new(egui::IconData {
            rgba: include_bytes!("../assets/racedesk-64.rgba").to_vec(),
            width: 64,
            height: 64,
        }))
        .with_inner_size(if expanded { [900., 700.] } else { size })
        .with_min_inner_size([360., 350.])
        .with_decorations(expanded);
    if let Some([x, y]) = geometry.widget_position {
        viewport = viewport.with_position([x, y]);
    }
    let options = eframe::NativeOptions {
        viewport,
        persist_window: false,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "RaceDesk",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
