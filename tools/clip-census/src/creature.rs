//! What a creature's attack clip does, measured from its bones: how far the head (its front) moves
//! into the blow, whether the body lunges, leaps or spins, whether it rears up first, and its
//! "tell", the wind-up a player can see before the blow lands. A creature may have no hands and few
//! key bones, so nothing here needs either. Model space as in [`crate::motion`] (X forward, Y left,
//! Z up); distances are shares of the model's size, so a rat and a dragon compare.

use glam::{Mat4, Vec3};

use crate::motion::{key_bone, pose, Bone, Tracks};

/// `KeyBoneID`s the census looks for: the waist, the low spine, the head and the jaw.
const KEY_WAIST: i16 = 5;
const KEY_SPINE_LOW: i16 = 4;
const KEY_HEAD: i16 = 6;
const KEY_JAW: i16 = 7;

/// The bone that stands for the creature's head, and how it was found: the head key bone, else
/// the jaw's, else the bone farthest forward (+X) at rest, which on a model facing +X is its snout
/// or its striking front.
pub fn head_bone(bones: &[Bone]) -> Option<(usize, &'static str)> {
    if let Some(i) = key_bone(bones, KEY_HEAD) {
        return Some((i, "head key bone"));
    }
    if let Some(i) = key_bone(bones, KEY_JAW) {
        return Some((i, "jaw key bone"));
    }
    let front = (0..bones.len()).max_by(|&a, &b| bones[a].pivot.x.total_cmp(&bones[b].pivot.x))?;
    Some((front, "farthest-forward bone"))
}

/// The bone that stands for the body: the waist key bone, else the low spine's. `None` measures the
/// body as the centre of all its bones.
pub fn body_bone(bones: &[Bone]) -> Option<usize> {
    key_bone(bones, KEY_WAIST).or_else(|| key_bone(bones, KEY_SPINE_LOW))
}

/// The rest skeleton's largest extent along X, Y or Z, model yards.
pub fn skeleton_extent(bones: &[Bone]) -> f32 {
    let Some(first) = bones.first() else { return 0.0 };
    let (lo, hi) = bones.iter().fold((first.pivot, first.pivot), |(lo, hi), b| (lo.min(b.pivot), hi.max(b.pivot)));
    (hi - lo).max_element()
}

/// A creature clip measured. Shares are of the model's size (its largest dimension).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CreatureMotion {
    pub duration: f32,
    /// The first impact key (`$AH0`-`$AH3`, `$CAH`), seconds; `None` for a clip without one, which
    /// is measured to the moment its head is farthest from rest instead.
    pub impact: Option<f32>,
    /// The moment the motion is measured to: the impact, or that farthest reach.
    pub strike: f32,
    /// The end of the wind-up: the moment before the strike when the head is farthest from where
    /// it lands (the top of a rear, the back of a draw), seconds. From here on it is the blow.
    pub coil: f32,
    /// The head's travel from rest (frame 0) to the strike: forward (+) or back, and up (+) or down.
    pub head_fwd: f32,
    pub head_up: f32,
    /// The body's forward travel to the strike, and its highest rise over the clip.
    pub body_fwd: f32,
    pub body_rise: f32,
    /// How far the whole body turns about the vertical over the clip, degrees (unwound).
    pub spin: f32,
    /// The tell: the head's farthest move from rest during the wind-up (up to the coil).
    pub tell: f32,
    /// The head's highest rise over its rest before the strike, and how far it comes down from
    /// there by the strike.
    pub rear: f32,
    pub rear_drop: f32,
}

