//! The real-time mixing core.

use rtrb::Consumer;
use ss_core::mixer::{ChannelId, Fader};
use ss_dsp::{ChannelStrip, ChatMixGains, Format, Gain, Processor, chatmix_gains};

use crate::limiter::soft_clip;
use crate::{COMMAND_QUEUE_CAPACITY, Command, upmix};

/// Audio of one mixer channel for the current callback (interleaved).
#[derive(Debug, Clone, Copy)]
pub struct ChannelInput<'a> {
    /// Which mixer channel this audio belongs to.
    pub channel: ChannelId,
    /// Interleaved samples.
    pub samples: &'a [f32],
    /// Channel count of `samples`.
    pub channels: usize,
}

/// Mixes up to four channel inputs into one output buffer.
///
/// Real-time safe: [`MixEngine::process`] never allocates, locks or blocks.
pub struct MixEngine {
    format: Format,
    max_frames: usize,
    strips: [ChannelStrip; ChannelId::COUNT],
    /// Per-channel fader × ChatMix gain, smoothed.
    levels: [Gain; ChannelId::COUNT],
    faders: [Fader; ChannelId::COUNT],
    chatmix: ChatMixGains,
    master: Gain,
    scratch: Vec<f32>,
    commands: Consumer<Command>,
}

impl MixEngine {
    pub(crate) fn new(format: Format, max_frames: usize, commands: Consumer<Command>) -> Self {
        Self {
            format,
            max_frames,
            strips: std::array::from_fn(|_| ChannelStrip::new(format)),
            levels: std::array::from_fn(|_| Gain::new(format)),
            faders: [Fader::UNITY; ChannelId::COUNT],
            chatmix: ChatMixGains {
                game: 1.0,
                chat: 1.0,
            },
            master: Gain::new(format),
            scratch: vec![0.0; max_frames * format.channels()],
            commands,
        }
    }

    /// Output format of the engine.
    #[must_use]
    pub const fn format(&self) -> Format {
        self.format
    }

