//! The piles, fitted to the installed map when the preview starts. A pile slumps against the rock
//! it came out of: one large mass with its flat cut face turned into the wall, a shoulder and
//! perched pieces of a middle size, small stones and a soil skirt spread across the floor toward
//! the path. Over it the roof is broken (a beam snapped out of it), a post
//! leans on it and boards lie at its foot at mixed angles, some pinned under it. Sealed side
//! passages fill to their lintels. Every piece is seated on the floor or on the rock below it, and
//! a piece that would pass through a wall or one of the mine's own doodads is left out.

use std::f32::consts::{FRAC_PI_2, PI};

use super::shapes::{add, cross, frame, len, matrix, mul, sub, turn, unit, V3};
use super::*;

/// The flat cut face of the rock's mesh looks along model yaw -37 degrees: turning a rock by this
/// much more than a direction faces it that way (views 14 and 20 showed it to the path).
const FACE: f32 = 0.65;
/// The beam model's length (its iron-shod foot at -5.77 to its end at 23.87) and its radius.
const BEAM_LEN: f32 = 29.64;
const BEAM_RADIUS: f32 = 1.0;
/// The board model's length.
const BOARD_LEN: f32 = 11.39;
/// Roofs higher than this over a pile's floor are out of sight: no beam snapped out of them.
const ROOF_REACH: f32 = 15.0;

/// A small authored jitter in [-1, 1]: the same on every run, different at every site and piece.
fn jit(seed: usize, k: usize) -> f32 {
    let mut h = (seed as u32 + 1).wrapping_mul(0x9E37_79B1) ^ (k as u32 + 7).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h % 2001) as f32 / 1000.0 - 1.0
}

/// A pile's own directions: `f` into the wall it slumps against (or through the doorway it fills),
/// `s` across. `at(u, v)` is `u` along `f` and `v` along `s` from the site.
#[derive(Clone, Copy)]
struct Axes {
    o: [f32; 2],
    f: [f32; 2],
    s: [f32; 2],
}

impl Axes {
    fn new(o: [f32; 3], angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self { o: [o[0], o[1]], f: [c, s], s: [-s, c] }
    }
    fn at(&self, u: f32, v: f32) -> [f32; 2] {
        [self.o[0] + self.f[0]*u + self.s[0]*v, self.o[1] + self.f[1]*u + self.s[1]*v]
    }
    fn angle(&self) -> f32 { self.f[1].atan2(self.f[0]) }
}

struct Builder<'a> {
    g: &'a Ground,
    props: Vec<Prop>,
    region: usize,
    site: &'static str,
    wet: bool,
    seed: usize,
    k: usize,
    /// The floor height the current site's pieces are found from.
    near: f32,
    /// Rock heaped onto a pile keeps under the roof (a slab through the ceiling of a beamed
    /// tunnel reads as a hole in it).
    under_roof: bool,
}

