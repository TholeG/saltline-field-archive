use super::dsp::{Bands, Click, Clip, Filter, Synth, decay, decibels, glide, round};

struct Mechanism {
    delay: f64,
    gain: f64,
    modes: &'static [[f64; 3]],
    high_pass: f64,
}

struct Shot {
    length: f64,
    pitch: &'static [f64],
    brightness: &'static [f64],
    noise: [[f64; 5]; 3],
    hold: f64,
    hold_release: f64,
    drive: f64,
    compression: f64,
    pressure_gain: f64,
    pitch_sweep: [f64; 2],
    pitch_release: f64,
    thump_release: f64,
    thump: f64,
    cutoff_end: f64,
    cutoff_mid: f64,
    cutoff_mid_release: f64,
    cutoff_top_release: f64,
    shelf_frequency: f64,
    shelf: f64,
    plateau: f64,
    mechanism: [Mechanism; 2],
    spring: [[f64; 3]; 2],
    spring_gain: f64,
}

const RIFLE: Shot = Shot {
    length: 0.64,
    pitch: &[1.0, 0.965, 1.035, 0.985, 1.018],
    brightness: &[1.0, 1.07, 0.94, 1.04, 0.97],
    noise: [
        [85.0, 220.0, 1.75, 0.115, 0.125],
        [220.0, 950.0, 2.2, 0.1, 0.052],
        [950.0, 5000.0, 1.5, 0.055, 0.026],
    ],
    hold: 0.022,
    hold_release: 0.0095,
    drive: 1.6,
    compression: 0.0004,
    pressure_gain: 2.5,
    pitch_sweep: [150.0, 62.0],
    pitch_release: 0.03,
    thump_release: 0.055,
    thump: 0.2,
    cutoff_end: 800.0,
    cutoff_mid: 2600.0,
    cutoff_mid_release: 0.07,
    cutoff_top_release: 0.0028,
    shelf_frequency: 230.0,
    shelf: 0.55,
    plateau: 0.8,
    mechanism: [
        Mechanism {
            delay: 0.021,
            gain: 0.05,
            modes: &[
                [2180.0, 0.007, 1.0],
                [3460.0, 0.006, 0.7],
                [1840.0, 0.009, 0.5],
            ],
            high_pass: 3000.0,
        },
        Mechanism {
            delay: 0.058,
            gain: 0.07,
            modes: &[
                [1930.0, 0.009, 1.0],
                [2910.0, 0.007, 0.75],
                [4380.0, 0.005, 0.5],
                [860.0, 0.014, 0.4],
            ],
            high_pass: 2400.0,
        },
    ],
    spring: [[706.0, 0.055, 1.0], [1418.0, 0.035, 0.4]],
    spring_gain: 0.02,
};

const PISTOL: Shot = Shot {
    length: 0.44,
    pitch: &[1.0, 1.04, 0.962, 1.02],
    brightness: &[1.0, 0.95, 1.06, 1.02],
    noise: [
        [100.0, 270.0, 1.3, 0.16, 0.055],
        [270.0, 1400.0, 2.2, 0.2, 0.038],
        [1400.0, 5500.0, 1.8, 0.12, 0.016],
    ],
    hold: 0.01,
    hold_release: 0.005,
    drive: 1.5,
    compression: 0.00045,
    pressure_gain: 2.5,
    pitch_sweep: [230.0, 105.0],
    pitch_release: 0.014,
    thump_release: 0.028,
    thump: 0.25,
    cutoff_end: 1000.0,
    cutoff_mid: 2200.0,
    cutoff_mid_release: 0.04,
    cutoff_top_release: 0.0024,
    shelf_frequency: 260.0,
    shelf: 0.2,
    plateau: 0.78,
    mechanism: [
        Mechanism {
            delay: 0.016,
            gain: 0.09,
            modes: &[
                [2920.0, 0.006, 1.0],
                [4410.0, 0.005, 0.8],
                [1650.0, 0.007, 0.4],
            ],
            high_pass: 3400.0,
        },
        Mechanism {
            delay: 0.046,
            gain: 0.12,
            modes: &[
                [3380.0, 0.008, 1.0],
                [5180.0, 0.007, 0.8],
                [1230.0, 0.01, 0.35],
            ],
            high_pass: 3000.0,
        },
    ],
    spring: [[1180.0, 0.02, 1.0], [2390.0, 0.012, 0.4]],
    spring_gain: 0.015,
};

