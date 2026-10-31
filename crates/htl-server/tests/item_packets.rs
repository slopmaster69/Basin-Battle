//! Every gear piece's item packet reads back whole through the client's own parser, with the
//! display the template names: a piece that doesn't parse is drawn bare on the character.
use benilla_protocol::messages::ServerPacket;

// Each test is its own executable, so it needs the binary's _get_timezone shim on windows-gnu.
#[cfg(all(windows, target_env = "gnu"))]
#[path = "../src/mingw_tz.rs"]
mod mingw_tz;
use htl_sim::items::{template, GearKey, ARCHETYPES, MAX_TIER, WEAPON_PIECE};
use htl_sim::Content;

#[test]
fn every_gear_piece_reads_back_through_the_client_parser() {
    let content = Content::builtin();
    let mut bad = Vec::new();
    let mut n = 0;
    for class in 0..content.classes.len() {
        for tier in 0..=MAX_TIER {
            for piece in 0..=WEAPON_PIECE {
                for quality in 1..=5u8 {
                    for archetype in 0..ARCHETYPES {
                        let key = GearKey { class, tier, quality, piece, archetype };
                        let Some(t) = template(&content, key.entry()) else { continue };
                        n += 1;
                        let p = htl_server::packets::item_query_response(&t, -1);
                        match benilla_protocol::messages::parse_server(p.opcode, &p.body) {
                            Ok(ServerPacket::ItemQueryResponse { entry, info: Some(info) }) => {
                                if entry != t.entry || info.display_info_id != t.display_id || info.inventory_type != t.inventory_type {
                                    bad.push(format!("{} {}: read back {} / display {} / type {}", t.entry, t.name, entry, info.display_info_id, info.inventory_type));
                                }
                            }
                            other => bad.push(format!("{} {}: {:?}", t.entry, t.name, other.map(|p| p.name()))),
                        }
                    }
                }
            }
        }
    }
    // The casters' wands, too.
    for i in 0..htl_sim::items::WANDS.len() as u32 {
        let t = template(&content, htl_sim::items::STARTER_WAND + i).expect("a wand");
        let p = htl_server::packets::item_query_response(&t, -1);
        match benilla_protocol::messages::parse_server(p.opcode, &p.body) {
            Ok(ServerPacket::ItemQueryResponse { info: Some(info), .. }) => {
                assert_eq!((info.display_info_id, info.inventory_type), (t.display_id, 26), "{}", t.name);
            }
            other => panic!("the wand: {:?}", other.map(|p| p.name())),
        }
    }
    assert!(n > 100);
    assert!(bad.is_empty(), "{} of {n} pieces don't read back:\n{}", bad.len(), bad.iter().take(20).cloned().collect::<Vec<_>>().join("\n"));
}
