// SPDX-License-Identifier: GPL-3.0-or-later
//! Sound-effect playback (optional `audio` feature).
//!
//! Loads `soundN.wav` from the data directory (or from embedded bytes) and plays the
//! effect ids the engine emits per frame. Degrades to silence when built without the
//! `audio` feature, when there is no audio device, or when the sound files are absent — so
//! it never panics and the CI build stays simple (audio playback is not exercised in
//! tests). [`Audio::debug_summary`] and the [`PlayOutcome`] returned by [`Audio::play`]
//! make the otherwise-silent failure modes visible under `--debug`.

use std::path::Path;

/// The screensaver's sound player.
pub struct Audio {
    #[cfg(feature = "audio")]
    backend: Option<Backend>,
    /// Whether the player was muted (so `--debug` can tell "muted" from "device failed").
    muted: bool,
    /// How many `soundN.wav` slots are populated (loaded from disk or embedded).
    sounds_loaded: usize,
}

/// The result of a [`Audio::play`] call — for `--debug` diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Without the `audio` feature only `Unavailable` is ever produced.
#[cfg_attr(not(feature = "audio"), allow(dead_code))]
pub enum PlayOutcome {
    /// No backend: muted, no audio device, or built without the `audio` feature.
    Unavailable,
    /// No sound is loaded for this id.
    NotLoaded,
    /// Could not create a playback sink on the device.
    SinkFailed,
    /// The WAV bytes failed to decode.
    DecodeFailed,
    /// Queued for playback.
    Played,
}

