use super::effects::{Reverb, level, limit, loop_filter, reverb};
use super::instruments::{big_drum, drum, gong, midi, musical_noise};
use super::{BAR, STEP};
use crate::dsp::{Bands, Clip, Filter, Synth, round};
use std::cell::RefCell;

struct DrumKit {
    rate: f64,
    drums: [[Clip; 2]; 3],
    big: Clip,
    steel: Clip,
}

thread_local! {
    static KIT: RefCell<Option<DrumKit>> = const { RefCell::new(None) };
}

impl DrumKit {
    fn new(rate: f64) -> Self {
        let mut synth = Synth::new(rate, (23063.0 + rate) as u32);
        let room = Reverb {
            tail: 2.3,
            size: 1.25,
            damping: 0.3,
            pre_delay: 0.022,
            high_pass: 170.0,
        };
        let render = |mut clip: Clip, duration, wet, room| {
            clip.samples.resize(round(duration * rate) as usize, 0.0);
            reverb(&mut clip, wet, room);
            clip
        };
        // Generate each velocity across all three pitches in the same order.
        let mut toms = |strength| {
            [
                render(drum(&mut synth, 55.0, strength), 2.0, 0.3, room),
                render(drum(&mut synth, 73.42, strength), 1.8, 0.28, room),
                render(drum(&mut synth, 110.0, strength), 1.6, 0.26, room),
            ]
        };
        let [low_soft, mid_soft, high_soft] = toms(0.55);
        let [low_hard, mid_hard, high_hard] = toms(1.0);
        let big = render(
            big_drum(&mut synth, 1.0),
            3.6,
            0.55,
            Reverb {
                tail: 3.2,
                size: 1.5,
                damping: 0.35,
                pre_delay: 0.03,
                high_pass: 150.0,
            },
        );
        let steel = render(
            gong(&mut synth, midi(57.0), 1.6, 0.8),
            3.0,
            0.5,
            Reverb {
                tail: 2.8,
                size: 1.4,
                damping: 0.22,
                pre_delay: 0.025,
                high_pass: 250.0,
            },
        );
        Self {
            rate,
            drums: [
                [low_soft, low_hard],
                [mid_soft, mid_hard],
                [high_soft, high_hard],
            ],
            big,
            steel,
        }
    }
}

fn kit<Result>(rate: f64, use_kit: impl FnOnce(&DrumKit) -> Result) -> Result {
    KIT.with_borrow_mut(|kit| {
        if kit.as_ref().is_none_or(|kit| kit.rate != rate) {
            *kit = Some(DrumKit::new(rate));
        }
        use_kit(kit.as_ref().unwrap())
    })
}

pub fn drums(synth: &mut Synth, _: usize) -> Clip {
    let mut clip = synth.silent(8.0 * BAR);
    kit(synth.rate, |kit| {
        let [low, mid, high] = &kit.drums;
        for bar in 0..8 {
            let start = bar as f64 * BAR;
            let ending = bar == 7;
            clip.mix_loop(&low[1], start, 0.62);
            clip.mix_loop(&low[1], start + 8.0 * STEP, if ending { 0.4 } else { 0.5 });
            if !ending {
                clip.mix_loop(&mid[0], start + 3.0 * STEP, 0.3);
                clip.mix_loop(&mid[1], start + 6.0 * STEP, 0.4);
                if bar % 2 == 1 {
                    clip.mix_loop(&high[1], start + 12.0 * STEP, 0.36);
                    clip.mix_loop(&high[0], start + 13.0 * STEP, 0.2);
                    clip.mix_loop(&mid[1], start + 14.0 * STEP, 0.44);
                    if bar == 3 {
                        clip.mix_loop(&low[0], start + 15.0 * STEP, 0.3);
                    }
                } else {
                    clip.mix_loop(&high[0], start + 11.0 * STEP, 0.3);
                    clip.mix_loop(&mid[1], start + 14.0 * STEP, 0.42);
                }
            }
            if bar == 0 || bar == 4 {
                clip.mix_loop(&kit.big, start, if bar == 0 { 0.6 } else { 0.5 });
            }
            if bar == 2 || bar == 6 {
                clip.mix_loop(
                    &kit.steel,
                    start + 8.0 * STEP,
                    if bar == 2 { 0.2 } else { 0.16 },
                );
            }
        }
    });
    loop_filter(&mut clip, Filter::High, 38.0, 0.707, 0.6);
    limit(&mut clip, 1.15);
    level(std::slice::from_mut(&mut clip), 0.122);
    clip
}

