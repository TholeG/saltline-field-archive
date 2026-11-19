use super::dsp::{Bands, Clip, Filter, Synth, decay, glide, round};
use std::f64::consts::TAU;

pub fn concrete_step(synth: &mut Synth, variant: usize) -> Clip {
    let pitch = [1.0, 1.07, 0.94, 1.03][variant & 3];
    let mut step = synth.silent(0.3);
    step.mix(
        &synth.knock(0.06, 112.0 * pitch, 58.0 * pitch, 0.018, 0.02, 0.002),
        0.0,
        0.75,
    );
    step.mix(
        &synth.burst(0.06, 0.016, Bands::low(720.0 * pitch), 0.0),
        0.0,
        0.9,
    );
    step.mix(
        &synth.burst(0.1, 0.028, Bands::band(2500.0 * pitch, 0.7), 0.004),
        0.012,
        0.22,
    );
    let mut scuff = synth.burst(0.2, 0.05, Bands::high(3800.0), 0.01);
    let mut grain = 1.0;
    for (index, sample) in scuff.iter_mut().enumerate() {
        if index % 70 == 0 {
            grain = 0.3 + synth.random() * 0.7;
        }
        *sample = (f64::from(*sample) * grain) as f32;
    }
    step.mix(&scuff, 0.02, 0.16);
    step
}

pub fn metal_step(synth: &mut Synth, variant: usize) -> Clip {
    let pitch = [1.0, 1.08, 0.92][variant % 3];
    let mut step = synth.silent(0.38);
    step.mix(
        &synth.knock(0.06, 120.0, 62.0, 0.018, 0.02, 0.002),
        0.0,
        0.6,
    );
    step.mix(&synth.burst(0.04, 0.008, Bands::low(2400.0), 0.0), 0.0, 0.6);
    step.mix(
        &synth.ring(
            0.36,
            &[
                [178.0 * pitch, 0.09, 1.0],
                [412.0 * pitch, 0.075, 0.8],
                [733.0 * pitch, 0.06, 0.55],
                [1214.0 * pitch, 0.045, 0.4],
                [1958.0 * pitch, 0.03, 0.3],
                [3130.0 * pitch, 0.016, 0.18],
            ],
        ),
        0.001,
        0.3,
    );
    step.mix(
        &synth.burst(0.08, 0.02, Bands::band(3000.0, 0.8), 0.003),
        0.01,
        0.1,
    );
    step
}

fn splash(synth: &mut Synth, duration: f64, energy: f64) -> Clip {
    let mut output = synth.silent(duration);
    let mut body = synth.noise(duration * 0.9);
    body.sweep(Filter::Band, glide(1500.0, 520.0, 1.0), 0.8);
    body.envelope(|time| decay(time, 0.085 * energy, 0.007));
    output.mix(&body, 0.0, 0.9);
    output.mix(
        &synth.burst(duration * 0.8, 0.07 * energy, Bands::high(3000.0), 0.012),
        0.01,
        0.25,
    );
    for _ in 0..round(5.0 * energy) as usize {
        let pitch = synth.range(500.0, 1700.0);
        let ripple_length = synth.range(0.008, 0.02);
        let limit = pitch * synth.range(1.3, 1.9);
        let ripple = synth.chirp(ripple_length, pitch, limit, true);
        output.mix(
            &ripple,
            synth.range(0.03, duration * 0.6),
            synth.range(0.1, 0.3),
        );
    }
    output
}

pub fn water_step(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = splash(synth, 0.46, 1.0 + variant as f64 * 0.08);
    output.mix(&synth.knock(0.06, 100.0, 55.0, 0.02, 0.02, 0.002), 0.0, 0.5);
    output
}

pub fn water_impact(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = splash(synth, 0.5, 0.8 + variant as f64 * 0.2);
    output.mix(&synth.chirp(0.035, 420.0, 1150.0, true), 0.0, 0.5);
    output
}

