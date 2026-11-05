---
title: "Spielstand-Sync im Vergleich: Hoard gegen Ludusavi, Syncthing, OpenSave und die anderen"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager, SaveState, EmuSync und mehr im Vergleich: worin jedes glänzt, mit Tabelle."
order: 4
updated: 2026-11-05
---

Steam Cloud deckt nur Spiele ab, die du bei Steam gekauft hast, und auch nur dann, wenn der Entwickler es eingeschaltet hat. Emulatoren, GOG, Epic, itch.io, Nicht-Steam-Spiele, alles Gemoddete: nichts davon ist dabei. Wer auf mehr als einem Rechner spielt, etwa Desktop und Steam Deck, kopiert am Ende Ordner von Hand und hofft, den neuesten erwischt zu haben.

Mehrere Tools lösen das, und sie tun nicht alle dasselbe. Manche legen lokale Backups an, manche spiegeln Ordner zwischen Geräten, manche laden in eine Cloud. Diese Seite geht sie durch und sagt, worin jedes wirklich gut ist. Hoard ist mein Projekt, deshalb kommt es zum Schluss, mit dem, was es nicht macht, und mit einer Tabelle, die man lesen kann, ohne dem Fließtext ein Wort zu glauben.

## Ludusavi

Das bekannteste, und das zu Recht. Ludusavi (von mtkennerly) ist ein kostenloses Open-Source-Backup-Tool mit Oberfläche und CLI, aufgebaut auf dem Community-Manifest der Spielstand-Pfade, das Zehntausende Spiele abdeckt — dasselbe Manifest, das fast alle hier verwenden, Hoard eingeschlossen. Es hält versionierte lokale Backups und kann sie über Rclone in deine eigene Cloud schieben.

**Am besten, wenn:** du lokale Backups, volle Kontrolle und nirgendwo einen Server willst. Die sicherste Wahl dieser Liste, und sie kostet nichts.

**Wo es aufhört:** Sync zwischen Rechnern ist etwas, das du selbst zusammenbaust. Backup planen, Rclone-Remote einrichten, und daran denken, auf dem anderen PC wiederherzustellen, *bevor* du spielst. Das funktioniert, aber nichts hindert dich daran, den letzten Schritt zu vergessen.

## Syncthing

Überhaupt kein Spiele-Tool, sondern ein allgemeiner Peer-to-Peer-Ordnerspiegel, und ein sehr guter. Zeig ihm einen Spielstandordner, und er taucht auf deinen anderen Geräten auf.

**Am besten, wenn:** du es ohnehin betreibst und die Dateien ohne Cloud dazwischen an zwei Orten haben willst.

**Wo es aufhört:** es spiegelt, es fotografiert nicht. Ein kaputter Spielstand erreicht jedes Gerät in Sekunden, genauso schnell wie ein guter. Die Dateiversionierung arbeitet pro Datei und hat keinen Begriff davon, was eine Spielsitzung ist — "zurück auf Dienstagabend" rekonstruierst du also von Hand. Zwei Maschinen, die beide offline gespielt haben, liefern dir Konfliktdateien, keine Zusammenführung.

## OpenSave

Peer-to-peer-Sync, eigens für Spielstände gebaut, in Go, MIT-lizenziert, für Windows, Linux, Steam Deck und seit Version 2.4 auch macOS. Kein Konto, kein Server: Geräte koppeln sich miteinander und synchronisieren über das LAN oder per Raumcode über ein Relay. Jede Änderung wird als Snapshot festgehalten, es gibt Branches für parallele Durchläufe, Konflikte werden über die Sync-Abstammung statt über die Uhrzeit aufgelöst, und übertragen werden nur die geänderten Blöcke. Optional lässt sich zu Drive, Dropbox, OneDrive oder WebDAV spiegeln.

**Am besten, wenn:** du partout kein Konto willst und deine Geräte oft genug gleichzeitig laufen.

