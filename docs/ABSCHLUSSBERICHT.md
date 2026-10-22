# Abschlussbericht · Homeboard 0.1.0-beta

Stand: 22. Oktober 2026. Rust/Axum/Tokio mit eingebetteter Oberfläche, ohne zusätzliche Laufzeitumgebung und ohne Docker.

## Gebaut und getestet

- Linux: kompilierbarer Quellcode, optimierter Build, `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` und zwölf bestandene Unit-/HTTP-Tests.
- Chromium: Dashboard, Passwort-Erstvergabe, Login/Logout, Text-Widget, Duplizieren, Rückgängig, echtes Pointer-Drag & Drop, Grössenänderung, Speichern, Vorschau und JSON-Export/-Import ohne JavaScript-Fehler.
- Eigener Code: JavaScript-Zähler funktioniert im tatsächlichen Sandbox-iframe; Zugriff auf das übergeordnete Dokument wird blockiert.
- Open-Meteo: echte Backend-Abfrage erfolgreich. Screenshots zeigen Live-Wetterdaten und die tatsächliche Browseroberfläche, keine simulierte Wetterantwort. Fünf Tage vollständig sichtbar. Desktop und mobile Darstellung geprüft.
- Windows auf GitHub `windows-latest`: erster vollständiger [Build-Lauf](https://github.com/747elias/Homeboard/actions/runs/39766738344) erfolgreich. MSVC-Build, rustfmt, Clippy, zwölf Tests und Inno-Setup-Kompilierung bestanden.
- Derselbe Windows-Lauf hat den Installer ausgeführt und automatischen Dienst/LocalService, HTTP, Anmeldung, Speichern, Dienstneustart, Konfigurationspersistenz, Zugriff-/Passwort-Reset und Deinstallation mit Datenerhalt erfolgreich geprüft.

## Noch auf echtem Windows 11 zu testen

Der Windows-Dienst und der Installer sind über einen Windows-CI-Runner getestet. Noch offen sind ein **physischer Windows-11-Neustart mit Zugriff vor Benutzeranmeldung**, reale LAN-Geräte mit erlaubten/gesperrten IPv4-/IPv6-Adressen und MAC-Nachbartabellen, Firewall-Profile im echten Netzwerk, SmartScreen, Google-Kalender mit persönlichem Konto und die interaktiven Setup-/Deinstallationsdialoge einschliesslich Datenlöschung. Der [manuelle Testplan](TESTPLAN.md) enthält die Schritte. Das ist keine Behauptung eines abgeschlossenen Hardwaretests.

## Repository und Veröffentlichungsprüfung

- [Repository Homeboard](https://github.com/747elias/Homeboard), Branch `main`, MIT-Lizenz.
- [Pre-release v0.1.0-beta](https://github.com/747elias/Homeboard/releases/tag/v0.1.0-beta).
- [Build-Workflow](https://github.com/747elias/Homeboard/actions/workflows/build.yml) und [Release-Workflow](https://github.com/747elias/Homeboard/actions/workflows/release.yml).
- Vorgesehene Release-Dateien: `homeboard.exe`, `Homeboard-Setup-0.1.0-beta.exe`, `SHA256SUMS.txt`. Der Release-Workflow veröffentlicht erst nach erfolgreichem Build und Installer-Smoke-Test. Der endgültige Lauf und die verifizierten Datei-Prüfsummen werden im Abschluss der Auslieferung festgehalten.

## Grenzen

Google Tasks wird ehrlich als Link-Karte dargestellt. Unbekannte iframe-Ziele fallen im Automatikmodus konservativ auf eine Karte zurück; manuell aktivierte iframes behalten immer einen Öffnen-Link. MAC-Regeln funktionieren nur im selben lokalen Netz und sind spoofbar. HTTP überträgt im LAN unverschlüsselt; der Editor bleibt standardmässig nur lokal erreichbar. EXE und Setup sind unsigniert. Diese Grenzen und Wiederherstellungsbefehle sind in der deutschen README dokumentiert.
