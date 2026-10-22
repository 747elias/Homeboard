# Changelog

## 0.2.0-beta – 2026-10-22

- Google-Tasks-/Keep-Karten durch lokale Aufgaben-Widgets ersetzt; automatische einmalige Migration mit Sicherung.
- Eigene Aufgabenverwaltung unter /tasks mit elf Ansichten und benannten gespeicherten Ansichten.
- Tabellenbearbeitung, Kanban-Drag-and-drop, Monatskalender, Woche, Agenda, Zeitleiste, Matrix, Projekte, Statistiken und Archiv.
- Projekte, Tags, Prioritäten, Termine, Unteraufgaben, Notizen, Links, Favoriten und Zeiterfassung.
- Serverseitige Wiederholungen, Mehrfachaktionen, Rückgängig und separate Aufgaben-Sicherungen.
- CSV-Export mit Formel-Schutz sowie JSON-Import (Zusammenführen/Ersetzen) und Export.
- Authentifizierung, CSRF, Whitelist und localhost-Bearbeitung gelten auch für Aufgaben.
- Zusätzliche Rust-/HTTP- und JavaScript-Tests sowie Aufgaben-Persistenztest im Windows-Installer-Smoke-Test.

## 0.1.0-beta – 2026-10-22

- Erste Beta: eingebettetes Dashboard und visueller Editor mit 12-Spalten-Raster.
- Uhr, Open-Meteo, Kalender, Link-Karten und frei konfigurierbare Basis-Widgets.
- Argon2-Anmeldung, CSRF-Schutz, Sitzungen, IP/CIDR/MAC-Zugriffsregeln.
- Sandbox für eigene Widgets, JSON-Import/Export, zehn automatische Sicherungen.
- Windows-Dienst, Inno-Setup-Installer, optionale private Firewall-Regel.
- Windows-Builds, Tests, Installer-Smoke-Test und automatisches Pre-release.
