use super::effects::{BRASS_FORMANTS, HALL, Reverb, formants, limit, reverb, upsample};
use super::instruments::{
    Brass, Strings, big_drum, brass, drum, drum_tone, gong, hammer, midi, saw, strings,
};
use super::{BAR, BEAT};
use crate::dsp::{Clip, Filter, Synth, glide, round};
use std::f64::consts::TAU;

const CELLS: [[f64; 2]; 4] = [[62.0, 69.0], [70.0, 65.0], [62.0, 67.0], [69.0, 64.0]];
const CELL_SECOND: f64 = 0.9375;

pub fn cell(synth: &mut Synth, variant: usize) -> Clip {
    let mut clip = synth.silent(5.4);
    let [a, b] = CELLS[variant % CELLS.len()];
    clip.mix(&hammer(synth, midi(a), 0.62, 3.2), 0.002, 0.55);
    clip.mix(&hammer(synth, midi(b), 0.8, 4.3), CELL_SECOND + 0.002, 0.6);
    clip.mix(
        &hammer(synth, midi(b - 12.0), 0.5, 4.0),
        CELL_SECOND + 0.002,
        0.2,
    );
    reverb(&mut clip, 0.5, HALL);
    clip
}

pub fn horn(synth: &mut Synth, variant: usize) -> Clip {
    let mut clip = synth.silent(4.8);
    let [a, b] = CELLS[variant % CELLS.len()];
    for shift in [-12.0, 0.0] {
        clip.mix(
            &brass(
                synth,
                midi(a + shift),
                CELL_SECOND - 0.1,
                Brass {
                    velocity: 0.7,
                    attack: 0.05,
                    release: 0.09,
                    voices: 4,
                    detune: 8.0,
                    brightness: 6.5,
                    ..Brass::default()
                },
            ),
            0.0,
            if shift != 0.0 { 0.3 } else { 0.36 },
        );
        clip.mix(
            &brass(
                synth,
                midi(b + shift),
                2.5,
                Brass {
                    velocity: 0.9,
                    attack: 0.07,
                    release: 0.25,
                    voices: 4,
                    detune: 9.0,
                    brightness: 6.5,
                    swell: [0.75, 1.0],
                    curve: 0.7,
                    ..Brass::default()
                },
            ),
            CELL_SECOND,
            if shift != 0.0 { 0.34 } else { 0.4 },
        );
    }
    clip.filter(Filter::High, 110.0, 0.707);
    formants(&mut clip, &BRASS_FORMANTS, 0.0);
    reverb(
        &mut clip,
        0.3,
        Reverb {
            tail: 2.4,
            size: 1.3,
            damping: 0.3,
            pre_delay: 0.025,
            high_pass: 220.0,
        },
    );
    clip
}

pub fn hit(synth: &mut Synth, variant: usize) -> Clip {
    let mut clip = synth.silent(4.6);
    clip.mix(
        &big_drum(synth, 1.2),
        0.03,
        if variant != 0 { 0.5 } else { 0.9 },
    );
    if variant != 0 {
        clip.mix(&gong(synth, midi(45.0), 2.6, 0.55), 0.03, 0.4);
        clip.mix(&gong(synth, midi(50.0) * 1.012, 2.2, 0.5), 0.09, 0.22);
    }
    clip.filter(Filter::Low, if variant != 0 { 1500.0 } else { 500.0 }, 0.7);
    reverb(
        &mut clip,
        1.1,
        Reverb {
            tail: 3.6,
            size: 1.6,
            damping: 0.4,
            pre_delay: 0.045,
            high_pass: 120.0,
        },
    );
    clip
}

