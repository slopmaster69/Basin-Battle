//! Server packets, built to the 1.12.1 layouts benilla's client reads.

use benilla_protocol::messages::opcode as op;

use crate::fields::Fields;
use crate::wire::Body;

pub use op::*;

/// A packet ready to send: opcode and body.
#[derive(Debug, Clone)]
pub struct Packet {
    pub opcode: u16,
    pub body: Vec<u8>,
}

impl Packet {
    pub fn new(opcode: u16, body: Body) -> Self {
        Self {
            opcode,
            body: body.into_vec(),
        }
    }
    pub fn empty(opcode: u16) -> Self {
        Self {
            opcode,
            body: Vec::new(),
        }
    }
}

pub const AUTH_OK: u8 = 0x0C;

pub fn auth_challenge(seed: u32) -> Packet {
    let mut b = Body::new();
    b.u32(seed);
    Packet::new(op::SMSG_AUTH_CHALLENGE, b)
}

pub fn auth_response_ok() -> Packet {
    let mut b = Body::new();
    b.u8(AUTH_OK).u32(0).u8(0).u32(0); // billing: time left, plan flags, time rested
    Packet::new(op::SMSG_AUTH_RESPONSE, b)
}

/// One character on the character-select screen.
pub struct CharEntry<'a> {
    pub guid: u64,
    pub name: &'a str,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub appearance: [u8; 5],
    pub level: u8,
    pub zone: u32,
    pub map: u32,
    pub pos: [f32; 3],
}

pub fn char_enum(chars: &[CharEntry]) -> Packet {
    let mut b = Body::new();
    b.u8(chars.len() as u8);
    for c in chars {
        b.u64(c.guid)
            .cstr(c.name)
            .u8(c.race)
            .u8(c.class)
            .u8(c.gender)
            .bytes(&c.appearance)
            .u8(c.level)
            .u32(c.zone)
            .u32(c.map)
            .vec3(c.pos)
            .u32(0) // guild
            .u32(0) // flags
            .u8(0) // first login: no
            .u32(0) // pet display
            .u32(0) // pet level
            .u32(0); // pet family
        for _ in 0..19 {
            b.u32(0).u8(0); // equipment: display id, inventory type
        }
        b.u32(0).u8(0); // first bag
    }
    Packet::new(op::SMSG_CHAR_ENUM, b)
}

pub fn login_verify_world(map: u32, pos: [f32; 3], facing: f32) -> Packet {
    let mut b = Body::new();
    b.u32(map).vec3(pos).f32(facing);
    Packet::new(op::SMSG_LOGIN_VERIFY_WORLD, b)
}

/// The far teleport's warning: the client covers the screen for a map change.
pub fn transfer_pending(map: u32) -> Packet {
    let mut b = Body::new();
    b.u32(map);
    Packet::new(op::SMSG_TRANSFER_PENDING, b)
}

/// The far teleport itself: the new map and where on it; the client answers
/// `MSG_MOVE_WORLDPORT_ACK` once it has loaded it.
pub fn new_world(map: u32, pos: [f32; 3], facing: f32) -> Packet {
    let mut b = Body::new();
    b.u32(map).vec3(pos).f32(facing);
    Packet::new(op::SMSG_NEW_WORLD, b)
}

pub fn tutorial_flags_all_seen() -> Packet {
    let mut b = Body::new();
    for _ in 0..8 {
        b.u32(u32::MAX);
    }
    Packet::new(op::SMSG_TUTORIAL_FLAGS, b)
}

/// The packed game-clock date the client shows: minute, hour, weekday, day, month, year.
pub fn wow_time_now() -> u32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hour, minute) = ((rem / 3600) as u32, ((rem % 3600) / 60) as u32);
    let weekday = ((days + 4) % 7) as u32; // 1970-01-01 was a Thursday; 0 = Sunday
    // Civil date from days since the epoch (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (yoe + era * 400 + if month <= 2 { 1 } else { 0 }) as u32;
    minute
        | (hour << 6)
        | (weekday << 11)
        | ((day - 1) << 14)
        | ((month - 1) << 20)
        | (year.saturating_sub(2000) << 24)
}

pub fn login_set_time_speed() -> Packet {
    let mut b = Body::new();
    b.u32(wow_time_now()).f32(1.0 / 60.0);
    Packet::new(op::SMSG_LOGIN_SETTIMESPEED, b)
}

pub fn initial_spells(spells: &[u16]) -> Packet {
    let mut b = Body::new();
    b.u8(0).u16(spells.len() as u16);
    for &s in spells {
        b.u16(s).u16(0);
    }
    b.u16(0); // no cooldowns
    Packet::new(op::SMSG_INITIAL_SPELLS, b)
}