**Wo es aufhört:** Peer-to-Peer heißt, der Spielstand lebt nur auf deinen Geräten. Stirbt das Deck mit der einzigen aktuellen Kopie und war die Spiegelung nie eingerichtet, war's das. Für einen Sync müssen beide Geräte laufen.

## OpenCloudSaves

Eine plattformübergreifende Oberfläche, die deine Spielstandordner in eine Cloud synchronisiert, für die du ohnehin zahlst — OneDrive, Google Drive, Dropbox, Nextcloud — mit Rclone darunter.

**Am besten, wenn:** du deine Spielstände in einem Speicherkonto haben willst, das du schon hast, mit Oberfläche statt Rclone-Konfigurationsdateien.

**Wo es aufhört:** Das Projekt wurde im Januar 2025 archiviert und wird nicht mehr gepflegt, Korrekturen für neue Spiele oder Systemupdates kommen also nicht mehr. Dazu gibt es keine inhaltsbasierte Deduplizierung. Zehn Kopien eines 2-GB-Spielstands sind 20 GB deines Drive-Kontingents, und Cloud-Laufwerke synchronisieren Dateien, keine Spielsitzungen — du bekommst also zurück, wie der Ordner damals eben aussah.

## Game Backup Monitor

Windows zuerst, und der Ursprung dieses ganzen Genres. GBM wartet auf den Spielprozess und packt den Spielstand beim Beenden mit 7-Zip ein, mit nummerierter Historie.

**Am besten, wenn:** du an einem einzigen Windows-PC sitzt und ein komprimiertes lokales Archiv ohne Nachdenken willst.

**Wo es aufhört:** es ist ein Backup-Tool, kein Sync-Tool. Das Archiv auf eine zweite Maschine zu bekommen, ist dein Problem, und Steam Deck / SteamOS ist nicht sein Zuhause.

## GameSave Manager

Ein altgedientes, kostenloses Windows-Werkzeug, Closed Source, mit eigener Spieledatenbank. Es sichert und stellt wieder her, und seine bekannteste Funktion, **Sync & Link**, verschiebt einen Spielstand-Ordner in einen Cloud-Ordner wie Dropbox oder OneDrive und hinterlässt einen Link, sodass der Cloud-Client ihn synchron hält.

**Am besten, wenn:** du unter Windows bist, ohnehin in Dropbox oder OneDrive lebst und der Spielstand einfach dort liegen soll.

**Wo es aufhört:** nur Windows, und Sync & Link bedeutet, dass ein allgemeiner Cloud-Client den aktiven Ordner synchronisiert, während das Spiel hineinschreibt — dasselbe Muster, das [Syncthing für Spielstände riskant](/guides/syncthing-game-saves) macht. Der Versionsverlauf ist das, was dein Cloud-Speicher aufbewahrt.

## Game Save Manager (GSM)

Trotz des Namens nicht dasselbe Programm wie oben. GSM von dyang886 ist eine Windows-App unter GPL-3.0, die installierte Spiele aus Steam, Epic, Battle.net und anderen Launchern über die Datenbank von PCGamingWiki findet, pro Spiel eine rotierende Reihe lokaler Backups bis zu einem von dir gesetzten Limit behält und deine ganze Historie in einem exportierten `.gsmr`-Archiv auf einen neuen PC mitnimmt. Es gibt sogar einen dritten Namensvetter: game-save-manager von mcthesw, beliebt in chinesischen Communitys, der Ordner, die du von Hand hinzufügst, nach Zeitplan sichert, mit WebDAV für eine Kopie außerhalb des Rechners.

**Am besten, wenn:** du an einem einzigen Windows-PC spielst und versionierte lokale Backups fast ohne Einrichtung willst, dazu einen einfachen Umzug auf einen neuen Rechner.

**Wo es aufhört:** Es synchronisiert nicht zwischen Geräten: Der zweite PC bekommt deine Spielstände, wenn du ein Archiv exportierst und dort importierst. Nur Windows.

## SaveState