fn render_shot(synth: &mut Synth, variant: usize, shot: &Shot) -> Clip {
    let rate = synth.rate;
    let pitch = shot.pitch[variant % shot.pitch.len()];
    let brightness = shot.brightness[variant % shot.brightness.len()];
    let mut output = synth.silent(shot.length);
    let length = output.len() - round(0.004 * rate) as usize;
    let mut blast = Clip::with_length(rate, length);
    let hold_frames = round(shot.hold * rate) as usize;
    let hold_fade = (-1.0 / (shot.hold_release * rate)).exp();
    for [low, high, gain, transient, release] in shot.noise {
        let mut band = synth.noise_frames(length);
        band.filter(Filter::High, low * pitch, 0.707);
        band.filter(Filter::High, low * pitch, 0.707);
        band.filter(Filter::Low, high * pitch * brightness, 0.707);
        let rms_squared = band.mean_square(0, length.min(round(0.12 * rate) as usize));
        let correction = 1.0 / (rms_squared + 1e-12).sqrt();
        let transient_fade = (-1.0 / (release * rate)).exp();
        let (mut transient, mut held) = (transient, gain);
        for (index, (sample, &noise)) in blast.iter_mut().zip(band.iter()).enumerate() {
            *sample =
                (f64::from(*sample) + f64::from(noise) * correction * (held + transient)) as f32;
            transient *= transient_fade;
            if index >= hold_frames {
                held *= hold_fade;
            }
        }
    }
    blast.mix(
        &synth.knock(
            0.3,
            shot.pitch_sweep[0] * pitch,
            shot.pitch_sweep[1] * pitch,
            shot.pitch_release,
            shot.thump_release,
            0.001,
        ),
        0.0,
        shot.thump,
    );
    blast.mix(
        &synth.pressure(shot.compression / pitch),
        0.0,
        shot.pressure_gain,
    );
    for sample in blast.iter_mut() {
        *sample = (f64::from(*sample) * shot.drive).tanh() as f32;
    }
    let cutoff = |position: f64| {
        let time = position * length as f64 / rate;
        (shot.cutoff_end + shot.cutoff_mid * (-time / shot.cutoff_mid_release).exp()) * brightness
            + 15000.0 * (-time / shot.cutoff_top_release).exp()
    };
    blast.sweep(Filter::Low, cutoff, 0.6);
    blast.sweep(Filter::Low, cutoff, 0.6);
    let mut bass = blast.clone();
    bass.filter(Filter::Low, shot.shelf_frequency, 0.707);
    blast.mix(&bass, 0.0, shot.shelf);
    blast.filter(Filter::High, 30.0, 0.707);
    let mean = blast.mean_square(
        round(0.004 * rate) as usize,
        round((shot.hold + 0.004) * rate) as usize,
    );
    blast.scale(shot.plateau / (mean + 1e-12).sqrt());
    blast.saturate(1.5);
    let start = 0.0005;
    output.mix(&blast, start, 1.0);
    for (index, part) in shot.mechanism.iter().enumerate() {
        let time = start + part.delay * synth.range(0.94, 1.06);
        let tones: Vec<_> = part
            .modes
            .iter()
            .map(|&[f, t, a]| [f * pitch, t, a])
            .collect();
        output.mix(
            &synth.click(
                &tones,
                Click {
                    high: part.high_pass,
                    tick: 0.6,
                    ring: 0.7,
                    ..Click::default()
                },
            ),
            time,
            part.gain,
        );
        if index == shot.mechanism.len() - 1 {
            let spring = shot.spring.map(|[f, t, a]| [f * pitch, t, a]);
            output.mix(&synth.ring(0.22, &spring), time + 0.002, shot.spring_gain);
        }
    }
    output
}

pub fn rifle(synth: &mut Synth, variant: usize) -> Clip {
    render_shot(synth, variant, &RIFLE)
}

pub fn pistol(synth: &mut Synth, variant: usize) -> Clip {
    render_shot(synth, variant, &PISTOL)
}

struct Outdoor {
    length: f64,
    reference: f64,
    ceiling: f64,
    slaps: &'static [&'static [[f64; 5]]],
    slap_hold: f64,
    slap_release: f64,
    cutoff: [f64; 2],
    early_level: f64,
    floor_level: f64,
    tail_release: f64,
    far_time: f64,
    far_level: f64,
    rumble: f64,
}

const RIFLE_OUTDOOR: Outdoor = Outdoor {
    length: 2.2,
    reference: 0.7,
    ceiling: 0.75,
    slaps: &[
        &[
            [0.078, -11.6, 4400.0, -0.8, 2.7],
            [0.124, -11.6, 3400.0, 0.7, 3.6],
            [0.171, -12.0, 2500.0, -0.3, 2.2],
            [0.262, -19.0, 1500.0, 0.5, 4.1],
        ],
        &[
            [0.084, -11.4, 4200.0, 0.8, 3.1],
            [0.131, -11.6, 3600.0, -0.6, 2.4],
            [0.177, -12.0, 2400.0, 0.35, 3.9],
            [0.286, -19.0, 1400.0, -0.5, 2.9],
        ],
    ],
    slap_hold: 0.024,
    slap_release: 0.009,
    cutoff: [3600.0, 600.0],
    early_level: -15.0,
    floor_level: -14.1,
    tail_release: 0.8,
    far_time: 0.42,
    far_level: -18.0,
    rumble: -22.0,
};