impl Builder<'_> {
    fn jit(&mut self) -> f32 {
        self.k += 1;
        jit(self.seed, self.k)
    }

    fn floor_at(&self, x: f32, y: f32, near: f32) -> Option<f32> {
        floor(&*self.g.land, [x, y, near], self.wet)
    }

    /// The highest surface under (x, y): the floor, or a rock or mound already in the pile.
    fn support(&self, x: f32, y: f32, near: f32) -> Option<f32> {
        let fl = self.floor_at(x, y, near)?;
        Some(self.props.iter().filter(|p| rocky(p.model) && (p.pos[0]-x).abs() < 6.0 * p.scale && (p.pos[1]-y).abs() < 6.0 * p.scale)
            .filter_map(|p| self.g.top_at(p, x, y)).filter(|t| *t < near + 12.0).fold(fl, f32::max))
    }

    fn prop(&self, model: usize, pos: V3, scale: f32, q: [f32; 4]) -> Prop {
        let m = matrix(q);
        Prop { region: self.region, site: self.site, model, pos, scale, rotation: q, facing: m[1][0].atan2(m[0][0]) }
    }

    /// The floor under a point, or over it where it is buried just under one ([`floor_under`]).
    fn floor_by(&self, v: V3) -> Option<f32> {
        let f = floor_under(&*self.g.land, v);
        f.is_finite().then_some(f)
    }

    /// The roof over a point of a piece whose lowest point is `low`, found from the floor there.
    fn roof_over(&self, v: V3, low: f32) -> Option<f32> {
        self.g.roof([v[0], v[1], floor_under(&*self.g.land, [v[0], v[1], low])])
    }

    /// A beam lodged in the roof by one end hangs there; nothing else does.
    fn anchored(&self, p: &Prop, points: &[V3]) -> bool {
        let low = points.iter().map(|v| v[2]).fold(f32::INFINITY, f32::min);
        p.model == BEAM && points.iter().any(|v| self.roof_over(*v, low).is_some_and(|r| v[2] > r + 0.25))
    }

    /// Wood and iron rest when they touch the floor or rock on both sides of their middle along
    /// one of their two flattest axes (a board on its two ends, a cart on its side, a plank with
    /// its foot on the floor and its top on the rock), or hang by an end lodged in the roof.
    fn rests(&self, p: &Prop, points: &[V3]) -> bool {
        if self.wet || rocky(p.model) { return true; }
        // Nothing sunk past a buried end after settling (a stair the floor finds under it).
        if self.below_floor(points, p.model == BEAM).any(|d| d > 0.3) { return false; }
        // Water is the cove's: nothing on the dry ground hangs out over it.
        if points.iter().any(|v| self.floor_by(*v).is_some_and(|f| self.g.land.water_near(v[0], v[1], f))) { return false; }
        if self.anchored(p, points) { return true; }
        let b = p.obb(0.0);
        let flat = rest_axes(&b);
        let rocks = self.rocks_near(&b);
        let mut ends = [[false; 2]; 2];
        for v in points {
            let Some(fl) = self.floor_by(*v) else { continue };
            let s = rocks.iter().filter_map(|q| self.g.top_at(q, v[0], v[1])).filter(|t| *t < v[2] + 0.6).fold(fl, f32::max);
            let pinned = rocks.iter().any(|q| self.g.holds_within(q, *v, -0.06));
            if !(pinned || (-0.3..=0.12).contains(&(v[2] - s))) { continue; }
            for (axis, (a, h)) in flat.iter().enumerate() {
                let d = shapes::dot(sub(*v, b.c), *a);
                if d < -0.3 * h { ends[axis][0] = true; }
                if d > 0.3 * h { ends[axis][1] = true; }
            }
        }
        ends.iter().any(|e| e[0] && e[1])
    }

    /// The rocks and mounds near a box, upright (what a piece there could rest on or bite into).
    fn rocks_near(&self, b: &Obb) -> Vec<&Prop> {
        let r = len(b.h);
        self.props.iter().filter(|q| rocky(q.model) && !q.hangs()).filter(|q| {
            let o = q.obb(0.0);
            (o.c[0] - b.c[0]).hypot(o.c[1] - b.c[1]) < r + len(o.h) + 0.5
        }).collect()
    }

    /// Lift wood and iron out of a floor that rises under them (a board over a hump, a crate on a
    /// slope): no point further than a buried end into the floor.
    fn settle(&self, p: &mut Prop) {
        if self.wet || rocky(p.model) { return; }
        // A stepped floor shows its next step up only once the piece has risen past this one; a
        // floor more than a stair over the piece is a wall it runs into, not one to climb.
        for _ in 0..4 {
            let points = self.g.points(p);
            let deepest = self.below_floor(&points, p.model == BEAM).fold(f32::MIN, f32::max);
            if !(0.15..=0.8).contains(&deepest) { return; }
            p.pos[2] += deepest - 0.15;
        }
    }

    /// How far each of a piece's points is under the floor at it (negative: over it), leaving out
    /// a beam's points inside the roof (its lodged end).
    fn below_floor<'p>(&'p self, points: &'p [V3], beam: bool) -> impl Iterator<Item = f32> + 'p {
        let low = points.iter().map(|v| v[2]).fold(f32::INFINITY, f32::min);
        points.iter().filter(move |v| !(beam && self.roof_over(**v, low).is_some_and(|r| v[2] > r - 0.6)))
            .filter_map(|v| self.floor_by(*v).map(|f| f - v[2]))
    }

    /// Whether a piece may stand here: never through one of the mine's doodads, wood and iron
    /// never through a wall or each other, never swallowed whole by rock, and nothing knee-high
    /// across the walk from the entrance to the cove.
    fn fits(&self, p: &Prop, points: &[V3]) -> bool {
        if p.pos.iter().any(|v| !v.is_finite()) || covers_view(p) { return false; }
        let rock = rocky(p.model);
        let whole = p.obb(0.0);
        for (model, n) in self.g.natives() {
            if whole.overlaps(n) && (if rock { rock_through(model, n, 1.3, |s| self.g.holds_within(p, s, 0.2)) } else { points.iter().any(|v| n.contains(*v)) }) {
                return false;
            }
        }
        if !rock && self.into_wall(p, points) { return false; }
        let reach = len(whole.h) + 1.0;
        let tall = |at: &V3| points.iter().any(|v| v[2] > at[2] + 0.7);
        if self.g.route().iter().any(|at| (at[0]-whole.c[0]).hypot(at[1]-whole.c[1]) < reach && tall(at) && covers(p, *at, 0.6)) {
            return false;
        }
        let r = len(whole.h);
        self.props.iter().all(|q| {
            let o = q.obb(0.0);
            if (o.c[0] - whole.c[0]).hypot(o.c[1] - whole.c[1]) > r + len(o.h) || !o.overlaps(&whole) { return true; }
            match (rock, rocky(q.model)) {
                (false, false) => !points.iter().any(|v| q.obb(0.15).contains(*v)) && !self.g.points(q).iter().any(|v| p.obb(0.15).contains(*v)),
                (false, true) => !self.g.holds(q, whole.c),
                (true, false) => !self.g.holds(p, q.obb(0.0).c),
                (true, true) => true,
            }
        })
    }

    /// Wood or iron reaching through rock: a line from its middle to any of its points meets a
    /// wall or the roof (each end lifted a knee's height off the floor under it, so the floor
    /// itself doesn't count; a beam's end lodged in the roof is meant to be there).
    fn into_wall(&self, p: &Prop, points: &[V3]) -> bool {
        let c = p.obb(0.0).c;
        let Some(fc) = self.floor_by(c) else { return true };
        let from = [c[0], c[1], c[2].max(fc + 0.35)];
        let anchored = self.anchored(p, points);
        let low = points.iter().map(|v| v[2]).fold(f32::INFINITY, f32::min);
        // Its box's corners, edges and faces, and every few of its own points (enough to find a
        // corner in the rock without a line to each of a crate's four hundred).
        let step = (points.len() / 48).max(1);
        let mut probes = p.obb(0.0).samples();
        probes.extend(points.iter().step_by(step));
        probes.iter().any(|s| {
            if anchored && self.roof_over(*s, low).is_some_and(|r| s[2] > r - 0.6) { return false; }
            let fz = self.floor_by(*s).unwrap_or(fc);
            let to = [s[0], s[1], s[2].max(fz + 0.35)];
            let d = sub(to, from);
            let l = len(d);
            l > 0.05 && self.g.ray(from, unit(d), l).is_some()
        })
    }

    fn place(&mut self, mut p: Prop) -> Option<usize> {
        self.settle(&mut p);
        let points = self.g.points(&p);
        let ok = self.rests(&p, &points) && self.fits(&p, &points);
        ok.then(|| {
            self.props.push(p);
            self.props.len() - 1
        })
    }

    /// A rock seated on whatever is under its skirt: the skirt's rim stays buried wherever the
    /// floor (or the rock it lands on) is lowest, so its open underside never shows.
    fn rock(&mut self, at: [f32; 2], near: f32, scale: f32, yaw: f32) -> Option<usize> {
        self.stone(ROCK, at, near, scale, yaw)
    }

    fn stone(&mut self, model: usize, at: [f32; 2], near: f32, scale: f32, yaw: f32) -> Option<usize> {
        let lean = [0.04 * self.jit(), 0.04 * self.jit()];
        let q = shapes::euler(lean[0], lean[1], yaw);
        let m = matrix(q);
        // Where its rim runs into the wall there is no floor to bury it in: the wall's rock hides it.
        let rim = self.g.rim(ROCK);
        let mut sunk = f32::INFINITY;
        for r in &rim {
            let w = turn(&m, mul(*r, scale));
            match self.support(at[0] + w[0], at[1] + w[1], near) {
                Some(s) => sunk = sunk.min(s - w[2]),
                // (Heaped rock sits whole on what's there: a rail at knee height is not a wall.)
                None if !self.under_roof && self.in_wall([at[0], at[1], near], [at[0] + w[0], at[1] + w[1]]) => {}
                None => return None,
            }
        }
        let Some(fl) = self.floor_at(at[0], at[1], near).filter(|_| sunk.is_finite()) else { return None };
        let p = self.prop(model, [at[0], at[1], sunk - 0.25 * scale - 0.1], scale, q);
        if self.under_roof {
            let top = self.g.points(&p).iter().map(|v| v[2]).fold(f32::MIN, f32::max);
            if self.g.roof([at[0], at[1], fl]).is_some_and(|r| top > r - 0.3) { return None; }
            // Its rim buried all round in the floor as the floor is found from the rim itself, or
            // in rock (not hanging over a drop the floor near the pile hides).
            for r in &rim {
                let w = add(p.pos, turn(&m, mul(*r, scale)));
                let rock = self.props.iter().filter(|q| rocky(q.model) && (q.pos[0] - w[0]).hypot(q.pos[1] - w[1]) < 8.0 * q.scale)
                    .filter_map(|q| self.g.top_at(q, w[0], w[1])).fold(f32::MIN, f32::max);
                if w[2] - floor_under(&*self.g.land, w).max(rock) > 0.05 { return None; }
            }
        }
        self.place(p)
    }

    /// Whether a point is inside the wall, seen from a floor point: a line across at knee height
    /// meets rock before it.
    fn in_wall(&self, from: V3, to: [f32; 2]) -> bool {
        let d = [to[0] - from[0], to[1] - from[1], 0.0];
        let l = len(d);
        l > 0.05 && self.g.ray([from[0], from[1], from[2] + 0.8], unit(d), l).is_some()
    }

    /// A rock at the first of `spots` (along and across a pile's axes) where it fits.
    fn rock_at(&mut self, ax: Axes, spots: &[(f32, f32)], near: f32, scale: f32, yaw: f32) -> Option<usize> {
        // Smaller where none of the spots has room for it: still a piece of the middle size.
        [1.0, 0.85, 0.7].iter().find_map(|f| spots.iter().find_map(|&(u, v)| self.rock(ax.at(u, v), near, scale * f, yaw)))
    }

    /// Gravel spread across the floor round `at`: half a dozen small dark stones (`scale` the
    /// spread's size), sunk to their crowns. The first stone, if any lay.
    fn mound(&mut self, model: usize, at: [f32; 2], near: f32, scale: f32, yaw: f32) -> Option<usize> {
        let mut first = None;
        for k in 0..if model == SKIRT { 4 } else { 3 } {
            let a = yaw + k as f32 * 2.4;
            let r = scale * (0.4 + 0.25 * k as f32) * (1.0 + 0.2 * self.jit());
            let size = if model == SKIRT { 0.06 + 0.02 * (k % 3) as f32 } else { 0.1 + 0.03 * (k % 2) as f32 };
            let p = self.stone(model, [at[0] + r * a.cos(), at[1] + r * a.sin()], near, size, a * 1.7);
            first = first.or(p);
        }
        first
    }

    /// A board whose underside runs from `a` to `b`, its broad face up and canted by `tilt`.
    fn board(&mut self, a: V3, b: V3, tilt: f32) -> Option<usize> {
        let l = len(sub(b, a));
        let d = unit(sub(b, a));
        let side = unit(cross([0.0, 0.0, 1.0], d));
        let up = unit(cross(d, side));
        let n = add(mul(up, tilt.cos()), mul(side, tilt.sin()));
        let q = frame(n, d);
        let scale = l / BOARD_LEN;
        let pos = sub(a, turn(&matrix(q), mul([-0.28, -5.57, 1.145], scale)));
        let p = self.prop(BOARD, pos, scale, q);
        self.place(p)
    }

    /// A beam whose axis runs from its iron-shod foot at `a` to its end at `b`.
    fn beam(&mut self, a: V3, b: V3) -> Option<usize> {
        let l = len(sub(b, a));
        let d = unit(sub(b, a));
        let side = if d[2].abs() > 0.97 { [1.0, 0.0, 0.0] } else { unit(cross([0.0, 0.0, 1.0], d)) };
        let q = frame(side, cross(d, side));
        let scale = l / BEAM_LEN;
        let pos = sub(a, turn(&matrix(q), mul([0.0, 0.1, -5.77], scale)));
        let p = self.prop(BEAM, pos, scale, q);
        self.place(p)
    }

    /// A board or beam of length `l` from a foot on the floor at `foot`, leaning toward `toward`
    /// at the lowest angle at which it clears the rock it comes to rest on (`clear` is how far
    /// its axis stands off a surface): its upper end, if it does come to rest on something.
    fn lean(&self, foot: V3, toward: [f32; 2], l: f32, clear: f32, on: usize) -> Option<V3> {
        let d = unit([toward[0] - foot[0], toward[1] - foot[1], 0.0]);
        // The step below the angle that clears everything met the rock where along its length:
        // it rests there only if that was its upper part (a stone by its foot holds nothing up).
        let mut met = None;
        for step in 0..40 {
            let a = (6.0 + step as f32).to_radians();
            let dir = [d[0] * a.cos(), d[1] * a.cos(), a.sin()];
            let mut hit = None;
            for i in 3..=20 {
                let p = add(foot, mul(dir, l * i as f32 / 20.0));
                let fl = self.floor_at(p[0], p[1], foot[2] + 0.5)?;
                let s = self.g.top_at(&self.props[on], p[0], p[1]).map_or(fl, |t| t.max(fl));
                if p[2] < s + clear { hit = Some(i); break; }
            }
            match hit {
                Some(i) => met = Some(i),
                // It ends where it meets the rock, its end bitten a little in: a piece reaching on
                // past the touch would hang its end over the far side of the dome.
                None => return met.filter(|i| *i >= 10).map(|i| add(add(foot, mul(dir, l * i as f32 / 20.0 + 0.5)), [0.0, 0.0, -0.12])),
            }
        }
        None
    }

    /// A board or beam leaning on rock `on`: its foot on the floor just past the rock's edge, out
    /// from its middle toward `out` (radians), its top resting on the rock.
    fn against(&mut self, on: usize, out: f32, l: f32, beam: bool) -> Option<usize> {
        let c = self.props[on].pos;
        let d = [out.cos(), out.sin()];
        let edge = (1..80).map(|k| k as f32 * 0.25)
            .find(|r| self.g.top_at(&self.props[on], c[0] + d[0] * r, c[1] + d[1] * r).is_none())?;
        for extra in [0.5, 1.0, 1.6] {
            let (x, y) = (c[0] + d[0] * (edge + extra), c[1] + d[1] * (edge + extra));
            let Some(fz) = self.floor_at(x, y, self.near) else { continue };
            let r = BEAM_RADIUS * l / BEAM_LEN;
            let foot = [x, y, if beam { fz + 1.2 * r } else { fz + 0.03 }];
            let Some(top) = self.lean(foot, [c[0], c[1]], l, if beam { 0.8 * r } else { 0.0 }, on) else { continue };
            let placed = if beam { self.beam(foot, top) } else { let tilt = 0.25 * self.jit(); self.board(foot, top, tilt) };
            if placed.is_some() { return placed; }
        }
        None
    }

    /// A model knocked over and seated by its turned box's lowest point.
    fn tipped(&mut self, model: usize, at: [f32; 2], near: f32, scale: f32, roll: f32, pitch: f32, yaw: f32) -> Option<usize> {
        let q = shapes::euler(roll, pitch, yaw);
        let m = &MODELS[model];
        let r = 0.5 * (m.max[0] - m.min[0]).max(m.max[1] - m.min[1]) * scale;
        let mut low = self.floor_at(at[0], at[1], near)?;
        for a in 0..4 {
            let t = a as f32 * FRAC_PI_2 + yaw + 0.4;
            low = low.min(self.floor_at(at[0] + r * t.cos(), at[1] + r * t.sin(), near)?);
        }
        let p = self.prop(model, [at[0], at[1], low - self.g.low(model, q) * scale - 0.03], scale, q);
        self.place(p)
    }

    /// A board lying on the floor from `a` (yaw `yaw`, `l` long), following the floor's slope.
    fn floor_board(&mut self, a: [f32; 2], near: f32, yaw: f32, l: f32) -> Option<usize> {
        let b = [a[0] + l * yaw.cos(), a[1] + l * yaw.sin()];
        let za = self.support(a[0], a[1], near)? + 0.03;
        let zb = self.support(b[0], b[1], near)? + 0.03;
        let tilt = 0.12 * self.jit();
        self.board([a[0], a[1], za], [b[0], b[1], zb], tilt)
    }

    /// A short beam lying on the floor.
    fn floor_beam(&mut self, a: [f32; 2], near: f32, yaw: f32, l: f32) -> Option<usize> {
        let b = [a[0] + l * yaw.cos(), a[1] + l * yaw.sin()];
        let r = BEAM_RADIUS * l / BEAM_LEN;
        let za = self.support(a[0], a[1], near)? + r;
        let zb = self.support(b[0], b[1], near)? + r;
        self.beam([a[0], a[1], za], [b[0], b[1], zb])
    }

    /// The wall a pile of size `sl` slumps against: the nearest, unless the way through runs past
    /// it, then the one that leaves the pile farthest from the way.
    fn wall_off_the_way(&self, at: V3, max: f32, sl: f32) -> Option<(f32, f32)> {
        let walls: Vec<(f32, f32)> = (0..16).filter_map(|k| {
            let a = k as f32 * PI / 8.0;
            self.g.ray([at[0], at[1], at[2] + 1.2], [a.cos(), a.sin(), 0.0], max).map(|d| (a, d))
        }).collect();
        let near_way: Vec<V3> = self.g.route().iter().filter(|r| (r[0]-at[0]).hypot(r[1]-at[1]) < 16.0 && (r[2]-at[2]).abs() < 4.0).copied().collect();
        if near_way.is_empty() {
            return walls.into_iter().min_by(|x, y| x.1.total_cmp(&y.1));
        }
        let clear = |&(a, d): &(f32, f32)| {
            let u = (d - 1.5 * sl).clamp(-1.0, 4.0);
            let c = [at[0] + a.cos() * u, at[1] + a.sin() * u];
            near_way.iter().map(|r| (r[0]-c[0]).hypot(r[1]-c[1])).fold(f32::INFINITY, f32::min)
        };
        walls.into_iter().max_by(|x, y| (clear(x) - 0.2 * x.1).total_cmp(&(clear(y) - 0.2 * y.1)))
    }

    /// The narrowest way across a passage at a point: the direction along it, and how far its
    /// walls stand to either side (left, right).
    fn passage(&self, at: V3) -> Option<(f32, f32, f32)> {
        (0..12).filter_map(|k| {
            let a = k as f32 * PI / 12.0;
            let side = |t: f32| self.g.ray([at[0], at[1], at[2] + 1.2], [t.cos(), t.sin(), 0.0], 14.0);
            Some((a, side(a + FRAC_PI_2)?, side(a - FRAC_PI_2)?))
        }).min_by(|x, y| (x.1 + x.2).total_cmp(&(y.1 + y.2)))
    }

    /// A pile's foot: small stones, the soil skirt and the timber heaped where it ends.
    fn foot(&mut self, ax: Axes, z0: f32, front: f32, main: Option<usize>, k: f32, hand: f32) {
        let yaw0 = ax.angle();
        let mut down = 0;
        for j in 0..14 {
            if down == 5 { break; }
            let u = front - (0.6 + 1.1 * (j % 3) as f32 + 0.5 * self.jit()) * k;
            let v = (-3.2 + 1.3 * (j % 6) as f32 + 0.5 * self.jit()) * k * hand;
            let s = (0.12 + 0.05 * ((j * 7) % 3) as f32 + 0.02 * self.jit()) * k.min(1.3);
            let yaw = yaw0 + FACE + 2.1 * j as f32;
            if self.rock(ax.at(u - (j / 6) as f32 * 0.8 * k, v), z0, s, yaw).is_some() { down += 1; }
        }
        let sk = 1.1 * k.min(1.3);
        let yaw = self.jit() * PI;
        let h = hand * k;
        for (u, v, f) in [(0.4, 0.6, 1.0), (-0.6, -1.4, 0.85), (0.0, 2.4, 0.75), (-1.4, 0.0, 0.6)] {
            if self.mound(SKIRT, ax.at(front + u * k, v * h), z0, sk * f, yaw).is_some() { break; }
        }
        let yaw = yaw0 + 1.2 + 0.4 * self.jit();
        for (u, v) in [(-1.6, 2.2), (-2.4, -1.0), (-1.0, 3.2)] {
            if self.mound(SCREE, ax.at(front + u * k, v * h), z0, 1.1 * k.min(1.3), yaw).is_some() { break; }
        }
        // A board pinned under the main mass: it runs in under the rock's skirt at floor level.
        if let Some(m) = main {
            let c = [self.props[m].pos[0], self.props[m].pos[1]];
            let a = ax.at(front - 1.2 * k, 1.4 * hand * k);
            let dir = unit([c[0] - a[0], c[1] - a[1], 0.0]);
            let yaw = dir[1].atan2(dir[0]) + 0.35 * hand;
            let l = (3.2 + 0.4 * self.jit()) * k.min(1.3);
            self.floor_board(a, z0, yaw, l);
        }
        // One leaning on the pile, its foot on the floor.
        if let Some(m) = main {
            let l = (3.4 + 0.5 * self.jit()) * k.min(1.3);
            let out = yaw0 + PI - 0.5 * hand;
            for turn in [0.0, 0.6, -0.6, 1.2, -1.2, 1.9, -1.9] {
                if self.against(m, out + turn, l, false).is_some() { break; }
            }
        }
        // Two loose on the floor at angles of their own, and a broken length of beam.
        let yaw = yaw0 + 2.0 + 0.8 * self.jit();
        let l = (2.6 + 0.6 * self.jit()) * k.min(1.3);
        for (u, v) in [(-2.8, 0.2), (-2.0, 1.6), (-3.6, -0.8), (-1.6, -1.8)] {
            if self.floor_board(ax.at(front + u * k, v * h), z0, yaw, l).is_some() { break; }
        }
        let yaw = yaw0 - 1.1 + 0.8 * self.jit();
        let l = (2.2 + 0.5 * self.jit()) * k.min(1.3);
        for (u, v) in [(-3.4, -2.4), (-2.6, 2.8), (-4.2, 1.2)] {
            if self.floor_board(ax.at(front + u * k, v * h), z0, yaw, l).is_some() { break; }
        }
        let yaw = yaw0 + FRAC_PI_2 + 0.6 * self.jit();
        let l = (3.0 + 0.6 * self.jit()) * k.min(1.3);
        for (u, v) in [(-0.6, 2.8), (-1.8, -3.0), (-3.0, 2.0)] {
            if self.floor_beam(ax.at(front + u * k, v * h), z0, yaw, l).is_some() { break; }
        }
    }

    /// Where the rock fell from: a beam snapped out of the roof onto the pile (or hanging from it),
    /// and a support post knocked over and leaning on the pile, no steeper than 45 degrees. (Rock
    /// hung from the roof read on film as a smooth lump floating under it, and is gone.)
    fn broken_roof(&mut self, ax: Axes, z0: f32, um: f32, sl: f32, hand: f32, face: f32, main: Option<usize>) {
        let over = ax.at(um - 0.8 * sl, 0.3 * hand * sl);
        let Some(fl) = self.floor_at(over[0], over[1], z0) else { return };
        let Some(roof) = self.g.roof([over[0], over[1], fl]) else { return };
        if roof - fl > ROOF_REACH { return; }
        let _ = face;
        // The beam: out of the roof beside the scar, down onto the pile, from the first of a few
        // places in the roof over the pile to the first place on the pile it reaches. Never a
        // stub left hanging from the roof: on film one read as a pole floating in the air.
        'beam: for (du, dv) in [(-2.0, 1.7), (-1.2, -1.8), (-2.6, -0.4), (-0.6, 2.4), (-1.6, 0.4), (0.0, -2.6)] {
            let t = ax.at(um + du * sl, dv * hand * sl);
            let Some(tf) = self.floor_at(t[0], t[1], z0) else { continue };
            let Some(tr) = self.g.roof([t[0], t[1], tf]).filter(|r| r - tf < ROOF_REACH) else { continue };
            let top = [t[0], t[1], tr + 0.6];
            for (fu, fv) in [(0.9, 0.9), (0.5, 0.0), (1.3, -0.9), (0.2, 1.6), (1.6, 0.4), (0.8, -1.6)] {
                // Its lower end on the pile itself, never down on the bare floor.
                let b = ax.at(um - fu * sl, fv * hand * sl);
                let fl = self.floor_at(b[0], b[1], z0).unwrap_or(z0);
                let Some(r) = self.support(b[0], b[1], z0).filter(|s| *s > fl + 0.5).map(|s| [b[0], b[1], s + 0.1]) else { continue };
                let shallow = (top[2] - r[2]) < 2.1 * (top[0] - r[0]).hypot(top[1] - r[1]);
                if len(sub(top, r)) < 11.0 && shallow && self.beam(top, r).is_some() { break 'beam; }
            }
        }
        // The broken post: its iron foot on the floor in front, its top on the pile (the first of
        // a few ways round the pile's front that has room).
        let Some(m) = main else { return };
        let long = (4.6 + 0.6 * self.jit()) * sl.clamp(0.8, 1.3);
        let out = ax.angle() + PI + 0.6 * hand;
        'post: for turn in [0.0, -0.7, 0.7, -1.3, 1.3, -2.0, 2.0, PI] {
            for l in [long, 0.75 * long, 1.3 * long] {
                if self.against(m, out + turn, l, true).is_some() { break 'post; }
            }
        }
    }

    /// The room's own wreckage beside the pile.
    fn dressing(&mut self, d: Dressing, ax: Axes, z0: f32, front: f32, k: f32, hand: f32) {
        let yaw0 = ax.angle();
        let near = |u: f32, v: f32| ax.at(front - u * k, v * k * hand);
        match d {
            Dressing::Tunnel => {
                self.rock(near(1.2, -3.8), z0, 0.22 * k, yaw0 + 2.4);
                let yaw = yaw0 + 0.9 + 0.5 * self.jit();
                self.floor_board(near(4.2, 1.5), z0, yaw, 2.6 * k.min(1.3));
            }
            Dressing::Timber => {
                let yaw = yaw0 + 0.7 + 0.4 * self.jit();
                self.tipped(LUMBER, near(2.4, -3.6), z0, 1.0, 1.15, 0.0, yaw);
                let yaw = yaw0 - 0.5 + 0.5 * self.jit();
                self.floor_board(near(4.6, 2.2), z0, yaw, 3.0 * k.min(1.3));
                let yaw = yaw0 + 1.4;
                self.floor_beam(near(5.2, -1.4), z0, yaw, 3.6 * k.min(1.3));
            }
            Dressing::Minecart => {
                self.tipped(CART, near(2.6, 3.4), z0, 1.35, 1.5, 0.1, yaw0 + 1.6);
                self.tipped(BARROW, near(4.2, -2.8), z0, 1.0, PI, 0.15, yaw0 + 0.6);
            }
            Dressing::Machine => {
                self.tipped(MACHINE, near(2.8, -3.6), z0, 1.0, 1.32, 0.2, yaw0 + 0.8);
                self.tipped(CART, near(3.6, 3.0), z0, 1.25, 1.5, 0.0, yaw0 + 2.0);
            }
            Dressing::Cannon => {
                for (u, v) in [(3.2, 3.2), (4.6, -2.2), (2.2, -4.2), (5.6, 1.0), (1.6, 4.6), (7.0, 4.0), (6.0, -5.0), (8.5, 0.0), (3.0, 6.5)] {
                    if self.tipped(CANNON, near(u, v), z0, 0.55, 1.1, 0.12, yaw0 + 0.9).is_some() { break; }
                }
                self.tipped(BARREL, near(2.0, -3.4), z0, 1.4, FRAC_PI_2, 0.0, yaw0 + 2.4);
                self.tipped(CRATE, near(4.4, -1.0), z0, 1.3, FRAC_PI_2, 0.0, yaw0 + 1.0);
            }
            Dressing::Cargo => {
                self.tipped(CARGO, near(2.4, 3.2), z0, 0.95, 1.45, 0.0, yaw0 + 0.4);
                self.tipped(CRATE, near(3.8, -2.6), z0, 1.4, 0.0, FRAC_PI_2, yaw0 + 2.1);
                self.tipped(BARREL, near(1.8, -4.0), z0, 1.5, FRAC_PI_2, 0.0, yaw0 + 1.5);
                // (No sacks: the split sack's pale cloth glowed green by the rear cell's lantern on
                // film, twice. A second barrel and crate instead.)
                self.tipped(BARREL, near(4.4, 0.8), z0, 1.3, FRAC_PI_2, 0.2, yaw0 + 0.3);
                self.tipped(CRATE, near(5.0, 2.4), z0, 1.1, FRAC_PI_2, 0.0, yaw0 + 2.6);
            }
            Dressing::Shore => {
                self.tipped(BOAT, near(2.6, 3.6), z0, 0.6, 0.2, 0.12, yaw0 + 1.1);
                self.tipped(CRATE, near(4.8, -2.0), z0, 1.4, FRAC_PI_2, 0.0, yaw0 + 2.8);
                let yaw = yaw0 + 0.4;
                self.floor_board(near(3.8, -4.4), z0, yaw, 3.0);
            }
        }
    }

    /// A cave-in slumped against the nearest wall.
    fn slump(&mut self, site: &Site, z0: f32, hand: f32) {
        let k = site.scale;
        // A collapse is bigger than the rocks set down before it (the entrance's old dome was
        // 2.4): as large as the roof lets it stand, nearly touching it.
        let mut sl = (1.9 * k).clamp(1.0, 2.8);
        if let Some(r) = self.g.roof([site.at[0], site.at[1], z0]) {
            sl = sl.min(((r - z0) - 0.9) / 3.0).max(0.8);
        }
        let found = self.wall_off_the_way([site.at[0], site.at[1], z0], 9.0, sl);
        let (back, dw) = found.unwrap_or((site.yaw, 9.0));
        let ax = Axes::new(site.at, back);
        // Its back half into the wall it came out of, its cut face turned to the rock.
        let um = if found.is_some() { (dw - 1.5 * sl).clamp(-1.0, 4.0) } else { 0.0 };
        let face = back + FACE + 0.15 * self.jit();
        let mut main = None;
        // Where the mine's own doodads stand in the way, it comes down a little to one side.
        for (du, dv, f) in [(0.0, 0.0, 1.0), (-1.0, 0.0, 0.85), (0.8, 0.0, 0.7), (0.0, 2.5, 0.8), (0.0, -2.5, 0.8),
            (-1.5, 2.5, 0.7), (-1.5, -2.5, 0.7), (-2.0, 0.0, 0.6), (0.0, 0.0, 0.55), (-1.0, 1.2, 0.5), (0.0, 0.0, 0.42), (-1.0, -1.2, 0.38)] {
            main = self.rock(ax.at(um + du * k, (0.4 + dv) * hand * k), z0, sl * f, face);
            if main.is_some() { break; }
        }
        // A second large mass beside it, a shoulder and pieces of a middle size, each at the first
        // of a few spots with room.
        let h = hand * sl;
        self.rock_at(ax, &[(um - 0.6 * sl, -2.6 * h), (um - 0.4 * sl, 2.8 * h), (um - 1.4 * sl, -1.6 * h)], z0, 0.8 * sl, face - 0.4);
        self.rock_at(ax, &[(um - 0.9 * sl, 3.0 * h), (um - 1.6 * sl, -3.0 * h), (um - 2.4 * sl, 2.0 * h)], z0, 0.62 * sl, face + 0.5);
        // (The middle size is a size of its own, a man's height or less, however big the mass.)
        self.rock_at(ax, &[(um - 2.3 * sl, -2.3 * h), (um - 2.6 * sl, 1.2 * h), (um - 3.0 * sl, -0.6 * h)], z0, (0.24 * sl).clamp(0.42, 0.7), face - 0.6);
        self.rock_at(ax, &[(um - 1.2 * sl, -0.5 * h), (um - 1.4 * sl, 1.5 * h), (um - 2.0 * sl, -1.6 * h)], z0, (0.2 * sl).clamp(0.36, 0.6), face + 1.9);
        self.rock(ax.at(um - 0.2 * sl, 1.6 * h), z0, (0.16 * sl).clamp(0.32, 0.5), face + 3.0);
        self.broken_roof(ax, z0, um, sl, hand, face, main);
        let front = um - 3.0 * sl;
        self.foot(ax, z0, front, main, k, hand);
        self.dressing(site.dressing, ax, z0, front, k, hand);
    }

    /// The rock at a site from `from` on, by the review's measure: each rock's footprint (its
    /// model's box across, scaled) times how far its turned box stands over the floor under it.
    pub(super) fn rock_volume(&self, site: &str) -> f32 {
        self.props.iter().filter(|p| p.site == site && rocky(p.model)).map(|p| {
            let m = &MODELS[p.model];
            let b = p.obb(0.0);
            let top = b.c[2] + (0..3).map(|i| b.a[i][2].abs() * b.h[i]).sum::<f32>();
            let fl = floor(&*self.g.land, [p.pos[0], p.pos[1], p.pos[2] + 1.0], true).unwrap_or(p.pos[2]);
            (m.max[0] - m.min[0]) * (m.max[1] - m.min[1]) * p.scale * p.scale * (top - fl).max(0.0)
        }).sum()
    }

    /// Big rock heaped on and round a site's pile until it holds `target` (see
    /// [`layout::OLD_ROCK`]): several large pieces jumbled over the first mass and against it,
    /// then smaller ones, each seated on whatever is under its rim and kept under the roof.
    fn heap(&mut self, site: &Site, z0: f32, target: f32) {
        let short = target - self.rock_volume(site.name);
        if short <= 0.0 { return; }
        let rocks: Vec<&Prop> = self.props.iter().filter(|p| p.site == site.name && p.model == ROCK && p.scale > 0.3).collect();
        let c = match rocks.iter().max_by(|a, b| a.scale.total_cmp(&b.scale)) {
            // (Round the largest: the pile's own mass.)
            Some(m) => [m.pos[0], m.pos[1]],
            None => [site.at[0], site.at[1]],
        };
        // The size whose rock (about 128 s^3 by the measure) would make up a third of what's
        // short: three or four of those and their smaller pieces.
        let s0 = (short / 3.0 / 128.0).cbrt().clamp(0.6, 2.4);
        self.under_roof = true;
        let mut added = 0;
        'all: for size in [1.0, 0.8, 0.65, 0.5] {
            for ring in [0.0, 0.8, 1.6, 2.4] {
                for a in 0..6 {
                    if added == 10 || self.rock_volume(site.name) >= target { break 'all; }
                    let ang = a as f32 * 1.05 + ring * 1.3 + size * 3.0;
                    let r = ring * s0 * 3.0;
                    let yaw = ang * 1.7 + self.jit();
                    if self.rock([c[0] + r * ang.cos(), c[1] + r * ang.sin()], z0, s0 * size, yaw).is_some() { added += 1; }
                }
            }
        }
        self.under_roof = false;
    }

    /// A side passage filled to its lintel: a row of large rocks across it, more stacked on them
    /// until the stack meets the roof, and the
    /// lintel's beam down across the front of the fill.
    fn seal(&mut self, site: &Site, z0: f32, hand: f32) {
        let at = [site.at[0], site.at[1], z0];
        let Some((along, wl, wr)) = self.passage(at) else { return self.slump(site, z0, hand) };
        // `f` points into the sealed side: along the passage, away from the region's way through.
        let view = layout::REGIONS[site.region].view;
        let away = [site.at[0] - view[0], site.at[1] - view[1]];
        let back = if along.cos() * away[0] + along.sin() * away[1] >= 0.0 { along } else { along + PI };
        let ax = Axes::new(site.at, back);
        let (wl, wr) = if back == along { (wl, wr) } else { (wr, wl) };
        let w = wl + wr;
        let mid = (wl - wr) * 0.5;
        let lintel = self.g.roof(at).unwrap_or(z0 + 6.0).min(z0 + 12.0);
        let n = (w / 6.5).ceil().max(1.0) as usize;
        let sb = ((w / n as f32 + 3.0) / 7.7).clamp(0.7, 1.9);
        let face = back + FACE;
        let mut first = None;
        for i in 0..n {
            let v = mid + (i as f32 - (n - 1) as f32 * 0.5) * w / n as f32;
            let turn = face + 0.3 * self.jit();
            for (u, f) in [(0.4, 1.0), (1.5, 0.9), (2.6, 0.8), (3.6, 0.7)] {
                if let Some(r) = self.rock(ax.at(u, v), z0, sb * f, turn) {
                    first = first.or(Some(r));
                    break;
                }
            }
        }
        let mut s = 0.8 * sb;
        for tier in 0..3 {
            let top = self.support(ax.at(0.6, mid)[0], ax.at(0.6, mid)[1], z0).unwrap_or(z0);
            if top > lintel - 1.4 { break; }
            let v = mid + 0.8 * hand * (tier as f32 - 0.5);
            self.rock(ax.at(0.8 + 0.3 * tier as f32, v), z0, s, face + 1.3 * tier as f32);
            s *= 0.8;
        }
        // Pieces of the middle size fallen in front of the fill.
        let h = hand * sb;
        self.rock_at(ax, &[(-1.2 * sb, mid + 1.4 * h), (-1.6 * sb, mid - 1.0 * h), (-0.8 * sb, mid)], z0, (0.5 * sb).clamp(0.35, 0.7), face + 2.0);
        self.rock_at(ax, &[(-1.8 * sb, mid - 1.8 * h), (-2.2 * sb, mid + 0.6 * h), (-1.4 * sb, mid + 2.2 * h)], z0, (0.38 * sb).clamp(0.32, 0.6), face - 1.6);
        // The lintel's beam, down across the face of the fill.
        let (p1, p2) = (ax.at(-1.4 * sb, mid - 0.36 * w), ax.at(-1.1 * sb, mid + 0.36 * w));
        let across = match (self.support(p1[0], p1[1], z0), self.support(p2[0], p2[1], z0)) {
            (Some(a), Some(b)) => {
                let r = BEAM_RADIUS * len(sub([p1[0], p1[1], a], [p2[0], p2[1], b])) / BEAM_LEN;
                self.beam([p1[0], p1[1], a + 0.9 * r], [p2[0], p2[1], b + 0.5 * r]).is_some()
            }
            _ => false,
        };
        if let (false, Some(f)) = (across, first) {
            let l = (0.6 * w).clamp(4.0, 8.0);
            for turn in [0.0, 0.7, -0.7, 1.4, -1.4] {
                if self.against(f, back + PI + turn, l, true).is_some() { break; }
            }
        }
        let front = -2.2 * sb;
        self.foot(ax, z0, front, first, site.scale, hand);
        self.dressing(site.dressing, ax, z0, front, site.scale, hand);
    }

    /// The Foundry's back door (view 5): the pile slumped against one jamb nearly to the lintel,
    /// the lintel's beam down from the far jamb onto it, and the way through kept on the far side.
    fn lintel(&mut self, site: &Site, z0: f32, hand: f32) {
        let at = [site.at[0], site.at[1], z0];
        let Some((along, wl, wr)) = self.passage(at) else { return self.slump(site, z0, hand) };
        let ax = Axes::new(site.at, along);
        let (w, mid) = (wl + wr, (wl - wr) * 0.5);
        let lintel = self.g.roof(at).unwrap_or(z0 + 7.0);
        let mut sl = (((lintel - z0) - 1.2) / 3.0).clamp(0.9, 1.7);
        // Against one jamb (the one clear of the way through and the view), in the doorway or a
        // little through it.
        let (mut main, mut at_u, mut hand, mut jamb) = (None, 0.0, hand, mid);
        'fit: for f in [1.0, 0.85, 0.7] {
            for side in [hand, -hand] {
                let j = mid + side * (w * 0.5 - 1.2 * sl * f);
                let face = along + side * FRAC_PI_2 + FACE;
                for u in [0.0, 2.0, -2.0, 3.5] {
                    main = self.rock(ax.at(u, j), z0, sl * f, face);
                    if main.is_some() {
                        (at_u, hand, jamb, sl) = (u, side, j, sl * f);
                        break 'fit;
                    }
                }
            }
        }
        let face = along + hand * FRAC_PI_2 + FACE;
        let jamb_at = |du: f32| du + at_u;
        // Stacked on toward the lintel, so the doorway is near full.
        let mut s2 = 0.78 * sl;
        for tier in 0..3 {
            let c = ax.at(jamb_at(0.0), jamb - hand * 0.4 * sl);
            let top = self.support(c[0], c[1], z0).unwrap_or(z0);
            if top > lintel - 1.2 { break; }
            let spot = ax.at(jamb_at(0.4 * tier as f32 - 0.4), jamb - hand * (0.6 + 0.7 * tier as f32) * sl);
            self.rock(spot, z0, s2, face + 1.1 * tier as f32);
            s2 *= 0.8;
        }
        let j = |du: f32, dv: f32| (jamb_at(du * sl), jamb - hand * dv * sl);
        self.rock_at(ax, &[j(1.6, 1.2), j(2.2, 0.2), j(1.0, 1.8)], z0, (0.6 * sl).min(0.7), face + 0.8);
        self.rock_at(ax, &[j(-1.8, 0.6), j(-1.4, 1.4), j(-2.4, 0.0)], z0, (0.48 * sl).min(0.6), face - 0.7);
        self.rock_at(ax, &[j(-0.6, 0.4), j(-1.0, 1.0), j(0.6, 1.2)], z0, (0.34 * sl).max(0.32), face + 2.2);
        // The lintel: still lodged at the far jamb's top, its other end down on the pile.
        let far = ax.at(jamb_at(-0.3), mid - hand * (w * 0.5 - 0.4));
        let on = ax.at(jamb_at(-0.5 * sl), jamb - hand * 0.6 * sl);
        if let Some(rest) = self.support(on[0], on[1], z0) {
            self.beam([far[0], far[1], lintel - 0.2], [on[0], on[1], rest + 0.35]);
        }
        let front = -2.6 * sl;
        let side = Axes::new([ax.at(at_u, jamb)[0], ax.at(at_u, jamb)[1], z0], along + PI);
        self.foot(side, z0, -1.0 * sl, main, site.scale, -hand);
        self.dressing(site.dressing, ax, z0, front, site.scale, hand);
    }

    /// Loose rock and timber across the floor under a whole roof.
    fn spill(&mut self, site: &Site, z0: f32, hand: f32) {
        let k = site.scale;
        let ax = Axes::new(site.at, site.yaw);
        let face = site.yaw + FACE;
        let main = self.rock(ax.at(0.6 * k, 0.8 * hand * k), z0, 0.5 * k, face);
        self.rock(ax.at(-0.8 * k, -1.6 * hand * k), z0, 0.3 * k, face + 2.0);
        self.foot(ax, z0, -1.0 * k, main, 0.8 * k, hand);
        self.dressing(site.dressing, ax, z0, -1.0 * k, k, hand);
    }
}

