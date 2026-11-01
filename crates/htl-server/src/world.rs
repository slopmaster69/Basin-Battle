//! The world server's sockets: the encrypted handshake, then a reader that hands packets to the
//! game thread and a writer that sends what the game thread queues.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use crossbeam_channel::{unbounded, Sender};
use wow_srp::normalized_string::NormalizedString;
use wow_srp::vanilla_header::{DecrypterHalf, EncrypterHalf, ProofSeed};

use crate::packets::{self, Packet};
use crate::realmd::SessionKeys;
use crate::wire::Reader;

pub type ConnId = u64;

/// What the network side tells the game thread.
pub enum Inbound {
    Connected {
        conn: ConnId,
        account: String,
        out: Sender<Packet>,
    },
    Packet {
        conn: ConnId,
        opcode: u32,
        body: Vec<u8>,
    },
    Disconnected {
        conn: ConnId,
    },
}

static NEXT_CONN: AtomicU64 = AtomicU64::new(1);

pub fn spawn(listener: TcpListener, keys: SessionKeys, game: Sender<Inbound>) {
    std::thread::Builder::new()
        .name("world-accept".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let keys = keys.clone();
                let game = game.clone();
                std::thread::spawn(move || {
                    let peer = stream.peer_addr().ok();
                    if let Err(e) = serve(stream, &keys, &game) {
                        eprintln!("[world] {peer:?}: {e:#}");
                    }
                });
            }
        })
        .expect("spawn world accept thread");
}

/// The largest size a two-byte server header holds. The 1.12 client reads the first size byte's
/// top bit as "a third size byte follows", so a size of `0x8000` or more must be sent in three
/// bytes, or the client reads one byte too many and every packet after it is garbage (the game
/// stops answering anything, logging out included).
const SHORT_SIZE_MAX: usize = 0x7fff;
/// The largest size the three-byte form holds: the first byte's low seven bits and two more.
const LONG_SIZE_MAX: usize = 0x7f_ffff;

/// A server packet's header, in the clear: the size (the 2-byte opcode plus the body) big-endian
/// in two bytes up to [`SHORT_SIZE_MAX`], else in three with the first byte's top bit set, then
/// the opcode little-endian. `None` past [`LONG_SIZE_MAX`].
fn server_header(p: &Packet) -> Option<Vec<u8>> {
    let size = p.body.len() + 2;
    let mut header = if size <= SHORT_SIZE_MAX {
        (size as u16).to_be_bytes().to_vec()
    } else if size <= LONG_SIZE_MAX {
        (size as u32 | 0x80_0000).to_be_bytes()[1..].to_vec()
    } else {
        return None;
    };
    header.extend_from_slice(&p.opcode.to_le_bytes());
    Some(header)
}

/// Header and body as one buffer, the header passed through `encrypt` first; `None` (and a line
/// in the log) for a packet too big for any header.
fn frame(p: &Packet, encrypt: impl FnOnce(&mut [u8])) -> Option<Vec<u8>> {
    let Some(mut header) = server_header(p) else {
        eprintln!("[world] dropping an oversized {:#06x} packet ({} bytes)", p.opcode, p.body.len());
        return None;
    };
    encrypt(&mut header);
    let mut out = Vec::with_capacity(header.len() + p.body.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&p.body);
    Some(out)
}

fn write_plain(s: &mut TcpStream, p: &Packet) -> std::io::Result<()> {
    match frame(p, |_| {}) {
        Some(out) => s.write_all(&out),
        None => Ok(()),
    }
}

fn write_encrypted(s: &mut TcpStream, e: &mut EncrypterHalf, p: &Packet) -> std::io::Result<()> {
    // The cipher runs a byte at a time over the whole header, three size bytes or two.
    match frame(p, |h| e.encrypt(h)) {
        Some(out) => s.write_all(&out),
        None => Ok(()),
    }
}