pub fn land(synth: &mut Synth, _: usize) -> Clip {
    let mut output = synth.silent(0.45);
    output.mix(
        &synth.knock(0.14, 124.0, 44.0, 0.03, 0.045, 0.002),
        0.0,
        1.0,
    );
    output.mix(&synth.burst(0.14, 0.035, Bands::low(520.0), 0.0), 0.0, 1.0);
    output.mix(
        &synth.burst(0.2, 0.06, Bands::band(1500.0, 0.7), 0.01),
        0.01,
        0.16,
    );
    for delay in [0.022, 0.05, 0.088] {
        let band = Bands::band(synth.range(2200.0, 4200.0), 2.0);
        let grit = synth.burst(0.012, 0.003, band, 0.0);
        output.mix(&grit, delay, synth.range(0.1, 0.22));
    }
    output.mix(
        &synth.burst(0.2, 0.05, Bands::high(3500.0), 0.01),
        0.015,
        0.1,
    );
    output.saturate(1.2);
    output
}

pub fn jump(synth: &mut Synth, _: usize) -> Clip {
    let mut output = synth.silent(0.26);
    output.mix(
        &synth.burst(0.2, 0.05, Bands::band(900.0, 0.7), 0.02),
        0.0,
        0.6,
    );
    output.mix(
        &synth.burst(0.1, 0.025, Bands::band(2600.0, 0.8), 0.004),
        0.005,
        0.35,
    );
    output.mix(&synth.knock(0.05, 95.0, 60.0, 0.02, 0.018, 0.003), 0.0, 0.4);
    output
}

pub fn hurt(synth: &mut Synth, _: usize) -> Clip {
    let mut output = synth.silent(0.5);
    output.mix(&synth.knock(0.25, 92.0, 38.0, 0.04, 0.06, 0.0015), 0.0, 1.1);
    output.mix(&synth.burst(0.2, 0.04, Bands::low(320.0), 0.0), 0.0, 1.2);
    output.mix(
        &synth.burst(0.06, 0.01, Bands::band(820.0, 1.0), 0.0),
        0.0,
        0.55,
    );
    output.mix(
        &synth.burst(0.03, 0.004, Bands::range(1800.0, 5000.0), 0.0),
        0.0,
        0.2,
    );
    output.saturate(1.6);
    output
}

pub fn ear_ring(synth: &mut Synth, _: usize) -> Clip {
    let mut output = synth.silent(2.4);
    for (index, sample) in output.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        let tones = (TAU * 3150.0 * time).sin()
            + 0.7 * (TAU * 4013.0 * time + 1.0).sin()
            + 0.35 * (TAU * 5310.0 * time + 2.0).sin();
        *sample = (tones * (1.0 - (-time / 0.12).exp()) * (-time / 0.55).exp()) as f32;
    }
    output
}

pub fn metal_impact(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.5);
    let pitch = [640.0, 880.0, 745.0][variant % 3] * synth.range(0.97, 1.03);
    output.mix(
        &synth.burst(0.015, 0.003, Bands::high(1500.0), 0.0),
        0.0,
        1.0,
    );
    output.mix(&synth.burst(0.04, 0.008, Bands::low(1800.0), 0.0), 0.0, 0.6);
    let modes: Vec<_> = [1.0, 1.58, 2.24, 2.92, 3.71, 4.93, 6.31, 8.1]
        .into_iter()
        .enumerate()
        .map(|(index, ratio)| {
            [
                pitch * ratio,
                0.13 / (1.0 + index as f64 * 0.55),
                1.0 / (1.0 + index as f64 * 0.5),
            ]
        })
        .collect();
    output.mix(&synth.ring(0.48, &modes), 0.0005, 0.42);
    output
}

#[derive(Clone, Copy)]
pub struct Debris {
    pub start: f64,
    pub skew: f64,
    pub gain_release: f64,
    pub min_length: f64,
    pub max_length: f64,
    pub bands: Bands,
}

impl Default for Debris {
    fn default() -> Self {
        Self {
            start: 0.0,
            skew: 1.0,
            gain_release: 0.0,
            min_length: 0.001,
            max_length: 0.004,
            bands: Bands::default(),
        }
    }
}

pub fn debris(synth: &mut Synth, duration: f64, count: usize, options: Debris) -> Clip {
    let mut output = synth.silent(duration);
    for _ in 0..count {
        let time =
            options.start + (duration - options.start - 0.02) * synth.random().powf(options.skew);
        let start = round(time * synth.rate) as usize;
        let length =
            round(synth.range(options.min_length, options.max_length) * synth.rate) as usize;
        let gain = synth.range(0.2, 1.0)
            * if options.gain_release != 0.0 {
                (-time / options.gain_release).exp()
            } else {
                1.0
            };
        for (index, sample) in output.iter_mut().skip(start).take(length).enumerate() {
            let burst =
                (synth.random() * 2.0 - 1.0) * gain * (-4.0 * index as f64 / length as f64).exp();
            *sample = (f64::from(*sample) + burst) as f32;
        }
    }
    output.bands(options.bands);
    output
}

