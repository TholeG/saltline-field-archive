use super::dsp::{Bands, Clip, Filter, Synth, decay, glide};
use super::foley::{Debris, debris};

fn compress(clip: &mut Clip, threshold: f64, ratio: f64, attack: f64, release: f64) {
    let attack = (-1.0 / (attack * clip.rate)).exp();
    let release = (-1.0 / (release * clip.rate)).exp();
    let knee = 1.0 / ratio - 1.0;
    let mut follower = 0.0;
    for sample in clip.iter_mut() {
        let peak = f64::from(sample.abs());
        let speed = if peak > follower { attack } else { release };
        follower = speed * follower + (1.0 - speed) * peak;
        if follower > threshold {
            *sample = (f64::from(*sample) * (follower / threshold).powf(knee)) as f32;
        }
    }
}

fn swept_body(
    synth: &mut Synth,
    length: f64,
    frequencies: [f64; 2],
    curve: f64,
    release: f64,
    attack: f64,
) -> Clip {
    let mut sound = synth.noise(length);
    sound.sweep(
        Filter::Low,
        glide(frequencies[0], frequencies[1], curve),
        0.7,
    );
    sound.envelope(|time| decay(time, release, attack));
    sound.normalize(1.0);
    sound
}

fn bass_body(synth: &mut Synth, length: f64, cutoff: f64, release: f64, attack: f64) -> Clip {
    let mut sound = synth.noise(length);
    sound.filter(Filter::Low, cutoff, 0.7);
    sound.envelope(|time| decay(time, release, attack));
    sound.normalize(1.0);
    sound
}

fn finish(mut clip: Clip, cutoff: f64, start: f64) -> Clip {
    clip.filter(Filter::High, cutoff, 0.707);
    clip.normalize(2.2);
    clip.saturate(1.6);
    clip.envelope(|time| 0.72 + (1.0 - 0.72) * (-(time - start).max(0.0) / 0.0012).exp());
    clip
}

pub fn explosion(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(2.2);
    let pitch = if variant != 0 { 0.9 } else { 1.0 };
    let start = 0.0005;
    let mut blast = synth.silent(0.08);
    blast.mix(&synth.pressure(0.0009), 0.0, 1.2);
    blast.mix(
        &synth.burst(0.03, 0.005, Bands::high(1500.0), 0.0),
        0.0,
        1.5,
    );
    blast.normalize(2.0);
    blast.saturate(2.0);

    let mut tail = synth.silent(2.1);
    tail.mix(
        &synth.knock(0.9, 122.0 * pitch, 47.0, 0.08, 0.16, 0.001),
        0.0,
        1.0,
    );
    tail.mix(
        &swept_body(synth, 1.4, [3000.0, 240.0], 0.5, 0.2, 0.002),
        0.0,
        1.3,
    );
    tail.mix(
        &synth.burst(0.45, 0.08, Bands::band(500.0 * pitch, 1.2), 0.0),
        0.0,
        0.8,
    );
    tail.mix(&bass_body(synth, 2.0, 115.0, 0.45, 0.03), 0.0, 0.4);
    for (delay, gain, cutoff) in [
        (0.13, 0.3, 1900.0),
        (0.235, 0.2, 1300.0),
        (0.41, 0.12, 800.0),
    ] {
        let echo = synth.burst(0.25, 0.045, Bands::range(110.0, cutoff), 0.004);
        tail.mix(&echo, delay * if variant != 0 { 1.1 } else { 1.0 }, gain);
    }
    tail.normalize(1.0);
    compress(&mut tail, 0.3, 4.0, 0.0005, 0.09);
    tail.normalize(1.6);
    tail.saturate(1.8);
    output.mix(&blast, start, 1.0);
    output.mix(&tail, start, 0.8);

    let mut fragments = debris(
        synth,
        1.9,
        70,
        Debris {
            start: 0.2,
            skew: 1.5,
            gain_release: 0.6,
            max_length: 0.008,
            bands: Bands::range(1500.0, 9000.0),
            ..Debris::default()
        },
    );
    fragments.normalize(1.0);
    output.mix(&fragments, 0.0, 0.3);
    for _ in 0..5 {
        let pitch = synth.range(1000.0, 3800.0);
        let release = synth.range(0.02, 0.05);
        let fragment = synth.ring(0.2, &[[pitch, release, 1.0], [pitch * 1.61, 0.02, 0.5]]);
        output.mix(&fragment, synth.range(0.35, 1.5), synth.range(0.03, 0.08));
    }
    finish(output, 22.0, start)
}

