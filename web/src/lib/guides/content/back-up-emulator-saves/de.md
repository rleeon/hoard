---
title: "So sicherst und synchronisierst du Emulator-Spielstände (RetroArch, Dolphin, PCSX2)"
description: "Sichere und synchronisiere deine Emulator-Speicherdateien und Savestates über mehrere PCs — RetroArch, Dolphin, PCSX2, DuckStation und mehr — automatisch mit Hoard."
order: 6
updated: 2026-09-01
---

Emulator-Stände gehen leicht verloren: Speicherdateien und Savestates liegen in verstreuten Ordnern, und eine Neuinstallation oder ein neuer PC kann Jahre an Fortschritt löschen. Hoard sichert sie automatisch und hält sie über mehrere Geräte synchron.

## Emulatoren, mit denen Hoard funktioniert

Hoard verarbeitet gängige Emulator-Speicherdateien (`.srm`, `.sav`, Memory Cards) und Savestates der beliebten Emulatoren, darunter:

- **RetroArch** — Stände und Savestates pro Core
- **Dolphin** (GameCube / Wii) — Memory Cards und GCI-Dateien
- **PCSX2** (PS2) — Memory Cards
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA** und mehr

Da Hoard Speicherordner mit derselben Community-Datenbank findet, die auch Ludusavi antreibt, werden viele Emulator-Pfade automatisch erkannt. Für alles Eigene kannst du Hoard von Hand auf einen Ordner verweisen.

## Emulator-Backups einrichten

1. **Installiere Hoard** für Windows, macOS oder Linux und melde dich an.
2. Öffne die **Bibliothek** und füge deinen Emulator hinzu, oder ergänze seinen Stände-/Savestate-Ordner manuell, falls du den Standardort geändert hast.
3. Lass den **Automatikmodus** an. Hoard sichert nach jeder Sitzung und führt eine versionierte Historie.
4. Installiere Hoard mit demselben Konto auf deinen anderen PCs, um diese Stände überall zu synchronisieren — siehe [Spielstände über PCs synchronisieren](/guides/sync-game-saves-across-pcs).

## Ludusavi für Emulatoren?

Ludusavi kann Emulator-Stände ebenfalls lokal sichern und ist dafür eine großartige kostenlose Option. Wenn diese Emulator-Stände zusätzlich automatisch zwischen Geräten synchronisieren und eine Cloud-Versionshistorie behalten sollen, ohne Rclone zu konfigurieren, hilft Hoard — lies den vollständigen [Vergleich Ludusavi vs. Hoard](/guides/ludusavi-alternative).

## Wo die einzelnen Emulatoren ihre Stände ablegen

Nützlich zu wissen, denn eine portable Installation legt all das ganz woanders ab:

- **RetroArch** — `saves/` und `states/` im Konfigurationsordner: `%APPDATA%\RetroArch` unter Windows, `~/.config/retroarch` unter Linux.
- **Dolphin** — Memory Cards unter `GC/`, Wii-Stände im emulierten NAND, in `Dokumente\Dolphin Emulator` oder `~/.local/share/dolphin-emu`.
- **PCSX2** — `memcards/`, unter `Dokumente\PCSX2` oder `~/.config/PCSX2`.
- **DuckStation** — `memcards/` und `savestates/` im eigenen Datenordner.
- **PPSSPP** — `PSP/SAVEDATA` für Stände, `PSP/PPSSPP_STATE` für Savestates.
- **RPCS3** — `dev_hdd0/home/00000001/savedata`.
- **Cemu** — `mlc01/usr/save`.
- **mGBA und die meisten eigenständigen Cores** — eine `.sav` neben der ROM, sofern nicht anders eingestellt.

Eine **portable Installation** — auf Handhelds und USB-Sticks der Normalfall — legt all das stattdessen neben die ausführbare Datei. Wenn das dein Setup ist, richte Hoard auf diesen Ordner, und er wird wie jeder andere Spielstand verfolgt.

## Spielstand und Savestate sind nicht dasselbe

Die Unterscheidung lohnt sich, denn beim Umzug verhalten sie sich verschieden:

- Ein **Spielstand** (`.srm`, eine Memory Card, ein `SAVEDATA`-Ordner) ist der eigene Stand des Spiels, geschrieben von der emulierten Konsole. Er wandert klaglos zwischen Rechnern und Emulatorversionen.
- Ein **Savestate** ist ein Abbild des Emulatorspeichers. Er hängt an genau diesem Build und oft am exakten Core, ein Savestate der einen Version kann sich in einer anderen also weigern zu laden.

Hoard sichert beides. Wundere dich nur nicht, wenn ein Savestate von einer aktualisierten Maschine auf einer veralteten nicht aufgeht: halte die Emulatorversionen gleich und verlass dich für Wichtiges auf Spielstände.

## Ein Emulator, viele Spiele

Ein Emulator ist ein einzelner Prozess, der Dutzende Titel beherbergt — genau das macht Emulator-Stände schwierig für ein Werkzeug, das in "dem laufenden Spiel" denkt. Hoard hält die Titel auseinander, statt den ganzen Emulator als einen Klumpen zu behandeln, sodass jedes Spiel seine eigene Historie bekommt und nicht einen gemeinsamen Haufen, der sich bei jedem Start von irgendetwas ändert.

## Emulator-Stände ohne unsere Server

All das funktioniert genauso gegen deinen eigenen Server: `hoard-server` betreiben, die App darauf richten, und deine Stände gehen von deiner Maschine auf deine Platte. Kein Konto bei uns, keine Telemetrie zu uns, nichts über unsere Server. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

## Tipp

Savestates sind an eine bestimmte Emulator-Version gebunden. Halte deine Emulatoren über alle PCs hinweg einheitlich aktuell, damit ein synchronisierter Savestate überall sauber lädt.

<!-- faq -->

## Häufige Fragen

### Sichert Hoard auch meine ROMs?

Nein. Es verfolgt Speicherordner, keine Spieldateien. ROMs sind groß, sie ändern sich nicht, und du hast sie bereits — da gibt es nichts zu versionieren.

### Mein Emulator ist eine portable Installation. Geht das?

Ja. Füge den Ordner neben der ausführbaren Datei von Hand hinzu, dann verfolgt Hoard ihn wie jeden anderen Speicherort. Auf Handhelds ist das der Normalfall.

### Kann ich Savestates zwischen zwei PCs synchronisieren?

Kannst du, und Hoard tut es. Ob ein Savestate lädt, hängt davon ab, dass die Emulatoren auf beiden Maschinen dieselbe Version haben — eine Grenze des Emulators, nicht der Synchronisierung. Spielstände haben das Problem nicht.

### Klappt es mit einem Emulator, der nicht auf der Liste steht?

Ziemlich sicher ja. Die gängigen werden automatisch erkannt, alles andere fügst du hinzu, indem du Hoard auf seinen Speicherordner richtest.

### Ändert Selbsthosten etwas für Emulatoren?

Nein. Gleiche Erkennung, gleiche Versionen, gleiche Synchronisierung. Nur der Speicher gehört dir.
