use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

pub const BUILTIN_SOUND: &[u8] = include_bytes!("../assets/race-ready.wav");
pub const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
type Sound = Box<dyn Source<Item = f32> + Send>;

fn open_output(selected: Option<&str>) -> Result<(OutputStream, String), String> {
    let host = rodio::cpal::default_host();
    let device = if let Some(name) = selected {
        host.output_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().ok().as_deref() == Some(name))
            .ok_or_else(|| {
                format!("Audio output '{name}' is unavailable. Choose another output in Settings.")
            })?
    } else {
        host.default_output_device()
            .ok_or("No Windows default audio output is available.")?
    };
    let name = device
        .name()
        .unwrap_or_else(|_| "selected audio output".into());
    let mut stream = OutputStreamBuilder::from_device(device)
        .and_then(|builder| builder.open_stream())
        .map_err(|e| {
            format!("Could not open '{name}': {e}. Check your audio device and Windows volume.")
        })?;
    stream.log_on_drop(false);
    Ok((stream, name))
}

pub fn decode_sound(path: Option<&Path>) -> Result<Sound, String> {
    match path {
        Some(path) => {
            let file = fs::File::open(path).map_err(|e| format!("Cannot open sound: {e}"))?;
            if file.metadata().map_err(|e| e.to_string())?.len() > MAX_FILE_BYTES {
                return Err("Sound files must be 20 MB or smaller.".into());
            }
            Ok(Box::new(
                Decoder::try_from(file).map_err(|e| format!("Cannot decode sound: {e}"))?,
            ))
        }
        None => Ok(Box::new(
            Decoder::try_from(Cursor::new(BUILTIN_SOUND.to_vec())).map_err(|e| e.to_string())?,
        )),
    }
}

pub fn import_sound(source: &Path, folder: &Path) -> Result<PathBuf, String> {
    let extension = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["wav", "mp3", "ogg", "flac"].contains(&extension.as_str()) {
        return Err("Choose a WAV, MP3, OGG or FLAC audio file.".into());
    }
    let mut decoded = decode_sound(Some(source))?;
    if decoded.next().is_none() {
        return Err("The selected file contains no playable audio.".into());
    }
    drop(decoded);
    let bytes = fs::read(source).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("Sound files must be 20 MB or smaller.".into());
    }
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let target = folder.join(format!("custom.{extension}"));
    let temporary = folder.join(format!("custom.{extension}.pending"));
    fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    fs::rename(temporary, &target).map_err(|e| e.to_string())?;
    Ok(target)
}

enum Command {
    Play {
        path: Option<PathBuf>,
        volume: u8,
        device: Option<String>,
    },
    Stop,
    Volume(u8),
}
enum Event {
    Started(Option<String>),
    Finished,
    Failed(String),
}

pub struct Audio {
    commands: mpsc::Sender<Command>,
    events: mpsc::Receiver<Event>,
    pub status: String,
    pub plays_started: u64,
    pub playing: bool,
    pub outputs: Vec<String>,
    pub default_output: Option<String>,
}
impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}
impl Audio {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let (events_tx, events_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut active: Option<(OutputStream, Sink)> = None;
            loop {
                let command = if active.is_some() {
                    rx.recv_timeout(Duration::from_millis(50))
                } else {
                    rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected)
                };
                match command {
                    Ok(Command::Play {
                        path,
                        volume,
                        device,
                    }) => {
                        active = None;
                        let mut fallback = None;
                        let source = match decode_sound(path.as_deref()) {
                            Ok(source) => Ok(source),
                            Err(error) if path.is_some() => {
                                fallback=Some(format!("Custom sound unavailable ({error}); playing Race Ready instead."));
                                decode_sound(None)
                            }
                            Err(error) => Err(error),
                        };
                        let result = source.and_then(|source| {
                            let (stream, output_name) = open_output(device.as_deref())?;
                            fallback = Some(format!(
                                "{}Playing through {output_name}.",
                                fallback.take().map(|s| format!("{s} ")).unwrap_or_default()
                            ));
                            let sink = Sink::connect_new(stream.mixer());
                            sink.set_volume(f32::from(volume.min(100)) / 100.);
                            sink.append(source.take_duration(Duration::from_secs(30)));
                            Ok((stream, sink))
                        });
                        match result {
                            Ok(player) => {
                                active = Some(player);
                                let _ = events_tx.send(Event::Started(fallback));
                            }
                            Err(error) => {
                                let _ = events_tx.send(Event::Failed(error));
                            }
                        }
                    }
                    Ok(Command::Stop) => {
                        active = None;
                        let _ = events_tx.send(Event::Finished);
                    }
                    Ok(Command::Volume(volume)) => {
                        if let Some((_, sink)) = &active {
                            sink.set_volume(f32::from(volume.min(100)) / 100.);
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                if active.as_ref().is_some_and(|(_, sink)| sink.empty()) {
                    active = None;
                    let _ = events_tx.send(Event::Finished);
                }
            }
        });
        let mut player = Self {
            commands: tx,
            events: events_rx,
            status: String::new(),
            plays_started: 0,
            playing: false,
            outputs: Vec::new(),
            default_output: None,
        };
        player.refresh_outputs();
        player
    }
    pub fn refresh_outputs(&mut self) {
        let host = rodio::cpal::default_host();
        self.default_output = host.default_output_device().and_then(|d| d.name().ok());
        self.outputs = host
            .output_devices()
            .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default();
        self.outputs.sort();
        self.outputs.dedup();
    }
    pub fn play(&mut self, path: Option<PathBuf>, volume: u8, device: Option<String>) -> bool {
        if self
            .commands
            .send(Command::Play {
                path,
                volume,
                device,
            })
            .is_err()
        {
            self.status = "Audio player is unavailable. Restart RaceDesk.".into();
            false
        } else {
            self.status = "Opening audio output…".into();
            true
        }
    }
    pub fn stop(&mut self) {
        let _ = self.commands.send(Command::Stop);
    }
    pub fn set_volume(&mut self, volume: u8) {
        let _ = self.commands.send(Command::Volume(volume));
    }
    pub fn poll(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Started(fallback) => {
                    self.playing = true;
                    self.plays_started += 1;
                    self.status = fallback.unwrap_or_else(|| {
                        "Playing through your Windows default audio output.".into()
                    });
                }
                Event::Finished => {
                    self.playing = false;
                    self.status = "Sound playback finished.".into();
                }
                Event::Failed(error) => {
                    self.playing = false;
                    self.status = format!("Sound error: {error}");
                }
            }
        }
    }
}
