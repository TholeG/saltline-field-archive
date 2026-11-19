use super::dsp::{Bands, Click, Clip, Filter, Synth, glide, triangle};
use std::f64::consts::{PI, TAU};

fn click(synth: &mut Synth, modes: &[[f64; 3]], high: f64, tick: f64) -> Clip {
    synth.click(
        modes,
        Click {
            high,
            tick,
            ..Click::default()
        },
    )
}

fn high_pass(mut clip: Clip, cutoff: f64) -> Clip {
    clip.filter(Filter::High, cutoff, 0.707);
    clip
}

fn shaped(mut clip: Clip, drive: f64, cutoff: f64) -> Clip {
    clip.normalize(1.0);
    clip.saturate(drive);
    high_pass(clip, cutoff)
}

struct Scrape {
    duration: f64,
    cutoff: [f64; 2],
    roughness: [f64; 2],
    high_mix: f64,
    q: f64,
}

fn scrape(synth: &mut Synth, spec: Scrape, envelope: impl Fn(f64) -> f64) -> Clip {
    let mut friction = synth.noise(spec.duration);
    let count = friction.len() as f64;
    let [start, end] = spec.cutoff;
    friction.sweep(Filter::Band, glide(start, end, 1.0), spec.q);
    if spec.high_mix > 0.0 {
        let mut harmonics = synth.noise(spec.duration);
        harmonics.sweep(
            Filter::Band,
            glide(start * 2.37, end * 2.37, 1.0),
            spec.q * 1.5,
        );
        friction.mix(&harmonics, 0.0, spec.high_mix);
    }
    let (mut level, mut target, mut next) = (0.5, 0.5, 0.0);
    for (index, sample) in friction.iter_mut().enumerate() {
        let index = index as f64;
        let t = index / count;
        if index >= next {
            target = 0.2 + synth.random() * 0.8;
            let frequency = spec.roughness[0] + (spec.roughness[1] - spec.roughness[0]) * t;
            next = index + synth.rate / frequency * (0.6 + synth.random() * 0.8);
        }
        level += (target - level) * 0.03;
        *sample = (f64::from(*sample) * (level * envelope(t))) as f32;
    }
    friction.normalize(1.0);
    friction
}

fn cloth(
    synth: &mut Synth,
    duration: f64,
    start: f64,
    end: f64,
    envelope: impl Fn(f64) -> f64,
) -> Clip {
    let mut high = synth.noise(duration);
    high.sweep(Filter::Band, glide(start, end, 1.0), 0.7);
    let mut low = synth.noise(duration);
    low.filter(Filter::Low, 420.0, 0.7);
    let length = high.len() as f64;
    let (mut level, mut target, mut next, mut grain, mut grain_target) = (0.4, 0.6, 0.0, 0.5, 0.5);
    for (index, (sample, &under)) in high.iter_mut().zip(low.iter()).enumerate() {
        let position = index as f64;
        if position >= next {
            target = 0.15 + synth.random() * 0.85;
            next = position + synth.rate * (0.006 + synth.random() * 0.016);
        }
        if index % 64 == 0 {
            grain_target = 0.5 + synth.random() * 0.5;
        }
        level += (target - level) * 0.004;
        grain += (grain_target - grain) * 0.05;
        *sample = ((f64::from(*sample) * level * grain + f64::from(under) * 0.9 * level)
            * envelope(position / length)) as f32;
    }
    high.normalize(1.0);
    high
}

fn gear_ticks(output: &mut Clip, synth: &mut Synth, times: &[f64], gain: f64, pitch: f64) {
    for &time in times {
        let pitch = pitch * synth.range(0.85, 1.2);
        let tick = click(
            synth,
            &[
                [pitch, 0.005, 1.0],
                [pitch * 1.47, 0.004, 0.6],
                [pitch * 2.3, 0.003, 0.3],
            ],
            3500.0,
            0.4,
        );
        output.mix(&tick, time, gain * synth.range(0.5, 1.0));
    }
}

