//! Sneed's Shredder on the real Mast Room floor: the broken machine's run back to its post, the
//! rigged log over the wreck, its tie-off, and the race for it while Sneed mends the machine.
//! No game assets stored here.
use htl_server::survival::land::DeadminesLand;
use htl_survival::{dist2d, facing, Content, Event, Id, Input, ShredderRig, Survival};

const DT: f32 = 0.05;
type Vec3 = [f32; 3];

fn along(p: Vec3, yaw: f32, d: f32) -> Vec3 {
    [p[0] + yaw.cos() * d, p[1] + yaw.sin() * d, p[2]]
}

fn live(s: &Survival, key: &str) -> Option<Id> {
    s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == key).map(|(id, _)| *id)
}

/// The Mast Room with only Sneed's pack in it and the door a hero comes in by (the machine works
/// with its back to it), the hero just inside that door.
fn mine(health: Option<u32>) -> (Survival, Id, Id) {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    c.dungeon.as_mut().unwrap().packs.retain(|p| p.key == "sneed");
    c.dungeon.as_mut().unwrap().doors.retain(|d| d.key == "mast_fog");
    let mut s = Survival::new_dungeon(c, Box::new(land), 17, 0.0);
    let hero = s.join("rigproof", "Rigproof", None);
    let p = s.players.get_mut(&hero).unwrap();
    if let Some(h) = health {
        p.loadout.clear();
        p.max_health = h;
        p.health = h;
    }
    let door = s.land().footing([-290.3, -531.0, 49.4]).expect("inside the room's door");
    s.players.get_mut(&hero).unwrap().last_safe = door;
    s.move_player(hero, door, std::f32::consts::FRAC_PI_2, Default::default());
    for _ in 0..20 { step(&mut s); }
    let machine = live(&s, "dm_shredder").expect("the Shredder at its post");
    (s, hero, machine)
}

fn step(s: &mut Survival) -> Vec<Event> {
    let ev = s.step(DT);
    for e in &ev {
        if let Event::Log(text) = e {
            assert!(!text.contains("put back") && !text.contains("fell through") && !text.contains("shredder rig"), "{text}");
        }
    }
    ev
}

fn rig(s: &Survival) -> ShredderRig {
    s.shredder_rig().expect("the post's rig")
}

fn put(s: &mut Survival, hero: Id, at: Vec3, toward: Vec3) {
    s.move_player(hero, at, facing(at, toward), Default::default());
}

/// The length of the walk from `a` to `b`, if there is one.
fn walk(s: &Survival, a: Vec3, b: Vec3) -> Option<f32> {
    htl_survival::land::walking_route(s.land(), &[a, b]).map(|r| r.windows(2).map(|p| dist2d(p[0], p[1])).sum())
}

