---
title: "So sicherst und synchronisierst du Emulator-Spielstände (RetroArch, Dolphin, PCSX2)"
description: "Emulator-Spielstände zwischen PCs und Steam Deck sichern und syncen: RetroArch, Dolphin, PCSX2, DuckStation und mehr, mit Verlauf und allen Speicherorten."
order: 6
updated: 2026-10-01
---

Emulator-Spielstände gehen leicht verloren: Speicherdateien und Savestates liegen in verstreuten Ordnern, und eine Neuinstallation oder ein neuer PC kann Jahre an Fortschritt löschen. Hoard sichert sie automatisch und hält sie zwischen deinen Rechnern synchron, Steam Deck eingeschlossen.

## Emulatoren, mit denen Hoard funktioniert

Hoard verarbeitet die üblichen Emulator-Speicherdateien (`.srm`, `.sav`, Memory Cards, Speicherordner pro Spiel) und Savestates. Wo diese Emulatoren speichern, weiß es von Haus aus:

- **Sony:** PCSX2 (PS2), DuckStation (PS1), PPSSPP (PSP), RPCS3 (PS3), shadPS4 (PS4), Vita3K (PS Vita)
- **Nintendo:** Dolphin (GameCube / Wii), Cemu (Wii U), Ryujinx, yuzu, Eden, Suyu, Citron und Sudachi (Switch), Citra / Azahar (3DS), melonDS (DS), mGBA (GBA), Project64 (N64)
- **Weitere:** RetroArch (Multisystem), xemu (Xbox), Flycast (Dreamcast)

Weil Hoard Speicherordner mit derselben Community-Datenbank findet, auf der auch Ludusavi aufbaut, werden viele Pfade automatisch erkannt. Für alles Eigene kannst du Hoard von Hand auf einen Ordner zeigen lassen.

## Emulator-Backups einrichten

1. **Installiere Hoard** für Windows, macOS oder Linux und melde dich an.
2. Öffne die **Bibliothek** und füge deinen Emulator hinzu, oder ergänze seinen Stände-/Savestate-Ordner manuell, falls du den Standardort geändert hast.
3. Lass den **Automatikmodus** an. Hoard sichert nach jeder Sitzung und führt eine versionierte Historie.
4. Installiere Hoard mit demselben Konto auf deinen anderen PCs, um diese Stände überall zu synchronisieren — siehe [Spielstände zwischen PCs synchronisieren](/guides/sync-game-saves-across-pcs).

## Ludusavi für Emulatoren?

Ludusavi kann Emulator-Spielstände ebenfalls lokal sichern und ist dafür eine großartige kostenlose Option. Wenn diese Stände zusätzlich automatisch zwischen Rechnern synchronisiert werden und einen Versionsverlauf in der Cloud haben sollen, ohne Rclone einzurichten, hilft Hoard — lies den vollständigen [Vergleich Ludusavi vs. Hoard](/guides/ludusavi-alternative).

## Cloud-Saves für jeden Emulator

Keiner der eigenständigen Emulatoren unten synchronisiert Spielstände von sich aus zwischen Rechnern: Die Stände sind einfache Dateien auf deiner Platte. Das ist eine gute Nachricht, denn jedes Werkzeug, das den richtigen Ordner beobachtet, kann sie mitnehmen. Hier steht, wo jeder speichert. „Steam Deck“ meint die Flatpak-Version aus dem Discover-Store.

### PCSX2 Cloud-Saves (PS2)

PCSX2 schreibt Memory Cards (`.ps2`-Dateien) nach `memcards/`:

- Windows: `Dokumente\PCSX2\memcards`
- Linux: `~/.config/PCSX2/memcards`
- Steam Deck: `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`

Eine Memory Card enthält die Stände aller Spiele, die du darauf gespielt hast, und reist deshalb als ein Stück: Eine ältere Version wiederherzustellen setzt die ganze Karte zurück, nicht ein einzelnes Spiel.

### Dolphin Cloud-Saves (GameCube und Wii)

GameCube-Stände liegen unter `GC/` (Memory-Card-Abbilder oder ein Ordner pro Karte), Wii-Stände im emulierten NAND unter `Wii/`:

- Windows: `Dokumente\Dolphin Emulator\GC` und `\Wii`
- Linux: `~/.local/share/dolphin-emu/GC` und `/Wii`
- Steam Deck: `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/`

### DuckStation Cloud-Saves (PS1)

DuckStation legt Memory Cards in `memcards/` ab und erstellt standardmäßig für jedes Spiel eine eigene Karte, was sich sehr gut synchronisieren lässt:

- Windows: `Dokumente\DuckStation\memcards` (neuere Versionen nutzen `%LOCALAPPDATA%\DuckStation\memcards`)
- Linux: `~/.local/share/duckstation/memcards`
- Steam Deck: `~/.var/app/org.duckstation.DuckStation/`, unter `data/` oder `config/`

### RetroArch Save-Sync

RetroArch trennt `saves/` (die Spielstände) von `states/` (Savestates). Hoard verfolgt den Spielstand-Ordner; füge `states/` als eigenen Eintrag hinzu, wenn du mit Savestates spielst:

- Windows: `%APPDATA%\RetroArch`, oder neben `retroarch.exe` bei einer portablen Installation
- Linux: `~/.config/retroarch`
- Steam Deck: `~/.var/app/org.libretro.RetroArch/config/retroarch`, oder `~/Emulation/saves/retroarch`, wenn du es mit EmuDeck eingerichtet hast

RetroArch hat außerdem ein eingebautes Cloud Sync, das mit einem WebDAV-Server spricht, den du bereitstellst. Das ist eine vernünftige Wahl, wenn du nur RetroArch nutzt und schon WebDAV betreibst. Hoard braucht kein WebDAV, führt einen Versionsverlauf zum Zurücksetzen und deckt auch die eigenständigen Emulatoren ab.

### PPSSPP (PSP)

Spielstände landen in `PSP/SAVEDATA`, Savestates in `PSP/PPSSPP_STATE`:

- Windows: `Dokumente\PPSSPP\PSP\SAVEDATA`, oder `memstick\PSP\SAVEDATA` neben der ausführbaren Datei bei einer portablen Installation
- Linux: `~/.config/ppsspp/PSP/SAVEDATA`
- Steam Deck: `~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA`

### RPCS3 (PS3)

Die Stände liegen in `dev_hdd0/home/00000001/savedata`, unter Windows im RPCS3-Ordner, unter Linux und auf dem Steam Deck in `~/.config/rpcs3/`.

### Switch-Emulatoren: Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

Ryujinx speichert in `bis/user/save` (unter `%APPDATA%\Ryujinx` oder `~/.config/Ryujinx`). Die yuzu-Familie nutzt `nand/user/save` im eigenen Ordner unter `%APPDATA%` oder `~/.local/share`.

Hier steckt eine Falle. Der yuzu-Baum sieht so aus: `save/<Konto>/<Profil>/<Titel-ID>/`, und die Profil-ID wird beim ersten Start des Emulators erzeugt, ist also bei jeder Installation anders. Synchronisierst du den ganzen `save/`-Ordner zwischen zwei Rechnern, landet auf jedem das Profil des anderen neben dem eigenen, und kein Spiel sieht den Fortschritt des anderen. Hoard steigt stattdessen bis in den Ordner jedes einzelnen Spiels hinab, sodass derselbe Titel auf beiden Rechnern zusammenfindet, egal wie das Profil heißt.

### Citra und Azahar (3DS)

Die Stände liegen tief unter `sdmc/Nintendo 3DS/<id0>/<id1>/title/…`, und `id0`/`id1` stammen aus den Schlüsseln der emulierten Konsole, unterscheiden sich also ebenfalls pro Installation. Hoard löst das wie beim Switch-Baum: ein Eintrag pro Spiel, zwischen den Rechnern zugeordnet.

### Der Rest

- **Cemu (Wii U):** `mlc01/usr/save`, unter `%APPDATA%\Cemu` oder `~/.local/share/Cemu`.
- **shadPS4 (PS4):** `savedata`, unter `%APPDATA%\shadPS4` oder `~/.local/share/shadPS4`.
- **Vita3K (PS Vita):** `ux0/user/00/savedata` in seinem Datenordner.
- **mGBA, melonDS und die meisten Emulatoren aus der Modulzeit:** eine `.sav` neben dem ROM, sofern du nichts anderes eingestellt hast. Füge die Stände aus dem ROM-Ordner von Hand hinzu.

## Emulator-Spielstände auf dem Steam Deck

Auf dem Steam Deck kommen die Emulatoren meist als Flatpak, ihre Ordner liegen also unter `~/.var/app/<id>/` statt in den üblichen `~/.config` oder `~/.local/share`. EmuDeck sammelt alles unter `~/Emulation/saves/`, ein Ordner pro Emulator. So oder so: Ordner einmal hinzufügen, Hoard beobachtet ihn.