fn serve(mut s: TcpStream, keys: &SessionKeys, game: &Sender<Inbound>) -> Result<()> {
    s.set_nodelay(true)?;
    s.set_read_timeout(Some(Duration::from_secs(30)))?;
    let seed = ProofSeed::new();
    write_plain(&mut s, &packets::auth_challenge(seed.seed()))?;

    // CMSG_AUTH_SESSION, unencrypted.
    let mut header = [0u8; 6];
    s.read_exact(&mut header)?;
    let size = u16::from_be_bytes([header[0], header[1]]) as usize;
    let opcode = u32::from_le_bytes([header[2], header[3], header[4], header[5]]);
    if opcode != packets::CMSG_AUTH_SESSION as u32 || !(4..=0x4000).contains(&size) {
        bail!("expected CMSG_AUTH_SESSION, got {opcode:#x}");
    }
    let mut body = vec![0u8; size - 4];
    s.read_exact(&mut body)?;
    let mut r = Reader::new(&body);
    let _build = r.u32()?;
    let _server_id = r.u32()?;
    let account = r.cstr()?.to_uppercase();
    let client_seed = r.u32()?;
    let digest = r.array::<20>()?;
    let key = keys
        .get(&account)
        .with_context(|| format!("{account} has not logged on"))?;
    let name = NormalizedString::new(&account).map_err(|e| anyhow::anyhow!("bad account name: {e}"))?;
    let crypto = seed
        .into_server_header_crypto(&name, key, digest, client_seed)
        .map_err(|_| anyhow::anyhow!("{account}: session proof mismatch"))?;
    let (mut enc, mut dec): (EncrypterHalf, DecrypterHalf) = crypto.split();
    write_encrypted(&mut s, &mut enc, &packets::auth_response_ok())?;

    // Writer: everything the game sends, in order.
    let conn = NEXT_CONN.fetch_add(1, Ordering::Relaxed);
    let (out_tx, out_rx) = unbounded::<Packet>();
    let mut ws = s.try_clone()?;
    std::thread::Builder::new()
        .name(format!("world-write-{conn}"))
        .spawn(move || {
            for p in out_rx {
                if write_encrypted(&mut ws, &mut enc, &p).is_err() {
                    break;
                }
            }
            let _ = ws.shutdown(std::net::Shutdown::Both);
        })?;
    game.send(Inbound::Connected {
        conn,
        account: account.clone(),
        out: out_tx,
    })?;
    println!("[world] {account} connected");

    // Reader: until the socket closes.
    s.set_read_timeout(Some(Duration::from_secs(120)))?;
    let result = (|| -> Result<()> {
        loop {
            let mut header = [0u8; 6];
            s.read_exact(&mut header)?;
            let h = dec.decrypt_client_header(header);
            let len = (h.size as usize)
                .checked_sub(4)
                .context("client header too short")?;
            let mut body = vec![0u8; len];
            s.read_exact(&mut body)?;
            game.send(Inbound::Packet {
                conn,
                opcode: h.opcode,
                body,
            })?;
        }
    })();
    let _ = game.send(Inbound::Disconnected { conn });
    println!("[world] {account} disconnected");
    match result {
        Err(e) if e
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::UnexpectedEof) =>
        {
            Ok(())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(body_len: usize) -> Packet {
        Packet { opcode: 0x0096, body: vec![0; body_len] }
    }

    /// The size the 1.12 client reads back from a header, as it reads it: two bytes while the
    /// first one's top bit is clear, else three; then the opcode.
    fn client_reads(h: &[u8]) -> (usize, u16, usize) {
        if h[0] & 0x80 == 0 {
            (usize::from(h[0]) << 8 | usize::from(h[1]), u16::from_le_bytes([h[2], h[3]]), 4)
        } else {
            let size = usize::from(h[0] & 0x7f) << 16 | usize::from(h[1]) << 8 | usize::from(h[2]);
            (size, u16::from_le_bytes([h[3], h[4]]), 5)
        }
    }

    #[test]
    fn a_header_reads_back_as_the_client_reads_it_on_both_sides_of_0x7fff() {
        for body in [0, 100, 0x7ffd, 0x7ffe, 0x8000, 34_200, 0xfffd, 0xfffe, 0x12343, LONG_SIZE_MAX - 2] {
            let h = server_header(&packet(body)).unwrap();
            let (size, opcode, len) = client_reads(&h);
            assert_eq!((size, opcode, len), (body + 2, 0x0096, h.len()), "a {body}-byte body");
        }
        // The largest two-byte size stays two bytes; one more takes three.
        assert_eq!(server_header(&packet(0x7ffd)).unwrap()[..2], [0x7f, 0xff]);
        assert_eq!(server_header(&packet(0x7ffe)).unwrap()[..3], [0x80, 0x80, 0x00]);
    }

    #[test]
    fn a_packet_too_big_for_any_header_is_dropped() {
        assert!(server_header(&packet(LONG_SIZE_MAX - 1)).is_none());
        assert!(frame(&packet(LONG_SIZE_MAX - 1), |_| {}).is_none());
    }
}
