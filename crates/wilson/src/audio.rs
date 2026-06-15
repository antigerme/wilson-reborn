// SPDX-License-Identifier: GPL-3.0-or-later
//! Sound-effect playback (optional `audio` feature).
//!
//! Loads `soundN.wav` from the data directory and plays the effect ids the engine
//! emits per frame. Degrades to silence when built without the `audio` feature, when
//! there is no audio device, or when the sound files are absent — so it never panics
//! and the CI build stays simple (audio playback is not exercised in tests).

use std::path::Path;

/// The screensaver's sound player.
pub struct Audio {
    #[cfg(feature = "audio")]
    backend: Option<Backend>,
}

impl Audio {
    /// Create the player, loading `soundN.wav` from `dir` if provided. When `muted`,
    /// no audio device is opened and playback is a no-op.
    #[cfg_attr(feature = "embed-data", allow(dead_code))] // embed builds use from_sounds
    pub fn new(dir: Option<&Path>, muted: bool) -> Self {
        #[cfg(feature = "audio")]
        {
            Audio {
                backend: if muted { None } else { Backend::new(dir) },
            }
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = (dir, muted);
            Audio {}
        }
    }

    /// Create the player from already-loaded sound bytes (`sounds[id]`), e.g. data
    /// embedded into the binary. When `muted`, playback is a no-op.
    #[cfg_attr(not(feature = "embed-data"), allow(dead_code))] // only embed builds use it
    pub fn from_sounds(sounds: Vec<Option<Vec<u8>>>, muted: bool) -> Self {
        #[cfg(feature = "audio")]
        {
            Audio {
                backend: if muted {
                    None
                } else {
                    Backend::from_sounds(sounds)
                },
            }
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = (sounds, muted);
            Audio {}
        }
    }

    /// Play sound effect `id` (no-op if unavailable).
    pub fn play(&self, id: u16) {
        #[cfg(feature = "audio")]
        if let Some(backend) = &self.backend {
            backend.play(id);
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = id;
        }
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
    #[cfg_attr(feature = "embed-data", allow(dead_code))]
    fn new(dir: Option<&Path>) -> Option<Self> {
        let mut sounds = vec![None; NUM_SOUNDS];
        if let Some(dir) = dir {
            for (id, slot) in sounds.iter_mut().enumerate() {
                // Match the file name case-insensitively (sound2.wav / SOUND2.WAV / …).
                if let Some(bytes) = wilson_dgds::find_ci(dir, &sound_filename(id as u16))
                    .and_then(|p| std::fs::read(p).ok())
                {
                    *slot = Some(bytes);
                }
            }
        }
        Backend::from_sounds(sounds)
    }

    #[cfg_attr(not(feature = "embed-data"), allow(dead_code))]
    fn from_sounds(sounds: Vec<Option<Vec<u8>>>) -> Option<Self> {
        let (stream, handle) = rodio::OutputStream::try_default().ok()?;
        Some(Backend {
            _stream: stream,
            handle,
            sounds,
        })
    }

    fn play(&self, id: u16) {
        let Some(Some(bytes)) = self.sounds.get(id as usize) else {
            return;
        };
        let Ok(sink) = rodio::Sink::try_new(&self.handle) else {
            return;
        };
        if let Ok(decoder) = rodio::Decoder::new(std::io::Cursor::new(bytes.clone())) {
            sink.append(decoder);
            sink.detach(); // play to completion in the background
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
        audio.play(0);
        audio.play(24);
    }

    #[test]
    fn muted_is_a_silent_no_op() {
        let audio = Audio::new(None, true);
        audio.play(0);
    }

    #[test]
    fn from_sounds_muted_is_a_silent_no_op() {
        // The embed-data path builds the player from in-memory bytes. Muted does not open
        // an audio device, must not panic, and play is a no-op. (We deliberately do NOT
        // open a real device here: probing two output streams concurrently crashes WASAPI
        // on Windows CI; the no-device unmuted path is covered by the test above.)
        let audio = Audio::from_sounds(vec![Some(vec![0u8; 8]); 25], true);
        audio.play(0);
        audio.play(24);
    }
}
