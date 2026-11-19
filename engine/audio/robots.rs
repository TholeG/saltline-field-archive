use super::dsp::{Bands, Click, Clip, Filter, Synth, decay, glide, round};
use super::foley::{Debris, debris};
use std::f64::consts::TAU;

fn sign(value: f64) -> f64 {
    if value == 0.0 { value } else { value.signum() }
}

pub fn gull_fire(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.26);
    let pitch = [1.0, 1.08, 0.93][variant % 3];
    let start = 0.0005;
    output.mix(
        &synth.burst(0.006, 0.0009, Bands::high(4200.0), 0.0),
        start,
        0.9,
    );
    let mut discharge = synth.silent(0.085);
    let mut phase = 0.0;
    for (index, sample) in discharge.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        let frequency = (520.0 + 2300.0 * (-time / 0.014).exp()) * pitch;
        phase += frequency / synth.rate;
        let pulse = sign((TAU * phase).sin()) * 0.6
            + (2.0 * (phase * 1.5 - (phase * 1.5).floor()) - 1.0) * 0.5;
        *sample =
            (pulse * (0.75 + 0.25 * (TAU * 310.0 * time).sin()) * (-time / 0.02).exp()) as f32;
    }
    discharge.filter(Filter::High, 700.0 * pitch, 0.8);
    discharge.filter(Filter::Low, 5200.0, 0.8);
    discharge.normalize(1.0);
    output.mix(&discharge, start, 1.0);
    output.mix(
        &synth.burst(0.12, 0.022, Bands::range(2600.0, 9500.0), 0.0015),
        start + 0.002,
        0.5,
    );
    output.mix(
        &synth.burst(0.05, 0.009, Bands::band(1750.0 * pitch, 2.5), 0.0),
        start,
        0.4,
    );
    let latch = synth.click(
        &[
            [3450.0 * pitch, 0.005, 1.0],
            [5300.0, 0.004, 0.6],
            [2050.0, 0.007, 0.4],
        ],
        Click {
            high: 3200.0,
            tick: 0.5,
            ..Click::default()
        },
    );
    output.mix(&latch, start + 0.024, 0.24);
    output.normalize(1.0);
    output.saturate(1.5);
    output.filter(Filter::High, 350.0, 0.707);
    output
}

pub fn hound_fire(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.42);
    let pitch = [1.0, 0.94, 1.06][variant % 3];
    let start = 0.0005;
    output.mix(
        &synth.burst(0.008, 0.0013, Bands::high(2800.0), 0.0),
        start,
        0.8,
    );
    let mut discharge = synth.silent(0.17);
    let (mut phase, mut sideband) = (0.0, 0.0);
    for (index, sample) in discharge.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        let frequency = (150.0 + 760.0 * (-time / 0.028).exp()) * pitch;
        phase += frequency / synth.rate;
        sideband += (frequency * 2.76 + 90.0) / synth.rate;
        let pulse = 2.0 * (phase - phase.floor()) - 1.0;
        *sample = ((pulse * 0.8 + (TAU * phase).sin() * (TAU * sideband).sin() * 0.9)
            * (-time / 0.04).exp()
            * (1.0 - (-time / 0.0006).exp())) as f32;
    }
    discharge.filter(Filter::High, 170.0, 0.8);
    discharge.filter(Filter::Low, 3600.0, 0.8);
    discharge.normalize(1.0);
    output.mix(&discharge, start, 1.0);
    output.mix(
        &synth.knock(0.09, 210.0 * pitch, 92.0 * pitch, 0.016, 0.024, 0.0008),
        start,
        0.6,
    );
    output.mix(
        &synth.burst(0.07, 0.014, Bands::band(640.0 * pitch, 1.8), 0.0),
        start,
        0.55,
    );
    let mut air = synth.noise(0.3);
    air.sweep(Filter::Band, glide(3800.0, 1500.0, 1.0), 0.9);
    air.envelope(|t| decay(t, 0.065, 0.012));
    air.normalize(1.0);
    output.mix(&air, start + 0.03, 0.34);
    let latch = synth.click(
        &[
            [1380.0 * pitch, 0.012, 1.0],
            [2260.0 * pitch, 0.009, 0.7],
            [3540.0, 0.006, 0.45],
        ],
        Click {
            high: 1800.0,
            tick: 0.6,
            ..Click::default()
        },
    );
    output.mix(&latch, start + 0.062, 0.26);
    output.normalize(1.0);
    output.saturate(1.7);
    output.filter(Filter::High, 80.0, 0.707);
    output
}

