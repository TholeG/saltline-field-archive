use super::dsp::{Bands, Clip, Filter, Synth, decay, glide, round};
use std::f64::consts::{PI, TAU};

pub fn relays(synth: &mut Synth, _: usize) -> Clip {
    let mut hum = synth.silent(2.0);
    let rate = synth.rate;
    for (index, sample) in hum.iter_mut().enumerate() {
        let time = index as f64 / rate;
        let phase = TAU * 50.0 * time;
        *sample = ((phase.sin() * 0.5
            + (2.0 * phase + 0.4).sin()
            + 0.5 * (3.0 * phase + 1.0).sin()
            + 0.3 * (4.0 * phase).sin()
            + 0.22 * (6.0 * phase + 2.0).sin()
            + 0.12 * (10.0 * phase).sin())
            * 0.3
            * (0.9 + 0.1 * (TAU * 0.5 * time).sin())
            + (TAU * 4100.0 * time).sin() * 0.012 * (0.6 + 0.4 * (TAU * time).sin())
            + (TAU * 1250.0 * time + 0.6 * (TAU * 2.0 * time).sin()).sin() * 0.02)
            as f32;
    }
    let noise = synth.loop_noise(2.0, 0.25, |clip| clip.filter(Filter::Band, 2100.0, 1.2));
    for (index, (sample, &noise)) in hum.iter_mut().zip(noise.iter()).enumerate() {
        let buzz = (0.5 + 0.5 * (TAU * 100.0 * index as f64 / rate).sin()).powf(3.0);
        *sample = (f64::from(*sample) + f64::from(noise) * 0.16 * buzz) as f32;
    }
    hum
}

pub fn servo(synth: &mut Synth, _: usize) -> Clip {
    let mut servo = synth.silent(2.0);
    let rate = synth.rate;
    for (index, sample) in servo.iter_mut().enumerate() {
        let time = index as f64 / rate;
        let modulation = 1.1 * (TAU * 0.5 * time).sin() + 0.5 * (TAU * 1.5 * time + 1.0).sin();
        let phase = TAU * 310.0 * time + modulation;
        let pulse = 0.7 + 0.3 * (TAU * 1.5 * time).sin();
        let motion = (phase.sin()
            + 0.45 * (2.0 * phase + modulation).sin()
            + 0.2 * (4.0 * phase + 0.5).sin())
            * 0.3
            * pulse;
        let motor = TAU * 62.0 * time;
        let tone = motor.sin()
            + 0.5 * (2.0 * motor).sin()
            + 0.33 * (3.0 * motor).sin()
            + 0.2 * (5.0 * motor).sin();
        *sample = (motion + tone * 0.28) as f32;
    }
    let noise = synth.loop_noise(2.0, 0.25, |clip| {
        clip.filter(Filter::High, 2800.0, 0.7);
        clip.filter(Filter::Low, 7000.0, 0.7);
    });
    for (index, (sample, &noise)) in servo.iter_mut().zip(noise.iter()).enumerate() {
        let pulse = 0.6 + 0.4 * (TAU * 1.5 * index as f64 / rate + 2.0).sin();
        *sample = (f64::from(*sample) + f64::from(noise) * 0.12 * pulse) as f32;
    }
    servo
}