/// Every yard of the room's floor a hero can stand on and walk to the post from.
fn room(s: &Survival, g: &ShredderRig) -> Vec<Vec3> {
    let mut out = Vec::new();
    for xi in -34..=34 {
        for yi in -34..=34 {
            let (x, y) = (g.home[0] + xi as f32, g.home[1] + yi as f32);
            let Some(p) = s.land().footing([x, y, g.home[2] + 0.5]) else { continue };
            if (p[2] - g.home[2]).abs() > 3.0 || s.land().blocked_near(x, y, p[2]) || s.land().water_near(x, y, p[2]) { continue; }
            if walk(s, p, g.home).is_some_and(|d| d <= 60.0) { out.push(p); }
        }
    }
    out
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn the_rig_stands_on_open_floor_its_places_walked_to_from_each_other() {
    let (s, _, _) = mine(Some(1_000_000));
    let g = rig(&s);
    let (work, tie, crawl) = (g.work.expect("a place to work"), g.tie.expect("a tie-off"), g.crawl.expect("a place to crawl out"));
    println!("post {:?} facing {:.3}; wreck {:?}; drop {:.1} yd along {:.3}, {:.1} yd wide", g.home, g.face, g.crash, g.len, g.across, g.half * 2.0);
    println!("work {work:?}; crawl {crawl:?}; tie-off {tie:?}: {:.1} yd from his work, {:.1} yd walked", dist2d(work, tie), walk(&s, work, tie).unwrap_or(f32::NAN));
    // A map of it: 1 yd a cell, x down, y across. # wall, x no floor, digits half yards over 47.5.
    let code = |k: i32| if k < 0 { '_' } else if k < 10 { char::from_digit(k as u32, 10).unwrap() } else { char::from(b'a' + (k - 10).min(25) as u8) };
    let floor = room(&s, &g);
    for xi in -22..=22 {
        let x = (g.home[0] - xi as f32).round();
        let mut line = format!("{x:>6.0} ");
        for yi in -32..=28 {
            let y = (g.home[1] + yi as f32).round();
            let at = [x, y, 0.0];
            let mut c = if s.land().blocked_near(x, y, g.home[2] + 0.3) { '#' } else {
                match s.land().footing([x, y, g.home[2] + 0.3]) { None => 'x', Some(p) => if floor.iter().any(|f| dist2d(*f, p) < 0.1) { code(((p[2] - 47.5) * 2.0).round() as i32) } else { '.' } }
            };
            let mid = along(g.crash, g.across, g.len / 2.0);
            let strip = |p: Vec3| {
                let (dx, dy) = (p[0] - mid[0], p[1] - mid[1]);
                let (a, b) = (g.across.cos() * dx + g.across.sin() * dy, -g.across.sin() * dx + g.across.cos() * dy);
                a.abs() <= g.len / 2.0 && b.abs() <= g.half
            };
            if strip(at) { c = '='; }
            for (p, ch) in [(g.home, 'P'), (g.crash, 'W'), (work, 'S'), (crawl, 'c'), (tie, 'T')] {
                if dist2d(p, at) < 0.71 { c = ch; }
            }
            line.push(c);
        }
        println!("{line}");
    }
    let mid = floor_at(&s, along(g.crash, g.across, g.len / 2.0));
    for (name, p) in [("wreck", g.crash), ("work", work), ("crawl", crawl), ("tie-off", tie)] {
        let f = s.land().footing([p[0], p[1], g.home[2] + 0.5]).unwrap_or_else(|| panic!("{name}: no floor"));
        assert!((f[2] - g.home[2]).abs() < 1.5, "{name} on the post's floor: {f:?}");
        assert!(!s.land().blocked_near(p[0], p[1], f[2]), "{name} in the open");
    }
    for (name, a, b) in [("crawl to work", crawl, work), ("work to tie-off", work, tie), ("post to tie-off", g.home, tie), ("door to tie-off", s.players.values().next().unwrap().pos, tie)] {
        let d = walk(&s, a, b).unwrap_or_else(|| panic!("{name}: no way"));
        assert!(d <= dist2d(a, b) * 1.6 + 2.0, "{name}: {d:.1} yd for {:.1}", dist2d(a, b));
    }
    // Whoever cuts it stands a swing's reach beyond the stake, away from the drop: open floor.
    let stand = floor_at(&s, along(tie, facing(mid, tie), 1.5));
    assert!(s.land().walk_clear(stand, tie), "room to swing at the tie-off");
    // Out of the drop to either side, all along it: a stride past its edge, walked straight.
    let mut k = 0.5;
    while k < g.len {
        let p = floor_at(&s, along(g.crash, g.across, k));
        for side in [1.0f32, -1.0] {
            let q = floor_at(&s, along(p, g.across + side * std::f32::consts::FRAC_PI_2, g.half + 0.8));
            let ok = (q[2] - p[2]).abs() < 1.0 && s.land().walk_clear(p, q);
            println!("out of the drop {k:.1} yd along it, side {side:+.0}: {}", if ok { "straight" } else { "NOT straight" });
        }
        k += 1.0;
    }
    // The pile it cut at, beside the wreck: whatever a hero can climb there, and whether any of
    // it is under the log.
    let (mut top, mut under) = (g.home[2], 0);
    for f in room(&s, &g) {
        if dist2d(f, g.crash) > 9.0 || f[2] < g.home[2] + 0.4 { continue; }
        top = top.max(f[2]);
        let (dx, dy) = (f[0] - g.crash[0], f[1] - g.crash[1]);
        let (a, b) = (g.across.cos() * dx + g.across.sin() * dy, -g.across.sin() * dx + g.across.cos() * dy);
        if a > -0.5 && a < g.len + 0.5 && b.abs() < g.half { under += 1; }
    }
    println!("the highest a hero stands within 9 yd of the wreck: {:.1} yd over the post's floor; {under} yd of that under the log", top - g.home[2]);
}

/// The floor under `p`.
fn floor_at(s: &Survival, p: Vec3) -> Vec3 {
    let z = s.shredder_rig().unwrap().home[2];
    s.land().footing([p[0], p[1], z + 0.5]).unwrap_or_else(|| panic!("no floor at {p:?}"))
}

/// One step of a hero's run toward `to` along the floor, looking at `look`; true once there.
#[allow(dead_code)]
fn run_to(s: &mut Survival, hero: Id, to: Vec3, look: Vec3) -> bool {
    let me = s.players[&hero].pos;
    let route = htl_survival::land::walking_route(s.land(), &[me, to]).unwrap_or_else(|| vec![me, to]);
    let (mut left, mut at) = (s.content.player.run_speed * DT, me);
    for p in route.iter().skip(1) {
        let d = dist2d(at, *p);
        if d <= left {
            left -= d;
            at = *p;
        } else {
            at = along(at, facing(at, *p), left);
            break;
        }
    }
    let at = s.land().footing([at[0], at[1], at[2] + 0.5]).unwrap_or(at);
    put(s, hero, at, look);
    dist2d(at, to) < 0.05
}

/// The machine pulled to where the hero stands at `at`, then broken (debug).
fn broken_at(at: [f32; 2]) -> (Survival, Id, Id, f32) {
    let (mut s, hero, machine) = mine(Some(1_000_000));
    let g = rig(&s);
    let at = floor_at(&s, [at[0], at[1], 0.0]);
    put(&mut s, hero, at, g.home);
    s.dev_hurt(hero, 0.99, "dm_shredder").unwrap();
    let t = s.clock;
    while s.clock < t + 25.0 && dist2d(s.mobs[&machine].pos, at) > 8.0 { step(&mut s); }
    assert!(dist2d(s.mobs[&machine].pos, at) <= 8.0, "the machine never came to {at:?}: {:?}", s.mobs[&machine].pos);
    put(&mut s, hero, at, g.home);
    for _ in 0..4 { step(&mut s); }
    s.dev_hurt(hero, 0.0, "dm_shredder").unwrap();
    let t0 = s.clock;
    (s, hero, machine, t0)
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn broken_anywhere_a_fight_can_be_the_machine_is_back_at_its_pile_in_about_3_s_and_hurts_nobody() {
    let (s, _, _) = mine(Some(1_000_000));
    let g = rig(&s);
    // Every yard of the room: how long its way back is, and how fast it runs it.
    let floor = room(&s, &g);
    let mut worst = (0.0f32, 0.0f32, g.home);
    let mut slow = 0;
    for p in &floor {
        let (len, speed) = s.runaway_pace(*p, g.home, g.crash);
        if len / speed > worst.1 { worst = (len, len / speed, *p); }
        if len / speed > 2.5 { slow += 1; }
    }
    println!("{} yd of floor a hero can stand on and walk to the post from; the longest way back {:.0} yd from {:.0?}: {:.1} s; {slow} yd of it over 2.5 s", floor.len(), worst.0, worst.2, worst.1);
    assert!(worst.1 < 4.0, "the run stays one beat from anywhere");
    // And run for real from where a fight is: before the door, on the walkway's ends, out on the
    // low floor either side, at the far wall, at its post.
    for at in [[-290.0f32, -527.0], [-289.0, -516.0], [-289.0, -490.0], [-279.0, -520.0], [-279.0, -497.0], [-272.0, -488.0], [-299.0, -512.0], [-300.0, -492.0], [-284.0, -503.0]] {
        let (mut s, hero, machine, t0) = broken_at(at);
        let from = s.mobs[&machine].pos;
        let (len, speed) = s.runaway_pace(from, g.home, g.crash);
        let mut said = false;
        let mut first = true;
        let (mut health, mut stood) = (0, [0.0; 3]);
        while first || s.shredder_stage() == "runaway" {
            assert!(s.clock - t0 < 7.0, "from {at:?}: it never crashed");
            let ev = step(&mut s);
            if first { (health, stood) = (s.players[&hero].health, s.players[&hero].pos); }
            first = false;
            said |= ev.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "No, no, not now!"));
            let m = &s.mobs[&machine];
            assert!(s.land().footing(m.pos).is_some(), "on the floor all the way: {:?}", m.pos);
        }
        let took = s.clock - t0;
        let wreck = s.mobs[&machine].pos;
        // Its line is said as it breaks, or, like every line, 2 s after the one before it (here
        // the pull's, when it is broken on the spot).
        let at_once = said;
        while !said && s.clock - t0 < 2.1 {
            said = step(&mut s).iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "No, no, not now!"));
        }
        println!("broken {:.0} yd from its post at {from:.0?}: {len:.0} yd back at {speed:.0} yd/s, crashed after {took:.2} s; stage {}{}", dist2d(from, g.home), s.shredder_stage(), if at_once { "" } else { "; its line waited its turn" });
        assert!(said, "its line");
        assert!(dist2d(wreck, g.crash) < 0.5 && (wreck[2] - g.crash[2]).abs() < 0.6, "the wreck lies at the pile: {wreck:?}");
        assert!((0.7..=3.6).contains(&took), "{took}");
        assert_eq!((s.players[&hero].health, s.players[&hero].pos), (health, stood), "nobody hit, nobody shoved");
    }
    let _ = Input::Attack;
}