pub fn action_buttons_empty() -> Packet {
    let mut b = Body::new();
    for _ in 0..120 {
        b.u32(0);
    }
    Packet::new(op::SMSG_ACTION_BUTTONS, b)
}

pub fn initialize_factions_empty() -> Packet {
    let mut b = Body::new();
    b.u32(64);
    for _ in 0..64 {
        b.u8(0).u32(0);
    }
    Packet::new(op::SMSG_INITIALIZE_FACTIONS, b)
}

pub fn bindpoint(pos: [f32; 3], map: u32, area: u32) -> Packet {
    let mut b = Body::new();
    b.vec3(pos).u32(map).u32(area);
    Packet::new(op::SMSG_BINDPOINTUPDATE, b)
}

pub fn pong(seq: u32) -> Packet {
    let mut b = Body::new();
    b.u32(seq);
    Packet::new(op::SMSG_PONG, b)
}

pub fn query_time_response() -> Packet {
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);
    let mut b = Body::new();
    b.u32(unix);
    Packet::new(op::SMSG_QUERY_TIME_RESPONSE, b)
}

pub fn logout_response_ok() -> Packet {
    let mut b = Body::new();
    b.u32(0).u8(1); // allowed, instant
    Packet::new(op::SMSG_LOGOUT_RESPONSE, b)
}

pub fn name_query_response(guid: u64, name: &str, race: u8, gender: u8, class: u8) -> Packet {
    let mut b = Body::new();
    b.u64(guid)
        .cstr(name)
        .u8(0) // realm name: empty
        .u32(race as u32)
        .u32(gender as u32)
        .u32(class as u32);
    Packet::new(op::SMSG_NAME_QUERY_RESPONSE, b)
}

pub struct CreatureInfo<'a> {
    pub entry: u32,
    pub name: &'a str,
    pub subname: &'a str,
    pub creature_type: u32,
    pub rank: u32,
    pub display_id: u32,
}

pub fn creature_query_response(info: &CreatureInfo) -> Packet {
    let mut b = Body::new();
    b.u32(info.entry)
        .cstr(info.name)
        .cstr("")
        .cstr("")
        .cstr("")
        .cstr(info.subname)
        .u32(0) // type flags
        .u32(info.creature_type)
        .u32(0) // family
        .u32(info.rank)
        .u32(0) // unknown
        .u32(0) // pet spell list
        .u32(info.display_id)
        .u8(0) // civilian
        .u8(0); // racial leader
    Packet::new(op::SMSG_CREATURE_QUERY_RESPONSE, b)
}

/// `SMSG_GAMEOBJECT_QUERY_RESPONSE`: the template, with nothing to use or open (all data zero).
pub fn gameobject_query_response(entry: u32, type_id: u32, display_id: u32, name: &str) -> Packet {
    let mut b = Body::new();
    b.u32(entry).u32(type_id).u32(display_id).cstr(name);
    for _ in 0..4 {
        b.cstr(""); // name2..name4, and the icon
    }
    for _ in 0..24 {
        b.u32(0);
    }
    Packet::new(op::SMSG_GAMEOBJECT_QUERY_RESPONSE, b)
}

pub fn gameobject_query_miss(entry: u32) -> Packet {
    let mut b = Body::new();
    b.u32(entry | 0x8000_0000);
    Packet::new(op::SMSG_GAMEOBJECT_QUERY_RESPONSE, b)
}

pub fn creature_query_miss(entry: u32) -> Packet {
    let mut b = Body::new();
    b.u32(entry | 0x8000_0000);
    Packet::new(op::SMSG_CREATURE_QUERY_RESPONSE, b)
}

// --- objects -----------------------------------------------------------------------------------

pub const TYPE_ITEM: u8 = 1;
pub const TYPE_CONTAINER: u8 = 2;
pub const TYPE_UNIT: u8 = 3;
pub const TYPE_PLAYER: u8 = 4;
pub const TYPE_GAMEOBJECT: u8 = 5;
pub const TYPE_DYNAMICOBJECT: u8 = 6;

const UPDATE_FLAG_SELF: u8 = 0x01;
const UPDATE_FLAG_ALL: u8 = 0x10;
const UPDATE_FLAG_LIVING: u8 = 0x20;
const UPDATE_FLAG_HAS_POSITION: u8 = 0x40;

/// Walk, run, run back, swim, swim back, turn rate: the stock values, with the run speed given.
pub fn speeds(run: f32) -> [f32; 6] {
    speeds_turning(run, std::f32::consts::PI)
}

/// The stock speeds with a run speed and a turn rate (radians a second).
pub fn speeds_turning(run: f32, turn: f32) -> [f32; 6] {
    [2.5, run, 4.5, 4.722_222, 2.5, turn]
}

