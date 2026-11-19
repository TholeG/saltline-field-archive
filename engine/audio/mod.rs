use std::cell::RefCell;

mod ambience;
mod dsp;
mod explosions;
mod foley;
mod mechanism;
mod music;
mod robots;
mod weapons;

use dsp::{Clip, Synth};

#[derive(Clone, Copy)]
enum Recipe {
    Mono(fn(&mut Synth, usize) -> Clip),
    Stereo(fn(&mut Synth, usize) -> [Clip; 2]),
}

const fn id(name: &str) -> u32 {
    let mut hash = 2166136261_u32;
    let bytes = name.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        hash = (hash ^ bytes[index] as u32).wrapping_mul(16777619);
        index += 1;
    }
    hash
}

const SOUNDS: &[(u32, Recipe)] = &[
    (id("rifle_shot"), Recipe::Mono(weapons::rifle)),
    (id("pistol_shot"), Recipe::Mono(weapons::pistol)),
    (id("rifle_tail_out"), Recipe::Stereo(weapons::rifle_outdoor)),
    (
        id("pistol_tail_out"),
        Recipe::Stereo(weapons::pistol_outdoor),
    ),
    (id("rifle_tail_in"), Recipe::Stereo(weapons::rifle_indoor)),
    (id("pistol_tail_in"), Recipe::Stereo(weapons::pistol_indoor)),
    (id("rifle_dry"), Recipe::Mono(mechanism::rifle_dry)),
    (id("pistol_dry"), Recipe::Mono(mechanism::pistol_dry)),
    (id("reload_magout"), Recipe::Mono(mechanism::rifle_mag_out)),
    (
        id("reload_maginsert"),
        Recipe::Mono(mechanism::rifle_mag_insert),
    ),
    (id("reload_magin"), Recipe::Mono(mechanism::rifle_mag_in)),
    (
        id("reload_boltback"),
        Recipe::Mono(mechanism::rifle_bolt_back),
    ),
    (id("reload_bolt"), Recipe::Mono(mechanism::rifle_bolt)),
    (id("pistol_magout"), Recipe::Mono(mechanism::pistol_mag_out)),
    (
        id("pistol_maginsert"),
        Recipe::Mono(mechanism::pistol_mag_insert),
    ),
    (id("pistol_magin"), Recipe::Mono(mechanism::pistol_mag_in)),
    (id("pistol_slide"), Recipe::Mono(mechanism::pistol_slide)),
    (id("weapon_holster"), Recipe::Mono(mechanism::holster)),
    (id("weapon_draw"), Recipe::Mono(mechanism::draw)),
    (id("gear_rustle"), Recipe::Mono(mechanism::rustle)),
    (id("shell_bounce"), Recipe::Mono(mechanism::shell_concrete)),
    (
        id("shell_bounce_metal"),
        Recipe::Mono(mechanism::shell_metal),
    ),
    (id("step_concrete"), Recipe::Mono(foley::concrete_step)),
    (id("step_water"), Recipe::Mono(foley::water_step)),
    (id("step_metal"), Recipe::Mono(foley::metal_step)),
    (id("land"), Recipe::Mono(foley::land)),
    (id("jump"), Recipe::Mono(foley::jump)),
    (id("hurt"), Recipe::Mono(foley::hurt)),
    (id("ear_ring"), Recipe::Mono(foley::ear_ring)),
    (id("impact_metal"), Recipe::Mono(foley::metal_impact)),
    (id("impact_concrete"), Recipe::Mono(foley::concrete_impact)),
    (id("impact_wood"), Recipe::Mono(foley::wood_impact)),
    (id("impact_water"), Recipe::Mono(foley::water_impact)),
    (id("impact_enemy"), Recipe::Mono(foley::enemy_impact)),
    (id("impact_glass"), Recipe::Mono(foley::glass_impact)),
    (id("whiz"), Recipe::Mono(foley::whiz)),
    (id("ricochet"), Recipe::Mono(foley::ricochet)),
    (id("hound_step"), Recipe::Mono(foley::hound_step)),
    (id("ui_click"), Recipe::Mono(foley::ui_click)),
    (id("ui_hover"), Recipe::Mono(foley::ui_hover)),
    (id("gull_fire"), Recipe::Mono(robots::gull_fire)),
    (id("hound_fire"), Recipe::Mono(robots::hound_fire)),
    (id("gull_die"), Recipe::Mono(robots::gull_fall)),
    (id("hound_die"), Recipe::Mono(robots::hound_fall)),
    (id("telegraph"), Recipe::Mono(robots::telegraph)),
    (id("alert"), Recipe::Mono(robots::alert)),
    (id("relay_cut"), Recipe::Mono(robots::relay_cut)),
    (id("explosion"), Recipe::Mono(explosions::explosion)),
    (id("explosion_big"), Recipe::Mono(explosions::explosion_big)),
    (id("music_drone"), Recipe::Stereo(music::loops::drone)),
    (
        id("music_strings"),
        Recipe::Stereo(music::loops::strings_loop),
    ),
    (id("music_pulse"), Recipe::Mono(music::loops::pulse)),
    (id("music_lift"), Recipe::Mono(music::loops::lift)),
    (id("music_brass"), Recipe::Mono(music::loops::brass_loop)),
    (id("music_drums"), Recipe::Mono(music::percussion::drums)),
    (id("music_ticks"), Recipe::Mono(music::percussion::ticks)),
    (id("mus_fill"), Recipe::Mono(music::percussion::fill)),
    (id("mus_cell"), Recipe::Mono(music::cues::cell)),
    (id("mus_horn"), Recipe::Mono(music::cues::horn)),
    (id("mus_hit"), Recipe::Mono(music::cues::hit)),
    (id("mus_riser"), Recipe::Mono(music::cues::riser)),
    (id("mus_impact"), Recipe::Mono(music::cues::impact)),
    (id("objective"), Recipe::Mono(music::cues::objective)),
    (id("death"), Recipe::Mono(music::cues::death)),
    (id("complete"), Recipe::Mono(music::cues::complete)),
    (id("relay_hum"), Recipe::Mono(ambience::relays)),
    (id("hound_servo"), Recipe::Mono(ambience::servo)),
    (id("gull_rotor"), Recipe::Mono(ambience::rotor)),
    (id("rain"), Recipe::Stereo(ambience::rain)),
    (id("rain_interior"), Recipe::Stereo(ambience::roof_rain)),
    (id("wind"), Recipe::Mono(ambience::wind)),
    (id("sea"), Recipe::Mono(ambience::sea)),
    (id("ir_outdoor"), Recipe::Stereo(ambience::outdoor_ir)),
    (id("ir_shed"), Recipe::Stereo(ambience::shed_ir)),
    (id("foghorn"), Recipe::Mono(ambience::horn)),
    (id("gull_cry"), Recipe::Mono(ambience::gull)),
    (id("halyard"), Recipe::Mono(ambience::halyard)),
    (id("container_bang"), Recipe::Mono(ambience::container_bang)),
    (id("thunder"), Recipe::Mono(ambience::thunder)),
];