Was auf einem Handheld zählt: Die Engine von Hoard läuft als Hintergrunddienst und sichert deshalb, wenn du ein Spiel im Spielmodus beendest, ganz ohne offenes Fenster. Nimmst du das Deck nach einer Session am Desktop in die Hand, ist der Spielstand schon da.

## Spielstand und Savestate sind nicht dasselbe

Es lohnt sich, beides zu trennen, weil es sich beim Reisen unterschiedlich verhält:

- Ein **Spielstand** (`.srm`, eine Memory Card, ein `SAVEDATA`-Ordner) ist die eigene Speicherung des Spiels, geschrieben von der emulierten Konsole. Er wandert ohne Murren zwischen Rechnern und Emulator-Versionen.
- Ein **Savestate** ist ein Abbild des Emulator-Speichers. Er hängt am Emulator-Build und oft am genauen Core, sodass ein State aus einer Version in einer anderen eventuell nicht lädt.

Hoard sichert beides. Wundere dich nur nicht, wenn sich ein State von einem aktualisierten Rechner auf einem veralteten nicht öffnen lässt — halte deine Emulatoren auf gleichen Versionen und verlass dich für Wichtiges auf Spielstände.

## Ein Emulator, viele Spiele

Ein Emulator ist ein einziger Prozess, der Dutzende Titel beherbergt, und genau das macht Emulator-Stände für ein Werkzeug schwierig, das in „dem laufenden Spiel“ denkt. Hoard hält die Titel auseinander, statt den ganzen Emulator als einen Klumpen zu behandeln, sodass jedes Spiel seine eigene Historie bekommt statt eines gemeinsamen Haufens, der sich bei jedem Start ändert. Geht ein Stand doch kaputt, kannst du [ihn auf eine frühere Version zurücksetzen](/guides/restore-a-game-save).

## Emulator-Stände ohne unsere Server

Alles hier funktioniert genauso gegen deinen eigenen Server: `hoard-server` starten, die App darauf zeigen lassen, und deine Stände gehen von deinem Rechner auf deine Platte. Kein Konto bei uns, keine Telemetrie an uns, nichts über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

## Tipp

Savestates hängen an einer bestimmten Emulator-Version. Halte deine Emulatoren auf allen PCs gleich aktuell, damit ein synchronisierter State überall sauber lädt.

<!-- faq -->

## Häufige Fragen

### Sichert Hoard auch meine ROMs?

Nein. Es verfolgt Speicherordner, keine Spieldateien. ROMs sind groß, ändern sich nicht, und du hast sie schon — da gibt es nichts zu versionieren.

### Haben PCSX2, Dolphin oder DuckStation eingebaute Cloud-Saves?

Nein. Sie schreiben Spielstände in lokale Ordner und überlassen das Synchronisieren dir. Richte ein Sync-Werkzeug auf die oben genannten Ordner, und die Stände folgen dir von Rechner zu Rechner.

### Hat RetroArch einen Cloud-Sync?

Ja, ein eingebautes Cloud Sync, das einen WebDAV-Server braucht, den du selbst betreibst oder mietest. Hoard ist die Alternative, wenn du kein WebDAV einrichten willst, einen Versionsverlauf zum Zurücksetzen möchtest oder auch eigenständige Emulatoren nutzt.

### Funktioniert es auf dem Steam Deck im Spielmodus?

Ja. Die Engine läuft als Hintergrunddienst, Spielstände werden also beim Beenden eines Spiels gesichert, ohne offenes Fenster. Flatpak- und EmuDeck-Ordner funktionieren wie jeder andere.

### Mein Emulator ist portabel installiert. Geht das?

Ja. Füge den Ordner neben der ausführbaren Datei von Hand hinzu, und Hoard verfolgt ihn wie jeden anderen Speicherort. Das ist auf Handhelds das übliche Setup.

### Kann ich Savestates zwischen zwei PCs synchronisieren?

Ja, und Hoard macht das. Ob ein State lädt, hängt davon ab, dass die Emulatoren auf beiden Rechnern dieselbe Version haben — eine Grenze des Emulators, nicht der Synchronisierung. Spielstände haben dieses Problem nicht.

### Funktioniert es mit einem Emulator, der nicht auf der Liste steht?

Höchstwahrscheinlich. Die Erkennung deckt die gängigen automatisch ab, und alles andere fügst du hinzu, indem du Hoard auf seinen Speicherordner zeigst.

### Ändert Selbst-Hosting etwas für Emulatoren?

Nein. Gleiche Erkennung, gleiche Versionen, gleiche Synchronisierung. Nur der Speicher gehört dir.