/// The axes a piece must touch down at both ends of to rest: a board's or beam's length alone, else
/// either of a box's two flattest.
pub(super) fn rest_axes(b: &Obb) -> Vec<(V3, f32)> {
    let mut axes: Vec<(V3, f32)> = (0..3).map(|i| (b.a[i], b.h[i])).collect();
    axes.sort_by(|x, y| y.1.total_cmp(&x.1));
    if axes[0].1 > 2.5 * axes[1].1 { return vec![axes[0]]; }
    axes.sort_by(|x, y| x.0[2].abs().total_cmp(&y.0[2].abs()));
    axes.truncate(2);
    axes
}

/// The views' close debris: (model, forward and side yards from the view, size, turn). Rocks are
/// seated, boards laid on the floor (the size their length in yards), mounds sunk.
fn apron(b: &mut Builder, r: &Region, rows: &[(usize, [f32; 2], f32, f32)]) {
    let (s, c) = r.facing.sin_cos();
    for &(model, [dx, dy], scale, yaw) in rows {
        let at = [r.view[0] + c * dx - s * dy, r.view[1] + s * dx + c * dy];
        let yaw = r.facing + yaw;
        match model {
            ROCK => { b.rock(at, r.view[2], scale, yaw); }
            SKIRT | SCREE => { b.mound(model, at, r.view[2], scale, yaw); }
            BOARD => { b.floor_board(at, r.view[2], yaw, scale); }
            BEAM => { b.floor_beam(at, r.view[2], yaw, scale); }
            m => { b.tipped(m, at, r.view[2], scale, 1.3, 0.1, yaw); }
        }
    }
}