#[derive(Default)]
struct Pcm {
    channels: usize,
    frames: usize,
    data: Vec<f32>,
}

thread_local! {
    static PCM: RefCell<Pcm> = RefCell::new(Pcm::default());
}

fn recipe(sound: u32) -> Option<Recipe> {
    SOUNDS
        .iter()
        .find_map(|&(id, recipe)| (id == sound).then_some(recipe))
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_has_sound(sound: u32) -> bool {
    recipe(sound).is_some()
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_synthesize(
    sound: u32,
    variant: i32,
    rate: f64,
    peak: f64,
    flags: u32,
) -> *const f32 {
    let Some(recipe) = recipe(sound) else {
        return std::ptr::null();
    };
    let mut synth = Synth::new(rate, sound.wrapping_add(variant.wrapping_mul(7919) as u32));
    let variant = variant as usize;
    let mut clips = match recipe {
        Recipe::Mono(render) => vec![render(&mut synth, variant)],
        Recipe::Stereo(render) => Vec::from(render(&mut synth, variant)),
    };
    let looping = flags & 1 != 0;
    let impulse = flags & 2 != 0;
    let fixed = flags & 4 != 0;
    if !looping {
        let (attack, release) = if impulse {
            (0.0, 0.05)
        } else {
            (0.0003, 0.012)
        };
        for channel in &mut clips {
            channel.fade(attack, release);
        }
    }
    let measured = clips.iter().map(Clip::peak).fold(0.0, f64::max);
    let gain = if fixed {
        1.0_f64.min(peak / if measured == 0.0 { 1.0 } else { measured })
    } else if measured > 0.0 {
        peak / measured
    } else {
        1.0
    };
    PCM.with_borrow_mut(|pcm| {
        pcm.channels = clips.len();
        pcm.frames = clips[0].len();
        if pcm.channels == 1 {
            let mut clip = clips.pop().unwrap();
            clip.scale(gain);
            pcm.data = clip.samples;
        } else {
            pcm.data.clear();
            for mut clip in clips {
                clip.scale(gain);
                pcm.data.extend(clip.samples);
            }
        }
        pcm.data.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_pcm_frames() -> usize {
    PCM.with_borrow(|pcm| pcm.frames)
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_pcm_channels() -> usize {
    PCM.with_borrow(|pcm| pcm.channels)
}