const PISTOL_OUTDOOR: Outdoor = Outdoor {
    length: 1.5,
    reference: 0.53,
    ceiling: 0.6,
    slaps: &[&[
        [0.076, -11.8, 5200.0, -0.7, 2.1],
        [0.121, -12.0, 3900.0, 0.75, 2.9],
        [0.168, -12.8, 2800.0, -0.25, 3.4],
    ]],
    slap_hold: 0.015,
    slap_release: 0.008,
    cutoff: [4400.0, 700.0],
    early_level: -15.0,
    floor_level: -14.6,
    tail_release: 0.55,
    far_time: 0.34,
    far_level: -22.0,
    rumble: -28.0,
};

fn outdoor(synth: &mut Synth, variant: usize, config: &Outdoor) -> [Clip; 2] {
    let seed = (synth.random() * 1e9).floor() as u32;
    let rate = synth.rate;
    let sample = |time: f64| round(time * rate) as usize;
    let mut rng = Synth::new(rate, seed);
    let mut reflections = Vec::new();
    for &[time, level, cutoff, pan, comb] in config.slaps[variant % config.slaps.len()] {
        let mut clip = rng.noise(0.16);
        clip.filter(Filter::High, 300.0, 0.7);
        clip.filter(Filter::Low, cutoff, 0.7);
        clip.envelope(|t| {
            (1.0 - (-t / 0.002).exp())
                * if t < config.slap_hold {
                    1.0
                } else {
                    (-(t - config.slap_hold) / config.slap_release).exp()
                }
        });
        clip.normalize(1.0);
        clip.comb(comb / 1000.0, 0.5, 0.7);
        let modes = [
            [rng.range(110.0, 190.0), 0.05, 1.0],
            [rng.range(240.0, 420.0), 0.035, 0.6],
        ];
        clip.mix(&rng.ring(0.1, &modes), 0.002, 0.12);
        clip.scale(config.reference * decibels(level) / clip.peak_rms(0.01, 0.002));
        reflections.push((clip, time - 0.007, pan));
    }
    let bed = rng.noise(config.length);
    let mut far = rng.burst(0.6, 0.14, Bands::range(80.0, 650.0), 0.04);
    far.scale(config.reference * decibels(config.far_level) / far.peak_rms(0.01, 0.002));
    let mut rumble = rng.noise(config.length - 0.1);
    rumble.filter(Filter::Low, 130.0, 0.7);
    rumble.filter(Filter::High, 34.0, 0.7);
    rumble.envelope(|t| (1.0 - (-t / 0.05).exp()) * (-(t / config.tail_release).powf(2.0)).exp());
    rumble.scale(
        config.reference * decibels(config.rumble)
            / rumble.rms((0.1 * rate) as usize, (0.3 * rate) as usize),
    );

    std::array::from_fn(|channel| {
        let mut rng = Synth::new(rate, seed + 101 + channel as u32 * 977);
        let mut output = rng.silent(config.length);
        let separate = rng.noise(config.length);
        let mut blend = rng.silent(config.length);
        // Both sources are added at f64 precision before storing the stereo noise.
        for (index, sample) in blend.iter_mut().enumerate() {
            *sample = (f64::from(bed[index]) * 0.72 + f64::from(separate[index]) * 0.68) as f32;
        }
        blend.sweep(
            Filter::Low,
            glide(config.cutoff[0], config.cutoff[1], 0.55),
            0.6,
        );
        blend.filter(Filter::High, 140.0, 0.7);
        let mut early = blend.clone();
        early.envelope(|t| decay(t, 0.03, 0.003));
        blend
            .envelope(|t| (1.0 - (-t / 0.03).exp()) * (-(t / config.tail_release).powf(2.0)).exp());
        output.mix(
            &early,
            0.004,
            config.reference * decibels(config.early_level) / early.rms(0, (0.05 * rate) as usize),
        );
        output.mix(
            &blend,
            0.004,
            config.reference * decibels(config.floor_level) / blend.rms(sample(0.1), sample(0.2)),
        );
        let side = if channel == 0 { -1.0 } else { 1.0 };
        for (reflection, time, pan) in &reflections {
            let offset = if pan * side > 0.0 {
                0.0
            } else {
                pan.abs() * 0.0009
            };
            output.mix(reflection, time + offset, 1.0 + 0.4 * pan * side);
        }
        let (delay, gain) = if channel == 0 {
            (0.0, 1.1)
        } else {
            (0.0007, 0.85)
        };
        output.mix(&far, config.far_time + delay, gain);
        output.mix(&rumble, 0.03, 1.0);
        output.ceiling(config.ceiling);
        output
    })
}

