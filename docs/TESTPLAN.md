# Manueller Testplan · Homeboard 0.1 Beta

Ausführung auf einem Windows-11-x64-Test-PC, zusätzlich einem erlaubten und einem gesperrten LAN-Gerät. Ergebnisse mit Windows-Version, Browser, Datum und Release-Prüfsumme festhalten. GitHub-Runner-Smoke-Tests ersetzen diesen Hardwaretest nicht.

| Prüfung | Schritte und erwartetes Ergebnis |
|---|---|
| Installation | EXE-Prüfsumme vergleichen; unsignierte EXE/SmartScreen-Hinweis prüfen. Setup als Administrator, Port 80, Firewall-Checkbox aus. Dateien in Program Files, geschützte Daten in ProgramData. Browser öffnet sich nicht. |
| Portkonflikt | Einen Test-Webserver auf 80 starten. Setup muss verständliche Meldung liefern. Zurück auf 8080 wechseln, Installation erfolgreich, URL benötigt :8080. Auch Konflikt nur auf IPv4 prüfen. |
| Privilegien | Als normaler Benutzer Browser öffnen: Dashboard und Editor funktionieren. Direkter Zugriff auf auth.json verweigert. Dienst läuft als LocalService, nicht als Administrator. |
| Erster Start | /edit lokal: Passwort kürzer als 12 Zeichen ablehnen, Wiederholung prüfen; langes Passwort setzen. auth.json enthält Argon2id-Hash, keinen Klartext. Erstvergabe von erlaubter LAN-IP bleibt verweigert. |
| Neustart | PC wirklich neu starten, vor Anmeldung von erlaubtem LAN-Gerät anfragen. Dienst läuft automatisch. Nach Anmeldung ist kein Browser automatisch geöffnet. |
| IP/CIDR | Leere Liste: localhost und ::1 erlaubt, andere IP zeigt 403. Einzelne IPv4 und IPv6, dann CIDR erlauben; Nachbaradresse ausserhalb des Bereichs bleibt gesperrt. |
| MAC | Windows-Nachbartabelle für lokales Gerät kontrollieren; dessen echte MAC erlauben. Nur dasselbe lokale Gerät darf zugreifen. Falsche MAC/fehlender Eintrag bleiben gesperrt. Über Router kein MAC-Match. Spoofbarkeit dokumentiert. |
| Editor lokal | Schalter eingeschaltet: auch erlaubte LAN-IP erhält auf /edit und /api/editor/config 403. Abschalten und speichern: LAN-Anmeldung möglich, ohne Sitzung keine Editor-Daten. |
| Notfall | Dienst stoppen, --reset-access ausführen, Dienst starten. Localhost bleibt erlaubt, alle LAN-Geräte gesperrt, Editor nur lokal. Bei laufendem Dienst muss Reset scheitern. |
| Passwort | Richtige/falsche Anmeldung, Logout und Passwortwechsel. Altes Passwort und alte Sitzung danach unbrauchbar. Nach Dienstneustart neu anmelden. Fünf Fehlversuche: weitere Versuche zeitweise 429. Passwort zurücksetzen, lokal neu setzen. |
| Cookies/CSRF | DevTools: HttpOnly/SameSite=Strict. Schreibanfrage ohne CSRF-Token bzw. mit fremdem Origin bleibt 403. Keine vertraulichen Daten in Logs. |
| Raster | Alle fünf Standardkarten verschieben/vergrössern, Überlappung: andere Karten rücken nach unten. Numerische Positionen, minimal/maximal gültige Grösse. Speichern, Browser neu laden: Layout gleich. |
| Widget-Aktionen | Jeden Typ hinzufügen, Titel/Inhalt/URL ändern, duplizieren und löschen. Rückgängig vor/nach Speichern. Leeres Dashboard speichern. Tastaturfokus und Smartphone-Ansicht prüfen. |
| Wetter | Ort „Zürich“ suchen und Treffer wählen, alternativ Koordinaten. Aktuell/Stunden/5 Tage plausibel, Zeitzone Zürich. Zwei identische Widgets nutzen Backend-Cache. Nach 15 Minuten Aktualisierung. |
| Kalender | Eigenen erlaubten Google-Embed-Link hinterlegen. Sichtbarer Kalender oder erklärende Ersatzkarte bei blockierter Anfrage/Anmeldung. Öffnen funktioniert. Privaten Kalender nicht öffentlich machen. |
| Tasks/iframe | Tasks-URL zeigt schöne Link-Karte. Unbekannte Website im Automatikmodus ebenfalls Link-Karte. Erlaubten Anbieter manuell aktivieren; bei Anbieterblockade bleibt Öffnen sichtbar. Keine Behauptung einer sicheren browserseitigen Fehlererkennung. |
| Eigener Code | HTML/CSS/JS z. B. Button mit eigenem Zähler; funktioniert im Widget. Versuch parent.document/parent.location zu ändern scheitert. Fetch/Form/Popup scheitert. Globales CSS kann Editor nicht zerstören. |
| Design/Vorschau | Hell/dunkel, Schrift, Farbe/Verlauf/Bild und Akzent prüfen. Vorschau zeigt Entwurf und verändert gespeichertes Dashboard nicht. Schliessen, weiter bearbeiten, speichern. |
| Import/Export | Export herunterladen, Widgets ändern, Export importieren, bestätigen, Vorschau und speichern. Passwort/Hash fehlen im Export. Ungültiges JSON, falsche Version, ungültige URL und >1 MB ablehnen. Rückgängig nach Import. |
| Parallel-Editor | Zwei Tabs laden, Tab A speichert. Veralteter Tab B bekommt Konflikt statt A zu überschreiben. |
| Sicherung | Elfmal ändern/speichern: exakt zehn Backups; Dienst stoppen, altes Backup wiederherstellen, starten. Beschädigte dashboard.json verursacht Startfehler und wird nicht überschrieben. |
| Offline | Vorher Wetter laden, Netzwerk trennen und Aktualisierung abwarten. Uhr/Text/Links bleiben bedienbar, Wetter als veraltet oder Hinweis. Defekte Bild-/iframe-URL betrifft nur dieses Widget. Ohne Netz neu starten: Oberfläche vollständig lokal. |
| Logs/Leerlauf | Verweigerte Anfrage mit Zeit/IP vorhanden, keine URL/Passwörter/Inhalte. Tägliche Rotation, höchstens 14 Logdateien. Taskmanager nach 5 Minuten Leerlauf: CPU nahe 0, RAM dokumentieren; kein Browser durch Dienst gestartet. |
| Firewall | Saubere Installation mit Checkbox aus: keine Homeboard-Regel. Checkbox an: genau eine TCP-Regel nur Profil Private und passender EXE/Port. Öffentliches Netzwerk bleibt geschlossen. |
| Deinstallation | Einmal „Daten behalten“: Dienst/Firewall/Startmenü/Programm weg, ProgramData bleibt. Neu installieren: Dashboard/Passwort vorhanden. Erneut deinstallieren mit „Daten löschen“: auch ProgramData weg. |

Protokollvorlage: `Datum | Tester | Windows/Browser | Prüfsumme | Fall | bestanden/fehlgeschlagen | Beobachtung`. Die Screenshots in der README stammen aus der echten Browseroberfläche; Wetter-Beispieldaten werden im Prüfbericht benannt, falls verwendet.
