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
    /// Create the player, loading `soundN.wav` from `dir` if provided.
    pub fn new(dir: Option<&Path>) -> Self {
        #[cfg(feature = "audio")]
        {
            Audio {
                backend: Backend::new(dir),
            }
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = dir;
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
fn sound_filename(id: u16) -> String {
    format!("sound{id}.wav")
}

#[cfg(feature = "audio")]
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
    fn new(dir: Option<&Path>) -> Option<Self> {
        let (stream, handle) = rodio::OutputStream::try_default().ok()?;
        let mut sounds = vec![None; NUM_SOUNDS];
        if let Some(dir) = dir {
            for (id, slot) in sounds.iter_mut().enumerate() {
                if let Ok(bytes) = std::fs::read(dir.join(sound_filename(id as u16))) {
                    *slot = Some(bytes);
                }
            }
        }
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
        let audio = Audio::new(None);
        audio.play(0);
        audio.play(24);
    }
}
