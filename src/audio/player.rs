use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, buffer::SamplesBuffer};
use std::cell::Cell;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::{Duration, Instant};

const MAX_EXTRAPOLATION: Duration = Duration::from_millis(60);

pub struct AudioPlayer {
    out: Option<Out>,
    /// Without a sound device the song still plays, silently, on this clock.
    silent: Cell<Silent>,
    started: bool,
    clock: Cell<Clock>,
}

struct Out {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Sink,
}

/// A song's clock with no sound device: wall time from `play`, at `speed`,
/// stopping at the song's end.
#[derive(Clone, Copy, Default)]
struct Silent {
    /// Position when the clock last started or stopped.
    pos: Duration,
    /// Running since; None while paused or stopped.
    since: Option<Instant>,
    speed: f32,
    len: Duration,
}

impl Silent {
    fn pos(&self) -> Duration {
        let run = self
            .since
            .map(|t| t.elapsed().mul_f32(self.speed.max(0.01)))
            .unwrap_or_default();
        (self.pos + run).min(self.len)
    }
    fn stopped(self) -> Silent {
        Silent {
            pos: self.pos(),
            since: None,
            ..self
        }
    }
}

/// True when `CASCADE_NO_AUDIO` is set: no sound device is opened at all
/// (recordings, tests, machines where the device misbehaves).
pub fn no_audio() -> bool {
    std::env::var_os("CASCADE_NO_AUDIO").is_some_and(|v| !v.is_empty() && v != "0")
}

#[derive(Clone, Copy)]
struct Clock {
    raw: Duration,
    raw_at: Instant,
    last: Duration,
}

