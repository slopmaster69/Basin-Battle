//! The host log's melee line: a hero's swings, light chain, hits, guards and dodges, written once
//! when the fight ends (so "did my fourth blow ever come out?" has an answer).

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::melee::MeleeTally;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;

fn world() -> (Survival, Id, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    c.player.field_regen = 0.0;
    c.player.rest_regen = 0.0;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("moneyman", "Moneyman", None);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    let def = s.content.mob("grunt").unwrap();
    let m = s.spawn_mob(def, [1.5, 0.0, 0.0], None, false);
    let x = s.mobs.get_mut(&m).unwrap();
    x.state = MobState::Idle { wander_at: 1e9 };
    x.health = 1_000_000;
    x.max_health = 1_000_000;
    (s, p, m)
}

/// Run `secs`, the foe held where it stands; the host log lines that came out.
fn run(s: &mut Survival, m: Id, secs: f32) -> Vec<String> {
    let at = s.mobs[&m].pos;
    let mut logs = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        if let Some(x) = s.mobs.get_mut(&m) {
            x.state = MobState::Idle { wander_at: 1e9 };
            x.path.clear();
            x.pos = at;
            x.facing = std::f32::consts::PI;
        }
        for e in s.step(DT) {
            if let Event::Log(l) = e {
                logs.push(l);
            }
        }
    }
    logs
}

/// Click until `blows` light swings have started.
fn click_until(s: &mut Survival, p: Id, m: Id, blows: u32) -> Vec<String> {
    let mut logs = Vec::new();
    for _ in 0..400 {
        if s.players[&p].melee.light >= blows {
            break;
        }
        s.input(p, Input::Attack);
        s.input(p, Input::AttackUp);
        logs.extend(run(s, m, DT));
    }
    logs
}

fn fight_line(logs: &[String]) -> String {
    logs.iter().find(|l| l.starts_with("Moneyman's fight:")).cloned().unwrap_or_else(|| panic!("no fight line in {logs:?}"))
}

#[test]
fn a_full_light_chain_counts_each_blow_and_prints_one_line_at_the_end() {
    let (mut s, p, m) = world();
    click_until(&mut s, p, m, 1);
    let len = s.players[&p].melee.chain_len as u32;
    assert!(len >= 2, "the weapon has a chain");
    click_until(&mut s, p, m, len);
    let t = s.players[&p].melee.clone();
    assert_eq!(t.light, len);
    assert_eq!(t.heavy, 0);
    assert_eq!(t.reached[..len as usize], vec![1; len as usize][..], "each blow once, in order");
    assert_eq!(t.dropped(), 0, "the chain wasn't dropped");
    // Quiet: nothing printed mid-fight, one line after the lull.
    let logs = run(&mut s, m, 2.0);
    assert!(logs.iter().all(|l| !l.contains("'s fight")), "{logs:?}");
    let logs = run(&mut s, m, 12.0);
    let line = fight_line(&logs);
    assert!(line.starts_with(&format!("Moneyman's fight: {len} swings (light {len}, heavy 0)")), "{line}");
    assert!(line.contains(&format!("blow {len} (finisher): 1x")), "{line}");
    assert!(line.contains("chains dropped 0"), "{line}");
    assert!(line.contains("parries 0 (perfect 0), guards 0, dodges 0 (perfect 0); dealt "), "{line}");
    // Cleared for the next fight.
    assert_eq!(s.players[&p].melee, MeleeTally::default());
    assert!(run(&mut s, m, 12.0).iter().all(|l| !l.contains("'s fight")), "no second line");
}

#[test]
fn a_chain_left_to_run_out_is_dropped_as_timed_out() {
    let (mut s, p, m) = world();
    click_until(&mut s, p, m, 1);
    // Past the swing and the grace, then start over.
    run(&mut s, m, 3.0);
    assert_eq!(s.players[&p].melee.open, 1, "one blow of a chain open");
    click_until(&mut s, p, m, 2);
    let t = &s.players[&p].melee;
    assert_eq!((t.reached[0], t.reached[1], t.timed_out), (2, 0, 1), "{t:?}");
}

#[test]
fn a_heavy_attack_in_the_chain_drops_it_as_switched() {
    let (mut s, p, m) = world();
    click_until(&mut s, p, m, 1);
    // The heavy goes once the blow's follow-through allows it.
    for _ in 0..40 {
        s.input(p, Input::Heavy);
        run(&mut s, m, DT);
        if s.players[&p].melee.heavy > 0 {
            break;
        }
    }
    let t = &s.players[&p].melee;
    assert_eq!((t.heavy, t.switched, t.open), (1, 1, 0), "{t:?}");
}

#[test]
fn a_dodge_is_counted_and_the_hero_falling_prints_the_line_at_once() {
    let (mut s, p, m) = world();
    click_until(&mut s, p, m, 1);
    run(&mut s, m, 1.0);
    s.input(p, Input::Dodge);
    run(&mut s, m, 0.1);
    assert_eq!(s.players[&p].melee.dodges, 1);
    // Struck down: the line comes with the fall, the chain left open counted as dropped.
    s.hurt_player(p, 1_000_000);
    let logs: Vec<String> = s.step(DT).into_iter().filter_map(|e| if let Event::Log(l) = e { Some(l) } else { None }).collect();
    let line = fight_line(&logs);
    assert!(line.contains("dodges 1 (perfect 0)"), "{line}");
    assert!(line.contains("taken "), "{line}");
    assert!(line.contains("lowest health 0/"), "{line}");
    assert!(line.contains("chains dropped 1"), "{line}");
}

#[test]
fn the_line_reads_as_the_example() {
    let t = MeleeTally {
        live: true,
        light: 24,
        heavy: 7,
        reached: [17, 9, 5, 3, 0, 0, 0, 0],
        chain_len: 4,
        timed_out: 6,
        hurt_ends: 1,
        switched: 4,
        hits: 18,
        whiffs: 6,
        parries: 2,
        guards: 5,
        dodges: 4,
        perfect_dodges: 2,
        perfect_parries: 1,
        dealt: 900,
        taken: 120,
        lowest: Some((40, 100)),
        ..MeleeTally::default()
    };
    assert_eq!(
        t.line("Moneyman").unwrap(),
        "Moneyman's fight: 31 swings (light 24, heavy 7); light chain started 17x, blow 2: 9x, blow 3: 5x, blow 4 (finisher): 3x; chains dropped 11 (timed out 6, hurt 1, switched 4); 18 hits, 6 whiffed; parries 2 (perfect 1), guards 5, dodges 4 (perfect 2); dealt 900, taken 120, lowest health 40/100"
    );
    assert_eq!(MeleeTally::default().line("Moneyman"), None, "an idle hero says nothing");
}
