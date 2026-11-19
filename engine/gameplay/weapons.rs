use std::cell::RefCell;

thread_local! {
    static WEAPON: RefCell<[f64; 7]> = const { RefCell::new([0.0; 7]) };
}

#[unsafe(no_mangle)]
pub extern "C" fn weapon_reload_allowed(
    ammo: f64,
    reserve: f64,
    capacity: f64,
    reloading: bool,
    switching: bool,
) -> bool {
    !reloading && !switching && ammo < capacity && reserve > 0.0
}

#[unsafe(no_mangle)]
pub extern "C" fn weapon_reload_transfer(ammo: f64, reserve: f64, capacity: f64) -> f64 {
    (capacity - ammo).min(reserve)
}

#[unsafe(no_mangle)]
pub extern "C" fn weapon_reload_time(time: f64, delta: f64, fresh: bool, forced: f64) -> f64 {
    if !forced.is_nan() {
        forced
    } else if fresh {
        time
    } else {
        time + delta
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn weapon_switch(
    time: f64,
    delta: f64,
    holster: f64,
    draw: f64,
    fresh: bool,
    holstering: bool,
) -> *const f64 {
    let mut time = if fresh { time } else { time + delta };
    let action = if holstering && time >= holster {
        time = (time - holster).min(draw * 0.5);
        1
    } else if !holstering && time >= draw {
        2
    } else {
        0
    };
    WEAPON.with_borrow_mut(|result| {
        result[..2].copy_from_slice(&[time, f64::from(action)]);
        result.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn weapon_trigger(
    dt: f64,
    mut cooldown: f64,
    mut buffer: f64,
    rpm: f64,
    mut ammo: f64,
    reserve: f64,
    flags: u32,
) -> *const f64 {
    let automatic = flags & 1 != 0;
    let pressed = flags & 2 != 0;
    let edge = flags & 4 != 0;
    let ready = flags & 8 != 0;
    let dry_done = flags & 16 != 0;

    cooldown -= dt;
    let mut shots = 0;
    let mut dry = false;
    let mut reload_delay = -1.0;
    let starting_cooldown = cooldown;
    let period = 60.0 / rpm;

    if automatic {
        if pressed && ready {
            if ammo > 0.0 {
                while cooldown <= 0.0 && ammo > 0.0 && shots < 3 {
                    shots += 1;
                    ammo -= 1.0;
                    cooldown += period;
                }
            } else if !dry_done {
                dry = true;
                if reserve > 0.0 {
                    reload_delay = 0.32;
                }
            }
        }
    } else {
        if edge {
            buffer = 0.13;
        } else if buffer > 0.0 {
            buffer -= dt;
        }
        if buffer > 0.0 && ready && cooldown <= 0.0 {
            buffer = 0.0;
            if ammo > 0.0 {
                shots = 1;
                cooldown = period;
            } else {
                dry = true;
                cooldown = 0.12;
                if reserve > 0.0 {
                    reload_delay = 0.28;
                }
            }
        }
    }
    cooldown = cooldown.max(-0.001);
    WEAPON.with_borrow_mut(|result| {
        *result = [
            starting_cooldown,
            cooldown,
            buffer,
            f64::from(shots),
            f64::from(dry as u8),
            reload_delay,
            period,
        ];
        result.as_ptr()
    })
}