pub fn rifle_dry(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.2);
    let start = 0.0005;
    sound.mix(
        &click(
            synth,
            &[
                [2260.0, 0.005, 1.0],
                [3640.0, 0.0045, 0.75],
                [5880.0, 0.003, 0.5],
                [8100.0, 0.002, 0.3],
            ],
            2200.0,
            0.9,
        ),
        start,
        1.0,
    );
    sound.mix(
        &synth.ring(0.05, &[[640.0, 0.009, 1.0], [1180.0, 0.007, 0.6]]),
        start,
        0.3,
    );
    sound.mix(
        &synth.burst(0.02, 0.004, Bands::low(900.0), 0.0),
        start,
        0.22,
    );
    sound.mix(
        &click(
            synth,
            &[[3050.0, 0.003, 1.0], [4700.0, 0.0025, 0.6]],
            3200.0,
            0.5,
        ),
        start + 0.0125,
        0.16,
    );
    sound.mix(
        &synth.ring(
            0.18,
            &[
                [742.0, 0.04, 1.0],
                [1493.0, 0.028, 0.45],
                [2260.0, 0.016, 0.2],
            ],
        ),
        start + 0.002,
        0.045,
    );
    high_pass(sound, 60.0)
}

pub fn pistol_dry(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.14);
    let start = 0.0005;
    sound.mix(
        &click(
            synth,
            &[
                [3480.0, 0.0038, 1.0],
                [5350.0, 0.003, 0.7],
                [7900.0, 0.0022, 0.45],
                [1950.0, 0.005, 0.4],
            ],
            3000.0,
            0.8,
        ),
        start,
        1.0,
    );
    sound.mix(
        &synth.ring(0.03, &[[980.0, 0.006, 1.0], [1620.0, 0.005, 0.5]]),
        start,
        0.22,
    );
    sound.mix(
        &click(
            synth,
            &[[4300.0, 0.0025, 1.0], [6600.0, 0.002, 0.5]],
            3800.0,
            0.5,
        ),
        start + 0.009,
        0.14,
    );
    sound.mix(
        &synth.ring(0.09, &[[1310.0, 0.018, 1.0], [2650.0, 0.012, 0.4]]),
        start + 0.002,
        0.03,
    );
    high_pass(sound, 90.0)
}

pub fn rifle_mag_out(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.38);
    let start = 0.003;
    sound.mix(
        &click(
            synth,
            &[
                [3340.0, 0.004, 1.0],
                [5210.0, 0.003, 0.6],
                [1880.0, 0.006, 0.45],
                [7600.0, 0.002, 0.3],
            ],
            2600.0,
            0.75,
        ),
        start,
        0.62,
    );
    let guide = scrape(
        synth,
        Scrape {
            duration: 0.17,
            cutoff: [2700.0, 1450.0],
            roughness: [210.0, 130.0],
            high_mix: 0.0,
            q: 1.8,
        },
        triangle(0.25, 0.8),
    );
    sound.mix(&guide, start + 0.018, 0.4);
    let mut bright = synth.noise(0.24);
    bright.filter(Filter::High, 4200.0, 0.7);
    bright.envelope(|t| (PI * t / 0.24).sin().powf(2.0));
    bright.normalize(1.0);
    sound.mix(&bright, start + 0.05, 0.075);
    let mut low = synth.noise(0.22);
    low.sweep(Filter::Band, glide(520.0, 260.0, 1.0), 0.8);
    low.envelope(|t| (PI * t / 0.22).sin().powf(2.0));
    low.normalize(1.0);
    sound.mix(&low, start + 0.08, 0.1);
    let stop = start + 0.2;
    sound.mix(
        &synth.ring(
            0.12,
            &[
                [418.0, 0.026, 1.0],
                [1135.0, 0.016, 0.6],
                [2280.0, 0.009, 0.4],
                [3420.0, 0.005, 0.25],
            ],
        ),
        stop,
        0.3,
    );
    sound.mix(
        &synth.burst(0.05, 0.009, Bands::low(1000.0), 0.0),
        stop,
        0.3,
    );
    sound.mix(
        &synth.burst(0.004, 0.0008, Bands::high(2800.0), 0.0),
        stop,
        0.18,
    );
    high_pass(sound, 70.0)
}