pub fn riser(synth: &mut Synth, _: usize) -> Clip {
    const PEAK: f64 = 5.0;
    let length = PEAK + 0.6;
    let rate = synth.rate;
    let mut noise = synth.noise(length);
    noise.sweep(
        Filter::Band,
        |t| 320.0 * 18.0_f64.powf((t * length / PEAK).min(1.0)),
        1.3,
    );
    let mut tones = synth.silent(length);
    let mut phases = std::array::from_fn::<_, 5, _>(|_| synth.random());
    let steps = [-11.0, -5.0, 0.0, 6.0, 12.0].map(|cents| 2.0_f64.powf(cents / 1200.0) / rate);
    let (mut phase, mut frequency, mut volume, mut thickness) = (0.0, 0.0, 0.0, 0.0);
    for (index, sample) in tones.iter_mut().enumerate() {
        if index % 16 == 0 {
            let time = index as f64 / rate;
            let progress = (time / PEAK).min(1.0);
            frequency = midi(50.0) * 2.0_f64.powf(progress.powf(1.8));
            phase += 16.0 * (3.2 + 9.6 * progress * progress) / rate;
            volume = if time < PEAK {
                progress.powf(2.2)
            } else {
                (-(time - PEAK) / 0.07).exp()
            } * (1.0 - 0.3 * (0.5 - 0.5 * (TAU * phase).cos()) * progress)
                * 0.2;
            thickness = if time < PEAK {
                progress.powf(2.6)
            } else {
                (-(time - PEAK) / 0.05).exp()
            } * 1.6;
        }
        let mut tone = 0.0;
        for (phase, step) in phases.iter_mut().zip(steps) {
            tone += saw(phase, frequency * step);
        }
        *sample = (tone * volume) as f32;
        noise[index] = (f64::from(noise[index]) * thickness) as f32;
    }
    tones.sweep(
        Filter::Low,
        |t| 350.0 * 11.0_f64.powf((t * length / PEAK).min(1.0)),
        0.9,
    );
    tones.filter(Filter::High, 200.0, 0.707);
    tones.mix(&noise, 0.0, 1.0);
    tones
}

pub fn impact(synth: &mut Synth, _: usize) -> Clip {
    let mut clip = synth.silent(5.0);
    clip.mix(&big_drum(synth, 1.25), 0.0, 1.0);
    clip.mix(&drum(synth, 55.0, 1.0), 0.0, 0.7);
    clip.mix(&gong(synth, midi(50.0), 2.4, 0.9), 0.0, 0.3);
    clip.mix(&gong(synth, midi(57.0) * 0.99, 1.8, 0.8), 0.004, 0.16);
    let mut chord = synth.silent(4.0);
    for (note, gain) in [(38.0, 1.0), (45.0, 0.8), (50.0, 0.9), (57.0, 0.5)] {
        chord.mix(
            &brass(
                synth,
                midi(note),
                1.3,
                Brass {
                    velocity: 1.0,
                    attack: 0.02,
                    release: 0.4,
                    voices: 5,
                    detune: 12.0,
                    brightness: 7.0,
                    swell: [1.0, 0.3],
                    curve: 0.5,
                    scoop: -60.0,
                    ..Brass::default()
                },
            ),
            0.0,
            0.24 * gain,
        );
    }
    formants(&mut chord, &BRASS_FORMANTS, 0.0);
    clip.mix(&chord, 0.0, 1.0);
    reverb(
        &mut clip,
        0.45,
        Reverb {
            tail: 3.4,
            size: 1.55,
            damping: 0.3,
            pre_delay: 0.03,
            high_pass: 160.0,
        },
    );
    limit(&mut clip, 1.3);
    clip
}

