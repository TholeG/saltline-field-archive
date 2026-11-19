use super::effects::{
    BRASS_FORMANTS, chorus, formants, level, limit, loop_filter, loop_sweep, mix_missing_tail,
    stereo_cutoff, upsample,
};
use super::instruments::{Brass, Strings, brass, midi, pluck, strings};
use super::{BAR, FOUNDATIONS, LOW_CHORDS, STEP};
use crate::dsp::{Clip, Filter, Synth, round};
use std::collections::BTreeMap;
use std::f64::consts::TAU;

fn bass_carrier(signal: &mut Clip, duration: f64) {
    let length = signal.len() as f64;
    let pitch = round(midi(38.0) * duration) / duration;
    let increment = pitch / signal.rate;
    let radians = TAU * increment;
    let recurrence = 2.0 * radians.cos();
    let (mut phase, mut amplitude, mut current, mut previous) = (0.0, 1.0, 0.0, -radians.sin());
    for (index, sample) in signal.iter_mut().enumerate() {
        if index % 256 == 0 {
            amplitude = 0.8 + 0.2 * (TAU * 4.0 * index as f64 / length - 1.2).sin();
            current = (radians * index as f64).sin();
            previous = (radians * (index as f64 - 1.0)).sin();
        }
        let slope = if phase < 0.25 {
            4.0 * phase
        } else if phase < 0.75 {
            2.0 - 4.0 * phase
        } else {
            4.0 * phase - 4.0
        };
        let tone = (0.8 * current + 0.24 * slope) * 1.5 * amplitude;
        *sample = (f64::from(*sample) + tone / (1.0 + tone * tone).sqrt() * 0.105) as f32;
        let next = recurrence * current - previous;
        previous = current;
        current = next;
        phase += increment;
        if phase >= 1.0 {
            phase -= 1.0;
        }
    }
}