pub fn concrete_impact(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.36);
    let pitch = [1.0, 1.1, 0.9][variant % 3];
    output.mix(
        &synth.burst(0.06, 0.013, Bands::low(950.0 * pitch), 0.0),
        0.0,
        1.1,
    );
    output.mix(
        &synth.knock(0.05, 150.0 * pitch, 70.0, 0.012, 0.016, 0.0006),
        0.0,
        0.6,
    );
    output.mix(
        &synth.burst(0.012, 0.002, Bands::high(2000.0), 0.0),
        0.0,
        0.6,
    );
    output.mix(
        &synth.burst(0.12, 0.024, Bands::band(3500.0 * pitch, 0.7), 0.0),
        0.003,
        0.3,
    );
    let grit = debris(
        synth,
        0.34,
        16,
        Debris {
            start: 0.03,
            skew: 1.8,
            gain_release: 0.1,
            bands: Bands::high(1800.0),
            ..Debris::default()
        },
    );
    output.mix(&grit, 0.0, 0.35);
    output
}

pub fn wood_impact(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.26);
    let pitch = if variant != 0 { 1.14 } else { 1.0 };
    output.mix(
        &synth.ring(
            0.2,
            &[
                [238.0 * pitch, 0.03, 1.0],
                [512.0 * pitch, 0.022, 0.8],
                [893.0 * pitch, 0.016, 0.6],
                [1410.0 * pitch, 0.01, 0.4],
            ],
        ),
        0.0,
        0.6,
    );
    output.mix(&synth.burst(0.03, 0.006, Bands::low(2100.0), 0.0), 0.0, 0.8);
    output.mix(
        &synth.burst(0.01, 0.002, Bands::high(3000.0), 0.0),
        0.0,
        0.5,
    );
    let splinters = debris(
        synth,
        0.2,
        6,
        Debris {
            start: 0.01,
            skew: 2.0,
            bands: Bands::band(2600.0, 1.5),
            ..Debris::default()
        },
    );
    output.mix(&splinters, 0.0, 0.3);
    output
}

pub fn enemy_impact(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.3);
    let pitch = [1.0, 1.12, 0.9][variant % 3];
    output.mix(
        &synth.burst(0.03, 0.007, Bands::high(4000.0), 0.0),
        0.0,
        0.9,
    );
    output.mix(
        &synth.ring(
            0.12,
            &[
                [2920.0 * pitch, 0.022, 1.0],
                [4730.0 * pitch, 0.018, 0.8],
                [7310.0 * pitch, 0.012, 0.6],
                [1180.0 * pitch, 0.03, 0.5],
            ],
        ),
        0.0,
        0.4,
    );
    output.mix(
        &synth.burst(0.05, 0.012, Bands::band(320.0 * pitch, 1.5), 0.0),
        0.0,
        0.9,
    );
    let mut spark = synth.noise(0.06);
    spark.filter(Filter::Band, 2400.0, 1.0);
    spark.envelope(|time| {
        (if (TAU * 118.0 * time).sin() > 0.2 {
            1.0
        } else {
            0.1
        }) * (-time / 0.02).exp()
    });
    output.mix(&spark, 0.004, 0.5);
    let grit = debris(
        synth,
        0.28,
        10,
        Debris {
            start: 0.02,
            skew: 1.6,
            gain_release: 0.08,
            bands: Bands::high(5000.0),
            ..Debris::default()
        },
    );
    output.mix(&grit, 0.0, 0.3);
    output
}

pub fn glass_impact(synth: &mut Synth, _: usize) -> Clip {
    let mut output = synth.silent(0.45);
    output.mix(
        &synth.burst(0.01, 0.002, Bands::high(2500.0), 0.0),
        0.0,
        1.0,
    );
    for _ in 0..9 {
        let pitch = synth.range(3200.0, 9500.0);
        let release = synth.range(0.01, 0.03);
        let shard = synth.ring(0.1, &[[pitch, release, 1.0], [pitch * 1.47, 0.01, 0.5]]);
        output.mix(
            &shard,
            synth.random().powf(2.0) * 0.3,
            synth.range(0.1, 0.35),
        );
    }
    output
}