pub fn explosion_big(synth: &mut Synth, variant: usize) -> Clip {
    let mut output = synth.silent(4.2);
    let pitch = if variant != 0 { 0.93 } else { 1.0 };
    let start = 0.0005;
    let mut blast = synth.silent(0.12);
    blast.mix(&synth.pressure(0.0014), 0.0, 1.3);
    blast.mix(
        &synth.burst(0.03, 0.004, Bands::high(2000.0), 0.0),
        0.0,
        1.6,
    );
    blast.mix(
        &synth.burst(0.1, 0.016, Bands::range(500.0, 8000.0), 0.0),
        0.0,
        1.1,
    );
    blast.normalize(2.4);
    blast.saturate(2.4);

    let mut tail = synth.silent(4.1);
    tail.mix(
        &synth.knock(2.4, 52.0 * pitch, 29.0, 0.28, 0.42, 0.004),
        0.0,
        1.0,
    );
    tail.mix(
        &synth.knock(1.4, 104.0 * pitch, 60.0, 0.2, 0.26, 0.002),
        0.0,
        0.7,
    );
    tail.mix(
        &swept_body(synth, 2.6, [4200.0, 170.0], 0.45, 0.36, 0.003),
        0.0,
        1.5,
    );
    tail.mix(
        &synth.burst(0.8, 0.14, Bands::band(300.0 * pitch, 1.0), 0.0),
        0.0,
        1.0,
    );
    tail.mix(&bass_body(synth, 4.0, 95.0, 1.0, 0.05), 0.0, 1.0);
    let mut rumble = synth.noise(4.0);
    rumble.sweep(Filter::Low, glide(420.0, 110.0, 0.6), 0.7);
    let peaks = std::array::from_fn::<_, 6, _>(|_| {
        [
            synth.range(0.5, 2.8),
            synth.range(0.12, 0.35),
            synth.range(0.4, 1.0),
        ]
    });
    let envelope = |time: f64| {
        let mut total = 0.3;
        for [center, width, amplitude] in peaks {
            let position = (time - center) / width;
            total += amplitude * (-position * position).exp();
        }
        total * (-time / 1.3).exp() * (1.0 - (-time / 0.15).exp())
    };
    rumble.block_envelope(64, envelope);
    rumble.normalize(1.0);
    tail.mix(&rumble, 0.0, 0.55);
    for (delay, gain, cutoff) in [
        (0.16, 0.42, 1700.0),
        (0.29, 0.34, 1300.0),
        (0.47, 0.26, 950.0),
        (0.79, 0.17, 700.0),
        (1.22, 0.1, 520.0),
    ] {
        let echo = synth.burst(0.5, 0.05 + delay * 0.07, Bands::range(70.0, cutoff), 0.006);
        tail.mix(&echo, delay * if variant != 0 { 1.08 } else { 1.0 }, gain);
    }
    tail.normalize(1.0);
    compress(&mut tail, 0.4, 3.0, 0.0005, 0.1);
    tail.normalize(1.7);
    tail.saturate(1.8);
    output.mix(&blast, start, 1.0);
    output.mix(&tail, start, 0.85);

    let mut fragments = debris(
        synth,
        3.4,
        170,
        Debris {
            start: 0.3,
            skew: 1.6,
            gain_release: 1.0,
            max_length: 0.01,
            bands: Bands::range(1200.0, 9000.0),
            ..Debris::default()
        },
    );
    fragments.normalize(1.0);
    output.mix(&fragments, 0.0, 0.42);
    for _ in 0..11 {
        let pitch = synth.range(900.0, 4200.0);
        let release = synth.range(0.02, 0.07);
        let fragment = synth.ring(
            0.3,
            &[
                [pitch, release, 1.0],
                [pitch * 1.61, 0.025, 0.5],
                [pitch * 2.4, 0.012, 0.25],
            ],
        );
        output.mix(&fragment, synth.range(0.35, 2.7), synth.range(0.04, 0.11));
    }
    let impact = synth.range(0.95, 1.3);
    for (delay, gain) in [(0.0, 1.0), (0.21, 0.5), (0.33, 0.28)] {
        output.mix(
            &synth.ring(
                1.0,
                &[
                    [312.0 * pitch, 0.2, 1.0],
                    [489.0 * pitch, 0.16, 0.8],
                    [744.0 * pitch, 0.12, 0.6],
                    [1183.0 * pitch, 0.08, 0.4],
                    [1735.0 * pitch, 0.05, 0.3],
                    [2620.0 * pitch, 0.03, 0.2],
                ],
            ),
            impact + delay,
            0.12 * gain,
        );
        output.mix(
            &synth.burst(0.02, 0.004, Bands::range(900.0, 5000.0), 0.0),
            impact + delay,
            0.1 * gain,
        );
    }
    finish(output, 18.0, start)
}
