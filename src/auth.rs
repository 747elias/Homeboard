use anyhow::{bail, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    net::IpAddr,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthFile {
    pub password_hash: Option<String>,
}
impl AuthFile {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("auth.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let value: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        if let Some(hash) = &value.password_hash {
            PasswordHash::new(hash).map_err(|_| anyhow::anyhow!("Ungültiger Passwort-Hash"))?;
        }
        Ok(value)
    }
    pub fn save(&self, dir: &Path) -> Result<()> {
        crate::config::atomic_write(&dir.join("auth.json"), &serde_json::to_vec(self)?)
    }
}
pub fn hash_password(password: &str) -> Result<String> {
    if !(12..=256).contains(&password.chars().count()) {
        bail!("Das Passwort muss 12 bis 256 Zeichen lang sein");
    }
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|_| anyhow::anyhow!("Passwort konnte nicht gehasht werden"))?
        .to_string())
}
pub fn verify(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).ok().is_some_and(|h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}
pub fn token() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub struct Session {
    pub csrf: String,
    pub expires: Instant,
    pub ip: IpAddr,
}
#[derive(Default)]
pub struct Sessions(pub HashMap<String, Session>);
impl Sessions {
    pub fn create(&mut self, ip: IpAddr) -> (String, String) {
        self.0.retain(|_, s| s.expires > Instant::now());
        if self.0.len() >= 64 {
            self.0.clear();
        }
        let (sid, csrf) = (token(), token());
        self.0.insert(
            sid.clone(),
            Session {
                csrf: csrf.clone(),
                expires: Instant::now() + Duration::from_secs(8 * 3600),
                ip,
            },
        );
        (sid, csrf)
    }
    pub fn get(&mut self, sid: &str, ip: IpAddr) -> Option<String> {
        self.0.retain(|_, s| s.expires > Instant::now());
        self.0
            .get(sid)
            .filter(|s| s.ip == ip)
            .map(|s| s.csrf.clone())
    }
}
struct Attempt {
    failures: u32,
    next: Instant,
    touched: Instant,
}
#[derive(Default)]
pub struct Limiter {
    ips: HashMap<IpAddr, Attempt>,
    global: VecDeque<Instant>,
}
impl Limiter {
    pub fn check(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        self.ips
            .retain(|_, a| now.duration_since(a.touched) < Duration::from_secs(1800));
        while self
            .global
            .front()
            .is_some_and(|t| now.duration_since(*t) > Duration::from_secs(60))
        {
            self.global.pop_front();
        }
        if self.global.len() >= 60 || self.ips.get(&ip).is_some_and(|a| a.next > now) {
            return false;
        }
        if self.ips.len() >= 1024 && !self.ips.contains_key(&ip) {
            return false;
        }
        self.global.push_back(now);
        true
    }
    pub fn failed(&mut self, ip: IpAddr) {
        let now = Instant::now();
        let a = self.ips.entry(ip).or_insert(Attempt {
            failures: 0,
            next: now,
            touched: now,
        });
        a.failures = a.failures.saturating_add(1);
        a.touched = now;
        if a.failures >= 5 {
            a.next = now + Duration::from_secs((2_u64.pow((a.failures - 4).min(8))).min(300));
        }
    }
    pub fn success(&mut self, ip: IpAddr) {
        self.ips.remove(&ip);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passwords_are_salted_and_verified() {
        let p = "Ein sehr gutes Passwort 2026";
        let a = hash_password(p).unwrap();
        let b = hash_password(p).unwrap();
        assert_ne!(a, b);
        assert!(verify(p, &a));
        assert!(!verify("falsch", &a));
        assert!(!verify(p, "invalid"));
        assert!(hash_password("kurz").is_err());
    }
    #[test]
    fn session_expiry_and_ip_binding() {
        let mut s = Sessions::default();
        let ip = "127.0.0.1".parse().unwrap();
        let (sid, csrf) = s.create(ip);
        assert_eq!(s.get(&sid, ip), Some(csrf));
        assert!(s.get(&sid, "10.0.0.1".parse().unwrap()).is_none());
        s.0.get_mut(&sid).unwrap().expires = Instant::now() - Duration::from_secs(1);
        assert!(s.get(&sid, ip).is_none());
    }
    #[test]
    fn repeated_failures_are_delayed() {
        let mut l = Limiter::default();
        let ip = "127.0.0.1".parse().unwrap();
        for _ in 0..5 {
            assert!(l.check(ip));
            l.failed(ip);
        }
        assert!(!l.check(ip));
        l.success(ip);
        assert!(l.check(ip));
    }
}