/// How an object being created is placed.
#[derive(Debug, Clone, Copy)]
pub enum Placement {
    /// A unit or player: position, facing, speeds.
    Living { pos: [f32; 3], facing: f32, speeds: [f32; 6] },
    /// Something that just sits somewhere (a ground effect).
    At { pos: [f32; 3], facing: f32 },
    /// No place in the world (an item in a bag).
    Nowhere,
}

/// One object to create in an `SMSG_UPDATE_OBJECT`.
pub struct Create<'a> {
    pub guid: u64,
    pub object_type: u8,
    pub is_self: bool,
    pub placement: Placement,
    pub fields: &'a Fields,
}

/// Build one `SMSG_UPDATE_OBJECT` from creates and value updates.
pub fn update_object(creates: &[Create], values: &[(u64, &Fields)]) -> Packet {
    let mut b = Body::new();
    b.u32((creates.len() + values.len()) as u32).u8(0);
    for c in creates {
        b.u8(3) // CREATE_OBJECT2
            .packed_guid(c.guid)
            .u8(c.object_type);
        let mut flags = UPDATE_FLAG_ALL;
        if c.is_self {
            flags |= UPDATE_FLAG_SELF;
        }
        match c.placement {
            Placement::Living { pos, facing, speeds } => {
                b.u8(flags | UPDATE_FLAG_LIVING | UPDATE_FLAG_HAS_POSITION)
                    .u32(0) // movement flags
                    .u32(crate::clock_ms())
                    .vec3(pos)
                    .f32(facing)
                    .u32(0); // fall time
                for s in speeds {
                    b.f32(s);
                }
            }
            Placement::At { pos, facing } => {
                b.u8(flags | UPDATE_FLAG_HAS_POSITION).vec3(pos).f32(facing);
            }
            Placement::Nowhere => {
                b.u8(flags);
            }
        }
        b.u32(1); // UPDATE_FLAG_ALL's word
        c.fields.write(&mut b);
    }
    for (guid, fields) in values {
        b.u8(0).packed_guid(*guid);
        fields.write(&mut b);
    }
    Packet::new(op::SMSG_UPDATE_OBJECT, b)
}

pub fn destroy_object(guid: u64) -> Packet {
    let mut b = Body::new();
    b.u64(guid);
    Packet::new(op::SMSG_DESTROY_OBJECT, b)
}

const SPLINE_FLAG_RUNMODE: u32 = 0x100;

/// Move a creature along `path` (its first point is where it is now) over `duration_ms`.
pub fn monster_move(guid: u64, spline_id: u32, path: &[[f32; 3]], duration_ms: u32) -> Packet {
    let mut b = Body::new();
    let start = path[0];
    let end = *path.last().expect("a path has points");
    b.packed_guid(guid)
        .vec3(start)
        .u32(spline_id)
        .u8(0) // move type: normal
        .u32(SPLINE_FLAG_RUNMODE)
        .u32(duration_ms)
        .u32((path.len() - 1) as u32)
        .vec3(end);
    if path.len() > 2 {
        for p in &path[1..path.len() - 1] {
            let pack = |v: f32, shift: u32, mask: i32| ((v * 4.0).round() as i32 & mask) << shift;
            let off = [end[0] - p[0], end[1] - p[1], end[2] - p[2]];
            b.i32(pack(off[0], 0, 0x7FF) | pack(off[1], 11, 0x7FF) | pack(off[2], 22, 0x3FF));
        }
    }
    Packet::new(op::SMSG_MONSTER_MOVE, b)
}

/// Move a creature along `path` at a walk, turned to `target` the while (circling a foe).
pub fn monster_move_facing(guid: u64, spline_id: u32, path: &[[f32; 3]], duration_ms: u32, target: u64) -> Packet {
    let mut b = Body::new();
    let start = path[0];
    let end = *path.last().expect("a path has points");
    b.packed_guid(guid)
        .vec3(start)
        .u32(spline_id)
        .u8(3) // move type: facing a target
        .u64(target)
        .u32(0) // spline flags: walking
        .u32(duration_ms)
        .u32(1)
        .vec3(end);
    Packet::new(op::SMSG_MONSTER_MOVE, b)
}

/// Stop a creature where it stands and turn it to `facing`.
pub fn monster_face(guid: u64, spline_id: u32, pos: [f32; 3], facing: f32) -> Packet {
    let mut b = Body::new();
    b.packed_guid(guid)
        .vec3(pos)
        .u32(spline_id)
        .u8(4) // move type: final angle
        .f32(facing)
        .u32(0) // spline flags
        .u32(0) // duration
        .u32(1)
        .vec3(pos);
    Packet::new(op::SMSG_MONSTER_MOVE, b)
}