pub fn rifle_mag_insert(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.26);
    let start = 0.004;
    sound.mix(
        &click(
            synth,
            &[[2210.0, 0.004, 1.0], [3580.0, 0.003, 0.6]],
            2400.0,
            0.5,
        ),
        start,
        0.3,
    );
    sound.mix(
        &synth.ring(0.06, &[[520.0, 0.014, 1.0], [1240.0, 0.009, 0.5]]),
        start,
        0.26,
    );
    let guide = scrape(
        synth,
        Scrape {
            duration: 0.19,
            cutoff: [1000.0, 3300.0],
            roughness: [140.0, 320.0],
            high_mix: 0.35,
            q: 1.8,
        },
        |t| t.powf(0.8) * if t > 0.93 { (1.0 - t) / 0.07 } else { 1.0 },
    );
    sound.mix(&guide, start + 0.012, 0.62);
    for (time, gain) in [(0.085, 0.14), (0.142, 0.2)] {
        sound.mix(
            &click(
                synth,
                &[[2850.0, 0.003, 1.0], [4420.0, 0.0025, 0.6]],
                3000.0,
                0.6,
            ),
            start + time,
            gain,
        );
    }
    sound.mix(
        &synth.burst(0.03, 0.006, Bands::band(720.0, 1.0), 0.0),
        start + 0.2,
        0.22,
    );
    high_pass(sound, 70.0)
}

pub fn rifle_mag_in(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.21);
    let start = 0.0008;
    let latch = start + 0.031;
    sound.mix(
        &synth.click(
            &[
                [2390.0, 0.0055, 1.0],
                [3760.0, 0.0045, 0.75],
                [5640.0, 0.0035, 0.5],
                [1270.0, 0.007, 0.45],
            ],
            Click {
                high: 2400.0,
                tick: 0.95,
                tick_release: 0.0008,
                ..Click::default()
            },
        ),
        start,
        0.62,
    );
    sound.mix(
        &synth.knock(0.08, 172.0, 74.0, 0.014, 0.02, 0.0008),
        start,
        0.5,
    );
    sound.mix(
        &synth.burst(0.04, 0.008, Bands::low(720.0), 0.0),
        start,
        0.3,
    );
    sound.mix(
        &synth.ring(0.07, &[[432.0, 0.016, 1.0], [915.0, 0.011, 0.6]]),
        start,
        0.2,
    );
    sound.mix(
        &synth.click(
            &[
                [3130.0, 0.011, 1.0],
                [4690.0, 0.009, 0.85],
                [7160.0, 0.006, 0.55],
                [1890.0, 0.012, 0.5],
                [9420.0, 0.0035, 0.3],
            ],
            Click {
                high: 2800.0,
                tick: 1.1,
                tick_release: 0.001,
                ..Click::default()
            },
        ),
        latch,
        1.05,
    );
    sound.mix(
        &synth.burst(0.012, 0.0022, Bands::band(2600.0, 0.8), 0.0),
        latch,
        0.4,
    );
    sound.mix(
        &synth.ring(
            0.16,
            &[
                [1485.0, 0.028, 1.0],
                [2215.0, 0.024, 0.6],
                [3320.0, 0.016, 0.35],
            ],
        ),
        latch + 0.0008,
        0.11,
    );
    shaped(sound, 1.5, 45.0)
}