pub fn gull_fall(synth: &mut Synth, _: usize) -> Clip {
    let mut output = synth.silent(1.5);
    let mut phases = [0.0; 4];
    output.mix(
        &synth.burst(0.02, 0.004, Bands::high(2500.0), 0.0),
        0.0,
        0.9,
    );
    let frames = round(1.1 * synth.rate) as usize;
    for (index, sample) in output.iter_mut().take(frames).enumerate() {
        let t = index as f64 / synth.rate;
        let slowdown = 0.16_f64.powf(t) * (1.0 + 0.06 * (TAU * (9.0 - 5.0 * t) * t).sin());
        let mut blades = 0.0;
        for (phase, frequency) in phases.iter_mut().zip([184.0, 207.0, 233.0, 258.0]) {
            *phase += TAU * frequency * slowdown / synth.rate;
            blades += phase.sin() + 0.5 * (2.0 * *phase).sin() + 0.3 * (3.0 * *phase).sin();
        }
        let pulse = 0.6 + 0.4 * sign((TAU * (26.0 - 16.0 * t) * t).sin());
        *sample = (f64::from(*sample)
            + blades * 0.16 * pulse * (-t / 0.55).exp() * (1.0 - (-t / 0.01).exp()))
            as f32;
    }
    let mut air = synth.noise(1.0);
    air.sweep(Filter::Band, glide(3200.0, 500.0, 1.0), 1.0);
    air.envelope(|t| decay(t, 0.35, 0.01));
    output.mix(&air, 0.0, 0.3);
    let impact = 0.72;
    output.mix(
        &synth.burst(0.2, 0.035, Bands::low(3000.0), 0.0),
        impact,
        1.0,
    );
    output.mix(
        &synth.knock(0.12, 130.0, 55.0, 0.02, 0.035, 0.0006),
        impact,
        0.8,
    );
    output.mix(
        &synth.ring(
            0.4,
            &[
                [705.0, 0.07, 1.0],
                [1490.0, 0.05, 0.7],
                [2610.0, 0.035, 0.5],
                [4020.0, 0.02, 0.3],
            ],
        ),
        impact,
        0.28,
    );
    let grit = debris(
        synth,
        0.7,
        26,
        Debris {
            start: 0.02,
            skew: 1.7,
            gain_release: 0.2,
            max_length: 0.008,
            bands: Bands::high(1500.0),
            ..Debris::default()
        },
    );
    output.mix(&grit, impact, 0.5);
    output.mix(
        &synth.ring(0.2, &[[1900.0, 0.03, 1.0], [3350.0, 0.02, 0.6]]),
        impact + 0.19,
        0.14,
    );
    output.saturate(1.4);
    output
}

pub fn hound_fall(synth: &mut Synth, _: usize) -> Clip {
    let mut output = synth.silent(1.9);
    let mut motor = synth.silent(1.3);
    let (mut phase, mut secondary) = (0.0, 0.0);
    for (index, sample) in motor.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        let stopped = 1.0 - time / 1.3;
        let pitch = 30.0 + 112.0 * stopped.powf(1.6);
        phase += pitch / synth.rate;
        secondary += (180.0 + 1050.0 * (-time / 0.3).exp()) / synth.rate;
        *sample = ((2.0 * (phase - phase.floor()) - 1.0)
            * (1.0 - (-time / 0.02).exp())
            * stopped.powf(0.7)
            + (TAU * secondary).sin() * 0.25 * (-time / 0.4).exp()) as f32;
    }
    motor.sweep(Filter::Low, glide(1100.0, 160.0, 1.0), 2.5);
    output.mix(&motor, 0.0, 0.55);
    output.mix(
        &synth.burst(0.03, 0.006, Bands::high(2000.0), 0.0),
        0.0,
        0.5,
    );
    for (time, gain, pitch) in [
        (0.62, 1.0, 1.0),
        (0.84, 0.75, 1.22),
        (0.98, 0.5, 0.86),
        (1.17, 0.42, 1.4),
        (1.33, 0.22, 1.1),
    ] {
        output.mix(
            &synth.knock(0.1, 105.0 * pitch, 48.0, 0.02, 0.035, 0.0006),
            time,
            gain * 0.9,
        );
        output.mix(
            &synth.burst(0.08, 0.016, Bands::low(2400.0), 0.0),
            time,
            gain * 0.8,
        );
        output.mix(
            &synth.ring(
                0.3,
                &[
                    [388.0 * pitch, 0.07, 1.0],
                    [861.0 * pitch, 0.05, 0.7],
                    [1540.0 * pitch, 0.035, 0.5],
                    [2750.0 * pitch, 0.02, 0.35],
                ],
            ),
            time,
            gain * 0.3,
        );
    }
    let grit = debris(
        synth,
        1.0,
        22,
        Debris {
            skew: 1.2,
            gain_release: 0.4,
            max_length: 0.007,
            bands: Bands::high(1800.0),
            ..Debris::default()
        },
    );
    output.mix(&grit, 0.62, 0.3);
    output.scale(0.55);
    output.saturate(1.5);
    output
}

