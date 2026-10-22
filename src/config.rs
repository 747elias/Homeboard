use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub revision: u64,
    pub widgets: Vec<Widget>,
    pub appearance: Appearance,
    pub access: Access,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Widget {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub settings: Settings,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub text: String,
    pub url: String,
    pub icon: String,
    pub timezone: String,
    pub latitude: f64,
    pub longitude: f64,
    pub place: String,
    pub html: String,
    pub css: String,
    pub js: String,
    pub embed_mode: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            text: String::new(),
            url: String::new(),
            icon: "↗".into(),
            timezone: "Europe/Zurich".into(),
            latitude: 47.3769,
            longitude: 8.5417,
            place: "Zürich".into(),
            html: String::new(),
            css: String::new(),
            js: String::new(),
            embed_mode: "auto".into(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    pub theme: String,
    pub background_type: String,
    pub background: String,
    pub accent: String,
    pub font_size: u32,
    pub custom_css: String,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            background_type: "gradient".into(),
            background: "linear-gradient(145deg, #10151f, #141c29 65%, #182833)".into(),
            accent: "#8bdfc5".into(),
            font_size: 16,
            custom_css: String::new(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Access {
    pub entries: Vec<String>,
    pub editor_local_only: bool,
}
impl Default for Config {
    fn default() -> Self {
        let make = |id: &str, kind: &str, title: &str, x, y, w, h, settings| Widget {
            id: id.into(),
            kind: kind.into(),
            title: title.into(),
            x,
            y,
            w,
            h,
            settings,
        };
        Self {
            version: 1,
            revision: 0,
            appearance: Appearance::default(),
            access: Access {
                entries: vec![],
                editor_local_only: true,
            },
            widgets: vec![
                make(
                    "clock",
                    "clock",
                    "Ein guter Tag beginnt hier.",
                    0,
                    0,
                    5,
                    3,
                    Settings::default(),
                ),
                make(
                    "weather",
                    "weather",
                    "Wetter in Zürich",
                    5,
                    0,
                    7,
                    5,
                    Settings::default(),
                ),
                make(
                    "calendar",
                    "embed",
                    "Mein Kalender",
                    0,
                    3,
                    5,
                    5,
                    Settings {
                        icon: "▦".into(),
                        ..Settings::default()
                    },
                ),
                make(
                    "tasks",
                    "embed",
                    "Meine Aufgaben",
                    5,
                    5,
                    4,
                    3,
                    Settings {
                        url: "https://tasks.google.com/".into(),
                        icon: "✓".into(),
                        ..Settings::default()
                    },
                ),
                make(
                    "focus",
                    "embed",
                    "Heute im Fokus",
                    9,
                    5,
                    3,
                    3,
                    Settings {
                        url: "https://keep.google.com/".into(),
                        icon: "◎".into(),
                        ..Settings::default()
                    },
                ),
            ],
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "::".into(),
            port: 80,
        }
    }
}
pub fn validate_url(value: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    let u = url::Url::parse(value).context("Ungültige URL")?;
    if !matches!(u.scheme(), "http" | "https")
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || value.len() > 4096
    {
        bail!("Nur HTTP(S)-URLs ohne Zugangsdaten sind erlaubt");
    }
    Ok(())
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 || self.widgets.len() > 100 {
            bail!("Ungültige Version oder zu viele Widgets (max. 100)");
        }
        let a = &self.appearance;
        if !matches!(a.theme.as_str(), "light" | "dark")
            || !matches!(a.background_type.as_str(), "color" | "gradient" | "image")
            || !(12..=28).contains(&a.font_size)
            || a.background.len() > 4096
            || a.custom_css.len() > 100_000
            || a.accent.len() != 7
            || !a.accent.starts_with('#')
            || !a.accent[1..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            bail!("Ungültige Darstellung");
        }
        if a.background_type == "image" {
            validate_url(&a.background)?;
        }
        if self.access.entries.len() > 256 {
            bail!("Maximal 256 Whitelist-Einträge");
        }
        for e in &self.access.entries {
            crate::access::parse_entry(e)?;
        }
        let mut ids = std::collections::HashSet::new();
        for w in &self.widgets {
            if w.id.is_empty()
                || w.id.len() > 80
                || !ids.insert(&w.id)
                || !w
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || !matches!(
                    w.kind.as_str(),
                    "clock"
                        | "weather"
                        | "embed"
                        | "text"
                        | "heading"
                        | "link"
                        | "image"
                        | "divider"
                        | "spacer"
                        | "code"
                )
                || w.w == 0
                || w.h == 0
                || w.x > 12
                || w.w > 12
                || w.x + w.w > 12
                || w.h > 30
                || w.y > 500
                || w.title.len() > 200
            {
                bail!("Ungültiges Widget oder Raster");
            }
            let s = &w.settings;
            validate_url(&s.url)?;
            if !s.latitude.is_finite()
                || !s.longitude.is_finite()
                || s.latitude.abs() > 90.0
                || s.longitude.abs() > 180.0
                || s.text.len() > 50_000
                || s.html.len() > 100_000
                || s.css.len() > 100_000
                || s.js.len() > 100_000
                || s.icon.chars().count() > 8
                || s.timezone.len() > 80
                || s.place.len() > 200
                || !matches!(s.embed_mode.as_str(), "auto" | "link" | "iframe")
            {
                bail!("Ungültige Widget-Einstellungen");
            }
        }
        Ok(())
    }
}
// A same-directory, flushed temporary file and atomic replacement keep the old file intact.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");
    let mut f = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let ok = unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(not(windows))]
    fs::rename(tmp, path)?;
    Ok(())
}
pub fn load(dir: &Path) -> Result<Config> {
    fs::create_dir_all(dir)?;
    let path = dir.join("dashboard.json");
    if !path.exists() {
        let c = Config::default();
        atomic_write(&path, &serde_json::to_vec_pretty(&c)?)?;
    }
    let c: Config = serde_json::from_slice(&fs::read(path)?)
        .context("dashboard.json ist beschädigt; Sicherung wiederherstellen")?;
    c.validate()?;
    Ok(c)
}
pub fn save(dir: &Path, c: &Config) -> Result<()> {
    c.validate()?;
    let path = dir.join("dashboard.json");
    let backups = dir.join("backups");
    fs::create_dir_all(&backups)?;
    if path.exists() {
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.9f");
        fs::copy(&path, backups.join(format!("dashboard-{stamp}.json")))?;
    }
    atomic_write(&path, &serde_json::to_vec_pretty(c)?)?;
    let mut files: Vec<_> = fs::read_dir(backups)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let remove = files.len().saturating_sub(10);
    for path in files.into_iter().take(remove) {
        fs::remove_file(path)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_backup_and_validation() {
        let d = tempfile::tempdir().unwrap();
        let mut c = load(d.path()).unwrap();
        c.widgets[0].title = "Neu".into();
        save(d.path(), &c).unwrap();
        assert_eq!(load(d.path()).unwrap().widgets[0].title, "Neu");
        for _ in 0..12 {
            save(d.path(), &c).unwrap();
        }
        assert_eq!(fs::read_dir(d.path().join("backups")).unwrap().count(), 10);
        c.widgets[0].settings.url = "javascript:alert(1)".into();
        assert!(save(d.path(), &c).is_err());
    }
    #[test]
    fn corrupt_config_is_not_overwritten() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("dashboard.json"), "broken").unwrap();
        assert!(load(d.path()).is_err());
    }
}