/// `SMSG_ENVIRONMENTALDAMAGELOG`: the victim's raw guid, the kind (2 a fall), the damage, and
/// nothing absorbed or resisted.
pub fn environmental_damage(victim: u64, kind: u8, damage: u32) -> Packet {
    let mut b = Body::new();
    b.u64(victim).u8(kind).u32(damage).u32(0).u32(0);
    Packet::new(op::SMSG_ENVIRONMENTALDAMAGELOG, b)
}

pub fn attack_start(attacker: u64, victim: u64) -> Packet {
    let mut b = Body::new();
    b.u64(attacker).u64(victim);
    Packet::new(op::SMSG_ATTACKSTART, b)
}

pub fn attack_stop(attacker: u64, victim: u64) -> Packet {
    let mut b = Body::new();
    b.packed_guid(attacker).packed_guid(victim).u32(0);
    Packet::new(op::SMSG_ATTACKSTOP, b)
}

const HITINFO_NORMALSWING2: u32 = 0x02;
const HITINFO_CRITICALHIT: u32 = 0x80;
const VICTIMSTATE_NORMAL: u32 = 1;

pub fn attacker_state_update(attacker: u64, victim: u64, damage: u32, crit: bool) -> Packet {
    let mut b = Body::new();
    let hit = HITINFO_NORMALSWING2 | if crit { HITINFO_CRITICALHIT } else { 0 };
    b.u32(hit)
        .packed_guid(attacker)
        .packed_guid(victim)
        .u32(damage)
        .u8(1) // one damage part
        .u32(0) // physical
        .f32(damage as f32)
        .u32(damage)
        .u32(0) // absorbed
        .u32(0) // resisted
        .u32(VICTIMSTATE_NORMAL)
        .u32(0) // attacker state
        .u32(0) // melee spell
        .u32(0); // blocked
    Packet::new(op::SMSG_ATTACKERSTATEUPDATE, b)
}

/// One blow, with how it came out: `victim_state` 1 hit, 2 dodge, 5 block (`blocked` the amount
/// stopped); `miss` a swing at nothing (no number shows); `silent` shows the result without the
/// attacker swinging again (the swing already played).
#[allow(clippy::too_many_arguments)]
pub fn swing_update(attacker: u64, victim: u64, damage: u32, victim_state: u32, blocked: u32, miss: bool, silent: bool, crit: bool) -> Packet {
    swing_update_with(attacker, victim, damage, victim_state, blocked, miss, silent, crit, false)
}

/// [`swing_update`], a crushing blow when `crushing` (the big blood spurt and the hurt cry).
#[allow(clippy::too_many_arguments)]
pub fn swing_update_with(attacker: u64, victim: u64, damage: u32, victim_state: u32, blocked: u32, miss: bool, silent: bool, crit: bool, crushing: bool) -> Packet {
    let mut b = Body::new();
    let mut hit = HITINFO_NORMALSWING2;
    if crushing {
        hit |= HITINFO_CRUSHING;
    }
    if miss {
        hit |= HITINFO_MISS;
    }
    if crit {
        hit |= HITINFO_CRITICALHIT;
    }
    if silent {
        hit |= HITINFO_NOACTION;
    }
    b.u32(hit)
        .packed_guid(attacker)
        .packed_guid(victim)
        .u32(damage)
        .u8(1)
        .u32(0)
        .f32(damage as f32)
        .u32(damage)
        .u32(0)
        .u32(0)
        .u32(victim_state)
        .u32(0)
        .u32(0)
        .u32(blocked);
    Packet::new(op::SMSG_ATTACKERSTATEUPDATE, b)
}

const HITINFO_MISS: u32 = 0x10;
const HITINFO_CRUSHING: u32 = 0x8000;
/// The swing's result without its animation (`0x10000`).
const HITINFO_NOACTION: u32 = 0x10000;
pub const VICTIM_HIT: u32 = 1;
pub const VICTIM_DODGE: u32 = 2;
pub const VICTIM_BLOCK: u32 = 5;
pub const VICTIM_PARRY: u32 = 3;

/// `SMSG_MOVE_KNOCK_BACK`: launch our mover along `dir` at `speed`, `lift` upward. The client
/// flies it (and lands it) itself, against the world's collision.
pub fn knock_back(guid: u64, counter: u32, dir: f32, speed: f32, lift: f32) -> Packet {
    let mut b = Body::new();
    b.packed_guid(guid).u32(counter).f32(dir.cos()).f32(dir.sin()).f32(speed).f32(-lift);
    Packet::new(op::SMSG_MOVE_KNOCK_BACK, b)
}

/// `SMSG_EMOTE`: a one-off gesture (a roar) on a unit.
pub fn emote(guid: u64, emote: u32) -> Packet {
    let mut b = Body::new();
    b.u32(emote).u64(guid);
    Packet::new(op::SMSG_EMOTE, b)
}