SaveState von Matteo842 ist ein Backup-Manager unter GPL-3.0 für Windows und Linux, mit einer separaten Android-App. Du ziehst eine Launcher-Verknüpfung darauf, und es findet den Spielstand-Ordner; die Emulator-Unterstützung geht tief: RetroArch, Dolphin, PCSX2-Memory-Cards, xemu und über zwanzig weitere. Backups sind komprimierte `.zip`-Archive mit Rotation, und ein optionaler Cloud-Sync kopiert sie zu Google Drive, WebDAV, FTP, einer Netzwerkfreigabe oder Git.

**Am besten, wenn:** du ein freundliches Backup-Tool mit breiter Emulator-Abdeckung willst und es dir nichts ausmacht, auf dem anderen Rechner von Hand wiederherzustellen.

**Wo es aufhört:** Es dreht sich um Backup-Archive, nicht um den lebenden Spielstand. Der Sync bewegt diese Archive zwischen Rechnern, aber eines davon in den Ordner des Spiels auf der anderen Seite zu bringen, bleibt eine Wiederherstellung, die du selbst startest. Kein macOS.

## EmuSync

EmuSync ist ein Sync-Tool unter GPL-3.0 für PC- und Emulator-Spielstände, gebaut vor allem für einen Windows-PC und ein Steam Deck. Du ordnest den Speicherort jedes Spiels auf jedem Gerät einem einzigen Sync-Punkt in deinem eigenen Google Drive, Dropbox oder OneDrive oder in einem freigegebenen Ordner zu, und es synchronisiert automatisch oder auf Knopfdruck. Ein Decky-Plugin zeigt den Status im Spielmodus.

**Am besten, wenn:** du zwischen einem Windows-PC und einem Deck spielst und deine Spielstände in einem Cloud-Speicher haben willst, den du schon hast.

**Wo es aufhört:** Der Cloud-Speicher hält den aktuellen Zustand, und Backup und Wiederherstellung sind lokal auf jedem Gerät statt einer gemeinsamen Historie. macOS ist nicht getestet, und die letzte Version ist vom Februar 2026.

## Aletheia

Eines der jüngeren, AGPL, und es geht genau die Stelle an, die alle anderen halb abdecken: die Launcher. Heroic, itch.io, Lutris, Steam, GOG Galaxy und Xbox, unter Windows, Linux und macOS.

**Am besten, wenn:** deine Bibliothek über Launcher verteilt ist, die andere Tools schlecht erkennen — vor allem Xbox/Game Pass und Heroic.

**Wo es aufhört:** ein junges Projekt mit bewusst engem Zuschnitt. Sichern und Wiederherstellen ist der Funktionsumfang; eine versionierte Cloud steht nicht dahinter.

## SaveSync

Das kommerzielle, auf Steam als Einmalkauf, mit Fokus auf Windows. Sein Kniff: Es zielt gar nicht auf dich-an-zwei-PCs, sondern auf Koop. Spielstände landen in privaten, nicht gelisteten Steam-Workshop-Einträgen, damit ein Freund deine Valheim- oder Factorio-Welt ziehen kann, und LAN-Sync gibt es auch.

**Am besten, wenn:** dein Problem "mein Freund hostet und ich brauche seinen Spielstand" lautet und nicht "meine Spielstände sollen mir folgen".

**Wo es aufhört:** Closed Source, Windows, an Steam als Transportweg gebunden, und eine Liste unterstützter Koop-Spiele statt allem, was du besitzt.

## Tachyon

Der neueste Name hier, als kostenlose Beta für Windows. Tachyon erkennt Spielstände von über 5.000 PC-Spielen, synchronisiert sie über die eigene Cloud und führt einen Versionsverlauf mit mehreren „Timelines“.

**Am besten, wenn:** du unter Windows bist, etwas willst, das sofort funktioniert, und dich eine Beta nicht stört.

**Wo es aufhört:** vorerst nur Windows (Linux und macOS sind als „kommt bald“ angekündigt), also noch kein Steam Deck; kein veröffentlichter Quellcode; kein eigener Server möglich. Preise nach der Beta sind nicht angekündigt.