impl Clock {
    fn at(pos: Duration) -> Self {
        Self {
            raw: pos,
            raw_at: Instant::now(),
            last: pos,
        }
    }
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let out = if no_audio() {
            log::info!("audio off (CASCADE_NO_AUDIO)");
            None
        } else {
            match Self::open() {
                Ok(o) => Some(o),
                Err(e) => {
                    log::warn!("no sound device ({e}); the song plays silently");
                    None
                }
            }
        };
        Ok(Self {
            out,
            silent: Cell::new(Silent {
                speed: 1.0,
                ..Silent::default()
            }),
            started: false,
            clock: Cell::new(Clock::at(Duration::ZERO)),
        })
    }

    fn open() -> Result<Out> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        sink.pause();
        Ok(Out {
            _stream: stream,
            stream_handle,
            sink,
        })
    }

    /// The device's position, or the silent clock's.
    fn get_pos(&self) -> Duration {
        match &self.out {
            Some(o) => o.sink.get_pos(),
            None => self.silent.get().pos(),
        }
    }

    fn is_paused(&self) -> bool {
        match &self.out {
            Some(o) => o.sink.is_paused(),
            None => self.silent.get().since.is_none(),
        }
    }

    #[allow(dead_code)]
    pub fn load(&mut self, path: &Path) -> Result<()> {
        let file = BufReader::new(File::open(path)?);
        let source = Decoder::new(file)?;
        if let Some(o) = &self.out {
            o.sink.append(source);
            o.sink.pause();
        }
        self.started = false;
        Ok(())
    }

    pub fn load_samples(
        &mut self,
        interleaved: Vec<i16>,
        channels: u16,
        sample_rate: u32,
    ) -> Result<()> {
        let frames = interleaved.len() as u64 / channels.max(1) as u64;
        let mut silent = self.silent.get();
        silent.len += Duration::from_secs_f64(frames as f64 / sample_rate.max(1) as f64);
        self.silent.set(silent);
        if let Some(o) = &self.out {
            let buf = SamplesBuffer::new(channels.max(1), sample_rate, interleaved);
            o.sink.append(buf);
            o.sink.pause();
        }
        self.started = false;
        Ok(())
    }

    pub fn play(&mut self) {
        match &self.out {
            Some(o) => o.sink.play(),
            None => {
                let mut silent = self.silent.get().stopped();
                silent.since = Some(Instant::now());
                self.silent.set(silent);
            }
        }
        self.started = true;
        self.clock.set(Clock::at(self.get_pos()));
    }

    pub fn pause(&mut self) {
        match &self.out {
            Some(o) => o.sink.pause(),
            None => self.silent.set(self.silent.get().stopped()),
        }
    }

    pub fn resume(&mut self) {
        let last = self.clock.get().last;
        match &self.out {
            Some(o) => o.sink.play(),
            None => {
                let mut silent = self.silent.get();
                if silent.since.is_none() {
                    silent.since = Some(Instant::now());
                    self.silent.set(silent);
                }
            }
        }
        self.clock.set(Clock {
            last,
            ..Clock::at(self.get_pos())
        });
    }

    pub fn stop(&mut self) {
        match &self.out {
            Some(o) => o.sink.stop(),
            None => {
                let mut silent = self.silent.get();
                silent.since = None;
                silent.pos = silent.len;
                self.silent.set(silent);
            }
        }
        self.started = false;
    }

    pub fn position_ms(&self) -> u64 {
        if !self.started {
            return 0;
        }
        let raw = self.get_pos();
        let mut clock = self.clock.get();
        if raw != clock.raw {
            clock.raw = raw;
            clock.raw_at = Instant::now();
        }
        let estimate = if self.is_paused() {
            clock.raw
        } else {
            clock.raw + clock.raw_at.elapsed().min(MAX_EXTRAPOLATION)
        };
        clock.last = estimate.max(clock.last);
        self.clock.set(clock);
        clock.last.as_millis() as u64
    }

    pub fn update_position(&self) {
        // no-op: position is queried directly from sink
    }

    #[allow(dead_code)]
    pub fn is_playing(&self) -> bool {
        !self.is_paused() && self.started
    }

    pub fn is_finished(&self) -> bool {
        match &self.out {
            Some(o) => o.sink.empty(),
            None => {
                let silent = self.silent.get();
                self.started && silent.pos() >= silent.len
            }
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        if let Some(o) = &self.out {
            o.sink.set_volume(volume);
        }
    }

    /// Jump playback to a wall-clock position. A successful seek may produce
    /// a small (~50ms) audible gap with `SamplesBuffer` — acceptable for
    /// practice-mode looping. Failures are swallowed; the caller can retry.
    pub fn seek_to_ms(&self, ms: u64) -> Result<()> {
        match &self.out {
            Some(o) => o
                .sink
                .try_seek(Duration::from_millis(ms))
                .map_err(|e| anyhow::anyhow!("seek failed: {e}"))?,
            None => {
                let mut silent = self.silent.get();
                silent.pos = Duration::from_millis(ms).min(silent.len);
                if silent.since.is_some() {
                    silent.since = Some(Instant::now());
                }
                self.silent.set(silent);
            }
        }
        self.clock.set(Clock::at(self.get_pos()));
        Ok(())
    }

    /// Set playback speed. `1.0` is normal. Values < 1.0 slow down and drop
    /// pitch; `rodio` does not time-stretch. Pitch change is acceptable for
    /// practice mode.
    pub fn set_speed(&self, speed: f32) {
        match &self.out {
            Some(o) => o.sink.set_speed(speed),
            None => {
                let mut silent = self.silent.get().stopped();
                silent.speed = speed;
                if self.silent.get().since.is_some() {
                    silent.since = Some(Instant::now());
                }
                self.silent.set(silent);
            }
        }
    }

    /// Fire-and-forget SFX on a detached sink sharing the same output device.
    /// Music playback and position are unaffected.
    pub fn play_sfx(&self, samples: Vec<f32>, sample_rate: u32, volume: f32) {
        let Some(o) = &self.out else { return };
        if let Ok(sfx_sink) = Sink::try_new(&o.stream_handle) {
            sfx_sink.set_volume(volume);
            sfx_sink.append(SamplesBuffer::new(1, sample_rate, samples));
            sfx_sink.detach();
        }
    }
}