pub fn rifle_outdoor(synth: &mut Synth, variant: usize) -> [Clip; 2] {
    outdoor(synth, variant, &RIFLE_OUTDOOR)
}

pub fn pistol_outdoor(synth: &mut Synth, variant: usize) -> [Clip; 2] {
    outdoor(synth, variant, &PISTOL_OUTDOOR)
}

struct Indoor {
    length: f64,
    reference: f64,
    ceiling: f64,
    cutoff: [f64; 2],
    release: f64,
    early: f64,
    level: f64,
    modes: f64,
}

const RIFLE_INDOOR: Indoor = Indoor {
    length: 1.3,
    reference: 0.7,
    ceiling: 0.7,
    cutoff: [7800.0, 1300.0],
    release: 0.15,
    early: -13.0,
    level: -9.0,
    modes: -21.0,
};
const PISTOL_INDOOR: Indoor = Indoor {
    length: 0.9,
    reference: 0.53,
    ceiling: 0.55,
    cutoff: [8500.0, 1700.0],
    release: 0.105,
    early: -13.0,
    level: -10.0,
    modes: -27.0,
};

fn indoor(synth: &mut Synth, variant: usize, config: &Indoor) -> [Clip; 2] {
    let seed = (synth.random() * 1e9).floor() as u32;
    let rate = synth.rate;
    let sample = |time: f64| round(time * rate) as usize;
    let mut rng = Synth::new(rate, seed);
    let shared = rng.noise(config.length);
    let taps = std::array::from_fn::<_, 18, _>(|index| {
        [
            0.006 + 0.085 * ((index as f64 + rng.random()) / 18.0).powf(1.3),
            0.9_f64.powf(index as f64) * rng.range(0.6, 1.1),
            rng.random() * 2.0 - 1.0,
            rng.range(3200.0, 7500.0),
        ]
    });
    let spacing = if variant & 1 != 0 { 0.051 } else { 0.046 };

    std::array::from_fn(|channel| {
        let mut rng = Synth::new(rate, seed + 211 + channel as u32 * 613);
        let side = if channel == 0 { -1.0 } else { 1.0 };
        let mut output = rng.silent(config.length);
        let mut early = rng.silent(0.5);
        for [time, gain, pan, cutoff] in taps {
            early.mix(
                &rng.burst(0.03, 0.005, Bands::range(350.0, cutoff), 0.0004),
                time + if pan * side > 0.0 { 0.0 } else { 0.0006 },
                gain * (1.0 + 0.4 * pan * side),
            );
        }
        for index in 1..8 {
            let index = f64::from(index);
            early.mix(
                &rng.burst(
                    0.04,
                    0.007,
                    Bands::range(600.0, 5200.0 / (1.0 + index * 0.3)),
                    0.0,
                ),
                spacing * index * if channel == 0 { 0.99 } else { 1.012 },
                0.45 * 0.72_f64.powf(index),
            );
        }
        output.mix(
            &early,
            0.0,
            config.reference * decibels(config.early) / early.rms(0, (0.05 * rate) as usize),
        );
        let separate = rng.noise(config.length);
        let mut blend = rng.silent(config.length);
        for (index, sample) in blend.iter_mut().enumerate() {
            *sample = (f64::from(shared[index]) * 0.7 + f64::from(separate[index]) * 0.7) as f32;
        }
        blend.sweep(
            Filter::Low,
            glide(config.cutoff[0], config.cutoff[1], 0.5),
            0.6,
        );
        blend.filter(Filter::High, 170.0, 0.7);
        blend.envelope(|t| decay(t, config.release, 0.006));
        output.mix(
            &blend,
            0.004,
            config.reference * decibels(config.level) / blend.rms(sample(0.1), sample(0.2)),
        );
        for (frequency, gain) in [(57.0, 1.0), (86.0, 0.8), (123.0, 0.6), (171.0, 0.4)] {
            let mut mode = rng.noise((config.length - 0.05).min(0.9));
            mode.filter(
                Filter::Band,
                frequency * if variant & 1 != 0 { 1.05 } else { 1.0 },
                8.0,
            );
            mode.envelope(|t| decay(t, config.release * 1.25, 0.02));
            output.mix(
                &mode,
                0.01,
                gain * config.reference * decibels(config.modes)
                    / mode.rms(sample(0.1), sample(0.2)),
            );
        }
        output.ceiling(config.ceiling);
        output
    })
}

pub fn rifle_indoor(synth: &mut Synth, variant: usize) -> [Clip; 2] {
    indoor(synth, variant, &RIFLE_INDOOR)
}

pub fn pistol_indoor(synth: &mut Synth, variant: usize) -> [Clip; 2] {
    indoor(synth, variant, &PISTOL_INDOOR)
}