## Steam-Deck-Plugins (Decky)

Auf dem Deck selbst übernehmen ein paar Decky-Loader-Plugins einen Teil der Arbeit aus dem Spielmodus heraus. Decky Cloud Save synchronisiert Ordner über rclone mit OneDrive, Google Drive oder Dropbox, nachdem ein Spiel geschlossen wurde; der Autor pflegt es nicht mehr und verweist auf zwei Nachfolger, Decky Ludusavi (eine Spielmodus-Oberfläche für Ludusavi) und SDH-GameSync (ein Fork, der Spielstände beim Start jedes Spiels herunter- und beim Beenden hochlädt). Steamback behält lokal die zehn neuesten Snapshots von Spielen, die Steam Cloud nutzen, aufgenommen beim Beenden.

**Am besten, wenn:** du vor allem auf dem Deck spielst und etwas willst, das du bedienen kannst, ohne den Spielmodus zu verlassen.

**Wo es aufhört:** Sie laufen nur auf dem Deck, und der Teil zwischen den Geräten hängt an etwas anderem: rclone und deinem Cloud-Speicher, Ludusavi oder Steam Cloud selbst. Die Snapshots von Steamback bleiben auf dem Deck und decken nur Steam-Cloud-Spiele ab.

## Eine Anmerkung zu EmuDeck

EmuDeck kommt in diesen Gesprächen auf und ist kein Konkurrent im üblichen Sinn: Es ist ein Installer und Konfigurator für Emulatoren auf dem Steam Deck, und der angebotene Sync ist eine Bequemlichkeit, die an diese Aufgabe angeflanscht ist (Rclone gegen ein Cloud-Laufwerk, nur für Emulator-Spielstände). Es überschneidet sich mit den Tools oben, ohne dasselbe zu sein: EmuDeck richtet deine Emulatoren ein, die Tools hier kümmern sich um die Spielstände der ganzen Bibliothek. Manche betreiben EmuDeck neben einem davon, und das ist ein sinnvolles Setup, kein doppeltes.

## Hoard

Hoard sichert jeden Spielstand, sobald das Spiel ihn fertig geschrieben hat, und hält ihn auf jedem Rechner synchron, auf dem du spielst. Die Engine läuft als Hintergrunddienst, `hoardd`, ohne Fenster, und arbeitet deshalb auch im Spielmodus von SteamOS weiter. Sie wartet, bis der Ordner ein paar Sekunden ruhig ist, lädt also nie eine halb geschriebene Datei hoch; sie macht beim Beenden noch eine Kopie; und sie tauscht nie den Spielstand eines laufenden Spiels aus: Was von einem anderen Rechner kommt, wartet, bis du das Spiel schließt.

- **Der andere Rechner darf aus sein.** Der neueste Spielstand wartet auf dem Server, also bekommt ihn das Deck, das du nächste Woche in die Hand nimmst, ohne dass der PC läuft. Peer-to-peer-Tools brauchen beide Geräte gleichzeitig wach.
- **Jede Version bleibt.** Jeder Spielstand ist eine Version, zu der du in der App oder mit `hoard restore` zurückkannst, auch nach einem Festplattenausfall oder einer Neuinstallation. Hoard One behält jede Version dauerhaft; auf deinem eigenen Server entscheidest du, wie viel Historie bleibt.
- **Nur was sich ändert, wird übertragen.** Uploads schicken nur geänderte Dateien, mit zstd komprimiert, und Versionen teilen sich alles, was gleich bleibt: Zehn Versionen eines 2-GB-Spielstands, an dem sich wenig ändert, belegen etwa 2 GB, nicht 20.
- **SHA-256 beim Hoch- und beim Herunterladen.** Beschädigung wird erkannt, bevor sie einen guten Spielstand überschreiben kann, und Hoard behält eine Kopie von allem, was es ersetzt.
- **Kostenlos und Open Source, mit eigenem Server, wenn du willst.** App, CLI und Server stehen unter AGPL-3.0. Hoard One, der verwaltete Server, den wir allen empfehlen, ist kostenlos mit 2 GB und drei Geräten, und es gibt nichts einzurichten. Oder du startest `hoard-server` mit Docker auf einem PC, einem NAS oder einem Raspberry Pi, auf lokaler Platte oder beliebigem S3-kompatiblem Speicher: kein Konto bei uns, keine Telemetrie zu uns, kein Kontingent von uns.
- **Windows, Linux, macOS und SteamOS**, mit derselben Desktop-App auf allen vieren und einer CLI für Server und Rechner ohne Bildschirm.
- **Erkennung für die ganze Bibliothek:** Steam, Epic, GOG, Proton- und Wine-Präfixe, Tausende Spiele über die Community-Spielstand-Datenbank und Voreinstellungen für 19 Emulatoren, von PCSX2, Dolphin und RPCS3 bis Ryujinx, Cemu, DuckStation und RetroArch.