pub fn objective(synth: &mut Synth, _: usize) -> Clip {
    let mut clip = synth.silent(5.0);
    for (delay, note, velocity, length, gain) in [
        (0.002, 65.0, 0.7, 1.6, 0.6),
        (0.002 + BEAT / 2.0, 64.0, 0.7, 1.6, 0.58),
        (0.002 + BEAT, 62.0, 0.9, 4.2, 0.72),
        (0.002 + BEAT, 50.0, 0.6, 4.0, 0.3),
        (0.002 + BEAT * 2.5, 74.0, 0.55, 3.0, 0.3),
    ] {
        clip.mix(&hammer(synth, midi(note), velocity, length), delay, gain);
    }
    let mut chord = synth.silent(4.6);
    for (note, gain) in [(50.0, 1.0), (57.0, 0.9), (62.0, 0.8)] {
        chord.mix(
            &brass(
                synth,
                midi(note),
                1.9,
                Brass {
                    velocity: 0.55,
                    attack: 0.3,
                    release: 0.5,
                    voices: 4,
                    detune: 8.0,
                    brightness: 5.0,
                    swell: [0.6, 1.0],
                    curve: 0.8,
                    scoop: 0.0,
                    ..Brass::default()
                },
            ),
            BEAT - 0.1,
            0.2 * gain,
        );
    }
    formants(&mut chord, &BRASS_FORMANTS, 0.0);
    clip.mix(&chord, 0.0, 1.0);
    clip.mix(&big_drum(synth, 1.0), BEAT, 0.3);
    reverb(&mut clip, 0.5, HALL);
    clip
}

pub fn death(synth: &mut Synth, _: usize) -> Clip {
    let mut clip = synth.silent(5.2);
    let rate = synth.rate / 2.0;
    let mut pad = Clip::new(rate, 5.2);
    let notes = [50.0, 57.0, 62.0, 65.0];
    let mut phases = [0.0; 12];
    let mut steps = [0.0; 12];
    for index in 0..phases.len() {
        phases[index] = synth.random();
        let detune = ((index % 3) as f64 - 1.0) * 9.0 + (synth.random() - 0.5) * 4.0;
        steps[index] = midi(notes[index / 3]) * 2.0_f64.powf(detune / 1200.0) / rate;
    }
    let (mut speed, mut gain) = (1.0, 0.0);
    for (index, sample) in pad.iter_mut().enumerate() {
        if index % 16 == 0 {
            let time = index as f64 / rate;
            speed = 2.0_f64.powf((-16.0 / 12.0) * (time / 4.2).min(1.0).powf(1.35));
            gain = 0.07 * (1.0 - (-time / 0.03).exp()) * (1.0 - time / 4.4).max(0.0).powf(1.2);
        }
        let mut sound = 0.0;
        for (phase, step) in phases.iter_mut().zip(steps) {
            sound += saw(phase, step * speed);
        }
        *sample = (sound * gain) as f32;
    }
    pad.sweep(Filter::Low, glide(2400.0, 140.0, 0.6), 0.9);
    pad.filter(Filter::High, 60.0, 0.707);
    clip.mix(&upsample(&pad, false), 0.0, 1.0);
    clip.mix(&big_drum(synth, 1.2), 0.0, 0.75);
    clip.mix(
        &drum_tone(synth.rate, 2.6, 98.0, 41.0, 0.9, 0.9, 0.004),
        0.02,
        0.4,
    );
    reverb(
        &mut clip,
        0.5,
        Reverb {
            tail: 3.4,
            size: 1.5,
            damping: 0.4,
            pre_delay: 0.03,
            high_pass: 150.0,
        },
    );
    clip
}

