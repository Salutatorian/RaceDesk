use racedesk::audio::{decode_sound, import_sound, BUILTIN_SOUND};
use rodio::Source;

#[test]
fn built_in_reminder_is_non_silent_and_short() {
    let source = decode_sound(None).unwrap();
    assert!(source.total_duration().unwrap().as_secs_f32() < 5.);
    assert!(source.fold(0.0_f32, |peak, sample| peak.max(sample.abs())) > 0.1);
    assert!(BUILTIN_SOUND.len() > 1000);
}

#[test]
fn imported_sound_survives_original_file_deletion() {
    let dir = std::env::temp_dir().join(format!("racedesk-audio-import-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("original.wav");
    std::fs::write(&source, BUILTIN_SOUND).unwrap();
    let imported = import_sound(&source, &dir.join("managed")).unwrap();
    std::fs::remove_file(source).unwrap();
    assert!(decode_sound(Some(&imported))
        .unwrap()
        .take(10000)
        .any(|x| x.abs() > 0.1));
    std::fs::remove_file(imported).unwrap();
    std::fs::remove_dir(dir.join("managed")).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn bad_import_does_not_replace_existing_sound() {
    let dir = std::env::temp_dir().join(format!("racedesk-audio-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("invalid.wav");
    let good = dir.join("valid.wav");
    std::fs::write(&good, BUILTIN_SOUND).unwrap();
    let existing = import_sound(&good, &dir.join("managed")).unwrap();
    std::fs::write(&bad, b"not audio").unwrap();
    assert!(import_sound(&bad, &dir.join("managed")).is_err());
    assert_eq!(std::fs::read(&existing).unwrap(), BUILTIN_SOUND);
    std::fs::remove_file(bad).unwrap();
    std::fs::remove_file(good).unwrap();
    std::fs::remove_file(existing).unwrap();
    std::fs::remove_dir(dir.join("managed")).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn old_preferences_gain_sound_defaults_and_custom_choices_roundtrip() {
    let mut settings: racedesk::model::Settings =
        serde_json::from_str(r#"{"sound":true}"#).unwrap();
    assert_eq!(settings.sound_volume, 80);
    assert!(settings.sound_file.is_none());
    assert!(settings.sound_device.is_none());
    settings.sound_volume = 42;
    settings.sound_file = Some("managed/custom.wav".into());
    settings.sound_device = Some("My headset".into());
    let loaded: racedesk::model::Settings =
        serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
    assert_eq!(loaded.sound_volume, 42);
    assert_eq!(loaded.sound_file, settings.sound_file);
    assert_eq!(loaded.sound_device, settings.sound_device);
}