pub fn telegraph(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.6);
    let hound = variant != 0;
    let length = 0.55;
    let (mut phase, mut partial) = (0.0, 0.0);
    for (index, sample) in output.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        let t = (time / length).min(1.0);
        let envelope = t.powf(1.4)
            * if time < length {
                1.0
            } else {
                (-(time - length) / 0.008).exp()
            };
        let tone = if hound {
            phase += (170.0 + 380.0 * t * t) / synth.rate;
            partial += (680.0 + 900.0 * t) / synth.rate;
            let pulse = 2.0 * (phase - phase.floor()) - 1.0;
            (pulse * (0.7 + 0.3 * (TAU * 38.0 * time).sin()) + 0.35 * (TAU * partial).sin())
                * envelope
        } else {
            phase += (620.0 + 1900.0 * t * t * t) / synth.rate;
            partial += (1240.0 + 3800.0 * t * t * t) * 1.003 / synth.rate;
            ((TAU * phase).sin()
                + 0.4 * (TAU * partial).sin()
                + 0.25 * sign((TAU * phase * 0.5).sin()))
                * envelope
                * (0.8 + 0.2 * (TAU * (20.0 + 60.0 * t) * time).sin())
        };
        *sample = tone as f32;
    }
    output.filter(Filter::Low, if hound { 2600.0 } else { 7000.0 }, 0.7);
    output.filter(Filter::High, if hound { 120.0 } else { 400.0 }, 0.7);
    let repeats = if hound { 7 } else { 5 };
    for repeat in 0..repeats {
        let start = length * (repeat as f64 / repeats as f64).sqrt();
        let tick = synth.ring(
            0.03,
            &[
                [if hound { 1500.0 } else { 3800.0 }, 0.004, 1.0],
                [if hound { 2600.0 } else { 6100.0 }, 0.003, 0.6],
            ],
        );
        output.mix(&tick, start, 0.12 + 0.2 * repeat as f64 / repeats as f64);
    }
    output
}

pub fn alert(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(0.52);
    let frequency = if variant != 0 { 622.0 } else { 1320.0 };
    let mut phase = 0.0;
    let mut tone = |start: f64, length: f64, frequency: f64, bend: f64| {
        let frames = round(length * synth.rate) as usize;
        let offset = round(start * synth.rate) as usize;
        for (index, sample) in output.iter_mut().skip(offset).take(frames).enumerate() {
            let time = index as f64 / synth.rate;
            let progress = index as f64 / frames as f64;
            phase += TAU * frequency * (1.0 + bend * progress) / synth.rate;
            let fm = (phase + 1.6 * (phase * 0.25).sin() + 0.4 * (TAU * 71.0 * time).sin()).sin();
            let envelope = 1.0_f64
                .min(index as f64 / (0.004 * synth.rate))
                .min((frames - index) as f64 / (0.012 * synth.rate));
            *sample = (f64::from(*sample) + (fm * 2.2).tanh() * envelope) as f32;
        }
    };
    tone(0.0, 0.09, frequency, 0.0);
    tone(0.12, 0.17, frequency * 1.4142, -0.05);
    if variant != 0 {
        tone(0.3, 0.2, frequency * 0.5, 0.5);
    }
    output.filter(
        Filter::Band,
        if variant != 0 { 1100.0 } else { 2100.0 },
        0.6,
    );
    output.mix(&synth.burst(0.4, 0.2, Bands::high(4000.0), 0.02), 0.0, 0.03);
    output
}

pub fn relay_cut(synth: &mut Synth, _: usize) -> Clip {
    let mut output = synth.silent(1.7);
    output.mix(
        &synth.burst(0.01, 0.002, Bands::high(2500.0), 0.0),
        0.0,
        1.0,
    );
    let mut sparks = synth.noise(0.5);
    sparks.filter(Filter::Band, 3000.0, 0.8);
    let mut gate = 1.0;
    for (index, sample) in sparks.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        if index % 220 == 0 {
            gate = if synth.random() < 0.55 + 0.4 * (1.0 - time / 0.5) {
                1.0
            } else {
                0.0
            };
        }
        let carrier = (TAU * 100.0 * time).sin();
        let gain = if carrier > -0.2 { 1.0 } else { 0.15 };
        *sample = ((f64::from(*sample) * gain + 0.35 * sign(carrier) * (synth.random() - 0.3))
            * gate
            * (-time / 0.3).exp()) as f32;
    }
    output.mix(&sparks, 0.005, 0.8);
    let mut wind_down = synth.silent(1.25);
    let (mut rotor, mut motor) = (0.0, 0.0);
    for (index, sample) in wind_down.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        let rest = 1.0 - time / 1.25;
        rotor += TAU * 100.0 * 0.22_f64.powf(time / 1.2) / synth.rate;
        motor += TAU * (150.0 + 1900.0 * rest.powf(2.2)) / synth.rate;
        let hum = rotor.sin()
            + 0.6 * (2.0 * rotor).sin()
            + 0.4 * (3.0 * rotor).sin()
            + 0.25 * (5.0 * rotor).sin();
        *sample = ((hum * 0.5 + motor.sin() * 0.22) * rest.powf(1.2) * (1.0 - (-time / 0.03).exp()))
            as f32;
    }
    output.mix(&wind_down, 0.4, 0.8);
    output.mix(
        &synth.knock(0.12, 140.0, 60.0, 0.03, 0.04, 0.0006),
        0.4,
        0.5,
    );
    output.saturate(1.3);
    output
}