pub const CHAT_SAY: u8 = 0x00;
pub const CHAT_PARTY: u8 = 0x01;
pub const CHAT_YELL: u8 = 0x05;
pub const CHAT_SYSTEM: u8 = 0x0A;
pub const CHAT_RAID_WARNING: u8 = 0x58;
pub const LANG_UNIVERSAL: u32 = 0;
/// Add-on data: the client hands it to add-ons and never prints it.
pub const LANG_ADDON: u32 = 0xFFFF_FFFF;
pub const LANG_COMMON: u32 = 7;

/// A chat line. `sender` is 0 for the server.
pub fn chat(kind: u8, language: u32, sender: u64, text: &str) -> Packet {
    let mut b = Body::new();
    b.u8(kind).u32(language).u64(sender);
    if matches!(kind, CHAT_SAY | CHAT_PARTY | CHAT_YELL) {
        b.u64(sender);
    }
    b.len_str(text).u8(0);
    Packet::new(op::SMSG_MESSAGECHAT, b)
}

pub fn notification(text: &str) -> Packet {
    let mut b = Body::new();
    b.cstr(text);
    Packet::new(op::SMSG_NOTIFICATION, b)
}

pub struct GroupMember<'a> {
    pub name: &'a str,
    pub guid: u64,
}

/// The party as `me` sees it: everyone else in `members`, `leader` leading.
pub fn group_list(others: &[GroupMember], leader: u64) -> Packet {
    let mut b = Body::new();
    b.u8(0) // party
        .u8(0) // my subgroup and flags
        .u32(others.len() as u32);
    for m in others {
        b.cstr(m.name).u64(m.guid).u8(1).u8(0);
    }
    b.u64(leader);
    if !others.is_empty() {
        b.u8(0) // free for all loot
            .u64(0)
            .u8(2) // threshold: uncommon
            .u8(0);
    }
    Packet::new(op::SMSG_GROUP_LIST, b)
}

/// An empty group list: not in a party.
pub fn group_none() -> Packet {
    group_list(&[], 0)
}

// --- spells -------------------------------------------------------------------------------------

/// What a spell is aimed at.
#[derive(Debug, Clone, Copy)]
pub enum Aim {
    /// No target: the caster itself.
    Caster,
    Unit(u64),
    Point([f32; 3]),
}

const CAST_FLAG_GO: u16 = 0x0100;
const CAST_FLAG_AMMO: u16 = 0x0020;
const TARGET_FLAG_UNIT: u16 = 0x0002;
const TARGET_FLAG_DEST_LOCATION: u16 = 0x0040;

/// The arrow an Auto Shot flies with.
pub const ARROW_DISPLAY: u32 = 2414;
/// A rifleman's bullet.
pub const BULLET_DISPLAY: u32 = 4233;

fn write_aim(b: &mut Body, aim: Aim) {
    match aim {
        Aim::Caster => {
            b.u16(0);
        }
        Aim::Unit(g) => {
            b.u16(TARGET_FLAG_UNIT).packed_guid(g);
        }
        Aim::Point(p) => {
            b.u16(TARGET_FLAG_DEST_LOCATION).vec3(p);
        }
    }
}

/// A cast begins: the caster plays its casting animation for `ms`.
pub fn spell_start(caster: u64, spell: u32, ms: u32, aim: Aim) -> Packet {
    spell_start_ammo(caster, spell, ms, aim, None)
}

/// A cast begins with ammunition: a ranged attack's start nocks this arrow (its display) for the
/// draw and hold.
pub fn spell_start_ammo(caster: u64, spell: u32, ms: u32, aim: Aim, ammo: Option<u32>) -> Packet {
    let mut b = Body::new();
    let flags = 0x2 | if ammo.is_some() { CAST_FLAG_AMMO } else { 0 };
    b.packed_guid(caster).packed_guid(caster).u32(spell).u16(flags).u32(ms);
    write_aim(&mut b, aim);
    if let Some(display) = ammo {
        b.u32(display).u32(24); // INVTYPE_AMMO
    }
    Packet::new(op::SMSG_SPELL_START, b)
}

/// A spell goes off: the client plays its missile and impacts.
pub fn spell_go(caster: u64, spell: u32, aim: Aim, hits: &[u64], ammo: Option<u32>) -> Packet {
    let mut b = Body::new();
    let flags = CAST_FLAG_GO | if ammo.is_some() { CAST_FLAG_AMMO } else { 0 };
    b.packed_guid(caster).packed_guid(caster).u32(spell).u16(flags);
    b.u8(hits.len().min(255) as u8);
    for h in hits.iter().take(255) {
        b.u64(*h);
    }
    b.u8(0); // misses
    write_aim(&mut b, aim);
    if let Some(display) = ammo {
        b.u32(display).u32(24); // INVTYPE_AMMO
    }
    Packet::new(op::SMSG_SPELL_GO, b)
}