/// Measure a creature clip. `head` is from [`head_bone`], `body` from [`body_bone`], `size` the
/// model's largest dimension (yards, unscaled).
pub fn measure_creature(bones: &[Bone], tracks: &[Tracks], duration: f32, impact: Option<f32>, head: usize, body: Option<usize>, size: f32) -> CreatureMotion {
    const N: usize = 80;
    let size = size.max(0.1);
    let duration = duration.max(1e-3);
    let place = |m: &[Mat4], i: usize| m[i].transform_point3(bones[i].pivot);
    let times: Vec<f32> = (0..=N).map(|k| duration * k as f32 / N as f32).collect();
    let poses: Vec<Vec<Mat4>> = times.iter().map(|&t| pose(bones, tracks, t)).collect();
    let all: Vec<Vec<Vec3>> = poses.iter().map(|m| (0..bones.len()).map(|i| place(m, i)).collect()).collect();
    let centre = |ps: &[Vec3]| ps.iter().copied().sum::<Vec3>() / ps.len().max(1) as f32;
    let body_at = |k: usize| match body {
        Some(b) => all[k][b],
        None => centre(&all[k]),
    };
    let head_at = |k: usize| all[k][head];
    let head0 = head_at(0);
    let k_strike = match impact {
        Some(t) if t > 0.0 => (((t / duration) * N as f32).round() as usize).clamp(1, N),
        _ => (1..=N)
            .max_by(|&a, &b| (head_at(a) - head0).length().total_cmp(&(head_at(b) - head0).length()))
            .filter(|&k| (head_at(k) - head0).length() > 1e-4)
            .unwrap_or(N / 2),
    };
    let strike = head_at(k_strike);
    // The wind-up's end: the frame before the strike farthest from where the head lands. A head
    // that goes straight from rest into the blow coils at frame 0 and has no tell.
    let mut k_coil = 0;
    for k in 0..=k_strike {
        if (head_at(k) - strike).length() > (head_at(k_coil) - strike).length() + 1e-5 {
            k_coil = k;
        }
    }
    let tell = (0..=k_coil).map(|k| (head_at(k) - head0).length()).fold(0.0, f32::max);
    let (rear, rear_drop) = {
        let peak = (0..=k_strike).map(|k| head_at(k).z - head0.z).fold(0.0, f32::max);
        (peak, peak - (strike.z - head0.z))
    };
    let body0 = body_at(0);
    let body_rise = (0..=N).map(|k| body_at(k).z - body0.z).fold(0.0, f32::max);
    // The turn about the vertical: the best-fit rotation of every bone about the bones' centre,
    // frame to frame, so a model needs no body bone and a swinging head alone barely counts.
    let mut spin = 0.0;
    for k in 1..=N {
        let (c0, c1) = (centre(&all[k - 1]), centre(&all[k]));
        let (mut cross, mut dot) = (0.0, 0.0);
        for (a, b) in all[k - 1].iter().zip(&all[k]) {
            let (a, b) = ((*a - c0).truncate(), (*b - c1).truncate());
            cross += a.perp_dot(b);
            dot += a.dot(b);
        }
        if cross.abs() + dot.abs() > 1e-8 {
            spin += cross.atan2(dot);
        }
    }
    CreatureMotion {
        duration,
        impact: impact.filter(|&t| t > 0.0),
        strike: times[k_strike],
        coil: times[k_coil],
        head_fwd: (strike.x - head0.x) / size,
        head_up: (strike.z - head0.z) / size,
        body_fwd: (body_at(k_strike).x - body0.x) / size,
        body_rise: body_rise / size,
        spin: spin.to_degrees().abs(),
        tell: tell / size,
        rear: rear / size,
        rear_drop: rear_drop / size,
    }
}

/// Below this tell (a share of the model's largest dimension) the wind-up is hard to see. A
/// starting point to check against the real census: a 2-3 yd creature whose head draws under a
/// tenth of its length (0.2-0.3 yd, a few pixels at the usual 10-15 yd camera) before the blow
/// reads no differently from its idle sway; the snakes, whose strikes are a short jab from rest,
/// should fall under it and a raptor's or a bear's wind-up well over.
pub const SMALL_TELL: f32 = 0.10;

/// What a creature clip is, in a few plain words.
pub fn describe_creature(m: &CreatureMotion) -> Vec<&'static str> {
    let mut tags = Vec::new();
    // The body carries forward a quarter of the model's size (a 3 yd raptor, 0.75 yd): the
    // character census's 0.8 yd lunge on a 2 yd body is 0.4 of its height, but a beast's size is
    // its length, which runs longer than a man's height.
    if m.body_fwd > 0.25 {
        tags.push("lunge");
    }
    // Off the ground by 0.15 of its size (the character census: 0.2 of the height).
    if m.body_rise > 0.15 {
        tags.push("leap");
    }
    if m.spin > 200.0 {
        tags.push("spin");
    }
    // The head rises 0.15 of the size and comes at least half of that back down into the blow.
    if m.rear > 0.15 && m.rear_drop > m.rear * 0.5 {
        tags.push("rear");
    }
    if m.tell < SMALL_TELL {
        tags.push("small");
    }
    // The character census's timing words, so the two tables read alike.
    match m.impact {
        Some(t) if t < 0.32 => tags.push("quick"),
        Some(t) if t > 0.55 => tags.push("slow"),
        _ => {}
    }
    tags
}

/// The display ids in a list file: numbers separated by whitespace or commas, with `#` comments
/// and any other word ignored; each id once, in the order first given.
pub fn parse_display_list(text: &str) -> Vec<u32> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("");
        for word in line.split(|c: char| c.is_whitespace() || c == ',') {
            if let Ok(id) = word.parse::<u32>() {
                if !out.contains(&id) {
                    out.push(id);
                }
            }
        }
    }
    out
}

