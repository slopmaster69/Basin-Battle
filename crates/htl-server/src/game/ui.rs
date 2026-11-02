//! The Basin Battle interface add-on's side channel.
//!
//! The add-on (`addon/BasinBattle`, installed with the client) draws the card picker, the wave
//! tracker and the big banners. It says hello with the chat command `.bb hello <version>`; from
//! then on the server sends that player add-on lines (`SMSG_MESSAGECHAT` in `LANG_ADDON`, which the
//! client never prints) with the prefix `BB`, instead of NPC dialogs for card picks and raid
//! warnings for announcements. A client without the add-on never says hello and keeps the dialogs.
//!
//! Each line is a verb and fields joined by `~`:
//!
//! | line | meaning |
//! |---|---|
//! | `O~id~kind~header~title~subtitle~cards~reroll~rerollLabel~skip~skipLabel~banishes~lock~more~gold` | a card pick begins |
//! | `C~id~i~icon~rarity~name~rank~maxRank~kind~school~cooldown~text~upgrade~tags` | one card of it |
//! | `OS~id` | draw pick `id` now |
//! | `OX` | no pick is open |
//! | `P~n` | picks waiting |
//! | `B~icon^rank^rarity^name^aura icon^max rank^kind^school^cooldown^text^next;...` | the cards held |
//! | `K~name~rank~icon~rarity` | the player took a card |
//! | `H~wave~cycle~phase~secs~hub~hubMax~enemies~claimed~bases~best` | the wave tracker |
//! | `S~style~title~subtitle` | a banner |
//! | `A~text` | an announcement |
//! | `X~caster~ability~ms~danger~warning~boss~interrupt` | a boss or elite casts something to dodge, or a caster casts a spell; `danger` 1 if it's on you, `interrupt` 1 if you can break it (Shield Slam) |
//! | `XI~` | a cast was interrupted |
//! | `Q~manual~gcd~rage~key^icon^name^rank^rarity^cooldown^remaining^cost^talent^on^lit;...` | the spell bar, six keys at most (cards' spells and talents' alike); `rage` -1 for non-warriors; `lit` 1 when the spell's next cast is empowered |
//! | `PR~name~icon` | one of your upgrades or talents just went off: call it out |
//! | `TS~style` | how the class's talents come: `trees` (three trees) or `class` (class talents, combinations and weapon paths, and each skill's own upgrades) |
//! | `TT~i~name~text~icon~background` | talent tree `i` (0-2) of the player's class |
//! | `TD~i~tree~tier~col~ranks~icon~name~text1|text2|...~grants~kind~requires` | talent `i` and what each rank says; `kind` is `tree`, `class`, `combo`, `path` or `pathcard`, `requires` the skills a combination links (names joined by `+`) |
//! | `TR~points~spent0,spent1,spent2~rank,rank,...` | the player's talents |
//! | `UP~key^icon^name^count^cap^open^deck^upgrade,upgrade,...;...` | the player's skills and their upgrades (`upgrade` is `name:times:text`), in bar order |
//! | `E~text` | a cast didn't go off, and why |
//! | `DP~secs~total~source^damage;...` | your damage meter since you reset it (`.bb meter reset`) |
//! | `DM~wave~name^class^damage;...` | the party's damage this wave, most first |
//! | `MC~name~ms` | your own cast bar, for shots the client draws none for (Aimed Shot); `MC~` ends it |
//! | `T~kind^title^detail;...` | the objective tracker: what to do now |
//! | `SH~key^icon^name^group^count^cost^rarity^note^text;...` | the Ancient's shop; `cost` -1 when it can't be bought |
//! | `SO~site` | open the shop window at a site (the player right-clicked the Ancient or a post) |
//! | `SS~name^protectors^springs^defenders;...` | where defences can stand: the Ancient, then each post |
//! | `SB~key~name` | the player bought `key` |
//! | `SA~here~vendor` | whether the player is at home by the shopkeeper (`here` 1), where the shop opens |
//! | `FR~next~site^name^planned^plannedElites^notes^boss^incoming^incomingElites^bossIncoming^holders;...` | the fronts: the next wave's plan (`next` is its number between waves, else 0) and what's coming now |
//! | `PG~help~text` | a player pinged: holding a front, or (`help` 1) calling for help |
//! | `MP~kind^x^y;...` | the battlefield on the world map (while it's open, `.bb map 1`): `x`, `y` 0-1000 across and down; kinds `e` enemy, `E` elite, `B` boss, `t` treant, `s` spring, `d` defender, `p` post, `h` the Ancient, `g` a guarded base, `c` a claimed one, `o` a cleared one, `P` a player |
//!
//! From version 2 the add-on casts the player's spells (`.bb cast <key>`) instead of them firing
//! on their own; `.bb auto` switches back and forth. From version 3 it has a talent window:
//! `.bb talent <i>` spends a point, `.bb untalent` takes them all back (between waves). From
//! version 4 the Ancient's shop is a window of cards: `.bb buy <key>` buys one, at home by the
//! shopkeeper (`SA` says when you are); `.bb shop` asks for it. `.bb hold <site>` says you'll hold a front (again
//! to let go), `.bb help` calls for help where you stand. `.bb move <a> <b>` swaps two spells on
//! the bar.
//!
//! | `PZ~names` | the world is paused while these players choose a card (empty: it runs again) |
//!
//! `reroll` and `skip` are option indices (or -1); choosing card `i` is choosing option `i`.

