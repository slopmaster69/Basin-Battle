//! What an attack clip does, measured from its bone tracks: the body's rise, tilt, spin and travel,
//! and the weapon hand's stroke into the blow. Everything is in the model's own space (WoW axes: X
//! forward, Y left, Z up) and relative to the body's height, so races compare.
//!
//! A vanilla M2 bone is a pivot with an unrotated rest pose: its matrix is
//! `parent · T(pivot) · T(translation) · R(rotation) · T(−pivot)`, and its place at a moment is
//! that matrix applied to its pivot.

use glam::{Mat4, Quat, Vec3};

/// One bone of the skeleton as the census reads it.
#[derive(Clone, Debug)]
pub struct Bone {
    pub parent: i16,
    pub pivot: Vec3,
    pub key: i16,
}

/// One bone's tracks in one clip: `(seconds, value)`, ascending.
#[derive(Clone, Debug, Default)]
pub struct Tracks {
    pub translation: Vec<(f32, Vec3)>,
    pub rotation: Vec<(f32, Quat)>,
}

/// A clip measured.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Motion {
    pub duration: f32,
    /// When the blow lands (its first impact keyframe), seconds; 0 when it has none.
    pub impact: f32,
    /// The body's highest rise over the clip, as a share of the model's height.
    pub rise: f32,
    /// The body's greatest tilt from upright, degrees (past ~100: a flip).
    pub tilt: f32,
    /// How far the body turns about the vertical over the clip, degrees (unwound).
    pub spin: f32,
    /// The body's travel to the impact, yards: forward (+) or back, and sideways.
    pub travel_fwd: f32,
    pub travel_side: f32,
    /// The weapon hand's stroke into the blow, as shares of the height: down (−) or up, sideways,
    /// forward; how high it was raised over the head before it; its fastest speed (heights a
    /// second).
    pub stroke_up: f32,
    pub stroke_side: f32,
    pub stroke_fwd: f32,
    pub raised: f32,
    pub speed: f32,
    /// The off hand moves more than the weapon hand (a left-handed or two-handed blow).
    pub off_hand_leads: bool,
}

fn sample_vec(keys: &[(f32, Vec3)], t: f32) -> Option<Vec3> {
    let (first, last) = (keys.first()?, keys.last()?);
    if t <= first.0 {
        return Some(first.1);
    }
    if t >= last.0 {
        return Some(last.1);
    }
    let i = keys.partition_point(|k| k.0 <= t);
    let (a, b) = (keys[i - 1], keys[i]);
    let s = if b.0 > a.0 { (t - a.0) / (b.0 - a.0) } else { 0.0 };
    Some(a.1.lerp(b.1, s))
}

fn sample_quat(keys: &[(f32, Quat)], t: f32) -> Option<Quat> {
    let (first, last) = (keys.first()?, keys.last()?);
    if t <= first.0 {
        return Some(first.1);
    }
    if t >= last.0 {
        return Some(last.1);
    }
    let i = keys.partition_point(|k| k.0 <= t);
    let (a, b) = (keys[i - 1], keys[i]);
    let s = if b.0 > a.0 { (t - a.0) / (b.0 - a.0) } else { 0.0 };
    Some(a.1.slerp(b.1, s))
}

/// Every bone's matrix at clip time `t`.
pub fn pose(bones: &[Bone], tracks: &[Tracks], t: f32) -> Vec<Mat4> {
    let n = bones.len();
    let mut m: Vec<Option<Mat4>> = vec![None; n];
    fn at(i: usize, bones: &[Bone], tracks: &[Tracks], t: f32, m: &mut Vec<Option<Mat4>>, depth: u32) -> Mat4 {
        if let Some(x) = m[i] {
            return x;
        }
        let parent = match usize::try_from(bones[i].parent).ok().filter(|&p| p < bones.len() && p != i) {
            Some(p) if depth < 256 => at(p, bones, tracks, t, m, depth + 1),
            _ => Mat4::IDENTITY,
        };
        let tr = tracks.get(i);
        let shift = tr.and_then(|x| sample_vec(&x.translation, t)).unwrap_or(Vec3::ZERO);
        let turn = tr.and_then(|x| sample_quat(&x.rotation, t)).unwrap_or(Quat::IDENTITY).normalize();
        let p = bones[i].pivot;
        let x = parent * Mat4::from_translation(p + shift) * Mat4::from_quat(turn) * Mat4::from_translation(-p);
        m[i] = Some(x);
        x
    }
    (0..n).map(|i| at(i, bones, tracks, t, &mut m, 0)).collect()
}

/// The bone carrying `KeyBoneID` `key`, if any.
pub fn key_bone(bones: &[Bone], key: i16) -> Option<usize> {
    bones.iter().position(|b| b.key == key)
}

