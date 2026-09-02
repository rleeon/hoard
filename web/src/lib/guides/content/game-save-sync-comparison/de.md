---
title: "Spielstand-Sync im Vergleich: Hoard gegen Ludusavi, Syncthing, OpenSave und die anderen"
description: "Ein ehrlicher Vergleich der Tools, die PC-Spielstände sichern und synchronisieren — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync und Hoard — mit Tabelle und einem Abschnitt darüber, wo Hoard verliert."
order: 4
updated: 2026-09-01
---

Steam Cloud deckt nur Spiele ab, die du bei Steam gekauft hast, und auch nur dann, wenn der Entwickler es eingeschaltet hat. Emulatoren, GOG, Epic, itch.io, Nicht-Steam-Spiele, alles Gemoddete: nichts davon ist dabei. Wer auf mehr als einem Rechner spielt, etwa Desktop und Steam Deck, kopiert am Ende Ordner von Hand und hofft, den neuesten erwischt zu haben.

Mehrere Tools lösen das, und sie tun nicht alle dasselbe. Manche legen lokale Backups an, manche spiegeln Ordner zwischen Geräten, manche laden in eine Cloud. Diese Seite geht sie durch und sagt, worin jedes wirklich gut ist. Hoard ist mein Projekt, deshalb kommt der ehrliche Teil am Schluss: ein Abschnitt darüber, wo Hoard verliert, und eine Tabelle, die man lesen kann, ohne dem Fließtext ein Wort zu glauben.

## Ludusavi

Das bekannteste, und das zu Recht. Ludusavi (von mtkennerly) ist ein kostenloses Open-Source-Backup-Tool mit Oberfläche und CLI, aufgebaut auf dem Community-Manifest der Spielstand-Pfade, das Zehntausende Spiele abdeckt — dasselbe Manifest, das fast alle hier verwenden, Hoard eingeschlossen. Es hält versionierte lokale Backups und kann sie über Rclone in deine eigene Cloud schieben.

**Am besten, wenn:** du lokale Backups, volle Kontrolle und nirgendwo einen Server willst. Die sicherste Wahl dieser Liste, und sie kostet nichts.

**Wo es aufhört:** Sync zwischen Rechnern ist etwas, das du selbst zusammenbaust. Backup planen, Rclone-Remote einrichten, und daran denken, auf dem anderen PC wiederherzustellen, *bevor* du spielst. Das funktioniert, aber nichts hindert dich daran, den letzten Schritt zu vergessen.

## Syncthing

Überhaupt kein Spiele-Tool, sondern ein allgemeiner Peer-to-Peer-Ordnerspiegel, und ein sehr guter. Zeig ihm einen Spielstandordner, und er taucht auf deinen anderen Geräten auf.

**Am besten, wenn:** du es ohnehin betreibst und die Dateien ohne Cloud dazwischen an zwei Orten haben willst.

**Wo es aufhört:** es spiegelt, es fotografiert nicht. Ein kaputter Spielstand erreicht jedes Gerät in Sekunden, genauso schnell wie ein guter. Die Dateiversionierung arbeitet pro Datei und hat keinen Begriff davon, was eine Spielsitzung ist — "zurück auf Dienstagabend" rekonstruierst du also von Hand. Zwei Maschinen, die beide offline gespielt haben, liefern dir Konfliktdateien, keine Zusammenführung.

## OpenSave

Peer-to-peer-Sync, eigens für Spielstände gebaut, in Go, MIT-lizenziert, für Windows, Linux und Steam Deck. Kein Konto, kein Server: Geräte koppeln sich miteinander und synchronisieren über das LAN oder per Raumcode über ein Relay. Jede Änderung wird als Snapshot festgehalten, es gibt Branches für parallele Durchläufe, Konflikte werden über die Sync-Abstammung statt über die Uhrzeit aufgelöst, und übertragen werden nur die geänderten Blöcke. Optional lässt sich zu Drive, Dropbox, OneDrive oder WebDAV spiegeln.

**Am besten, wenn:** du partout kein Konto willst und deine Geräte oft genug gleichzeitig laufen.

