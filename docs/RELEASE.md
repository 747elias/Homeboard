Homeboard 0.2 (Beta) für Windows 11 x64.

- **Homeboard-Setup-0.2.0-beta.exe**: empfohlene Installation, richtet den automatischen Backend-Dienst ein.
- **homeboard.exe**: einzelne EXE mit eingebetteter Oberfläche; ohne Installer im Konsolenmodus startbar.
- **SHA256SUMS.txt**: SHA-256-Prüfsummen beider EXE-Dateien.

Nach der Installation selbst http://localhost/edit öffnen und ein Admin-Passwort setzen. Kein Browser-Autostart. Localhost ist immer erlaubt; LAN-Zugriff muss freigegeben werden.

Die EXE ist nicht signiert. Windows SmartScreen kann deshalb warnen. Dies ist eine Beta; bitte den manuellen Windows-Testplan in `docs/TESTPLAN.md` beachten. Aufgaben werden vollständig lokal unter /tasks verwaltet; MAC-Regeln gelten nur im selben lokalen Netz.

Neu in 0.2: eigene Aufgabenverwaltung mit elf Ansichten, Tabelle mit direkter Bearbeitung, Kanban, Kalender, Filtern, gespeicherten Ansichten, Projekten, Wiederholungen, Unteraufgaben, Notizen, Zeit, Mehrfachaktionen und CSV-/JSON-Export. Alte Aufgaben-Link-Karten werden mit Sicherung in lokale Widgets umgewandelt. Kein Google-Tasks-Konto und keine Tasks-API nötig. Vorhandene Dashboard- und Passwortdaten bleiben beim Upgrade erhalten.