/// Measure a clip. `body` is the waist (or the spine), `head` the head, `hand` and `off` the hands
/// (attachment points 1 and 2).
#[allow(clippy::too_many_arguments)]
pub fn measure(bones: &[Bone], tracks: &[Tracks], duration: f32, impact: f32, body: usize, head: usize, hand: usize, off: Option<usize>) -> Motion {
    const N: usize = 80;
    let height = bones[head].pivot.z.max(bones.iter().map(|b| b.pivot.z).fold(0.0, f32::max)).max(0.1);
    let place = |m: &[Mat4], i: usize| m[i].transform_point3(bones[i].pivot);
    let times: Vec<f32> = (0..=N).map(|k| duration * k as f32 / N as f32).collect();
    let poses: Vec<Vec<Mat4>> = times.iter().map(|&t| pose(bones, tracks, t)).collect();
    let body0 = place(&poses[0], body);
    let up0 = poses[0][body].transform_vector3(Vec3::Z).normalize_or_zero();
    let mut rise: f32 = 0.0;
    let mut tilt: f32 = 0.0;
    let mut spin = 0.0;
    let mut last_yaw: Option<f32> = None;
    for m in &poses {
        let b = place(m, body);
        rise = rise.max(b.z - body0.z);
        let up = m[body].transform_vector3(Vec3::Z).normalize_or_zero();
        tilt = tilt.max(up.dot(up0).clamp(-1.0, 1.0).acos().to_degrees());
        let fwd = m[body].transform_vector3(Vec3::X);
        if fwd.truncate().length() > 1e-3 {
            let yaw = fwd.y.atan2(fwd.x);
            if let Some(ly) = last_yaw {
                let mut d = yaw - ly;
                while d > std::f32::consts::PI {
                    d -= std::f32::consts::TAU;
                }
                while d < -std::f32::consts::PI {
                    d += std::f32::consts::TAU;
                }
                spin += d;
            }
            last_yaw = Some(yaw);
        }
    }
    let k_impact = if impact > 0.0 { ((impact / duration.max(1e-3)) * N as f32).round() as usize } else { N / 2 };
    let k_impact = k_impact.clamp(1, N);
    let at_impact = place(&poses[k_impact], body);
    // The stroke: the second half of the way into the blow.
    let k_from = k_impact / 2;
    let rel = |m: &[Mat4], i: usize| place(m, i) - place(m, body);
    let path = |i: usize| -> (Vec3, f32, f32) {
        let d = rel(&poses[k_impact], i) - rel(&poses[k_from], i);
        let mut len = 0.0;
        let mut fastest: f32 = 0.0;
        for k in k_from..k_impact {
            let step = (place(&poses[k + 1], i) - place(&poses[k], i)).length();
            len += step;
            fastest = fastest.max(step / (duration / N as f32).max(1e-4));
        }
        (d, len, fastest)
    };
    let (d, len, fastest) = path(hand);
    let off_leads = off.is_some_and(|o| path(o).1 > len * 1.25);
    let (d, fastest) = match off {
        Some(o) if off_leads => {
            let p = path(o);
            (p.0, p.2)
        }
        _ => (d, fastest),
    };
    let lead = if off_leads { off.unwrap_or(hand) } else { hand };
    let raised = (0..=k_impact).map(|k| place(&poses[k], lead).z - place(&poses[k], head).z).fold(f32::MIN, f32::max);
    Motion {
        duration,
        impact,
        rise: rise / height,
        tilt,
        spin: spin.to_degrees().abs(),
        travel_fwd: at_impact.x - body0.x,
        travel_side: at_impact.y - body0.y,
        stroke_up: d.z / height,
        stroke_side: d.y / height,
        stroke_fwd: d.x / height,
        raised: raised / height,
        speed: fastest / height,
        off_hand_leads: off_leads,
    }
}