pub(super) fn compose(g: &Ground) -> (Vec<Prop>, Vec<&'static str>) {
    let mut b = Builder { g, props: Vec::new(), region: 0, site: "", wet: false, seed: 0, k: 0, near: 0.0, under_roof: false };
    let mut missing = Vec::new();
    for (index, site) in layout::SITES.iter().enumerate() {
        (b.region, b.site, b.wet, b.seed, b.k) = (site.region, site.name, false, index, 0);
        let Some(z0) = b.floor_at(site.at[0], site.at[1], site.at[2]) else {
            missing.push(site.name);
            continue;
        };
        b.near = z0;
        let hand = if index % 2 == 0 { 1.0 } else { -1.0 };
        match site.fall {
            Fall::Slump => b.slump(site, z0, hand),
            Fall::Seal => b.seal(site, z0, hand),
            Fall::Lintel => b.lintel(site, z0, hand),
            Fall::Spill => b.spill(site, z0, hand),
        }
    }
    // Then rock heaped on each pile until it has at least the old layout's there, with some to
    // spare (after every pile has its own parts, so one's heap doesn't crowd out the next's).
    // Where a pile can't take it (the walk runs by it, the shore drops into the water), the rest
    // of its region's piles take more, until the region has the old layout's and more.
    let heap_at = |b: &mut Builder, index: usize, target: f32| {
        let site = &layout::SITES[index];
        // (The cove's shore piles may run into the shallows.)
        (b.region, b.site, b.wet, b.seed, b.k) = (site.region, site.name, site.region == 6, 500 + index, 0);
        let Some(z0) = b.floor_at(site.at[0], site.at[1], site.at[2]) else { return };
        b.near = z0;
        b.heap(site, z0, target);
        b.wet = false;
    };
    for (index, site) in layout::SITES.iter().enumerate() {
        heap_at(&mut b, index, 1.25 * layout::old_rock(site.name));
    }
    for region in 0..layout::REGIONS.len() {
        let sites: Vec<usize> = (0..layout::SITES.len()).filter(|i| layout::SITES[*i].region == region).collect();
        let old: f32 = sites.iter().map(|i| layout::old_rock(layout::SITES[*i].name)).sum();
        for round in 0..3 {
            let have: f32 = sites.iter().map(|i| b.rock_volume(layout::SITES[*i].name)).sum();
            if have >= 1.2 * old { break; }
            let each = (1.2 * old - have) / sites.len() as f32;
            for &i in &sites {
                let now = b.rock_volume(layout::SITES[i].name);
                heap_at(&mut b, i, now + each * (1.0 + 0.5 * round as f32));
            }
        }
    }
    for (region, r) in layout::REGIONS.iter().enumerate() {
        (b.region, b.site, b.wet, b.seed, b.k, b.near) = (region, "Rubble apron", false, 1000 + region, 0, r.view[2]);
        apron(&mut b, r, layout::APRONS[region]);
    }
    cove(&mut b);
    (b.props, missing)
}