/// `SMSG_SPELLLOGMISS`: a spell (an arrow) missed `victim`; `how` is 1 miss, 3 dodge, 4 parry,
/// 5 block. The client floats the word over them.
pub fn spell_log_miss(caster: u64, spell: u32, victim: u64, how: u8) -> Packet {
    let mut b = Body::new();
    b.u32(spell).u64(caster).u8(0).u32(1).u64(victim).u8(how);
    Packet::new(op::SMSG_SPELLLOGMISS, b)
}

/// Spell damage for the combat log and floating numbers.
pub fn spell_damage(victim: u64, attacker: u64, spell: u32, damage: u32, school: u8, crit: bool, periodic: bool) -> Packet {
    let mut b = Body::new();
    b.packed_guid(victim)
        .packed_guid(attacker)
        .u32(spell)
        .u32(damage)
        .u8(school)
        .u32(0) // absorbed
        .i32(0) // resisted
        .u8(periodic as u8)
        .u8(0)
        .u32(0) // blocked
        .u32(if crit { 0x2 } else { 0 })
        .u8(0);
    Packet::new(op::SMSG_SPELLNONMELEEDAMAGELOG, b)
}

pub fn spell_heal(target: u64, healer: u64, spell: u32, amount: u32, crit: bool) -> Packet {
    let mut b = Body::new();
    b.packed_guid(target).packed_guid(healer).u32(spell).u32(amount).u8(crit as u8);
    Packet::new(op::SMSG_SPELLHEALLOG, b)
}

/// A channel bar on the caster's own screen.
pub fn channel_start(spell: u32, ms: u32) -> Packet {
    let mut b = Body::new();
    b.u32(spell).u32(ms);
    Packet::new(op::MSG_CHANNEL_START, b)
}

pub fn channel_update(remaining_ms: u32) -> Packet {
    let mut b = Body::new();
    b.u32(remaining_ms);
    Packet::new(op::MSG_CHANNEL_UPDATE, b)
}

pub fn aura_duration(slot: u8, ms: i32) -> Packet {
    let mut b = Body::new();
    b.u8(slot).i32(ms);
    Packet::new(op::SMSG_UPDATE_AURA_DURATION, b)
}

// --- experience -----------------------------------------------------------------------------------

pub fn xp_gain(victim: Option<u64>, amount: u32) -> Packet {
    let mut b = Body::new();
    match victim {
        Some(v) => {
            b.u64(v).u32(amount).u8(0).u32(amount).f32(1.0);
        }
        None => {
            b.u64(0).u32(amount).u8(1);
        }
    }
    Packet::new(op::SMSG_LOG_XPGAIN, b)
}

pub fn level_up(level: u32, health: i32) -> Packet {
    let mut b = Body::new();
    b.u32(level).i32(health);
    for _ in 0..5 {
        b.i32(0);
    }
    for _ in 0..5 {
        b.i32(0);
    }
    Packet::new(op::SMSG_LEVELUP_INFO, b)
}

// --- movement -------------------------------------------------------------------------------------

/// Move a player somewhere else on the map: their client jumps there and answers.
pub fn teleport(guid: u64, counter: u32, pos: [f32; 3], facing: f32) -> Packet {
    let mut b = Body::new();
    b.packed_guid(guid).u32(counter).u32(0).u32(crate::clock_ms()).vec3(pos).f32(facing).u32(0);
    Packet::new(op::MSG_MOVE_TELEPORT_ACK, b)
}

/// What others see when a player is teleported: a heartbeat at the new spot.
pub fn heartbeat(guid: u64, pos: [f32; 3], facing: f32) -> Packet {
    let mut b = Body::new();
    b.packed_guid(guid).u32(0).u32(crate::clock_ms()).vec3(pos).f32(facing).u32(0);
    Packet::new(op::MSG_MOVE_HEARTBEAT, b)
}

/// Hold a player in place (they can still turn), or let them go.
pub fn force_move_root(guid: u64, counter: u32, root: bool) -> Packet {
    let mut b = Body::new();
    b.packed_guid(guid).u32(counter);
    Packet::new(if root { op::SMSG_FORCE_MOVE_ROOT } else { op::SMSG_FORCE_MOVE_UNROOT }, b)
}

pub fn force_run_speed(guid: u64, counter: u32, speed: f32) -> Packet {
    let mut b = Body::new();
    b.packed_guid(guid).u32(counter).f32(speed);
    Packet::new(op::SMSG_FORCE_RUN_SPEED_CHANGE, b)
}

pub fn force_run_back_speed(guid: u64, counter: u32, speed: f32) -> Packet {
    let mut b = Body::new();
    b.packed_guid(guid).u32(counter).f32(speed);
    Packet::new(op::SMSG_FORCE_RUN_BACK_SPEED_CHANGE, b)
}

