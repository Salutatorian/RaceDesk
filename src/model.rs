use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub const TYPES: [&str; 5] = ["Oval", "Sports Car", "Formula Car", "Dirt", "RX"];
pub const LICENCES: [&str; 6] = ["R", "D", "C", "B", "A", "P"];

pub fn drag_position(origin: [i32; 2], pointer_origin: [i32; 2], pointer: [i32; 2]) -> [i32; 2] {
    [
        origin[0] + pointer[0] - pointer_origin[0],
        origin[1] + pointer[1] - pointer_origin[1],
    ]
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Filters {
    pub types: BTreeSet<String>,
    pub licences: BTreeSet<String>,
    pub official: [bool; 2],
    pub fixed: [bool; 2],
    pub owned_cars: bool,
    pub owned_tracks: bool,
    pub favourite_series: bool,
    pub favourite_cars: bool,
    pub favourite_tracks: bool,
    pub search: String,
}
impl Default for Filters {
    fn default() -> Self {
        Self {
            types: TYPES.map(String::from).into(),
            licences: LICENCES.map(String::from).into(),
            official: [true; 2],
            fixed: [true; 2],
            owned_cars: false,
            owned_tracks: false,
            favourite_series: false,
            favourite_cars: false,
            favourite_tracks: false,
            search: String::new(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub filters: Filters,
    pub owned_cars: BTreeSet<i64>,
    pub owned_tracks: BTreeSet<i64>,
    pub favourite_series: BTreeSet<i64>,
    pub favourite_cars: BTreeSet<i64>,
    pub favourite_tracks: BTreeSet<i64>,
    pub alert_series: BTreeSet<i64>,
    pub alert_once: BTreeSet<String>,
    pub delivered: BTreeSet<String>,
    pub lead_minutes: i64,
    pub alerts_enabled: bool,
    pub sound: bool,
    pub desktop: bool,
    pub widget_position: Option<[f32; 2]>,
    pub widget_size: [f32; 2],
    pub initialized: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            filters: Filters::default(),
            owned_cars: BTreeSet::new(),
            owned_tracks: BTreeSet::new(),
            favourite_series: BTreeSet::new(),
            favourite_cars: BTreeSet::new(),
            favourite_tracks: BTreeSet::new(),
            alert_series: BTreeSet::new(),
            alert_once: BTreeSet::new(),
            delivered: BTreeSet::new(),
            lead_minutes: 10,
            alerts_enabled: true,
            sound: true,
            desktop: true,
            widget_position: None,
            widget_size: [470., 610.],
            initialized: false,
        }
    }
}
impl Settings {
    pub fn matches(&self, r: &Race) -> bool {
        let f = &self.filters;
        f.types.contains(&r.kind)
            && f.licences.contains(&r.licence)
            && f.official[r.official as usize]
            && f.fixed[r.fixed as usize]
            && (!f.owned_tracks || self.owned_tracks.contains(&r.track_id))
            && (!f.owned_cars || r.car_ids.iter().any(|x| self.owned_cars.contains(x)))
            && (!f.favourite_series || self.favourite_series.contains(&r.series_id))
            && (!f.favourite_cars || r.car_ids.iter().any(|x| self.favourite_cars.contains(x)))
            && (!f.favourite_tracks || self.favourite_tracks.contains(&r.track_id))
            && (f.search.is_empty()
                || format!("{} {} {}", r.series, r.track, r.cars)
                    .to_lowercase()
                    .contains(&f.search.to_lowercase()))
    }
    pub fn alert_due(&self, series: i64, start: i64, now: i64, lead: i64) -> bool {
        let key = format!("{series}:{start}");
        self.alerts_enabled
            && start > now
            && start - now <= lead * 60
            && (self.alert_series.contains(&series) || self.alert_once.contains(&key))
            && !self.delivered.contains(&key)
    }
    pub fn initialize_content(&mut self, d: &Data) {
        for c in &d.cars {
            if c.free {
                self.owned_cars.insert(c.id);
            }
        }
        for t in &d.tracks {
            if t.free {
                self.owned_tracks.insert(t.id);
            }
        }
        self.initialized = true;
    }
}
#[derive(Clone)]
pub struct Race {
    pub series_id: i64,
    pub series: String,
    pub track_id: i64,
    pub track: String,
    pub kind: String,
    pub licence: String,
    pub official: bool,
    pub fixed: bool,
    pub car_ids: Vec<i64>,
    pub cars: String,
    pub week: i64,
    pub begin: i64,
    pub end: i64,
    pub descriptors: Vec<Value>,
    pub length: String,
    pub rain: i64,
}
impl Default for Race {
    fn default() -> Self {
        Self {
            series_id: 0,
            series: String::new(),
            track_id: 0,
            track: String::new(),
            kind: "Sports Car".into(),
            licence: "R".into(),
            official: true,
            fixed: false,
            car_ids: vec![],
            cars: String::new(),
            week: 0,
            begin: 0,
            end: 0,
            descriptors: vec![],
            length: String::new(),
            rain: 0,
        }
    }
}
pub fn next_repeat(begin: i64, end: i64, interval: i64, offset: i64, now: i64) -> Option<i64> {
    if interval <= 0 {
        return None;
    }
    let first = begin + offset;
    let n = if now <= first {
        first
    } else {
        first + ((now - first + interval - 1) / interval) * interval
    };
    (n >= begin && n < end).then_some(n)
}
impl Race {
    pub fn next(&self, now: i64) -> Option<i64> {
        let from = now.max(self.begin);
        self.descriptors
            .iter()
            .filter_map(|d| {
                if d["repeating"].as_bool() == Some(true) {
                    let interval = d["repeat_minutes"].as_i64().unwrap_or(0) * 60;
                    let offset = d["first_session_time"]
                        .as_str()
                        .unwrap_or("00:00:00")
                        .split(':')
                        .filter_map(|s| s.parse::<i64>().ok())
                        .fold(0, |a, b| a * 60 + b);
                    // Source repeats from Tuesday 00:00 UTC, including fortnightly weeks.
                    let date = DateTime::from_timestamp(from, 0)?.date_naive();
                    let back = (date.weekday().num_days_from_monday() as i64 + 6) % 7;
                    let anchor = (date - Duration::days(back))
                        .and_hms_opt(0, 0, 0)?
                        .and_utc()
                        .timestamp();
                    let mut candidate = next_repeat(anchor, self.end, interval, offset, from)?;
                    // Honour declared racing days and daily time ranges when supplied.
                    for _ in 0..10080 {
                        if candidate >= self.end {
                            return None;
                        }
                        let minutes = (candidate - anchor).rem_euclid(86400) / 60;
                        let day = (candidate - anchor).div_euclid(86400).rem_euclid(7);
                        let day_ok = d["day_offset"]
                            .as_array()
                            .is_none_or(|days| days.iter().any(|x| x.as_i64() == Some(day)));
                        let end = d["end_offset_minutes"].as_i64().unwrap_or(1439);
                        if day_ok && minutes <= end {
                            return Some(candidate);
                        }
                        candidate += interval;
                    }
                    None
                } else {
                    d["session_times"]
                        .as_array()?
                        .iter()
                        .filter_map(|x| parse_time(x.as_str()?))
                        .filter(|t| *t >= from && *t < self.end)
                        .min()
                }
            })
            .min()
    }
}
pub struct Content {
    pub id: i64,
    pub name: String,
    pub free: bool,
}
pub struct Data {
    pub races: Vec<Race>,
    pub cars: Vec<Content>,
    pub tracks: Vec<Content>,
    pub fetched: String,
}
pub fn parse_time(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|x| x.timestamp())
        .or_else(|| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()?
                .and_hms_opt(0, 0, 0)
                .map(|x| x.and_utc().timestamp())
        })
}
fn array(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn text(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}
impl Data {
    pub fn parse(s: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
        if array(&v["series"]).is_empty()
            || array(&v["cars"]).is_empty()
            || array(&v["tracks"]).is_empty()
        {
            return Err("Schedule catalog is missing or empty".into());
        }
        let mut races = vec![];
        for series in array(&v["series"]) {
            let season = series["seasonid"].as_i64().unwrap_or(0).to_string();
            let overrides = &v["overrides"][&season];
            let Some(mut begin) = parse_time(series["start"].as_str().unwrap_or("")) else {
                continue;
            };
            begin += overrides["weekStartOffset"]["days"].as_i64().unwrap_or(0) * 86400;
            let Some(end) = parse_time(series["end"].as_str().unwrap_or("")) else {
                continue;
            };
            let date = DateTime::from_timestamp(end, 0).unwrap().date_naive();
            let end = (date - Duration::days(date.weekday().num_days_from_monday() as i64)
                + Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
            let weeks = array(&series["tracks"]);
            let off = array(&overrides["offWeeks"]);
            if weeks.is_empty() {
                continue;
            }
            let step = ((end - begin) as f64 / (weeks.len() + off.len()) as f64).round() as i64;
            let mut order: Vec<f64> = weeks
                .iter()
                .filter_map(|w| w["raceweek"].as_f64())
                .chain(off.iter().filter_map(|x| x.as_f64().map(|x| x - 1.0)))
                .collect();
            order.sort_by(f64::total_cmp);
            for w in weeks {
                let week = w["raceweek"].as_i64().unwrap_or(0);
                let index = order.iter().position(|x| *x == week as f64).unwrap_or(0) as i64;
                let start = (begin + step * index).div_euclid(86400) * 86400;
                let cars = if !array(&w["carsForWeek"]).is_empty() {
                    array(&w["carsForWeek"])
                        .iter()
                        .filter_map(Value::as_i64)
                        .collect()
                } else {
                    array(&series["cars"])
                        .iter()
                        .filter_map(|x| x["sku"].as_i64())
                        .collect()
                };
                let names = if let Some(n) = w["carsForWeekName"].as_str() {
                    n.to_string()
                } else if !array(&w["carsForWeekNames"]).is_empty() {
                    array(&w["carsForWeekNames"])
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    array(&series["carclasses"])
                        .iter()
                        .filter_map(|x| x["shortname"].as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let level = series["minlicenselevel"].as_i64().unwrap_or(1);
                let licence = match level {
                    0..=3 => "R",
                    4..=7 => "D",
                    8..=11 => "C",
                    12..=15 => "B",
                    16..=19 => "A",
                    _ => "P",
                };
                let kind = match series["catid"].as_i64().unwrap_or(5) {
                    1 => "Oval",
                    3 => "Dirt",
                    4 => "RX",
                    6 => "Formula Car",
                    _ => "Sports Car",
                };
                let length = if let Some(laps) = w["race_lap_limit"].as_i64().filter(|x| *x > 0) {
                    format!("{laps} laps")
                } else if let Some(min) = w["race_time_limit"].as_i64() {
                    format!("{min} min")
                } else {
                    "Length unavailable".into()
                };
                races.push(Race {
                    series_id: series["seriesid"].as_i64().unwrap_or(0),
                    series: text(&series["seriesname"]),
                    track_id: w["pkgid"].as_i64().unwrap_or(0),
                    track: text(&w["name"]),
                    kind: kind.into(),
                    licence: licence.into(),
                    official: series["isOfficial"].as_bool().unwrap_or(false),
                    fixed: series["isFixedSetup"].as_bool().unwrap_or(false),
                    car_ids: cars,
                    cars: names,
                    week,
                    begin: start,
                    end: start + step,
                    descriptors: array(&w["race_time_descriptors"]).to_vec(),
                    length,
                    rain: w["precipChance"].as_i64().unwrap_or(0),
                });
            }
        }
        let cars = array(&v["cars"])
            .iter()
            .map(|x| Content {
                id: x["sku"].as_i64().unwrap_or(0),
                name: text(&x["name"]),
                free: x["freeWithSubscription"].as_bool().unwrap_or(false),
            })
            .collect();
        let tracks = array(&v["tracks"])
            .iter()
            .map(|x| Content {
                id: x["pkgid"].as_i64().unwrap_or(0),
                name: text(&x["name"]),
                free: x["default"].as_bool().unwrap_or(false),
            })
            .collect();
        Ok(Self {
            races,
            cars,
            tracks,
            fetched: text(&v["fetched_at"]),
        })
    }
}
pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("lol", "RaceDesk", "RaceDesk")
        .unwrap()
        .data_local_dir()
        .to_path_buf()
}
pub fn save_settings(path: &Path, settings: &Settings) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("Invalid settings path")?)
        .map_err(|e| e.to_string())?;
    let tmp = path.with_extension("pending");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // ReplaceFile semantics via rename on Windows replaces the existing destination.
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
pub fn now() -> i64 {
    Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeating_start_is_not_before_now_and_stops_at_week_end() {
        assert_eq!(next_repeat(100, 1000, 60, 10, 171), Some(230));
        assert_eq!(next_repeat(100, 200, 60, 10, 171), None);
        assert_eq!(next_repeat(100, 1000, 60, 10, 170), Some(170));
        assert_eq!(next_repeat(100, 1000, 0, 10, 170), None);
    }
    #[test]
    fn ownership_requires_any_car_and_the_track() {
        let race = Race {
            car_ids: vec![1, 2],
            track_id: 9,
            ..Race::default()
        };
        let mut settings = Settings::default();
        settings.filters.owned_cars = true;
        settings.filters.owned_tracks = true;
        settings.owned_cars.insert(2);
        assert!(!settings.matches(&race));
        settings.owned_tracks.insert(9);
        assert!(settings.matches(&race));
        settings.filters.types.clear();
        assert!(!settings.matches(&race));
    }
    #[test]
    fn preferences_and_alert_dedup_survive_restart() {
        let mut s = Settings::default();
        s.filters.owned_tracks = true;
        s.alert_series.insert(23);
        s.delivered.insert("23:500".into());
        let restored: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert!(restored.filters.owned_tracks);
        assert!(restored.alert_series.contains(&23));
        assert!(!restored.alert_due(23, 500, 300, 15));
        assert!(restored.alert_due(23, 600, 300, 15));
        assert!(!restored.alert_due(23, 250, 300, 15));
    }
    #[test]
    fn explicit_sessions_do_not_leak_into_following_week() {
        let begin = parse_time("2026-11-10T00:00:00Z").unwrap();
        let race = Race {
            begin,
            end: begin + 604800,
            descriptors: vec![
                serde_json::json!({"repeating":false,"session_times":["2026-11-11T03:00:00Z","2026-11-17T03:00:00Z"]}),
            ],
            ..Race::default()
        };
        assert_eq!(race.next(begin), parse_time("2026-11-11T03:00:00Z"));
        assert_eq!(race.next(parse_time("2026-11-12T00:00:00Z").unwrap()), None);
    }
    #[test]
    fn live_snapshot_has_valid_current_mazda_starts() {
        let data = Data::parse(include_str!("../schedule.json")).unwrap();
        let time = parse_time("2026-11-12T14:01:00Z").unwrap();
        let race = data
            .races
            .iter()
            .find(|r| {
                r.series == "Global Mazda MX-5 Cup by Fanatec" && r.begin <= time && r.end > time
            })
            .unwrap();
        assert_eq!(race.next(time), parse_time("2026-11-12T14:30:00Z"));
        assert_eq!(race.week, 8);
        assert_eq!(race.licence, "R");
        assert!(data.races.len() > 1900);
    }
    #[test]
    fn atomic_save_replaces_existing_preferences() {
        let folder = std::env::temp_dir().join(format!("racedesk-test-{}", std::process::id()));
        let file = folder.join("settings.json");
        let mut settings = Settings::default();
        save_settings(&file, &settings).unwrap();
        settings.lead_minutes = 27;
        save_settings(&file, &settings).unwrap();
        let loaded: Settings =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(loaded.lead_minutes, 27);
        std::fs::remove_file(&file).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
    #[test]
    fn drag_uses_pointer_delta_from_original_window_position() {
        assert_eq!(
            drag_position([200, 100], [240, 125], [390, 205]),
            [350, 180]
        );
        assert_eq!(drag_position([200, 100], [240, 125], [140, 25]), [100, 0]);
        assert_eq!(
            drag_position([200, 100], [240, 125], [240, 125]),
            [200, 100]
        );
    }
}