pub fn rotor(synth: &mut Synth, _: usize) -> Clip {
    let mut motor = synth.silent(2.0);
    let rate = synth.rate;
    for (frequency, modulation) in [(184.0, 2.5), (207.5, 3.5), (233.0, 2.0), (258.5, 4.5)] {
        let phase = synth.random() * TAU;
        let modulation_phase = synth.random() * TAU;
        for (index, sample) in motor.iter_mut().enumerate() {
            let time = index as f64 / rate;
            let x = TAU * frequency * time + phase;
            let harmonics = x.sin()
                + 0.5 * (2.0 * x + 1.0).sin()
                + 0.3 * (3.0 * x + 2.0).sin()
                + 0.16 * (4.0 * x).sin()
                + 0.08 * (6.0 * x).sin();
            let pulse = 0.72 + 0.28 * (TAU * modulation * time + modulation_phase).sin();
            *sample = (f64::from(*sample) + harmonics * pulse * 0.25) as f32;
        }
    }
    let noise = synth.loop_noise(2.0, 0.25, |clip| {
        clip.filter(Filter::High, 1100.0, 0.7);
        clip.filter(Filter::Low, 5200.0, 0.7);
    });
    for (index, (sample, &noise)) in motor.iter_mut().zip(noise.iter()).enumerate() {
        let pulse = 0.55 + 0.45 * (TAU * 46.0 * index as f64 / rate).sin();
        *sample = (f64::from(*sample) + f64::from(noise) * pulse * 0.5) as f32;
    }
    motor
}

pub fn wind(synth: &mut Synth, _: usize) -> Clip {
    let mut low = synth.noise(8.5);
    let mut high = low.clone();
    low.sweep(
        Filter::Band,
        |position| {
            let t = position * 8.5;
            380.0 + 190.0 * (TAU * t / 8.0).sin() + 90.0 * (TAU * t / 2.667 + 1.0).sin()
        },
        1.3,
    );
    high.sweep(
        Filter::Band,
        |position| {
            let t = position * 8.5;
            1350.0 + 260.0 * (TAU * t / 4.0 + 2.0).sin()
        },
        9.0,
    );
    for (index, (sample, &high)) in low.iter_mut().zip(high.iter()).enumerate() {
        let time = index as f64 / synth.rate;
        let breeze = 0.7 + 0.3 * (TAU * time / 8.0 + 0.5).sin();
        let gusts = (0.5 + 0.5 * (TAU * time / 4.0 + 1.0).sin()).powf(2.0);
        *sample = (f64::from(*sample) * breeze + f64::from(high) * 0.22 * gusts) as f32;
    }
    low.seamless((synth.rate * 8.0) as usize)
}

pub fn sea(synth: &mut Synth, _: usize) -> Clip {
    let mut bed = synth.noise(8.5);
    let mut splash = synth.noise(8.5);
    let mut surf = synth.noise(8.5);
    bed.filter(Filter::Low, 1100.0, 0.6);
    bed.filter(Filter::High, 70.0, 0.7);
    splash.filter(Filter::High, 2400.0, 0.7);
    surf.filter(Filter::Band, 520.0, 1.2);
    for (index, sample) in bed.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        let phase = (time % 4.0) / 4.0;
        let swell = (0.5 - 0.5 * (TAU * phase).cos()).powf(1.5);
        let breaking = (-((phase - 0.5) / 0.05).powf(2.0)).exp();
        let wash = (-((phase - 0.68) / 0.16).powf(2.0)).exp();
        let smaller_wave = 0.5 - 0.5 * (TAU * ((time + 1.3) % 2.667) / 2.667).cos();
        *sample = (f64::from(*sample) * (0.3 + 0.7 * swell + 0.25 * smaller_wave)
            + f64::from(surf[index]) * breaking * 1.8
            + f64::from(splash[index]) * wash * 0.35) as f32;
    }
    bed.seamless((synth.rate * 8.0) as usize)
}