    /// Mixes `inputs` into `output` (interleaved, engine channel count).
    ///
    /// * Each channel should appear at most once; duplicates are ignored.
    /// * At most `max_frames` frames are produced; any remaining output is silenced.
    /// * Shorter inputs are padded with silence.
    pub fn process(&mut self, inputs: &[ChannelInput<'_>], output: &mut [f32]) {
        self.apply_pending_commands();
        output.fill(0.0);

        let out_channels = self.format.channels();
        let frames = (output.len() / out_channels).min(self.max_frames);
        let block = frames * out_channels;
        let mut seen = [false; ChannelId::COUNT];

        for input in inputs {
            let index = input.channel.index();
            if seen[index] || input.channels == 0 {
                continue;
            }
            seen[index] = true;

            let scratch = &mut self.scratch[..block];
            scratch.fill(0.0);
            let input_frames = frames.min(input.samples.len() / input.channels);
            upmix::convert(
                input.samples,
                input.channels,
                scratch,
                out_channels,
                input_frames,
            );
            self.strips[index].process(scratch);
            self.levels[index].process(scratch);
            for (out, sample) in output[..block].iter_mut().zip(scratch.iter()) {
                *out += *sample;
            }
        }

        let mixed = &mut output[..block];
        self.master.process(mixed);
        for sample in mixed {
            *sample = soft_clip(*sample);
        }
    }

    fn apply_pending_commands(&mut self) {
        // Bounded so a flooding producer cannot stall the callback.
        for _ in 0..COMMAND_QUEUE_CAPACITY {
            let Ok(command) = self.commands.pop() else {
                break;
            };
            self.apply(command);
        }
    }

    fn apply(&mut self, command: Command) {
        match command {
            Command::SetStrip(channel, settings) => self.strips[channel.index()].set(&settings),
            Command::SetFader(channel, fader) => {
                self.faders[channel.index()] = fader;
                self.update_level(channel);
            }
            Command::SetMaster(fader) => self.master.set_linear(fader.effective_gain()),
            Command::SetChatMix(balance) => {
                self.chatmix = chatmix_gains(balance);
                self.update_level(ChannelId::Game);
                self.update_level(ChannelId::Chat);
            }
        }
    }

    fn update_level(&mut self, channel: ChannelId) {
        let chatmix = match channel {
            ChannelId::Game => self.chatmix.game,
            ChannelId::Chat => self.chatmix.chat,
            ChannelId::Media | ChannelId::Aux => 1.0,
        };
        let gain = self.faders[channel.index()].effective_gain() * chatmix;
        self.levels[channel.index()].set_linear(gain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EngineController, create};
    use ss_core::headset::{ChatMixBalance, Percent};

    const FRAMES: usize = 256;
    /// Callbacks needed for 10 ms gain smoothing to settle (~0.5 s at 48 kHz).
    const SETTLE_CALLBACKS: usize = 100;

    fn engine() -> (MixEngine, EngineController) {
        create(48_000.0, 2, FRAMES).unwrap()
    }

    /// Runs several callbacks with constant inputs and returns the last left-channel sample.
    fn settle(engine: &mut MixEngine, inputs: &[(ChannelId, f32)]) -> f32 {
        let buffers: Vec<Vec<f32>> = inputs.iter().map(|(_, v)| vec![*v; FRAMES * 2]).collect();
        let mut output = vec![0.0; FRAMES * 2];
        for _ in 0..SETTLE_CALLBACKS {
            let channel_inputs: Vec<ChannelInput<'_>> = inputs
                .iter()
                .zip(&buffers)
                .map(|((channel, _), samples)| ChannelInput {
                    channel: *channel,
                    samples,
                    channels: 2,
                })
                .collect();
            engine.process(&channel_inputs, &mut output);
        }
        output[FRAMES * 2 - 2]
    }

    fn balance(game: u8, chat: u8) -> ChatMixBalance {
        ChatMixBalance {
            game: Percent::new(game).unwrap(),
            chat: Percent::new(chat).unwrap(),
        }
    }

    #[test]
    fn silence_without_inputs() {
        let (mut engine, _) = engine();
        assert!(settle(&mut engine, &[]).abs() < f32::EPSILON);
    }

    #[test]
    fn channels_are_summed() {
        let (mut engine, _) = engine();
        let out = settle(
            &mut engine,
            &[(ChannelId::Game, 0.2), (ChannelId::Media, 0.3)],
        );
        assert!((out - 0.5).abs() < 1e-3, "{out}");
    }

    #[test]
    fn chatmix_attenuates_only_chat_side() {
        let (mut engine, mut control) = engine();
        control.send(Command::SetChatMix(balance(100, 50))).unwrap();
        let chat = settle(&mut engine, &[(ChannelId::Chat, 0.4)]);
        assert!(
            (chat - 0.1).abs() < 1e-3,
            "chat at 50 % → 0.25 gain, got {chat}"
        );
        let media = settle(&mut engine, &[(ChannelId::Media, 0.4)]);
        assert!((media - 0.4).abs() < 1e-3, "media unaffected, got {media}");
    }

    #[test]
    fn fader_mute_and_master() {
        let (mut engine, mut control) = engine();
        control
            .send(Command::SetFader(
                ChannelId::Game,
                Fader {
                    gain: 0.5,
                    muted: false,
                },
            ))
            .unwrap();
        control
            .send(Command::SetMaster(Fader {
                gain: 0.5,
                muted: false,
            }))
            .unwrap();
        let out = settle(&mut engine, &[(ChannelId::Game, 0.8)]);
        assert!((out - 0.2).abs() < 1e-3, "{out}");
        control
            .send(Command::SetFader(
                ChannelId::Game,
                Fader {
                    gain: 1.0,
                    muted: true,
                },
            ))
            .unwrap();
        assert!(settle(&mut engine, &[(ChannelId::Game, 0.8)]).abs() < 1e-4);
    }

    #[test]
    fn output_is_limited_and_duplicates_ignored() {
        let (mut engine, _) = engine();
        let out = settle(
            &mut engine,
            &[
                (ChannelId::Game, 0.9),
                (ChannelId::Chat, 0.9),
                (ChannelId::Game, 0.9),
            ],
        );
        assert!(out <= 1.0 && out > 0.9, "{out}");
    }

    #[test]
    fn oversized_output_is_truncated_and_silenced() {
        let (mut engine, _) = engine();
        let input = vec![0.5; FRAMES * 4];
        let mut output = vec![1.0; FRAMES * 4];
        engine.process(
            &[ChannelInput {
                channel: ChannelId::Media,
                samples: &input,
                channels: 2,
            }],
            &mut output,
        );
        assert!(output[FRAMES * 2..].iter().all(|s| *s == 0.0));
    }

    #[test]
    fn queue_reports_busy_when_full() {
        let (_engine, mut control) = engine();
        let command = Command::SetMaster(Fader::UNITY);
        for _ in 0..crate::COMMAND_QUEUE_CAPACITY {
            control.send(command).unwrap();
        }
        assert!(control.send(command).is_err());
    }
}