pub fn rifle_bolt_back(synth: &mut Synth, variant: usize) -> Clip {
    let short = variant == 1;
    let travel = if short { 0.085 } else { 0.15 };
    let mut sound = synth.silent(if short { 0.16 } else { 0.26 });
    let start = 0.001;
    sound.mix(
        &click(
            synth,
            &[
                [2710.0, 0.004, 1.0],
                [4130.0, 0.003, 0.6],
                [1560.0, 0.006, 0.4],
            ],
            2600.0,
            0.7,
        ),
        start,
        if short { 0.4 } else { 0.45 },
    );
    let guide = scrape(
        synth,
        Scrape {
            duration: travel,
            cutoff: [1350.0, if short { 2000.0 } else { 2650.0 }],
            roughness: [260.0, 420.0],
            high_mix: 0.5,
            q: 2.2,
        },
        |t| 0.35 + 0.65 * t,
    );
    sound.mix(&guide, start + 0.006, 0.34);
    let count = if short { 4 } else { 8 };
    for index in 0..count {
        let t = (index as f64 + 0.5) / count as f64;
        let time = start + 0.01 + travel * t.powf(0.85);
        let pitch = 1900.0 + 1500.0 * t;
        let tooth = synth.click(
            &[
                [pitch, 0.003, 1.0],
                [pitch * 1.62, 0.0025, 0.6],
                [pitch * 2.71, 0.0018, 0.3],
            ],
            Click {
                high: 2800.0,
                tick: 0.6,
                jitter: 0.04,
                ..Click::default()
            },
        );
        sound.mix(&tooth, time, 0.2 + 0.2 * t);
    }
    let mut spring = synth.silent(travel);
    let length = spring.len() as f64;
    let mut phase = 0.0;
    for (index, sample) in spring.iter_mut().enumerate() {
        let t = index as f64 / length;
        phase += TAU * (470.0 + 300.0 * t) / synth.rate;
        *sample =
            ((phase.sin() + 0.4 * (2.02 * phase).sin()) * t * ((1.0 - t) * 12.0).min(1.0)) as f32;
    }
    sound.mix(&spring, start + 0.008, 0.055);
    let stop = start + 0.012 + travel;
    if short {
        sound.mix(
            &synth.burst(0.02, 0.004, Bands::band(1500.0, 1.0), 0.0),
            stop,
            0.3,
        );
    } else {
        sound.mix(
            &click(
                synth,
                &[
                    [1660.0, 0.012, 1.0],
                    [2750.0, 0.01, 0.7],
                    [4210.0, 0.006, 0.5],
                    [6300.0, 0.004, 0.3],
                ],
                2000.0,
                1.0,
            ),
            stop,
            0.75,
        );
        sound.mix(
            &synth.knock(0.05, 205.0, 100.0, 0.012, 0.014, 0.0008),
            stop,
            0.35,
        );
    }
    shaped(sound, 1.2, 60.0)
}

fn accelerating_noise(synth: &mut Synth, length: f64, low: f64, high: f64, curve: f64) -> Clip {
    let mut slide = synth.noise(length);
    slide.sweep(Filter::Band, glide(low, high, 1.0), 2.5);
    slide.envelope(|t| (t / length).powf(curve));
    slide.normalize(1.0);
    slide
}

fn jittered_modes<const N: usize>(synth: &mut Synth, modes: [[f64; 3]; N]) -> [[f64; 3]; N] {
    modes.map(|[pitch, release, gain]| [pitch * synth.range(0.985, 1.015), release, gain])
}

pub fn rifle_bolt(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.33);
    let start = 0.0008;
    let stop = start + 0.019;
    sound.mix(
        &click(
            synth,
            &[[2950.0, 0.003, 1.0], [4580.0, 0.0025, 0.6]],
            3000.0,
            0.6,
        ),
        start,
        0.11,
    );
    sound.mix(
        &accelerating_noise(synth, 0.018, 1700.0, 5600.0, 1.6),
        start + 0.001,
        0.16,
    );
    let mut spring = synth.chirp(0.018, 850.0, 2500.0, false);
    spring.envelope(|t| (t / 0.018).powf(2.0));
    sound.mix(&spring, start + 0.001, 0.05);
    sound.mix(
        &synth.burst(0.02, 0.0016, Bands::high(1400.0), 0.0),
        stop,
        1.15,
    );
    sound.mix(
        &synth.burst(0.04, 0.007, Bands::range(500.0, 1900.0), 0.0),
        stop,
        0.95,
    );
    sound.mix(
        &synth.ring(
            0.08,
            &[
                [820.0, 0.012, 1.0],
                [1240.0, 0.011, 0.9],
                [1710.0, 0.009, 0.7],
            ],
        ),
        stop,
        0.4,
    );
    sound.mix(
        &synth.knock(0.07, 255.0, 104.0, 0.01, 0.02, 0.0006),
        stop,
        0.55,
    );
    let modes = jittered_modes(
        synth,
        [
            [2290.0, 0.024, 1.0],
            [3420.0, 0.02, 0.85],
            [4640.0, 0.016, 0.6],
            [2880.0, 0.018, 0.5],
            [6410.0, 0.008, 0.35],
            [8350.0, 0.005, 0.25],
        ],
    );
    sound.mix(&synth.ring(0.2, &modes), stop, 0.46);
    sound.mix(
        &synth.ring(0.28, &[[706.0, 0.07, 1.0], [1421.0, 0.045, 0.35]]),
        stop + 0.002,
        0.07,
    );
    shaped(sound, 1.6, 45.0)
}