/// `SMSG_SPLINE_SET_RUN_SPEED` / `_RUN_BACK_SPEED`: a unit's speed, for everyone else's view of
/// it (no ack).
pub fn spline_speed(guid: u64, back: bool, speed: f32) -> Packet {
    let mut b = Body::new();
    b.packed_guid(guid).f32(speed);
    Packet::new(if back { op::SMSG_SPLINE_SET_RUN_BACK_SPEED } else { op::SMSG_SPLINE_SET_RUN_SPEED }, b)
}

/// `SMSG_PLAY_OBJECT_SOUND`: a sound kit (`SoundEntries.dbc`) from a unit, where it stands.
pub fn play_object_sound(guid: u64, sound: u32) -> Packet {
    let mut b = Body::new();
    b.u32(sound).u64(guid);
    Packet::new(op::SMSG_PLAY_OBJECT_SOUND, b)
}

/// `SMSG_PLAY_SPELL_VISUAL`: a spell visual kit (an impact's look and sound) on a unit.
pub fn play_spell_visual(guid: u64, kit: u32) -> Packet {
    let mut b = Body::new();
    b.u64(guid).u32(kit);
    Packet::new(op::SMSG_PLAY_SPELL_VISUAL, b)
}

// --- dialogs -------------------------------------------------------------------------------------

/// A dialog window with options, from `source`.
pub fn gossip_message(source: u64, text_id: u32, options: &[(u8, &str)]) -> Packet {
    let mut b = Body::new();
    b.u64(source).u32(text_id).u32(options.len().min(15) as u32);
    for (i, (icon, text)) in options.iter().take(15).enumerate() {
        b.u32(i as u32).u8(*icon).u8(0).cstr(text);
    }
    b.u32(0); // quests
    Packet::new(op::SMSG_GOSSIP_MESSAGE, b)
}

pub fn gossip_complete() -> Packet {
    Packet::empty(op::SMSG_GOSSIP_COMPLETE)
}

/// The greeting text of a dialog.
pub fn npc_text(text_id: u32, text: &str) -> Packet {
    let mut b = Body::new();
    b.u32(text_id);
    for i in 0..8 {
        let (p, t) = if i == 0 { (1.0f32, text) } else { (0.0, "") };
        b.f32(p).cstr(t).cstr(t).u32(0);
        for _ in 0..3 {
            b.u32(0).u32(0);
        }
    }
    Packet::new(op::SMSG_NPC_TEXT_UPDATE, b)
}

// --- items ------------------------------------------------------------------------------------------

/// An item's template, as the tooltip shows it.
pub fn item_query_response(t: &htl_sim::items::ItemTemplate, class_mask: i32) -> Packet {
    item_query_response_stacked(t, class_mask, 1)
}

/// An item template whose items stack `stack` high.
pub fn item_query_response_stacked(t: &htl_sim::items::ItemTemplate, class_mask: i32, stack: u32) -> Packet {
    item_query_response_bag(t, class_mask, stack, 0)
}

/// An item template, a bag of `slots` when that's above 0.
pub fn item_query_response_bag(t: &htl_sim::items::ItemTemplate, class_mask: i32, stack: u32, slots: u32) -> Packet {
    let mut b = Body::new();
    b.u32(t.entry)
        .u32(t.item_class)
        .u32(t.subclass)
        .cstr(&t.name)
        .cstr("")
        .cstr("")
        .cstr("")
        .u32(t.display_id)
        .u32(t.quality as u32)
        .u32(0) // flags
        .u32(0) // buy price
        .u32(0) // sell price
        .u32(t.inventory_type)
        .i32(class_mask)
        .i32(-1) // any race
        .u32(t.item_level)
        .u32(t.required_level)
        .u32(0) // skill
        .u32(0)
        .u32(0) // spell
        .u32(0) // honor rank
        .u32(0) // city rank
        .u32(0) // reputation
        .u32(0)
        .u32(0) // max count
        .u32(stack.max(1)) // stackable
        .u32(slots); // container slots
    for i in 0..10 {
        let (k, v) = t.stats.get(i).copied().unwrap_or((0, 0));
        b.u32(k).i32(v);
    }
    let (lo, hi) = t.damage.unwrap_or((0.0, 0.0));
    b.f32(lo).f32(hi).u32(0);
    for _ in 0..4 {
        b.f32(0.0).f32(0.0).u32(0);
    }
    b.u32(t.armor);
    for _ in 0..6 {
        b.i32(0);
    }
    b.u32(t.delay_ms).u32(if t.inventory_type == 15 { 2 } else { 0 }).f32(0.0);
    for i in 0..5 {
        match t.use_spell.filter(|_| i == 0) {
            // On use, no charges.
            Some((spell, cooldown)) => b.u32(spell).u32(0).i32(0).i32(cooldown).u32(0).i32(-1),
            None => b.u32(0).u32(0).i32(0).i32(-1).u32(0).i32(-1),
        };
    }
    b.u32(1) // bind on pickup
        .cstr(&t.description)
        .u32(0) // page text
        .u32(0) // language
        .u32(0) // page material
        .u32(0) // start quest
        .u32(0) // lock
        .i32(t.material as i32)
        .u32(t.sheath)
        .u32(0) // random property
        .u32(0) // block
        .u32(0) // item set: ours is in the description
        .u32(0) // max durability
        .u32(0) // area
        .u32(0) // map
        .u32(0); // bag family
    Packet::new(op::SMSG_ITEM_QUERY_SINGLE_RESPONSE, b)
}

