use anyhow::{bail, Result};
use ipnet::IpNet;
use std::net::IpAddr;

#[derive(Debug, PartialEq)]
pub enum Entry {
    Ip(IpAddr),
    Net(IpNet),
    Mac([u8; 6]),
}
pub fn normalize(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v) => v.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        _ => ip,
    }
}
pub fn mac(value: &str) -> Option<[u8; 6]> {
    let cleaned = value.replace('-', ":");
    let parts: Vec<_> = cleaned.split(':').collect();
    if parts.len() != 6 || parts.iter().any(|p| p.len() != 2) {
        return None;
    }
    let mut result = [0; 6];
    for (i, p) in parts.iter().enumerate() {
        result[i] = u8::from_str_radix(p, 16).ok()?;
    }
    if result == [0; 6] || result == [255; 6] || result[0] & 1 != 0 {
        return None;
    }
    Some(result)
}
pub fn parse_entry(value: &str) -> Result<Entry> {
    let v = value.trim();
    if let Ok(ip) = v.parse::<IpAddr>() {
        return Ok(Entry::Ip(normalize(ip)));
    }
    if let Ok(net) = v.parse::<IpNet>() {
        return Ok(Entry::Net(net));
    }
    if let Some(m) = mac(v.strip_prefix("mac:").unwrap_or(v)) {
        return Ok(Entry::Mac(m));
    }
    bail!("Ungültiger Whitelist-Eintrag: IP, CIDR oder MAC erwartet")
}
pub fn allowed(ip: IpAddr, entries: &[String], neighbor: Option<[u8; 6]>) -> bool {
    let ip = normalize(ip);
    ip.is_loopback()
        || entries
            .iter()
            .filter_map(|e| parse_entry(e).ok())
            .any(|e| match e {
                Entry::Ip(other) => ip == other,
                Entry::Net(net) => net.contains(&ip),
                Entry::Mac(m) => neighbor == Some(m),
            })
}
// Windows' neighbor table contains directly attached IPv4 (ARP) and IPv6 (NDP) peers.
// No packets are trusted to supply a MAC, and no forwarded HTTP header is consulted.
pub async fn neighbor(ip: IpAddr) -> Option<[u8; 6]> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let script = format!("Get-NetNeighbor -IPAddress '{}' -ErrorAction SilentlyContinue | Where-Object {{ $_.State -ne 'Unreachable' -and $_.State -ne 'Incomplete' }} | Select-Object -ExpandProperty LinkLayerAddress", normalize(ip));
        let mut cmd = tokio::process::Command::new("powershell.exe");
        cmd.args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .kill_on_drop(true);
        cmd.as_std_mut().creation_flags(0x08000000);
        let output = tokio::time::timeout(std::time::Duration::from_secs(3), cmd.output())
            .await
            .ok()?
            .ok()?;
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout)
                .lines()
                .find_map(|s| mac(s.trim()));
        }
        None
    }
    #[cfg(not(windows))]
    {
        let _ = ip;
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }
    #[test]
    fn loopback_cannot_be_locked_out() {
        for s in ["127.0.0.1", "::1", "::ffff:127.0.0.1"] {
            assert!(allowed(ip(s), &[], None));
        }
    }
    #[test]
    fn exact_and_cidr_v4_v6() {
        let list = vec![
            "192.168.1.0/24".into(),
            "2001:db8::/64".into(),
            "10.0.0.2".into(),
        ];
        for s in ["192.168.1.23", "2001:db8::3", "10.0.0.2", "::ffff:10.0.0.2"] {
            assert!(allowed(ip(s), &list, None));
        }
        for s in ["192.168.2.2", "2001:db9::1", "10.0.0.3"] {
            assert!(!allowed(ip(s), &list, None));
        }
    }
    #[test]
    fn mac_is_only_a_local_neighbor_match() {
        let list = vec!["mac:02-AA-bb-cc-dd-ee".into()];
        assert!(allowed(ip("192.168.1.2"), &list, mac("02:aa:bb:cc:dd:ee")));
        assert!(!allowed(ip("192.168.1.2"), &list, None));
        assert!(!allowed(ip("192.168.1.2"), &list, mac("02:aa:bb:cc:dd:ef")));
        assert!(parse_entry("nonsense").is_err());
        assert!(mac("ff:ff:ff:ff:ff:ff").is_none());
    }
}