pub fn fill(synth: &mut Synth, variant: usize) -> Clip {
    let mut clip = synth.silent(BAR + 1.1);
    kit(synth.rate, |kit| {
        let mut strike = |family: usize, strong: usize, position, gain| {
            clip.mix(&kit.drums[family][strong], position * STEP, gain);
        };
        match variant {
            0 => {
                for hit in 0..8 {
                    strike(
                        usize::from(hit < 4),
                        1 - hit % 2,
                        8.0 + hit as f64,
                        0.28 + 0.06 * hit as f64,
                    );
                }
                strike(2, 1, 6.0, 0.4);
            }
            1 => {
                for (family, velocity, position, gain) in [
                    (2, 1, 0.0, 0.4),
                    (2, 0, 2.0, 0.26),
                    (2, 1, 3.0, 0.42),
                    (1, 1, 6.0, 0.46),
                    (1, 0, 8.0, 0.3),
                    (1, 1, 9.0, 0.48),
                    (0, 1, 12.0, 0.56),
                    (0, 0, 14.0, 0.4),
                    (0, 1, 15.0, 0.6),
                ] {
                    strike(family, velocity, position, gain);
                }
            }
            _ => {
                for (family, velocity, position, gain) in [
                    (0, 1, 8.0, 0.5),
                    (1, 1, 11.0, 0.5),
                    (2, 1, 12.0, 0.4),
                    (2, 1, 13.0, 0.46),
                    (0, 1, 14.0, 0.62),
                    (0, 0, 15.0, 0.42),
                ] {
                    strike(family, velocity, position, gain);
                }
                clip.mix(&kit.steel, 4.0 * STEP, 0.2);
            }
        }
    });
    let fade = round(0.35 * synth.rate) as usize;
    for (index, sample) in clip.iter_mut().rev().take(fade).rev().enumerate() {
        *sample = (f64::from(*sample) * (1.0 - index as f64 / fade as f64)) as f32;
    }
    limit(&mut clip, 1.1);
    clip
}

pub fn ticks(synth: &mut Synth, _: usize) -> Clip {
    let rate = synth.rate;
    let mut clip = synth.silent(6.0 * BAR);
    let mut first = synth.resonance(
        0.06,
        &[
            [2349.3, 0.011, 1.0],
            [3520.0, 0.006, 0.4],
            [5870.0, 0.004, 0.2],
        ],
        false,
    );
    let mut second = synth.resonance(
        0.06,
        &[
            [1760.0, 0.012, 1.0],
            [2637.0, 0.007, 0.35],
            [4186.0, 0.004, 0.15],
        ],
        false,
    );
    first.mix(
        &musical_noise(synth, rate, 0.004, 0.001, Bands::high(3000.0), 0.0),
        0.0,
        0.5,
    );
    second.mix(
        &musical_noise(synth, rate, 0.004, 0.001, Bands::high(2500.0), 0.0),
        0.0,
        0.4,
    );
    let texture = std::array::from_fn::<_, 6, _>(|index| {
        musical_noise(
            synth,
            rate,
            0.07,
            0.012 + 0.012 * (index % 3) as f64,
            Bands::high(5200.0 + 900.0 * (index % 2) as f64),
            0.0012,
        )
    });
    let bell = gong(synth, midi(86.0), 0.5, 0.7);
    const PATTERNS: [[f64; 16]; 3] = [
        [
            0.9, 0.3, 0.45, 0.7, 0.28, 0.4, 0.75, 0.3, 0.85, 0.3, 0.45, 0.7, 0.3, 0.42, 0.72, 0.4,
        ],
        [
            0.9, 0.25, 0.5, 0.65, 0.3, 0.3, 0.8, 0.45, 0.8, 0.25, 0.3, 0.75, 0.35, 0.5, 0.7, 0.55,
        ],
        [
            0.85, 0.35, 0.4, 0.75, 0.25, 0.5, 0.7, 0.25, 0.9, 0.4, 0.5, 0.65, 0.25, 0.35, 0.8, 0.6,
        ],
    ];
    for bar in 0..6 {
        for beat in 0..16 {
            let time = bar as f64 * BAR + beat as f64 * STEP;
            match beat % 4 {
                0 => clip.mix_loop(&first, time, if beat == 0 { 0.42 } else { 0.32 }),
                2 => clip.mix_loop(&second, time, 0.24),
                _ => {}
            }
            let shift = if bar >= 3 { 8 } else { 0 };
            let volume = PATTERNS[bar % 3][(beat + shift) % 16] * (0.9 + 0.2 * synth.random());
            let swing = if beat % 2 == 1 { 0.01 } else { 0.0 };
            clip.mix_loop(&texture[(beat * 5 + bar) % 6], time + swing, 0.2 * volume);
        }
    }
    clip.mix_loop(&bell, 2.0 * BAR + 14.0 * STEP, 0.1);
    clip.mix_loop(&bell, 5.0 * BAR + 11.0 * STEP, 0.08);
    loop_filter(&mut clip, Filter::High, 1100.0, 0.707, 0.4);
    level(std::slice::from_mut(&mut clip), 0.03);
    clip
}