## Das Detail, an dem Steam Deck ↔ PC hängt

Gut zu wissen, egal welches Tool du nimmst. Der Cloud-Spielstand eines Steam-Spiels liegt in `<AppID>/remote/`, und der Ordner *darüber* enthält `remotecache.vdf`, Erfolgsstände, Statistiken und Spielzeitzähler — alles Dinge, die sich zwischen Deck und Desktop berechtigterweise unterscheiden.

Synchronisiere den übergeordneten Ordner, und du hast einen Dauerkonflikt zwischen zwei Maschinen, die sich über keinen einzigen Spielstand uneinig waren. Hoard verfolgt `remote/`, nicht den Elternordner. Jedem Tool, dem du einen Ordner von Hand zuweist, kann man dasselbe beibringen — und es ist das Erste, was man prüft, wenn ein Sync-Setup ohne sichtbaren Grund ständig Konflikte meldet.

## Was Hoard nicht macht

- **Ohne Server synchronisieren.** Hoard synchronisiert immer über einen: Hoard One, wenn du nichts betreiben willst, oder deinen eigenen. Genau das erlaubt, dass das andere Gerät aus ist, und hinterlässt eine Kopie, die beide Rechner überlebt. Willst du unter keinen Umständen einen Server, sind OpenSave, Syncthing oder Ludusavi dafür gebaut.
- **Koop-Welten teilen.** Eine Welt an einen Freund weiterzugeben, dafür ist SaveSync gebaut; Hoard kümmert sich um deine eigenen Spielstände.
- **Zwei auseinandergelaufene Spielstände zusammenführen.** Kein Tool kann zwei Spielverläufe zu einem vereinen. Hoard behält beide als Versionen, auf jedem Rechner, und du wählst.

## Der Unterschied zwischen Hoard One und Selbsthosten

Vergleiche zu Hoard werfen diese beiden fast immer in einen Topf, und das Ergebnis stimmt dann nicht. Deshalb klar gesagt:

- **Hoard One** ist die verwaltete Variante, die wir allen empfehlen: du meldest dich an, und deine Stände liegen auf unseren Servern in der EU.
- **Ein selbst gehostetes Hoard gehört vollständig dir.** Du betreibst `hoard-server` auf deinem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir sehen weder Spielstand noch Spieltitel noch E-Mail-Adresse, weil davon nichts bei uns ankommt. Würde Hoard One morgen abgeschaltet, liefe ein selbst gehostetes Setup unverändert weiter.

Derselbe Code, dieselbe Erkennung. Es ändert sich, wem der Speicher gehört, und eine Voreinstellung: Ein selbst gehosteter Server dünnt alte Versionen mit der Zeit aus, solange du das nicht abschaltest. Ein Detail der Genauigkeit halber: dein eigener Server hat sehr wohl eigene Zugänge — einen Benutzer und ein Token je Gerät — aber die liegen in deiner Datenbank, nicht in unserer.

## Die Tabelle