pub fn drone(synth: &mut Synth, _: usize) -> [Clip; 2] {
    let rate = synth.rate / 2.0;
    let duration = 16.0 * BAR;
    let mut bass = Clip::new(rate, duration);
    let mut channels = [bass.clone(), bass.clone()];
    bass_carrier(&mut bass, duration);
    let mut low = [Clip::new(rate, 4.0 * BAR)];
    let mut high = low.clone();
    strings(
        &mut low,
        synth,
        midi(38.0),
        -0.9,
        4.0 * BAR - 1.0,
        Strings {
            voices: 3,
            detune: 6.0,
            attack: 1.5,
            release: 0.9,
            gain: 0.15,
            swell: [0.8, 1.0],
            ..Strings::default()
        },
    );
    strings(
        &mut high,
        synth,
        midi(50.0),
        -0.9,
        4.0 * BAR - 1.0,
        Strings {
            voices: 3,
            detune: 8.0,
            attack: 1.7,
            release: 0.9,
            gain: 0.16,
            swell: [0.85, 1.0],
            ..Strings::default()
        },
    );
    for bar in 0..4 {
        bass.mix_loop(&low[0], bar as f64 * 4.0 * BAR, 1.0);
        bass.mix_loop(&high[0], (bar as f64 * 4.0 + 2.0) * BAR, 1.0);
    }
    loop_sweep(
        &mut bass,
        |t| 520.0 + 150.0 * (TAU * t - 0.6).sin() + 70.0 * (TAU * 3.0 * t + 1.0).sin(),
        0.8,
    );
    strings(
        &mut channels,
        synth,
        midi(62.0),
        -1.0,
        8.0 * BAR - 0.8,
        Strings {
            voices: 2,
            detune: 7.0,
            attack: 2.2,
            release: 0.8,
            gain: 0.085,
            width: 0.5,
            ..Strings::default()
        },
    );
    for (index, (root, high)) in FOUNDATIONS.into_iter().zip(LOW_CHORDS).enumerate() {
        let begin = index as f64 * 2.0 * BAR - 0.7;
        strings(
            &mut channels,
            synth,
            midi(root),
            begin,
            2.0 * BAR - 0.1,
            Strings {
                voices: 3,
                detune: 9.0,
                attack: 1.4,
                release: 0.7,
                gain: 0.12,
                width: 0.8,
                swell: [0.75, 1.0],
                ..Strings::default()
            },
        );
        strings(
            &mut channels,
            synth,
            midi(high),
            begin + 0.25,
            2.0 * BAR - 0.35,
            Strings {
                voices: 3,
                detune: 10.0,
                attack: 1.7,
                release: 0.75,
                gain: 0.095,
                width: 0.9,
                swell: [0.7, 1.0],
                ..Strings::default()
            },
        );
    }
    for (index, root) in FOUNDATIONS.into_iter().enumerate() {
        strings(
            &mut channels,
            synth,
            midi(root),
            (8.0 + index as f64 * 2.0) * BAR - 0.7,
            2.0 * BAR - 0.1,
            Strings {
                voices: 4,
                detune: 10.0,
                attack: 1.6,
                release: 0.75,
                gain: 0.16,
                width: 0.85,
                swell: [0.8, 1.0],
                ..Strings::default()
            },
        );
    }
    strings(
        &mut channels,
        synth,
        midi(74.0),
        8.0 * BAR + 0.5,
        6.0 * BAR,
        Strings {
            voices: 2,
            detune: 5.0,
            attack: 3.5,
            release: 1.2,
            gain: 0.045,
            width: 0.9,
            swell: [0.6, 1.0],
            ..Strings::default()
        },
    );
    strings(
        &mut channels,
        synth,
        midi(81.0),
        10.0 * BAR,
        3.5 * BAR,
        Strings {
            voices: 2,
            detune: 4.0,
            attack: 3.0,
            release: 1.3,
            gain: 0.02,
            width: 0.9,
            ..Strings::default()
        },
    );
    strings(
        &mut channels,
        synth,
        midi(60.0),
        14.0 * BAR - 0.4,
        2.0 * BAR - 0.4,
        Strings {
            voices: 3,
            detune: 9.0,
            attack: 1.5,
            release: 0.7,
            gain: 0.09,
            width: 0.7,
            ..Strings::default()
        },
    );
    strings(
        &mut channels,
        synth,
        midi(64.0),
        14.0 * BAR - 0.1,
        2.0 * BAR - 0.7,
        Strings {
            voices: 3,
            detune: 9.0,
            attack: 1.7,
            release: 0.7,
            gain: 0.085,
            width: 0.9,
            ..Strings::default()
        },
    );
    for channel in &mut channels {
        loop_sweep(
            channel,
            |t| 1250.0 + 380.0 * (TAU * 2.0 * t + 0.5).sin() + 160.0 * (TAU * 5.0 * t).sin(),
            0.75,
        );
        formants(channel, &[(290.0, 1.3, 0.4), (1050.0, 1.6, 0.3)], 0.3);
    }
    chorus(&mut channels, 7.0, 13.0, 2.2, 0.3);
    for channel in &mut channels {
        mix_missing_tail(channel, &bass);
    }
    stereo_cutoff(&mut channels, 42.0, 120.0);
    level(&mut channels, 0.071);
    channels.map(|channel| upsample(&channel, true))
}