use std::time::{Duration, Instant};

use htl_sim::{CardView, EntityId, Menu, MenuKind, OfferKind, OfferView, PhaseName, RerollCost, Status};

use super::*;

/// The add-on line prefix.
pub(crate) const PREFIX: &str = "BB";
/// The add-on version this server speaks to.
pub(crate) const VERSION: u32 = 4;
/// The wave tracker's hub bar updates at most this often.
const HUD_EVERY: Duration = Duration::from_millis(250);
/// The objective tracker is looked at this often.
const GUIDE_EVERY: Duration = Duration::from_millis(400);

/// The wave tracker as last sent.
#[derive(Default)]
pub(crate) struct Hud {
    sent: Option<(Status, Instant)>,
    pending: Option<Status>,
}

fn clean(s: &str) -> String {
    s.replace(['~', '\n', '\r'], " ")
}

fn rarity_index(r: htl_sim::content::Rarity) -> u8 {
    use htl_sim::content::Rarity::*;
    match r {
        Common => 0,
        Uncommon => 1,
        Rare => 2,
        Epic => 3,
        Legendary => 4,
    }
}

fn phase_key(p: PhaseName) -> &'static str {
    match p {
        PhaseName::Waiting => "waiting",
        PhaseName::Countdown => "countdown",
        PhaseName::Wave => "wave",
        PhaseName::Intermission => "intermission",
        PhaseName::Defeat => "defeat",
    }
}

/// The picker's header, title and subtitle for each kind of pick.
fn offer_heading(o: &OfferView) -> (String, String, String) {
    let more = "Rarer cards are stronger. Picking a spell you hold ranks it up.";
    match o.kind {
        OfferKind::Start => (
            "A NEW RUN".into(),
            "Choose Your First Spell".into(),
            "Cast it with key 1. Every pick after this shapes your build.".into(),
        ),
        OfferKind::HeadStart => ("HEAD START".into(), "Choose a Second Spell".into(), "A gift of your Renown.".into()),
        OfferKind::Level => (format!("LEVEL {}", o.level), "Level Up!".into(), more.into()),
        OfferKind::Wave => (format!("WAVE {} CLEARED", o.wave), "Spoils of Battle".into(), more.into()),
        OfferKind::Boss => ("BOSS SLAIN".into(), "Boss Defeated!".into(), "A boss's hoard holds rarer cards.".into()),
        OfferKind::Relic => ("RELIC CACHE".into(), "Relic Found!".into(), "Relics hold rare and epic cards.".into()),
        OfferKind::Legendary => (
            "ESCALATION".into(),
            "Choose a Legendary".into(),
            "Power to match the rising Scourge.".into(),
        ),
    }
}


/// How often the damage meters go out while they change.
const METER_EVERY: std::time::Duration = std::time::Duration::from_millis(1000);

impl Game {
    /// Send one add-on line to a connection.
    pub(crate) fn ui_line(&self, conn: ConnId, line: &str) {
        let Some(guid) = self.world_of(conn).map(|w| w.guid) else { return };
        let text = format!("{PREFIX}\t{line}");
        self.send(conn, packets::chat(packets::CHAT_PARTY, packets::LANG_ADDON, guid, &text));
    }

    pub(crate) fn has_addon(&self, conn: ConnId) -> bool {
        self.world_of(conn).is_some_and(|w| w.addon)
    }