fn rain_channel(synth: &mut Synth) -> Clip {
    let rate = synth.rate;
    let high = synth.loop_noise(6.0, 0.4, |clip| {
        clip.filter(Filter::High, 900.0, 0.6);
        clip.filter(Filter::Low, 8500.0, 0.6);
    });
    let low = synth.loop_noise(6.0, 0.4, |clip| clip.filter(Filter::Band, 420.0, 0.5));
    let mut bed = Clip::with_length(rate, high.len());
    bed.mix(&high, 0.0, 0.5);
    bed.mix(&low, 0.0, 0.75);
    let mut drops = Clip::with_length(rate, bed.len());
    for _ in 0..1020 {
        let mut start = (synth.random() * drops.len() as f64).floor() as usize;
        let length = round(synth.range(0.0006, 0.003) * rate) as usize;
        let gain = synth.random().powf(3.0) * 1.6 + 0.15;
        for index in 0..length {
            let envelope = (-5.0 * index as f64 / length as f64).exp();
            drops[start] =
                (f64::from(drops[start]) + (synth.random() * 2.0 - 1.0) * gain * envelope) as f32;
            start += 1;
            if start == drops.len() {
                start = 0;
            }
        }
    }
    drops.sweep_passes(Filter::High, |_| 2200.0, |_| 0.7, 2);
    for _ in 0..30 {
        let frequency = synth.range(700.0, 2100.0);
        let duration = synth.range(0.008, 0.02);
        let droplet = synth.chirp(duration, frequency, frequency * 1.7, true);
        drops.mix_loop(&droplet, synth.random() * 6.0, synth.range(0.08, 0.3));
    }
    bed.mix(&drops, 0.0, 0.55);
    bed
}

fn roof_rain_channel(synth: &mut Synth) -> Clip {
    let rate = synth.rate;
    let bed = synth.loop_noise(6.0, 0.4, |clip| {
        clip.filter(Filter::Low, 1300.0, 0.6);
        clip.filter(Filter::High, 110.0, 0.7);
    });
    let mut drops = Clip::with_length(rate, bed.len());
    for _ in 0..1560 {
        let index = (synth.random() * drops.len() as f64).floor() as usize;
        let volume = (synth.random() * 2.0 - 1.0) * (0.2 + synth.random().powf(4.0) * 3.0);
        drops[index] = (f64::from(drops[index]) + volume) as f32;
    }
    let mut result = Clip::with_length(rate, bed.len());
    result.mix(&bed, 0.0, 0.8);
    for (frequency, q, level) in [
        (1820.0, 14.0, 1.0),
        (2740.0, 16.0, 0.8),
        (4150.0, 18.0, 0.6),
        (6200.0, 14.0, 0.35),
        (940.0, 8.0, 0.7),
    ] {
        let mut resonate = drops.clone();
        resonate.sweep_passes(Filter::Band, |_| frequency, |_| q, 2);
        for (sample, &layer) in result.iter_mut().zip(resonate.iter()) {
            *sample = (f64::from(*sample) + f64::from(layer) * level * 5.0) as f32;
        }
    }
    for _ in 0..4 {
        let frequency = synth.range(900.0, 1600.0);
        let drop = synth.chirp(0.014, frequency, frequency * 1.8, true);
        result.mix_loop(&drop, synth.random() * 6.0, 0.25);
    }
    result
}

pub fn rain(synth: &mut Synth, _: usize) -> [Clip; 2] {
    [
        rain_channel(&mut synth.fork()),
        rain_channel(&mut synth.fork()),
    ]
}

pub fn roof_rain(synth: &mut Synth, _: usize) -> [Clip; 2] {
    [
        roof_rain_channel(&mut synth.fork()),
        roof_rain_channel(&mut synth.fork()),
    ]
}

fn outdoor_channel(synth: &mut Synth, right: bool) -> Clip {
    let mut clip = synth.silent(1.4);
    for (time, strength) in [
        (0.062, 0.55),
        (0.089, 0.42),
        (0.118, 0.36),
        (0.151, 0.26),
        (0.178, 0.2),
        (0.243, 0.1),
    ] {
        let reflection = synth.burst(0.01, 0.0018, Bands::range(700.0, 5500.0), 0.0);
        clip.mix(
            &reflection,
            time * if right { 1.07 } else { 0.96 } + synth.range(0.0, 0.006),
            strength * synth.range(0.6, 1.15),
        );
    }
    let mut tail = synth.noise(1.3);
    tail.filter(Filter::High, 300.0, 0.7);
    tail.sweep(Filter::Low, glide(6000.0, 900.0, 0.6), 0.7);
    tail.envelope(|t| decay(t, 0.27, 0.04));
    clip.mix(&tail, 0.04, 0.045);
    clip
}

