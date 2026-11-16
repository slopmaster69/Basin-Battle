//! A swing that strikes several creatures goes down its list while the first blows may already have
//! changed the world. Found by the soak (Nov 16, Shadowfang Keep): the nearer was a summoner, its
//! fall dismissed what it had called up, and the swing went on to the add it had just lost and
//! panicked on its missing entry (`self.mobs[&mid]`).

use super::*;
use crate::land::FlatLand;

const DT: f32 = 0.05;

#[test]
fn a_cleave_that_fells_a_summoner_does_not_then_strike_its_dismissed_add() {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::bare(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    let h = s.join("cleave", "Cleave", None);
    s.move_player(h, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    let axe = s.content.item("axe").expect("an axe");
    s.players.get_mut(&h).unwrap().weapon = axe;
    // The summoner in front, one hit from death; its add a little farther along the same swing.
    let def = s.content.mob("grunt").unwrap();
    let boss = s.spawn_mob(def, [1.6, 0.0, 0.0], None, false);
    let add = s.spawn_mob(def, [2.4, 0.3, 0.0], None, false);
    s.mobs.get_mut(&add).unwrap().summoned_by = Some(boss);
    s.mobs.get_mut(&boss).unwrap().health = 1;
    s.set_aim_line(h, 0.0, 0.0);
    for _ in 0..40 {
        s.input(h, Input::Attack);
        s.input(h, Input::AttackUp);
        s.step(DT);
    }
    assert!(s.mobs.get(&boss).is_none_or(|m| m.health == 0), "the summoner fell to the swing");
    assert!(!s.mobs.contains_key(&add), "and its add went with it, the swing none the worse");
}