    /// The connections running the add-on.
    fn addon_conns(&self) -> Vec<ConnId> {
        self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, _)| *c).collect()
    }

    fn icon(&self, spell: u32) -> String {
        self.icons.get(&spell).cloned().unwrap_or_else(|| "INV_Misc_QuestionMark".into())
    }

    /// A spell's icon texture name, for lines sent from elsewhere.
    pub(crate) fn spell_icon(&self, spell: u32) -> String {
        self.icon(spell)
    }

    /// `.bb <verb> ...` from the add-on.
    pub(crate) fn ui_command(&mut self, conn: ConnId, me: EntityId, words: &mut std::str::SplitWhitespace) {
        let verb = words.next().unwrap_or("");
        let arg = words.next().unwrap_or("");
        let n = arg.parse::<i64>().unwrap_or(-1);
        match verb {
            "hello" => {
                if let Some(w) = self.world_mut(conn) {
                    w.addon = true;
                    w.addon_version = n.max(1) as u32;
                    w.shop.clear();
                    w.at_shop = None;
                }
                println!("[game] {} runs the Basin Battle add-on (v{n})", self.world_of(conn).map_or("?", |w| &w.name));
                // From version 2 the add-on has a spell bar: spells are cast by hand.
                self.sim.set_manual(me, n >= 2);
                if let Some(w) = self.world_mut(conn) {
                    w.guide.clear();
                }
                self.ui_line(conn, &format!("V~{VERSION}"));
                // An add-on older than this server's: say so, and how to update.
                let account = self.sessions.get(&conn).map_or(String::new(), |s| s.account.clone());
                if (n.max(1) as u32) < VERSION && crate::first_told(format!("bb:{account}:{n}")) {
                    self.say_to(conn, &format!("Your Basin Battle add-on is older than this server's (v{}; the server has v{VERSION}): some screens may be missing. {}", n.max(1), crate::HOW_TO_UPDATE));
                }
                if n >= 3 {
                    self.ui_talent_defs(conn, me);
                } else {
                    // An add-on without the talent window: points spend themselves.
                    self.sim.auto_talents(me);
                }
                self.ui_resend(conn, me);
            }
            "pick" if n >= 0 => {
                if self.sim.menu(me).is_some_and(|m| m.offer.is_some()) {
                    self.sim.menu_select(me, n as usize);
                } else if self.sim.offers_waiting(me) > 0 {
                    self.sim.reopen_offers(me);
                }
            }
            "banish" if n >= 0 => {
                if !self.sim.banish_offered(me, n as usize) {
                    self.say_to(conn, "No banishes left.");
                }
            }
            "lock" if n >= 0 => {
                self.sim.lock_offered(me, n as usize);
            }
            "later" => {
                if self.sim.menu(me).is_some_and(|m| m.offer.is_some()) {
                    self.sim.menu_closed(me);
                }
                self.ui_line(conn, &format!("P~{}", self.sim.offers_waiting(me)));
            }
            // `.bb off`: back to the plain dialogs, if the add-on misbehaves.
            "off" => {
                if let Some(w) = self.world_mut(conn) {
                    w.addon = false;
                }
                self.say_to(conn, "Basin Battle interface off: picks come as dialogs again until you relog.");
                if self.sim.menu(me).is_some_and(|m| m.offer.is_some()) || self.sim.offers_waiting(me) > 0 {
                    self.sim.reopen_offers(me);
                }
            }
            "talent" if n >= 0 => {
                if let Err(e) = self.sim.learn_talent(me, n as usize) {
                    self.ui_line(conn, &format!("E~{e}"));
                }
            }
            "untalent" => match self.sim.reset_talents(me) {
                Ok(()) => self.ui_line(conn, "A~Your talent points are back: spend them again."),
                Err(e) => self.ui_line(conn, &format!("E~{e}")),
            },
            "cast" if n >= 1 => {
                if let Err(e) = self.sim.cast_slot(me, (n - 1) as usize) {
                    self.ui_line(conn, &format!("E~{}", e.text()));
                }
            }
            // `.bb auto`: spells fire on their own (again), or go back to casting by hand.
            "auto" => {
                let manual = self.sim.spell_bar(me).is_some_and(|b| b.manual);
                self.sim.set_manual(me, !manual);
                self.say_to(
                    conn,
                    if manual {
                        "Your spells now fire on their own. Type .bb auto to cast them yourself again."
                    } else {
                        "You cast your spells yourself again: 1, 2, 3, 4, Q and E."
                    },
                );
            }
            // `.bb move 1 4`: swap the spells on keys 1 and 4 (slots from 1).
            "move" if n >= 1 => {
                let to = words.next().and_then(|w| w.parse::<usize>().ok()).unwrap_or(0);
                if to >= 1 && !self.sim.move_spell(me, (n - 1) as usize, to - 1) {
                    self.ui_line(conn, "E~That spell can't move there.");
                }
            }
            // `.bb map 1` while the world map is open, `.bb map 0` when it closes.
            "map" => {
                if let Some(w) = self.world_mut(conn) {
                    w.map_open = n == 1;
                }
                if n == 1 {
                    let line = self.map_line();
                    self.ui_line(conn, &line);
                }
            }
            "hold" if n >= 0 => {
                if self.sim.ping(me, htl_sim::Ping::Hold(n as usize)).is_none() {
                    self.ui_line(conn, "E~No such place to hold.");
                }
            }
            "help" => {
                self.sim.ping(me, htl_sim::Ping::Help);
            }
            "shop" => {
                self.ui_shop(conn, me, true);
                if self.sim.at_shop(me) {
                    self.ui_line(conn, &format!("SO~{}", n.max(0)));
                } else {
                    let v = self.sim.vendor_name();
                    self.ui_line(conn, &format!("E~Shopping is done at home: visit {}.", clean(&v)));
                }
            }
            "buy" if !arg.is_empty() => match self.sim.buy(me, arg) {
                Ok(name) => {
                    self.ui_line(conn, &format!("SB~{}~{}", clean(arg), clean(&name)));
                    self.ui_shop(conn, me, true);
                }
                Err(e) => self.ui_line(conn, &format!("E~{}", clean(&e))),
            },
            "meter" if arg == "reset" => {
                self.sim.reset_meter(me);
                if let Some(w) = self.world_mut(conn) {
                    w.meters.2 = None;
                }
            }
            "open" => {
                if !self.sim.reopen_offers(me) {
                    self.ui_line(conn, "OX");
                    self.ui_line(conn, "P~0");
                }
            }
            _ => {}
        }
    }

    /// Everything the add-on draws, from scratch: after hello.
    fn ui_resend(&mut self, conn: ConnId, me: EntityId) {
        let status = self.sim.status();
        self.ui_hud_to(conn, &status);
        self.ui_build(conn, me);
        self.ui_bar(conn, me);
        self.ui_talents(conn, me);
        match self.sim.menu(me).cloned() {
            Some(m) if m.offer.is_some() => {
                // The dialog the server sent before hello gives way to the picker.
                self.send(conn, packets::gossip_complete());
                self.ui_offer(conn, me, &m);
            }
            _ => {
                if self.sim.offers_waiting(me) > 0 {
                    self.sim.reopen_offers(me);
                } else {
                    self.ui_line(conn, "OX");
                }
            }
        }
        self.ui_line(conn, &format!("P~{}", self.sim.offers_waiting(me)));
    }

    /// A menu event for an add-on player: picks go to the picker, anything else to a dialog.
    /// Returns false when the dialog should be sent as usual.
    pub(crate) fn ui_menu(&mut self, conn: ConnId, player: EntityId, menu: Option<&Menu>) -> bool {
        if !self.has_addon(conn) {
            return false;
        }
        match menu {
            Some(m) if m.offer.is_some() => {
                self.ui_offer(conn, player, m);
                self.ui_line(conn, &format!("P~{}", self.sim.offers_waiting(player)));
                true
            }
            Some(m) if m.kind == MenuKind::Forge && self.world_of(conn).is_some_and(|w| w.addon_version >= 4) => {
                // The shop window instead of the dialog.
                let site = self.sim.site_of(m.source);
                self.sim.menu_closed(player);
                self.send(conn, packets::gossip_complete());
                self.ui_shop(conn, player, true);
                self.ui_line(conn, &format!("SO~{site}"));
                true
            }
            Some(_) => {
                self.ui_line(conn, "OX");
                false
            }
            None => {
                self.ui_line(conn, "OX");
                self.ui_line(conn, &format!("P~{}", self.sim.offers_waiting(player)));
                false
            }
        }
    }

    fn ui_offer(&mut self, conn: ConnId, player: EntityId, m: &Menu) {
        let Some(o) = &m.offer else { return };
        self.offer_seq += 1;
        let id = self.offer_seq;
        let (header, title, subtitle) = offer_heading(o);
        let (reroll, reroll_label) = match o.reroll {
            Some((i, RerollCost::Free(n))) => (i as i64, format!("Reroll ({n})")),
            Some((i, RerollCost::Gold(g))) => (i as i64, format!("Reroll ({g}g)")),
            None => (-1, String::new()),
        };
        let (skip, skip_label) = match o.skip {
            Some((i, gold, left)) => (i as i64, format!("Skip +{gold}g ({left})")),
            None => (-1, String::new()),
        };
        let kind = match o.kind {
            OfferKind::Start => "start",
            OfferKind::HeadStart => "headstart",
            OfferKind::Level => "level",
            OfferKind::Wave => "wave",
            OfferKind::Boss => "boss",
            OfferKind::Relic => "relic",
            OfferKind::Legendary => "legendary",
        };
        self.ui_line(
            conn,
            &format!(
                "O~{id}~{kind}~{}~{}~{}~{}~{reroll}~{}~{skip}~{}~{}~{}~{}~{}",
                clean(&header),
                clean(&title),
                clean(&subtitle),
                o.cards.len(),
                clean(&reroll_label),
                clean(&skip_label),
                o.banishes,
                u8::from(o.can_lock),
                o.more,
                o.gold
            ),
        );
        for (i, c) in o.cards.iter().enumerate() {
            let line = self.card_line(id, i, c);
            self.ui_line(conn, &line);
        }
        self.ui_build(conn, player);
        self.ui_line(conn, &format!("OS~{id}"));
    }

    fn card_line(&self, id: u32, i: usize, c: &CardView) -> String {
        format!(
            "C~{id}~{i}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}",
            self.icon(c.icon_spell),
            rarity_index(c.rarity),
            clean(&c.name),
            c.rank,
            c.max_rank,
            c.text.kind,
            c.text.school,
            htl_sim::describe::num(c.text.cooldown),
            clean(&c.text.text),
            clean(&c.text.upgrade),
            clean(&c.tags.join(",")),
            clean(&c.slot)
        )
    }

    /// The spells row: what the player holds.
    pub(crate) fn ui_build(&self, conn: ConnId, player: EntityId) {
        let held: Vec<String> = self
            .sim
            .held_cards(player)
            .iter()
            .map(|h| {
                let f = |s: &str| clean(s).replace([';', '^'], " ");
                format!(
                    "{}^{}^{}^{}^{}^{}^{}^{}^{}^{}^{}",
                    self.icon(h.icon_spell),
                    h.rank,
                    rarity_index(h.rarity),
                    f(&h.name),
                    self.icon(h.aura_spell),
                    h.max_rank,
                    h.text.kind,
                    h.text.school,
                    htl_sim::describe::num(h.text.cooldown),
                    f(&h.text.text),
                    f(&h.next)
                )
            })
            .collect();
        self.ui_line(conn, &format!("B~{}", held.join(";")));
    }

    /// The spell bar: what's on keys 1-6, cooldowns, rage.
    pub(crate) fn ui_bar(&self, conn: ConnId, player: EntityId) {
        let Some(bar) = self.sim.spell_bar(player) else { return };
        let f = |s: &str| clean(s).replace([';', '^'], " ");
        let slots: Vec<String> = bar
            .slots
            .iter()
            .map(|s| {
                format!(
                    "{}^{}^{}^{}^{}^{:.1}^{:.1}^{}^{}^{}^{}",
                    f(&s.key),
                    self.icon(s.icon_spell),
                    f(&s.name),
                    s.rank,
                    rarity_index(s.rarity),
                    s.cooldown,
                    s.remaining,
                    s.cost,
                    u8::from(s.talent),
                    u8::from(s.on),
                    u8::from(s.lit)
                )
            })
            .collect();
        let rage = bar.rage.map_or(-1, |r| r as i64);
        self.ui_line(conn, &format!("Q~{}~{:.2}~{rage}~{}", u8::from(bar.manual), bar.gcd, slots.join(";")));
    }

    /// The class's talent trees and talents, once (they never change).
    pub(crate) fn ui_talent_defs(&self, conn: ConnId, player: EntityId) {
        let Some(class) = self.sim.player_view(player).map(|v| v.class) else { return };
        let kit = self.sim.content().kit(class);
        self.ui_line(conn, &format!("TS~{}", if kit.class_talents { "class" } else { "trees" }));
        for (i, t) in kit.trees.iter().enumerate() {
            self.ui_line(conn, &format!("TT~{i}~{}~{}~{}~{}", clean(&t.name), clean(&t.text), self.icon(t.icon), clean(&t.background)));
        }
        for (i, t) in kit.talents.iter().enumerate() {
            let texts: Vec<String> = (1..=t.ranks).map(|r| clean(&t.text_at(r)).replace('|', "/")).collect();
            let kind = if !kit.class_talents {
                "tree"
            } else if t.path.is_some() {
                "path"
            } else if t.path_only.is_some() {
                "pathcard"
            } else if t.requires.len() >= 2 {
                "combo"
            } else {
                "class"
            };
            let requires: Vec<String> = t.requires.iter().map(|&r| clean(&kit.spells[r].name).replace('+', " ")).collect();
            self.ui_line(
                conn,
                &format!(
                    "TD~{i}~{}~{}~{}~{}~{}~{}~{}~{}~{kind}~{}",
                    t.tree,
                    t.tier,
                    t.col,
                    t.ranks,
                    self.icon(t.icon),
                    clean(&t.name),
                    texts.join("|"),
                    u8::from(t.grants.is_some()),
                    requires.join("+")
                ),
            );
        }
    }

    /// The player's talents: points to spend, points in each tree, ranks.
    pub(crate) fn ui_talents(&self, conn: ConnId, player: EntityId) {
        let points = self.sim.talent_points(player);
        let spent = self.sim.tree_points(player);
        let ranks: Vec<String> = self.sim.talent_ranks(player).iter().map(|r| r.to_string()).collect();
        self.ui_line(conn, &format!("TR~{points}~{},{},{}~{}", spent[0], spent[1], spent[2], ranks.join(",")));
        // Each skill's upgrades, for the build window.
        let f = |s: &str| clean(s).replace([';', '^', ',', ':'], " ");
        let skills: Vec<String> = self
            .sim
            .build_view(player)
            .iter()
            .map(|b| {
                let ups: Vec<String> = b.taken.iter().map(|(n, t, x)| format!("{}:{x}:{}", f(n), f(t))).collect();
                format!("{}^{}^{}^{}^{}^{}^{}^{}", f(&b.key), self.icon(b.icon_spell), f(&b.name), b.count, b.cap, b.open, b.deck, ups.join(","))
            })
            .collect();
        self.ui_line(conn, &format!("UP~{}", skills.join(";")));
    }

    /// Everything worth a dot on the world map.
    fn map_line(&self) -> String {
        let arena = self.sim.arena();
        let mut out = Vec::new();
        let hub = self.sim.hub_id();
        for e in self.sim.entities() {
            if !e.alive {
                continue;
            }
            let kind = match &e.what {
                htl_sim::What::Hub => "h",
                htl_sim::What::Enemy { kind, .. } if kind.boss => "B",
                htl_sim::What::Enemy { elite: true, .. } => "E",
                htl_sim::What::Enemy { .. } => "e",
                htl_sim::What::Npc { npc: NpcKind::Protector, .. } => "t",
                htl_sim::What::Npc { npc: NpcKind::Spring, .. } => "s",
                htl_sim::What::Npc { npc: NpcKind::Post(_), .. } => "p",
                htl_sim::What::Npc { npc: NpcKind::Banner { claimed: true }, .. } => "c",
                htl_sim::What::Npc { npc: NpcKind::Banner { claimed: false }, base } => {
                    if self.sim.base_state(*base) == Some(htl_sim::BaseState::Cleared) {
                        "o"
                    } else {
                        "g"
                    }
                }
                htl_sim::What::Ally { owner, pet: false } if *owner == hub => "d",
                htl_sim::What::Player { .. } => "P",
                _ => continue,
            };
            if let Some((x, y)) = arena.map_xy(e.pos) {
                out.push(format!("{kind}^{}^{}", (x * 1000.0) as u32, (y * 1000.0) as u32));
            }
        }
        // Calls for help, for ten seconds.
        for (at, pos) in &self.pings {
            if at.elapsed().as_secs() < 10 {
                if let Some((x, y)) = arena.map_xy(*pos) {
                    out.push(format!("!^{}^{}", (x * 1000.0) as u32, (y * 1000.0) as u32));
                }
            }
        }
        format!("MP~{}", out.join(";"))
    }

    /// The fronts tracker line, the same for everyone.
    fn fronts_line(&self) -> String {
        let f = |s: &str| clean(s).replace([';', '^'], " ");
        let st = self.sim.status();
        let next = if matches!(st.phase, PhaseName::Countdown | PhaseName::Intermission) { st.wave + 1 } else { 0 };
        let rows: Vec<String> = self
            .sim
            .fronts()
            .iter()
            .map(|r| {
                format!(
                    "{}^{}^{}^{}^{}^{}^{}^{}^{}^{}",
                    r.site,
                    f(&r.name),
                    r.planned,
                    r.planned_elites,
                    f(&r.notes.join(", ")),
                    f(r.boss.as_deref().unwrap_or("")),
                    r.incoming,
                    r.incoming_elites,
                    u8::from(r.boss_incoming),
                    f(&r.holders.join(", "))
                )
            })
            .collect();
        format!("FR~{next}~{}~{}", rows.join(";"), self.sim.wave_event_name().unwrap_or(""))
    }

    /// The Ancient's shop for one player, when it changed (or `force`).
    pub(crate) fn ui_shop(&mut self, conn: ConnId, me: EntityId, force: bool) {
        let f = |s: &str| clean(s).replace([';', '^'], " ");
        let items: Vec<String> = self
            .sim
            .shop(me)
            .iter()
            .map(|i| {
                format!(
                    "{}^{}^{}^{}^{}^{}^{}^{}^{}",
                    f(&i.key),
                    if i.icon_name.is_empty() { self.icon(i.icon_spell) } else { i.icon_name.clone() },
                    f(&i.name),
                    f(i.group),
                    f(&i.count),
                    i.cost.map_or(-1, |c| c as i64),
                    i.rarity,
                    f(&i.note),
                    f(&i.text)
                )
            })
            .collect();
        let sites: Vec<String> = self
            .sim
            .shop_sites()
            .iter()
            .map(|s| format!("{}^{}^{}^{}", f(&s.name), s.protectors, s.springs, s.defenders))
            .collect();
        let line = format!("SS~{}\u{1}SH~{}", sites.join(";"), items.join(";"));
        let changed = self.world_of(conn).is_some_and(|w| w.shop != line);
        if changed || force {
            // The sites first, so the cards know where they'd go.
            for part in line.split('\u{1}') {
                self.ui_line(conn, part);
            }
            if let Some(w) = self.world_mut(conn) {
                w.shop = line;
            }
        }
    }

    /// The objective tracker, for every add-on player whose tracker changed.
    pub(crate) fn ui_guides(&mut self) {
        let due = self.guide_at.map_or(true, |t| t.elapsed() >= GUIDE_EVERY);
        if !due {
            return;
        }
        self.guide_at = Some(Instant::now());
        let players: Vec<(ConnId, EntityId)> = self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, w)| (*c, w.entity)).collect();
        let fronts = self.fronts_line();
        let boss = self.boss_line();
        let watching: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon && w.map_open).map(|(c, _, _)| *c).collect();
        if !watching.is_empty() {
            let line = self.map_line();
            for c in watching {
                self.ui_line(c, &line);
            }
        }
        for &(conn, me) in &players {
            if self.world_of(conn).is_some_and(|w| w.addon_version >= 4) {
                self.ui_shop(conn, me, false);
                let here = self.sim.at_shop(me);
                if self.world_of(conn).is_some_and(|w| w.at_shop != Some(here)) {
                    let v = self.sim.vendor_name();
                    self.ui_line(conn, &format!("SA~{}~{}", u8::from(here), clean(&v)));
                    if let Some(w) = self.world_mut(conn) {
                        w.at_shop = Some(here);
                    }
                }
                if self.world_of(conn).is_some_and(|w| w.fronts != fronts) {
                    self.ui_line(conn, &fronts);
                    if let Some(w) = self.world_mut(conn) {
                        w.fronts = fronts.clone();
                    }
                }
                if self.world_of(conn).is_some_and(|w| w.boss != boss) {
                    self.ui_line(conn, &boss);
                    if let Some(w) = self.world_mut(conn) {
                        w.boss = boss.clone();
                    }
                }
            }
        }
        for (conn, me) in players {
            let f = |s: &str| clean(s).replace([';', '^'], " ");
            let text = self
                .sim
                .guidance(me)
                .iter()
                .map(|o| format!("{}^{}^{}", o.kind, f(&o.title), f(&o.detail)))
                .collect::<Vec<_>>()
                .join(";");
            let changed = self.world_of(conn).is_some_and(|w| w.guide != text);
            if changed {
                self.ui_line(conn, &format!("T~{text}"));
                if let Some(w) = self.world_mut(conn) {
                    w.guide = text;
                }
            }
        }
    }

    /// The boss bar: `BH~name~health in thousandths`, or `BH~` with no boss in the field.
    fn boss_line(&self) -> String {
        self.sim
            .entities()
            .into_iter()
            .find_map(|v| match v.what {
                htl_sim::What::Enemy { kind, .. } if kind.boss && v.alive => {
                    let name = clean(&self.sim.enemy_def(kind).name);
                    Some(format!("BH~{name}~{}", v.health as u64 * 1000 / v.max_health.max(1) as u64))
                }
                _ => None,
            })
            .unwrap_or_else(|| "BH~".to_string())
    }

    pub(crate) fn ui_card_taken(&self, player: EntityId, key: &str, rank: u8) {
        let Some(conn) = self.conn_of(player) else { return };
        if !self.has_addon(conn) {
            return;
        }
        if let Some(def) = self.sim.content().cards.iter().find(|c| c.key == key) {
            self.ui_line(
                conn,
                &format!("K~{}~{rank}~{}~{}", clean(&def.name), self.icon(def.spell), rarity_index(def.rarity)),
            );
        }
        self.ui_build(conn, player);
    }

    /// An enemy casts something to dodge: add-on players get a cast bar for bosses and elites. No
    /// warning text: the spell and the marks on the ground are the warning.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn ui_enemy_cast(&self, caster: EntityId, name: &str, ms: u32, warn: &str, endangered: &[EntityId], big: bool, interruptible: bool) {
        let who = self
            .sim
            .entities()
            .into_iter()
            .find(|e| e.id == caster)
            .and_then(|e| match e.what {
                htl_sim::What::Enemy { kind, elite } => Some((self.sim.enemy_def(kind).name.clone(), kind.boss, elite)),
                _ => None,
            });
        let Some((who, boss, elite)) = who else { return };
        if !(boss || elite || big || interruptible) {
            return;
        }
        for (c, _, w) in self.in_world() {
            if w.addon {
                let danger = endangered.contains(&w.entity);
                // Who can break it is told so; an ordinary caster's spell is shown only to them and
                // to whoever it's aimed at (no nagging the rest).
                let can = interruptible && self.sim.can_interrupt(w.entity, caster);
                if !(boss || elite) && !danger && !can {
                    continue;
                }
                self.ui_line(*c, &format!("X~{}~{}~{ms}~{}~{}~{}~{}", clean(&who), clean(name), u8::from(danger), clean(warn), u8::from(boss || elite), u8::from(can)));
            }
        }
    }

    pub(crate) fn ui_splash(&self, style: &str, title: &str, subtitle: &str) {
        for c in self.addon_conns() {
            self.ui_line(c, &format!("S~{style}~{}~{}", clean(title), clean(subtitle)));
        }
    }

    /// A big announcement: add-on players get it as a line of their own; everyone else as a raid
    /// warning. Returns the connections that still need the raid warning.
    pub(crate) fn ui_announce(&self, text: &str) -> Vec<ConnId> {
        let mut rest = Vec::new();
        for (c, _, w) in self.in_world() {
            if w.addon {
                self.ui_line(*c, &format!("A~{}", clean(text)));
            } else {
                rest.push(*c);
            }
        }
        rest
    }

    fn ui_hud_to(&self, conn: ConnId, s: &Status) {
        let best = self.saves.best_depth();
        self.ui_line(
            conn,
            &format!(
                "H~{}~{}~{}~{}~{}~{}~{}~{}~{}~{best}",
                s.wave,
                s.cycle,
                phase_key(s.phase),
                s.seconds_left,
                s.hub_health,
                s.hub_max_health,
                s.enemies_left,
                s.bases_claimed,
                s.bases
            ),
        );
    }

    /// A new status from the sim: the wave tracker follows, the hub bar a few times a second.
    pub(crate) fn ui_status(&mut self, s: Status) {
        let urgent = match &self.hud.sent {
            None => true,
            Some((last, _)) => Status { hub_health: last.hub_health, hub_max_health: last.hub_max_health, ..s } != *last,
        };
        self.hud.pending = Some(s);
        if urgent {
            self.ui_flush_hud(true);
        }
    }

    /// Send a held-back status once enough time has passed (called every tick).
    /// The damage meters, about once a second while they change: your own since you reset it
    /// (`DP~secs~total~source^damage;...`) and the party's this wave (`DM~wave~name^class^damage;...`).
    pub(crate) fn ui_meters(&mut self, now: bool) {
        let f = |s: &str| clean(s).replace([';', '^'], " ");
        let party: Vec<String> = self.sim.party_meter().iter().map(|r| format!("{}^{}^{}", f(&r.name), r.class, r.damage)).collect();
        let party = format!("DM~{}~{}", self.sim.status().wave, party.join(";"));
        let conns: Vec<(ConnId, EntityId)> = self.in_world().filter(|(_, _, w)| w.addon && w.addon_version >= 4).map(|(c, _, w)| (*c, w.entity)).collect();
        for (c, me) in conns {
            let Some(m) = self.sim.meter(me) else { continue };
            let rows: Vec<String> = m.rows.iter().take(8).map(|(l, n)| format!("{}^{n}", f(l))).collect();
            let own = format!("DP~{:.0}~{}~{}", m.secs, m.total, rows.join(";"));
            let Some(w) = self.world_mut(c) else { continue };
            let due = now || w.meters.2.map_or(true, |at| at.elapsed() >= METER_EVERY);
            if !due || (w.meters.0 == own && w.meters.1 == party) {
                continue;
            }
            let send_own = w.meters.0 != own;
            let send_party = w.meters.1 != party;
            w.meters = (own.clone(), party.clone(), Some(Instant::now()));
            if send_own {
                self.ui_line(c, &own);
            }
            if send_party {
                self.ui_line(c, &party);
            }
        }
    }

    pub(crate) fn ui_flush_hud(&mut self, now: bool) {
        let Some(s) = self.hud.pending else { return };
        let due = now || self.hud.sent.as_ref().map_or(true, |(_, at)| at.elapsed() >= HUD_EVERY);
        if !due {
            return;
        }
        for c in self.addon_conns() {
            self.ui_hud_to(c, &s);
        }
        self.hud.sent = Some((s, Instant::now()));
        self.hud.pending = None;
    }
}

#[cfg(test)]
mod version_tests {
    /// The add-on's own `VERSION`, read from its source: the server's must keep up with it.
    #[test]
    fn the_server_knows_the_add_ons_version() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../addon/BasinBattle/BasinBattle.lua")).unwrap();
        let version: u32 = src.lines().find_map(|l| l.strip_prefix("local VERSION = ")).and_then(|v| v.trim().parse().ok()).expect("local VERSION = N");
        assert_eq!(version, super::VERSION);
    }
}