pub fn pistol_mag_out(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.27);
    let start = 0.003;
    sound.mix(
        &click(
            synth,
            &[
                [4120.0, 0.0035, 1.0],
                [6340.0, 0.0028, 0.6],
                [2480.0, 0.005, 0.4],
            ],
            3200.0,
            0.7,
        ),
        start,
        0.6,
    );
    let guide = scrape(
        synth,
        Scrape {
            duration: 0.09,
            cutoff: [3100.0, 2050.0],
            roughness: [300.0, 200.0],
            high_mix: 0.4,
            q: 2.4,
        },
        triangle(0.2, 0.8),
    );
    sound.mix(&guide, start + 0.012, 0.42);
    sound.mix(
        &synth.ring(0.1, &[[3320.0, 0.02, 1.0], [5150.0, 0.014, 0.5]]),
        start + 0.02,
        0.05,
    );
    let mut bright = synth.noise(0.14);
    bright.filter(Filter::High, 4500.0, 0.7);
    bright.envelope(|t| (PI * t / 0.14).sin().powf(2.0));
    bright.normalize(1.0);
    sound.mix(&bright, start + 0.04, 0.05);
    sound.mix(
        &click(
            synth,
            &[
                [1720.0, 0.008, 1.0],
                [2960.0, 0.006, 0.6],
                [690.0, 0.012, 0.5],
            ],
            2600.0,
            0.4,
        ),
        start + 0.108,
        0.3,
    );
    high_pass(sound, 90.0)
}

pub fn pistol_mag_insert(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.19);
    let start = 0.003;
    sound.mix(
        &click(
            synth,
            &[[2900.0, 0.0035, 1.0], [4650.0, 0.003, 0.6]],
            2800.0,
            0.5,
        ),
        start,
        0.3,
    );
    let guide = scrape(
        synth,
        Scrape {
            duration: 0.115,
            cutoff: [1650.0, 4300.0],
            roughness: [220.0, 420.0],
            high_mix: 0.4,
            q: 2.2,
        },
        |t| t.powf(0.7) * if t > 0.92 { (1.0 - t) / 0.08 } else { 1.0 },
    );
    sound.mix(&guide, start + 0.01, 0.6);
    sound.mix(
        &click(
            synth,
            &[[3700.0, 0.003, 1.0], [5600.0, 0.0025, 0.5]],
            3200.0,
            0.6,
        ),
        start + 0.07,
        0.16,
    );
    high_pass(sound, 90.0)
}

pub fn pistol_mag_in(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.15);
    let start = 0.0008;
    let latch = start + 0.013;
    sound.mix(
        &click(
            synth,
            &[
                [2620.0, 0.004, 1.0],
                [4050.0, 0.0035, 0.7],
                [1480.0, 0.006, 0.4],
            ],
            2600.0,
            0.8,
        ),
        start,
        0.55,
    );
    sound.mix(
        &synth.knock(0.05, 215.0, 108.0, 0.01, 0.013, 0.0008),
        start,
        0.3,
    );
    sound.mix(
        &synth.burst(0.03, 0.006, Bands::range(350.0, 1600.0), 0.0),
        start,
        0.4,
    );
    sound.mix(
        &synth.ring(0.04, &[[760.0, 0.008, 1.0], [1330.0, 0.007, 0.7]]),
        start,
        0.25,
    );
    sound.mix(
        &click(
            synth,
            &[
                [3610.0, 0.008, 1.0],
                [5420.0, 0.0065, 0.75],
                [8130.0, 0.004, 0.45],
                [2110.0, 0.008, 0.4],
            ],
            3200.0,
            1.0,
        ),
        latch,
        1.0,
    );
    sound.mix(
        &synth.ring(0.08, &[[1920.0, 0.016, 1.0], [2870.0, 0.012, 0.5]]),
        latch + 0.0006,
        0.07,
    );
    shaped(sound, 1.4, 60.0)
}

