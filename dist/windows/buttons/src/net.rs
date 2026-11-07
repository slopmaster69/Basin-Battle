//! Addresses, whether a server answers, and GitHub's releases.

use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// "host:port", or "host" (port 3724, the login port).
pub fn split_address(address: &str) -> (String, u16) {
    if let Some((h, p)) = address.rsplit_once(':') {
        if !h.is_empty() && !h.contains(':') && !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(p) = p.parse() {
                return (h.into(), p);
            }
        }
    }
    (address.into(), 3724)
}

/// Whether something answers on host:port within `ms`.
pub fn reach(host: &str, port: u16, ms: u64) -> bool {
    let end = Instant::now() + Duration::from_millis(ms);
    let Ok(addrs) = (host, port).to_socket_addrs() else { return false };
    for a in addrs {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        if TcpStream::connect_timeout(&a, left).is_ok() {
            return true;
        }
    }
    false
}

/// This PC's address on the home network: the one a packet to the internet would leave from
/// (nothing is sent).
pub fn lan_address() -> Option<String> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

/// This PC's address on the internet, asked of ipify (4 s), or nothing.
pub fn public_address() -> Option<String> {
    let text = agent(4).get("https://api.ipify.org").call().ok()?.into_string().ok()?;
    let t = text.trim().to_string();
    t.chars().all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':').then_some(t)
}

/// This PC's Tailscale address, if Tailscale is installed and up.
pub fn tailscale_address() -> Option<String> {
    #[cfg(windows)]
    let exe = std::env::var_os("ProgramFiles")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Program Files"))
        .join("Tailscale")
        .join("tailscale.exe");
    #[cfg(not(windows))]
    let exe = std::path::PathBuf::from("/Applications/Tailscale.app/Contents/MacOS/Tailscale");
    if !exe.is_file() {
        return None;
    }
    let mut c = std::process::Command::new(exe);
    c.args(["ip", "-4"]);
    crate::proc::quiet(&mut c);
    let out = c.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().map(str::trim).find(|l| l.split('.').count() == 4 && l.chars().all(|c| c.is_ascii_digit() || c == '.')).map(String::from)
}

/// An HTTP agent over the system's own TLS, with a timeout on everything.
pub fn agent(secs: u64) -> ureq::Agent {
    let tls = native_tls::TlsConnector::new().expect("the system's TLS");
    ureq::AgentBuilder::new()
        .tls_connector(Arc::new(tls))
        .timeout_connect(Duration::from_secs(secs.min(8)))
        .timeout(Duration::from_secs(secs))
        .user_agent(concat!("Elwynn-Survival-launcher/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// A release on GitHub with its Windows download.
#[derive(Clone, Debug)]
pub struct Release {
    /// "v0.5.0", or "latest 1a2b3c4" for the playtest channel (the commit from the release's name).
    pub version: String,
    pub asset_url: String,
    pub asset_name: String,
    pub asset_size: u64,
}

/// The newest release of `repository`: the latest proper release, or the `latest` pre-release
/// that main rebuilds at every merge. `Err` says why it couldn't be asked, in a player's words.
pub fn latest_release(repository: &str, channel: crate::folder::Channel) -> Result<Option<Release>, String> {
    use crate::folder::Channel;
    let url = match channel {
        Channel::Release => format!("https://api.github.com/repos/{repository}/releases/latest"),
        Channel::Latest => format!("https://api.github.com/repos/{repository}/releases/tags/latest"),
        Channel::Fixed => return Ok(None),
    };
    let resp = agent(10).get(&url).set("Accept", "application/vnd.github+json").call();
    let json: serde_json::Value = match resp {
        Ok(r) => r.into_json().map_err(|e| format!("GitHub's answer couldn't be read ({e})."))?,
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(ureq::Error::Status(code, _)) => return Err(format!("GitHub answered {code}. Trying again later.")),
        Err(_) => return Err("Couldn't reach GitHub to check for a new version (offline?).".into()),
    };
    let tag = json["tag_name"].as_str().unwrap_or("").to_string();
    let version = match channel {
        Channel::Latest => {
            // The release is named "Latest main (1a2b3c4)".
            let name = json["name"].as_str().unwrap_or("");
            let sha = name.rsplit('(').next().and_then(|s| s.split(')').next()).unwrap_or("").trim();
            format!("latest {}", if sha.is_empty() { "none" } else { sha })
        }
        _ => tag,
    };
    let assets = json["assets"].as_array().cloned().unwrap_or_default();
    let pick = assets
        .iter()
        .find(|a| a["name"].as_str().map(|n| n.contains("for Windows") && n.ends_with(".zip")).unwrap_or(false))
        .or_else(|| assets.iter().find(|a| a["name"].as_str().map(|n| n.ends_with(".zip")).unwrap_or(false)));
    // A release's download is built after it's published (about half an hour): none yet means
    // nothing to update to this time.
    let Some(a) = pick else { return Ok(None) };
    Ok(Some(Release {
        version,
        asset_url: a["browser_download_url"].as_str().unwrap_or("").to_string(),
        asset_name: a["name"].as_str().unwrap_or("download.zip").to_string(),
        asset_size: a["size"].as_u64().unwrap_or(0),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(split_address("24.64.116.126:3725"), ("24.64.116.126".into(), 3725));
        assert_eq!(split_address("host"), ("host".into(), 3724));
        assert_eq!(split_address("pc.tail1234.ts.net:3725"), ("pc.tail1234.ts.net".into(), 3725));
    }
}