**Wo es aufhört:** Peer-to-Peer heißt, der Spielstand lebt nur auf deinen Geräten. Stirbt das Deck mit der einzigen aktuellen Kopie und war die Spiegelung nie eingerichtet, war's das. Für einen Sync müssen beide Geräte laufen, und einen macOS-Build gibt es nicht.

## OpenCloudSaves

Eine plattformübergreifende Oberfläche, die deine Spielstandordner in eine Cloud synchronisiert, für die du ohnehin zahlst — OneDrive, Google Drive, Dropbox, Nextcloud — mit Rclone darunter.

**Am besten, wenn:** du deine Spielstände in einem Speicherkonto haben willst, das du schon hast, mit Oberfläche statt Rclone-Konfigurationsdateien.

**Wo es aufhört:** es gibt keine inhaltsbasierte Deduplizierung. Zehn Kopien eines 2-GB-Spielstands sind 20 GB deines Drive-Kontingents, und Cloud-Laufwerke synchronisieren Dateien, keine Spielsitzungen — du bekommst also zurück, wie der Ordner damals eben aussah.

## Game Backup Monitor

Windows zuerst, und der Ursprung dieses ganzen Genres. GBM wartet auf den Spielprozess und packt den Spielstand beim Beenden mit 7-Zip ein, mit nummerierter Historie.

**Am besten, wenn:** du an einem einzigen Windows-PC sitzt und ein komprimiertes lokales Archiv ohne Nachdenken willst.

**Wo es aufhört:** es ist ein Backup-Tool, kein Sync-Tool. Das Archiv auf eine zweite Maschine zu bekommen, ist dein Problem, und Steam Deck / SteamOS ist nicht sein Zuhause.

## Aletheia

Das jüngste der Runde, AGPL, und es geht genau die Stelle an, die alle anderen halb abdecken: die Launcher. Heroic, itch.io, Lutris, Steam, GOG Galaxy und Xbox, unter Windows, Linux und macOS.

**Am besten, wenn:** deine Bibliothek über Launcher verteilt ist, die andere Tools schlecht erkennen — vor allem Xbox/Game Pass und Heroic.

**Wo es aufhört:** ein junges Projekt mit bewusst engem Zuschnitt. Sichern und Wiederherstellen ist der Funktionsumfang; eine versionierte Cloud steht nicht dahinter.

## SaveSync

Das kommerzielle, auf Steam als Einmalkauf, mit Fokus auf Windows. Sein Kniff: Es zielt gar nicht auf dich-an-zwei-PCs, sondern auf Koop. Spielstände landen in privaten, nicht gelisteten Steam-Workshop-Einträgen, damit ein Freund deine Valheim- oder Factorio-Welt ziehen kann, und LAN-Sync gibt es auch.

**Am besten, wenn:** dein Problem "mein Freund hostet und ich brauche seinen Spielstand" lautet und nicht "meine Spielstände sollen mir folgen".

**Wo es aufhört:** Closed Source, Windows, an Steam als Transportweg gebunden, und eine Liste unterstützter Koop-Spiele statt allem, was du besitzt.

## Eine Anmerkung zu EmuDeck

EmuDeck kommt in diesen Gesprächen auf und ist kein Konkurrent im üblichen Sinn: Es ist ein Installer und Konfigurator für Emulatoren auf dem Steam Deck, und der angebotene Sync ist eine Bequemlichkeit, die an diese Aufgabe angeflanscht ist (Rclone gegen ein Cloud-Laufwerk, nur für Emulator-Spielstände). Es überschneidet sich mit den Tools oben, ohne dasselbe zu sein: EmuDeck richtet deine Emulatoren ein, die Tools hier kümmern sich um die Spielstände der ganzen Bibliothek. Manche betreiben EmuDeck neben einem davon, und das ist ein sinnvolles Setup, kein doppeltes.

## Hoard

Hoard nimmt die Spielsitzung als Einheit. Die Engine läuft als Hintergrunddienst — `hoardd`, ohne Fenster, also funktioniert sie im Game Mode von SteamOS —, merkt, dass du aufgehört hast zu spielen, und macht dann den Snapshot, statt mitten im Spiel auf jeden Schreibvorgang zu reagieren.