| Tool | Automatischer Sync | Funktioniert, wenn das andere Gerät aus ist | Wo die Spielstände liegen | Historie | Plattformen | Lizenz |
|---|---|---|---|---|---|---|
| **Hoard** | Ja, bei jedem Speichern | Ja | Hoard One oder eigener Server (Platte oder S3-kompatibel) | Jede Version, dedupliziert | Win · Linux · macOS · Deck | AGPL-3.0, kostenlos |
| **Ludusavi** | Manuell, oder Rclone, das du einrichtest | Nur mit einem Rclone-Remote | Lokal, dazu dein Rclone-Remote | Versionierte lokale Backups | Win · Linux · macOS | MIT, kostenlos |
| **Syncthing** | Ja, fortlaufender Spiegel | Nein, beide müssen laufen | Nur auf deinen Geräten | Versionierung pro Datei | Alles | MPL-2.0, kostenlos |
| **OpenSave** | Ja, peer-to-peer | Nein, außer du richtest einen Cloud-Spiegel ein | Deine Geräte, optionaler Cloud-Spiegel | Snapshots und Zweige | Win · Linux · macOS · Deck | MIT, kostenlos |
| **OpenCloudSaves** | Über deinen Cloud-Speicher | Ja, über den Cloud-Speicher | OneDrive / Drive / Dropbox / Nextcloud | Was der Cloud-Speicher behält | Win · Linux · macOS | Open Source, archiviert |
| **Game Backup Monitor** | Nein | Kein Sync | Lokale 7-Zip-Archive | Nummerierte Backups | Windows · Linux | GPL-3.0, kostenlos |
| **GameSave Manager** | Über deinen Cloud-Speicher (Sync & Link) | Ja, über den Cloud-Speicher | Lokal, dazu Dropbox / OneDrive | Was der Cloud-Speicher behält | Windows | Kostenlos, Closed Source |
| **Game Save Manager (GSM)** | Nein, Export und Import | Kein Sync | Lokale Backups | Rotierende lokale Backups | Windows | GPL-3.0, kostenlos |
| **SaveState** | Optional, für die Backup-Archive | Ja, über deinen Speicher | Lokale Zips, dazu Drive / WebDAV / FTP / SMB / Git | Rotierende Archive | Windows · Linux | GPL-3.0, kostenlos |
| **EmuSync** | Ja, über deinen Cloud-Speicher | Ja, über den Cloud-Speicher | Drive / Dropbox / OneDrive oder ein freigegebener Ordner | Lokale Backups | Windows · Linux · Deck | GPL-3.0, kostenlos |
| **Aletheia** | Backup und Wiederherstellung pro Launcher | Kein Sync | Dein Speicher | Backups | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Ja, auch mit Freunden | Ja, über Steam | Private Steam-Workshop-Einträge | Je nach App | Windows | Kostenpflichtig, Closed Source |
| **Tachyon** | Ja, über die eigene Cloud | Ja | Die Cloud von Tachyon | Versionen und Zeitlinien | Windows (Beta) | Kostenlose Beta, kein veröffentlichter Quellcode |

## Also welches

Wenn Backups und Sync einfach passieren sollen, zwischen PCs und einem Steam Deck, mit jeder Version, mit einem anderen Rechner, der aus sein darf, und mit der Möglichkeit, dass alles dir gehört, dann ist das Hoard. [Lade es herunter](/download), melde dich bei Hoard One an oder lies vorher, [wie du es mit Docker selbst hostest](/guides/self-host-hoard). Kostenlos ist es so oder so.

Für engere Aufgaben: ein einzelner Windows-PC und nur lokale Archive, Game Backup Monitor oder GSM. Lokale Backups mit voller Kontrolle und nirgends ein Server, Ludusavi. Unter keinen Umständen ein Konto, mit Geräten, die meist gleichzeitig laufen, OpenSave oder Syncthing. Spielstände, die in einem Drive-Ordner liegen müssen, für den du schon zahlst, Ludusavi mit einem Rclone-Remote oder EmuSync; OpenCloudSaves hat das gemacht, ist aber archiviert. Eine Koop-Welt mit Freunden teilen, SaveSync.

