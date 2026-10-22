# Abschlussbericht · Homeboard 0.2.0-beta

Ersetzt die früheren Google-Tasks-/Keep-Link-Karten durch lokale Aufgabenverwaltung. Google Kalender bleibt optional erhalten. Rust/Axum/Tokio, eine EXE mit eingebetteter Oberfläche und Windows-Dienst wie bisher.

## Lieferumfang

Elf vollständige Ansichten: Tabelle, Liste, Kanban, Monatskalender, Wochenplan, Agenda, Zeitleiste, Prioritätenmatrix, Projekte, Statistiken und Archiv. Dazu eigene gespeicherte Ansichten, direkte Tabellenbearbeitung, Filter/Suche/Gruppierung/Sortierung, Mehrfachaktionen, Projekte/Farben, Unteraufgaben, Tags, Notizen, Links, Favoriten, Wiederholungen, Timer/Minuten, Vorlagen, CSV-/JSON-Export und JSON-Import. Automatisches Speichern, Revisionskonflikte, Rückgängig und zehn Aufgaben-Sicherungen.

Die Aufgabenverwaltung verwendet dieselbe Anmeldung, Whitelist, localhost-Bearbeitungsbeschränkung und denselben CSRF-Schutz wie der Editor. Erlaubte IPs dürfen Aufgaben lesen. Keine externe Aufgaben-API und keine Google-Anmeldung für Aufgaben.

## Prüfungen

- 17 Rust-Unit-/HTTP-Tests: bisherige Schutzmechanismen, Aufgaben-Persistenz/Sicherungen, Datenvalidierung, Monatsende/Werktage, wiederholtes Speichern, Migration alter Karten, Session-/CSRF-/Revisionsschutz und Leser-/Bearbeiterzugriff.
- Vier JavaScript-Testgruppen: Zürcher Tagesgrenze/Sommerzeit, kombinierte Filter/Unteraufgabensuche, Sortierung/Matrix und Excel-Formelschutz im CSV.
- Browserprüfung: alle elf Ansichten, neue Aufgabe/Details, direkte Prioritätsänderung, Suche, Gruppierung, Spalten, gespeicherte Ansicht, Unteraufgabe, Notiz, Zeit, Duplizieren, Rückgängig, Kanban-Drag-and-drop, Export/Import, Archiv/Wiederherstellung und Anmeldung. Screenshots enthalten eigens angelegte Testaufgaben aus der tatsächlichen Anwendung.
- Windows-Workflows führen MSVC-Build, rustfmt, Clippy, Rust-/JavaScript-Tests, Inno-Setup und den Installer-Smoke-Test aus. Dieser prüft Installation, LocalService/Autostart-Konfiguration, HTTP, Anmeldung, Dashboard-/Aufgaben-Speichern, Persistenz nach Dienstneustart, Reset und Deinstallation mit Datenerhalt.

Die konkreten Ergebnisse der Windows-Läufe und Release-Uploads sind über die folgenden Links dokumentiert. Die Chat-Auslieferung nennt den verifizierten Endstand.

## Links und Dateien

- [Repository](https://github.com/747elias/Homeboard), Branch main.
- [Pre-release v0.2.0-beta](https://github.com/747elias/Homeboard/releases/tag/v0.2.0-beta).
- [Build-Workflow](https://github.com/747elias/Homeboard/actions/workflows/build.yml).
- [Release-Workflow](https://github.com/747elias/Homeboard/actions/workflows/release.yml).
- Release-Dateien: `homeboard.exe`, `Homeboard-Setup-0.2.0-beta.exe`, `SHA256SUMS.txt`.

## Noch auf echtem Windows 11 zu testen

Physischer Neustart mit Zugriff vor Benutzeranmeldung, Upgrade einer realen 0.1-Installation, interaktive Setup-/Deinstallationsdialoge, Firewall-Profile, echte LAN-/IPv6-/MAC-Geräte sowie Kalender mit persönlichem Konto. Windows-CI ersetzt diesen Hardwaretest nicht. Der manuelle Testplan ist erweitert.

## Grenzen

Maximal 2.000 Aufgaben und 1 MB kompakte Aufgabendaten, 100 Projekte/eigene Ansichten; keine externe Synchronisation, Rollenverwaltung, Betriebssystem-Erinnerungen und kein CSV-Import. Ein Timer lebt im geöffneten Detailfenster und wird beim Speichern übernommen. Aufgaben beginnen ohne Beispieldaten. Wiederherstellung externer Kontoinhalte ist kein Bestandteil der Kartenmigration. EXE/Installer sind unsigniert; HTTP ist im LAN unverschlüsselt; MAC-Regeln bleiben auf das lokale Netz beschränkt und spoofbar.
