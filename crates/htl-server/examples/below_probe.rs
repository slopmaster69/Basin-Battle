//! The Deadmines on the real ground: `below_probe <Data>` loads the instance, lays out the world
//! below from the rules, and reports every lamp, door, box and foe (the ground under each, and
//! whether it's walled in), then walks from the first lamp to each of the others and to every foe,
//! and runs a hero through a fight with whatever is nearest.
use std::time::Instant;

use htl_server::survival::land::DeadminesLand;
use htl_survival::{dist2d, Content, Input, ObjKind, Survival};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data = std::path::Path::new(&args[1]);
    let t0 = Instant::now();
    let loaded = htl_map::deadmines::load(data)?;
    println!("loaded the Deadmines in {:.1}s: {} floors", t0.elapsed().as_secs_f32(), loaded.ground.floor_count());
    let land = DeadminesLand::new(loaded);
    // A second copy for walking the layout end to end (the rules' own plans stop short).
    let far = DeadminesLand::new(htl_map::deadmines::load(data)?);
    let c = Content::builtin();
    let mut s = Survival::new_dungeon(c.clone(), Box::new(land), 7, 0.0);
    let d = c.dungeon.clone().unwrap();
    let report = |s: &Survival, what: &str, at: [f32; 3]| {
        let z = s.land().height_near(at[0], at[1], at[2]);
        let blocked = s.land().blocked_near(at[0], at[1], at[2]);
        println!("  {what:<34} at {:>7.1} {:>7.1} {:>6.1}  ground {z:>6.1}{}", at[0], at[1], at[2], if blocked { "  WALLED" } else { "" });
    };
    for l in &d.lamps {
        report(&s, &format!("lamp {}", l.key), l.at);
    }
    for dr in &d.doors {
        report(&s, &format!("door {} ({})", dr.key, dr.kind), dr.at);
    }
    for b in &d.boxes {
        report(&s, &format!("box {}", b.key), b.at);
    }
    let mobs: Vec<(u32, String, [f32; 3])> = s.mobs.iter().map(|(id, m)| (*id, c.mobs[m.def].key.clone(), m.pos)).collect();
    for (id, k, p) in &mobs {
        report(&s, &format!("foe {id} {k}"), *p);
    }
    let start = d.lamps[0].at;
    let mut targets: Vec<(String, [f32; 3])> = d.lamps.iter().skip(1).map(|l| (format!("lamp {}", l.key), l.at)).collect();
    targets.extend(mobs.iter().map(|(id, k, p)| (format!("foe {id} {k}"), *p)));
    targets.push(("the front door".into(), d.exit));
    // Each from the lamp before it (the way it's walked), or the nearest other lamp.
    let lamps: Vec<[f32; 3]> = d.lamps.iter().map(|l| l.at).collect();
    for (n, (what, to)) in targets.iter().enumerate() {
        let from = if n + 1 < lamps.len() {
            lamps[n]
        } else {
            *lamps
                .iter()
                .filter(|l| dist2d(**l, *to) > 1.0)
                .min_by(|a, b| dist2d(**a, *to).total_cmp(&dist2d(**b, *to)))
                .unwrap_or(&start)
        };
        let t = Instant::now();
        match far.walk(from, *to) {
            Some(p) => println!("  walk to {what:<30} {:>4} turns from {:>6.0} {:>6.0}, ends {:>6.1} ({:.0} ms)", p.len(), from[0], from[1], p.last().map_or(0.0, |q| q[2]), t.elapsed().as_secs_f32() * 1000.0),
            None => println!("  walk to {what:<30} NO WAY from {:>6.0} {:>6.0} ({:.0} ms)", from[0], from[1], t.elapsed().as_secs_f32() * 1000.0),
        }
    }
    // A hero at the first lamp, then by each foe in turn: does it come, and does a tick keep up?
    let id = s.join("probe", "Probe", None);
    println!("hero at {:?}", s.players[&id].pos);
    let lamp = s.objs.iter().find(|(_, o)| matches!(&o.kind, ObjKind::Lamp { key } if *key == d.lamps[0].key)).map(|(id, _)| *id);
    s.interact(id, lamp);
    let _ = s.step(0.05);
    println!("lit: {:?}", s.lit_lamps());
    for (mid, k, p) in mobs.iter() {
        let near = [p[0] + 6.0, p[1], p[2]];
        let z = s.land().height_near(near[0], near[1], near[2]);
        s.move_player(id, [near[0], near[1], z], 3.14, Default::default());
        let t = Instant::now();
        let mut worst = 0.0f32;
        let mut came = f32::MAX;
        for _ in 0..100 {
            let t1 = Instant::now();
            let _ = s.step(0.05);
            worst = worst.max(t1.elapsed().as_secs_f32());
            if let Some(m) = s.mobs.get(mid) {
                came = came.min(dist2d(m.pos, s.players[&id].pos));
            }
            s.input(id, Input::Guard);
        }
        let m = s.mobs.get(mid);
        let (pc, pn, pw) = DeadminesLand::plan_stats();
        println!(
            "  by {k:<14} it came within {came:.1} yd; it stands at z {:.1} (hero {:.1}); worst tick {:.1} ms, 5 s in {:.0} ms; {pc} plans {:.0} ms (worst {:.1})",
            m.map_or(0.0, |m| m.pos[2]),
            s.players[&id].pos[2],
            worst * 1000.0,
            t.elapsed().as_secs_f32() * 1000.0,
            pn as f32 / 1e6,
            pw as f32 / 1e6
        );
        s.input(id, Input::GuardUp);
        if s.players[&id].health == 0 {
            let t = Instant::now();
            let _ = s.step(30.0);
            println!("    the hero fell: back in {:.0} ms", t.elapsed().as_secs_f32() * 1000.0);
        }
    }
    Ok(())
}
