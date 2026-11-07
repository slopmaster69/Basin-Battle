//! What Play and Host do: where things are, the project's names and its server (built in from
//! project.toml), the player's saved choices, finding WoW 1.12.1, the update from GitHub, this
//! PC's own server and starting the game. Nothing in here draws anything: Play's window is in
//! `bin/play.rs`, Host's console in `bin/host.rs`.
//!
//! Two kinds of folder (`folder::Kind`):
//! - the Windows download: Play.exe, Host.exe, benilla.exe, benilla-config\, server\, version.txt
//!   and the player's settings.txt side by side, and nothing else;
//! - a clone of this repository, which is how a Mac plays for now: Play then plays through
//!   scripts/play.sh and never updates (git does).

pub mod folder;
pub mod game;
pub mod net;
pub mod proc;
pub mod server;
pub mod update;
pub mod wow;
