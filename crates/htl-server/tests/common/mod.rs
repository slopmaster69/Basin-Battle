//! A benilla protocol client for the integration tests: its logon, its world handshake, and its
//! packet parser and event decoder over everything the server sends. A packet benilla can't read,
//! or reads with bytes left over, fails the test.
#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use benilla_protocol::messages::{self, opcode as op, ServerPacket};
use benilla_srp::{HeaderCrypto, NormalizedString, ProofSeed};

// Each test is its own executable, so it needs the binary's _get_timezone shim on windows-gnu.
#[cfg(all(windows, target_env = "gnu"))]
#[path = "../../src/mingw_tz.rs"]
mod mingw_tz;

fn locale() -> u32 {
    u32::from_be_bytes(*b"enUS")
}

pub struct Client {
    pub s: TcpStream,
    pub crypto: HeaderCrypto,
    /// Every packet received, parsed.
    pub seen: Vec<ServerPacket>,
    /// The app events each decoded to, as text.
    pub events: Vec<String>,
    /// The account logged in as.
    pub user: String,
    /// Bytes read but not yet made into a packet, and the decrypted header of the packet they
    /// begin, if it's been read: a read that times out mid-packet keeps what it got, so the next
    /// one carries on from there instead of losing the stream's place.
    buf: Vec<u8>,
    head: Option<(usize, u16)>,
}

/// How much longer than its own figure a `wait` may take to see what it's waiting for. It costs
/// nothing when the packet comes in time, and a busy CI runner can be several times slower than
/// the machine a test's figure was picked on.
pub const SLACK: f32 = 3.0;


impl Client {
    /// Log on and into realm `realm`'s world server.
    pub fn connect(login: SocketAddr, user: &str, pass: &str, realm: &str) -> anyhow::Result<Self> {
        let logon = benilla_protocol::logon(&format!("127.0.0.1:{}", login.port()), user, pass, locale())?;
        let realm = logon.realms.iter().find(|r| r.name == realm).ok_or_else(|| anyhow::anyhow!("no realm {realm}"))?;
        let mut s = TcpStream::connect(&realm.address)?;
        s.set_read_timeout(Some(Duration::from_secs(5)))?;
        // SMSG_AUTH_CHALLENGE, in the clear.
        let mut head = [0u8; 4];
        s.read_exact(&mut head)?;
        assert_eq!(u16::from_le_bytes([head[2], head[3]]), op::SMSG_AUTH_CHALLENGE);
        let mut seed = [0u8; 4];
        s.read_exact(&mut seed)?;
        let server_seed = u32::from_le_bytes(seed);
        let proof_seed = ProofSeed::new();
        let client_seed = proof_seed.seed();
        let name = NormalizedString::new(user).unwrap();
        let (proof, crypto) = proof_seed.into_client_header_crypto(&name, logon.session_key, server_seed);
        let body = messages::auth_session(
            5875,
            &user.to_uppercase(),
            client_seed,
            &proof,
            &messages::addons::STOCK_SECURE_ADDONS,
        );
        let mut out = Vec::new();
        out.extend_from_slice(&((body.len() + 4) as u16).to_be_bytes());
        out.extend_from_slice(&(op::CMSG_AUTH_SESSION as u32).to_le_bytes());
        out.extend_from_slice(&body);
        s.write_all(&out)?;
        let mut c = Client {
            s,
            crypto,
            seen: Vec::new(),
            events: Vec::new(),
            user: user.to_string(),
            buf: Vec::new(),
            head: None,
        };
        match c.recv()? {
            ServerPacket::AuthResponse { result, .. } => assert_eq!(result, messages::AUTH_OK),
            other => panic!("expected AUTH_RESPONSE, got {}", other.name()),
        }
        Ok(c)
    }

    pub fn send(&mut self, opcode: u16, body: &[u8]) {
        let mut head = Vec::new();
        head.extend_from_slice(&((body.len() + 4) as u16).to_be_bytes());
        head.extend_from_slice(&(opcode as u32).to_le_bytes());
        self.crypto.encrypter().encrypt(&mut head);
        head.extend_from_slice(body);
        self.s.write_all(&head).unwrap();
    }

    /// Read until at least `n` bytes are buffered. A timeout keeps what arrived (unlike
    /// `read_exact`, which drops a partly read packet on the floor).
    fn fill(&mut self, n: usize) -> anyhow::Result<()> {
        let mut chunk = [0u8; 4096];
        while self.buf.len() < n {
            let got = self.s.read(&mut chunk)?;
            if got == 0 {
                anyhow::bail!("the server closed the connection");
            }
            self.buf.extend_from_slice(&chunk[..got]);
        }
        Ok(())
    }