/// The ship's old footprint is water over a seabed at -16 yd: broken rock masses seated on that
/// bottom, and a sparse trail of floating wreckage where the decks and stores were.
fn cove(b: &mut Builder) {
    (b.region, b.site, b.wet, b.seed, b.k, b.near) = (6, "Juggernaut collapse basin", true, 2000, 0, -16.0);
    for (at, scale, yaw) in [([-80.0, -820.0], 8.8, 0.5), ([-60.0, -850.0], 7.5, 2.3),
        ([-100.0, -850.0], 7.1, 4.1), ([-60.0, -800.0], 6.3, 1.2), ([-105.0, -821.0], 8.1, 3.8)] {
        if let Some(z) = b.floor_at(at[0], at[1], -16.0) {
            for (off, s, turn) in [([0.0, 0.0], scale, yaw), ([5.0, 3.0], scale * 0.55, yaw + 1.5)] {
                let p = b.prop(ROCK, [at[0] + off[0], at[1] + off[1], z - 0.66 * s], s, shapes::euler(0.0, 0.0, turn));
                b.place(p);
            }
        }
    }
    // Native DeadminesInstance_33_32 MCLQ: all 42 water chunks have surface -0.023 yd. These are
    // floating fragments, deliberately part sunk, not a replacement walkable ship or a hidden hull.
    const WATER: f32 = -0.023;
    for (name, x, y, yaw) in [
        ("East scaffold drift", -24.0, -832.0, 1.4),
        ("East stores drift", -34.0, -846.0, 3.2),
        ("Broken bow timbers", -51.0, -806.0, 0.8),
        ("Stern wreck trail", -88.0, -861.0, 4.1),
        ("Quartermaster's scattered cargo", -103.0, -828.0, 2.4),
    ] {
        if !b.g.land.water_near(x, y, -16.0) { continue; }
        b.site = name;
        let q = shapes::euler(0.25, 0.12, yaw);
        let p = b.prop(BOAT, [x, y, WATER - 1.25 * 1.1 - 0.15], 1.1, q);
        b.place(p);
        let q = shapes::euler(0.05, 0.0, yaw + 1.2);
        let p = b.prop(LUMBER, [x + 4.0, y - 2.0, WATER - 0.3], 1.3, q);
        b.place(p);
        let d = [(yaw - 0.4).cos(), (yaw - 0.4).sin()];
        b.board([x - 4.0, y + 2.0, WATER - 0.1], [x - 4.0 + 3.6 * d[0], y + 2.0 + 3.6 * d[1], WATER - 0.05], 0.08);
        let q = shapes::euler(0.4, 0.15, yaw + 2.0);
        let p = b.prop(CRATE, [x + 2.0, y + 3.0, WATER - 0.7], 1.1, q);
        b.place(p);
    }
}