Wenn du von OpenCloudSaves kommst und eigentlich deine Spielstände in einem Speicher haben wolltest, den du kontrollierst, und nicht unbedingt in Drive, dann macht ein selbst gehostetes Hoard genau das, mit Historie obendrauf: Es schreibt auf lokale Platte oder in jeden S3-kompatiblen Bucket, etwa Backblaze B2 oder MinIO. Wägst du es gegen Ludusavi ab, gibt es einen [ausführlicheren Ludusavi-Vergleich](/guides/ludusavi-alternative).

## Direkte Vergleiche

Jeder davon geht tiefer als der Abschnitt oben, samt der Punkte, an denen das andere Werkzeug gewinnt:

- [Hoard gegen Ludusavi](/guides/ludusavi-alternative)
- [Hoard als Steam-Cloud-Alternative](/guides/steam-cloud-alternative)
- [Peer-to-peer gegen einen eigenen Server](/guides/opensave-alternative)
- [Syncthing für Spielstände: was bricht](/guides/syncthing-game-saves)
- [Hoard gegen GameSave Manager und die anderen Game Save Manager](/guides/game-save-manager-alternative)
- [OpenCloudSaves ersetzen, jetzt wo es archiviert ist](/guides/opencloudsaves-alternative)
- [Eine kostenlose Alternative zu SaveSync](/guides/savesync-alternative)

<!-- faq -->

## Häufige Fragen

### Welches dieser Werkzeuge führt eine Versionshistorie?

Hoard behält jeden Spielstand als Version, zu der du zurückkannst, auf jedem Rechner. Ludusavi, GSM und SaveState führen versionierte oder rotierende lokale Backups, und OpenSave behält Snapshots auf deinen Geräten. Die meisten übrigen synchronisieren oder kopieren den aktuellen Zustand — ein beschädigter Spielstand wandert damit getreulich auf die andere Maschine.

### Welches funktioniert ohne Server und ohne Konto?

Ludusavi mit lokalen Backups, und jedes Peer-to-peer-Werkzeug. Hoard zählt ebenfalls dazu, wenn du selbst hostest: kein Konto bei uns, und nichts, was über unsere Server läuft.

### Welches synchronisiert, wenn mein anderes Gerät aus ist?

Jedes Tool, das den Spielstand woanders als auf deinen Geräten ablegt: Hoard, über Hoard One oder deinen eigenen Server, und die Tools, die in einen Cloud-Speicher schreiben, etwa EmuSync oder Ludusavi mit Rclone. Peer-to-peer-Sync (OpenSave ohne seinen Spiegel, Syncthing) braucht beide Geräte gleichzeitig an.

### Welches deckt Spiele ab, die nicht auf Steam sind?

Alle Spielstand-Verwalter hier, denn sie finden Stände über dieselbe Community-Datenbank statt über einen Store. Die Ausnahme ist Steam Cloud: sie deckt nur Steam-Spiele ab, deren Entwickler sie aktiviert hat.

### Muss ich mich für eines entscheiden?

Nein, und viele tun es nicht. Ein lokales Backup-Werkzeug und ein Sync-Werkzeug lösen unterschiedliche Hälften des Problems. Die einzige Regel: richte niemals eines auf den Backup-Ordner des anderen, sonst synchronisierst du einen veralteten Spiegel statt deines echten Spielstands.

### Was ist das eine Detail, an dem die meisten Eigenbau-Setups scheitern?

Den Ordner über `<AppID>/remote/` in Steams `userdata` zu synchronisieren. Der übergeordnete Ordner enthält `remotecache.vdf` sowie Dateien für Erfolge und Spielzeit, die sich pro Rechner unterscheiden sollen — jeder Start sieht dann nach einem Konflikt aus, obwohl sich kein Stand bewegt hat.
