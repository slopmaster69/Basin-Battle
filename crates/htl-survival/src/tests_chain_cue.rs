//! The light chain on the hero's own screen (Corey: "I wasn't able to tell if I was doing my
//! attack combo correctly?"): each blow carries its place in the chain (`Strike::chain`, sent in
//! its `AN` line for the add-on's pips), and the set's last blow, the finisher, flashes on the
//! swinger as it comes ([`KIT_FINISHER`]) whether it strikes anything or not. Cleave's flash on a
//! foe it strikes stays as it was. The chain lengths per weapon family stay as they are.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.02;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const PI: f32 = std::f32::consts::PI;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

/// A hero of this class, made as the creation screen makes one, at the origin facing +x.
fn hero_of(s: &mut Survival, name: &str, class: &str) -> Id {
    let account = name.to_lowercase();
    let save = s.fresh_save(&account, name, class, HeroLook::default());
    let id = s.join(&account, name, Some(&save));
    s.move_player(id, ORIGIN, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

/// A creature in front of the hero, facing them, that never strikes on its own.
fn foe(s: &mut Survival) -> Id {
    let def = s.content.mob("grunt").unwrap();
    let id = s.spawn_mob(def, [1.5, 0.0, 0.0], None, false);
    hold(s, id);
    let m = s.mobs.get_mut(&id).unwrap();
    m.health = 1_000_000;
    m.max_health = 1_000_000;
    id
}

fn hold(s: &mut Survival, m: Id) {
    if let Some(m) = s.mobs.get_mut(&m) {
        m.pos = [1.5, 0.0, 0.0];
        m.facing = PI;
        m.state = MobState::Idle { wander_at: 1e9 };
        m.path.clear();
        m.ready_at = 1e9;
    }
}

fn strike(s: &Survival, p: Id) -> Strike {
    match &s.players[&p].action {
        Action::Swing { strike, .. } => strike.clone(),
        a => panic!("no blow: {a:?}"),
    }
}

/// One blow (the off hand's on `off`), pressed late in the last one's recovery as a chain is,
/// then run on to late in its own; `each` before every tick. The blow as it started, and the
/// events.
fn blow(s: &mut Survival, p: Id, off: bool, each: &mut dyn FnMut(&mut Survival)) -> (Strike, Vec<Event>) {
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    s.input(p, if off { Input::Guard } else { Input::Attack });
    let st = strike(s, p);
    let mut ev = Vec::new();
    for _ in 0..200 {
        each(s);
        ev.extend(s.step(DT));
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + strike.recovery * 0.75) {
            break;
        }
    }
    (st, ev)
}

fn flashed(ev: &[Event], on: Id, kit: u32) -> bool {
    ev.iter().any(|e| matches!(e, Event::Visual { id, kit: k, .. } if *id == on && *k == kit))
}

/// A weapon of this family (its `[[moveset]]`), as an item a hero can hold.
fn weapon_of_family(s: &Survival, family: &str) -> usize {
    let w = s.content.weapons.iter().find(|w| w.moveset_key() == family && !w.casts() && s.content.item(&w.key).is_some());
    let w = w.unwrap_or_else(|| panic!("a {family} weapon"));
    s.content.item(&w.key).unwrap()
}

/// Each family's chain, as it is (Corey: the lengths stay).
const FAMILIES: [(&str, u8); 9] = [
    ("dagger", 4),
    ("fist", 4),
    ("two_hand", 2),
    ("maul", 2),
    ("one_hand", 3),
    ("axe_one_hand", 3),
    ("mace_one_hand", 3),
    ("axe_two_hand", 3),
    ("polearm", 3),
];

#[test]
fn a_whiffed_finisher_flashes_on_the_swinger_and_no_earlier_blow_does() {
    for (family, len) in FAMILIES {
        let mut s = world();
        let p = hero_of(&mut s, "Bren", "warrior");
        let w = weapon_of_family(&s, family);
        s.players.get_mut(&p).unwrap().weapon = w;
        // Nothing in reach: every blow whiffs.
        for k in 1..=len {
            let (st, ev) = blow(&mut s, p, false, &mut |_| {});
            assert_eq!(st.chain, (k, len), "{family}: blow {k}");
            assert!(!st.run);
            assert_eq!(st.finisher(), k == len, "{family}: blow {k}");
            assert_eq!(flashed(&ev, p, KIT_FINISHER), k == len, "{family}: the flash on the swinger, blow {k} of {len}");
            assert!(!ev.iter().any(|e| matches!(e, Event::Visual { kit: KIT_CLEAVE, .. })), "{family}: nothing struck, no Cleave");
        }
        // The next press starts a new chain.
        let (st, ev) = blow(&mut s, p, false, &mut |_| {});
        assert_eq!(st.chain, (1, len), "{family}: a new chain");
        assert!(!flashed(&ev, p, KIT_FINISHER));
    }
}

#[test]
fn a_finisher_that_lands_keeps_cleave_on_the_foe_and_flashes_on_the_swinger() {
    let mut s = world();
    let p = hero_of(&mut s, "Bren", "warrior");
    let w = weapon_of_family(&s, "one_hand");
    s.players.get_mut(&p).unwrap().weapon = w;
    let m = foe(&mut s);
    for k in 1..=3 {
        let (st, ev) = blow(&mut s, p, false, &mut |s| hold(s, m));
        assert!(ev.iter().any(|e| matches!(e, Event::Swing { victim: Some(v), .. } if *v == m)), "blow {k} reaches the foe");
        let last = k == 3;
        assert_eq!(st.chain, (k, 3));
        assert_eq!(flashed(&ev, p, KIT_FINISHER), last, "blow {k}: the flash on the swinger");
        assert_eq!(flashed(&ev, m, KIT_CLEAVE), last, "blow {k}: Cleave on the foe");
        assert!(!flashed(&ev, m, KIT_FINISHER), "the swinger's flash is on the swinger");
    }
}

#[test]
fn a_heavy_attack_is_no_chains_blow() {
    let mut s = world();
    let p = hero_of(&mut s, "Bren", "warrior");
    for _ in 0..100 {
        if matches!(s.players[&p].action, Action::Swing { .. }) {
            break;
        }
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        s.input(p, Input::Heavy);
        let _ = s.step(DT);
    }
    let st = strike(&s, p);
    assert_eq!((st.chain, st.finisher()), ((0, 0), false), "{st:?}");
}

#[test]
fn a_two_blade_runs_last_blow_is_its_finisher_on_the_swinger() {
    let mut s = world();
    let p = hero_of(&mut s, "Ayla", "rogue");
    let d = s.content.item("dagger").unwrap();
    let h = s.players.get_mut(&p).unwrap();
    h.weapon = d;
    h.off_weapon = Some(d);
    h.off_empty = false;
    let len = s.run_length(&s.players[&p]);
    assert_eq!(len, 4);
    for k in 1..=len {
        let (st, ev) = blow(&mut s, p, k % 2 == 0, &mut |_| {});
        assert_eq!(st.chain, (k, len), "blow {k}: {st:?}");
        // The first blow is the light chain's (either could follow); from the second, the run's.
        assert_eq!(st.run, k >= 2, "blow {k}");
        assert_eq!(st.finish, k == len);
        assert_eq!(flashed(&ev, p, KIT_FINISHER), k == len, "blow {k}");
    }
}

#[test]
fn a_chain_stays_open_for_the_grace_and_a_run_longer() {
    let s = world();
    let grace = s.content.combat.combo_grace;
    assert_eq!(s.chain_window(false), grace);
    assert_eq!(s.chain_window(true), grace + DUAL_WINDOW);
}