fn shed_channel(synth: &mut Synth, right: bool) -> Clip {
    let mut clip = synth.silent(2.2);
    let mut tail = synth.noise(2.18);
    tail.sweep(Filter::Low, glide(5200.0, 650.0, 0.5), 0.7);
    tail.envelope(|t| decay(t, 0.33, 0.006));
    clip.mix(&tail, 0.012, 0.3);
    let mut body = synth.noise(2.1);
    body.filter(Filter::Low, 260.0, 0.7);
    body.envelope(|t| decay(t, 0.4, 0.01));
    clip.mix(&body, 0.012, 0.5);
    for (time, strength) in [
        (0.018, 0.7),
        (0.031, 0.55),
        (0.047, 0.45),
        (0.093, 0.3),
        (0.139, 0.2),
    ] {
        let reflection = synth.burst(0.008, 0.0015, Bands::low(4500.0), 0.0);
        clip.mix(
            &reflection,
            time * if right { 1.09 } else { 0.95 },
            strength,
        );
    }
    clip
}

pub fn outdoor_ir(synth: &mut Synth, _: usize) -> [Clip; 2] {
    [
        outdoor_channel(&mut synth.fork(), false),
        outdoor_channel(&mut synth.fork(), true),
    ]
}

pub fn shed_ir(synth: &mut Synth, _: usize) -> [Clip; 2] {
    [
        shed_channel(&mut synth.fork(), false),
        shed_channel(&mut synth.fork(), true),
    ]
}

pub fn horn(synth: &mut Synth, _: usize) -> Clip {
    let mut clip = synth.silent(5.0);
    let partials = [0.0, 1.0, 0.435, 0.268, 0.19, 0.145, 0.116, 0.097];
    for (index, sample) in clip.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        let flutter = 0.0025 * (TAU * 4.3 * time).sin();
        let gain = (time / 0.45).min(1.0)
            * ((5.0 - time) / 1.5).min(1.0)
            * if time < 3.2 {
                1.0
            } else {
                (-(time - 3.2) / 0.7).exp()
            };
        let mut sound = 0.0;
        for (i, amplitude) in partials.iter().enumerate().skip(1) {
            let harmonic = i as f64;
            sound += (TAU * 92.5 * harmonic * time * (1.0 + flutter) + harmonic).sin() * amplitude;
        }
        for (i, amplitude) in partials.iter().enumerate().take(5).skip(1) {
            let harmonic = i as f64;
            sound += 0.45 * (TAU * 138.6 * harmonic * time + harmonic * 2.0).sin() * amplitude;
        }
        *sample = (sound * gain) as f32;
    }
    clip.filter(Filter::Low, 520.0, 0.7);
    clip
}

pub fn gull(synth: &mut Synth, variant: usize) -> Clip {
    let mut clip = synth.silent(1.4);
    let calls = if variant != 0 { 3 } else { 4 };
    let mut start = 0.02;
    for call in 0..calls {
        let duration = synth.range(0.17, 0.26) * if variant != 0 { 1.4 } else { 1.0 };
        let length = round(duration * synth.rate) as usize;
        let offset = round(start * synth.rate) as usize;
        let base = if variant != 0 { 1250.0 } else { 1550.0 } * (1.0 - call as f64 * 0.04);
        let mut phase = 0.0;
        let take = length.min(clip.len().saturating_sub(offset));
        for (index, sample) in clip.iter_mut().skip(offset).take(take).enumerate() {
            let t = index as f64 / length as f64;
            let pitch = base * (1.0 + 0.55 * (PI * t.powf(0.6)).sin() - 0.25 * t);
            phase += TAU * pitch / synth.rate;
            let tremolo = 1.0 + 0.3 * (TAU * 63.0 * index as f64 / synth.rate).sin();
            let signal = (phase.sin()
                + 0.6 * (2.0 * phase + 0.5).sin()
                + 0.35 * (3.0 * phase).sin()
                + 0.15 * (4.0 * phase).sin())
                * tremolo
                * (PI * t).sin().powf(0.7)
                * (1.0 - call as f64 * 0.14);
            *sample = (f64::from(*sample) + signal) as f32;
        }
        start += duration + synth.range(0.05, 0.12);
    }
    clip.filter(Filter::Low, 4200.0, 0.7);
    clip
}