impl Audio {
    /// Create the player, loading `soundN.wav` from `dir` if provided. When `muted`,
    /// no audio device is opened, no files are read, and playback is a no-op.
    #[cfg_attr(feature = "embed-data", allow(dead_code))] // embed builds use from_sounds
    pub fn new(dir: Option<&Path>, muted: bool) -> Self {
        #[cfg(feature = "audio")]
        {
            let mut sounds = vec![None; NUM_SOUNDS];
            if !muted {
                if let Some(dir) = dir {
                    for (id, slot) in sounds.iter_mut().enumerate() {
                        // Match the file name case-insensitively (sound2.wav / SOUND2.WAV).
                        if let Some(bytes) = wilson_dgds::find_ci(dir, &sound_filename(id as u16))
                            .and_then(|p| std::fs::read(p).ok())
                        {
                            *slot = Some(bytes);
                        }
                    }
                    // Fallback: the original distribution (e.g. scrantic-run.zip) ships no
                    // soundN.wav — the effects are embedded as WAVs in SCRANTIC.EXE/.SCR.
                    if sounds.iter().all(Option::is_none) {
                        for exe in ["SCRANTIC.EXE", "SCRANTIC.SCR"] {
                            let extracted = wilson_dgds::find_ci(dir, exe)
                                .and_then(|p| std::fs::read(p).ok())
                                .map(|b| wilson_dgds::sounds_from_scrantic_exe(&b));
                            if let Some(ex) = extracted {
                                if ex.iter().any(Option::is_some) {
                                    sounds = ex;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Audio::assemble(sounds, muted)
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = dir;
            Audio {
                muted,
                sounds_loaded: 0,
            }
        }
    }

    /// Create the player from already-loaded sound bytes (`sounds[id]`), e.g. data
    /// embedded into the binary. When `muted`, playback is a no-op.
    #[cfg_attr(not(feature = "embed-data"), allow(dead_code))] // only embed builds use it
    pub fn from_sounds(sounds: Vec<Option<Vec<u8>>>, muted: bool) -> Self {
        #[cfg(feature = "audio")]
        {
            Audio::assemble(sounds, muted)
        }
        #[cfg(not(feature = "audio"))]
        {
            let sounds_loaded = sounds.iter().filter(|s| s.is_some()).count();
            Audio {
                muted,
                sounds_loaded,
            }
        }
    }

    /// Count the loaded sounds and (unless muted) open the audio device.
    #[cfg(feature = "audio")]
    fn assemble(sounds: Vec<Option<Vec<u8>>>, muted: bool) -> Self {
        let sounds_loaded = sounds.iter().filter(|s| s.is_some()).count();
        Audio {
            backend: if muted {
                None
            } else {
                Backend::from_sounds(sounds)
            },
            muted,
            sounds_loaded,
        }
    }

    /// Play sound effect `id`, returning what happened (no-op + [`PlayOutcome::Unavailable`]
    /// when unavailable).
    pub fn play(&self, id: u16) -> PlayOutcome {
        #[cfg(feature = "audio")]
        {
            match &self.backend {
                Some(backend) => backend.play(id),
                None => PlayOutcome::Unavailable,
            }
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = id;
            PlayOutcome::Unavailable
        }
    }

    /// A one-line summary of the audio state for the `--debug` HUD/log: whether the
    /// `audio` feature is compiled in, whether it is muted, whether an output device
    /// opened, and how many sound effects are loaded.
    pub fn debug_summary(&self) -> String {
        #[cfg(feature = "audio")]
        let device = self.backend.is_some();
        #[cfg(not(feature = "audio"))]
        let device = false;
        format!(
            "feature={} muted={} device_open={} sounds_loaded={}",
            cfg!(feature = "audio"),
            self.muted,
            device,
            self.sounds_loaded,
        )
    }
}

/// The on-disk filename for sound effect `id`.
#[cfg(feature = "audio")]
#[cfg_attr(feature = "embed-data", allow(dead_code))]
fn sound_filename(id: u16) -> String {
    format!("sound{id}.wav")
}

#[cfg(feature = "audio")]
#[cfg_attr(feature = "embed-data", allow(dead_code))]
const NUM_SOUNDS: usize = 25;

#[cfg(feature = "audio")]
struct Backend {
    // The stream must be kept alive for playback to work.
    _stream: rodio::OutputStream,
    handle: rodio::OutputStreamHandle,
    sounds: Vec<Option<Vec<u8>>>,
}

#[cfg(feature = "audio")]
impl Backend {
    fn from_sounds(sounds: Vec<Option<Vec<u8>>>) -> Option<Self> {
        let (stream, handle) = rodio::OutputStream::try_default().ok()?;
        Some(Backend {
            _stream: stream,
            handle,
            sounds,
        })
    }

    fn play(&self, id: u16) -> PlayOutcome {
        let Some(Some(bytes)) = self.sounds.get(id as usize) else {
            return PlayOutcome::NotLoaded;
        };
        let Ok(sink) = rodio::Sink::try_new(&self.handle) else {
            return PlayOutcome::SinkFailed;
        };
        match rodio::Decoder::new(std::io::Cursor::new(bytes.clone())) {
            Ok(decoder) => {
                sink.append(decoder);
                sink.detach(); // play to completion in the background
                PlayOutcome::Played
            }
            Err(_) => PlayOutcome::DecodeFailed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "audio")]
    #[test]
    fn filename() {
        assert_eq!(sound_filename(0), "sound0.wav");
        assert_eq!(sound_filename(24), "sound24.wav");
    }

    #[test]
    fn silent_without_device_or_files() {
        // No data dir (and usually no device in CI): must not panic and play is a no-op.
        let audio = Audio::new(None, false);
        assert_eq!(audio.play(0), PlayOutcome::Unavailable);
        assert_eq!(audio.play(24), PlayOutcome::Unavailable);
    }

    #[test]
    fn muted_is_a_silent_no_op() {
        let audio = Audio::new(None, true);
        assert_eq!(audio.play(0), PlayOutcome::Unavailable);
        assert!(audio.debug_summary().contains("muted=true"));
    }

    #[test]
    fn from_sounds_muted_is_a_silent_no_op() {
        // The embed-data path builds the player from in-memory bytes. Muted does not open
        // an audio device, must not panic, and play is a no-op. (We deliberately do NOT
        // open a real device here: probing two output streams concurrently crashes WASAPI
        // on Windows CI; the no-device unmuted path is covered by the test above.)
        let audio = Audio::from_sounds(vec![Some(vec![0u8; 8]); 25], true);
        assert_eq!(audio.play(0), PlayOutcome::Unavailable);
        assert_eq!(audio.play(24), PlayOutcome::Unavailable);
        // The summary reports the loaded count even when muted (device stays closed).
        assert!(audio.debug_summary().contains("sounds_loaded=25"));
    }
}