- **Versionshistorie pro Sitzung.** Jede Sitzung ist eine Version, zu der du zurückkannst, auch nach einem Plattenausfall oder einer Neuinstallation.
- **Deduplizierung über Inhalts-Hashes.** Zehn Versionen eines 2-GB-Spielstands kosten rund 2 GB, nicht 20 GB. Übertragungen sind zstd-komprimiert.
- **SHA-256 beim Hochladen und beim Herunterladen.** Beschädigungen werden erkannt, bevor sie einen guten Spielstand überschreiben können. Nichts wird stillschweigend überschrieben — darum geht es im Kern.
- **Cloud oder selbst gehostet, dasselbe Binary.** Hoard Cloud hat einen kostenlosen Tarif (2 GB, 3 Geräte, volle Historie). Oder du betreibst `hoard-server` selbst per Docker Compose gegen beliebigen S3-kompatiblen Speicher — MinIO, Garage, Backblaze B2 — ohne Konto und ohne Kontingent. AGPL-3.0.
- **Windows, Linux, macOS**, dazu eine headless CLI für ein Steam Deck oder einen Server.
- **Emulatoren in der Beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP und weitere als Voreinstellungen.

## Das Detail, an dem Steam Deck ↔ PC hängt

Gut zu wissen, egal welches Tool du nimmst. Der Cloud-Spielstand eines Steam-Spiels liegt in `<AppID>/remote/`, und der Ordner *darüber* enthält `remotecache.vdf`, Erfolgsstände, Statistiken und Spielzeitzähler — alles Dinge, die sich zwischen Deck und Desktop berechtigterweise unterscheiden.

Synchronisiere den übergeordneten Ordner, und du hast einen Dauerkonflikt zwischen zwei Maschinen, die sich über keinen einzigen Spielstand uneinig waren. Hoard verfolgt `remote/`, nicht den Elternordner. Jedem Tool, dem du einen Ordner von Hand zuweist, kann man dasselbe beibringen — und es ist das Erste, was man prüft, wenn ein Sync-Setup ohne sichtbaren Grund ständig Konflikte meldet.

## Wo Hoard verliert

- **Es will einen Server.** Cloud-Konto oder eigene Kiste, so oder so ist es Infrastruktur, und OpenSave oder Ludusavi brauchen keine.
- **Emulator-Unterstützung ist Beta.** Portable Installationen und die Eigenheiten einzelner Emulatoren erwischen es noch, und Aletheia und OpenSave decken manche Launcher- und Emulator-Sonderfälle heute besser ab.
- **macOS ist auf echter Hardware kaum getestet.** Es baut und läuft, aber niemand hat monatelang darauf gelebt.
- **Es ist jung.** Ludusavi und Game Backup Monitor haben Jahre an Fehlerberichten hinter sich. Hoard nicht, und das zählt bei etwas, das einen 200-Stunden-Spielstand hütet.
- **Es macht kein Koop-Teilen.** Wenn du einem Freund eine Welt geben willst, ist SaveSync dafür gebaut und Hoard nicht.

## Der Unterschied zwischen Hoard Cloud und Selbsthosten

Vergleiche zu Hoard werfen diese beiden fast immer in einen Topf, und das Ergebnis stimmt dann nicht. Deshalb klar gesagt:

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Stände liegen auf unseren Servern in der EU.
- **Ein selbst gehostetes Hoard gehört vollständig dir.** Du betreibst `hoard-server` auf deinem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir sehen weder Spielstand noch Spieltitel noch E-Mail-Adresse, weil davon nichts bei uns ankommt. Würde Hoard Cloud morgen abgeschaltet, liefe ein selbst gehostetes Setup unverändert weiter.

Dasselbe Binary, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört. Ein Detail der Genauigkeit halber: dein eigener Server hat sehr wohl eigene Zugänge — einen Benutzer und ein Token je Gerät — aber die liegen in deiner Datenbank, nicht in unserer.

## Die Tabelle