    pub fn recv(&mut self) -> anyhow::Result<ServerPacket> {
        if self.head.is_none() {
            self.fill(4)?;
            let mut head = [0u8; 4];
            head.copy_from_slice(&self.buf[..4]);
            self.buf.drain(..4);
            // Decrypted once only: the header cipher's state moves on with every header.
            self.crypto.decrypter().decrypt(&mut head);
            let size = u16::from_be_bytes([head[0], head[1]]) as usize;
            let opcode = u16::from_le_bytes([head[2], head[3]]);
            self.head = Some((size, opcode));
        }
        let (size, opcode) = self.head.unwrap();
        self.fill(size - 2)?;
        let body: Vec<u8> = self.buf.drain(..size - 2).collect();
        self.head = None;
        let (packet, tail) = messages::parse_server_with_tail(opcode, &body)
            .unwrap_or_else(|e| panic!("benilla can't parse {opcode:#06x}: {e}"));
        assert_eq!(tail, 0, "{} left {tail} bytes unread", packet.name());
        // And the app's event decoder takes it.
        let again = messages::parse_server(opcode, &body).unwrap();
        self.events.push(format!("{:?}", benilla_protocol::decode(again)));
        let keep = messages::parse_server(opcode, &body).unwrap();
        self.seen.push(keep);
        Ok(packet)
    }

    /// Read until `want` matches, or `secs` (times `SLACK`) pass.
    pub fn wait(&mut self, secs: f32, want: impl FnMut(&ServerPacket) -> bool) -> Option<ServerPacket> {
        self.wait_exactly(secs * SLACK, want)
    }

    /// Read until `want` matches or exactly `secs` pass: for making sure something *doesn't*
    /// come, where a longer wait only slows the test.
    pub fn wait_exactly(&mut self, secs: f32, mut want: impl FnMut(&ServerPacket) -> bool) -> Option<ServerPacket> {
        let end = Instant::now() + Duration::from_secs_f32(secs);
        while Instant::now() < end {
            self.s.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
            match self.recv() {
                Ok(p) if want(&p) => return Some(p),
                Ok(_) => {}
                Err(e) if is_timeout(&e) => {}
                Err(e) => panic!("connection failed: {e:#}"),
            }
        }
        None
    }

    /// Read whatever arrives over `secs`.
    pub fn drain(&mut self, secs: f32) {
        let _ = self.wait_exactly(secs, |_| false);
    }

    /// Read until `done` holds for what's been seen, or `secs` (times `SLACK`) pass; whether it
    /// came true. Use it instead of a `drain` followed by a look at `seen`: it moves on the
    /// moment the server has said it, and a slow machine gets the time it needs.
    pub fn until(&mut self, secs: f32, mut done: impl FnMut(&Client) -> bool) -> bool {
        let end = Instant::now() + Duration::from_secs_f32(secs * SLACK);
        loop {
            if done(self) {
                return true;
            }
            if Instant::now() >= end {
                return false;
            }
            self.s.set_read_timeout(Some(Duration::from_millis(50))).unwrap();
            match self.recv() {
                Ok(_) => {}
                Err(e) if is_timeout(&e) => {}
                Err(e) => panic!("connection failed: {e:#}"),
            }
        }
    }

    /// The character list, then into the world as the character `pick` chooses; our guid.
    pub fn enter_with(&mut self, pick: impl Fn(&[messages::Character]) -> usize) -> u64 {
        self.send(op::CMSG_CHAR_ENUM, &[]);
        let chars = match self.wait(3.0, |p| matches!(p, ServerPacket::CharEnum { .. })) {
            Some(ServerPacket::CharEnum { characters }) => characters,
            _ => panic!("no character list"),
        };
        let guid = chars[pick(&chars)].guid;
        self.send(op::CMSG_PLAYER_LOGIN, &guid.to_le_bytes());
        let verify = self.wait(3.0, |p| matches!(p, ServerPacket::LoginVerifyWorld { .. }));
        assert!(verify.is_some(), "no SMSG_LOGIN_VERIFY_WORLD");
        let me = self.wait(3.0, |p| creates(p).iter().any(|(g, t)| *g == guid && *t == messages::ObjectType::Player));
        assert!(me.is_some(), "no create for our own player");
        guid
    }

    pub fn chat(&mut self, text: &str) {
        self.send(op::CMSG_MESSAGECHAT, &messages::messagechat(0, 7, text));
    }

    /// A whisper (to yourself): how the add-on sends commands while its player is dead.
    pub fn whisper(&mut self, to: &str, text: &str) {
        self.send(op::CMSG_MESSAGECHAT, &messages::messagechat_kind(6, 7, Some(to), text));
    }

    pub fn move_to(&mut self, pos: [f32; 3]) {
        let mut b = Vec::new();
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        for v in pos {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&0f32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        self.send(op::MSG_MOVE_HEARTBEAT, &b);
    }
}

fn is_timeout(e: &anyhow::Error) -> bool {
    e.downcast_ref::<std::io::Error>()
        .is_some_and(|io| matches!(io.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut))
}

pub fn creates(p: &ServerPacket) -> Vec<(u64, messages::ObjectType)> {
    match p {
        ServerPacket::UpdateObject { objects, .. } => objects
            .iter()
            .filter_map(|o| match o {
                messages::Object::Create { guid, object_type, .. } => Some((*guid, *object_type)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}


/// The add-on lines (`prefix\t...`) a client saw, without the prefix.
pub fn addon_lines(c: &Client, prefix: &str) -> Vec<String> {
    let p = format!("{prefix}\t");
    c.seen
        .iter()
        .filter_map(|x| match x {
            ServerPacket::MessageChat(m) if m.is_addon() => m.text.strip_prefix(p.as_str()).map(str::to_string),
            _ => None,
        })
        .collect()
}
