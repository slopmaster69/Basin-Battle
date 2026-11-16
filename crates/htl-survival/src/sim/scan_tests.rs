//! The creature scans (`waits_turn`, `caster_waits`, `attackers_on`) against the straightforward
//! versions they replaced, on a crowded scene: the same answer for every creature and hero, tick
//! after tick. (The old ones built each creature's whole blow, `attack_now`, to read one flag.)

use super::*;
use crate::land::FlatLand;

fn caster_waits_old(s: &Survival, id: Id, foe: Foe) -> bool {
    let Foe::Hero(h) = foe else { return false };
    if s.content.ai.turn_beat <= 0.0 {
        return false;
    }
    s.mobs.iter().any(|(oid, o)| *oid != id && o.health > 0 && matches!(o.state, MobState::Windup { foe: Foe::Hero(x), .. } if x == h) && !s.attack_now(*oid).ranged)
}

fn attackers_on_old(s: &Survival, hero: Id, except: Id) -> usize {
    s.mobs
        .iter()
        .filter(|(id, m)| {
            **id != except
                && m.health > 0
                && !s.attack_now(**id).ranged
                && match m.state {
                    MobState::Windup { foe: Foe::Hero(h), .. } => h == hero,
                    MobState::Recover { foe: Foe::Hero(h), .. } => h == hero && m.pattern.is_some(),
                    _ => false,
                }
        })
        .count()
}

/// `waits_turn` as it was, down to the `attack_now` calls.
fn waits_turn_old(s: &Survival, id: Id, foe: Foe, dist: f32, settle: bool) -> bool {
    let Foe::Hero(h) = foe else { return false };
    let (now, ai) = (s.clock, &s.content.ai);
    let Some(me) = s.mobs.get(&id) else { return false };
    if ai.turn_beat > 0.0 && s.players.get(&h).is_some_and(|p| matches!(p.action, Action::Stagger { .. })) {
        return true;
    }
    if settle && s.settling(id, dist) {
        return true;
    }
    if ai.turn_beat <= 0.0 {
        return false;
    }
    let open = s.players.get(&h).is_some_and(|p| match &p.action {
        Action::Eat { .. } | Action::Charge { .. } | Action::Cast { .. } | Action::Channel { .. } => true,
        Action::Swing { t, strike, .. } => *t > strike.windup + strike.active,
        _ => false,
    });
    let hp = s.players.get(&h).map(|p| p.pos);
    let bolt = hp.is_some_and(|hp| s.missiles.iter().any(|ms| ms.owner != id && ms.land_at > now && dist2d(ms.at, hp) <= ms.radius + 4.0));
    let winding = s.mobs.iter().any(|(oid, o)| *oid != id && o.health > 0 && matches!(o.state, MobState::Windup { foe: Foe::Hero(x), .. } if x == h));
    let temper = s.temper(id);
    if me.role == Role::Flanker && now - me.role_at < FLANK_MOST && !open && !s.behind(id, h) && s.others_on(h, id) {
        return true;
    }
    if temper.is_some_and(|t| t.backstab) {
        return !(open || s.behind(id, h)) || bolt || winding;
    }
    if open && temper.and_then(|t| t.punish).unwrap_or(true) {
        return bolt || winding;
    }
    if temper.and_then(|t| t.waits_turn) == Some(false) {
        return bolt || winding;
    }
    if bolt {
        return true;
    }
    s.mobs.iter().any(|(oid, o)| {
        let ranged = || s.attack_now(*oid).ranged;
        *oid != id
            && o.health > 0
            && match o.state {
                MobState::Windup { foe: Foe::Hero(x), .. } => x == h,
                MobState::Recover { foe: Foe::Hero(x), .. } => x == h && !ranged(),
                MobState::Chase { foe: Foe::Hero(x) } => {
                    x == h
                        && !ranged()
                        && (now - o.string_done_at < ai.turn_beat
                            || (o.string_done_at < me.string_done_at && o.ready_at <= now && hp.is_some_and(|hp| dist2d(o.pos, hp) <= PRESS_RANGE)))
                }
                _ => false,
            }
    })
}

#[test]
fn scans_match_the_old_ones_on_a_crowded_scene() {
    scans_match(150, 100);
}

#[test]
fn scans_match_the_old_ones_on_a_thin_scene() {
    scans_match(14, 0);
}

fn scans_match(n: usize, least: usize) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let band: Vec<Id> = (0..5)
        .map(|k| {
            let h = s.join(&format!("h{k}"), &format!("H{k}"), None);
            s.move_player(h, [3.0 + k as f32, k as f32 * 2.0, 0.0], std::f32::consts::PI, Moving::default());
            let p = s.players.get_mut(&h).unwrap();
            p.health = 1_000_000;
            p.max_health = 1_000_000;
            h
        })
        .collect();
    let _ = s.step(0.0);
    let keys = ["blackrock_grunt", "blackrock_caster", "prowler", "kobold_tunneler", "grunt", "archer", "shaman", "wolf"];
    for i in 0..n {
        let Some(def) = s.content.mob(keys[i % keys.len()]) else { continue };
        let a = i as f32 * 0.7;
        let r = 8.0 + (i % 5) as f32 * 2.0;
        let id = s.spawn_mob(def, [a.cos() * r, a.sin() * r, 0.0], None, true);
        s.raid.raiders.push(id);
        s.engage(id, Foe::Hero(band[i % band.len()]));
    }
    let (mut checked, mut waiting, mut casters, mut attackers) = (0usize, 0usize, 0usize, 0usize);
    for tick in 0..400 {
        for h in &band {
            if let Some(p) = s.players.get_mut(h) {
                p.health = p.max_health;
                p.stamina = 100.0;
            }
            if tick % 8 == 0 {
                s.input(*h, Input::Attack);
            }
        }
        let _ = s.step(0.05);
        if tick % 3 != 0 {
            continue;
        }
        let ids: Vec<Id> = s.mobs.keys().copied().collect();
        for &h in &band {
            for &id in &ids {
                let foe = Foe::Hero(h);
                assert_eq!(s.attack_ranged(id), s.attack_now(id).ranged, "ranged, tick {tick}");
                let (n, o) = (s.caster_waits(id, foe), caster_waits_old(&s, id, foe));
                assert_eq!(n, o, "caster_waits, tick {tick}");
                casters += n as usize;
                let (n, o) = (s.attackers_on(h, id), attackers_on_old(&s, h, id));
                assert_eq!(n, o, "attackers_on, tick {tick}");
                attackers += n;
                for (dist, settle) in [(1.0, true), (5.0, false), (12.0, true)] {
                    let (n, o) = (s.waits_turn(id, foe, dist, settle), waits_turn_old(&s, id, foe, dist, settle));
                    assert_eq!(n, o, "waits_turn, tick {tick}");
                    waiting += n as usize;
                    checked += 1;
                }
            }
        }
    }
    eprintln!("scan check: {checked} waits_turn, {waiting} waiting, {casters} caster holds, {attackers} attackers");
    assert!(waiting >= least && attackers >= least && checked > waiting, "the scene should exercise the scans (both answers)");
}
