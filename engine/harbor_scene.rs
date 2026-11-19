use std::{cell::RefCell, collections::BTreeMap};

thread_local! {
    static SCENE_TABLE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static BOAT_BASE: RefCell<[f64; 4]> = const { RefCell::new([0.0; 4]) };
    static CONTAINER_VARIANT: RefCell<[f64; 4]> = const { RefCell::new([0.0; 4]) };
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_scene_table(length: usize) -> *mut u32 {
    SCENE_TABLE.with_borrow_mut(|table| {
        table.resize(length, 0);
        table.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn backdrop_plan(selection: u32, showcase: bool) -> u32 {
    let boat = selection == 0 || selection == 1;
    let crane = selection == 0 || selection == 2;
    let ship = selection == 0 || selection == 3;
    let sky = selection == 0 || selection == 4;
    BOAT_BASE.with_borrow_mut(|base| {
        let (x, y, z) = if showcase {
            (24.0, 0.34, 15.0)
        } else {
            (31.5, -1.6, 21.0)
        };
        *base = [x, y, z, -std::f64::consts::FRAC_PI_2];
    });
    u32::from(boat)
        | (u32::from(crane) << 1)
        | (u32::from(ship) << 2)
        | (u32::from(sky) << 3)
        | (u32::from(crane || sky) << 4)
}

#[unsafe(no_mangle)]
pub extern "C" fn backdrop_boat_base() -> *const f64 {
    BOAT_BASE.with_borrow(|base| base.as_ptr())
}

const NO_PARENT: u32 = u32::MAX;

#[unsafe(no_mangle)]
pub extern "C" fn harbor_dynamic_roots() {
    SCENE_TABLE.with_borrow_mut(|table| {
        for start in (0..table.len()).step_by(3) {
            let parent = table[start];
            let no_merge = table[start + 1] != 0;
            let parent_dynamic = parent != NO_PARENT && table[parent as usize * 3 + 2] != 0;
            table[start + 2] = if parent_dynamic {
                2
            } else {
                u32::from(no_merge)
            };
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_merge_groups() -> u32 {
    SCENE_TABLE.with_borrow_mut(|table| {
        let mut groups = BTreeMap::new();
        let mut next = 0;
        for mesh in table.chunks_exact_mut(5) {
            let [material, shadows, layers, keep, group] = mesh else {
                unreachable!()
            };
            *group = if *keep != 0 {
                u32::MAX
            } else {
                *groups
                    .entry((*material, *shadows, *layers))
                    .or_insert_with(|| {
                        let group = next;
                        next += 1;
                        group
                    })
            };
        }
        next
    })
}

const STANDARD: u32 = 1;
const VERTEX_COLORS: u32 = 2;
const TRANSPARENT: u32 = 4;
const BASIC: u32 = 8;
const KNOWN_NAME: u32 = 16;
const COLOR_VARIANT: u32 = STANDARD | KNOWN_NAME;

#[unsafe(no_mangle)]
pub extern "C" fn harbor_material_candidate(flags: u32) -> bool {
    flags & (STANDARD | KNOWN_NAME | VERTEX_COLORS | TRANSPARENT) == COLOR_VARIANT
}

#[unsafe(no_mangle)]
pub extern "C" fn harbor_assign_materials(count: usize) {
    SCENE_TABLE.with_borrow_mut(|table| {
        for index in 0..count {
            let offset = index * 4;
            let flags = table[offset];
            let candidate = table[offset + 1] as usize;
            let selected = if candidate < count && harbor_material_candidate(flags) {
                candidate
            } else {
                index
            };
            let flags = table[selected * 4];
            let receive_shadow = flags & BASIC == 0;
            let cast_shadow = receive_shadow && flags & TRANSPARENT == 0;
            table[offset + 2] = selected as u32;
            table[offset + 3] = u32::from(cast_shadow) | (u32::from(receive_shadow) << 1);
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn container_variant(
    seed: f64,
    opening: f64,
    wear: f64,
    color: i32,
    model: u32,
) -> *const f64 {
    // Output: cache seed, tenths of door travel, wear rounded to halves, paint index.
    let seed_value = if seed.is_nan() {
        0.0
    } else {
        seed.floor().abs()
    };
    let paint = if color >= 0 {
        f64::from(color)
    } else if seed.is_nan() {
        f64::from([0, 1, 2][model as usize])
    } else {
        (seed_value * 7.0 + 3.0) % 8.0
    };
    CONTAINER_VARIANT.with_borrow_mut(|variant| {
        *variant = [
            seed_value % 6.0,
            (opening.clamp(0.0, 1.0) * 10.0).round() / 10.0,
            (wear.clamp(0.0, 1.0) * 2.0).round() / 2.0,
            paint,
        ];
        variant.as_ptr()
    })
}