/// (Debug) The hero's health back to full: these measure time, not who survives.
fn mend(s: &mut Survival, hero: Id) {
    let p = s.players.get_mut(&hero).unwrap();
    p.health = p.max_health;
}

/// One step with the hero on Sneed: run at him until in reach, then face him and `blow`.
fn fight_him(s: &mut Survival, hero: Id, him: Id, blow: &[Input]) -> Vec<Event> {
    let (me, his) = (s.players[&hero].pos, s.mobs[&him].pos);
    if dist2d(me, his) > 2.6 {
        let to = along(his, facing(his, me), 2.2);
        let to = s.land().footing([to[0], to[1], his[2] + 0.5]).unwrap_or(to);
        run_to(s, hero, to, his);
    } else {
        put(s, hero, me, his);
        for b in blow { s.input(hero, *b); }
    }
    mend(s, hero);
    step(s)
}

/// The machine broken with a warrior at `at`, then Sneed out of it and on his feet. With `stay`
/// the hero waits where they stand (he comes to them); else they go and stand on him.
fn on_foot(at: [f32; 2], stay: bool, spend: bool) -> (Survival, Id, Id) {
    let (mut s, hero, machine) = mine(None);
    mend(&mut s, hero);
    let g = rig(&s);
    if spend {
        // The log cut before the fight is ever at the wreck: nothing under it.
        let tie = g.tie.unwrap();
        let stand = floor_at(&s, along(tie, g.face, 1.5));
        put(&mut s, hero, stand, tie);
        s.input(hero, Input::Attack);
        for _ in 0..60 { mend(&mut s, hero); step(&mut s); }
        assert_eq!(rig(&s).state, "down", "the log cut at the pull");
    }
    let at = floor_at(&s, [at[0], at[1], 0.0]);
    put(&mut s, hero, at, g.home);
    if s.mobs[&machine].state.foe().is_none() { s.dev_hurt(hero, 0.99, "dm_shredder").unwrap(); }
    let t = s.clock;
    while s.clock < t + 25.0 && dist2d(s.mobs[&machine].pos, at) > 8.0 { mend(&mut s, hero); step(&mut s); }
    put(&mut s, hero, at, g.home);
    mend(&mut s, hero);
    s.dev_hurt(hero, 0.0, "dm_shredder").unwrap();
    while live(&s, "dm_sneed").is_none() { mend(&mut s, hero); step(&mut s); }
    let him = live(&s, "dm_sneed").unwrap();
    while s.shredder_stage() != "on foot" {
        assert!(s.clock < t + 60.0);
        mend(&mut s, hero);
        step(&mut s);
    }
    if !stay {
        let t = s.clock;
        while s.clock < t + 1.0 { fight_him(&mut s, hero, him, &[]); }
    }
    (s, hero, him)
}

