//! A creature's blow whose look is silent sounds as its wind-up begins (the lab's sound probe,
//! Nov 7: the Crossbowman's shot and the Evoker's Void Burst played nothing at all).

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::Foe;
use crate::sim::*;

fn heard_as_it_winds_up(mob: &str, string: &str) -> (u32, bool) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("ayla", "Ayla", None);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    let def = s.content.mob(mob).unwrap();
    let k = s.content.mobs[def].patterns.iter().position(|x| x.name == string).unwrap();
    let m = s.spawn_mob(def, [12.0, 0.0, 0.0], None, false);
    let _ = s.step(0.0);
    s.begin_pattern(m, k, false);
    s.windup(m, Foe::Hero(p), s.players[&p].pos);
    let sound = s.attack_now(m).sound;
    let ev = s.step(0.02);
    (sound, ev.iter().any(|e| matches!(e, Event::Sound { id, sound: x } if *id == m && *x == sound)))
}

#[test]
fn the_crossbowmans_shot_is_heard_as_it_draws() {
    let (sound, heard) = heard_as_it_winds_up("dm_crossbowman", "bolt");
    assert!(sound != 0 && heard, "sound {sound}");
}

#[test]
fn the_evokers_void_burst_is_heard_as_it_gathers() {
    let (sound, heard) = heard_as_it_winds_up("dm_evoker", "void burst");
    assert!(sound != 0 && heard, "sound {sound}");
}

#[test]
fn the_brainwashed_nobles_voice_is_heard_as_it_gathers() {
    let (sound, heard) = heard_as_it_winds_up("dm_noble", "the voice");
    assert!(sound != 0 && heard, "sound {sound}");
}

#[test]
fn the_crossbowmans_kick_is_no_bow_drawn() {
    let (sound, _) = heard_as_it_winds_up("dm_crossbowman", "kick");
    assert_eq!(sound, 0);
}
