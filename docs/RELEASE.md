Homeboard 0.1 (Beta) für Windows 11 x64.

- **Homeboard-Setup-0.1.0-beta.exe**: empfohlene Installation, richtet den automatischen Backend-Dienst ein.
- **homeboard.exe**: einzelne EXE mit eingebetteter Oberfläche; ohne Installer im Konsolenmodus startbar.
- **SHA256SUMS.txt**: SHA-256-Prüfsummen beider EXE-Dateien.

Nach der Installation selbst http://localhost/edit öffnen und ein Admin-Passwort setzen. Kein Browser-Autostart. Localhost ist immer erlaubt; LAN-Zugriff muss freigegeben werden.

Die EXE ist nicht signiert. Windows SmartScreen kann deshalb warnen. Dies ist eine Beta; bitte den manuellen Windows-Testplan in `docs/TESTPLAN.md` beachten. Google Tasks wird als Link geöffnet; MAC-Regeln gelten nur im selben lokalen Netz.