const LINE_BOLT: &str = "Just... one... bolt...";

fn said(ev: &[Event], line: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == line))
}

/// The race from `at`: the hero notices his line `late` s after it, runs to the tie-off and cuts
/// it once his hands are on the wreck. (won, his run s, the hero at the stake s, cut s, end s).
fn race(at: [f32; 2], stay: bool, late: f32, show: bool) -> (bool, f32, f32) {
    let (mut s, hero, him) = on_foot(at, stay, false);
    let g = rig(&s);
    let (tie, work) = (g.tie.unwrap(), g.work.unwrap());
    // Until his line.
    let t = s.clock;
    loop {
        assert!(s.clock < t + 20.0, "he never went to mend it");
        let ev = if stay { mend(&mut s, hero); step(&mut s) } else { fight_him(&mut s, hero, him, &[]) };
        if said(&ev, LINE_BOLT) { break; }
    }
    let t0 = s.clock;
    let (me, his) = (s.players[&hero].pos, s.mobs[&him].pos);
    let stand = floor_at(&s, along(tie, facing(tie, me), 1.5));
    let (theirs, his_way) = (walk(&s, me, stand).unwrap_or(f32::NAN), walk(&s, his, work).unwrap_or(f32::NAN));
    let (mut hands, mut there, mut cut, mut won, mut end) = (None, None, None, false, 0.0);
    while s.clock < t0 + 30.0 {
        let now = s.clock - t0;
        if now >= late && there.is_none() && run_to(&mut s, hero, stand, tie) { there = Some(s.clock - t0 + DT); }
        if there.is_some() && cut.is_none() && s.shredder_stage() == "mending" {
            put(&mut s, hero, stand, tie);
            s.input(hero, Input::Attack);
            cut = Some(now);
        }
        mend(&mut s, hero);
        step(&mut s);
        if s.shredder_stage() == "mending" { hands.get_or_insert(s.clock - t0); }
        if rig(&s).state == "down" || live(&s, "dm_shredder").is_some() {
            won = s.shredder_stage() == "spent" && live(&s, "dm_shredder").is_none();
            end = s.clock - t0;
            break;
        }
    }
    if show {
        println!("  noticed {late:.1} s late (he {his_way:.1} yd from his work, the hero {theirs:.1} yd from the stake): his hands on it at {:.2} s, the hero at the stake at {:.2} s, cut at {:.2} s: {} at {end:.2} s",
            hands.unwrap_or(-1.0), there.unwrap_or(-1.0), cut.unwrap_or(-1.0), if won { "STOPPED" } else { "the machine stands" });
    }
    (won, hands.unwrap_or(f32::NAN), end)
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn the_race_for_the_log_is_won_by_a_hero_who_goes_at_once_and_lost_by_one_who_waits() {
    // On him by the wreck (where he crawls out, and where a hero who went for him stands), and
    // held off at three places a fight is.
    let mut tables = Vec::new();
    // Where the hero is at his line, and how late they may go and still win, and lose.
    let stands: [(&str, [f32; 2], bool, &[f32], &[f32]); 4] = [
        ("on him by the wreck", [-284.0, -503.0], false, &[0.0, 1.0, 1.5, 2.5], &[3.0, 3.5, 5.0]),
        ("waiting at the walkway's south end", [-289.0, -516.0], true, &[0.0], &[1.5, 3.0]),
        ("waiting on the low floor north-east", [-272.0, -488.0], true, &[0.0, 1.0, 1.5], &[3.0, 5.0]),
        ("waiting before the door", [-290.0, -527.0], true, &[0.0, 1.5], &[3.5, 5.0])];
    for (name, at, stay, wins, loses) in stands {
        println!("{name}:");
        let mut table = Vec::new();
        for late in [0.0f32, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 5.0] {
            let (won, hands, _) = race(at, stay, late, true);
            assert!(hands.is_finite(), "he reaches his work from {at:?}");
            table.push((late, won));
        }
        tables.push((name, table, wins, loses));
    }
    for (name, table, wins, loses) in tables {
        let won = |late: f32| table.iter().find(|r| r.0 == late).unwrap().1;
        assert!(wins.iter().all(|l| won(*l)), "{name}: a hero who goes by then wins it");
        assert!(!loses.iter().any(|l| won(*l)), "{name}: a hero who waits loses it");
    }
}

/// A warrior with what a hero joins with, on Sneed with the log already spent. `all_along`: on
/// him from when he is on his feet, else from when his hands are on the wreck. Returns how it ended,
/// how long after his hands were on it, his posture's peak and his health as he began.
fn stance(blow: &[Input], all_along: bool) -> (&'static str, f32, u32, u32) {
    let (mut s, hero, him) = on_foot([-284.0, -503.0], false, true);
    let t = s.clock;
    loop {
        assert!(s.clock < t + 20.0, "he never went to mend it");
        let ev = fight_him(&mut s, hero, him, if all_along { blow } else { &[] });
        if said(&ev, LINE_BOLT) { break; }
    }
    let t0 = s.clock;
    let (mut hands, mut peak, mut health0, mut out) = (None, 0, 0, "the machine stands");
    while s.clock < t0 + 30.0 {
        let mending = s.shredder_stage() == "mending";
        fight_him(&mut s, hero, him, if all_along || mending { blow } else { &[] });
        if s.shredder_stage() == "mending" && hands.is_none() { hands = Some(s.clock - t0); health0 = s.mobs[&him].health; }
        if live(&s, "dm_shredder").is_some() { break; }
        if s.mobs.get(&him).is_none_or(|m| m.health == 0) { out = "he is dead"; break; }
        peak = peak.max(s.posture_of(him));
        if s.shredder_stage() == "spent" { out = "STANCE BROKEN"; break; }
    }
    (out, s.clock - t0 - hands.unwrap_or(0.0), peak, health0)
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn with_the_log_spent_a_warrior_on_him_breaks_his_stance_before_the_machine_stands_and_only_just() {
    // The log cut at the pull, nobody under it. A warrior with what a hero joins with, beside
    // him at the wreck. (At his own stance of 160 the machine stood: light blows reached 97 of
    // it in the 4.2 s, heavy ones 67. `MEND_POSTURE` is sized from this.)
    for (name, blow) in [("light blows", &[Input::Attack][..]), ("heavy blows", &[Input::Heavy][..])] {
        // On him ever since he was on his feet: he is dead, or reeling, before the machine stands.
        let (out, secs, peak, health) = stance(blow, true);
        println!("{name}, on him all along: his hands on it with {health} health; {out} {secs:.2} s after that; his posture peaked at {peak}");
        assert_ne!(out, "the machine stands", "{name}, all along");
        // Begun only as his hands reach the wreck, from full health and no posture: his stance
        // breaks in time, and not early.
        let (out, secs, peak, health) = stance(blow, false);
        println!("{name}, begun as he starts mending: his hands on it with {health} health; {out} {secs:.2} s after that; his posture peaked at {peak}");
        assert_eq!(out, "STANCE BROKEN", "{name}, begun as he starts mending");
        assert!((2.5..=4.1).contains(&secs), "possible, and hard: {secs}");
    }
}

/// How far `p` stands outside the wreck's drawn pose: GoblinShredder's Death as it lies at
/// `origin` facing `face` (the sequence's own bounds, measured; not the small static hull).
fn clearance(p: Vec3, origin: Vec3, face: f32, scale: f32) -> f32 {
    let (dx, dy) = (p[0] - origin[0], p[1] - origin[1]);
    let (x, y) = (face.cos() * dx + face.sin() * dy, -face.sin() * dx + face.cos() * dy);
    [x - 3.2587168 * scale, -4.287402 * scale - x, y - 4.791185 * scale, -4.0007997 * scale - y].into_iter().fold(f32::NEG_INFINITY, f32::max)
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn left_alone_he_walks_to_his_work_clear_of_the_wreck_a_plain_hit_lands_and_stops_nothing_and_the_machine_stands_where_it_lay() {
    let (mut s, hero, him) = on_foot([-289.0, -516.0], true, false);
    let g = rig(&s);
    let work = g.work.unwrap();
    let old = *s.mobs.iter().find(|(_, m)| m.health == 0 && s.content.mobs[m.def].key == "dm_shredder").expect("the wreck lies there").0;
    let (wreck, lay, scale) = (s.mobs[&old].pos, s.mobs[&old].facing, s.content.mobs[s.mobs[&old].def].scale);
    // On foot, to his work, every step of it walked on the floor.
    let t = s.clock;
    let mut was = s.mobs[&him].pos;
    let began = loop {
        assert!(s.clock < t + 25.0, "he never began to mend it");
        mend(&mut s, hero);
        let ev = step(&mut s);
        let at = s.mobs[&him].pos;
        assert!(s.land().footing(at).is_some(), "on the floor: {at:?}");
        assert!(dist2d(was, at) <= 7.5 * DT + 0.002, "no jump: {was:?} to {at:?}");
        was = at;
        if let Some(ms) = ev.iter().find_map(|e| match e { Event::Cast { caster, spell: 2575, ms } if *caster == him => Some(*ms), _ => None }) {
            assert_eq!(ms, 4200, "the bar the heroes see is the repair's own length");
            assert!(ev.iter().any(|e| matches!(e, Event::MobStop { id, facing, .. } if *id == him && (*facing - s.mobs[&him].facing).abs() < 0.001)), "the client is told which way he works");
            break s.clock;
        }
    };
    let at = s.mobs[&him].pos;
    assert!(dist2d(at, work) < 0.6, "at his one place under the log: {at:?}");
    let body = 0.5 * s.content.mobs[s.mobs[&him].def].scale;
    let gap = clearance(at, wreck, lay, scale);
    println!("the wreck at {wreck:.2?} lying {lay:.3}; he works at {at:.2?}, {gap:.2} yd outside its pose (his body {body:.2} yd)");
    assert!(gap >= body + 0.1, "he hammers beside the wreck, not inside it: {gap}");
    // A plain blow from beside him lands, hurts him, and stops nothing.
    let beside = floor_at(&s, along(at, facing(at, g.tie.unwrap()), 1.6));
    assert!(s.land().walk_clear(beside, at), "an open way to him");
    put(&mut s, hero, beside, at);
    let hp = s.mobs[&him].health;
    s.input(hero, Input::Attack);
    for _ in 0..14 { mend(&mut s, hero); step(&mut s); }
    assert!(s.mobs[&him].health < hp, "a plain blow lands on him at his work");
    assert_eq!(s.shredder_stage(), "mending", "and does not stop it");
    // Left to it, the machine stands where the wreck lay, at a third of its health. (His health
    // is read as he climbs in: a blow already swung lands after the hero is put away.)
    let far = floor_at(&s, [-289.0, -516.0, 0.0]);
    put(&mut s, hero, far, wreck);
    let mut hp = s.mobs[&him].health;
    while live(&s, "dm_shredder").is_none() {
        assert!(s.clock < began + 4.2 + 0.2, "it stands when the repair is done");
        hp = s.mobs[&him].health;
        mend(&mut s, hero);
        step(&mut s);
    }
    assert!(s.clock - began >= 4.2 - DT, "not before: {}", s.clock - began);
    let back = live(&s, "dm_shredder").unwrap();
    assert!(dist2d(s.mobs[&back].pos, wreck) < 0.1, "where the wreck lay, not where he worked");
    assert_eq!(s.mobs[&back].health, s.mobs[&back].max_health / 3);
    assert!(!s.mobs.contains_key(&old) && !s.mobs.contains_key(&him), "one machine, no wreck, no driver");
    // Broken again it runs back the same way, he is out with the health he had, and there is no
    // second repair (on his feet again his stage is "spent").
    s.dev_hurt(hero, 0.0, "dm_shredder").unwrap();
    let t = s.clock;
    while live(&s, "dm_sneed").is_none() || s.shredder_stage() != "spent" {
        assert!(s.clock < t + 15.0, "he never came out again");
        mend(&mut s, hero);
        step(&mut s);
    }
    let again = live(&s, "dm_sneed").unwrap();
    assert_eq!(s.mobs[&again].health, hp);
    let lies = s.mobs.values().find(|m| m.health == 0 && s.content.mobs[m.def].key == "dm_shredder").expect("the second wreck lies there as he gets up").pos;
    assert!(dist2d(lies, wreck) < 0.5, "the second wreck lies where the first did: {lies:?}");
    for _ in 0..300 {
        mend(&mut s, hero);
        assert!(!step(&mut s).iter().any(|e| matches!(e, Event::Cast { spell: 2575, .. })), "no second repair");
    }
    assert!(!s.slain("sneed"));
}