pub fn pistol_slide(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.2);
    let start = 0.0008;
    let stop = start + 0.012;
    sound.mix(
        &click(
            synth,
            &[[3900.0, 0.0028, 1.0], [5900.0, 0.0022, 0.6]],
            3400.0,
            0.6,
        ),
        start,
        0.13,
    );
    sound.mix(
        &accelerating_noise(synth, 0.011, 2400.0, 7200.0, 1.5),
        start + 0.001,
        0.17,
    );
    sound.mix(
        &synth.burst(0.012, 0.0011, Bands::high(2400.0), 0.0),
        stop,
        1.1,
    );
    sound.mix(
        &synth.burst(0.025, 0.005, Bands::range(700.0, 2400.0), 0.0),
        stop,
        0.8,
    );
    sound.mix(
        &synth.ring(0.05, &[[1180.0, 0.008, 1.0], [1760.0, 0.007, 0.8]]),
        stop,
        0.3,
    );
    sound.mix(
        &synth.knock(0.04, 330.0, 165.0, 0.007, 0.011, 0.0005),
        stop,
        0.32,
    );
    let modes = jittered_modes(
        synth,
        [
            [3060.0, 0.012, 1.0],
            [4490.0, 0.011, 0.85],
            [6920.0, 0.008, 0.6],
            [9350.0, 0.005, 0.35],
            [1710.0, 0.012, 0.4],
        ],
    );
    sound.mix(&synth.ring(0.12, &modes), stop, 0.5);
    sound.mix(
        &synth.ring(0.1, &[[1190.0, 0.022, 1.0], [2400.0, 0.014, 0.4]]),
        stop + 0.002,
        0.05,
    );
    shaped(sound, 1.5, 70.0)
}

pub fn holster(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.31);
    sound.mix(
        &cloth(synth, 0.26, 2500.0, 900.0, triangle(0.32, 1.2)),
        0.004,
        0.7,
    );
    gear_ticks(&mut sound, synth, &[0.055, 0.112, 0.168], 0.2, 4100.0);
    sound.mix(
        &synth.knock(0.07, 125.0, 68.0, 0.02, 0.022, 0.003),
        0.215,
        0.5,
    );
    sound.mix(
        &synth.burst(0.05, 0.012, Bands::low(520.0), 0.0),
        0.215,
        0.45,
    );
    sound.mix(
        &synth.ring(0.05, &[[610.0, 0.012, 1.0], [1340.0, 0.008, 0.5]]),
        0.217,
        0.14,
    );
    high_pass(sound, 50.0)
}

pub fn draw(synth: &mut Synth, _: usize) -> Clip {
    let mut sound = synth.silent(0.29);
    sound.mix(
        &cloth(synth, 0.22, 900.0, 2800.0, triangle(0.5, 1.2)),
        0.004,
        0.65,
    );
    gear_ticks(&mut sound, synth, &[0.04, 0.097, 0.15], 0.17, 4600.0);
    let stop = 0.205;
    sound.mix(
        &synth.ring(
            0.06,
            &[
                [615.0, 0.012, 1.0],
                [1510.0, 0.008, 0.6],
                [2920.0, 0.005, 0.4],
            ],
        ),
        stop,
        0.4,
    );
    sound.mix(
        &synth.burst(0.02, 0.004, Bands::low(1400.0), 0.0),
        stop,
        0.45,
    );
    sound.mix(
        &synth.burst(0.004, 0.0008, Bands::high(2600.0), 0.0),
        stop,
        0.3,
    );
    sound.mix(
        &click(
            synth,
            &[[2480.0, 0.004, 1.0], [3900.0, 0.003, 0.6]],
            2800.0,
            0.5,
        ),
        stop + 0.022,
        0.14,
    );
    high_pass(sound, 60.0)
}