pub fn whiz(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.32);
    let mut air = synth.noise(0.3);
    let high = [5200.0, 4300.0, 6100.0][variant % 3];
    let low = [1300.0, 1000.0, 1700.0][variant % 3];
    let center = 0.05;
    air.sweep(Filter::Band, glide(high, low, 0.8), 3.5);
    air.envelope(|time| {
        let width = if time < center { 0.018 } else { 0.055 };
        let position = (time - center) / width;
        (-position * position).exp()
    });
    air.normalize(1.0);
    output.mix(&air, 0.0, 1.0);
    output.mix(&synth.pressure(0.00018), center - 0.002, 0.5);
    output.mix(
        &synth.burst(0.006, 0.0007, Bands::high(3500.0), 0.0),
        center - 0.002,
        0.55,
    );
    output.mix(&synth.chirp(0.2, high * 0.5, low * 0.6, true), 0.0, 0.03);
    output.filter(Filter::High, 300.0, 0.707);
    output
}

pub fn ricochet(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.75);
    let mut scrape = synth.noise(0.75);
    let start = [3300.0, 2700.0, 3900.0][variant % 3];
    let finish = [820.0, 1100.0, 700.0][variant % 3];
    let mut phase = 0.0;
    for (index, sample) in output.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        let frequency = finish + (start - finish) * (-time / 0.16).exp();
        let modulation = 1.0 + 0.035 * (TAU * (18.0 + 30.0 * time) * time).sin();
        phase += TAU * frequency * modulation / synth.rate;
        *sample = ((phase.sin() + 0.3 * (2.0 * phase).sin())
            * (1.0 - (-time / 0.004).exp())
            * (-time / 0.19).exp()) as f32;
    }
    scrape.sweep(
        Filter::Band,
        |t| finish + (start - finish) * (-t * 0.75 / 0.16).exp(),
        6.0,
    );
    scrape.envelope(|t| decay(t, 0.15, 0.003));
    output.mix(&scrape, 0.0, 1.6);
    output.mix(
        &synth.burst(0.01, 0.002, Bands::high(2500.0), 0.0),
        0.0,
        0.7,
    );
    output
}

pub fn hound_step(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.42);
    let pitch = [1.0, 1.07, 0.93][variant % 3];
    output.mix(
        &synth.chirp(0.035, 920.0 * pitch, 610.0 * pitch, true),
        0.0,
        0.06,
    );
    output.mix(
        &synth.knock(0.2, 84.0 * pitch, 41.0, 0.03, 0.05, 0.002),
        0.03,
        1.1,
    );
    output.mix(&synth.burst(0.12, 0.03, Bands::low(360.0), 0.0), 0.03, 1.1);
    output.mix(
        &synth.burst(0.03, 0.006, Bands::range(500.0, 2600.0), 0.0),
        0.03,
        0.4,
    );
    output.mix(
        &synth.ring(
            0.12,
            &[
                [522.0 * pitch, 0.026, 1.0],
                [1133.0 * pitch, 0.02, 0.7],
                [1871.0 * pitch, 0.014, 0.5],
                [3040.0 * pitch, 0.008, 0.3],
            ],
        ),
        0.03,
        0.2,
    );
    output.mix(
        &synth.burst(0.14, 0.04, Bands::high(3000.0), 0.006),
        0.03 + 0.01,
        0.06,
    );
    output.saturate(1.5);
    output
}

pub fn ui_click(synth: &mut Synth, _: usize) -> Clip {
    let mut output = synth.silent(0.09);
    output.mix(
        &synth.ring(
            0.08,
            &[
                [1820.0, 0.009, 1.0],
                [2730.0, 0.007, 0.6],
                [640.0, 0.012, 0.5],
            ],
        ),
        0.0,
        1.0,
    );
    output.mix(
        &synth.burst(0.004, 0.001, Bands::high(2500.0), 0.0),
        0.0,
        0.5,
    );
    output
}

pub fn ui_hover(synth: &mut Synth, _: usize) -> Clip {
    let mut output = synth.silent(0.06);
    output.mix(
        &synth.ring(0.05, &[[3200.0, 0.008, 1.0], [4800.0, 0.005, 0.3]]),
        0.0,
        1.0,
    );
    output
}