pub fn complete(synth: &mut Synth, _: usize) -> Clip {
    let mut clip = synth.silent(12.5);
    let mut pad = [Clip::new(synth.rate / 2.0, 12.5)];
    for (time, duration, notes, gain, swell) in [
        (0.0, BAR, &[46.0, 53.0, 58.0, 62.0][..], 0.075, [0.7, 1.0]),
        (BAR, BAR, &[48.0, 55.0, 60.0, 64.0][..], 0.08, [0.75, 1.0]),
        (
            2.0 * BAR,
            2.6 * BAR,
            &[38.0, 45.0, 50.0, 57.0, 62.0][..],
            0.085,
            [1.0, 0.5],
        ),
    ] {
        for &note in notes {
            strings(
                &mut pad,
                synth,
                midi(note),
                time - 0.25,
                duration,
                Strings {
                    voices: 3,
                    detune: 9.0,
                    attack: 0.5,
                    release: 0.7,
                    gain: gain * if note < 48.0 { 1.1 } else { 0.8 },
                    swell,
                    tail: 2.2,
                    ..Strings::default()
                },
            );
        }
    }
    pad[0].sweep(Filter::Low, |t| 1700.0 - 700.0 * t, 0.75);
    pad[0].filter(Filter::High, 45.0, 0.707);
    clip.mix(&upsample(&pad[0], false), 0.0, 1.0);

    let opening = Brass {
        velocity: 0.6,
        attack: 0.2,
        release: 0.2,
        swell: [0.6, 1.0],
        voices: 3,
        detune: 8.0,
        brightness: 5.0,
        scoop: 0.0,
        ..Brass::default()
    };
    let approach = Brass {
        velocity: 0.7,
        swell: [0.65, 1.0],
        brightness: 5.5,
        ..opening
    };
    let ending = Brass {
        velocity: 0.7,
        attack: 0.1,
        release: 0.8,
        swell: [1.0, 0.3],
        curve: 0.8,
        scoop: -20.0,
        ..opening
    };
    let mut chord = synth.silent(12.5);
    for (time, duration, config, notes, gain) in [
        (
            0.0,
            BAR - 0.2,
            opening,
            &[(46.0, 0.8), (58.0, 1.0)][..],
            0.13,
        ),
        (
            BAR,
            BAR - 0.2,
            approach,
            &[(48.0, 0.8), (60.0, 1.0)][..],
            0.14,
        ),
        (
            2.0 * BAR,
            2.0 * BAR,
            ending,
            &[(38.0, 0.9), (50.0, 1.0), (57.0, 0.8)][..],
            0.15,
        ),
    ] {
        for &(note, accent) in notes {
            chord.mix(
                &brass(synth, midi(note), duration, config),
                time,
                gain * accent,
            );
        }
    }
    formants(&mut chord, &BRASS_FORMANTS, 0.0);
    clip.mix(&chord, 0.0, 1.0);

    for (time, note, velocity, length, gain) in [
        (0.0, 65.0, 0.7, 2.6, 0.5),
        (2.0 * BEAT, 67.0, 0.72, 2.4, 0.5),
        (BAR, 69.0, 0.85, 3.4, 0.6),
        (BAR, 57.0, 0.5, 3.0, 0.2),
        (2.0 * BAR, 62.0, 0.9, 5.5, 0.7),
        (2.0 * BAR, 50.0, 0.7, 5.5, 0.4),
        (2.0 * BAR + CELL_SECOND, 69.0, 0.7, 5.0, 0.5),
        (3.0 * BAR, 74.0, 0.6, 4.5, 0.4),
        (3.0 * BAR + 2.0 * BEAT, 78.0, 0.45, 4.4, 0.3),
    ] {
        clip.mix(
            &hammer(synth, midi(note), velocity, length),
            time + 0.002,
            gain,
        );
    }
    clip.mix(&big_drum(synth, 1.0), 0.0, 0.35);
    clip.mix(&drum(synth, 55.0, 0.8), BAR, 0.3);
    clip.mix(&big_drum(synth, 1.25), 2.0 * BAR, 0.6);
    reverb(
        &mut clip,
        0.5,
        Reverb {
            tail: 3.6,
            size: 1.55,
            damping: 0.3,
            pre_delay: 0.03,
            high_pass: 190.0,
        },
    );
    let start = round(10.4 * synth.rate) as usize;
    let length = clip.len().saturating_sub(start) as f64;
    for (index, sample) in clip.iter_mut().skip(start).enumerate() {
        *sample = (f64::from(*sample) * (1.0 - index as f64 / length).powf(1.5)) as f32;
    }
    clip
}