pub fn rustle(synth: &mut Synth, variant: usize) -> Clip {
    let mut sound = synth.silent(0.2);
    let (start, end) = if variant % 2 == 0 {
        (1100.0, 2400.0)
    } else {
        (2300.0, 1200.0)
    };
    sound.mix(
        &cloth(synth, 0.17, start, end, triangle(0.4, 1.3)),
        0.004,
        0.8,
    );
    gear_ticks(
        &mut sound,
        synth,
        &[0.03 + 0.02 * variant as f64, 0.11],
        0.1,
        4300.0,
    );
    high_pass(sound, 80.0)
}

fn shell(synth: &mut Synth, pitch: f64, release: f64) -> Clip {
    let modes = [
        (4180.0, 1.0),
        (6830.0, 0.7),
        (9760.0, 0.5),
        (11900.0, 0.3),
        (2470.0, 0.3),
    ];
    let modes = std::array::from_fn::<_, 5, _>(|i| {
        [
            modes[i].0 * pitch,
            release / (1.0 + i as f64 * 0.35),
            modes[i].1,
        ]
    });
    synth.ring(release * 6.0, &modes)
}

pub fn shell_concrete(synth: &mut Synth, variant: usize) -> Clip {
    let mut sound = synth.silent(0.5);
    let pitch = [1.0, 1.09, 0.93, 1.04][variant & 3];
    let (mut time, mut gain) = (0.0006, 1.0);
    for bounce in 0..if variant == 1 { 2 } else { 3 } {
        let local_pitch =
            pitch * if bounce & 1 != 0 { 1.19 } else { 1.0 } * synth.range(0.97, 1.04);
        let release = synth.range(0.016, 0.03);
        sound.mix(&shell(synth, local_pitch, release), time, gain * 0.5);
        sound.mix(
            &synth.burst(0.004, 0.0008, Bands::high(3200.0), 0.0),
            time,
            gain * 0.5,
        );
        sound.mix(
            &synth.burst(0.012, 0.0025, Bands::band(1900.0, 0.8), 0.0),
            time,
            gain * 0.3,
        );
        time += synth.range(0.085, 0.15) * if bounce == 0 { 1.0 } else { 0.72 };
        gain *= synth.range(0.42, 0.6);
    }
    high_pass(sound, 200.0)
}

pub fn shell_metal(synth: &mut Synth, variant: usize) -> Clip {
    let mut sound = synth.silent(0.7);
    let pitch = [1.0, 1.11, 0.94][variant % 3];
    let (mut time, mut gain) = (0.0006, 1.0);
    for bounce in 0..3 {
        let local_pitch =
            pitch * if bounce & 1 != 0 { 1.19 } else { 1.0 } * synth.range(0.97, 1.04);
        let release = synth.range(0.045, 0.075);
        sound.mix(&shell(synth, local_pitch, release), time, gain * 0.5);
        sound.mix(
            &synth.burst(0.003, 0.0006, Bands::high(4500.0), 0.0),
            time,
            gain * 0.55,
        );
        sound.mix(
            &synth.ring(
                0.3,
                &[
                    [1320.0 * pitch, 0.05, 1.0],
                    [2130.0 * pitch, 0.04, 0.6],
                    [3350.0 * pitch, 0.028, 0.4],
                    [640.0 * pitch, 0.03, 0.3],
                ],
            ),
            time,
            gain * 0.16,
        );
        time += synth.range(0.075, 0.13) * if bounce == 0 { 1.0 } else { 0.7 };
        gain *= synth.range(0.5, 0.66);
    }
    if variant != 1 {
        for bounce in 0..6 {
            let local_pitch = pitch * synth.range(1.0, 1.25);
            sound.mix(&shell(synth, local_pitch, 0.012), time, gain * 0.3);
            time += 0.016 + 0.004 * bounce as f64;
            gain *= 0.75;
        }
    }
    high_pass(sound, 200.0)
}