pub fn strings_loop(synth: &mut Synth, _: usize) -> [Clip; 2] {
    let initial = Clip::new(synth.rate, 8.0 * BAR);
    let mut channels = [initial.clone(), initial];
    for (index, (root, upper)) in [69.0, 70.0, 70.0, 69.0]
        .into_iter()
        .zip([74.0, 74.0, 74.0, 76.0])
        .enumerate()
    {
        let start = index as f64 * 2.0 * BAR - 0.35;
        let tremolo = Some((12.8, 0.5));
        strings(
            &mut channels,
            synth,
            midi(root),
            start,
            2.0 * BAR - 0.15,
            Strings {
                voices: 4,
                detune: 10.0,
                attack: 0.9,
                release: 0.45,
                gain: 0.1,
                width: 0.9,
                pan: -0.15,
                swell: [0.5, 1.0],
                curve: 1.5,
                tremolo,
                ..Strings::default()
            },
        );
        strings(
            &mut channels,
            synth,
            midi(upper),
            start + 0.1,
            2.0 * BAR - 0.25,
            Strings {
                voices: 4,
                detune: 11.0,
                attack: 1.1,
                release: 0.45,
                gain: 0.085,
                width: 0.9,
                pan: 0.15,
                swell: [0.45, 1.0],
                curve: 1.5,
                tremolo,
                ..Strings::default()
            },
        );
        strings(
            &mut channels,
            synth,
            midi(root - 12.0),
            start,
            2.0 * BAR - 0.15,
            Strings {
                voices: 2,
                detune: 8.0,
                attack: 1.2,
                release: 0.5,
                gain: 0.05,
                width: 0.6,
                swell: [0.6, 1.0],
                ..Strings::default()
            },
        );
    }
    strings(
        &mut channels,
        synth,
        midi(86.0),
        0.4,
        8.0 * BAR - 2.2,
        Strings {
            voices: 2,
            detune: 4.0,
            attack: 3.0,
            release: 0.5,
            gain: 0.012,
            width: 1.0,
            swell: [0.7, 1.0],
            ..Strings::default()
        },
    );
    for channel in &mut channels {
        loop_sweep(
            channel,
            |t| 2900.0 + 800.0 * (TAU * 4.0 * t - 1.57).sin() + 300.0 * (TAU * 3.0 * t).sin(),
            0.7,
        );
        formants(channel, &[(480.0, 1.4, 0.3), (1500.0, 1.8, 0.28)], 0.3);
    }
    chorus(&mut channels, 5.0, 11.0, 1.8, 0.25);
    for channel in &mut channels {
        loop_filter(channel, Filter::High, 240.0, 0.707, 0.4);
    }
    level(&mut channels, 0.062);
    channels
}

fn pulse_notes(bar: usize, transpose: i32) -> impl Iterator<Item = (usize, i32, f64)> {
    let chord = (bar >> 1) & 3;
    let sparse = bar >= 8;
    [
        1.0, 0.42, 0.52, 0.86, 0.4, 0.5, 0.8, 0.46, 0.95, 0.4, 0.52, 0.82, 0.42, 0.5, 0.76, 0.5,
    ]
    .into_iter()
    .enumerate()
    .filter_map(move |(position, mut energy)| {
        if sparse && [5, 13].contains(&position) {
            return None;
        }
        let harmony = [53, 53, 55, 52];
        let root = FOUNDATIONS[chord] as i32;
        let mut note = match position {
            3 | 11 => root,
            6 | 14 => harmony[chord] + 12,
            _ => 50,
        };
        if sparse {
            if position == 6 {
                note = 62;
            }
            if position == 14 {
                note = root;
            }
            if position == 8 {
                energy = 1.0;
            }
        }
        if bar == 15 {
            let accent = if [0, 3, 6, 8, 11, 14].contains(&position) {
                0.12
            } else {
                0.0
            };
            energy = (0.45 + 0.04 * position as f64 + accent).min(1.0);
        }
        if bar % 4 == 3 && position == 15 {
            note = root;
        }
        Some((position, note + transpose, energy))
    })
}

