//! The login server (realmd): SRP6 logon against the shared server password, then the realm list.
//!
//! There are no accounts. Any account name logs in, as long as the password is the server's
//! password, so a host hands out one password and everyone picks their own name.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use wow_srp::normalized_string::NormalizedString;
use wow_srp::server::SrpVerifier;
use wow_srp::{PublicKey, GENERATOR, LARGE_SAFE_PRIME_LITTLE_ENDIAN, SESSION_KEY_LENGTH};

use crate::wire::Body;

const CMD_AUTH_LOGON_CHALLENGE: u8 = 0x00;
const CMD_AUTH_LOGON_PROOF: u8 = 0x01;
const CMD_AUTH_RECONNECT_CHALLENGE: u8 = 0x02;
const CMD_REALM_LIST: u8 = 0x10;

const RESULT_SUCCESS: u8 = 0x00;
/// What vmangos answers to a wrong password; the client shows "incorrect password".
const RESULT_FAIL_UNKNOWN_ACCOUNT: u8 = 0x04;
const RESULT_FAIL_VERSION_INVALID: u8 = 0x09;

/// The 1.12.1 client build.
pub const CLIENT_BUILD: u16 = 5875;

/// The constant salt every mangos-family realmd sends for the client-integrity hash. We don't
/// check the answer, but the client knows how to answer this one.
const VERSION_CHALLENGE: [u8; 16] = [
    0xba, 0xa3, 0x1e, 0x99, 0xa0, 0x0b, 0x21, 0x57, 0xfc, 0x37, 0x3f, 0xb3, 0x69, 0xcd, 0xd2, 0xf1,
];

/// Session keys of accounts that logged on, by uppercased account name, for the world server to
/// check its handshake against.
#[derive(Clone, Default)]
pub struct SessionKeys(Arc<Mutex<HashMap<String, [u8; SESSION_KEY_LENGTH as usize]>>>);

impl SessionKeys {
    pub fn insert(&self, account: &str, key: [u8; SESSION_KEY_LENGTH as usize]) {
        self.0
            .lock()
            .unwrap()
            .insert(account.to_uppercase(), key);
    }
    pub fn get(&self, account: &str) -> Option<[u8; SESSION_KEY_LENGTH as usize]> {
        self.0.lock().unwrap().get(&account.to_uppercase()).copied()
    }
}

/// One game on the realm list: its name and its world server's port.
#[derive(Clone)]
pub struct Realm {
    pub name: String,
    pub world_port: u16,
}

#[derive(Clone)]
pub struct RealmConfig {
    /// The games this server runs, as the realm list shows them (the launcher picks one by name).
    pub realms: Vec<Realm>,
    pub password: String,
    /// The address players on the internet use to reach this machine. Players on this machine or
    /// the same network are sent the address they connected to instead.
    pub public_host: Option<String>,
    /// The Bookmaker's Board, served over HTTP on this port (as the board host).
    pub board: Option<crate::survival::board::Shared>,
}

pub fn spawn(listener: TcpListener, config: RealmConfig, keys: SessionKeys) {
    std::thread::Builder::new()
        .name("realmd".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let config = config.clone();
                let keys = keys.clone();
                std::thread::spawn(move || {
                    let peer = stream.peer_addr().ok();
                    if let Err(e) = serve(stream, &config, &keys) {
                        eprintln!("[login] {peer:?}: {e:#}");
                    }
                });
            }
        })
        .expect("spawn realmd thread");
}

fn read_exact<const N: usize>(s: &mut TcpStream) -> Result<[u8; N]> {
    let mut b = [0u8; N];
    s.read_exact(&mut b)?;
    Ok(b)
}

