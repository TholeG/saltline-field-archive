use std::cell::RefCell;

thread_local! {
    static AUDIO: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_buffer(length: usize) -> *mut f32 {
    AUDIO.with_borrow_mut(|samples| {
        samples.resize(length, 0.0);
        samples.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_filter(b0: f64, b1: f64, b2: f64, a1: f64, a2: f64) {
    AUDIO.with_borrow_mut(|samples| {
        let (mut z1, mut z2) = (0.0, 0.0);
        for sample in samples {
            let input = f64::from(*sample);
            let output = b0 * input + z1;
            z1 = b1 * input - a1 * output + z2;
            z2 = b2 * input - a2 * output;
            *sample = output as f32;
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_peak_fade(
    start: usize,
    length: usize,
    attack: usize,
    release: usize,
) -> f64 {
    AUDIO.with_borrow_mut(|samples| {
        let channel = &mut samples[start..start + length];
        for (index, sample) in channel.iter_mut().take(attack).enumerate() {
            *sample = (f64::from(*sample) * (index as f64 / attack as f64)) as f32;
        }
        for (index, sample) in channel.iter_mut().rev().take(release).enumerate() {
            let fraction = index as f64 / release as f64;
            *sample = (f64::from(*sample) * (fraction * fraction)) as f32;
        }
        channel
            .iter()
            .fold(0.0_f64, |peak, sample| peak.max(f64::from(sample.abs())))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_scale(gain: f64) {
    AUDIO.with_borrow_mut(|samples| {
        for sample in samples {
            *sample = (f64::from(*sample) * gain) as f32;
        }
    });
}