pub fn halyard(synth: &mut Synth, variant: usize) -> Clip {
    let mut clip = synth.silent(1.1);
    let pitch = if variant != 0 { 1.19 } else { 1.0 };
    let mut start = 0.01;
    for _ in 0..if variant != 0 { 3 } else { 4 } {
        let release = synth.range(0.06, 0.13);
        let ring = synth.ring(
            0.5,
            &[
                [2310.0 * pitch, release, 1.0],
                [3740.0 * pitch, 0.07, 0.7],
                [5910.0 * pitch, 0.04, 0.5],
                [8130.0 * pitch, 0.025, 0.3],
                [1012.0 * pitch, 0.03, 0.3],
            ],
        );
        clip.mix(&ring, start, synth.range(0.35, 1.0));
        clip.mix(
            &synth.burst(0.004, 0.001, Bands::high(3000.0), 0.0),
            start,
            0.3,
        );
        start += synth.range(0.09, 0.26);
    }
    clip
}

pub fn container_bang(synth: &mut Synth, _: usize) -> Clip {
    let mut ring = synth.silent(2.2);
    ring.mix(
        &synth.ring(
            2.2,
            &[
                [47.0, 0.7, 1.0],
                [77.0, 0.55, 0.9],
                [121.0, 0.4, 0.7],
                [163.0, 0.3, 0.5],
                [214.0, 0.22, 0.4],
                [330.0, 0.12, 0.25],
            ],
        ),
        0.0,
        0.5,
    );
    ring.mix(&synth.burst(0.5, 0.11, Bands::low(420.0), 0.0), 0.0, 1.0);
    let mut clip = synth.silent(2.8);
    for (delay, gain) in [(0.0, 1.0), (0.36, 0.3), (0.61, 0.12)] {
        clip.mix(&ring, delay, gain);
    }
    clip
}

pub fn thunder(synth: &mut Synth, variant: usize) -> Clip {
    let mut clip = synth.silent(6.5);
    let mut rumble = synth.noise(6.4);
    rumble.sweep(
        Filter::Low,
        glide(if variant != 0 { 520.0 } else { 380.0 }, 70.0, 0.6),
        0.7,
    );
    let peaks = std::array::from_fn::<_, 9, _>(|_| {
        [
            synth.range(0.05, 3.6),
            synth.range(0.12, 0.5),
            synth.range(0.4, 1.0),
        ]
    });
    let envelope = |time: f64| {
        let mut sum = 0.0;
        for [position, width, height] in peaks {
            let distance = (time - position) / width;
            sum += height * (-distance * distance).exp();
        }
        (0.25 + sum) * (-time / 1.9).exp() * (1.0 - (-time / 0.05).exp())
    };
    rumble.block_envelope(32, envelope);
    clip.mix(&rumble, 0.0, 1.0);
    let mut bass = synth.noise(6.0);
    bass.filter(Filter::Band, 48.0, 1.2);
    bass.envelope(|time| decay(time, 2.0, 0.2));
    clip.mix(&bass, 0.05, 3.0);
    if variant != 0 {
        clip.mix(
            &synth.burst(0.3, 0.06, Bands::range(300.0, 2500.0), 0.0),
            0.02,
            0.25,
        );
    }
    clip
}