fn serve(mut s: TcpStream, config: &RealmConfig, keys: &SessionKeys) -> Result<()> {
    // An HTTP request (another server's wagers for the board): no login command starts with a
    // letter.
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut first = [0u8; 4];
    let n = s.peek(&mut first)?;
    if n > 0 && matches!(first[0], b'G' | b'P') {
        if let (true, Some(b)) = (crate::survival::board::is_http(&first[..n]) || n < 4, &config.board) {
            crate::survival::board::Board::serve(b, s);
        }
        return Ok(());
    }
    s.set_read_timeout(Some(Duration::from_secs(300)))?;
    s.set_nodelay(true)?;
    let mut account: Option<String> = None;
    let mut proof_state = None;
    loop {
        let mut cmd = [0u8; 1];
        if s.read(&mut cmd)? == 0 {
            return Ok(());
        }
        match cmd[0] {
            CMD_AUTH_LOGON_CHALLENGE | CMD_AUTH_RECONNECT_CHALLENGE => {
                let _protocol = read_exact::<1>(&mut s)?[0];
                let size = u16::from_le_bytes(read_exact::<2>(&mut s)?) as usize;
                let mut body = vec![0u8; size];
                s.read_exact(&mut body)?;
                let mut r = crate::wire::Reader::new(&body);
                let _game = r.array::<4>()?;
                let _version = r.array::<3>()?;
                let build = r.u16()?;
                let _platform = r.array::<4>()?;
                let _os = r.array::<4>()?;
                let _locale = r.array::<4>()?;
                let _tz = r.u32()?;
                let _ip = r.array::<4>()?;
                let name_len = r.u8()? as usize;
                let name = String::from_utf8_lossy(r.take(name_len)?).to_uppercase();
                if cmd[0] == CMD_AUTH_RECONNECT_CHALLENGE {
                    // No reconnect support: refusing makes the client log on afresh.
                    s.write_all(&[CMD_AUTH_RECONNECT_CHALLENGE, RESULT_FAIL_UNKNOWN_ACCOUNT])?;
                    continue;
                }
                if build != CLIENT_BUILD {
                    s.write_all(&[CMD_AUTH_LOGON_CHALLENGE, 0, RESULT_FAIL_VERSION_INVALID])?;
                    continue;
                }
                let (Ok(user), Ok(pass)) = (
                    NormalizedString::new(&name),
                    NormalizedString::new(&config.password),
                ) else {
                    s.write_all(&[CMD_AUTH_LOGON_CHALLENGE, 0, RESULT_FAIL_UNKNOWN_ACCOUNT])?;
                    continue;
                };
                let proof = SrpVerifier::from_username_and_password(user, pass).into_proof();
                let mut reply = Body::new();
                reply
                    .u8(CMD_AUTH_LOGON_CHALLENGE)
                    .u8(0)
                    .u8(RESULT_SUCCESS)
                    .bytes(proof.server_public_key())
                    .u8(1)
                    .u8(GENERATOR)
                    .u8(32)
                    .bytes(&LARGE_SAFE_PRIME_LITTLE_ENDIAN)
                    .bytes(proof.salt())
                    .bytes(&VERSION_CHALLENGE)
                    .u8(0); // security flags: no PIN, no matrix card
                s.write_all(&reply.0)?;
                account = Some(name);
                proof_state = Some(proof);
            }
            CMD_AUTH_LOGON_PROOF => {
                let client_public = read_exact::<32>(&mut s)?;
                let client_proof = read_exact::<20>(&mut s)?;
                let _crc = read_exact::<20>(&mut s)?;
                let _keys = read_exact::<1>(&mut s)?;
                let _security = read_exact::<1>(&mut s)?;
                let (Some(proof), Some(name)) = (proof_state.take(), account.clone()) else {
                    bail!("logon proof before a challenge");
                };
                let Ok(client_public) = PublicKey::from_le_bytes(client_public) else {
                    s.write_all(&[CMD_AUTH_LOGON_PROOF, RESULT_FAIL_UNKNOWN_ACCOUNT, 3, 0])?;
                    continue;
                };
                match proof.into_server(client_public, client_proof) {
                    Ok((server, server_proof)) => {
                        keys.insert(&name, *server.session_key());
                        let mut reply = Body::new();
                        reply
                            .u8(CMD_AUTH_LOGON_PROOF)
                            .u8(RESULT_SUCCESS)
                            .bytes(&server_proof)
                            .u32(0); // hardware survey id: none
                        s.write_all(&reply.0)?;
                        println!("[login] {name} logged on");
                    }
                    Err(_) => {
                        println!("[login] {name}: wrong server password");
                        s.write_all(&[CMD_AUTH_LOGON_PROOF, RESULT_FAIL_UNKNOWN_ACCOUNT, 3, 0])?;
                    }
                }
            }
            CMD_REALM_LIST => {
                let _unused = read_exact::<4>(&mut s)?;
                if account.is_none() {
                    bail!("realm list before logon");
                }
                let host = world_host(&s, config)?;
                s.write_all(&realm_list(config, &host))?;
            }
            other => bail!("unknown login command {other:#04x}"),
        }
    }
}

/// The host this player should dial for a world: the public host for players arriving from the
/// internet, else the address of this machine they already reached.
fn world_host(s: &TcpStream, config: &RealmConfig) -> Result<String> {
    let peer = s.peer_addr().context("peer address")?;
    let local = s.local_addr().context("local address")?;
    let host = match (&config.public_host, is_local(&peer)) {
        (Some(public), false) => public.clone(),
        _ => local.ip().to_string(),
    };
    Ok(host)
}

fn is_local(addr: &SocketAddr) -> bool {
    match addr.ip() {
        // 100.64.0.0/10 is where Tailscale and other virtual networks put players.
        IpAddr::V4(ip) => {
            ip.is_loopback() || ip.is_private() || ip.is_link_local() || (ip.octets()[0] == 100 && (ip.octets()[1] & 0xC0) == 64)
        }
        IpAddr::V6(ip) => ip.is_loopback() || ip.to_ipv4_mapped().is_some_and(|v4| v4.is_private()),
    }
}

fn realm_list(config: &RealmConfig, host: &str) -> Vec<u8> {
    let mut realms = Body::new();
    realms.u32(0).u8(config.realms.len() as u8); // header padding, the count
    for (k, r) in config.realms.iter().enumerate() {
        realms
            .u32(0) // realm type: normal
            .u8(0) // flags
            .cstr(&r.name)
            .cstr(&format!("{host}:{}", r.world_port))
            .f32(0.0) // population: low
            .u8(0) // characters here: the list is made up on the spot
            .u8(1) // category
            .u8(k as u8 + 1); // realm id
    }
    realms.u16(0x0002); // footer
    let mut out = Body::new();
    out.u8(CMD_REALM_LIST).u16(realms.0.len() as u16).bytes(&realms.0);
    out.into_vec()
}