/// What a measured clip is, in a few words, and the kinds of blow it fits.
pub fn describe(m: &Motion) -> (Vec<&'static str>, Vec<&'static str>) {
    let mut tags = Vec::new();
    let flip = m.tilt > 100.0;
    let leap = m.rise > 0.2;
    let spin = m.spin > 200.0;
    let lunge = m.travel_fwd.hypot(m.travel_side) > 0.8;
    let quick = m.impact > 0.0 && m.impact < 0.32;
    let slow = m.impact > 0.55;
    if flip {
        tags.push("flip");
    }
    if leap {
        tags.push("leap");
    }
    if spin {
        tags.push("spin");
    }
    if lunge {
        tags.push("lunge");
    }
    if quick {
        tags.push("quick");
    }
    if slow {
        tags.push("slow");
    }
    let (up, side, fwd) = (m.stroke_up, m.stroke_side.abs(), m.stroke_fwd);
    let stroke = if m.raised > 0.0 && up < -0.3 && -up >= side {
        "overhead"
    } else if up > 0.3 && up >= side {
        "upswing"
    } else if side > 0.3 && side >= fwd.abs() {
        "sweep"
    } else if fwd > 0.25 {
        "thrust"
    } else if up < -0.25 {
        "chop"
    } else {
        "short"
    };
    tags.push(stroke);
    if m.off_hand_leads {
        tags.push("off-hand");
    }
    let mut fits = Vec::new();
    if quick && !flip && !leap && !spin {
        fits.push("flurry");
    }
    if !flip && !leap && !spin && !quick && !slow {
        fits.push("cut");
    }
    if slow && matches!(stroke, "overhead" | "chop") && !flip {
        fits.push("heavy");
    }
    if stroke == "sweep" || spin {
        fits.push("wide");
    }
    if lunge || leap {
        fits.push("gap closer");
    }
    if flip || spin || (leap && slow) {
        fits.push("boss special");
    }
    if fits.is_empty() {
        fits.push("cut");
    }
    (tags, fits)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stick body: a root on the ground, hips a yard up, a head over them, a right arm out to
    /// the side at the shoulder with a hand on its end.
    fn body() -> Vec<Bone> {
        vec![
            Bone { parent: -1, pivot: Vec3::ZERO, key: -1 },
            Bone { parent: 0, pivot: Vec3::new(0.0, 0.0, 1.0), key: 5 },
            Bone { parent: 1, pivot: Vec3::new(0.0, 0.0, 1.8), key: 6 },
            Bone { parent: 1, pivot: Vec3::new(0.0, -0.3, 1.5), key: 3 },
            Bone { parent: 3, pivot: Vec3::new(0.0, -0.3, 0.9), key: -1 },
        ]
    }

    fn still() -> Vec<Tracks> {
        vec![Tracks::default(); 5]
    }

    #[test]
    fn an_overhead_chop_raises_the_hand_over_the_head_and_brings_it_down() {
        let b = body();
        let mut t = still();
        // The arm swings from straight up (the hand over the head) to forward-down by the impact.
        let up = Quat::from_rotation_x(std::f32::consts::PI);
        let down = Quat::from_rotation_y(-0.6);
        t[3].rotation = vec![(0.0, up), (0.3, up), (0.6, down), (1.0, down)];
        let m = measure(&b, &t, 1.0, 0.6, 1, 2, 4, None);
        let (tags, fits) = describe(&m);
        assert!(m.raised > 0.0, "raised over the head: {m:?}");
        assert!(tags.contains(&"overhead"), "{tags:?} {m:?}");
        assert!(tags.contains(&"slow") && fits.contains(&"heavy"), "{tags:?} {fits:?}");
        assert!(m.rise.abs() < 1e-4 && m.tilt < 1.0, "the body stays put");
    }

    #[test]
    fn a_leaping_flip_is_a_boss_special_not_a_flurry() {
        let b = body();
        let mut t = still();
        // The hips rise a yard and turn head over heels (about the side axis), back by the end.
        t[1].translation = vec![(0.0, Vec3::ZERO), (0.4, Vec3::new(0.5, 0.0, 1.0)), (0.8, Vec3::new(1.2, 0.0, 0.0))];
        t[1].rotation = vec![
            (0.0, Quat::IDENTITY),
            (0.2, Quat::from_rotation_y(2.1)),
            (0.4, Quat::from_rotation_y(4.2)),
            (0.6, Quat::from_rotation_y(6.28)),
        ];
        let m = measure(&b, &t, 1.0, 0.7, 1, 2, 4, None);
        let (tags, fits) = describe(&m);
        assert!(tags.contains(&"flip") && tags.contains(&"leap"), "{tags:?} {m:?}");
        assert!(fits.contains(&"boss special") && fits.contains(&"gap closer"), "{fits:?}");
        assert!(!fits.contains(&"flurry"));
    }

    #[test]
    fn a_quick_sweep_and_a_spin() {
        let b = body();
        let mut t = still();
        // A quick swing round from the right side across the front.
        t[3].rotation = vec![(0.0, Quat::IDENTITY), (0.12, Quat::from_rotation_x(-1.4)), (0.25, Quat::from_rotation_x(1.4) * Quat::from_rotation_z(1.5)), (0.5, Quat::IDENTITY)];
        let m = measure(&b, &t, 0.5, 0.25, 1, 2, 4, None);
        let (tags, fits) = describe(&m);
        assert!(tags.contains(&"quick"), "{tags:?} {m:?}");
        assert!(fits.contains(&"flurry"), "{fits:?}");
        // The whole body turning a full circle about the vertical.
        let mut s = still();
        s[1].rotation = (0..=8).map(|k| (k as f32 * 0.1, Quat::from_rotation_z(k as f32 * std::f32::consts::FRAC_PI_4))).collect();
        let m = measure(&b, &s, 0.8, 0.6, 1, 2, 4, None);
        assert!(m.spin > 300.0, "{m:?}");
        assert!(describe(&m).0.contains(&"spin"));
    }
}
