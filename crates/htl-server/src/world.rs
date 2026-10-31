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

fn write_plain(s: &mut TcpStream, p: &Packet) -> std::io::Result<()> {
    let size = (p.body.len() + 2) as u16;
    let mut out = Vec::with_capacity(4 + p.body.len());
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(&p.opcode.to_le_bytes());
    out.extend_from_slice(&p.body);
    s.write_all(&out)
}

fn write_encrypted(s: &mut TcpStream, e: &mut EncrypterHalf, p: &Packet) -> std::io::Result<()> {
    if p.body.len() + 2 > u16::MAX as usize {
        eprintln!("[world] dropping an oversized {:#06x} packet", p.opcode);
        return Ok(());
    }
    let header = e.encrypt_server_header((p.body.len() + 2) as u16, p.opcode);
    let mut out = Vec::with_capacity(4 + p.body.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&p.body);
    s.write_all(&out)
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