/// The verdict on a cast or item use: `failed` with a `SpellCastResult` reason, or fine.
/// Lift a cooldown the client is showing for `spell` on `guid`.
pub fn clear_cooldown(spell: u32, guid: u64) -> Packet {
    let mut b = Body::new();
    b.u32(spell).u64(guid);
    Packet::new(op::SMSG_CLEAR_COOLDOWN, b)
}

pub fn cast_result(spell: u32, failed: Option<u8>) -> Packet {
    let mut b = Body::new();
    b.u32(spell);
    match failed {
        Some(reason) => b.u8(2).u8(reason),
        None => b.u8(0),
    };
    Packet::new(op::SMSG_CAST_RESULT, b)
}

/// Someone else's cast was broken: their cast bar and animation stop.
pub fn spell_failed_other(caster: u64, spell: u32) -> Packet {
    let mut b = Body::new();
    b.u64(caster).u32(spell);
    Packet::new(op::SMSG_SPELL_FAILED_OTHER, b)
}

/// `SPELL_FAILED_INTERRUPTED`: the cast bar says "Interrupted".
pub const SPELL_FAILED_INTERRUPTED: u8 = 0x23;

pub fn item_query_miss(entry: u32) -> Packet {
    let mut b = Body::new();
    b.u32(entry | 0x8000_0000);
    Packet::new(op::SMSG_ITEM_QUERY_SINGLE_RESPONSE, b)
}

/// "You receive item: ..." for a new item in slot `slot` of the backpack (or worn).
pub fn item_push(player: u64, slot: u8, entry: u32, looted: bool) -> Packet {
    let mut b = Body::new();
    b.u64(player)
        .u32(if looted { 0 } else { 1 })
        .u32(0) // created
        .u32(1) // show in chat
        .u8(255) // the player's own inventory
        .u32(slot as u32)
        .u32(entry)
        .u32(0)
        .u32(0)
        .u32(1);
    Packet::new(op::SMSG_ITEM_PUSH_RESULT, b)
}

/// An inventory move refused. Reason 1 carries the level needed.
pub fn inventory_failure(reason: u8, required_level: Option<u32>, item: u64, item2: u64) -> Packet {
    let mut b = Body::new();
    b.u8(reason);
    if reason == 1 {
        b.u32(required_level.unwrap_or(0));
    }
    b.u64(item).u64(item2).u8(0);
    Packet::new(op::SMSG_INVENTORY_CHANGE_FAILURE, b)
}

/// What a corpse holds for this looter.
pub fn loot_response(corpse: u64, gold: u32, items: &[(u8, u32, u32)]) -> Packet {
    let mut b = Body::new();
    b.u64(corpse).u8(1).u32(gold).u8(items.len() as u8);
    for (slot, entry, display) in items {
        b.u8(*slot).u32(*entry).u32(1).u32(*display).u32(0).u32(0).u8(0);
    }
    Packet::new(op::SMSG_LOOT_RESPONSE, b)
}

pub fn loot_error(corpse: u64) -> Packet {
    let mut b = Body::new();
    b.u64(corpse).u8(0).u8(4); // locked
    Packet::new(op::SMSG_LOOT_RESPONSE, b)
}

pub fn loot_removed(slot: u8) -> Packet {
    let mut b = Body::new();
    b.u8(slot);
    Packet::new(op::SMSG_LOOT_REMOVED, b)
}

pub fn loot_money_notify(amount: u32) -> Packet {
    let mut b = Body::new();
    b.u32(amount);
    Packet::new(op::SMSG_LOOT_MONEY_NOTIFY, b)
}

pub fn loot_clear_money() -> Packet {
    Packet::empty(op::SMSG_LOOT_CLEAR_MONEY)
}

pub fn loot_release(corpse: u64) -> Packet {
    let mut b = Body::new();
    b.u64(corpse).u8(1);
    Packet::new(op::SMSG_LOOT_RELEASE_RESPONSE, b)
}