fn bass_thud(synth: &Synth) -> Clip {
    let mut clip = synth.silent(0.3);
    let note = midi(38.0);
    let (mut near, mut far) = (0.0, 0.0);
    for (index, sample) in clip.iter_mut().enumerate() {
        let time = index as f64 / synth.rate;
        let square = if (time * note) % 1.0 < 0.5 { 1.0 } else { -1.0 };
        let alpha = 1.0 - (-TAU * (90.0 + 380.0 * (-time / 0.05).exp()) / synth.rate).exp();
        near += (square - near) * alpha;
        far += (near - far) * alpha;
        *sample = (far * (-time / 0.11).exp() * (index as f64 / 60.0).min(1.0)) as f32;
    }
    let fade = round(0.03 * synth.rate) as usize;
    for (index, sample) in clip.iter_mut().rev().take(fade).enumerate() {
        *sample = (f64::from(*sample) * (index as f64 / fade as f64)) as f32;
    }
    clip
}

pub fn pulse(synth: &mut Synth, _: usize) -> Clip {
    let mut clip = synth.silent(16.0 * BAR);
    let mut bass = clip.clone();
    let thud = bass_thud(synth);
    let mut notes = BTreeMap::new();
    for bar in 0..16 {
        for (position, note, energy) in pulse_notes(bar, 0) {
            let velocity = energy * (0.93 + 0.14 * synth.random());
            let tier = if velocity > 0.78 {
                2
            } else if velocity > 0.55 {
                1
            } else {
                0
            };
            let sample = notes.entry((note, tier)).or_insert_with(|| {
                pluck(
                    synth,
                    midi(f64::from(note)),
                    [0.4, 0.7, 1.0][tier],
                    0.5,
                    0.12 + 0.05 * tier as f64,
                )
            });
            clip.mix_loop(
                sample,
                bar as f64 * BAR + position as f64 * STEP,
                0.3 * (0.35 + 0.65 * velocity),
            );
        }
        bass.mix_loop(&thud, bar as f64 * BAR, 0.2);
        bass.mix_loop(&thud, bar as f64 * BAR + 2.0 * super::BEAT, 0.14);
    }
    loop_filter(&mut clip, Filter::High, 115.0, 0.707, 0.4);
    let mut short = clip.clone();
    loop_filter(&mut short, Filter::Low, 1900.0, 0.7, 0.4);
    let mut long = short.clone();
    loop_filter(&mut long, Filter::Low, 1200.0, 0.7, 0.4);
    let delay = round(3.0 * STEP * synth.rate) / synth.rate;
    clip.mix_loop(&short, delay, 0.27);
    clip.mix_loop(&long, delay * 2.0, 0.1);
    mix_missing_tail(&mut clip, &bass);
    clip.normalize(1.0);
    limit(&mut clip, 1.6);
    level(std::slice::from_mut(&mut clip), 0.078);
    clip
}

pub fn lift(synth: &mut Synth, _: usize) -> Clip {
    let mut clip = synth.silent(8.0 * BAR);
    let mut pad = [clip.clone()];
    let mut notes = BTreeMap::new();
    for bar in 0..8 {
        let number = bar + if bar >= 4 { 8 } else { 0 };
        for (position, note, energy) in pulse_notes(number, 12) {
            let velocity = (energy * 1.08 * (0.93 + 0.14 * synth.random())).min(1.0);
            let tier = if velocity > 0.78 {
                2
            } else if velocity > 0.55 {
                1
            } else {
                0
            };
            let sample = notes.entry((note, tier)).or_insert_with(|| {
                pluck(
                    synth,
                    midi(f64::from(note)),
                    [0.55, 0.8, 1.0][tier],
                    0.4,
                    0.1 + 0.03 * tier as f64,
                )
            });
            clip.mix_loop(
                sample,
                bar as f64 * BAR + position as f64 * STEP,
                0.22 * (0.4 + 0.6 * velocity),
            );
        }
    }
    loop_filter(&mut clip, Filter::High, 230.0, 0.707, 0.4);
    for (bar, note) in [74.0, 76.0, 77.0, 79.0, 81.0, 82.0, 84.0, 86.0]
        .into_iter()
        .enumerate()
    {
        let start = bar as f64 * BAR - 0.12;
        strings(
            &mut pad,
            synth,
            midi(note),
            start,
            BAR - 0.1,
            Strings {
                voices: 3,
                detune: 11.0,
                attack: 0.35,
                release: 0.3,
                gain: 0.1,
                swell: [0.55, 1.0],
                curve: 1.4,
                tremolo: Some((12.8, 0.4)),
                ..Strings::default()
            },
        );
        strings(
            &mut pad,
            synth,
            midi(note - 12.0),
            start,
            BAR - 0.1,
            Strings {
                voices: 2,
                detune: 8.0,
                attack: 0.4,
                release: 0.3,
                gain: 0.06,
                swell: [0.6, 1.0],
                ..Strings::default()
            },
        );
    }
    loop_sweep(
        &mut pad[0],
        |t| 3300.0 + 900.0 * (TAU * t - 1.57).sin(),
        0.7,
    );
    loop_filter(&mut pad[0], Filter::High, 300.0, 0.707, 0.4);
    let mut echo = clip.clone();
    loop_filter(&mut echo, Filter::Low, 2600.0, 0.7, 0.4);
    clip.mix_loop(&echo, round(3.0 * STEP * synth.rate) / synth.rate, 0.22);
    mix_missing_tail(&mut clip, &pad[0]);
    level(std::slice::from_mut(&mut clip), 0.071);
    clip
}