| Tool | Automatischer Sync zwischen Geräten | Wo die Spielstände liegen | Historie | Plattformen | Lizenz |
|---|---|---|---|---|---|
| **Hoard** | Ja, pro Spielsitzung | Hoard Cloud oder eigener Server (S3-kompatibel) | Versioniert pro Sitzung, dedupliziert | Win · Linux · macOS · Deck | AGPL-3.0, kostenloser Tarif |
| **Ludusavi** | Manuell, oder Rclone, das du einrichtest | Lokal, plus dein Rclone-Remote | Versionierte lokale Backups | Win · Linux · macOS | Kostenlos, Open Source |
| **Syncthing** | Ja, fortlaufender Spiegel | Nur deine Geräte | Versionierung pro Datei | Alles | Kostenlos, Open Source |
| **OpenSave** | Ja, peer-to-peer | Deine Geräte, optionale Cloud-Spiegelung | Snapshots und Branches | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Ja, über dein Cloud-Laufwerk | OneDrive / Drive / Dropbox / Nextcloud | Was das Laufwerk aufhebt | Win · Linux · macOS | Kostenlos, Open Source |
| **Game Backup Monitor** | Nein | Lokale 7-Zip-Archive | Nummerierte Backups | Windows | Kostenlos, Open Source |
| **Aletheia** | Sichern und Wiederherstellen pro Launcher | Dein Speicher | Backups | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Ja, auch mit Freunden | Private Steam-Workshop-Einträge | Laut App | Windows | Kostenpflichtig, Closed Source |

## Also welches

Willst du eine Maschine gesichert haben und sonst nichts, nimm Ludusavi oder Game Backup Monitor. Willst du unter keinen Umständen ein Konto und laufen deine Geräte meist gleichzeitig, OpenSave. Sollen die Spielstände in einem Drive-Ordner landen, für den du schon zahlst, OpenCloudSaves. Teilst du eine Koop-Welt mit Freunden, SaveSync.

Willst du, dass Backup *und* Sync zwischen PCs und einem Steam Deck einfach passieren, mit einer Version pro Sitzung, zu der du zurückkannst, und der Option, das Ganze selbst zu hosten, dann ist Hoard dafür da. [Lade es herunter](/download) oder lies vorher, [wie man es mit Docker selbst hostet](/guides/self-host-hoard). Es gibt außerdem einen [ausführlichen Ludusavi-Vergleich](/guides/ludusavi-alternative), falls du genau damit abwägst.

<!-- faq -->

## Häufige Fragen

### Welches dieser Werkzeuge führt eine Versionshistorie?

Hoard behält jede Sitzung als Version, zu der du zurückkannst. Ludusavi führt versionierte lokale Backups. Die meisten übrigen synchronisieren oder kopieren den aktuellen Zustand — ein beschädigter Spielstand wandert damit getreulich auf die andere Maschine.

### Welches funktioniert ohne Server und ohne Konto?

Ludusavi mit lokalen Backups, und jedes Peer-to-peer-Werkzeug. Hoard zählt ebenfalls dazu, wenn du selbst hostest: kein Konto bei uns, und nichts, was über unsere Server läuft.

### Welches deckt Spiele ab, die nicht auf Steam sind?

Alle Spielstand-Verwalter hier, denn sie finden Stände über dieselbe Community-Datenbank statt über einen Store. Die Ausnahme ist Steam Cloud: sie deckt nur Steam-Spiele ab, deren Entwickler sie aktiviert hat.

### Muss ich mich für eines entscheiden?

Nein, und viele tun es nicht. Ein lokales Backup-Werkzeug und ein Sync-Werkzeug lösen unterschiedliche Hälften des Problems. Die einzige Regel: richte niemals eines auf den Backup-Ordner des anderen, sonst synchronisierst du einen veralteten Spiegel statt deines echten Spielstands.

### Was ist das eine Detail, an dem die meisten Eigenbau-Setups scheitern?

Den Ordner über `<AppID>/remote/` in Steams `userdata` zu synchronisieren. Der übergeordnete Ordner enthält `remotecache.vdf` sowie Dateien für Erfolge und Spielzeit, die sich pro Rechner unterscheiden sollen — jeder Start sieht dann nach einem Konflikt aus, obwohl sich kein Stand bewegt hat.