/// A model path's file name without its folders or extension: `Creature\Snake\Snake.mdx` → `Snake`.
pub fn model_stem(path: &str) -> &str {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    /// A four-legged stick beast facing +X, 2 yd long: a root on the ground, a spine bone over it,
    /// a neck forward and up, a head on its end, and a tail behind.
    fn beast(keys: bool) -> Vec<Bone> {
        let k = |id: i16| if keys { id } else { -1 };
        vec![
            Bone { parent: -1, pivot: Vec3::ZERO, key: -1 },
            Bone { parent: 0, pivot: Vec3::new(0.0, 0.0, 0.8), key: k(5) },
            Bone { parent: 1, pivot: Vec3::new(0.6, 0.0, 1.0), key: -1 },
            Bone { parent: 2, pivot: Vec3::new(1.0, 0.0, 1.1), key: k(6) },
            Bone { parent: 1, pivot: Vec3::new(-1.0, 0.0, 0.7), key: -1 },
        ]
    }

    fn still() -> Vec<Tracks> {
        vec![Tracks::default(); 5]
    }

    #[test]
    fn the_head_is_the_key_bone_else_the_front() {
        assert_eq!(head_bone(&beast(true)), Some((3, "head key bone")));
        assert_eq!(head_bone(&beast(false)), Some((3, "farthest-forward bone")));
        assert_eq!(body_bone(&beast(true)), Some(1));
        assert_eq!(body_bone(&beast(false)), None);
        assert!((skeleton_extent(&beast(true)) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn a_jab_from_rest_is_small_and_quick() {
        // The snake: the neck snaps forward straight from rest into the bite, no draw-back.
        let b = beast(false);
        let mut t = still();
        t[2].translation = vec![(0.0, Vec3::ZERO), (0.25, Vec3::new(0.5, 0.0, 0.0)), (0.6, Vec3::ZERO)];
        let m = measure_creature(&b, &t, 0.6, Some(0.25), 3, None, 2.0);
        let tags = describe_creature(&m);
        assert!(m.head_fwd > 0.2, "the head strikes forward: {m:?}");
        assert!(m.tell < 1e-4 && m.coil == 0.0, "no wind-up: {m:?}");
        assert!(tags.contains(&"small") && tags.contains(&"quick"), "{tags:?}");
        assert!(!tags.contains(&"lunge") && !tags.contains(&"rear"), "{tags:?}");
    }

    #[test]
    fn a_rear_up_then_down_has_a_tell() {
        // The head rises a yard, holds, and comes down forward into the blow.
        let b = beast(true);
        let mut t = still();
        t[2].translation = vec![(0.0, Vec3::ZERO), (0.4, Vec3::new(-0.2, 0.0, 1.0)), (0.6, Vec3::new(0.3, 0.0, -0.3)), (1.0, Vec3::ZERO)];
        let m = measure_creature(&b, &t, 1.0, Some(0.6), 3, Some(1), 2.0);
        let tags = describe_creature(&m);
        assert!((m.coil - 0.4).abs() < 0.02, "the coil is the top of the rear: {m:?}");
        assert!(m.tell > 0.45, "{m:?}");
        assert!(tags.contains(&"rear") && tags.contains(&"slow") && !tags.contains(&"small"), "{tags:?} {m:?}");
        assert!(m.body_fwd.abs() < 1e-4 && m.spin < 1.0, "the body stays put: {m:?}");
    }

    #[test]
    fn a_pounce_lunges_and_leaps_and_a_turn_spins() {
        // The whole body, from the root, jumps a yard up and forward and lands 1.2 yd on.
        let b = beast(false);
        let mut t = still();
        t[0].translation = vec![(0.0, Vec3::ZERO), (0.3, Vec3::new(0.6, 0.0, 0.6)), (0.5, Vec3::new(1.2, 0.0, 0.0))];
        let m = measure_creature(&b, &t, 0.8, Some(0.5), 3, None, 2.0);
        let tags = describe_creature(&m);
        assert!(tags.contains(&"lunge") && tags.contains(&"leap"), "{tags:?} {m:?}");
        // The whole body turning a full circle about the vertical, with no body key bone.
        let mut s = still();
        s[0].rotation = (0..=8).map(|k| (k as f32 * 0.1, Quat::from_rotation_z(k as f32 * std::f32::consts::FRAC_PI_4))).collect();
        let m = measure_creature(&b, &s, 0.8, None, 3, None, 2.0);
        assert!((m.spin - 360.0).abs() < 5.0, "{m:?}");
        assert!(describe_creature(&m).contains(&"spin"));
        assert_eq!(m.impact, None);
    }

    #[test]
    fn without_an_impact_it_measures_to_the_farthest_reach() {
        let b = beast(false);
        let mut t = still();
        t[2].translation = vec![(0.0, Vec3::ZERO), (0.5, Vec3::new(0.4, 0.0, 0.0)), (1.0, Vec3::ZERO)];
        let m = measure_creature(&b, &t, 1.0, None, 3, None, 2.0);
        assert!((m.strike - 0.5).abs() < 0.02 && (m.head_fwd - 0.2).abs() < 0.01, "{m:?}");
        let tags = describe_creature(&m);
        assert!(!tags.contains(&"quick") && !tags.contains(&"slow"), "{tags:?}");
    }

    #[test]
    fn display_lists_skip_comments_and_words() {
        let text = "# displays\n2464  # sc_gallant Scarlet Gallant\n1,2, 3\n  44 raptor 1.5\n2464\n# 99\n";
        assert_eq!(parse_display_list(text), vec![2464, 1, 2, 3, 44]);
        assert_eq!(model_stem("Creature\\Snake\\Snake.mdx"), "Snake");
        assert_eq!(model_stem("creature/raptor/raptor.m2"), "raptor");
        assert_eq!(model_stem("Bare"), "Bare");
    }
}