pub fn brass_loop(synth: &mut Synth, _: usize) -> Clip {
    let mut clip = synth.silent(8.0 * BAR);
    let stabs = [
        [38.0, 50.0, 57.0],
        [38.0, 53.0, 58.0],
        [38.0, 55.0, 58.0],
        [38.0, 52.0, 57.0],
    ];
    let swells = [
        [50.0, 57.0, 62.0],
        [53.0, 58.0, 62.0],
        [55.0, 58.0, 62.0],
        [52.0, 57.0, 64.0],
    ];
    let passages = [
        (0, 0, 1.0, 0),
        (0, 11, 0.6, 0),
        (1, 4, 0.9, 12),
        (2, 0, 0.9, 0),
        (2, 6, 0.7, 0),
        (3, 3, 0.55, 0),
        (3, 14, 0.75, 0),
        (4, 0, 1.0, 0),
        (4, 8, 0.6, 0),
        (5, 4, 0.9, 12),
        (6, 0, 0.9, 0),
        (6, 6, 0.7, 0),
        (6, 11, 0.55, 0),
        (7, 0, 1.0, 16),
    ];
    for (bar, position, velocity, duration) in passages {
        let start = f64::from(bar) * BAR + f64::from(position) * STEP;
        let chord = (bar as usize >> 1) & 3;
        if duration == 0 {
            for note in stabs[chord] {
                let sound = brass(
                    synth,
                    midi(note),
                    0.2,
                    Brass {
                        voices: 3,
                        detune: 8.0,
                        velocity,
                        attack: 0.012,
                        release: 0.07,
                        brightness: 9.0,
                        ..Brass::default()
                    },
                );
                let gain = 0.17 * (0.5 + 0.5 * velocity) * if note == 38.0 { 0.8 } else { 1.0 };
                clip.mix_loop(&sound, start - 0.006, gain);
            }
        } else {
            for note in swells[chord] {
                let sound = brass(
                    synth,
                    midi(note),
                    f64::from(duration) * STEP - 0.05,
                    Brass {
                        voices: 3,
                        detune: 9.0,
                        velocity,
                        attack: 0.25,
                        release: 0.1,
                        brightness: 7.0,
                        swell: [0.12, 1.0],
                        curve: 2.2,
                        ..Brass::default()
                    },
                );
                clip.mix_loop(&sound, start, 0.15);
            }
        }
    }
    loop_filter(&mut clip, Filter::High, 60.0, 0.707, 0.5);
    formants(&mut clip, &BRASS_FORMANTS, 0.3);
    clip.normalize(1.0);
    limit(&mut clip, 1.3);
    level(std::slice::from_mut(&mut clip), 0.08);
    clip
}
