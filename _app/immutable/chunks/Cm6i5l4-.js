var Pe=Object.defineProperty;var Ce=(n,e,a)=>e in n?Pe(n,e,{enumerable:!0,configurable:!0,writable:!0,value:a}):n[e]=a;var g=(n,e,a)=>Ce(n,typeof e!="symbol"?e+"":e,a);import{L as we,D as ce}from"./B3cuW3tw.js";const ze=`---
title: "So sicherst und synchronisierst du Emulator-Spielstände (RetroArch, Dolphin, PCSX2)"
description: "Emulator-Spielstände zwischen PCs und Steam Deck sichern und syncen: RetroArch, Dolphin, PCSX2, DuckStation und mehr, mit Verlauf und allen Speicherorten."
order: 6
updated: 2026-10-01
---

Emulator-Spielstände gehen leicht verloren: Speicherdateien und Savestates liegen in verstreuten Ordnern, und eine Neuinstallation oder ein neuer PC kann Jahre an Fortschritt löschen. Hoard sichert sie automatisch und hält sie zwischen deinen Rechnern synchron, Steam Deck eingeschlossen.

## Emulatoren, mit denen Hoard funktioniert

Hoard verarbeitet die üblichen Emulator-Speicherdateien (\`.srm\`, \`.sav\`, Memory Cards, Speicherordner pro Spiel) und Savestates. Wo diese Emulatoren speichern, weiß es von Haus aus:

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

PCSX2 schreibt Memory Cards (\`.ps2\`-Dateien) nach \`memcards/\`:

- Windows: \`Dokumente\\PCSX2\\memcards\`
- Linux: \`~/.config/PCSX2/memcards\`
- Steam Deck: \`~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards\`

Eine Memory Card enthält die Stände aller Spiele, die du darauf gespielt hast, und reist deshalb als ein Stück: Eine ältere Version wiederherzustellen setzt die ganze Karte zurück, nicht ein einzelnes Spiel.

### Dolphin Cloud-Saves (GameCube und Wii)

GameCube-Stände liegen unter \`GC/\` (Memory-Card-Abbilder oder ein Ordner pro Karte), Wii-Stände im emulierten NAND unter \`Wii/\`:

- Windows: \`Dokumente\\Dolphin Emulator\\GC\` und \`\\Wii\`
- Linux: \`~/.local/share/dolphin-emu/GC\` und \`/Wii\`
- Steam Deck: \`~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/\`

### DuckStation Cloud-Saves (PS1)

DuckStation legt Memory Cards in \`memcards/\` ab und erstellt standardmäßig für jedes Spiel eine eigene Karte, was sich sehr gut synchronisieren lässt:

- Windows: \`Dokumente\\DuckStation\\memcards\` (neuere Versionen nutzen \`%LOCALAPPDATA%\\DuckStation\\memcards\`)
- Linux: \`~/.local/share/duckstation/memcards\`
- Steam Deck: \`~/.var/app/org.duckstation.DuckStation/\`, unter \`data/\` oder \`config/\`

### RetroArch Save-Sync

RetroArch trennt \`saves/\` (die Spielstände) von \`states/\` (Savestates). Hoard verfolgt den Spielstand-Ordner; füge \`states/\` als eigenen Eintrag hinzu, wenn du mit Savestates spielst:

- Windows: \`%APPDATA%\\RetroArch\`, oder neben \`retroarch.exe\` bei einer portablen Installation
- Linux: \`~/.config/retroarch\`
- Steam Deck: \`~/.var/app/org.libretro.RetroArch/config/retroarch\`, oder \`~/Emulation/saves/retroarch\`, wenn du es mit EmuDeck eingerichtet hast

RetroArch hat außerdem ein eingebautes Cloud Sync, das mit einem WebDAV-Server spricht, den du bereitstellst. Das ist eine vernünftige Wahl, wenn du nur RetroArch nutzt und schon WebDAV betreibst. Hoard braucht kein WebDAV, führt einen Versionsverlauf zum Zurücksetzen und deckt auch die eigenständigen Emulatoren ab.

### PPSSPP (PSP)

Spielstände landen in \`PSP/SAVEDATA\`, Savestates in \`PSP/PPSSPP_STATE\`:

- Windows: \`Dokumente\\PPSSPP\\PSP\\SAVEDATA\`, oder \`memstick\\PSP\\SAVEDATA\` neben der ausführbaren Datei bei einer portablen Installation
- Linux: \`~/.config/ppsspp/PSP/SAVEDATA\`
- Steam Deck: \`~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA\`

### RPCS3 (PS3)

Die Stände liegen in \`dev_hdd0/home/00000001/savedata\`, unter Windows im RPCS3-Ordner, unter Linux und auf dem Steam Deck in \`~/.config/rpcs3/\`.

### Switch-Emulatoren: Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

Ryujinx speichert in \`bis/user/save\` (unter \`%APPDATA%\\Ryujinx\` oder \`~/.config/Ryujinx\`). Die yuzu-Familie nutzt \`nand/user/save\` im eigenen Ordner unter \`%APPDATA%\` oder \`~/.local/share\`.

Hier steckt eine Falle. Der yuzu-Baum sieht so aus: \`save/<Konto>/<Profil>/<Titel-ID>/\`, und die Profil-ID wird beim ersten Start des Emulators erzeugt, ist also bei jeder Installation anders. Synchronisierst du den ganzen \`save/\`-Ordner zwischen zwei Rechnern, landet auf jedem das Profil des anderen neben dem eigenen, und kein Spiel sieht den Fortschritt des anderen. Hoard steigt stattdessen bis in den Ordner jedes einzelnen Spiels hinab, sodass derselbe Titel auf beiden Rechnern zusammenfindet, egal wie das Profil heißt.

### Citra und Azahar (3DS)

Die Stände liegen tief unter \`sdmc/Nintendo 3DS/<id0>/<id1>/title/…\`, und \`id0\`/\`id1\` stammen aus den Schlüsseln der emulierten Konsole, unterscheiden sich also ebenfalls pro Installation. Hoard löst das wie beim Switch-Baum: ein Eintrag pro Spiel, zwischen den Rechnern zugeordnet.

### Der Rest

- **Cemu (Wii U):** \`mlc01/usr/save\`, unter \`%APPDATA%\\Cemu\` oder \`~/.local/share/Cemu\`.
- **shadPS4 (PS4):** \`savedata\`, unter \`%APPDATA%\\shadPS4\` oder \`~/.local/share/shadPS4\`.
- **Vita3K (PS Vita):** \`ux0/user/00/savedata\` in seinem Datenordner.
- **mGBA, melonDS und die meisten Emulatoren aus der Modulzeit:** eine \`.sav\` neben dem ROM, sofern du nichts anderes eingestellt hast. Füge die Stände aus dem ROM-Ordner von Hand hinzu.

## Emulator-Spielstände auf dem Steam Deck

Auf dem Steam Deck kommen die Emulatoren meist als Flatpak, ihre Ordner liegen also unter \`~/.var/app/<id>/\` statt in den üblichen \`~/.config\` oder \`~/.local/share\`. EmuDeck sammelt alles unter \`~/Emulation/saves/\`, ein Ordner pro Emulator. So oder so: Ordner einmal hinzufügen, Hoard beobachtet ihn.

Was auf einem Handheld zählt: Die Engine von Hoard läuft als Hintergrunddienst und sichert deshalb, wenn du ein Spiel im Spielmodus beendest, ganz ohne offenes Fenster. Nimmst du das Deck nach einer Session am Desktop in die Hand, ist der Spielstand schon da.

## Spielstand und Savestate sind nicht dasselbe

Es lohnt sich, beides zu trennen, weil es sich beim Reisen unterschiedlich verhält:

- Ein **Spielstand** (\`.srm\`, eine Memory Card, ein \`SAVEDATA\`-Ordner) ist die eigene Speicherung des Spiels, geschrieben von der emulierten Konsole. Er wandert ohne Murren zwischen Rechnern und Emulator-Versionen.
- Ein **Savestate** ist ein Abbild des Emulator-Speichers. Er hängt am Emulator-Build und oft am genauen Core, sodass ein State aus einer Version in einer anderen eventuell nicht lädt.

Hoard sichert beides. Wundere dich nur nicht, wenn sich ein State von einem aktualisierten Rechner auf einem veralteten nicht öffnen lässt — halte deine Emulatoren auf gleichen Versionen und verlass dich für Wichtiges auf Spielstände.

## Ein Emulator, viele Spiele

Ein Emulator ist ein einziger Prozess, der Dutzende Titel beherbergt, und genau das macht Emulator-Stände für ein Werkzeug schwierig, das in „dem laufenden Spiel“ denkt. Hoard hält die Titel auseinander, statt den ganzen Emulator als einen Klumpen zu behandeln, sodass jedes Spiel seine eigene Historie bekommt statt eines gemeinsamen Haufens, der sich bei jedem Start ändert. Geht ein Stand doch kaputt, kannst du [ihn auf eine frühere Version zurücksetzen](/guides/restore-a-game-save).

## Emulator-Stände ohne unsere Server

Alles hier funktioniert genauso gegen deinen eigenen Server: \`hoard-server\` starten, die App darauf zeigen lassen, und deine Stände gehen von deinem Rechner auf deine Platte. Kein Konto bei uns, keine Telemetrie an uns, nichts über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

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
`,Ae=`---
title: "How to back up and sync emulator saves (RetroArch, Dolphin, PCSX2)"
description: "Back up and sync emulator saves between PCs and Steam Deck: RetroArch, Dolphin, PCSX2, DuckStation and more, with version history and where each one saves."
order: 6
updated: 2026-10-01
related: sync-game-saves-across-pcs, back-up-game-saves, ludusavi-alternative
---

Emulator saves are easy to lose: save files and save states live in scattered folders, and a reinstall or a new PC can wipe years of progress. Hoard backs them up automatically and keeps them in sync across machines, including a Steam Deck.

## Emulators Hoard works with

Hoard handles standard emulator save files (\`.srm\`, \`.sav\`, memory cards, per-title save folders) and save states. It knows where these keep their saves out of the box:

- **Sony:** PCSX2 (PS2), DuckStation (PS1), PPSSPP (PSP), RPCS3 (PS3), shadPS4 (PS4), Vita3K (PS Vita)
- **Nintendo:** Dolphin (GameCube / Wii), Cemu (Wii U), Ryujinx, yuzu, Eden, Suyu, Citron and Sudachi (Switch), Citra / Azahar (3DS), melonDS (DS), mGBA (GBA), Project64 (N64)
- **Others:** RetroArch (multi-system), xemu (Xbox), Flycast (Dreamcast)

Because Hoard locates save folders using the same community database that powers Ludusavi, many paths are detected automatically. For anything custom, you can point Hoard at a folder by hand.

## Set up emulator save backups

1. **Install Hoard** for Windows, macOS or Linux and sign in.
2. Open the **Library** and add your emulator, or add its saves/states folder manually if you've changed the default location.
3. Keep **automatic mode** on. Hoard backs up after each session and keeps a versioned history.
4. Install Hoard on your other PCs with the same account to sync those saves everywhere — see [syncing saves across PCs](/guides/sync-game-saves-across-pcs).

## Ludusavi for emulators?

Ludusavi can back up emulator saves locally too, and it's a great free option for that. If you also want those emulator saves to sync automatically between machines and keep a cloud version history without configuring Rclone, that's where Hoard helps — read the full [Ludusavi vs Hoard comparison](/guides/ludusavi-alternative).

## Cloud saves for each emulator

None of the standalone emulators below syncs saves between machines on its own: the saves are plain files on your disk. That's good news, because any tool that watches the right folder can carry them. Here is where each one keeps them. "Steam Deck" means the Flatpak build you get from the Discover store.

### PCSX2 cloud saves (PS2)

PCSX2 writes memory cards (\`.ps2\` files) to \`memcards/\`:

- Windows: \`Documents\\PCSX2\\memcards\`
- Linux: \`~/.config/PCSX2/memcards\`
- Steam Deck: \`~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards\`

One memory card holds the saves of every game you've played on it, so it travels as one item: restoring an older version rolls back the whole card, not a single game.

### Dolphin cloud saves (GameCube and Wii)

GameCube saves live under \`GC/\` (memory card images or one folder per card), Wii saves in the emulated NAND under \`Wii/\`:

- Windows: \`Documents\\Dolphin Emulator\\GC\` and \`\\Wii\`
- Linux: \`~/.local/share/dolphin-emu/GC\` and \`/Wii\`
- Steam Deck: \`~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/\`

### DuckStation cloud saves (PS1)

DuckStation keeps memory cards in \`memcards/\`, and by default it makes a separate card for each game, which suits syncing well:

- Windows: \`Documents\\DuckStation\\memcards\` (newer builds use \`%LOCALAPPDATA%\\DuckStation\\memcards\`)
- Linux: \`~/.local/share/duckstation/memcards\`
- Steam Deck: \`~/.var/app/org.duckstation.DuckStation/\` under \`data/\` or \`config/\`

### RetroArch save sync

RetroArch splits \`saves/\` (the in-game saves) from \`states/\` (save states). Hoard tracks the saves folder; add \`states/\` as its own entry if you play with states:

- Windows: \`%APPDATA%\\RetroArch\`, or next to \`retroarch.exe\` on a portable install
- Linux: \`~/.config/retroarch\`
- Steam Deck: \`~/.var/app/org.libretro.RetroArch/config/retroarch\`, or \`~/Emulation/saves/retroarch\` if you set it up with EmuDeck

RetroArch also has a built-in Cloud Sync that talks to a WebDAV server you provide. It's a reasonable choice if you only use RetroArch and already run WebDAV. Hoard needs no WebDAV, keeps a version history you can roll back, and covers the standalone emulators too.

### PPSSPP (PSP)

Saves go to \`PSP/SAVEDATA\`, states to \`PSP/PPSSPP_STATE\`:

- Windows: \`Documents\\PPSSPP\\PSP\\SAVEDATA\`, or \`memstick\\PSP\\SAVEDATA\` next to the executable on a portable install
- Linux: \`~/.config/ppsspp/PSP/SAVEDATA\`
- Steam Deck: \`~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA\`

### RPCS3 (PS3)

Saves live in \`dev_hdd0/home/00000001/savedata\`, inside the RPCS3 folder on Windows and under \`~/.config/rpcs3/\` on Linux and Steam Deck.

### Switch emulators: Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

Ryujinx keeps saves in \`bis/user/save\` (under \`%APPDATA%\\Ryujinx\` or \`~/.config/Ryujinx\`). The yuzu family uses \`nand/user/save\` under its own folder in \`%APPDATA%\` or \`~/.local/share\`.

There's a trap here. The yuzu-style tree goes \`save/<account>/<profile>/<title-id>/\`, and the profile ID is generated the first time the emulator runs, so it's different on every install. Sync the whole \`save/\` folder between two machines and each one ends up with the other's profile next to its own, and neither game sees the other's progress. Hoard steps down to each game's own folder instead, so the same title matches across machines no matter what the profile is called.

### Citra and Azahar (3DS)

Saves sit deep under \`sdmc/Nintendo 3DS/<id0>/<id1>/title/…\`, and \`id0\`/\`id1\` come from the emulated console's keys, so they also differ per install. Hoard handles it the same way as the Switch tree: one entry per game, matched across machines.

### The rest

- **Cemu (Wii U):** \`mlc01/usr/save\`, under \`%APPDATA%\\Cemu\` or \`~/.local/share/Cemu\`.
- **shadPS4 (PS4):** \`savedata\`, under \`%APPDATA%\\shadPS4\` or \`~/.local/share/shadPS4\`.
- **Vita3K (PS Vita):** \`ux0/user/00/savedata\` inside its data folder.
- **mGBA, melonDS and most cartridge-era emulators:** a \`.sav\` next to the ROM, unless you told them otherwise. Add the ROM folder's saves by hand.

## Emulator saves on a Steam Deck

On a Steam Deck the emulators usually come from Flatpak, so their folders sit under \`~/.var/app/<id>/\` rather than the usual \`~/.config\` or \`~/.local/share\`. EmuDeck gathers everything under \`~/Emulation/saves/\`, one folder per emulator. Either way, add the folder once and Hoard watches it.

The part that matters on a handheld: Hoard's engine runs as a background service, so it backs up after you quit a game in Game Mode without any window open. Pick the Deck up after a session on the desktop and the save is already there.

## Save files and save states are not the same thing

Worth separating, because they behave differently when they travel:

- A **save file** (\`.srm\`, a memory card, a \`SAVEDATA\` folder) is the game's own save, written by the emulated console. It moves between machines and between emulator versions without complaint.
- A **save state** is a dump of emulator memory. It's tied to the emulator build, and often to the exact core, so a state written by one version may refuse to load in another.

Hoard backs up both. Just don't be surprised when a state from an updated machine won't open on a stale one — keep your emulators on matching versions, and lean on save files for anything you care about.

## One emulator, many games

An emulator is a single process hosting dozens of titles, which is what makes emulator saves awkward for a tool that thinks in terms of "the running game". Hoard keeps the titles apart rather than treating the whole emulator as one blob, so each game gets its own history instead of a single pile that changes every time you launch anything. If a save does go wrong, you can [roll it back to an earlier version](/guides/restore-a-game-save).

## Emulator saves without our servers

Everything here works the same against your own server: run \`hoard-server\`, point the app at it, and your saves go from your machine to your disk. No account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

## Tip

Save states are tied to a specific emulator version. Keep your emulators updated consistently across PCs so a synced state loads cleanly everywhere.

<!-- faq -->

## Frequently asked questions

### Does Hoard back up my ROMs too?

No. It tracks save folders, not game files. ROMs are large, they don't change, and you already have them — there's nothing to version.

### Do PCSX2, Dolphin or DuckStation have cloud saves built in?

No. They write saves to local folders and leave syncing to you. Point a sync tool at the folders listed above and the saves follow you between machines.

### Does RetroArch have cloud sync?

Yes, a built-in Cloud Sync that needs a WebDAV server you run or rent. Hoard is the alternative if you'd rather not set up WebDAV, want a version history to roll back to, or also play on standalone emulators.

### Does it work on a Steam Deck in Game Mode?

Yes. The engine runs as a background service, so saves are backed up when you quit a game, with no window open. Flatpak and EmuDeck folders work the same as any other.

### My emulator is a portable install. Does that work?

Yes. Add the folder next to the executable by hand and Hoard tracks it like any other save location. This is the usual setup on handhelds.

### Can I sync save states between two PCs?

You can, and Hoard will. Whether a state loads depends on the emulators being the same version on both machines, which is an emulator limitation rather than a sync one. Save files don't have that problem.

### Will it work with an emulator that isn't on the list?

Almost certainly. Detection covers the common ones automatically, and anything else you can add by pointing Hoard at its saves folder.

### Does self-hosting change anything for emulators?

No. Same detection, same versions, same sync. Only the storage is yours.
`,Le=`---
title: "Cómo hacer copia y sincronizar partidas de emuladores (RetroArch, Dolphin, PCSX2)"
description: "Copia y sincroniza partidas de emuladores entre PC y Steam Deck: RetroArch, Dolphin, PCSX2, DuckStation y más, con historial y dónde guarda cada uno."
order: 6
updated: 2026-10-01
---

Las partidas de emulador se pierden con facilidad: los archivos de guardado y los estados guardados viven en carpetas dispersas, y una reinstalación o un PC nuevo pueden borrar años de progreso. Hoard hace la copia automáticamente y los mantiene sincronizados entre equipos, Steam Deck incluida.

## Emuladores con los que funciona Hoard

Hoard gestiona los archivos de guardado estándar de emulador (\`.srm\`, \`.sav\`, memory cards, carpetas de guardado por juego) y los estados guardados. Sabe de serie dónde guardan estos:

- **Sony:** PCSX2 (PS2), DuckStation (PS1), PPSSPP (PSP), RPCS3 (PS3), shadPS4 (PS4), Vita3K (PS Vita)
- **Nintendo:** Dolphin (GameCube / Wii), Cemu (Wii U), Ryujinx, yuzu, Eden, Suyu, Citron y Sudachi (Switch), Citra / Azahar (3DS), melonDS (DS), mGBA (GBA), Project64 (N64)
- **Otros:** RetroArch (multisistema), xemu (Xbox), Flycast (Dreamcast)

Como Hoard localiza las carpetas de guardado con la misma base de datos comunitaria que utiliza Ludusavi, muchas rutas se detectan automáticamente. Para cualquier ruta personalizada, puedes apuntar Hoard a una carpeta a mano.

## Configura la copia de partidas de emulador

1. **Instala Hoard** para Windows, macOS o Linux e inicia sesión.
2. Abre la **Biblioteca** y añade tu emulador, o añade manualmente su carpeta de guardados/estados si has cambiado la ubicación por defecto.
3. Mantén el **modo automático** activado. Hoard hace la copia tras cada sesión y guarda un historial versionado.
4. Instala Hoard en tus otros PC con la misma cuenta para sincronizar esas partidas en todas partes; mira [cómo sincronizar partidas entre PC](/guides/sync-game-saves-across-pcs).

## ¿Ludusavi para emuladores?

Ludusavi también puede hacer copia de partidas de emulador en local, y es una gran opción gratuita para eso. Si además quieres que esas partidas de emulador se sincronicen automáticamente entre equipos y mantengan un historial de versiones en la nube sin configurar Rclone, ahí es donde ayuda Hoard; lee la [comparativa completa entre Ludusavi y Hoard](/guides/ludusavi-alternative).

## Partidas en la nube para cada emulador

Ninguno de los emuladores sueltos de abajo sincroniza partidas entre máquinas por su cuenta: las partidas son ficheros normales en tu disco. Eso es buena noticia, porque cualquier herramienta que vigile la carpeta correcta puede llevarlas. Aquí tienes dónde guarda cada uno. «Steam Deck» se refiere a la versión Flatpak que se instala desde la tienda Discover.

### Partidas de PCSX2 en la nube (PS2)

PCSX2 escribe las memory cards (ficheros \`.ps2\`) en \`memcards/\`:

- Windows: \`Documentos\\PCSX2\\memcards\`
- Linux: \`~/.config/PCSX2/memcards\`
- Steam Deck: \`~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards\`

Una memory card guarda las partidas de todos los juegos que hayas jugado en ella, así que viaja como una sola pieza: restaurar una versión anterior devuelve atrás la tarjeta entera, no un juego suelto.

### Partidas de Dolphin en la nube (GameCube y Wii)

Las partidas de GameCube viven en \`GC/\` (imágenes de memory card o una carpeta por tarjeta) y las de Wii en la NAND emulada, en \`Wii/\`:

- Windows: \`Documentos\\Dolphin Emulator\\GC\` y \`\\Wii\`
- Linux: \`~/.local/share/dolphin-emu/GC\` y \`/Wii\`
- Steam Deck: \`~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/\`

### Partidas de DuckStation en la nube (PS1)

DuckStation guarda las memory cards en \`memcards/\` y, por defecto, crea una tarjeta distinta para cada juego, algo que encaja muy bien con la sincronización:

- Windows: \`Documentos\\DuckStation\\memcards\` (las versiones recientes usan \`%LOCALAPPDATA%\\DuckStation\\memcards\`)
- Linux: \`~/.local/share/duckstation/memcards\`
- Steam Deck: \`~/.var/app/org.duckstation.DuckStation/\`, dentro de \`data/\` o \`config/\`

### Sincronizar partidas de RetroArch

RetroArch separa \`saves/\` (las partidas del juego) de \`states/\` (los estados guardados). Hoard rastrea la carpeta de partidas; añade \`states/\` como entrada propia si juegas con estados:

- Windows: \`%APPDATA%\\RetroArch\`, o junto a \`retroarch.exe\` en una instalación portable
- Linux: \`~/.config/retroarch\`
- Steam Deck: \`~/.var/app/org.libretro.RetroArch/config/retroarch\`, o \`~/Emulation/saves/retroarch\` si lo instalaste con EmuDeck

RetroArch tiene además un Cloud Sync integrado que habla con un servidor WebDAV que pones tú. Es una opción razonable si sólo usas RetroArch y ya tienes WebDAV. Hoard no necesita WebDAV, guarda un historial de versiones al que volver y cubre también los emuladores sueltos.

### PPSSPP (PSP)

Las partidas van a \`PSP/SAVEDATA\` y los estados a \`PSP/PPSSPP_STATE\`:

- Windows: \`Documentos\\PPSSPP\\PSP\\SAVEDATA\`, o \`memstick\\PSP\\SAVEDATA\` junto al ejecutable en una instalación portable
- Linux: \`~/.config/ppsspp/PSP/SAVEDATA\`
- Steam Deck: \`~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA\`

### RPCS3 (PS3)

Las partidas viven en \`dev_hdd0/home/00000001/savedata\`, dentro de la carpeta de RPCS3 en Windows y bajo \`~/.config/rpcs3/\` en Linux y Steam Deck.

### Emuladores de Switch: Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

Ryujinx guarda las partidas en \`bis/user/save\` (bajo \`%APPDATA%\\Ryujinx\` o \`~/.config/Ryujinx\`). La familia de yuzu usa \`nand/user/save\` dentro de su propia carpeta en \`%APPDATA%\` o \`~/.local/share\`.

Aquí hay una trampa. El árbol de tipo yuzu va \`save/<cuenta>/<perfil>/<id-del-juego>/\`, y el identificador de perfil se genera la primera vez que arranca el emulador, así que es distinto en cada instalación. Si sincronizas la carpeta \`save/\` entera entre dos máquinas, cada una acaba con el perfil de la otra al lado del suyo, y ningún juego ve el progreso del otro. Hoard baja hasta la carpeta propia de cada juego, así que el mismo título casa entre máquinas se llame como se llame el perfil.

### Citra y Azahar (3DS)

Las partidas están muy abajo, en \`sdmc/Nintendo 3DS/<id0>/<id1>/title/…\`, y \`id0\`/\`id1\` salen de las claves de la consola emulada, así que también cambian en cada instalación. Hoard lo resuelve igual que el árbol de Switch: una entrada por juego, emparejada entre máquinas.

### El resto

- **Cemu (Wii U):** \`mlc01/usr/save\`, bajo \`%APPDATA%\\Cemu\` o \`~/.local/share/Cemu\`.
- **shadPS4 (PS4):** \`savedata\`, bajo \`%APPDATA%\\shadPS4\` o \`~/.local/share/shadPS4\`.
- **Vita3K (PS Vita):** \`ux0/user/00/savedata\` dentro de su carpeta de datos.
- **mGBA, melonDS y la mayoría de emuladores de la época de cartuchos:** un \`.sav\` junto a la ROM, salvo que les hayas dicho otra cosa. Añade a mano las partidas de la carpeta de ROMs.

## Partidas de emulador en una Steam Deck

En una Steam Deck los emuladores suelen venir de Flatpak, así que sus carpetas están bajo \`~/.var/app/<id>/\` en vez de en los habituales \`~/.config\` o \`~/.local/share\`. EmuDeck lo reúne todo en \`~/Emulation/saves/\`, una carpeta por emulador. En cualquier caso, añades la carpeta una vez y Hoard la vigila.

Lo que importa en una portátil: el motor de Hoard corre como un servicio en segundo plano, así que hace la copia al salir de un juego en el modo Juego sin ninguna ventana abierta. Coges la Deck después de una sesión en el sobremesa y la partida ya está ahí.

## Partida guardada y estado guardado no son lo mismo

Vale la pena separarlos, porque se comportan distinto cuando viajan:

- Una **partida guardada** (\`.srm\`, una memory card, una carpeta \`SAVEDATA\`) es el guardado propio del juego, escrito por la consola emulada. Se mueve entre máquinas y entre versiones del emulador sin protestar.
- Un **estado guardado** es un volcado de la memoria del emulador. Está atado a esa compilación, y a menudo al núcleo exacto, así que un estado escrito por una versión puede negarse a cargar en otra.

Hoard copia los dos. Sólo que no te sorprenda que un estado de una máquina actualizada no abra en una que se quedó atrás: mantén los emuladores en versiones iguales y apóyate en las partidas guardadas para lo que te importe.

## Un emulador, muchos juegos

Un emulador es un solo proceso que aloja decenas de títulos, y eso es lo que vuelve incómodas las partidas de emulador para una herramienta que piensa en términos de «el juego que está corriendo». Hoard mantiene los títulos separados en lugar de tratar el emulador entero como un único bulto, así que cada juego tiene su propio historial y no un montón común que cambia cada vez que abres cualquier cosa. Si una partida se estropea, puedes [volver a una versión anterior](/guides/restore-a-game-save).

## Partidas de emulador sin pasar por nuestros servidores

Todo esto funciona igual contra tu propio servidor: levanta \`hoard-server\`, apunta la aplicación ahí, y tus partidas van de tu máquina a tu disco. Sin cuenta con nosotros, sin telemetría hacia nosotros, nada por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

## Consejo

Los estados guardados dependen de una versión concreta del emulador. Mantén tus emuladores actualizados de forma coherente entre PC para que un estado sincronizado cargue bien en todas partes.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard copia también mis ROMs?

No. Rastrea carpetas de partidas, no ficheros de juego. Las ROMs son grandes, no cambian y ya las tienes: no hay nada que versionar.

### ¿PCSX2, Dolphin o DuckStation tienen partidas en la nube integradas?

No. Escriben las partidas en carpetas locales y te dejan la sincronización a ti. Apunta una herramienta de sincronización a las carpetas de arriba y las partidas te seguirán entre máquinas.

### ¿RetroArch tiene sincronización en la nube?

Sí, un Cloud Sync integrado que necesita un servidor WebDAV que montes o alquiles tú. Hoard es la alternativa si prefieres no configurar WebDAV, quieres un historial de versiones al que volver o también juegas con emuladores sueltos.

### ¿Funciona en una Steam Deck en el modo Juego?

Sí. El motor corre como un servicio en segundo plano, así que las partidas se copian al salir de un juego, sin ninguna ventana abierta. Las carpetas de Flatpak y de EmuDeck funcionan igual que cualquier otra.

### Mi emulador es portable. ¿Funciona igual?

Sí. Añade a mano la carpeta que está junto al ejecutable y Hoard la rastreará como cualquier otra ubicación de partidas. Es el montaje habitual en consolas de mano.

### ¿Puedo sincronizar estados guardados entre dos PC?

Puedes, y Hoard lo hará. Que un estado cargue depende de que los emuladores estén en la misma versión en las dos máquinas, y eso es una limitación del emulador, no de la sincronización. Las partidas guardadas no tienen ese problema.

### ¿Funcionará con un emulador que no está en la lista?

Casi seguro que sí. La detección cubre los habituales de forma automática, y cualquier otro lo añades apuntando Hoard a su carpeta de partidas.

### ¿Cambia algo con emuladores si me autoalojo?

No. La misma detección, las mismas versiones, la misma sincronización. Lo único tuyo es el almacenamiento.
`,He=`---
title: "Comment sauvegarder et synchroniser les sauvegardes d'émulateur (RetroArch, Dolphin, PCSX2)"
description: "Sauvegardez et synchronisez vos sauvegardes d'émulateur entre PC et Steam Deck : RetroArch, Dolphin, PCSX2, DuckStation, avec historique et emplacements."
order: 6
updated: 2026-10-01
---

Les sauvegardes d'émulateur se perdent facilement : fichiers de sauvegarde et save states vivent dans des dossiers éparpillés, et une réinstallation ou un nouveau PC peut effacer des années de progression. Hoard les sauvegarde automatiquement et les garde synchronisés entre vos machines, Steam Deck compris.

## Émulateurs pris en charge par Hoard

Hoard gère les fichiers de sauvegarde d'émulateur standard (\`.srm\`, \`.sav\`, cartes mémoire, dossiers de sauvegarde par jeu) et les save states. Il sait d'office où ces émulateurs rangent leurs sauvegardes :

- **Sony :** PCSX2 (PS2), DuckStation (PS1), PPSSPP (PSP), RPCS3 (PS3), shadPS4 (PS4), Vita3K (PS Vita)
- **Nintendo :** Dolphin (GameCube / Wii), Cemu (Wii U), Ryujinx, yuzu, Eden, Suyu, Citron et Sudachi (Switch), Citra / Azahar (3DS), melonDS (DS), mGBA (GBA), Project64 (N64)
- **Autres :** RetroArch (multisystème), xemu (Xbox), Flycast (Dreamcast)

Comme Hoard localise les dossiers de sauvegarde avec la même base de données communautaire que Ludusavi, beaucoup de chemins sont détectés automatiquement. Pour un emplacement personnalisé, vous pouvez indiquer un dossier à la main.

## Configurer les sauvegardes d'émulateur

1. **Installez Hoard** pour Windows, macOS ou Linux et connectez-vous.
2. Ouvrez la **Bibliothèque** et ajoutez votre émulateur, ou ajoutez son dossier de sauvegardes/états manuellement si vous avez changé l'emplacement par défaut.
3. Gardez le **mode automatique** activé. Hoard sauvegarde après chaque session et conserve un historique versionné.
4. Installez Hoard sur vos autres PC avec le même compte pour synchroniser ces sauvegardes partout — voir [synchroniser ses parties entre plusieurs PC](/guides/sync-game-saves-across-pcs).

## Ludusavi pour les émulateurs ?

Ludusavi peut aussi sauvegarder localement les parties d'émulateur, et c'est une excellente option gratuite pour cela. Si vous voulez en plus que ces sauvegardes se synchronisent automatiquement entre machines et gardent un historique de versions dans le cloud sans configurer Rclone, c'est là que Hoard aide — lisez la [comparaison complète Ludusavi vs Hoard](/guides/ludusavi-alternative).

## Sauvegardes cloud pour chaque émulateur

Aucun des émulateurs autonomes ci-dessous ne synchronise seul les sauvegardes entre machines : ce sont de simples fichiers sur votre disque. C'est une bonne nouvelle, car n'importe quel outil qui surveille le bon dossier peut les transporter. Voici où chacun les range. « Steam Deck » désigne la version Flatpak installée depuis la boutique Discover.

### Sauvegardes cloud PCSX2 (PS2)

PCSX2 écrit les cartes mémoire (fichiers \`.ps2\`) dans \`memcards/\` :

- Windows : \`Documents\\PCSX2\\memcards\`
- Linux : \`~/.config/PCSX2/memcards\`
- Steam Deck : \`~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards\`

Une carte mémoire contient les sauvegardes de tous les jeux joués dessus, elle voyage donc d'un bloc : restaurer une version antérieure remet toute la carte en arrière, pas un seul jeu.

### Sauvegardes cloud Dolphin (GameCube et Wii)

Les sauvegardes GameCube vivent sous \`GC/\` (images de carte mémoire ou un dossier par carte), celles de Wii dans la NAND émulée sous \`Wii/\` :

- Windows : \`Documents\\Dolphin Emulator\\GC\` et \`\\Wii\`
- Linux : \`~/.local/share/dolphin-emu/GC\` et \`/Wii\`
- Steam Deck : \`~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/\`

### Sauvegardes cloud DuckStation (PS1)

DuckStation range les cartes mémoire dans \`memcards/\` et crée par défaut une carte distincte pour chaque jeu, ce qui se prête très bien à la synchronisation :

- Windows : \`Documents\\DuckStation\\memcards\` (les versions récentes utilisent \`%LOCALAPPDATA%\\DuckStation\\memcards\`)
- Linux : \`~/.local/share/duckstation/memcards\`
- Steam Deck : \`~/.var/app/org.duckstation.DuckStation/\`, sous \`data/\` ou \`config/\`

### Synchroniser les sauvegardes RetroArch

RetroArch sépare \`saves/\` (les sauvegardes du jeu) de \`states/\` (les save states). Hoard suit le dossier des sauvegardes ; ajoutez \`states/\` comme entrée à part si vous jouez avec des états :

- Windows : \`%APPDATA%\\RetroArch\`, ou à côté de \`retroarch.exe\` pour une installation portable
- Linux : \`~/.config/retroarch\`
- Steam Deck : \`~/.var/app/org.libretro.RetroArch/config/retroarch\`, ou \`~/Emulation/saves/retroarch\` si vous l'avez installé avec EmuDeck

RetroArch possède aussi un Cloud Sync intégré qui dialogue avec un serveur WebDAV que vous fournissez. C'est un choix raisonnable si vous n'utilisez que RetroArch et avez déjà un WebDAV. Hoard n'a pas besoin de WebDAV, garde un historique de versions à restaurer et couvre aussi les émulateurs autonomes.

### PPSSPP (PSP)

Les sauvegardes vont dans \`PSP/SAVEDATA\`, les états dans \`PSP/PPSSPP_STATE\` :

- Windows : \`Documents\\PPSSPP\\PSP\\SAVEDATA\`, ou \`memstick\\PSP\\SAVEDATA\` à côté de l'exécutable pour une installation portable
- Linux : \`~/.config/ppsspp/PSP/SAVEDATA\`
- Steam Deck : \`~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA\`

### RPCS3 (PS3)

Les sauvegardes vivent dans \`dev_hdd0/home/00000001/savedata\`, dans le dossier de RPCS3 sous Windows et sous \`~/.config/rpcs3/\` sous Linux et sur Steam Deck.

### Émulateurs Switch : Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

Ryujinx range les sauvegardes dans \`bis/user/save\` (sous \`%APPDATA%\\Ryujinx\` ou \`~/.config/Ryujinx\`). La famille yuzu utilise \`nand/user/save\` dans son propre dossier sous \`%APPDATA%\` ou \`~/.local/share\`.

Il y a un piège. L'arborescence de type yuzu est \`save/<compte>/<profil>/<id-du-jeu>/\`, et l'identifiant de profil est généré au premier lancement de l'émulateur : il diffère donc à chaque installation. Synchronisez tout le dossier \`save/\` entre deux machines et chacune se retrouve avec le profil de l'autre à côté du sien, sans qu'aucun jeu ne voie la progression de l'autre. Hoard descend plutôt jusqu'au dossier propre à chaque jeu, pour que le même titre corresponde d'une machine à l'autre, quel que soit le nom du profil.

### Citra et Azahar (3DS)

Les sauvegardes sont enfouies sous \`sdmc/Nintendo 3DS/<id0>/<id1>/title/…\`, et \`id0\`/\`id1\` dérivent des clés de la console émulée : ils diffèrent donc aussi à chaque installation. Hoard procède comme pour la Switch : une entrée par jeu, associée entre machines.

### Les autres

- **Cemu (Wii U) :** \`mlc01/usr/save\`, sous \`%APPDATA%\\Cemu\` ou \`~/.local/share/Cemu\`.
- **shadPS4 (PS4) :** \`savedata\`, sous \`%APPDATA%\\shadPS4\` ou \`~/.local/share/shadPS4\`.
- **Vita3K (PS Vita) :** \`ux0/user/00/savedata\` dans son dossier de données.
- **mGBA, melonDS et la plupart des émulateurs de l'ère cartouche :** un \`.sav\` à côté de la ROM, sauf réglage contraire. Ajoutez à la main les sauvegardes du dossier des ROM.

## Sauvegardes d'émulateur sur Steam Deck

Sur Steam Deck, les émulateurs viennent généralement de Flatpak : leurs dossiers se trouvent donc sous \`~/.var/app/<id>/\` plutôt que dans les habituels \`~/.config\` ou \`~/.local/share\`. EmuDeck regroupe tout sous \`~/Emulation/saves/\`, un dossier par émulateur. Dans tous les cas, ajoutez le dossier une fois et Hoard le surveille.

Ce qui compte sur une console portable : le moteur de Hoard tourne comme un service en arrière-plan, il sauvegarde donc quand vous quittez un jeu en mode Jeu, sans aucune fenêtre ouverte. Reprenez le Deck après une session sur le PC fixe, la sauvegarde est déjà là.

## Sauvegarde et état sauvegardé, ce n'est pas pareil

Mieux vaut les distinguer, car ils ne voyagent pas de la même manière :

- Une **sauvegarde** (\`.srm\`, une carte mémoire, un dossier \`SAVEDATA\`) est la sauvegarde propre du jeu, écrite par la console émulée. Elle passe d'une machine et d'une version d'émulateur à l'autre sans problème.
- Un **save state** est une copie de la mémoire de l'émulateur. Il est lié à la version de l'émulateur, souvent au core exact, et un état créé par une version peut refuser de se charger dans une autre.

Hoard sauvegarde les deux. Ne soyez simplement pas surpris si un état venu d'une machine à jour ne s'ouvre pas sur une machine en retard : gardez vos émulateurs sur les mêmes versions et comptez sur les sauvegardes pour ce qui compte.

## Un émulateur, beaucoup de jeux

Un émulateur est un seul processus qui héberge des dizaines de titres, et c'est ce qui rend ses sauvegardes délicates pour un outil qui raisonne en « jeu en cours ». Hoard sépare les titres au lieu de traiter l'émulateur comme un bloc, chaque jeu a donc son propre historique au lieu d'un tas commun qui change à chaque lancement. Si une sauvegarde tourne mal, vous pouvez [revenir à une version antérieure](/guides/restore-a-game-save).

## Sauvegardes d'émulateur sans passer par nos serveurs

Tout ceci fonctionne de la même façon avec votre propre serveur : lancez \`hoard-server\`, pointez l'application dessus, et vos sauvegardes vont de votre machine à votre disque. Pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

## Astuce

Les save states dépendent d'une version précise de l'émulateur. Mettez vos émulateurs à jour de façon cohérente sur tous vos PC pour qu'un état synchronisé se charge partout.

<!-- faq -->

## Questions fréquentes

### Hoard sauvegarde-t-il aussi mes ROM ?

Non. Il suit les dossiers de sauvegarde, pas les fichiers de jeu. Les ROM sont lourdes, ne changent pas, et vous les avez déjà : il n'y a rien à versionner.

### PCSX2, Dolphin ou DuckStation ont-ils des sauvegardes cloud intégrées ?

Non. Ils écrivent les sauvegardes dans des dossiers locaux et vous laissent la synchronisation. Pointez un outil de synchro sur les dossiers ci-dessus et vos sauvegardes vous suivront d'une machine à l'autre.

### RetroArch a-t-il une synchro cloud ?

Oui, un Cloud Sync intégré qui nécessite un serveur WebDAV que vous hébergez ou louez. Hoard est l'alternative si vous préférez éviter WebDAV, voulez un historique de versions à restaurer ou jouez aussi sur des émulateurs autonomes.

### Est-ce que ça marche sur Steam Deck en mode Jeu ?

Oui. Le moteur tourne comme un service en arrière-plan : les sauvegardes sont copiées quand vous quittez un jeu, sans fenêtre ouverte. Les dossiers Flatpak et EmuDeck fonctionnent comme n'importe quels autres.

### Mon émulateur est une version portable. Ça fonctionne ?

Oui. Ajoutez à la main le dossier situé à côté de l'exécutable et Hoard le suit comme n'importe quel autre emplacement. C'est la configuration habituelle sur les consoles portables.

### Puis-je synchroniser des save states entre deux PC ?

Oui, et Hoard le fera. Qu'un état se charge dépend de la même version d'émulateur sur les deux machines, une limite de l'émulateur et non de la synchronisation. Les sauvegardes n'ont pas ce problème.

### Est-ce que ça marchera avec un émulateur absent de la liste ?

Très probablement. La détection couvre automatiquement les plus courants, et vous ajoutez les autres en indiquant à Hoard leur dossier de sauvegarde.

### L'auto-hébergement change-t-il quelque chose pour les émulateurs ?

Non. Même détection, mêmes versions, même synchronisation. Seul le stockage vous appartient.
`,je=`---
title: "Come fare il backup e sincronizzare i salvataggi degli emulatori (RetroArch, Dolphin, PCSX2)"
description: "Backup e sync dei salvataggi degli emulatori tra PC e Steam Deck: RetroArch, Dolphin, PCSX2, DuckStation e altri, con cronologia e percorsi di ognuno."
order: 6
updated: 2026-10-01
---

I salvataggi degli emulatori si perdono facilmente: file di salvataggio e save state vivono in cartelle sparse, e una reinstallazione o un PC nuovo possono cancellare anni di progressi. Hoard ne fa il backup in automatico e li tiene sincronizzati tra le tue macchine, Steam Deck compresa.

## Emulatori con cui funziona Hoard

Hoard gestisce i file di salvataggio standard degli emulatori (\`.srm\`, \`.sav\`, memory card, cartelle di salvataggio per gioco) e i save state. Sa già dove salvano questi emulatori:

- **Sony:** PCSX2 (PS2), DuckStation (PS1), PPSSPP (PSP), RPCS3 (PS3), shadPS4 (PS4), Vita3K (PS Vita)
- **Nintendo:** Dolphin (GameCube / Wii), Cemu (Wii U), Ryujinx, yuzu, Eden, Suyu, Citron e Sudachi (Switch), Citra / Azahar (3DS), melonDS (DS), mGBA (GBA), Project64 (N64)
- **Altri:** RetroArch (multisistema), xemu (Xbox), Flycast (Dreamcast)

Dato che Hoard trova le cartelle di salvataggio con lo stesso database comunitario usato da Ludusavi, molti percorsi vengono rilevati in automatico. Per qualsiasi percorso personalizzato puoi indicare una cartella a mano.

## Imposta i backup dei salvataggi degli emulatori

1. **Installa Hoard** per Windows, macOS o Linux e accedi.
2. Apri la **Libreria** e aggiungi il tuo emulatore, oppure aggiungi manualmente la sua cartella di salvataggi/stati se hai cambiato la posizione predefinita.
3. Tieni attiva la **modalità automatica**. Hoard fa il backup dopo ogni sessione e conserva una cronologia versionata.
4. Installa Hoard sugli altri PC con lo stesso account per sincronizzare quei salvataggi ovunque — vedi [sincronizzare i salvataggi tra più PC](/guides/sync-game-saves-across-pcs).

## Ludusavi per gli emulatori?

Anche Ludusavi può fare il backup locale dei salvataggi degli emulatori, ed è un'ottima opzione gratuita per questo. Se vuoi anche che quei salvataggi si sincronizzino in automatico tra le macchine e mantengano una cronologia delle versioni nel cloud senza configurare Rclone, è lì che aiuta Hoard — leggi il [confronto completo tra Ludusavi e Hoard](/guides/ludusavi-alternative).

## Salvataggi nel cloud per ogni emulatore

Nessuno degli emulatori standalone qui sotto sincronizza da solo i salvataggi tra macchine: sono semplici file sul tuo disco. È una buona notizia, perché qualsiasi strumento che osservi la cartella giusta può portarli con sé. Ecco dove li tiene ciascuno. "Steam Deck" indica la versione Flatpak installata dallo store Discover.

### Salvataggi nel cloud di PCSX2 (PS2)

PCSX2 scrive le memory card (file \`.ps2\`) in \`memcards/\`:

- Windows: \`Documenti\\PCSX2\\memcards\`
- Linux: \`~/.config/PCSX2/memcards\`
- Steam Deck: \`~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards\`

Una memory card contiene i salvataggi di tutti i giochi usati su di essa, quindi viaggia come un pezzo unico: ripristinare una versione precedente riporta indietro l'intera card, non un singolo gioco.

### Salvataggi nel cloud di Dolphin (GameCube e Wii)

I salvataggi GameCube stanno in \`GC/\` (immagini di memory card o una cartella per card), quelli Wii nella NAND emulata in \`Wii/\`:

- Windows: \`Documenti\\Dolphin Emulator\\GC\` e \`\\Wii\`
- Linux: \`~/.local/share/dolphin-emu/GC\` e \`/Wii\`
- Steam Deck: \`~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/\`

### Salvataggi nel cloud di DuckStation (PS1)

DuckStation tiene le memory card in \`memcards/\` e, di default, crea una card separata per ogni gioco, cosa che si sposa benissimo con la sincronizzazione:

- Windows: \`Documenti\\DuckStation\\memcards\` (le versioni recenti usano \`%LOCALAPPDATA%\\DuckStation\\memcards\`)
- Linux: \`~/.local/share/duckstation/memcards\`
- Steam Deck: \`~/.var/app/org.duckstation.DuckStation/\`, sotto \`data/\` o \`config/\`

### Sincronizzare i salvataggi di RetroArch

RetroArch separa \`saves/\` (i salvataggi dei giochi) da \`states/\` (i save state). Hoard segue la cartella dei salvataggi; aggiungi \`states/\` come voce a parte se giochi con gli stati:

- Windows: \`%APPDATA%\\RetroArch\`, oppure accanto a \`retroarch.exe\` in un'installazione portable
- Linux: \`~/.config/retroarch\`
- Steam Deck: \`~/.var/app/org.libretro.RetroArch/config/retroarch\`, oppure \`~/Emulation/saves/retroarch\` se l'hai configurato con EmuDeck

RetroArch ha anche un Cloud Sync integrato che parla con un server WebDAV fornito da te. È una scelta sensata se usi solo RetroArch e hai già WebDAV. Hoard non ha bisogno di WebDAV, tiene una cronologia delle versioni da ripristinare e copre anche gli emulatori standalone.

### PPSSPP (PSP)

I salvataggi vanno in \`PSP/SAVEDATA\`, gli stati in \`PSP/PPSSPP_STATE\`:

- Windows: \`Documenti\\PPSSPP\\PSP\\SAVEDATA\`, oppure \`memstick\\PSP\\SAVEDATA\` accanto all'eseguibile in un'installazione portable
- Linux: \`~/.config/ppsspp/PSP/SAVEDATA\`
- Steam Deck: \`~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA\`

### RPCS3 (PS3)

I salvataggi stanno in \`dev_hdd0/home/00000001/savedata\`, dentro la cartella di RPCS3 su Windows e sotto \`~/.config/rpcs3/\` su Linux e Steam Deck.

### Emulatori Switch: Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

Ryujinx salva in \`bis/user/save\` (sotto \`%APPDATA%\\Ryujinx\` o \`~/.config/Ryujinx\`). La famiglia yuzu usa \`nand/user/save\` nella propria cartella in \`%APPDATA%\` o \`~/.local/share\`.

Qui c'è una trappola. L'albero in stile yuzu è \`save/<account>/<profilo>/<id-gioco>/\`, e l'ID del profilo viene generato al primo avvio dell'emulatore, quindi è diverso in ogni installazione. Se sincronizzi l'intera cartella \`save/\` tra due macchine, ciascuna si ritrova il profilo dell'altra accanto al proprio, e nessun gioco vede i progressi dell'altro. Hoard scende invece fino alla cartella di ogni singolo gioco, così lo stesso titolo combacia tra le macchine qualunque sia il nome del profilo.

### Citra e Azahar (3DS)

I salvataggi stanno in profondità sotto \`sdmc/Nintendo 3DS/<id0>/<id1>/title/…\`, e \`id0\`/\`id1\` derivano dalle chiavi della console emulata, quindi cambiano anch'essi a ogni installazione. Hoard li gestisce come l'albero Switch: una voce per gioco, abbinata tra le macchine.

### Il resto

- **Cemu (Wii U):** \`mlc01/usr/save\`, sotto \`%APPDATA%\\Cemu\` o \`~/.local/share/Cemu\`.
- **shadPS4 (PS4):** \`savedata\`, sotto \`%APPDATA%\\shadPS4\` o \`~/.local/share/shadPS4\`.
- **Vita3K (PS Vita):** \`ux0/user/00/savedata\` nella sua cartella dati.
- **mGBA, melonDS e la maggior parte degli emulatori dell'era delle cartucce:** un \`.sav\` accanto alla ROM, salvo diversa impostazione. Aggiungi a mano i salvataggi della cartella delle ROM.

## Salvataggi degli emulatori su Steam Deck

Su Steam Deck gli emulatori arrivano di solito da Flatpak, quindi le loro cartelle stanno sotto \`~/.var/app/<id>/\` invece che nei soliti \`~/.config\` o \`~/.local/share\`. EmuDeck raccoglie tutto sotto \`~/Emulation/saves/\`, una cartella per emulatore. In ogni caso aggiungi la cartella una volta e Hoard la tiene d'occhio.

Ciò che conta su una portatile: il motore di Hoard gira come servizio in background, quindi fa il backup quando esci da un gioco in modalità Gioco, senza finestre aperte. Riprendi la Deck dopo una sessione sul fisso e il salvataggio è già lì.

## Salvataggio e save state non sono la stessa cosa

Vale la pena distinguerli, perché si comportano diversamente quando viaggiano:

- Un **salvataggio** (\`.srm\`, una memory card, una cartella \`SAVEDATA\`) è il salvataggio del gioco stesso, scritto dalla console emulata. Passa tra macchine e versioni dell'emulatore senza problemi.
- Un **save state** è una copia della memoria dell'emulatore. È legato alla build dell'emulatore, spesso al core esatto, quindi uno stato creato da una versione potrebbe non caricarsi in un'altra.

Hoard fa il backup di entrambi. Solo, non stupirti se uno stato da una macchina aggiornata non si apre su una rimasta indietro: tieni gli emulatori alla stessa versione e affidati ai salvataggi per ciò che conta.

## Un emulatore, tanti giochi

Un emulatore è un unico processo che ospita decine di titoli, ed è questo a rendere scomodi i suoi salvataggi per uno strumento che ragiona in termini di "gioco in esecuzione". Hoard tiene separati i titoli invece di trattare l'emulatore come un unico blocco, così ogni gioco ha la propria cronologia invece di un mucchio comune che cambia a ogni avvio. Se un salvataggio si rovina, puoi [tornare a una versione precedente](/guides/restore-a-game-save).

## Salvataggi di emulatore senza passare dai nostri server

Tutto questo funziona allo stesso modo con il tuo server: avvia \`hoard-server\`, punta l'app lì, e i tuoi salvataggi vanno dalla tua macchina al tuo disco. Nessun account presso di noi, nessuna telemetria verso di noi, niente passa dai nostri server. Vedi [come fare il self-host di Hoard](/guides/self-host-hoard).

## Suggerimento

I save state dipendono da una versione precisa dell'emulatore. Aggiorna gli emulatori in modo coerente su tutti i PC, così uno stato sincronizzato si carica ovunque.

<!-- faq -->

## Domande frequenti

### Hoard fa il backup anche delle mie ROM?

No. Segue le cartelle di salvataggio, non i file di gioco. Le ROM sono grandi, non cambiano e le hai già: non c'è niente da versionare.

### PCSX2, Dolphin o DuckStation hanno salvataggi nel cloud integrati?

No. Scrivono i salvataggi in cartelle locali e lasciano a te la sincronizzazione. Punta uno strumento di sync sulle cartelle elencate sopra e i salvataggi ti seguiranno tra le macchine.

### RetroArch ha una sincronizzazione cloud?

Sì, un Cloud Sync integrato che richiede un server WebDAV gestito o affittato da te. Hoard è l'alternativa se preferisci non configurare WebDAV, vuoi una cronologia delle versioni a cui tornare o giochi anche con emulatori standalone.

### Funziona su Steam Deck in modalità Gioco?

Sì. Il motore gira come servizio in background, quindi i salvataggi vengono copiati quando esci da un gioco, senza finestre aperte. Le cartelle Flatpak ed EmuDeck funzionano come tutte le altre.

### Il mio emulatore è portable. Funziona?

Sì. Aggiungi a mano la cartella accanto all'eseguibile e Hoard la segue come qualsiasi altra posizione di salvataggio. È la configurazione tipica sulle console portatili.

### Posso sincronizzare i save state tra due PC?

Sì, e Hoard lo fa. Che uno stato si carichi dipende dall'avere la stessa versione dell'emulatore su entrambe le macchine, un limite dell'emulatore e non della sincronizzazione. I salvataggi non hanno questo problema.

### Funzionerà con un emulatore che non è nella lista?

Quasi certamente. Il rilevamento copre in automatico quelli comuni, e qualsiasi altro lo aggiungi indicando a Hoard la sua cartella di salvataggio.

### Il self-host cambia qualcosa per gli emulatori?

No. Stesso rilevamento, stesse versioni, stessa sincronizzazione. Solo lo spazio di archiviazione è tuo.
`,xe=`---
title: "エミュレーターのセーブをバックアップ・同期する方法（RetroArch、Dolphin、PCSX2）"
description: "RetroArch、Dolphin、PCSX2、DuckStationなどのエミュレーターのセーブをPCとSteam Deck間でバックアップ・同期。履歴付き、保存場所の一覧も。"
order: 6
updated: 2026-10-01
---

エミュレーターのセーブは失われやすいものです。セーブファイルとセーブステートはあちこちのフォルダーに散らばり、再インストールや新しい PC で何年分もの進行が消えることがあります。Hoard はそれらを自動でバックアップし、Steam Deck を含むすべてのマシン間で同期し続けます。

## Hoard が対応するエミュレーター

Hoard は標準的なエミュレーターのセーブファイル（\`.srm\`、\`.sav\`、メモリーカード、ゲームごとのセーブフォルダー）とセーブステートを扱います。次のエミュレーターについては、セーブの場所を最初から把握しています。

- **ソニー系:** PCSX2（PS2）、DuckStation（PS1）、PPSSPP（PSP）、RPCS3（PS3）、shadPS4（PS4）、Vita3K（PS Vita）
- **任天堂系:** Dolphin（ゲームキューブ / Wii）、Cemu（Wii U）、Ryujinx・yuzu・Eden・Suyu・Citron・Sudachi（Switch）、Citra / Azahar（3DS）、melonDS（DS）、mGBA（GBA）、Project64（N64）
- **その他:** RetroArch（マルチシステム）、xemu（Xbox）、Flycast（ドリームキャスト）

Hoard は Ludusavi と同じコミュニティデータベースを使ってセーブフォルダーを探すため、多くのパスは自動で検出されます。独自の場所は、フォルダーを手動で指定できます。

## エミュレーターのセーブバックアップを設定する

1. Windows、macOS、Linux 用の **Hoard をインストール**してサインインします。
2. **ライブラリ** を開いてエミュレーターを追加します。既定の場所を変更している場合は、セーブ／ステートのフォルダーを手動で追加します。
3. **自動モード** をオンのままにします。Hoard は各セッション後にバックアップし、世代履歴を保持します。
4. 他の PC にも同じアカウントで Hoard をインストールすれば、それらのセーブがどこでも同期されます。[複数の PC 間でセーブを同期する方法](/guides/sync-game-saves-across-pcs)も参照してください。

## エミュレーターに Ludusavi？

Ludusavi もエミュレーターのセーブをローカルにバックアップでき、その用途なら優れた無料の選択肢です。さらにそのセーブをマシン間で自動同期し、Rclone を設定せずにクラウドでバージョン履歴を残したいなら、そこで Hoard が役立ちます。[Ludusavi と Hoard の詳しい比較](/guides/ludusavi-alternative)をどうぞ。

## エミュレーターごとのクラウドセーブ

以下の単体エミュレーターは、どれもマシン間でセーブを自分で同期しません。セーブはディスク上のただのファイルです。これは良い知らせで、正しいフォルダーを監視するツールなら何でも運べるということです。それぞれの保存場所は次のとおりです。「Steam Deck」は Discover ストアからインストールする Flatpak 版を指します。

### PCSX2 のクラウドセーブ（PS2）

PCSX2 はメモリーカード（\`.ps2\` ファイル）を \`memcards/\` に書き込みます。

- Windows: \`Documents\\PCSX2\\memcards\`
- Linux: \`~/.config/PCSX2/memcards\`
- Steam Deck: \`~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards\`

1 枚のメモリーカードには、そのカードで遊んだ全ゲームのセーブが入っているため、1 つの単位として移動します。古いバージョンに戻すと、1 本のゲームではなくカード全体が巻き戻ります。

### Dolphin のクラウドセーブ（ゲームキューブと Wii）

ゲームキューブのセーブは \`GC/\`（メモリーカードのイメージ、またはカードごとのフォルダー）に、Wii のセーブはエミュレートされた NAND の \`Wii/\` にあります。

- Windows: \`Documents\\Dolphin Emulator\\GC\` と \`\\Wii\`
- Linux: \`~/.local/share/dolphin-emu/GC\` と \`/Wii\`
- Steam Deck: \`~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/\`

### DuckStation のクラウドセーブ（PS1）

DuckStation はメモリーカードを \`memcards/\` に置き、既定ではゲームごとに別のカードを作ります。これは同期と相性抜群です。

- Windows: \`Documents\\DuckStation\\memcards\`（新しいバージョンは \`%LOCALAPPDATA%\\DuckStation\\memcards\`）
- Linux: \`~/.local/share/duckstation/memcards\`
- Steam Deck: \`~/.var/app/org.duckstation.DuckStation/\` の \`data/\` または \`config/\` 内

### RetroArch のセーブ同期

RetroArch は \`saves/\`（ゲーム内セーブ）と \`states/\`（セーブステート）を分けています。Hoard はセーブフォルダーを追跡します。ステートも使うなら \`states/\` を別のエントリーとして追加してください。

- Windows: \`%APPDATA%\\RetroArch\`、ポータブル版なら \`retroarch.exe\` の隣
- Linux: \`~/.config/retroarch\`
- Steam Deck: \`~/.var/app/org.libretro.RetroArch/config/retroarch\`、EmuDeck で導入した場合は \`~/Emulation/saves/retroarch\`

RetroArch には、自分で用意した WebDAV サーバーとやり取りする Cloud Sync も組み込まれています。RetroArch だけを使い、すでに WebDAV があるなら妥当な選択です。Hoard は WebDAV が不要で、巻き戻せるバージョン履歴を保持し、単体エミュレーターもカバーします。

### PPSSPP（PSP）

セーブは \`PSP/SAVEDATA\`、ステートは \`PSP/PPSSPP_STATE\` に入ります。

- Windows: \`Documents\\PPSSPP\\PSP\\SAVEDATA\`、ポータブル版なら実行ファイルの隣の \`memstick\\PSP\\SAVEDATA\`
- Linux: \`~/.config/ppsspp/PSP/SAVEDATA\`
- Steam Deck: \`~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA\`

### RPCS3（PS3）

セーブは \`dev_hdd0/home/00000001/savedata\` にあります。Windows では RPCS3 のフォルダー内、Linux と Steam Deck では \`~/.config/rpcs3/\` の下です。

### Switch エミュレーター: Ryujinx、yuzu、Eden、Suyu、Citron、Sudachi

Ryujinx はセーブを \`bis/user/save\`（\`%APPDATA%\\Ryujinx\` または \`~/.config/Ryujinx\` の下）に置きます。yuzu 系は \`%APPDATA%\` または \`~/.local/share\` にある各自のフォルダー内の \`nand/user/save\` を使います。

ここには落とし穴があります。yuzu 系のツリーは \`save/<アカウント>/<プロファイル>/<タイトルID>/\` という構造で、プロファイル ID はエミュレーターの初回起動時に生成されるため、インストールごとに異なります。\`save/\` フォルダー全体を 2 台のマシンで同期すると、それぞれに相手のプロファイルが自分のものと並んで置かれ、どちらのゲームも相手の進行を認識しません。Hoard は代わりに各ゲーム自身のフォルダーまで降りていくので、プロファイル名が何であっても同じタイトルがマシン間で一致します。

### Citra と Azahar（3DS）

セーブは \`sdmc/Nintendo 3DS/<id0>/<id1>/title/…\` の深い場所にあり、\`id0\`/\`id1\` はエミュレートされた本体の鍵から作られるため、これもインストールごとに異なります。Hoard は Switch のツリーと同じように扱い、ゲームごとに 1 つのエントリーとしてマシン間で対応付けます。

### その他

- **Cemu（Wii U）:** \`mlc01/usr/save\`。\`%APPDATA%\\Cemu\` または \`~/.local/share/Cemu\` の下。
- **shadPS4（PS4）:** \`savedata\`。\`%APPDATA%\\shadPS4\` または \`~/.local/share/shadPS4\` の下。
- **Vita3K（PS Vita）:** データフォルダー内の \`ux0/user/00/savedata\`。
- **mGBA、melonDS など、カートリッジ時代のほとんどのエミュレーター:** 設定を変えていなければ ROM の隣に \`.sav\` が作られます。ROM フォルダーのセーブを手動で追加してください。

## Steam Deck でのエミュレーターのセーブ

Steam Deck ではエミュレーターは通常 Flatpak で入るため、フォルダーはいつもの \`~/.config\` や \`~/.local/share\` ではなく \`~/.var/app/<id>/\` の下にあります。EmuDeck はすべてを \`~/Emulation/saves/\` にエミュレーターごとのフォルダーでまとめます。いずれの場合も、フォルダーを一度追加すれば Hoard が監視します。

携帯機で大事なのは次の点です。Hoard のエンジンはバックグラウンドサービスとして動くため、ゲームモードでゲームを終了すると、ウィンドウを開かなくてもバックアップされます。デスクトップで遊んだ後に Deck を手に取れば、セーブはもう届いています。

## セーブデータとセーブステートは別物

移動するときの振る舞いが違うので、分けて考える価値があります。

- **セーブファイル**（\`.srm\`、メモリーカード、\`SAVEDATA\` フォルダー）は、エミュレートされたゲーム機が書き込むゲーム自身のセーブです。マシン間やエミュレーターのバージョン間を問題なく移動できます。
- **セーブステート**はエミュレーターのメモリのダンプです。エミュレーターのビルド、多くの場合は特定のコアに縛られるため、あるバージョンで作ったステートが別のバージョンでは読み込めないことがあります。

Hoard は両方をバックアップします。ただ、更新済みのマシンのステートが古いマシンで開けなくても驚かないでください。エミュレーターのバージョンを揃え、大事なものはセーブファイルに頼りましょう。

## エミュレーターは 1 つ、ゲームは多数

エミュレーターは数十本のタイトルを抱える 1 つのプロセスで、「実行中のゲーム」という単位で考えるツールにとって、これがエミュレーターのセーブを扱いにくくしています。Hoard はエミュレーター全体を 1 つの塊として扱わずにタイトルを分けるので、何かを起動するたびに変わる共通の山ではなく、ゲームごとに独自の履歴が残ります。セーブが壊れても、[以前のバージョンに戻す](/guides/restore-a-game-save)ことができます。

## 当方のサーバーを介さないエミュレーターのバックアップ

ここで説明したことはすべて、自分のサーバーでも同じように動きます。\`hoard-server\` を起動してアプリをそこに向ければ、セーブはあなたのマシンからあなたのディスクへ届きます。当社のアカウントも、当社へのテレメトリーも不要で、当社のサーバーを何も通りません。[Hoard をセルフホストする方法](/guides/self-host-hoard)を参照してください。

## ヒント

セーブステートは特定のエミュレーターのバージョンに依存します。同期したステートがどこでも正しく読み込めるよう、各 PC のエミュレーターを揃えて更新しましょう。

<!-- faq -->

## よくある質問

### Hoard は ROM もバックアップしますか？

いいえ。追跡するのはセーブフォルダーで、ゲームファイルではありません。ROM は大きく、変化せず、すでに手元にあるので、バージョン管理するものがありません。

### PCSX2、Dolphin、DuckStation にクラウドセーブは組み込まれていますか？

いいえ。セーブをローカルのフォルダーに書き込み、同期はユーザーに任せています。上に挙げたフォルダーに同期ツールを向ければ、セーブはマシン間でついてきます。

### RetroArch にクラウド同期はありますか？

はい。自分で運用またはレンタルする WebDAV サーバーが必要な Cloud Sync が組み込まれています。WebDAV を設定したくない、巻き戻せるバージョン履歴がほしい、単体エミュレーターも使う、という場合は Hoard が代わりになります。

### ゲームモードの Steam Deck でも動きますか？

はい。エンジンがバックグラウンドサービスとして動くので、ゲームを終了したときにウィンドウなしでセーブがバックアップされます。Flatpak や EmuDeck のフォルダーも他と同じように使えます。

### エミュレーターがポータブル版です。使えますか？

はい。実行ファイルの隣にあるフォルダーを手動で追加すれば、Hoard は他のセーブ場所と同じように追跡します。携帯機ではこれが一般的な構成です。

### 2 台の PC 間でセーブステートを同期できますか？

できますし、Hoard はそうします。ステートが読み込めるかどうかは両方のマシンのエミュレーターが同じバージョンかどうかにかかっており、これは同期ではなくエミュレーター側の制約です。セーブファイルにはこの問題がありません。

### リストにないエミュレーターでも使えますか？

ほぼ確実に使えます。一般的なものは自動で検出され、それ以外はセーブフォルダーを Hoard に指定すれば追加できます。

### セルフホストするとエミュレーターで何か変わりますか？

いいえ。検出も、バージョンも、同期も同じです。ストレージがあなたのものになるだけです。
`,Oe=`---
title: "Como fazer backup e sincronizar saves de emuladores (RetroArch, Dolphin, PCSX2)"
description: "Backup e sincronização de saves de emuladores entre PC e Steam Deck: RetroArch, Dolphin, PCSX2, DuckStation e mais, com histórico e onde cada um grava."
order: 6
updated: 2026-10-01
---

Os saves de emulador perdem-se com facilidade: ficheiros de save e save states vivem em pastas espalhadas, e uma reinstalação ou um PC novo podem apagar anos de progresso. O Hoard faz backup deles automaticamente e mantém-nos sincronizados entre as tuas máquinas, Steam Deck incluída.

## Emuladores com que o Hoard funciona

O Hoard trata os ficheiros de save habituais dos emuladores (\`.srm\`, \`.sav\`, memory cards, pastas de save por jogo) e os save states. Sabe de origem onde estes emuladores guardam:

- **Sony:** PCSX2 (PS2), DuckStation (PS1), PPSSPP (PSP), RPCS3 (PS3), shadPS4 (PS4), Vita3K (PS Vita)
- **Nintendo:** Dolphin (GameCube / Wii), Cemu (Wii U), Ryujinx, yuzu, Eden, Suyu, Citron e Sudachi (Switch), Citra / Azahar (3DS), melonDS (DS), mGBA (GBA), Project64 (N64)
- **Outros:** RetroArch (multissistema), xemu (Xbox), Flycast (Dreamcast)

Como o Hoard encontra as pastas de save com a mesma base de dados comunitária que o Ludusavi usa, muitos caminhos são detetados automaticamente. Para qualquer caminho personalizado, podes apontar o Hoard para uma pasta à mão.

## Configurar backups de saves de emulador

1. **Instala o Hoard** para Windows, macOS ou Linux e inicia sessão.
2. Abre a **Biblioteca** e adiciona o teu emulador, ou adiciona manualmente a sua pasta de saves/estados se mudaste a localização predefinida.
3. Mantém o **modo automático** ligado. O Hoard faz backup depois de cada sessão e guarda um histórico versionado.
4. Instala o Hoard nos teus outros PCs com a mesma conta para sincronizar esses saves em todo o lado — vê [como sincronizar saves entre PCs](/guides/sync-game-saves-across-pcs).

## Ludusavi para emuladores?

O Ludusavi também consegue fazer backup local de saves de emulador, e é uma ótima opção gratuita para isso. Se além disso quiseres que esses saves se sincronizem automaticamente entre máquinas e tenham um histórico de versões na nuvem sem configurar o Rclone, é aí que o Hoard ajuda — lê a [comparação completa entre Ludusavi e Hoard](/guides/ludusavi-alternative).

## Saves na nuvem para cada emulador

Nenhum dos emuladores independentes abaixo sincroniza saves entre máquinas por conta própria: os saves são ficheiros normais no teu disco. Isso é uma boa notícia, porque qualquer ferramenta que vigie a pasta certa consegue levá-los. Eis onde cada um os guarda. «Steam Deck» refere-se à versão Flatpak instalada a partir da loja Discover.

### Saves na nuvem do PCSX2 (PS2)

O PCSX2 escreve as memory cards (ficheiros \`.ps2\`) em \`memcards/\`:

- Windows: \`Documentos\\PCSX2\\memcards\`
- Linux: \`~/.config/PCSX2/memcards\`
- Steam Deck: \`~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards\`

Uma memory card guarda os saves de todos os jogos que jogaste nela, por isso viaja como uma peça única: restaurar uma versão anterior recua a card inteira, não um jogo só.

### Saves na nuvem do Dolphin (GameCube e Wii)

Os saves de GameCube vivem em \`GC/\` (imagens de memory card ou uma pasta por card) e os de Wii na NAND emulada, em \`Wii/\`:

- Windows: \`Documentos\\Dolphin Emulator\\GC\` e \`\\Wii\`
- Linux: \`~/.local/share/dolphin-emu/GC\` e \`/Wii\`
- Steam Deck: \`~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/\`

### Saves na nuvem do DuckStation (PS1)

O DuckStation guarda as memory cards em \`memcards/\` e, por predefinição, cria uma card separada para cada jogo, o que encaixa muito bem na sincronização:

- Windows: \`Documentos\\DuckStation\\memcards\` (as versões recentes usam \`%LOCALAPPDATA%\\DuckStation\\memcards\`)
- Linux: \`~/.local/share/duckstation/memcards\`
- Steam Deck: \`~/.var/app/org.duckstation.DuckStation/\`, em \`data/\` ou \`config/\`

### Sincronizar saves do RetroArch

O RetroArch separa \`saves/\` (os saves dos jogos) de \`states/\` (os save states). O Hoard acompanha a pasta de saves; adiciona \`states/\` como entrada própria se jogas com estados:

- Windows: \`%APPDATA%\\RetroArch\`, ou junto a \`retroarch.exe\` numa instalação portátil
- Linux: \`~/.config/retroarch\`
- Steam Deck: \`~/.var/app/org.libretro.RetroArch/config/retroarch\`, ou \`~/Emulation/saves/retroarch\` se o configuraste com o EmuDeck

O RetroArch tem ainda um Cloud Sync integrado que fala com um servidor WebDAV fornecido por ti. É uma escolha razoável se só usas o RetroArch e já tens WebDAV. O Hoard não precisa de WebDAV, guarda um histórico de versões para recuperar e cobre também os emuladores independentes.

### PPSSPP (PSP)

Os saves vão para \`PSP/SAVEDATA\` e os estados para \`PSP/PPSSPP_STATE\`:

- Windows: \`Documentos\\PPSSPP\\PSP\\SAVEDATA\`, ou \`memstick\\PSP\\SAVEDATA\` junto ao executável numa instalação portátil
- Linux: \`~/.config/ppsspp/PSP/SAVEDATA\`
- Steam Deck: \`~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA\`

### RPCS3 (PS3)

Os saves vivem em \`dev_hdd0/home/00000001/savedata\`, dentro da pasta do RPCS3 no Windows e em \`~/.config/rpcs3/\` no Linux e na Steam Deck.

### Emuladores de Switch: Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

O Ryujinx guarda os saves em \`bis/user/save\` (em \`%APPDATA%\\Ryujinx\` ou \`~/.config/Ryujinx\`). A família yuzu usa \`nand/user/save\` dentro da sua própria pasta em \`%APPDATA%\` ou \`~/.local/share\`.

Há aqui uma armadilha. A árvore ao estilo yuzu é \`save/<conta>/<perfil>/<id-do-jogo>/\`, e o ID do perfil é gerado na primeira vez que o emulador arranca, por isso é diferente em cada instalação. Se sincronizares a pasta \`save/\` inteira entre duas máquinas, cada uma fica com o perfil da outra ao lado do seu, e nenhum jogo vê o progresso do outro. O Hoard desce antes até à pasta de cada jogo, para que o mesmo título corresponda entre máquinas seja qual for o nome do perfil.

### Citra e Azahar (3DS)

Os saves estão bem fundo em \`sdmc/Nintendo 3DS/<id0>/<id1>/title/…\`, e \`id0\`/\`id1\` vêm das chaves da consola emulada, por isso também mudam em cada instalação. O Hoard trata-os como a árvore da Switch: uma entrada por jogo, emparelhada entre máquinas.

### O resto

- **Cemu (Wii U):** \`mlc01/usr/save\`, em \`%APPDATA%\\Cemu\` ou \`~/.local/share/Cemu\`.
- **shadPS4 (PS4):** \`savedata\`, em \`%APPDATA%\\shadPS4\` ou \`~/.local/share/shadPS4\`.
- **Vita3K (PS Vita):** \`ux0/user/00/savedata\` dentro da sua pasta de dados.
- **mGBA, melonDS e a maioria dos emuladores da era dos cartuchos:** um \`.sav\` junto à ROM, salvo se lhes disseste outra coisa. Adiciona à mão os saves da pasta das ROMs.

## Saves de emulador na Steam Deck

Na Steam Deck os emuladores costumam vir em Flatpak, por isso as pastas ficam em \`~/.var/app/<id>/\` em vez dos habituais \`~/.config\` ou \`~/.local/share\`. O EmuDeck junta tudo em \`~/Emulation/saves/\`, uma pasta por emulador. Seja como for, adicionas a pasta uma vez e o Hoard vigia-a.

O que conta numa portátil: o motor do Hoard corre como um serviço em segundo plano, por isso faz o backup quando sais de um jogo no modo Jogo, sem nenhuma janela aberta. Pegas na Deck depois de uma sessão no PC e o save já lá está.

## Save e save state não são a mesma coisa

Vale a pena separá-los, porque comportam-se de forma diferente quando viajam:

- Um **save** (\`.srm\`, uma memory card, uma pasta \`SAVEDATA\`) é o save do próprio jogo, escrito pela consola emulada. Passa entre máquinas e versões do emulador sem problemas.
- Um **save state** é uma cópia da memória do emulador. Está preso à build do emulador, muitas vezes ao core exato, por isso um estado criado numa versão pode recusar-se a carregar noutra.

O Hoard faz backup dos dois. Só não te surpreendas se um estado de uma máquina atualizada não abrir numa que ficou para trás: mantém os emuladores na mesma versão e confia nos saves para o que importa.

## Um emulador, muitos jogos

Um emulador é um único processo que aloja dezenas de títulos, e é isso que torna os seus saves incómodos para uma ferramenta que pensa em «o jogo que está a correr». O Hoard mantém os títulos separados em vez de tratar o emulador como um bloco único, por isso cada jogo tem o seu histórico em vez de um monte comum que muda sempre que abres alguma coisa. Se um save se estragar, podes [voltar a uma versão anterior](/guides/restore-a-game-save).

## Saves de emulador sem passar pelos nossos servidores

Tudo isto funciona da mesma forma com o teu próprio servidor: arranca o \`hoard-server\`, aponta a aplicação para lá, e os teus saves vão da tua máquina para o teu disco. Sem conta connosco, sem telemetria para nós, nada passa pelos nossos servidores. Vê [como fazer self-host do Hoard](/guides/self-host-hoard).

## Dica

Os save states dependem de uma versão concreta do emulador. Mantém os emuladores atualizados de forma coerente entre PCs para que um estado sincronizado carregue bem em todo o lado.

<!-- faq -->

## Perguntas frequentes

### O Hoard também faz backup das minhas ROMs?

Não. Acompanha pastas de saves, não ficheiros de jogo. As ROMs são grandes, não mudam e já as tens: não há nada para versionar.

### O PCSX2, o Dolphin ou o DuckStation têm saves na nuvem integrados?

Não. Escrevem os saves em pastas locais e deixam a sincronização contigo. Aponta uma ferramenta de sincronização às pastas acima e os saves seguem-te entre máquinas.

### O RetroArch tem sincronização na nuvem?

Sim, um Cloud Sync integrado que precisa de um servidor WebDAV que geres ou alugas. O Hoard é a alternativa se preferires não configurar WebDAV, quiseres um histórico de versões para recuperar ou também jogares com emuladores independentes.

### Funciona numa Steam Deck no modo Jogo?

Sim. O motor corre como um serviço em segundo plano, por isso os saves são copiados quando sais de um jogo, sem janela aberta. As pastas de Flatpak e do EmuDeck funcionam como qualquer outra.

### O meu emulador é portátil. Funciona?

Sim. Adiciona à mão a pasta junto ao executável e o Hoard acompanha-a como qualquer outra localização de saves. É a configuração habitual nas consolas portáteis.

### Posso sincronizar save states entre dois PCs?

Podes, e o Hoard fá-lo. Se um estado carrega depende de os emuladores estarem na mesma versão nas duas máquinas, uma limitação do emulador e não da sincronização. Os saves não têm esse problema.

### Funciona com um emulador que não está na lista?

Quase de certeza. A deteção cobre os habituais automaticamente, e qualquer outro adicionas apontando o Hoard para a sua pasta de saves.

### O self-host muda alguma coisa para os emuladores?

Não. A mesma deteção, as mesmas versões, a mesma sincronização. Só o armazenamento é teu.
`,Ge=`---
title: "如何备份和同步模拟器存档（RetroArch、Dolphin、PCSX2）"
description: "在 PC 和 Steam Deck 之间备份与同步 RetroArch、Dolphin、PCSX2、DuckStation 等模拟器存档，保留版本历史，并列出各自的存档位置。"
order: 6
updated: 2026-10-01
---

模拟器存档很容易丢失：存档文件和即时存档散落在各处的文件夹里，一次重装或换一台新电脑就可能抹掉多年的进度。Hoard 会自动备份它们，并在你的所有设备之间保持同步，包括 Steam Deck。

## Hoard 支持的模拟器

Hoard 处理标准的模拟器存档文件（\`.srm\`、\`.sav\`、记忆卡、按游戏划分的存档文件夹）以及即时存档。下面这些模拟器的存档位置它都已内置：

- **索尼：** PCSX2（PS2）、DuckStation（PS1）、PPSSPP（PSP）、RPCS3（PS3）、shadPS4（PS4）、Vita3K（PS Vita）
- **任天堂：** Dolphin（GameCube / Wii）、Cemu（Wii U）、Ryujinx、yuzu、Eden、Suyu、Citron 和 Sudachi（Switch）、Citra / Azahar（3DS）、melonDS（DS）、mGBA（GBA）、Project64（N64）
- **其他：** RetroArch（多平台）、xemu（Xbox）、Flycast（Dreamcast）

由于 Hoard 使用与 Ludusavi 相同的社区数据库来定位存档文件夹，许多路径都能自动识别。对于自定义的位置，你也可以手动指定文件夹。

## 设置模拟器存档备份

1. **安装 Hoard**（Windows、macOS 或 Linux）并登录。
2. 打开**库**并添加你的模拟器；如果你更改了默认位置，请手动添加它的存档／即时存档文件夹。
3. 保持**自动模式**开启。Hoard 会在每次会话后备份，并保留版本历史。
4. 在其他 PC 上用同一账号安装 Hoard，这些存档就会在所有设备间同步——参见[如何在多台 PC 之间同步存档](/guides/sync-game-saves-across-pcs)。

## 模拟器用 Ludusavi？

Ludusavi 也能在本地备份模拟器存档，这方面它是很棒的免费选择。如果你还希望这些存档在设备之间自动同步，并在云端保留版本历史，而不用配置 Rclone，那就是 Hoard 派上用场的地方——请阅读完整的 [Ludusavi 与 Hoard 对比](/guides/ludusavi-alternative)。

## 各模拟器的云存档

下面这些独立模拟器都不会自己在设备之间同步存档：存档只是你磁盘上的普通文件。这其实是好事，因为任何监视正确文件夹的工具都能把它们带走。下面是各自的存档位置。“Steam Deck”指的是从 Discover 商店安装的 Flatpak 版本。

### PCSX2 云存档（PS2）

PCSX2 把记忆卡（\`.ps2\` 文件）写到 \`memcards/\`：

- Windows：\`Documents\\PCSX2\\memcards\`
- Linux：\`~/.config/PCSX2/memcards\`
- Steam Deck：\`~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards\`

一张记忆卡里存着你在上面玩过的所有游戏的存档，因此它作为一个整体迁移：恢复旧版本会回滚整张卡，而不是单个游戏。

### Dolphin 云存档（GameCube 和 Wii）

GameCube 存档位于 \`GC/\`（记忆卡镜像或每张卡一个文件夹），Wii 存档位于模拟 NAND 的 \`Wii/\` 中：

- Windows：\`Documents\\Dolphin Emulator\\GC\` 和 \`\\Wii\`
- Linux：\`~/.local/share/dolphin-emu/GC\` 和 \`/Wii\`
- Steam Deck：\`~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/\`

### DuckStation 云存档（PS1）

DuckStation 把记忆卡放在 \`memcards/\`，并且默认为每个游戏单独建一张卡，这非常适合同步：

- Windows：\`Documents\\DuckStation\\memcards\`（较新的版本使用 \`%LOCALAPPDATA%\\DuckStation\\memcards\`）
- Linux：\`~/.local/share/duckstation/memcards\`
- Steam Deck：\`~/.var/app/org.duckstation.DuckStation/\` 下的 \`data/\` 或 \`config/\`

### RetroArch 存档同步

RetroArch 把 \`saves/\`（游戏内存档）和 \`states/\`（即时存档）分开。Hoard 跟踪存档文件夹；如果你也用即时存档，把 \`states/\` 作为单独的条目添加：

- Windows：\`%APPDATA%\\RetroArch\`，便携版则在 \`retroarch.exe\` 旁边
- Linux：\`~/.config/retroarch\`
- Steam Deck：\`~/.var/app/org.libretro.RetroArch/config/retroarch\`；如果是用 EmuDeck 配置的，则在 \`~/Emulation/saves/retroarch\`

RetroArch 还内置了 Cloud Sync，需要连接你自己提供的 WebDAV 服务器。如果你只用 RetroArch 且已有 WebDAV，这是个合理的选择。Hoard 不需要 WebDAV，保留可回滚的版本历史，并且也覆盖各独立模拟器。

### PPSSPP（PSP）

存档放在 \`PSP/SAVEDATA\`，即时存档放在 \`PSP/PPSSPP_STATE\`：

- Windows：\`Documents\\PPSSPP\\PSP\\SAVEDATA\`，便携版则在可执行文件旁的 \`memstick\\PSP\\SAVEDATA\`
- Linux：\`~/.config/ppsspp/PSP/SAVEDATA\`
- Steam Deck：\`~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA\`

### RPCS3（PS3）

存档位于 \`dev_hdd0/home/00000001/savedata\`：Windows 上在 RPCS3 文件夹内，Linux 和 Steam Deck 上在 \`~/.config/rpcs3/\` 下。

### Switch 模拟器：Ryujinx、yuzu、Eden、Suyu、Citron、Sudachi

Ryujinx 把存档放在 \`bis/user/save\`（位于 \`%APPDATA%\\Ryujinx\` 或 \`~/.config/Ryujinx\` 下）。yuzu 系列则使用各自文件夹中的 \`nand/user/save\`，位于 \`%APPDATA%\` 或 \`~/.local/share\` 下。

这里有个陷阱。yuzu 式的目录结构是 \`save/<账户>/<用户档案>/<游戏ID>/\`，而用户档案 ID 是在模拟器首次运行时生成的，所以每次安装都不一样。如果在两台设备之间同步整个 \`save/\` 文件夹，每台都会在自己的档案旁边多出对方的档案，两边的游戏都看不到对方的进度。Hoard 则会深入到每个游戏自己的文件夹，因此无论档案叫什么，同一款游戏都能在设备之间对应上。

### Citra 和 Azahar（3DS）

存档藏得很深，位于 \`sdmc/Nintendo 3DS/<id0>/<id1>/title/…\`，而 \`id0\`/\`id1\` 由模拟主机的密钥生成，所以同样因安装而异。Hoard 的处理方式与 Switch 相同：每个游戏一个条目，在设备之间配对。

### 其他

- **Cemu（Wii U）：**\`mlc01/usr/save\`，位于 \`%APPDATA%\\Cemu\` 或 \`~/.local/share/Cemu\` 下。
- **shadPS4（PS4）：**\`savedata\`，位于 \`%APPDATA%\\shadPS4\` 或 \`~/.local/share/shadPS4\` 下。
- **Vita3K（PS Vita）：**其数据文件夹中的 \`ux0/user/00/savedata\`。
- **mGBA、melonDS 以及大多数卡带时代的模拟器：**除非另有设置，存档是 ROM 旁边的一个 \`.sav\`。请手动添加 ROM 文件夹中的存档。

## Steam Deck 上的模拟器存档

在 Steam Deck 上，模拟器通常来自 Flatpak，所以它们的文件夹位于 \`~/.var/app/<id>/\`，而不是常见的 \`~/.config\` 或 \`~/.local/share\`。EmuDeck 会把一切集中到 \`~/Emulation/saves/\`，每个模拟器一个文件夹。无论哪种方式，添加一次文件夹，Hoard 就会持续监视。

在掌机上最关键的一点：Hoard 的引擎作为后台服务运行，所以在游戏模式下退出游戏时，无需打开任何窗口就会完成备份。在台式机上玩完一局后拿起 Deck，存档已经在那里了。

## 存档文件和即时存档不是一回事

值得把两者分开，因为它们在迁移时表现不同：

- **存档文件**（\`.srm\`、记忆卡、\`SAVEDATA\` 文件夹）是游戏自己的存档，由被模拟的主机写入。它可以在设备之间、模拟器版本之间顺利迁移。
- **即时存档**是模拟器内存的快照。它依赖于模拟器的构建版本，往往还依赖于具体的核心，所以某个版本生成的即时存档可能在另一个版本中无法加载。

Hoard 会备份两者。只是如果更新过的设备上的即时存档在旧版本设备上打不开，不必惊讶——保持各设备上模拟器版本一致，重要的进度依靠存档文件。

## 一个模拟器，许多游戏

一个模拟器就是承载几十款游戏的单个进程，这正是模拟器存档让“以正在运行的游戏为单位”思考的工具难以处理的原因。Hoard 会把各个游戏分开，而不是把整个模拟器当成一个整体，因此每款游戏都有自己的历史，而不是一堆每次启动任何游戏都会变化的混合数据。如果某个存档出了问题，你可以[回滚到更早的版本](/guides/restore-a-game-save)。

## 不经过我们服务器的模拟器备份

以上内容在你自己的服务器上同样适用：运行 \`hoard-server\`，把应用指向它，你的存档就会从你的设备直接存到你的磁盘。无需我们的账号，不向我们发送遥测，不经过我们的任何服务器。参见[如何自托管 Hoard](/guides/self-host-hoard)。

## 提示

即时存档依赖于特定的模拟器版本。请在各台 PC 上一致地更新模拟器，让同步过来的即时存档在任何地方都能正常加载。

<!-- faq -->

## 常见问题

### Hoard 也会备份我的 ROM 吗？

不会。它跟踪的是存档文件夹，而不是游戏文件。ROM 体积大、不会变化，而且你本来就有——没有需要版本管理的东西。

### PCSX2、Dolphin 或 DuckStation 内置云存档吗？

没有。它们把存档写到本地文件夹，同步交给你自己。把同步工具指向上面列出的文件夹，存档就会跟着你在设备之间流转。

### RetroArch 有云同步吗？

有，内置的 Cloud Sync，但需要一台你自己运行或租用的 WebDAV 服务器。如果你不想配置 WebDAV、想要可回滚的版本历史，或者也在用独立模拟器，Hoard 就是替代方案。

### 在游戏模式下的 Steam Deck 上能用吗？

能。引擎作为后台服务运行，所以退出游戏时会自动备份存档，无需打开窗口。Flatpak 和 EmuDeck 的文件夹与其他文件夹用法相同。

### 我的模拟器是便携版，能用吗？

能。手动添加可执行文件旁边的文件夹，Hoard 会像对待其他存档位置一样跟踪它。这在掌机上是常见配置。

### 能在两台 PC 之间同步即时存档吗？

能，Hoard 会同步。即时存档能否加载取决于两台设备上的模拟器版本是否一致，这是模拟器的限制，而不是同步的问题。存档文件没有这个问题。

### 不在列表里的模拟器也能用吗？

几乎可以肯定。常见模拟器会自动识别，其他的只要把 Hoard 指向其存档文件夹即可添加。

### 自托管对模拟器有什么影响吗？

没有。同样的识别、同样的版本、同样的同步。只是存储归你所有。
`,Ee=`---
title: "So sicherst du deine Spielstände automatisch"
description: "Sichere deine PC-Spielstände nach jeder Session automatisch, mit Versionsverlauf, damit Absturz, Neuinstallation oder Mod nie deinen Fortschritt löschen."
order: 1
updated: 2026-10-02
---

Ein verlorener Spielstand bedeutet verlorene Stunden an Fortschritt. Hoard sichert deine PC-Spielstände automatisch und führt eine vollständige Versionshistorie, sodass du immer zurückgehen kannst.

## Was Hoard sichert

Hoard erkennt die Speicherordner der Spiele, die du spielst, und kopiert sie in deine eigene Cloud — entweder Hoard Cloud oder einen selbst gehosteten Server. Jedes Backup ist versioniert, ältere Kopien werden also nie überschrieben.

Um zu finden, wo jedes Spiel seine Stände ablegt, nutzt Hoard dieselbe Community-Datenbank für Speicherorte, die auch Ludusavi antreibt — die Erkennung funktioniert also sofort für Tausende von Titeln. Der Unterschied liegt darin, was danach passiert: Statt das Backup auf deiner Festplatte zu belassen, versioniert Hoard es automatisch in der Cloud.

## Automatische Backups einrichten

1. **Lade Hoard herunter und installiere es** für Windows, macOS oder Linux von der Download-Seite.
2. Melde dich an oder richte die App auf deinen selbst gehosteten Server aus.
3. Öffne die **Bibliothek**. Hoard sucht nach installierten Spielen und listet die gefundenen Stände auf.
4. Füge die Spiele hinzu, die du schützen willst. Hoard findet jeden Speicherordner automatisch; du kannst einen Pfad von Hand ergänzen, falls ein Spiel nicht erkannt wird.
5. Lass den **Automatikmodus** an. Hoard überwacht die Speicherordner und sichert sie, nachdem du aufhörst zu spielen.

Ab jetzt wird jede Sitzung erfasst, ohne dass du etwas tun musst.

## Wo PC-Spiele ihre Stände wirklich ablegen

Es gibt keinen einzigen Ort, und genau deshalb existiert so ein Werkzeug. In der Praxis landet ein Spielstand an einer dieser Stellen:

- **In Steam**, unter \`userdata/<UserID>/<AppID>/remote/\` — dem Ordner, den Steam Cloud selbst synchronisiert.
- **\`Dokumente\\My Games\\…\`**, das Nächste, was Windows an Konvention zu bieten hat.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` oder \`LocalLow\`**, wo die meisten Unity- und Unreal-Spiele schreiben.
- **\`%USERPROFILE%\\Saved Games\`**, genutzt von einer kleineren, aber hartnäckigen Gruppe von Titeln.
- **Im Installationsordner des Spiels selbst**, wo erstaunlich viele ältere Titel weiterhin speichern.
- **Unter Linux** \`~/.local/share\` oder \`~/.config\` für native Spiele, und im Proton-Prefix — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — für Windows-Spiele.
- **Unter macOS** \`~/Library/Application Support\`.

Woher das Spiel stammt, spielt kaum eine Rolle: Titel von GOG, Epic und itch landen an derselben Handvoll Orte, denn das entscheiden Engine und Entwickler, nicht der Store.

Für einige beliebte Spiele gibt es eine eigene Seite mit den genauen Pfaden, dem Ordnerinhalt und den Fallen, die man meiden sollte: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Crimson Desert](/guides/crimson-desert-save-location), [Marvel's Spider-Man 2](/guides/spider-man-2-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location) und [Palworld](/guides/palworld-save-location).

## Was gesichert wird und was nicht

Ein Speicherordner enthält selten nur Spielstände, deshalb sortiert Hoard, was es findet, auf drei Stapel:

- **Spielstanddaten** werden gesichert und wiederhergestellt. Das ist dein Fortschritt.
- **Dateien, die zu einem bestimmten Rechner gehören** — Konfiguration, Logs und Ähnliches — werden hochgeladen, damit sie Teil des Backups sind, aber nie über die Kopie eines anderen PCs geschrieben. Deine Grafikeinstellungen bleiben deine.
- **Müll** — Caches, Absturzberichte, temporäre Dateien — wird ignoriert, damit ein Backup nicht mit Dingen aufquillt, die du nie zurückhaben willst.

## Wann gesichert wird

Hoard beobachtet den Ordner und sichert ihn, **nachdem du aufgehört hast zu spielen**, nicht während ein Spiel Dateien offen hält. Wurde der Stand vor Sekunden geschrieben, wartet es, bis Ruhe einkehrt: eine Datei im Schreibvorgang ist keine Datei, die man halb sichern will.

Jede Sicherung ist eine Version. Snapshots werden per Inhalts-Hash gespeichert, unveränderte Dateien also nur einmal — zehn Versionen eines 2 GB großen Stands kosten etwa 2 GB, nicht 20.

## Sichern ohne unsere Server

Wenn du lieber niemandes Cloud nutzt, betreibe \`hoard-server\` selbst und richte die App darauf. Deine Stände gehen von deinem PC auf deine Platte: kein Konto bei uns, keine Telemetrie zu uns, und nichts, was über unsere Server läuft. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

## Tipp: Prüfe deine Historie

Öffne den Reiter **Historie** eines Spiels, um jedes Backup mit Datum und Größe zu sehen. Von dort kannst du jede frühere Version mit einem Klick wiederherstellen. Deine Stände werden verschlüsselt übertragen, in der EU gespeichert, und du kannst sie jederzeit exportieren oder löschen.

Nutzt du bereits ein lokales Backup-Tool wie Ludusavi? Du kannst es behalten — aber wenn diese Backups in der Cloud landen und zwischen Geräten synchronisieren sollen, ohne dass du Rclone selbst einrichtest, ist genau das, was Hoard automatisiert. Siehe [Ludusavi vs. Hoard](/guides/ludusavi-alternative) für einen fairen Vergleich.

<!-- faq -->

## Häufige Fragen

### Sichert Hoard, während ich spiele?

Nein. Es wartet, bis du aufhörst und der Speicherordner zur Ruhe kommt, damit ein Backup nie eine halb geschriebene Datei ist.

### Wie viel Platz brauchen meine Spielstände?

Weniger als gedacht. Versionen werden per Inhalts-Hash dedupliziert, neuen Platz belegt also nur, was sich zwischen zwei Sitzungen wirklich geändert hat — die meisten Sammlungen passen bequem in ein paar Gigabyte.

### Was, wenn eines meiner Spiele nicht erkannt wird?

Richte Hoard von Hand auf den Ordner, dann verfolgt es ihn wie jeden anderen. Die Erkennung deckt Tausende Titel ab, aber ein Spiel, das an einer ungewöhnlichen Stelle speichert oder das du von Hand installiert hast, braucht manchmal den Hinweis.

### Sichert es auch meine Mods?

Hoard verfolgt den Speicherordner, Mods an anderer Stelle sind also nicht Teil des Backups. Das ist Absicht: Mods sind groß, sie lassen sich neu herunterladen, und ein zwischen Rechnern synchronisierter Mod-Ordner schafft mehr Probleme, als er löst.

### Ändert Selbsthosten etwas an den Backups?

Überhaupt nicht. Gleiche Erkennung, gleiche Versionen, gleiche automatische Sicherung. Nur der Speicher gehört dir.
`,Me=`---
title: "How to back up your game saves automatically"
description: "Back up PC game saves automatically after every session, with version history, so a crash, reinstall or bad mod never wipes your progress."
order: 1
updated: 2026-10-02
related: restore-a-game-save, sync-game-saves-across-pcs, steam-cloud-alternative
---

Losing a save file means losing hours of progress. Hoard backs up your PC game saves automatically and keeps a full version history, so you can always go back.

## What Hoard backs up

Hoard detects the save folders of the games you play and copies them to your own cloud — either Hoard Cloud or a server you host yourself. Every backup is versioned, so older copies are never overwritten.

To find where each game stores its saves, Hoard reads the same community save-location database that powers Ludusavi, so detection works out of the box for thousands of titles. The difference is what happens next: instead of leaving the backup on your disk, Hoard versions it in the cloud automatically.

## Set up automatic backups

1. **Download and install Hoard** for Windows, macOS or Linux from the download page.
2. Sign in, or point the app at your self-hosted server.
3. Open the **Library**. Hoard scans for installed games and lists the saves it finds.
4. Add the games you want to protect. Hoard locates each save folder automatically; you can add a path by hand if a game isn't detected.
5. Leave **automatic mode** on. Hoard watches the save folders and backs them up after you stop playing.

From now on every session is captured without you doing anything.

## Where PC games actually keep their saves

There is no single place, which is the whole reason a tool like this exists. In practice a save ends up in one of these:

- **Inside Steam**, at \`userdata/<UserID>/<AppID>/remote/\` — the folder Steam Cloud itself syncs.
- **\`Documents\\My Games\\…\`**, the closest thing Windows has to a convention.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` or \`LocalLow\`** — where most Unity and Unreal games write.
- **\`%USERPROFILE%\\Saved Games\`**, used by a smaller but stubborn set of titles.
- **The game's own install folder**, which is where a surprising number of older titles still save.
- **On Linux**, \`~/.local/share\` or \`~/.config\` for native games, and inside the Proton prefix — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — for Windows games.
- **On macOS**, \`~/Library/Application Support\`.

Where the game came from barely matters: GOG, Epic and itch titles land in the same handful of places, because it's the engine and the developer that decide, not the launcher.

For a few popular games there's a page with the exact paths, what's in the folder and the traps to avoid: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Crimson Desert](/guides/crimson-desert-save-location), [Marvel's Spider-Man 2](/guides/spider-man-2-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location) and [Palworld](/guides/palworld-save-location).

## What gets backed up, and what doesn't

A save folder is rarely just saves, so Hoard sorts what it finds into three piles:

- **Save data** is backed up and restored. This is your progress.
- **Files that belong to one machine** — configuration, logs, and similar — are uploaded so they're part of the backup, but never written back over another PC's copy. Your graphics settings stay yours.
- **Junk** — caches, crash dumps, temporary files — is ignored, so a backup doesn't balloon with things you'd never want back.

## When a backup happens

Hoard watches the folder and captures it **after you stop playing**, not while a game is holding files open. If the save was written to seconds ago, it waits until things go quiet: a file being written is not a file worth capturing halfway.

Each capture is a version. Snapshots are stored by content hash, so unchanged files are stored once — ten versions of a 2 GB save cost about 2 GB, not 20.

## Backing up without our servers

If you'd rather not use anyone's cloud, run \`hoard-server\` yourself and point the app at it. Your saves go from your PC to your disk: no account with us, no telemetry to us, and nothing passing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

## Tip: check your history

Open a game's **History** tab to see every backup with its date and size. From there you can restore any previous version in one click. Your saves travel encrypted, are stored in the EU, and you can export or delete them whenever you want.

Already use a local backup tool like Ludusavi? You can keep it — but if you want those backups to land in the cloud and sync between machines without scripting Rclone yourself, that's exactly what Hoard automates. See [Ludusavi vs Hoard](/guides/ludusavi-alternative) for a fair comparison.

<!-- faq -->

## Frequently asked questions

### Does Hoard back up while I'm playing?

No. It waits until you stop and the save folder goes quiet, so a backup is never a half-written file.

### How much space do my saves need?

Less than you'd think. Versions are deduplicated by content hash, so only what actually changed between sessions takes new space — most save collections sit comfortably in a couple of gigabytes.

### What if one of my games isn't detected?

Point Hoard at the folder by hand and it will track it like any other. Detection covers thousands of titles, but a game that saves somewhere unusual, or one you installed by hand, sometimes needs the hint.

### Does it back up my mods?

Hoard tracks the save folder, so mods living elsewhere aren't part of the backup. That's deliberate: mods are large, they're re-downloadable, and a mod folder syncing between machines causes more problems than it solves.

### Does self-hosting change how backups work?

Not at all. Same detection, same versions, same automatic capture. Only the storage is yours.
`,We=`---
title: "Cómo hacer copias de seguridad de tus partidas automáticamente"
description: "Haz copia de tus partidas de PC automáticamente tras cada sesión, con historial de versiones, para que un fallo, una reinstalación o un mod no te borren nada."
order: 1
updated: 2026-10-02
---

Perder una partida guardada significa perder horas de progreso. Hoard hace copias de seguridad de tus partidas de PC automáticamente y guarda un historial completo de versiones, para que siempre puedas volver atrás.

## Qué guarda Hoard

Hoard detecta las carpetas de guardado de los juegos a los que juegas y las copia a tu propia nube: Hoard Cloud o un servidor que alojes tú mismo. Cada copia está versionada, así que las versiones antiguas nunca se sobrescriben.

Para saber dónde guarda cada juego sus partidas, Hoard usa la misma base de datos comunitaria de ubicaciones que utiliza Ludusavi, así que la detección funciona desde el primer momento con miles de títulos. La diferencia está en lo que pasa después: en vez de dejar la copia en tu disco, Hoard la versiona en la nube automáticamente.

## Configura las copias automáticas

1. **Descarga e instala Hoard** para Windows, macOS o Linux desde la página de descargas.
2. Inicia sesión o apunta la app a tu servidor autoalojado.
3. Abre la **Biblioteca**. Hoard busca los juegos instalados y lista las partidas que encuentra.
4. Añade los juegos que quieras proteger. Hoard localiza cada carpeta de guardado automáticamente; puedes añadir una ruta a mano si un juego no se detecta.
5. Deja activado el **modo automático**. Hoard vigila las carpetas de guardado y hace la copia cuando dejas de jugar.

A partir de ahí cada sesión queda guardada sin que hagas nada.

## Dónde guardan realmente sus partidas los juegos de PC

No hay un único sitio, y ése es justo el motivo de que exista una herramienta así. En la práctica, una partida acaba en alguno de estos lugares:

- **Dentro de Steam**, en \`userdata/<UserID>/<AppID>/remote/\`, la carpeta que sincroniza el propio Steam Cloud.
- **\`Documentos\\My Games\\…\`**, lo más parecido a una convención que tiene Windows.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` o \`LocalLow\`**, donde escriben la mayoría de juegos de Unity y Unreal.
- **\`%USERPROFILE%\\Saved Games\`**, que usa un grupo más pequeño pero tozudo de títulos.
- **La propia carpeta de instalación del juego**, donde todavía guardan sorprendentes cantidades de títulos antiguos.
- **En Linux**, \`~/.local/share\` o \`~/.config\` para los juegos nativos, y dentro del prefijo de Proton — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — para los de Windows.
- **En macOS**, \`~/Library/Application Support\`.

De dónde venga el juego importa poco: los de GOG, Epic e itch caen en el mismo puñado de sitios, porque lo deciden el motor y el desarrollador, no la tienda.

Para algunos juegos populares hay una página con las rutas exactas, qué hay en la carpeta y las trampas que evitar: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Crimson Desert](/guides/crimson-desert-save-location), [Marvel's Spider-Man 2](/guides/spider-man-2-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location) y [Palworld](/guides/palworld-save-location).

## Qué se copia y qué no

Una carpeta de partidas rara vez contiene sólo partidas, así que Hoard reparte lo que encuentra en tres montones:

- **Los datos de partida** se copian y se restauran. Eso es tu progreso.
- **Los ficheros que son de una máquina concreta** — configuración, registros y similares — se suben para que formen parte de la copia, pero nunca se escriben encima de la copia de otro PC. Tus ajustes gráficos siguen siendo tuyos.
- **La basura** — cachés, volcados de fallos, temporales — se ignora, para que una copia no se hinche con cosas que nunca querrías de vuelta.

## Cuándo se hace la copia

Hoard vigila la carpeta y la captura **cuando dejas de jugar**, no mientras el juego tiene los ficheros abiertos. Si la partida se escribió hace unos segundos, espera a que la cosa se calme: un fichero que se está escribiendo no es un fichero que merezca capturarse a medias.

Cada captura es una versión. Las instantáneas se guardan por hash de contenido, así que un fichero que no cambia se almacena una sola vez: diez versiones de una partida de 2 GB ocupan unos 2 GB, no 20.

## Copias sin pasar por nuestros servidores

Si prefieres no usar la nube de nadie, levanta \`hoard-server\` tú mismo y apunta la aplicación ahí. Tus partidas van de tu PC a tu disco: sin cuenta con nosotros, sin telemetría hacia nosotros y sin nada que pase por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

## Consejo: revisa tu historial

Abre la pestaña **Historial** de un juego para ver cada copia con su fecha y tamaño. Desde ahí puedes restaurar cualquier versión anterior con un clic. Tus partidas viajan cifradas, se almacenan en la UE y puedes exportarlas o borrarlas cuando quieras.

¿Ya usas una herramienta de copia local como Ludusavi? Puedes seguir usándola, pero si quieres que esas copias acaben en la nube y se sincronicen entre equipos sin montar Rclone a mano, eso es justo lo que Hoard automatiza. Mira [Ludusavi frente a Hoard](/guides/ludusavi-alternative) para una comparativa justa.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard hace copias mientras juego?

No. Espera a que salgas y a que la carpeta de partidas se quede quieta, así que una copia nunca es un fichero a medio escribir.

### ¿Cuánto espacio necesitan mis partidas?

Menos del que imaginas. Las versiones se deduplican por hash de contenido, así que sólo ocupa espacio nuevo lo que cambió de verdad entre sesiones: la mayoría de colecciones caben de sobra en un par de gigas.

### ¿Y si uno de mis juegos no se detecta?

Apunta Hoard a la carpeta a mano y la rastreará como cualquier otra. La detección cubre miles de títulos, pero un juego que guarde en un sitio raro, o que hayas instalado a mano, a veces necesita la pista.

### ¿Copia también mis mods?

Hoard rastrea la carpeta de partidas, así que los mods que vivan en otro sitio no entran en la copia. Es deliberado: los mods son grandes, se vuelven a descargar, y una carpeta de mods sincronizándose entre máquinas da más problemas de los que resuelve.

### ¿Cambia algo si me autoalojo?

Nada. La misma detección, las mismas versiones, la misma captura automática. Lo único tuyo es el almacenamiento.
`,Ie=`---
title: "Comment sauvegarder vos parties automatiquement"
description: "Sauvegardez vos parties PC automatiquement après chaque session, avec historique, pour qu'un crash, une réinstallation ou un mod n'efface rien."
order: 1
updated: 2026-10-02
---

Perdre une sauvegarde, c'est perdre des heures de progression. Hoard sauvegarde vos parties PC automatiquement et conserve un historique complet des versions, pour que vous puissiez toujours revenir en arrière.

## Ce que Hoard sauvegarde

Hoard détecte les dossiers de sauvegarde des jeux auxquels vous jouez et les copie vers votre propre cloud — Hoard Cloud ou un serveur que vous hébergez vous-même. Chaque sauvegarde est versionnée, les anciennes copies ne sont donc jamais écrasées.

Pour trouver où chaque jeu range ses sauvegardes, Hoard utilise la même base de données communautaire d'emplacements que celle qui alimente Ludusavi : la détection fonctionne donc d'emblée pour des milliers de titres. La différence, c'est ce qui se passe ensuite : au lieu de laisser la sauvegarde sur votre disque, Hoard la versionne automatiquement dans le cloud.

## Configurer les sauvegardes automatiques

1. **Téléchargez et installez Hoard** pour Windows, macOS ou Linux depuis la page de téléchargement.
2. Connectez-vous, ou pointez l'application vers votre serveur auto-hébergé.
3. Ouvrez la **Bibliothèque**. Hoard recherche les jeux installés et liste les sauvegardes trouvées.
4. Ajoutez les jeux à protéger. Hoard localise chaque dossier de sauvegarde automatiquement ; vous pouvez ajouter un chemin à la main si un jeu n'est pas détecté.
5. Laissez le **mode automatique** activé. Hoard surveille les dossiers de sauvegarde et les sauvegarde après que vous arrêtez de jouer.

Désormais, chaque session est capturée sans que vous ayez à faire quoi que ce soit.

## Où les jeux PC rangent vraiment leurs sauvegardes

Il n'y a pas d'endroit unique, et c'est précisément pour ça qu'un outil comme celui-ci existe. En pratique, une sauvegarde atterrit dans l'un de ces endroits :

- **Dans Steam**, sous \`userdata/<UserID>/<AppID>/remote/\` — le dossier que Steam Cloud synchronise lui-même.
- **\`Documents\\My Games\\…\`**, ce qui se rapproche le plus d'une convention sous Windows.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` ou \`LocalLow\`**, où écrivent la plupart des jeux Unity et Unreal.
- **\`%USERPROFILE%\\Saved Games\`**, utilisé par un groupe plus restreint mais tenace de titres.
- **Le dossier d'installation du jeu lui-même**, où un nombre surprenant de titres anciens sauvegardent encore.
- **Sous Linux**, \`~/.local/share\` ou \`~/.config\` pour les jeux natifs, et dans le préfixe Proton — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — pour les jeux Windows.
- **Sous macOS**, \`~/Library/Application Support\`.

La provenance du jeu ne change presque rien : les titres GOG, Epic et itch atterrissent dans la même poignée d'endroits, car ce sont le moteur et le développeur qui décident, pas la boutique.

Pour quelques jeux populaires, une page détaille les chemins exacts, le contenu du dossier et les pièges à éviter : [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Crimson Desert](/guides/crimson-desert-save-location), [Marvel's Spider-Man 2](/guides/spider-man-2-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location) et [Palworld](/guides/palworld-save-location).

## Ce qui est sauvegardé, et ce qui ne l'est pas

Un dossier de sauvegarde ne contient presque jamais que des sauvegardes, alors Hoard trie ce qu'il trouve en trois tas :

- **Les données de sauvegarde** sont sauvegardées et restaurées. C'est votre progression.
- **Les fichiers propres à une machine** — configuration, journaux et compagnie — sont envoyés pour faire partie de la sauvegarde, mais jamais réécrits par-dessus la copie d'un autre PC. Vos réglages graphiques restent les vôtres.
- **Le déchet** — caches, rapports de plantage, fichiers temporaires — est ignoré, pour qu'une sauvegarde n'enfle pas avec ce que vous ne voudriez jamais récupérer.

## Quand la sauvegarde a lieu

Hoard surveille le dossier et le capture **après que vous avez arrêté de jouer**, pas pendant qu'un jeu garde des fichiers ouverts. Si la sauvegarde a été écrite il y a quelques secondes, il attend que le calme revienne : un fichier en cours d'écriture ne mérite pas d'être capturé à moitié.

Chaque capture est une version. Les instantanés sont stockés par empreinte de contenu : un fichier inchangé n'est stocké qu'une fois — dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20.

## Sauvegarder sans passer par nos serveurs

Si vous préférez n'utiliser le cloud de personne, faites tourner \`hoard-server\` vous-même et pointez l'application dessus. Vos sauvegardes vont de votre PC à votre disque : aucun compte chez nous, aucune télémétrie vers nous, et rien qui passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

## Astuce : consultez votre historique

Ouvrez l'onglet **Historique** d'un jeu pour voir chaque sauvegarde avec sa date et sa taille. De là, vous pouvez restaurer n'importe quelle version précédente en un clic. Vos sauvegardes circulent chiffrées, sont stockées dans l'UE, et vous pouvez les exporter ou les supprimer quand vous voulez.

Vous utilisez déjà un outil de sauvegarde locale comme Ludusavi ? Vous pouvez le garder — mais si vous voulez que ces sauvegardes arrivent dans le cloud et se synchronisent entre vos machines sans scripter Rclone vous-même, c'est précisément ce que Hoard automatise. Voir [Ludusavi vs Hoard](/guides/ludusavi-alternative) pour une comparaison équitable.

<!-- faq -->

## Questions fréquentes

### Hoard sauvegarde-t-il pendant que je joue ?

Non. Il attend que vous ayez arrêté et que le dossier se calme, pour qu'une sauvegarde ne soit jamais un fichier à moitié écrit.

### Quelle place prennent mes sauvegardes ?

Moins qu'on ne croit. Les versions sont dédupliquées par empreinte de contenu : seule la partie réellement modifiée entre deux sessions occupe de la place — la plupart des collections tiennent largement dans quelques gigaoctets.

### Et si l'un de mes jeux n'est pas détecté ?

Pointez Hoard sur le dossier à la main et il le suivra comme les autres. La détection couvre des milliers de titres, mais un jeu qui sauvegarde à un endroit inhabituel, ou que vous avez installé à la main, a parfois besoin de l'indice.

### Est-ce qu'il sauvegarde mes mods ?

Hoard suit le dossier de sauvegarde : les mods rangés ailleurs ne font pas partie de la sauvegarde. C'est volontaire — les mods sont volumineux, ils se retéléchargent, et un dossier de mods synchronisé entre machines crée plus de problèmes qu'il n'en résout.

### L'auto-hébergement change-t-il quelque chose aux sauvegardes ?

Rien du tout. Même détection, mêmes versions, même capture automatique. Seul le stockage est à vous.
`,Re=`---
title: "Come fare il backup dei salvataggi automaticamente"
description: "Backup automatico dei salvataggi PC dopo ogni sessione, con cronologia delle versioni: un crash, una reinstallazione o una mod non cancellano più nulla."
order: 1
updated: 2026-10-02
---

Perdere un salvataggio significa perdere ore di progressi. Hoard fa il backup dei tuoi salvataggi PC automaticamente e conserva una cronologia completa delle versioni, così puoi sempre tornare indietro.

## Cosa salva Hoard

Hoard rileva le cartelle di salvataggio dei giochi a cui giochi e le copia sul tuo cloud — Hoard Cloud o un server che ospiti tu stesso. Ogni backup è versionato, quindi le copie più vecchie non vengono mai sovrascritte.

Per trovare dove ogni gioco conserva i salvataggi, Hoard usa lo stesso database comunitario di posizioni che alimenta Ludusavi, quindi il rilevamento funziona da subito per migliaia di titoli. La differenza è ciò che succede dopo: invece di lasciare il backup sul disco, Hoard lo versiona automaticamente nel cloud.

## Imposta i backup automatici

1. **Scarica e installa Hoard** per Windows, macOS o Linux dalla pagina di download.
2. Accedi, oppure punta l'app al tuo server self-hosted.
3. Apri la **Libreria**. Hoard cerca i giochi installati ed elenca i salvataggi trovati.
4. Aggiungi i giochi che vuoi proteggere. Hoard individua ogni cartella di salvataggio automaticamente; puoi aggiungere un percorso a mano se un gioco non viene rilevato.
5. Lascia attiva la **modalità automatica**. Hoard sorveglia le cartelle di salvataggio e fa il backup dopo che smetti di giocare.

Da ora ogni sessione viene catturata senza che tu faccia nulla.

## Dove i giochi PC tengono davvero i salvataggi

Non esiste un posto solo, ed è esattamente il motivo per cui uno strumento così esiste. Nella pratica un salvataggio finisce in uno di questi punti:

- **Dentro Steam**, in \`userdata/<UserID>/<AppID>/remote/\`, la cartella che Steam Cloud sincronizza per conto suo.
- **\`Documenti\\My Games\\…\`**, la cosa più simile a una convenzione che Windows abbia.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` o \`LocalLow\`**, dove scrive la maggior parte dei giochi Unity e Unreal.
- **\`%USERPROFILE%\\Saved Games\`**, usata da un gruppo più ristretto ma testardo di titoli.
- **La cartella di installazione del gioco**, dove sorprendentemente molti titoli vecchi salvano ancora.
- **Su Linux**, \`~/.local/share\` o \`~/.config\` per i giochi nativi, e dentro il prefisso Proton — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — per quelli Windows.
- **Su macOS**, \`~/Library/Application Support\`.

Da dove arrivi il gioco conta poco: i titoli GOG, Epic e itch finiscono negli stessi pochi posti, perché a decidere sono il motore e lo sviluppatore, non il negozio.

Per alcuni giochi popolari c'è una pagina con i percorsi esatti, cosa c'è nella cartella e le trappole da evitare: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Crimson Desert](/guides/crimson-desert-save-location), [Marvel's Spider-Man 2](/guides/spider-man-2-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location) e [Palworld](/guides/palworld-save-location).

## Cosa viene salvato e cosa no

Una cartella di salvataggi contiene raramente solo salvataggi, quindi Hoard divide ciò che trova in tre mucchi:

- **I dati di salvataggio** vengono copiati e ripristinati. Quelli sono i tuoi progressi.
- **I file che appartengono a una macchina specifica** — configurazione, log e simili — vengono caricati per far parte del backup, ma mai riscritti sopra la copia di un altro PC. Le tue impostazioni grafiche restano tue.
- **La spazzatura** — cache, dump dei crash, temporanei — viene ignorata, così un backup non si gonfia con roba che non rivorresti mai.

## Quando avviene il backup

Hoard sorveglia la cartella e la cattura **dopo che smetti di giocare**, non mentre il gioco tiene i file aperti. Se il salvataggio è stato scritto pochi secondi fa, aspetta che tutto si calmi: un file in scrittura non è un file da catturare a metà.

Ogni cattura è una versione. Gli snapshot sono archiviati per hash del contenuto, quindi un file invariato viene salvato una volta sola: dieci versioni di un salvataggio da 2 GB occupano circa 2 GB, non 20.

## Backup senza passare dai nostri server

Se preferisci non usare il cloud di nessuno, fai girare \`hoard-server\` per conto tuo e punta l'app lì. I salvataggi vanno dal tuo PC al tuo disco: nessun account con noi, nessuna telemetria verso di noi e niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

## Suggerimento: controlla la cronologia

Apri la scheda **Cronologia** di un gioco per vedere ogni backup con data e dimensione. Da lì puoi ripristinare qualsiasi versione precedente con un clic. I tuoi salvataggi viaggiano cifrati, sono archiviati nell'UE, e puoi esportarli o eliminarli quando vuoi.

Usi già uno strumento di backup locale come Ludusavi? Puoi tenerlo — ma se vuoi che quei backup finiscano nel cloud e si sincronizzino tra le macchine senza scriptare Rclone a mano, è esattamente ciò che Hoard automatizza. Vedi [Ludusavi vs Hoard](/guides/ludusavi-alternative) per un confronto equo.

<!-- faq -->

## Domande frequenti

### Hoard fa backup mentre gioco?

No. Aspetta che tu smetta e che la cartella dei salvataggi si calmi, così un backup non è mai un file scritto a metà.

### Quanto spazio occupano i miei salvataggi?

Meno di quanto pensi. Le versioni sono deduplicate per hash del contenuto, quindi occupa spazio nuovo solo ciò che è davvero cambiato tra una sessione e l'altra: quasi tutte le collezioni stanno comode in un paio di gigabyte.

### E se uno dei miei giochi non viene rilevato?

Punta Hoard alla cartella a mano e la traccerà come qualsiasi altra. Il rilevamento copre migliaia di titoli, ma un gioco che salva in un posto insolito, o installato a mano, a volte ha bisogno dell'indizio.

### Fa il backup anche delle mod?

Hoard traccia la cartella dei salvataggi, quindi le mod che stanno altrove non entrano nel backup. È voluto: le mod sono grandi, si riscaricano, e una cartella di mod sincronizzata tra macchine crea più problemi di quanti ne risolva.

### Il self-hosting cambia il funzionamento dei backup?

Per niente. Stesso rilevamento, stesse versioni, stessa cattura automatica. L'unica cosa tua è lo spazio di archiviazione.
`,_e=`---
title: "ゲームのセーブデータを自動でバックアップする方法"
description: "プレイ終了ごとにPCのセーブを自動バックアップ。バージョン履歴付きなので、クラッシュや再インストール、壊れたModでも進行を失いません。"
order: 1
updated: 2026-10-02
---

セーブデータを失うことは、何時間もの進行を失うことです。Hoard は PC ゲームのセーブデータを自動でバックアップし、完全なバージョン履歴を保持するので、いつでも巻き戻せます。

## Hoard がバックアップするもの

Hoard はプレイしているゲームのセーブフォルダーを検出し、あなた自身のクラウド（Hoard Cloud または自分でホストするサーバー）へコピーします。各バックアップは世代管理されるため、古いコピーが上書きされることはありません。

各ゲームがどこにセーブを保存しているかを見つけるために、Hoard は Ludusavi を支えているのと同じコミュニティのセーブ位置データベースを利用します。そのため数千タイトルで検出がすぐに機能します。違いはその後にあります。バックアップをディスクに残すのではなく、Hoard は自動的にクラウドで世代管理します。

## 自動バックアップを設定する

1. ダウンロードページから Windows、macOS、Linux 向けの **Hoard をダウンロードしてインストール** します。
2. サインインするか、アプリを自分のセルフホストサーバーに向けます。
3. **ライブラリ** を開きます。Hoard がインストール済みのゲームを探し、見つけたセーブを一覧表示します。
4. 保護したいゲームを追加します。Hoard は各セーブフォルダーを自動で特定します。ゲームが検出されない場合は手動でパスを追加できます。
5. **自動モード** をオンのままにします。Hoard はセーブフォルダーを監視し、プレイを終えた後にバックアップします。

これ以降、何もしなくても毎回のセッションが記録されます。

## PC ゲームのセーブは実際どこに置かれるのか

置き場所は 1 か所に決まっていません。こういうツールが必要になる理由が、まさにそこにあります。実際には次のどこかに落ち着きます。

- **Steam の中**、\`userdata/<UserID>/<AppID>/remote/\`。Steam クラウド自身が同期するフォルダーです。
- **\`ドキュメント\\My Games\\…\`**。Windows にある慣習らしきものの中では、いちばん近いもの。
- **\`%APPDATA%\`、\`%LOCALAPPDATA%\`、\`LocalLow\`**。Unity や Unreal のゲームの多くはここに書きます。
- **\`%USERPROFILE%\\Saved Games\`**。数は少ないものの、頑固に使い続けるタイトル群があります。
- **ゲームのインストールフォルダーそのもの**。古いタイトルには、いまだにここへ保存するものが驚くほどあります。
- **Linux** では、ネイティブのゲームは \`~/.local/share\` か \`~/.config\`、Windows 版のゲームは Proton プレフィックスの中、\`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`。
- **macOS** では \`~/Library/Application Support\`。

ゲームの入手元はほとんど関係ありません。GOG、Epic、itch のタイトルも同じ数か所に落ち着きます。決めているのはストアではなく、エンジンと開発者だからです。

人気のあるいくつかのゲームについては、正確なパス、フォルダーの中身、避けるべき落とし穴をまとめたページがあります: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [紅の砂漠](/guides/crimson-desert-save-location), [Marvel's Spider-Man 2](/guides/spider-man-2-save-location), [バルダーズ・ゲート3](/guides/baldurs-gate-3-save-location)、[パルワールド](/guides/palworld-save-location)。

## 何がバックアップされ、何がされないか

セーブフォルダーの中身がセーブだけということはまずないので、Hoard は見つけたものを 3 つに仕分けます。

- **セーブデータ** はバックアップされ、復元されます。これがあなたの進行です。
- **特定のマシンに属するファイル**、つまり設定やログなどはバックアップに含めるためアップロードされますが、他の PC のコピーを上書きすることはありません。グラフィック設定はそのマシンのものであり続けます。
- **ゴミ**、つまりキャッシュやクラッシュダンプ、一時ファイルは無視されます。二度と要らないもので、バックアップが膨らまないようにするためです。

## バックアップが行われるタイミング

Hoard はフォルダーを監視し、**プレイを終えたあと** に取り込みます。ゲームがファイルを開いている最中には行いません。数秒前にセーブが書かれたばかりなら、落ち着くまで待ちます。書き込み中のファイルは、半端な状態で取り込む価値がないからです。

取り込みのたびに 1 つの世代ができます。スナップショットは内容ハッシュで保存されるため、変わっていないファイルは一度だけ保存されます。2 GB のセーブの 10 世代は約 20 GB ではなく約 2 GB です。

## 当方のサーバーを介さないバックアップ

誰のクラウドも使いたくない場合は、\`hoard-server\` を自分で動かし、アプリをそこに向けてください。セーブは自分の PC から自分のディスクへ移ります。当方のアカウントも、当方へのテレメトリも、当方のサーバーを通るものもありません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

## ヒント：履歴を確認する

ゲームの **履歴** タブを開くと、各バックアップを日付とサイズ付きで確認できます。そこからどの過去バージョンもワンクリックで復元できます。セーブは暗号化されて転送され、EU 内に保存され、いつでもエクスポートや削除が可能です。

すでに Ludusavi のようなローカルバックアップツールを使っていますか？ そのまま使い続けても構いません。ただし、それらのバックアップをクラウドに送り、Rclone を自分でスクリプトせずに端末間で同期したいなら、まさにそれを Hoard が自動化します。公平な比較は [Ludusavi と Hoard](/guides/ludusavi-alternative) をご覧ください。

<!-- faq -->

## よくある質問

### プレイ中もバックアップされますか？

いいえ。プレイを終えてセーブフォルダーが静かになるまで待つので、書き込み途中のファイルがバックアップになることはありません。

### セーブにはどれくらいの容量が必要ですか？

思っているより少なくて済みます。世代は内容ハッシュで重複排除されるため、新たに容量を使うのはセッション間で実際に変わった分だけです。多くの場合、数ギガバイトに余裕で収まります。

### 検出されないゲームがある場合は？

そのフォルダーを手動で指定すれば、他と同じように追跡します。検出は数千タイトルをカバーしますが、変わった場所に保存するゲームや、手動でインストールしたものには、ヒントが要ることがあります。

### Mod もバックアップされますか？

Hoard が追跡するのはセーブフォルダーなので、別の場所にある Mod はバックアップに入りません。これは意図的です。Mod は容量が大きく、再ダウンロードでき、マシン間で同期すると解決するより多くの問題を生むからです。

### セルフホストするとバックアップの動きは変わりますか？

まったく変わりません。同じ検出、同じ世代、同じ自動取り込みです。自分のものになるのは保存先だけです。
`,Te=`---
title: "Como fazer backup dos teus saves automaticamente"
description: "Faz backup dos saves do PC automaticamente após cada sessão, com histórico de versões, para que um crash, reinstalação ou mod não apague o teu progresso."
order: 1
updated: 2026-10-02
---

Perder um save significa perder horas de progresso. O Hoard faz backup dos teus saves de PC automaticamente e guarda um histórico completo de versões, para que possas sempre voltar atrás.

## O que o Hoard guarda

O Hoard deteta as pastas de save dos jogos a que jogas e copia-as para a tua própria nuvem — Hoard Cloud ou um servidor que alojes tu mesmo. Cada backup é versionado, por isso as cópias antigas nunca são sobrescritas.

Para encontrar onde cada jogo guarda os saves, o Hoard usa a mesma base de dados comunitária de localizações que alimenta o Ludusavi, por isso a deteção funciona logo para milhares de títulos. A diferença está no que acontece a seguir: em vez de deixar o backup no teu disco, o Hoard versiona-o automaticamente na nuvem.

## Configurar backups automáticos

1. **Descarrega e instala o Hoard** para Windows, macOS ou Linux a partir da página de download.
2. Inicia sessão, ou aponta a app para o teu servidor self-hosted.
3. Abre a **Biblioteca**. O Hoard procura jogos instalados e lista os saves que encontra.
4. Adiciona os jogos que queres proteger. O Hoard localiza cada pasta de save automaticamente; podes adicionar um caminho à mão se um jogo não for detetado.
5. Deixa o **modo automático** ligado. O Hoard vigia as pastas de save e faz backup quando paras de jogar.

A partir daí cada sessão é capturada sem que faças nada.

## Onde os jogos de PC guardam mesmo os saves

Não há um sítio único, e é exatamente por isso que uma ferramenta destas existe. Na prática, um save acaba num destes lugares:

- **Dentro da Steam**, em \`userdata/<UserID>/<AppID>/remote/\` — a pasta que a própria Steam Cloud sincroniza.
- **\`Documentos\\My Games\\…\`**, o mais parecido com uma convenção que o Windows tem.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` ou \`LocalLow\`**, onde escrevem a maioria dos jogos Unity e Unreal.
- **\`%USERPROFILE%\\Saved Games\`**, usada por um grupo menor mas teimoso de títulos.
- **A própria pasta de instalação do jogo**, onde ainda guardam surpreendentemente muitos títulos antigos.
- **No Linux**, \`~/.local/share\` ou \`~/.config\` para jogos nativos, e dentro do prefixo Proton — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — para os de Windows.
- **No macOS**, \`~/Library/Application Support\`.

De onde veio o jogo pouco importa: os títulos de GOG, Epic e itch caem no mesmo punhado de sítios, porque quem decide é o motor e o programador, não a loja.

Para alguns jogos populares há uma página com os caminhos exatos, o que há na pasta e as armadilhas a evitar: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Crimson Desert](/guides/crimson-desert-save-location), [Marvel's Spider-Man 2](/guides/spider-man-2-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location) e [Palworld](/guides/palworld-save-location).

## O que é copiado e o que não é

Uma pasta de saves raramente contém só saves, por isso o Hoard separa o que encontra em três montes:

- **Os dados de save** são copiados e restaurados. Isso é o teu progresso.
- **Os ficheiros que pertencem a uma máquina concreta** — configuração, registos e afins — são enviados para fazerem parte da cópia, mas nunca escritos por cima da cópia de outro PC. As tuas definições gráficas continuam tuas.
- **O lixo** — caches, despejos de erro, temporários — é ignorado, para que uma cópia não inche com coisas que nunca quererias de volta.

## Quando é feita a cópia

O Hoard vigia a pasta e captura-a **depois de parares de jogar**, não enquanto o jogo tem ficheiros abertos. Se o save foi escrito há segundos, espera que as coisas acalmem: um ficheiro a ser escrito não é um ficheiro que valha a pena capturar a meio.

Cada captura é uma versão. Os snapshots são guardados por hash de conteúdo, por isso um ficheiro que não muda é guardado uma só vez: dez versões de um save de 2 GB ocupam cerca de 2 GB, não 20.

## Cópias sem passar pelos nossos servidores

Se preferes não usar a nuvem de ninguém, corre o \`hoard-server\` tu mesmo e aponta a aplicação para lá. Os teus saves vão do teu PC para o teu disco: sem conta connosco, sem telemetria para nós e sem nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

## Dica: verifica o teu histórico

Abre o separador **Histórico** de um jogo para ver cada backup com data e tamanho. A partir daí podes restaurar qualquer versão anterior com um clique. Os teus saves viajam cifrados, são guardados na UE, e podes exportá-los ou apagá-los quando quiseres.

Já usas uma ferramenta de backup local como o Ludusavi? Podes mantê-la — mas se queres que esses backups cheguem à nuvem e sincronizem entre máquinas sem configurares o Rclone tu mesmo, é exatamente isso que o Hoard automatiza. Vê [Ludusavi vs Hoard](/guides/ludusavi-alternative) para uma comparação justa.

<!-- faq -->

## Perguntas frequentes

### O Hoard faz cópias enquanto jogo?

Não. Espera que saias e que a pasta de saves fique quieta, por isso uma cópia nunca é um ficheiro escrito a meio.

### Quanto espaço ocupam os meus saves?

Menos do que imaginas. As versões são desduplicadas por hash de conteúdo, por isso só ocupa espaço novo aquilo que mudou mesmo entre sessões: a maioria das coleções cabe à vontade em dois gigabytes.

### E se um dos meus jogos não for detetado?

Aponta o Hoard para a pasta à mão e ele segue-a como qualquer outra. A deteção cobre milhares de títulos, mas um jogo que guarde num sítio invulgar, ou que tenhas instalado à mão, às vezes precisa da pista.

### Também copia as minhas mods?

O Hoard segue a pasta de saves, por isso mods que vivam noutro sítio não entram na cópia. É de propósito: as mods são grandes, voltam a descarregar-se, e uma pasta de mods a sincronizar entre máquinas dá mais problemas do que resolve.

### O self-hosting muda a forma como as cópias funcionam?

Nada. A mesma deteção, as mesmas versões, a mesma captura automática. Só o armazenamento é teu.
`,Be=`---
title: "如何自动备份游戏存档"
description: "每次游戏结束后自动备份 PC 游戏存档并保留版本历史，崩溃、重装或坏掉的 Mod 都不会再抹掉你的进度。"
order: 1
updated: 2026-10-02
---

丢失一个存档就意味着丢失数小时的进度。Hoard 会自动备份你的 PC 游戏存档，并保留完整的版本历史，让你随时都能回退。

## Hoard 备份什么

Hoard 会检测你所玩游戏的存档文件夹，并把它们复制到你自己的云端——Hoard Cloud 或你自行托管的服务器。每个备份都带版本，因此旧的副本永远不会被覆盖。

为了找到每款游戏把存档保存在哪里，Hoard 使用与 Ludusavi 相同的社区存档位置数据库，因此对成千上万款游戏的检测开箱即用。区别在于之后发生的事：Hoard 不会把备份留在你的磁盘上，而是自动在云端进行版本管理。

## 设置自动备份

1. 从下载页面**下载并安装 Hoard**（Windows、macOS 或 Linux）。
2. 登录，或将应用指向你自行托管的服务器。
3. 打开**库**。Hoard 会扫描已安装的游戏，并列出找到的存档。
4. 添加你想保护的游戏。Hoard 会自动定位每个存档文件夹；如果某款游戏未被检测到，你可以手动添加路径。
5. 保持**自动模式**开启。Hoard 会监视存档文件夹，并在你停止游戏后进行备份。

从此每一次游戏会话都会被记录，你无需做任何事。

## PC 游戏的存档究竟放在哪里

并没有统一的位置，而这正是需要这类工具的原因。实际上，存档通常落在下面某个地方：

- **在 Steam 内部**，位于 \`userdata/<UserID>/<AppID>/remote/\`——也就是 Steam 云存档自己同步的那个文件夹。
- **\`文档\\My Games\\…\`**，这是 Windows 上最接近约定俗成的位置。
- **\`%APPDATA%\`、\`%LOCALAPPDATA%\` 或 \`LocalLow\`**，大多数 Unity 和 Unreal 游戏写在这里。
- **\`%USERPROFILE%\\Saved Games\`**，被数量不多但很执着的一批游戏使用。
- **游戏自己的安装目录**，出人意料的是，相当多的老游戏仍然存在那里。
- **在 Linux 上**，原生游戏用 \`~/.local/share\` 或 \`~/.config\`；Windows 游戏则在 Proton 前缀内：\`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`。
- **在 macOS 上**，\`~/Library/Application Support\`。

游戏从哪儿买的几乎无关紧要：GOG、Epic 和 itch 的游戏同样落在这几个位置，因为决定权在引擎和开发者手里，不在商店。

对于几款热门游戏，另有专门页面介绍确切路径、文件夹内容以及要避开的坑：[Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [红色沙漠](/guides/crimson-desert-save-location), [漫威蜘蛛侠 2](/guides/spider-man-2-save-location), [博德之门 3](/guides/baldurs-gate-3-save-location) 和 [幻兽帕鲁](/guides/palworld-save-location)。

## 什么会被备份，什么不会

存档文件夹里很少只有存档，所以 Hoard 会把找到的东西分成三类：

- **存档数据**会被备份，也会被还原。这就是你的进度。
- **属于某一台机器的文件**——配置、日志之类——会上传以便进入备份，但绝不会覆盖另一台 PC 上的副本。你的画质设置依然是你的。
- **垃圾**——缓存、崩溃转储、临时文件——会被忽略，免得备份被你永远不想要回的东西撑大。

## 备份发生在什么时候

Hoard 会盯着文件夹，并在**你停止游玩之后**抓取它，而不是在游戏还占着文件的时候。如果存档是几秒前刚写入的，它会等到一切安静下来：正在写入的文件，不值得抓一半。

每次抓取就是一个版本。快照按内容哈希存储，因此未改动的文件只存一份——一个 2 GB 存档的十个版本大约占 2 GB，而不是 20 GB。

## 不经过我们服务器的备份

如果你不想用任何人的云，可以自己运行 \`hoard-server\`，把应用指向它。你的存档从你的 PC 走到你的磁盘：没有我们这边的账号，没有发往我们的遥测，也没有任何东西经过我们的服务器。参见[如何自托管 Hoard](/guides/self-host-hoard)。

## 提示：查看你的历史

打开某款游戏的**历史**标签，即可看到每个备份及其日期和大小。你可以从那里一键还原任何先前版本。你的存档以加密方式传输，存储在欧盟境内，你随时可以导出或删除。

已经在用像 Ludusavi 这样的本地备份工具？你可以继续用——但如果你希望这些备份进入云端并在多台机器之间同步，而无需自己编写 Rclone 脚本，那正是 Hoard 所自动化的。公平对比请见 [Ludusavi 与 Hoard](/guides/ludusavi-alternative)。

<!-- faq -->

## 常见问题

### 我在玩的时候 Hoard 会备份吗？

不会。它会等到你退出、存档文件夹安静下来才动手，所以备份绝不会是一个写到一半的文件。

### 我的存档需要多少空间？

比你想的少。版本按内容哈希去重，因此只有两次游玩之间真正变化的部分才占用新空间——大多数存档收藏放在几个 GB 里绰绰有余。

### 如果某个游戏没被检测到怎么办？

手动把 Hoard 指向那个文件夹，它就会像追踪其他游戏一样追踪它。检测覆盖数千款游戏，但存在不寻常位置、或你手动安装的游戏，有时需要你给个提示。

### 它会备份我的模组吗？

Hoard 追踪的是存档文件夹，所以放在别处的模组不在备份范围内。这是刻意的：模组体积大、可以重新下载，而在多台机器之间同步模组文件夹带来的麻烦多过好处。

### 自托管会改变备份的工作方式吗？

完全不会。同样的检测、同样的版本、同样的自动抓取。只有存储归你所有。
`,Ne=`---
title: "Baldur's Gate 3: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Baldur's Gate 3 seine Spielstände unter Windows, auf dem Steam Deck und dem Mac ablegt, was Spielstand und was Mods sind, der Ehrenmodus und Backups."
order: 23
updated: 2026-10-02
---

Unter Windows legt Baldur's Gate 3 seine Spielstände in \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\` ab, ein Ordner pro Spielstand. Diesen Pfad nennt Larian in der eigenen Support-FAQ. Darunter findest du die Pfade für Steam Deck und Mac, was neben den Spielständen liegt, den Ehrenmodus und wie du alles gesichert hältst.

## Wo Baldur's Gate 3 seine Spielstände ablegt

- **Windows:** \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`
- **Steam Deck und Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`
- **Mac:** \`~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`

Larian hat die native Linux-Version eingestellt, also läuft das Spiel auf dem Steam Deck über Proton, und die Spielstände liegen im Proton-Präfix, das Steam dafür anlegt; \`1086940\` ist die Steam-App-ID des Spiels. Ist es auf der microSD-Karte installiert, liegt der \`compatdata\`-Ordner auf der Karte.

## Was Spielstand ist und was nicht

Jeder Spielstand ist **ein Ordner** in \`Story\` mit einer \`.lsv\`-Datei und einem Vorschaubild. Alles drumherum ist etwas anderes:

- **\`Mods\`** (unter \`Baldur's Gate 3\`) enthält die Mod-Dateien.
- **\`modsettings.lsx\`** (unter \`PlayerProfiles\\Public\`) ist die Liste der aktiven Mods und ihre Ladereihenfolge.
- **Einstellungen** wie Grafik und Steuerung sind Konfigurationsdateien neben dem Profil, kein Teil eines Spielstands.

Die Falle sind die Mods. Ein mit Mods erstellter Spielstand erwartet beim Laden dieselben aktiven Mods. Nimmst du einen gemoddeten Spielstand auf einen anderen PC mit, nimm die Mod-Liste mit, sonst warnt das Spiel vor fehlenden Mods und der Spielstand lädt womöglich nicht wie erwartet.

## Der Ehrenmodus

Der Ehrenmodus hat einen einzigen Spielstand, den das Spiel beim Spielen überschreibt, und fällt deine Gruppe, ist der Ehrendurchlauf vorbei (du kannst im benutzerdefinierten Modus weiterspielen, ohne die Ehre). Diesen Spielstand zu sichern ist deine Entscheidung: Eine Kopie vor einem schweren Kampf ist technisch ein Weg zurück, und manche wollen genau das nach einem Absturz oder Bug, während andere es als Schummeln am Modus sehen. Ein Backup-Werkzeug behält die Versionen so oder so; ob du je eine wiederherstellst, ist eine Sache zwischen dir und den Würfeln.

## Hat Baldur's Gate 3 Cloud-Saves?

Ja. Auf Steam nutzt es Steam Cloud, die die neuesten Spielstände zwischen Rechnern mit demselben Konto synchron hält. Sie hält nur den aktuellen Stand: Wird ein Spielstand beschädigt oder von einem Mod-Update zerstört, wird genau diese Version synchronisiert.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere den ganzen Ordner \`PlayerProfiles\` vom obigen Pfad (er enthält \`Savegames\` und \`modsettings.lsx\`).
3. Zum Wiederherstellen das Spiel schließen und den Ordner zurückkopieren.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) sichert den Spielstand-Ordner jedes Mal, wenn du aufhörst zu spielen, und behält jede Version — ein von einem Mod-Update oder Patch zerstörter Spielstand ist also nur eine Wiederherstellung entfernt. Außerdem hält es den Ordner zwischen deinen PCs und einem Steam Deck synchron.

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Baldur's Gate 3 wird über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Jede Session fügt eine Version mit allen Spielständen hinzu. Um zurückzugehen, öffne den Verlauf und [stelle eine ältere Version wieder her](/guides/restore-a-game-save); was gerade auf deinem PC liegt, wird vorher gesichert, ein Versuch mit einer alten Version ist also nie eine Einbahnstraße.

<!-- faq -->

## Häufige Fragen

### Wo liegen die Spielstände von Baldur's Gate 3 auf dem Steam Deck?

Im Proton-Präfix des Spiels: \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`.

### Kann ich einen gemoddeten Spielstand auf einen anderen PC mitnehmen?

Ja, sofern auf dem anderen PC dieselben Mods installiert und in derselben Reihenfolge aktiv sind. Kopiere \`modsettings.lsx\` zusammen mit dem Spielstand und installiere dieselben Mod-Dateien.

### Kann ich einen Spielstand im Ehrenmodus sichern?

Der Spielstand ist ein gewöhnlicher Ordner, also ja, jedes Backup-Werkzeug kann ihn kopieren. Ob das Wiederherstellen zum Geist des Modus passt, entscheidest du.

### Warum meldet mein Spielstand fehlende Mods?

Er wurde mit Mods erstellt, die jetzt nicht aktiv sind. Aktiviere dieselben Mods in derselben Reihenfolge, und er lädt normal.
`,Ve=`---
title: "Baldur's Gate 3 save location (PC & Steam Deck)"
description: "Where Baldur's Gate 3 keeps its saves on Windows, Steam Deck and Mac, what's a save and what's mods or settings, Honour Mode, and how to back saves up."
order: 23
updated: 2026-10-02
related: sync-game-saves-across-pcs, restore-a-game-save, steam-cloud-alternative
---

On Windows, Baldur's Gate 3 keeps its saves in \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`, one folder per save. That's the path Larian gives in its own support FAQ. Below are the Steam Deck and Mac paths, what sits next to the saves, Honour Mode, and how to keep it all backed up.

## Where Baldur's Gate 3 keeps its saves

- **Windows:** \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`
- **Steam Deck and Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`
- **Mac:** \`~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`

Larian retired the native Linux build, so on a Steam Deck the game runs through Proton and the saves sit inside the Proton prefix Steam keeps for it; \`1086940\` is the game's Steam app ID. If it's installed on the microSD card, the \`compatdata\` folder is on the card.

## What's a save and what isn't

Each save is **a folder** inside \`Story\`, holding an \`.lsv\` file and a thumbnail. Everything else around it is something else:

- **\`Mods\`** (under \`Baldur's Gate 3\`) holds mod files.
- **\`modsettings.lsx\`** (under \`PlayerProfiles\\Public\`) is the list of mods that are switched on, and their load order.
- **Settings** such as graphics and controls are config files next to the profile, not part of a save.

The one that catches people is mods. A save made with mods expects the same mods to be active when it loads. If you move a modded save to another PC, bring the mod list with it, or the game warns you about missing mods and the save may not load as expected.

## Honour Mode

Honour Mode keeps a single save that the game overwrites as you play, and if your party falls, the Honour run is over (you can carry on in Custom Mode, without the Honour). Backing up that save is your call: a copy taken before a hard fight is technically a way back, and some players want exactly that after a crash or a bug, while others consider it cheating the mode. A backup tool keeps the versions either way; whether you ever restore one is between you and the dice.

## Does Baldur's Gate 3 have cloud saves?

Yes. On Steam it uses Steam Cloud, which keeps the latest saves in step between machines on the same account. It holds the current state only: if a save gets corrupted or a mod update breaks it, that's the version that syncs.

## Back it up by hand

1. Close the game completely.
2. Copy the whole \`PlayerProfiles\` folder from the path above (it contains \`Savegames\` and \`modsettings.lsx\`).
3. To restore, close the game and copy it back.

## Keep it backed up and in sync with Hoard

[Hoard](/download) backs up the save folder each time you stop playing and keeps every version, so a save broken by a mod update or a bad patch is one restore away. It also keeps the folder in sync between your PCs and a Steam Deck.

1. Install Hoard and sign in, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. Baldur's Gate 3 is detected from your Steam library and the community save database.
3. Play. When you quit, the first version appears in the history.

Each session adds a version with every save in it. To go back, open the history and [restore an earlier version](/guides/restore-a-game-save); what's on your PC right now is backed up first, so trying an old one is never a one-way trip.

<!-- faq -->

## Frequently asked questions

### Where are Baldur's Gate 3 saves on Steam Deck?

Inside the game's Proton prefix: \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`.

### Can I move a modded save to another PC?

Yes, as long as the other PC has the same mods installed and switched on in the same order. Copy \`modsettings.lsx\` along with the save, and install the same mod files.

### Can I back up an Honour Mode save?

The save is an ordinary folder, so yes, any backup tool can copy it. Whether restoring it fits the spirit of the mode is up to you.

### Why does my save say mods are missing?

It was made with mods that aren't active now. Turn the same mods back on, in the same order, and it loads normally.
`,Ue=`---
title: "Dónde están las partidas de Baldur's Gate 3 (PC y Steam Deck)"
description: "Dónde guarda Baldur's Gate 3 sus partidas en Windows, Steam Deck y Mac, qué es partida y qué son mods o ajustes, el modo Honor y cómo copiarlas."
order: 23
updated: 2026-10-02
---

En Windows, Baldur's Gate 3 guarda sus partidas en \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`, una carpeta por partida. Es la ruta que da Larian en su propio FAQ de soporte. Debajo tienes las rutas de Steam Deck y Mac, lo que hay junto a las partidas, el modo Honor y cómo tenerlo todo copiado.

## Dónde guarda Baldur's Gate 3 las partidas

- **Windows:** \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`
- **Steam Deck y Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`
- **Mac:** \`~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`

Larian retiró la versión nativa para Linux, así que en una Steam Deck el juego corre con Proton y las partidas están dentro del prefijo de Proton que Steam mantiene para él; \`1086940\` es el ID del juego en Steam. Si está instalado en la microSD, la carpeta \`compatdata\` está en la tarjeta.

## Qué es partida y qué no

Cada partida es **una carpeta** dentro de \`Story\`, con un fichero \`.lsv\` y una miniatura. Lo que hay alrededor es otra cosa:

- **\`Mods\`** (dentro de \`Baldur's Gate 3\`) guarda los ficheros de los mods.
- **\`modsettings.lsx\`** (dentro de \`PlayerProfiles\\Public\`) es la lista de mods activos y su orden de carga.
- **Los ajustes**, como gráficos y controles, son ficheros de configuración junto al perfil, no parte de una partida.

El que pilla a la gente son los mods. Una partida hecha con mods espera que esos mismos mods estén activos al cargarla. Si llevas una partida con mods a otro PC, llévate también la lista de mods, o el juego avisará de que faltan y puede que la partida no cargue como esperas.

## El modo Honor

El modo Honor mantiene una única partida que el juego sobrescribe mientras juegas, y si tu grupo cae, la partida de Honor se acaba (puedes seguir en modo Personalizado, sin el Honor). Copiar esa partida es decisión tuya: una copia de antes de un combate difícil es, técnicamente, una vuelta atrás, y hay jugadores que la quieren justo para un cuelgue o un fallo del juego, mientras otros lo ven como hacer trampa al modo. Una herramienta de copias guarda las versiones igual; si alguna vez restauras una, es cosa tuya y de los dados.

## ¿Baldur's Gate 3 tiene partidas en la nube?

Sí. En Steam usa Steam Cloud, que mantiene al día las últimas partidas entre máquinas con la misma cuenta. Sólo guarda el estado actual: si una partida se corrompe o la actualización de un mod la rompe, ésa es la versión que se sincroniza.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia la carpeta \`PlayerProfiles\` entera desde la ruta de arriba (contiene \`Savegames\` y \`modsettings.lsx\`).
3. Para restaurar, cierra el juego y vuelve a copiarla.

## Copia y sincronización automáticas con Hoard

[Hoard](/download) copia la carpeta de partidas cada vez que dejas de jugar y guarda todas las versiones, así que una partida rota por la actualización de un mod o por un parche está a una restauración. También mantiene la carpeta sincronizada entre tus PC y una Steam Deck.

1. Instala Hoard e inicia sesión, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. Baldur's Gate 3 se detecta a partir de tu biblioteca de Steam y de la base de datos comunitaria de partidas.
3. Juega. Al salir, la primera versión aparece en el historial.

Cada sesión añade una versión con todas las partidas dentro. Para volver atrás, abre el historial y [restaura una versión anterior](/guides/restore-a-game-save); lo que tienes ahora en el PC se copia antes, así que probar una vieja nunca es un viaje sin vuelta.

<!-- faq -->

## Preguntas frecuentes

### ¿Dónde están las partidas de Baldur's Gate 3 en Steam Deck?

Dentro del prefijo de Proton del juego: \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`.

### ¿Puedo pasar una partida con mods a otro PC?

Sí, siempre que el otro PC tenga los mismos mods instalados y activos en el mismo orden. Copia \`modsettings.lsx\` junto con la partida e instala los mismos ficheros de mods.

### ¿Puedo hacer copia de una partida del modo Honor?

La partida es una carpeta normal, así que sí, cualquier herramienta de copias puede copiarla. Si restaurarla encaja con el espíritu del modo, lo decides tú.

### ¿Por qué mi partida dice que faltan mods?

Se hizo con mods que ahora no están activos. Vuelve a activar los mismos mods, en el mismo orden, y cargará con normalidad.
`,Fe=`---
title: "Emplacement des sauvegardes de Baldur's Gate 3 (PC et Steam Deck)"
description: "Où Baldur's Gate 3 range ses sauvegardes sous Windows, sur Steam Deck et sur Mac, ce qui est sauvegarde et ce qui est mod ou réglage, le mode Honneur, les backups."
order: 23
updated: 2026-10-02
---

Sous Windows, Baldur's Gate 3 range ses sauvegardes dans \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`, un dossier par sauvegarde. C'est le chemin que donne Larian dans sa propre FAQ. Vous trouverez ci-dessous les chemins sur Steam Deck et Mac, ce qui se trouve à côté des sauvegardes, le mode Honneur et comment tout garder sauvegardé.

## Où Baldur's Gate 3 range ses sauvegardes

- **Windows :** \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`
- **Steam Deck et Linux** (Proton) : \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`
- **Mac :** \`~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`

Larian a retiré la version Linux native : sur Steam Deck, le jeu passe par Proton et les sauvegardes se trouvent dans le préfixe Proton que Steam garde pour lui ; \`1086940\` est l'identifiant Steam du jeu. S'il est installé sur la carte microSD, le dossier \`compatdata\` est sur la carte.

## Ce qui est une sauvegarde et ce qui ne l'est pas

Chaque sauvegarde est **un dossier** dans \`Story\`, avec un fichier \`.lsv\` et une miniature. Tout ce qui l'entoure est autre chose :

- **\`Mods\`** (sous \`Baldur's Gate 3\`) contient les fichiers des mods.
- **\`modsettings.lsx\`** (sous \`PlayerProfiles\\Public\`) est la liste des mods activés et leur ordre de chargement.
- **Les réglages**, comme les graphismes et les commandes, sont des fichiers de configuration à côté du profil, pas une partie d'une sauvegarde.

Le piège, ce sont les mods. Une sauvegarde créée avec des mods attend les mêmes mods actifs au chargement. Si vous déplacez une sauvegarde moddée sur un autre PC, emportez la liste des mods, sinon le jeu signale des mods manquants et la sauvegarde risque de ne pas se charger comme prévu.

## Le mode Honneur

Le mode Honneur n'a qu'une seule sauvegarde, que le jeu écrase au fil de la partie, et si votre groupe tombe, la partie Honneur est terminée (vous pouvez continuer en mode Personnalisé, sans l'Honneur). Sauvegarder ce fichier, c'est votre choix : une copie avant un combat difficile est techniquement un retour en arrière, et certains la veulent justement après un plantage ou un bug, tandis que d'autres y voient une triche. Un outil de sauvegarde garde les versions de toute façon ; en restaurer une, c'est entre vous et les dés.

## Baldur's Gate 3 a-t-il des sauvegardes cloud ?

Oui. Sur Steam, il utilise Steam Cloud, qui garde les dernières sauvegardes synchronisées entre les machines d'un même compte. Il ne conserve que l'état actuel : si une sauvegarde est corrompue ou cassée par la mise à jour d'un mod, c'est cette version qui se synchronise.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez tout le dossier \`PlayerProfiles\` depuis le chemin ci-dessus (il contient \`Savegames\` et \`modsettings.lsx\`).
3. Pour restaurer, fermez le jeu et recopiez-le.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) sauvegarde le dossier à chaque fois que vous arrêtez de jouer et garde toutes les versions : une sauvegarde cassée par la mise à jour d'un mod ou un patch n'est qu'à une restauration. Il garde aussi le dossier synchronisé entre vos PC et un Steam Deck.

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Baldur's Gate 3 est détecté à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes.
3. Jouez. En quittant, la première version apparaît dans l'historique.

Chaque session ajoute une version contenant toutes les sauvegardes. Pour revenir en arrière, ouvrez l'historique et [restaurez une version antérieure](/guides/restore-a-game-save) ; ce qui se trouve sur votre PC est sauvegardé avant, donc essayer une ancienne version n'est jamais un aller simple.

<!-- faq -->

## Questions fréquentes

### Où sont les sauvegardes de Baldur's Gate 3 sur Steam Deck ?

Dans le préfixe Proton du jeu : \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`.

### Puis-je déplacer une sauvegarde moddée sur un autre PC ?

Oui, à condition que l'autre PC ait les mêmes mods installés et activés dans le même ordre. Copiez \`modsettings.lsx\` avec la sauvegarde et installez les mêmes fichiers de mods.

### Puis-je sauvegarder une partie en mode Honneur ?

La sauvegarde est un dossier ordinaire, donc oui, n'importe quel outil peut la copier. Que la restaurer respecte l'esprit du mode, c'est à vous de voir.

### Pourquoi ma sauvegarde signale-t-elle des mods manquants ?

Elle a été créée avec des mods qui ne sont plus actifs. Réactivez les mêmes mods, dans le même ordre, et elle se charge normalement.
`,Ke=`---
title: "Dove sono i salvataggi di Baldur's Gate 3 (PC e Steam Deck)"
description: "Dove Baldur's Gate 3 tiene i salvataggi su Windows, Steam Deck e Mac, cosa è salvataggio e cosa sono mod o impostazioni, la modalità Onore e il backup."
order: 23
updated: 2026-10-02
---

Su Windows, Baldur's Gate 3 tiene i salvataggi in \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`, una cartella per salvataggio. È il percorso che Larian indica nella propria FAQ di supporto. Qui sotto trovi i percorsi su Steam Deck e Mac, cosa c'è accanto ai salvataggi, la modalità Onore e come tenere tutto al sicuro.

## Dove Baldur's Gate 3 tiene i salvataggi

- **Windows:** \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`
- **Steam Deck e Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`
- **Mac:** \`~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`

Larian ha ritirato la versione Linux nativa, quindi su Steam Deck il gioco gira con Proton e i salvataggi stanno nel prefisso Proton che Steam tiene per lui; \`1086940\` è l'ID Steam del gioco. Se è installato sulla microSD, la cartella \`compatdata\` è sulla scheda.

## Cosa è salvataggio e cosa no

Ogni salvataggio è **una cartella** dentro \`Story\`, con un file \`.lsv\` e una miniatura. Tutto quello che c'è intorno è altro:

- **\`Mods\`** (sotto \`Baldur's Gate 3\`) contiene i file delle mod.
- **\`modsettings.lsx\`** (sotto \`PlayerProfiles\\Public\`) è l'elenco delle mod attive e il loro ordine di caricamento.
- **Le impostazioni**, come grafica e comandi, sono file di configurazione accanto al profilo, non parte di un salvataggio.

La trappola sono le mod. Un salvataggio creato con delle mod si aspetta le stesse mod attive al caricamento. Se sposti un salvataggio con mod su un altro PC, porta con te anche l'elenco delle mod, altrimenti il gioco segnala mod mancanti e il salvataggio potrebbe non caricarsi come previsto.

## La modalità Onore

La modalità Onore ha un unico salvataggio che il gioco sovrascrive mentre giochi, e se il tuo gruppo cade la partita in Onore è finita (puoi continuare in modalità Personalizzata, senza l'Onore). Fare il backup di quel salvataggio è una tua scelta: una copia prima di uno scontro difficile è tecnicamente una via di ritorno, e alcuni la vogliono proprio dopo un crash o un bug, mentre altri lo considerano barare. Uno strumento di backup tiene comunque le versioni; se ne ripristini mai una, è una questione tra te e i dadi.

## Baldur's Gate 3 ha i salvataggi nel cloud?

Sì. Su Steam usa Steam Cloud, che tiene allineati gli ultimi salvataggi tra le macchine con lo stesso account. Tiene solo lo stato attuale: se un salvataggio si corrompe o l'aggiornamento di una mod lo rompe, è quella la versione che si sincronizza.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia l'intera cartella \`PlayerProfiles\` dal percorso qui sopra (contiene \`Savegames\` e \`modsettings.lsx\`).
3. Per ripristinare, chiudi il gioco e ricopiala.

## Backup e sync automatici con Hoard

[Hoard](/download) fa il backup della cartella ogni volta che smetti di giocare e tiene tutte le versioni, così un salvataggio rotto dall'aggiornamento di una mod o da una patch è a un ripristino di distanza. Tiene anche la cartella sincronizzata tra i tuoi PC e una Steam Deck.

1. Installa Hoard e accedi, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. Baldur's Gate 3 viene rilevato dalla tua libreria Steam e dal database comunitario dei salvataggi.
3. Gioca. Quando esci, la prima versione compare nella cronologia.

Ogni sessione aggiunge una versione con tutti i salvataggi dentro. Per tornare indietro, apri la cronologia e [ripristina una versione precedente](/guides/restore-a-game-save); quello che hai ora sul PC viene salvato prima, quindi provare una versione vecchia non è mai un viaggio di sola andata.

<!-- faq -->

## Domande frequenti

### Dove sono i salvataggi di Baldur's Gate 3 su Steam Deck?

Nel prefisso Proton del gioco: \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`.

### Posso spostare un salvataggio con mod su un altro PC?

Sì, purché l'altro PC abbia le stesse mod installate e attive nello stesso ordine. Copia \`modsettings.lsx\` insieme al salvataggio e installa gli stessi file delle mod.

### Posso fare il backup di un salvataggio in modalità Onore?

Il salvataggio è una cartella normale, quindi sì, qualsiasi strumento di backup può copiarlo. Se ripristinarlo rispetti lo spirito della modalità, lo decidi tu.

### Perché il mio salvataggio dice che mancano delle mod?

È stato creato con mod che ora non sono attive. Riattiva le stesse mod, nello stesso ordine, e si caricherà normalmente.
`,Qe=`---
title: "バルダーズ・ゲート3（Baldur's Gate 3）のセーブデータの場所（PC・Steam Deck）"
description: "Baldur's Gate 3のセーブデータがWindows、Steam Deck、Macのどこにあるか、セーブとModや設定の違い、オナーモード、バックアップ方法を解説。"
order: 23
updated: 2026-10-02
---

Windows では、Baldur's Gate 3 のセーブデータは \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\` にあり、セーブ 1 つにつきフォルダーが 1 つです。Larian が自社のサポート FAQ で案内しているパスです。以下では Steam Deck と Mac のパス、セーブの隣にあるもの、オナーモード、そしてすべてをバックアップしておく方法を説明します。

## Baldur's Gate 3 のセーブデータの場所

- **Windows:** \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`
- **Steam Deck と Linux**（Proton）: \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`
- **Mac:** \`~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`

Larian はネイティブの Linux 版を廃止したため、Steam Deck ではゲームは Proton で動き、セーブは Steam が用意する Proton プレフィックスの中にあります。\`1086940\` はこのゲームの Steam アプリ ID です。microSD カードにインストールしている場合、\`compatdata\` フォルダーはカード上にあります。

## セーブとそれ以外

各セーブは \`Story\` の中の **フォルダー** で、\`.lsv\` ファイルとサムネイルが入っています。周りにあるものは別物です。

- **\`Mods\`**（\`Baldur's Gate 3\` の下）には Mod のファイルが入っています。
- **\`modsettings.lsx\`**（\`PlayerProfiles\\Public\` の下）は、有効な Mod の一覧と読み込み順です。
- **設定**（グラフィックや操作など）はプロファイルの隣にある設定ファイルで、セーブの一部ではありません。

つまずきやすいのは Mod です。Mod を入れて作ったセーブは、読み込むときに同じ Mod が有効であることを前提にします。Mod 入りのセーブを別の PC に移すなら Mod の一覧も一緒に移してください。そうしないと Mod 不足の警告が出て、セーブが期待どおりに読み込めないことがあります。

## オナーモード

オナーモードではセーブは 1 つだけで、プレイ中にゲームが上書きしていきます。パーティーが全滅するとオナーでの冒険は終わりです（オナーを失ったうえでカスタムモードとして続けることはできます）。このセーブをバックアップするかはあなた次第です。難しい戦闘の前のコピーは技術的には「戻る手段」で、クラッシュやバグの後にまさにそれを求める人もいれば、モードへのズルだと考える人もいます。バックアップツールはどちらにせよバージョンを残します。それを復元するかどうかは、あなたとダイスの問題です。

## Baldur's Gate 3 にクラウドセーブはありますか？

あります。Steam では Steam クラウドを使い、同じアカウントのマシン間で最新のセーブをそろえます。保持するのは現在の状態だけなので、セーブが壊れたり Mod の更新で壊れたりすると、そのバージョンが同期されます。

## 手動でバックアップする

1. ゲームを完全に終了します。
2. 上記のパスから \`PlayerProfiles\` フォルダーを丸ごとコピーします（\`Savegames\` と \`modsettings.lsx\` を含みます）。
3. 復元するときは、ゲームを終了してコピーし戻します。

## Hoard で自動バックアップと同期

[Hoard](/download) は、プレイをやめるたびにセーブフォルダーをバックアップし、すべてのバージョンを保持します。Mod の更新やパッチで壊れたセーブも、復元 1 回で戻せます。さらにフォルダーを PC と Steam Deck の間で同期します。

1. Hoard をインストールしてサインインするか、[自分のサーバー](/guides/self-host-hoard)を指定します。
2. **ライブラリ** を開きます。Baldur's Gate 3 は Steam ライブラリとコミュニティのセーブデータベースから検出されます。
3. プレイします。終了すると、最初のバージョンが履歴に表示されます。

セッションごとに、すべてのセーブを含むバージョンが追加されます。戻りたいときは履歴を開いて[以前のバージョンを復元](/guides/restore-a-game-save)してください。今 PC にあるものは先にバックアップされるので、古いバージョンを試しても片道切符にはなりません。

<!-- faq -->

## よくある質問

### Steam Deck での Baldur's Gate 3 のセーブデータはどこですか？

ゲームの Proton プレフィックスの中です: \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`。

### Mod 入りのセーブを別の PC に移せますか？

はい。移し先の PC に同じ Mod がインストールされ、同じ順番で有効になっていれば可能です。セーブと一緒に \`modsettings.lsx\` をコピーし、同じ Mod ファイルをインストールしてください。

### オナーモードのセーブをバックアップできますか？

セーブは普通のフォルダーなので、どのバックアップツールでもコピーできます。それを復元することがモードの精神に合うかどうかは、あなたが決めてください。

### セーブに「Mod が不足している」と表示されるのはなぜですか？

今は有効になっていない Mod を使って作られたセーブだからです。同じ Mod を同じ順番で有効にすれば、普通に読み込めます。
`,Xe=`---
title: "Onde ficam os saves de Baldur's Gate 3 (PC e Steam Deck)"
description: "Onde o Baldur's Gate 3 guarda os saves no Windows, Steam Deck e Mac, o que é save e o que são mods ou definições, o modo Honra e como fazer backup."
order: 23
updated: 2026-10-02
---

No Windows, o Baldur's Gate 3 guarda os saves em \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`, uma pasta por save. É o caminho que a Larian indica no seu FAQ de suporte. Abaixo tens os caminhos na Steam Deck e no Mac, o que está ao lado dos saves, o modo Honra e como manter tudo com backup.

## Onde o Baldur's Gate 3 guarda os saves

- **Windows:** \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`
- **Steam Deck e Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`
- **Mac:** \`~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`

A Larian retirou a versão nativa para Linux, por isso na Steam Deck o jogo corre com o Proton e os saves estão dentro do prefixo do Proton que o Steam mantém para ele; \`1086940\` é o ID do jogo no Steam. Se estiver instalado no cartão microSD, a pasta \`compatdata\` está no cartão.

## O que é save e o que não é

Cada save é **uma pasta** dentro de \`Story\`, com um ficheiro \`.lsv\` e uma miniatura. O que está à volta é outra coisa:

- **\`Mods\`** (dentro de \`Baldur's Gate 3\`) guarda os ficheiros dos mods.
- **\`modsettings.lsx\`** (dentro de \`PlayerProfiles\\Public\`) é a lista de mods ativos e a sua ordem de carregamento.
- **As definições**, como gráficos e controlos, são ficheiros de configuração junto ao perfil, não fazem parte de um save.

A armadilha são os mods. Um save feito com mods espera os mesmos mods ativos ao carregar. Se levares um save com mods para outro PC, leva também a lista de mods, ou o jogo avisa que faltam mods e o save pode não carregar como esperas.

## O modo Honra

O modo Honra tem um único save que o jogo sobrescreve enquanto jogas, e se o teu grupo cair, a partida em Honra acaba (podes continuar no modo Personalizado, sem a Honra). Fazer backup desse save é decisão tua: uma cópia antes de um combate difícil é, tecnicamente, uma forma de voltar atrás, e há quem a queira precisamente depois de um crash ou de um bug, enquanto outros o veem como fazer batota ao modo. Uma ferramenta de backup guarda as versões na mesma; se alguma vez restaurares uma, é entre ti e os dados.

## O Baldur's Gate 3 tem saves na nuvem?

Sim. No Steam usa o Steam Cloud, que mantém em dia os últimos saves entre máquinas com a mesma conta. Só guarda o estado atual: se um save se corromper ou a atualização de um mod o estragar, é essa a versão que se sincroniza.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a pasta \`PlayerProfiles\` inteira do caminho acima (contém \`Savegames\` e \`modsettings.lsx\`).
3. Para restaurar, fecha o jogo e volta a copiá-la.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) faz backup da pasta de saves sempre que deixas de jogar e guarda todas as versões, por isso um save estragado pela atualização de um mod ou por um patch está a um restauro de distância. Também mantém a pasta sincronizada entre os teus PCs e uma Steam Deck.

1. Instala o Hoard e inicia sessão, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. O Baldur's Gate 3 é detetado a partir da tua biblioteca Steam e da base de dados comunitária de saves.
3. Joga. Ao sair, a primeira versão aparece no histórico.

Cada sessão junta uma versão com todos os saves lá dentro. Para voltar atrás, abre o histórico e [restaura uma versão anterior](/guides/restore-a-game-save); o que tens agora no PC é copiado antes, por isso experimentar uma antiga nunca é uma viagem sem volta.

<!-- faq -->

## Perguntas frequentes

### Onde ficam os saves de Baldur's Gate 3 na Steam Deck?

Dentro do prefixo do Proton do jogo: \`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`.

### Posso levar um save com mods para outro PC?

Sim, desde que o outro PC tenha os mesmos mods instalados e ativos pela mesma ordem. Copia o \`modsettings.lsx\` junto com o save e instala os mesmos ficheiros de mods.

### Posso fazer backup de um save do modo Honra?

O save é uma pasta normal, por isso sim, qualquer ferramenta de backup o pode copiar. Se restaurá-lo combina com o espírito do modo, decides tu.

### Porque é que o meu save diz que faltam mods?

Foi feito com mods que agora não estão ativos. Volta a ativar os mesmos mods, pela mesma ordem, e carrega normalmente.
`,$e=`---
title: "博德之门 3（Baldur's Gate 3）存档位置（PC 与 Steam Deck）"
description: "Baldur's Gate 3 在 Windows、Steam Deck 和 Mac 上的存档位置，哪些是存档、哪些是 Mod 或设置，荣誉模式，以及如何备份存档。"
order: 23
updated: 2026-10-02
---

在 Windows 上，Baldur's Gate 3 把存档放在 \`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`，每个存档一个文件夹。这是 Larian 在官方支持 FAQ 中给出的路径。下面是 Steam Deck 和 Mac 上的路径、存档旁边都有什么、荣誉模式，以及如何把一切都备份好。

## Baldur's Gate 3 的存档位置

- **Windows：**\`%LOCALAPPDATA%\\Larian Studios\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story\`
- **Steam Deck 和 Linux**（Proton）：\`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`
- **Mac：**\`~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`

Larian 已经停止了原生 Linux 版，所以在 Steam Deck 上游戏通过 Proton 运行，存档位于 Steam 为它维护的 Proton 前缀中；\`1086940\` 是游戏的 Steam 应用 ID。如果装在 microSD 卡上，\`compatdata\` 文件夹就在卡上。

## 哪些是存档，哪些不是

每个存档都是 \`Story\` 里的**一个文件夹**，内含一个 \`.lsv\` 文件和一张缩略图。周围的其他东西都不是存档：

- **\`Mods\`**（在 \`Baldur's Gate 3\` 下）存放 Mod 文件。
- **\`modsettings.lsx\`**（在 \`PlayerProfiles\\Public\` 下）是已启用 Mod 的列表及其加载顺序。
- **设置**，例如画面和操作，是放在档案旁边的配置文件，不属于存档。

最容易出问题的是 Mod。用 Mod 创建的存档在读取时，需要同样的 Mod 处于启用状态。如果你把带 Mod 的存档搬到另一台 PC，请把 Mod 列表一起带过去，否则游戏会提示缺少 Mod，存档也可能无法按预期读取。

## 荣誉模式

荣誉模式只有一个存档，游戏会在你游玩时不断覆盖它；如果队伍全灭，这次荣誉之旅就结束了（你可以在失去荣誉的前提下以自定义模式继续）。要不要备份这个存档由你决定：在一场硬仗之前留一份副本，从技术上说就是一条退路；有些玩家正是在崩溃或 bug 之后需要它，也有人认为这是在对这个模式作弊。备份工具无论如何都会保留版本；要不要恢复，就是你和骰子之间的事了。

## Baldur's Gate 3 有云存档吗？

有。在 Steam 上它使用 Steam 云，会在同一账号的设备之间同步最新的存档。它只保存当前状态：如果存档损坏，或被某个 Mod 的更新弄坏，同步过去的就是那个版本。

## 手动备份

1. 完全关闭游戏。
2. 复制上面路径中的整个 \`PlayerProfiles\` 文件夹（其中包含 \`Savegames\` 和 \`modsettings.lsx\`）。
3. 恢复时，关闭游戏，把它复制回去。

## 用 Hoard 自动备份和同步

[Hoard](/download) 会在你每次停止游戏时备份存档文件夹，并保留每一个版本，所以被 Mod 更新或补丁弄坏的存档，只需一次恢复就能找回。它还会在你的 PC 和 Steam Deck 之间保持这个文件夹同步。

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**。Baldur's Gate 3 会通过你的 Steam 库和社区存档数据库被识别出来。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

每次游玩都会增加一个包含所有存档的版本。想退回去时，打开历史记录并[恢复旧版本](/guides/restore-a-game-save)；你电脑上现有的内容会先被备份，所以试一个旧版本从来不是单程票。

<!-- faq -->

## 常见问题

### Steam Deck 上的 Baldur's Gate 3 存档在哪里？

在游戏的 Proton 前缀里：\`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story\`。

### 能把带 Mod 的存档搬到另一台 PC 吗？

能，前提是另一台 PC 装了同样的 Mod，并以相同顺序启用。把 \`modsettings.lsx\` 和存档一起复制过去，并安装同样的 Mod 文件。

### 能备份荣誉模式的存档吗？

存档就是一个普通文件夹，所以可以，任何备份工具都能复制它。恢复它是否符合这个模式的精神，由你自己决定。

### 为什么我的存档提示缺少 Mod？

因为它是用现在没有启用的 Mod 创建的。按相同顺序重新启用同样的 Mod，就能正常读取。
`,Je=`---
title: "Crimson Desert: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Crimson Desert seine Spielstände unter Windows, auf dem Steam Deck und dem Mac ablegt, welcher Ordner sie wirklich enthält und wie du sie sicherst."
order: 21
updated: 2026-10-02
---

Unter Windows legt Crimson Desert seine Spielstände in \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` ab. Diesen Ordner nennt Pearl Abyss in seiner eigenen FAQ. Darunter findest du die Pfade für Steam Deck und Mac, was drinsteckt und wie du ihn gesichert hältst.

## Wo Crimson Desert seine Spielstände ablegt

- **Windows:** \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` (also \`C:\\Users\\<du>\\AppData\\Local\\Pearl Abyss\\CD\\save\`)
- **Steam Deck und Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`
- **Mac, Steam-Version:** \`~/Library/Application Support/Pearl Abyss/CD/save\`
- **Mac, App-Store-Version:** \`~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save\`

Es gibt keine native Linux-Version, also läuft das Spiel auf dem Steam Deck über Proton, und die Spielstände liegen im Proton-Präfix, das Steam dafür anlegt; \`3321460\` ist die Steam-App-ID des Spiels. Ist es auf der microSD-Karte installiert, liegt der \`compatdata\`-Ordner auf der Karte.

\`AppData\` ist unter Windows ein versteckter Ordner. Am schnellsten fügst du \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` in die Adressleiste des Datei-Explorers ein.

## Was im Ordner liegt

In \`save\` gibt es zwei Unterordner. Laut Pearl Abyss **enthält der mit dem numerischen Namen die Spielstände, die du im Spiel anlegst**. Sichere beim Backup den ganzen Ordner \`save\`, statt einzelne Dateien herauszusuchen: Er ist klein, und du lässt nichts zurück, was das Spiel braucht.

Steam- und App-Store-Version nutzen auf dem Mac unterschiedliche Pfade. Wechselst du zwischen ihnen, kopiere die Spielstände einmal von Hand hinüber.

## Hat Crimson Desert Cloud-Saves?

Ja, die Steam-Version hat Steam Cloud, die die neuesten Spielstände zwischen Rechnern mit demselben Steam-Konto synchron hält.

Was sie nicht tut:

- **Ältere Versionen behalten.** Steam Cloud hält den aktuellen Stand. Wird ein Spielstand beschädigt, wird der beschädigte synchronisiert.
- **Andere Stores abdecken.** Eine Kopie aus dem Mac App Store und eine von Steam teilen sich keine Cloud.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere den ganzen Ordner \`save\` vom obigen Pfad an einen sicheren Ort: ein anderes Laufwerk, einen USB-Stick, einen Cloud-Ordner.
3. Zum Wiederherstellen das Spiel schließen und den Ordner zurückkopieren, Vorhandenes ersetzen.

Als einmalige Sicherung vor einem großen Update oder einer Neuinstallation ist das in Ordnung. Als Routine hängt es davon ab, dass du daran denkst, und du hast immer nur die letzte Kopie.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) sichert den Spielstand-Ordner jedes Mal, wenn du aufhörst zu spielen, und behält jede Version — so kommst du von einem kaputten Spielstand oder einer bereuten Entscheidung zurück. Außerdem hält es den Ordner zwischen deinen PCs und einem Steam Deck synchron.

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Crimson Desert wird über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt, am obigen Pfad.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Ab dann fügt jede Session eine Version hinzu, und die neueste liegt auf dem Rechner, an den du dich als Nächstes setzt. Geht ein Spielstand kaputt, ist [eine ältere Version wiederherzustellen](/guides/restore-a-game-save) eine Sache von zwei Klicks.

<!-- faq -->

## Häufige Fragen

### Wo liegen die Spielstände von Crimson Desert auf dem Steam Deck?

Im Proton-Präfix des Spiels: \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`. Liegt das Spiel auf der microSD-Karte, ist auch der \`compatdata\`-Ordner dort.

### Welcher Unterordner enthält meine Spielstände?

Der mit dem numerischen Namen in \`save\`. Sichere trotzdem den ganzen Ordner \`save\`, damit nichts fehlt.

### Ich finde den AppData-Ordner nicht. Wo ist er?

Er ist standardmäßig versteckt. Füge \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` in die Adressleiste des Datei-Explorers ein und drück Enter, oder blende versteckte Elemente im Menü „Ansicht“ ein.

### Kann ich auf Desktop und Steam Deck mit demselben Spielstand spielen?

Ja. Steam Cloud macht das für den neuesten Spielstand auf demselben Steam-Konto. Hoard auch, und es behält eine Version pro Session, sodass du zurückkannst, wenn etwas kaputtgeht.
`,Ze=`---
title: "Crimson Desert save location (PC & Steam Deck)"
description: "Where Crimson Desert keeps its saves on Windows, Steam Deck and Mac, which folder actually holds them, and how to back them up or move them between PCs."
order: 21
updated: 2026-10-02
related: sync-game-saves-across-pcs, restore-a-game-save, steam-cloud-alternative
---

On Windows, Crimson Desert keeps its saves in \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\`. That's the folder Pearl Abyss points to in its own FAQ. Below are the Steam Deck and Mac paths, what's inside, and how to keep it backed up.

## Where Crimson Desert keeps its saves

- **Windows:** \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` (that is, \`C:\\Users\\<you>\\AppData\\Local\\Pearl Abyss\\CD\\save\`)
- **Steam Deck and Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`
- **Mac, Steam version:** \`~/Library/Application Support/Pearl Abyss/CD/save\`
- **Mac, App Store version:** \`~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save\`

There's no native Linux build, so on a Steam Deck the game runs through Proton and its saves sit inside the Proton prefix Steam keeps for it; \`3321460\` is the game's Steam app ID. If the game is on the microSD card, the \`compatdata\` folder is on the card.

\`AppData\` is a hidden folder on Windows. The quickest way in is to paste \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` into the File Explorer address bar.

## What's in the folder

Inside \`save\` there are two subfolders. According to Pearl Abyss, **the one with a numeric name holds the saves you create in the game**. When you back up, take the whole \`save\` folder rather than picking files: it's small, and you won't leave anything the game needs behind.

The Steam and App Store versions on Mac use different paths. If you switch between them, copy the saves across by hand once.

## Does Crimson Desert have cloud saves?

Yes, the Steam version has Steam Cloud, which keeps the latest saves in step between machines on the same Steam account.

What it doesn't do:

- **Keep older versions.** Steam Cloud holds the current state. If a save gets corrupted, the corrupted one is what syncs.
- **Cover other stores.** A Mac App Store copy and a Steam copy don't share a cloud.

## Back it up by hand

1. Close the game completely.
2. Copy the whole \`save\` folder from the path above somewhere safe: another drive, a USB stick, a cloud folder.
3. To restore, close the game and copy it back, replacing what's there.

It's fine as a one-off before a big update or a reinstall. As a routine it depends on you remembering, and you only ever have the copy from last time.

## Keep it backed up and in sync with Hoard

[Hoard](/download) backs up the save folder each time you stop playing and keeps every version, so you can step back from a broken save or a choice you regret. It also keeps the folder in sync between your PCs and a Steam Deck.

1. Install Hoard and sign in, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. Crimson Desert is detected from your Steam library and the community save database, at the path above.
3. Play. When you quit, the first version appears in the history.

From then on every session adds a version, and the newest one is on whichever machine you sit down at next. If a save goes wrong, [restoring an older one](/guides/restore-a-game-save) takes a couple of clicks.

<!-- faq -->

## Frequently asked questions

### Where are Crimson Desert saves on Steam Deck?

Inside the game's Proton prefix: \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`. If the game is on the microSD card, the \`compatdata\` folder is on the card.

### Which subfolder has my saves?

The one with the numeric name inside \`save\`. Back up the whole \`save\` folder anyway, so nothing is left out.

### I can't find the AppData folder. Where is it?

It's hidden by default. Paste \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` into the File Explorer address bar and press Enter, or turn on hidden items in the View menu.

### Can I play on my desktop and my Steam Deck with the same save?

Yes. Steam Cloud does it for the latest save on the same Steam account. Hoard does it too, and keeps a version per session, so you can go back if something breaks.
`,Ye=`---
title: "Dónde están las partidas de Crimson Desert (PC y Steam Deck)"
description: "Dónde guarda Crimson Desert sus partidas en Windows, Steam Deck y Mac, qué carpeta las contiene de verdad y cómo copiarlas o llevarlas de un PC a otro."
order: 21
updated: 2026-10-02
---

En Windows, Crimson Desert guarda sus partidas en \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\`. Es la carpeta que indica Pearl Abyss en su propio FAQ. Debajo tienes las rutas de Steam Deck y Mac, qué hay dentro y cómo tenerla siempre copiada.

## Dónde guarda Crimson Desert las partidas

- **Windows:** \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` (es decir, \`C:\\Users\\<tú>\\AppData\\Local\\Pearl Abyss\\CD\\save\`)
- **Steam Deck y Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`
- **Mac, versión de Steam:** \`~/Library/Application Support/Pearl Abyss/CD/save\`
- **Mac, versión de la App Store:** \`~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save\`

No hay versión nativa para Linux, así que en una Steam Deck el juego corre con Proton y sus partidas están dentro del prefijo de Proton que Steam mantiene para él; \`3321460\` es el ID del juego en Steam. Si está en la microSD, la carpeta \`compatdata\` está en la tarjeta.

\`AppData\` es una carpeta oculta en Windows. Lo más rápido es pegar \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` en la barra de direcciones del Explorador de archivos.

## Qué hay en la carpeta

Dentro de \`save\` hay dos subcarpetas. Según Pearl Abyss, **la que tiene un nombre numérico guarda las partidas que creas en el juego**. Al hacer la copia, llévate la carpeta \`save\` entera en lugar de elegir ficheros: ocupa poco y no te dejas nada que el juego necesite.

Las versiones de Steam y de la App Store en Mac usan rutas distintas. Si cambias de una a otra, copia las partidas a mano una vez.

## ¿Crimson Desert tiene partidas en la nube?

Sí, la versión de Steam tiene Steam Cloud, que mantiene al día las últimas partidas entre máquinas con la misma cuenta de Steam.

Lo que no hace:

- **Guardar versiones anteriores.** Steam Cloud guarda el estado actual. Si una partida se corrompe, lo que se sincroniza es la corrupta.
- **Cubrir otras tiendas.** Una copia de la App Store de Mac y una de Steam no comparten nube.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia la carpeta \`save\` entera desde la ruta de arriba a un sitio seguro: otro disco, un USB, una carpeta en la nube.
3. Para restaurar, cierra el juego y vuelve a copiarla, sustituyendo lo que haya.

Está bien como copia puntual antes de una actualización grande o una reinstalación. Como rutina depende de que te acuerdes, y sólo tienes la copia de la última vez.

## Copia y sincronización automáticas con Hoard

[Hoard](/download) copia la carpeta de partidas cada vez que dejas de jugar y guarda todas las versiones, así que puedes volver atrás desde una partida rota o una decisión de la que te arrepientes. También mantiene la carpeta sincronizada entre tus PC y una Steam Deck.

1. Instala Hoard e inicia sesión, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. Crimson Desert se detecta a partir de tu biblioteca de Steam y de la base de datos comunitaria de partidas, en la ruta de arriba.
3. Juega. Al salir, la primera versión aparece en el historial.

A partir de ahí cada sesión añade una versión, y la más nueva está en la máquina en la que te sientes después. Si una partida se estropea, [restaurar una anterior](/guides/restore-a-game-save) son un par de clics.

<!-- faq -->

## Preguntas frecuentes

### ¿Dónde están las partidas de Crimson Desert en Steam Deck?

Dentro del prefijo de Proton del juego: \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`. Si el juego está en la microSD, la carpeta \`compatdata\` está en la tarjeta.

### ¿Qué subcarpeta tiene mis partidas?

La del nombre numérico, dentro de \`save\`. Aun así, copia la carpeta \`save\` entera para no dejarte nada.

### No encuentro la carpeta AppData. ¿Dónde está?

Está oculta por defecto. Pega \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` en la barra de direcciones del Explorador de archivos y pulsa Intro, o activa los elementos ocultos en el menú Vista.

### ¿Puedo jugar en el sobremesa y en la Steam Deck con la misma partida?

Sí. Steam Cloud lo hace con la última partida en la misma cuenta de Steam. Hoard también, y guarda una versión por sesión, así que puedes volver atrás si algo se rompe.
`,ea=`---
title: "Emplacement des sauvegardes de Crimson Desert (PC et Steam Deck)"
description: "Où Crimson Desert range ses sauvegardes sous Windows, sur Steam Deck et sur Mac, quel dossier les contient vraiment et comment les sauvegarder ou les transférer."
order: 21
updated: 2026-10-02
---

Sous Windows, Crimson Desert range ses sauvegardes dans \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\`. C'est le dossier qu'indique Pearl Abyss dans sa propre FAQ. Vous trouverez ci-dessous les chemins sur Steam Deck et Mac, ce qu'il contient et comment le garder sauvegardé.

## Où Crimson Desert range ses sauvegardes

- **Windows :** \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` (soit \`C:\\Users\\<vous>\\AppData\\Local\\Pearl Abyss\\CD\\save\`)
- **Steam Deck et Linux** (Proton) : \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`
- **Mac, version Steam :** \`~/Library/Application Support/Pearl Abyss/CD/save\`
- **Mac, version App Store :** \`~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save\`

Il n'y a pas de version Linux native : sur Steam Deck, le jeu passe par Proton et ses sauvegardes se trouvent dans le préfixe Proton que Steam garde pour lui ; \`3321460\` est l'identifiant Steam du jeu. S'il est sur la carte microSD, le dossier \`compatdata\` est sur la carte.

\`AppData\` est un dossier caché sous Windows. Le plus rapide est de coller \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` dans la barre d'adresse de l'Explorateur de fichiers.

## Ce que contient le dossier

Dans \`save\`, il y a deux sous-dossiers. Selon Pearl Abyss, **celui au nom numérique contient les sauvegardes que vous créez en jeu**. Pour sauvegarder, prenez tout le dossier \`save\` plutôt que de choisir des fichiers : il est léger et vous n'oubliez rien dont le jeu a besoin.

Les versions Steam et App Store sur Mac utilisent des chemins différents. Si vous passez de l'une à l'autre, copiez les sauvegardes à la main une fois.

## Crimson Desert a-t-il des sauvegardes cloud ?

Oui, la version Steam a Steam Cloud, qui garde les dernières sauvegardes synchronisées entre les machines d'un même compte Steam.

Ce qu'il ne fait pas :

- **Garder les anciennes versions.** Steam Cloud conserve l'état actuel. Si une sauvegarde est corrompue, c'est la version corrompue qui se synchronise.
- **Couvrir les autres boutiques.** Une copie Mac App Store et une copie Steam ne partagent pas de cloud.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez tout le dossier \`save\` depuis le chemin ci-dessus vers un endroit sûr : un autre disque, une clé USB, un dossier cloud.
3. Pour restaurer, fermez le jeu et recopiez-le en remplaçant l'existant.

C'est bien pour une copie ponctuelle avant une grosse mise à jour ou une réinstallation. En routine, cela dépend de votre mémoire, et vous n'avez que la dernière copie.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) sauvegarde le dossier à chaque fois que vous arrêtez de jouer et garde toutes les versions, pour revenir en arrière après une sauvegarde cassée ou un choix regretté. Il garde aussi le dossier synchronisé entre vos PC et un Steam Deck.

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Crimson Desert est détecté à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes, au chemin ci-dessus.
3. Jouez. En quittant, la première version apparaît dans l'historique.

Ensuite, chaque session ajoute une version, et la plus récente est sur la machine où vous vous asseyez ensuite. Si une sauvegarde tourne mal, [restaurer une version antérieure](/guides/restore-a-game-save) prend deux clics.

<!-- faq -->

## Questions fréquentes

### Où sont les sauvegardes de Crimson Desert sur Steam Deck ?

Dans le préfixe Proton du jeu : \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`. Si le jeu est sur la carte microSD, le dossier \`compatdata\` est sur la carte.

### Quel sous-dossier contient mes sauvegardes ?

Celui au nom numérique, dans \`save\`. Sauvegardez quand même tout le dossier \`save\`, pour ne rien oublier.

### Je ne trouve pas le dossier AppData. Où est-il ?

Il est caché par défaut. Collez \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` dans la barre d'adresse de l'Explorateur et appuyez sur Entrée, ou affichez les éléments masqués dans le menu Affichage.

### Puis-je jouer sur mon PC fixe et mon Steam Deck avec la même sauvegarde ?

Oui. Steam Cloud le fait pour la dernière sauvegarde sur un même compte Steam. Hoard aussi, et il garde une version par session, pour revenir en arrière si quelque chose casse.
`,aa=`---
title: "Dove sono i salvataggi di Crimson Desert (PC e Steam Deck)"
description: "Dove Crimson Desert tiene i salvataggi su Windows, Steam Deck e Mac, quale cartella li contiene davvero e come farne il backup o spostarli tra PC."
order: 21
updated: 2026-10-02
---

Su Windows, Crimson Desert tiene i salvataggi in \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\`. È la cartella che Pearl Abyss indica nella propria FAQ. Qui sotto trovi i percorsi su Steam Deck e Mac, cosa c'è dentro e come tenerla al sicuro.

## Dove Crimson Desert tiene i salvataggi

- **Windows:** \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` (cioè \`C:\\Users\\<tu>\\AppData\\Local\\Pearl Abyss\\CD\\save\`)
- **Steam Deck e Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`
- **Mac, versione Steam:** \`~/Library/Application Support/Pearl Abyss/CD/save\`
- **Mac, versione App Store:** \`~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save\`

Non esiste una versione Linux nativa, quindi su Steam Deck il gioco gira con Proton e i salvataggi stanno nel prefisso Proton che Steam tiene per lui; \`3321460\` è l'ID Steam del gioco. Se è sulla microSD, la cartella \`compatdata\` è sulla scheda.

\`AppData\` è una cartella nascosta su Windows. Il modo più rapido è incollare \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` nella barra degli indirizzi di Esplora file.

## Cosa c'è nella cartella

Dentro \`save\` ci sono due sottocartelle. Secondo Pearl Abyss, **quella con il nome numerico contiene i salvataggi che crei nel gioco**. Per il backup prendi l'intera cartella \`save\` invece di scegliere i file: è leggera e non lasci indietro nulla che serva al gioco.

Le versioni Steam e App Store su Mac usano percorsi diversi. Se passi dall'una all'altra, copia i salvataggi a mano una volta.

## Crimson Desert ha i salvataggi nel cloud?

Sì, la versione Steam ha Steam Cloud, che tiene allineati gli ultimi salvataggi tra le macchine con lo stesso account Steam.

Cosa non fa:

- **Tenere le versioni precedenti.** Steam Cloud tiene lo stato attuale. Se un salvataggio si corrompe, si sincronizza quello corrotto.
- **Coprire altri store.** Una copia del Mac App Store e una di Steam non condividono il cloud.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia l'intera cartella \`save\` dal percorso qui sopra in un posto sicuro: un altro disco, una chiavetta USB, una cartella cloud.
3. Per ripristinare, chiudi il gioco e ricopiala sostituendo quella esistente.

Va bene come copia occasionale prima di un grosso aggiornamento o di una reinstallazione. Come routine dipende dalla tua memoria, e hai solo l'ultima copia.

## Backup e sync automatici con Hoard

[Hoard](/download) fa il backup della cartella ogni volta che smetti di giocare e tiene tutte le versioni, così puoi tornare indietro da un salvataggio rotto o da una scelta di cui ti penti. Tiene anche la cartella sincronizzata tra i tuoi PC e una Steam Deck.

1. Installa Hoard e accedi, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. Crimson Desert viene rilevato dalla tua libreria Steam e dal database comunitario dei salvataggi, nel percorso qui sopra.
3. Gioca. Quando esci, la prima versione compare nella cronologia.

Da lì ogni sessione aggiunge una versione, e la più recente è sulla macchina a cui ti siedi dopo. Se un salvataggio si rompe, [ripristinarne uno precedente](/guides/restore-a-game-save) richiede un paio di clic.

<!-- faq -->

## Domande frequenti

### Dove sono i salvataggi di Crimson Desert su Steam Deck?

Nel prefisso Proton del gioco: \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`. Se il gioco è sulla microSD, la cartella \`compatdata\` è sulla scheda.

### Quale sottocartella contiene i miei salvataggi?

Quella con il nome numerico dentro \`save\`. Fai comunque il backup dell'intera cartella \`save\`, così non manca nulla.

### Non trovo la cartella AppData. Dov'è?

È nascosta di default. Incolla \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` nella barra degli indirizzi di Esplora file e premi Invio, oppure attiva gli elementi nascosti nel menu Visualizza.

### Posso giocare sul fisso e sulla Steam Deck con lo stesso salvataggio?

Sì. Steam Cloud lo fa per l'ultimo salvataggio sullo stesso account Steam. Anche Hoard, e tiene una versione per sessione, così puoi tornare indietro se qualcosa si rompe.
`,na=`---
title: "紅の砂漠（Crimson Desert）のセーブデータの場所（PC・Steam Deck）"
description: "Crimson DesertのセーブデータがWindows、Steam Deck、Macのどこにあるか、実際にセーブが入っているフォルダー、バックアップやPC間での移し方を解説。"
order: 21
updated: 2026-10-02
---

Windows では、Crimson Desert のセーブデータは \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` にあります。Pearl Abyss が自社の FAQ で案内しているフォルダーです。以下では Steam Deck と Mac のパス、中身、そしてバックアップを保ち続ける方法を説明します。

## Crimson Desert のセーブデータの場所

- **Windows:** \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\`（つまり \`C:\\Users\\<ユーザー名>\\AppData\\Local\\Pearl Abyss\\CD\\save\`）
- **Steam Deck と Linux**（Proton）: \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`
- **Mac、Steam 版:** \`~/Library/Application Support/Pearl Abyss/CD/save\`
- **Mac、App Store 版:** \`~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save\`

ネイティブの Linux 版はないため、Steam Deck ではゲームは Proton で動き、セーブは Steam が用意する Proton プレフィックスの中にあります。\`3321460\` はこのゲームの Steam アプリ ID です。microSD カードにインストールしている場合、\`compatdata\` フォルダーはカード上にあります。

\`AppData\` は Windows では隠しフォルダーです。いちばん早いのは、エクスプローラーのアドレスバーに \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` を貼り付ける方法です。

## フォルダーの中身

\`save\` の中にはサブフォルダーが 2 つあります。Pearl Abyss によれば、**数字の名前のほうに、ゲーム内で作ったセーブが入っています**。バックアップするときは、ファイルを選ぶのではなく \`save\` フォルダーを丸ごとコピーしてください。容量は小さく、ゲームに必要なものを取りこぼしません。

Mac の Steam 版と App Store 版はパスが異なります。両者を行き来する場合は、一度だけ手動でセーブをコピーしてください。

## Crimson Desert にクラウドセーブはありますか？

あります。Steam 版には Steam クラウドがあり、同じ Steam アカウントのマシン間で最新のセーブをそろえてくれます。

しないこと:

- **古いバージョンを残すこと。** Steam クラウドが保持するのは現在の状態です。セーブが壊れると、壊れたものが同期されます。
- **ほかのストアをカバーすること。** Mac App Store 版と Steam 版はクラウドを共有しません。

## 手動でバックアップする

1. ゲームを完全に終了します。
2. 上記のパスから \`save\` フォルダーを丸ごと、別のドライブや USB メモリ、クラウドフォルダーなど安全な場所にコピーします。
3. 復元するときは、ゲームを終了してコピーし戻し、既存のものと置き換えます。

大型アップデートや再インストールの前に一度だけ取るなら十分です。習慣にするには覚えていることが前提で、手元にあるのは前回のコピーだけです。

## Hoard で自動バックアップと同期

[Hoard](/download) は、プレイをやめるたびにセーブフォルダーをバックアップし、すべてのバージョンを保持します。壊れたセーブや後悔した選択からも戻れます。さらにフォルダーを PC と Steam Deck の間で同期します。

1. Hoard をインストールしてサインインするか、[自分のサーバー](/guides/self-host-hoard)を指定します。
2. **ライブラリ** を開きます。Crimson Desert は Steam ライブラリとコミュニティのセーブデータベースから、上記のパスで検出されます。
3. プレイします。終了すると、最初のバージョンが履歴に表示されます。

それ以降はセッションごとにバージョンが追加され、次に座ったマシンに最新のものが届いています。セーブがおかしくなったら、[以前のバージョンへの復元](/guides/restore-a-game-save)は数クリックで済みます。

<!-- faq -->

## よくある質問

### Steam Deck での Crimson Desert のセーブデータはどこですか？

ゲームの Proton プレフィックスの中です: \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`。ゲームが microSD カードにある場合、\`compatdata\` フォルダーはカード上にあります。

### どのサブフォルダーにセーブが入っていますか？

\`save\` の中の、数字の名前のフォルダーです。それでも取りこぼしがないよう、\`save\` フォルダー全体をバックアップしてください。

### AppData フォルダーが見つかりません。どこにありますか？

既定では隠しフォルダーです。エクスプローラーのアドレスバーに \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` を貼り付けて Enter を押すか、［表示］メニューで隠しファイルを表示してください。

### デスクトップと Steam Deck で同じセーブを使えますか？

はい。Steam クラウドは同じ Steam アカウントで最新のセーブについてそれを行います。Hoard も同じことができ、さらにセッションごとにバージョンを残すので、何か壊れても戻れます。
`,oa=`---
title: "Onde ficam os saves de Crimson Desert (PC e Steam Deck)"
description: "Onde o Crimson Desert guarda os saves no Windows, Steam Deck e Mac, que pasta os contém realmente e como fazer backup ou levá-los de um PC para outro."
order: 21
updated: 2026-10-02
---

No Windows, o Crimson Desert guarda os saves em \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\`. É a pasta que a Pearl Abyss indica no seu próprio FAQ. Abaixo tens os caminhos na Steam Deck e no Mac, o que há lá dentro e como a manter sempre com backup.

## Onde o Crimson Desert guarda os saves

- **Windows:** \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` (ou seja, \`C:\\Users\\<tu>\\AppData\\Local\\Pearl Abyss\\CD\\save\`)
- **Steam Deck e Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`
- **Mac, versão Steam:** \`~/Library/Application Support/Pearl Abyss/CD/save\`
- **Mac, versão App Store:** \`~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save\`

Não há versão nativa para Linux, por isso na Steam Deck o jogo corre com o Proton e os saves estão dentro do prefixo do Proton que o Steam mantém para ele; \`3321460\` é o ID do jogo no Steam. Se estiver no cartão microSD, a pasta \`compatdata\` está no cartão.

\`AppData\` é uma pasta oculta no Windows. O mais rápido é colar \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` na barra de endereço do Explorador de Ficheiros.

## O que há na pasta

Dentro de \`save\` há duas subpastas. Segundo a Pearl Abyss, **a que tem um nome numérico guarda os saves que crias no jogo**. Ao fazer o backup, leva a pasta \`save\` inteira em vez de escolher ficheiros: ocupa pouco e não deixas nada de que o jogo precise.

As versões Steam e App Store no Mac usam caminhos diferentes. Se mudares de uma para a outra, copia os saves à mão uma vez.

## O Crimson Desert tem saves na nuvem?

Sim, a versão Steam tem Steam Cloud, que mantém em dia os últimos saves entre máquinas com a mesma conta Steam.

O que não faz:

- **Guardar versões anteriores.** O Steam Cloud guarda o estado atual. Se um save se corromper, é o corrompido que se sincroniza.
- **Cobrir outras lojas.** Uma cópia da App Store do Mac e uma do Steam não partilham nuvem.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a pasta \`save\` inteira do caminho acima para um sítio seguro: outro disco, uma pen USB, uma pasta na nuvem.
3. Para restaurar, fecha o jogo e volta a copiá-la, substituindo o que lá estiver.

Serve como cópia pontual antes de uma grande atualização ou de uma reinstalação. Como rotina depende de te lembrares, e só tens a cópia da última vez.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) faz backup da pasta de saves sempre que deixas de jogar e guarda todas as versões, por isso podes voltar atrás depois de um save estragado ou de uma escolha de que te arrependes. Também mantém a pasta sincronizada entre os teus PCs e uma Steam Deck.

1. Instala o Hoard e inicia sessão, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. O Crimson Desert é detetado a partir da tua biblioteca Steam e da base de dados comunitária de saves, no caminho acima.
3. Joga. Ao sair, a primeira versão aparece no histórico.

A partir daí cada sessão junta uma versão, e a mais recente está na máquina em que te sentares a seguir. Se um save se estragar, [restaurar um anterior](/guides/restore-a-game-save) são dois cliques.

<!-- faq -->

## Perguntas frequentes

### Onde ficam os saves de Crimson Desert na Steam Deck?

Dentro do prefixo do Proton do jogo: \`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`. Se o jogo estiver no cartão microSD, a pasta \`compatdata\` está no cartão.

### Que subpasta tem os meus saves?

A do nome numérico, dentro de \`save\`. Mesmo assim, faz backup da pasta \`save\` inteira para não deixares nada de fora.

### Não encontro a pasta AppData. Onde está?

Está oculta por predefinição. Cola \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` na barra de endereço do Explorador de Ficheiros e carrega em Enter, ou ativa os itens ocultos no menu Ver.

### Posso jogar no PC e na Steam Deck com o mesmo save?

Sim. O Steam Cloud fá-lo com o último save na mesma conta Steam. O Hoard também, e guarda uma versão por sessão, por isso podes voltar atrás se algo se estragar.
`,sa=`---
title: "红色沙漠（Crimson Desert）存档位置（PC 与 Steam Deck）"
description: "Crimson Desert 在 Windows、Steam Deck 和 Mac 上的存档位置，哪个文件夹真正存放存档，以及如何备份存档或在 PC 之间迁移。"
order: 21
updated: 2026-10-02
---

在 Windows 上，Crimson Desert 把存档放在 \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\`。这是 Pearl Abyss 在其官方 FAQ 中给出的文件夹。下面是 Steam Deck 和 Mac 上的路径、里面有什么，以及如何让它一直有备份。

## Crimson Desert 的存档位置

- **Windows：**\`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\`（即 \`C:\\Users\\<你的用户名>\\AppData\\Local\\Pearl Abyss\\CD\\save\`）
- **Steam Deck 和 Linux**（Proton）：\`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`
- **Mac，Steam 版：**\`~/Library/Application Support/Pearl Abyss/CD/save\`
- **Mac，App Store 版：**\`~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save\`

游戏没有原生 Linux 版，所以在 Steam Deck 上通过 Proton 运行，存档位于 Steam 为它维护的 Proton 前缀中；\`3321460\` 是游戏的 Steam 应用 ID。如果装在 microSD 卡上，\`compatdata\` 文件夹就在卡上。

\`AppData\` 在 Windows 上是隐藏文件夹。最快的办法是把 \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` 粘贴到文件资源管理器的地址栏。

## 文件夹里有什么

\`save\` 里有两个子文件夹。根据 Pearl Abyss 的说法，**名字是数字的那个存放你在游戏里创建的存档**。备份时请复制整个 \`save\` 文件夹，而不是挑文件：它不大，也不会漏掉游戏需要的东西。

Mac 上的 Steam 版和 App Store 版路径不同。如果在两者之间切换，请手动复制一次存档。

## Crimson Desert 有云存档吗？

有，Steam 版支持 Steam 云，会在同一 Steam 账号的设备之间同步最新的存档。

它做不到的：

- **保留旧版本。** Steam 云只保存当前状态。存档一旦损坏，同步过去的就是损坏的那份。
- **覆盖其他商店。** Mac App Store 版和 Steam 版不共享云。

## 手动备份

1. 完全关闭游戏。
2. 把上面路径中的整个 \`save\` 文件夹复制到安全的地方：另一块硬盘、U 盘或云盘文件夹。
3. 恢复时，关闭游戏，把它复制回去并替换原有内容。

在大版本更新或重装前做一次没问题。但要当成习惯，就得靠你记得，而且手上永远只有上一次的那份。

## 用 Hoard 自动备份和同步

[Hoard](/download) 会在你每次停止游戏时备份存档文件夹，并保留每一个版本，所以存档坏了或做了后悔的选择都能退回去。它还会在你的 PC 和 Steam Deck 之间保持这个文件夹同步。

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**。Crimson Desert 会通过你的 Steam 库和社区存档数据库，在上面的路径被识别出来。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

此后每次游玩都会增加一个版本，最新的那个会出现在你下一次坐下来的设备上。如果存档出了问题，[恢复旧版本](/guides/restore-a-game-save)只需点几下。

<!-- faq -->

## 常见问题

### Steam Deck 上的 Crimson Desert 存档在哪里？

在游戏的 Proton 前缀里：\`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save\`。如果游戏装在 microSD 卡上，\`compatdata\` 文件夹就在卡上。

### 哪个子文件夹是我的存档？

\`save\` 里名字是数字的那个。不过还是请备份整个 \`save\` 文件夹，免得漏掉东西。

### 我找不到 AppData 文件夹，它在哪里？

它默认是隐藏的。把 \`%LOCALAPPDATA%\\Pearl Abyss\\CD\\save\` 粘贴到文件资源管理器的地址栏并按回车，或者在“查看”菜单里显示隐藏的项目。

### 我能在台式机和 Steam Deck 上用同一个存档吗？

能。Steam 云会在同一 Steam 账号下同步最新的存档。Hoard 也能做到，而且每次游玩保留一个版本，出了问题可以退回去。
`,ia=`---
title: "Cyberpunk 2077: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Cyberpunk 2077 seine Spielstände unter Windows, auf dem Steam Deck und dem Mac ablegt, was in den Ordnern steckt und wie du sie sicherst oder umziehst."
order: 20
updated: 2026-10-02
---

Unter Windows legt Cyberpunk 2077 seine Spielstände in \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\` ab, ein Ordner pro Spielstand. Das ist die kurze Antwort. Der Rest der Seite behandelt die Pfade auf Steam Deck und Mac, was wirklich im Ordner liegt und wie du ihn gesichert hältst.

## Wo Cyberpunk 2077 seine Spielstände ablegt

- **Windows** (Steam, GOG oder Epic): \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`
- **Steam Deck und Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`
- **Mac:** \`~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves\`

Die Pfade für Windows und Mac nennt CD Projekt Red auf seinen eigenen Support-Seiten. Auf dem Steam Deck läuft das Spiel in einem Proton-Präfix, einem kleinen Windows-Ordnerbaum, den Steam für jedes Spiel anlegt; \`1091500\` ist die Steam-App-ID von Cyberpunk. Ist das Spiel auf der microSD-Karte installiert, suche \`steamapps/compatdata/1091500\` auf der Karte.

## Was im Ordner liegt

Cyberpunk schreibt keine einzelne Spielstand-Datei, sondern **einen Ordner pro Spielstand**: \`AutoSave-0\`, \`AutoSave-1\` und so weiter, \`ManualSave-0\`, \`ManualSave-1\` und \`QuickSave-0\`. Jeder enthält den Spielstand selbst (\`sav.dat\`) sowie Screenshot und Metadaten für das Lademenü.

Daraus folgen zwei Dinge:

- **Sichere den übergeordneten Ordner, nicht einen einzelnen Spielstand.** Wer nur den neuesten \`ManualSave\` kopiert, lässt die Autosaves weg, die oft den jüngsten Fortschritt haben.
- **Autosaves rotieren.** Das Spiel verwendet eine kleine Zahl von \`AutoSave\`-Ordnern immer wieder und überschreibt den ältesten. Ein Autosave von vor drei Stunden ist meist schon weg — deshalb lohnt sich ein Verlauf außerhalb des Spiels.

Die Einstellungen liegen nicht hier. Grafik und Steuerung stehen in \`UserSettings.json\` unter \`%LOCALAPPDATA%\\CD Projekt Red\\Cyberpunk 2077\`, neben Caches und Logs. Genau diesen Ordner sichern viele (und manche Werkzeuge) aus Versehen: Er enthält nichts, dessen Verlust dich Fortschritt kostet.

## Hat Cyberpunk 2077 Cloud-Saves?

Ja. Die Steam-Version nutzt Steam Cloud, die GOG-Version die Cloud von GOG Galaxy. Beide halten den neuesten Stand deiner Spielstände zwischen Rechnern desselben Stores synchron.

Was keine von beiden tut:

- **Ältere Versionen behalten.** Wird ein Spielstand beschädigt oder von einem Mod zerstört, liegt auch in der Cloud die kaputte Kopie.
- **Stores verbinden.** Steam Cloud und GOGs Cloud sprechen nicht miteinander, obwohl PC-Spielstände von Steam, GOG und Epic in jeder Version laden, wenn du den Ordner hinüberkopierst.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere den ganzen Ordner \`Cyberpunk 2077\` vom obigen Pfad auf einen USB-Stick, ein anderes Laufwerk oder in einen Cloud-Ordner.
3. Zum Wiederherstellen das Spiel schließen und den Ordner zurückkopieren, Vorhandenes ersetzen.

Das funktioniert, aber nur so oft, wie du daran denkst, und du hast immer nur die Kopie vom letzten Mal.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) sichert den Spielstand-Ordner jedes Mal, wenn du aufhörst zu spielen, und behält jede Version — ein beschädigter Spielstand oder ein weggerotierter Autosave ist also einen Klick entfernt. Außerdem synchronisiert es den Ordner zwischen deinen PCs und einem Steam Deck.

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Cyberpunk wird über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt.
3. Prüfe, dass der angezeigte Ordner der unter \`Saved Games\\CD Projekt Red\\Cyberpunk 2077\` ist. Steht dort der Ordner unter \`AppData\\Local\`, ändere ihn: Der enthält nur Einstellungen.
4. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Hoard verfolgt den ganzen Ordner, also landet jeder \`AutoSave\`, \`ManualSave\` und \`QuickSave\` in derselben Version. Mit Deck und Desktop wartet die neueste Version auf dem Gerät, das du als Nächstes in die Hand nimmst — siehe [wie die Synchronisierung zwischen PCs funktioniert](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Häufige Fragen

### Wo liegen die Spielstände von Cyberpunk 2077 auf dem Steam Deck?

Im Proton-Präfix des Spiels: \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`. Liegt das Spiel auf der microSD-Karte, ist auch der \`compatdata\`-Ordner dort.

### Kann ich meine Spielstände von GOG zu Steam mitnehmen?

Ja. PC-Spielstände sind bei Steam, GOG und Epic dieselben. Kopiere die Spielstand-Ordner bei geschlossenem Spiel an denselben Pfad der anderen Installation, und sie erscheinen im Lademenü.

### Warum gibt es so viele AutoSave-Ordner?

Das Spiel hält ein paar Autosave-Plätze vor und überschreibt jedes Mal den ältesten. Es sind normale Spielstände, die eben von selbst ersetzt werden.

### Warum ist mein älterer Autosave verschwunden?

Weil sein Platz wiederverwendet wurde. Das Spiel behält nur wenige. Ein Backup-Werkzeug mit Versionen ist der einzige Weg, ihn nach dem Rotieren zurückzuholen.

### Synchronisiert Hoard auch meine Einstellungen?

Nein. Die Einstellungen liegen in einem anderen Ordner, der nicht zum Spielstand gehört, also behält jeder Rechner seine eigenen — meistens genau richtig, denn Deck und Desktop brauchen unterschiedliche Grafikeinstellungen. Mehr dazu unter [Spielstände zwischen PCs synchronisieren](/guides/sync-game-saves-across-pcs).
`,ra=`---
title: "Cyberpunk 2077 save location (PC & Steam Deck)"
description: "Where Cyberpunk 2077 keeps its saves on Windows, Steam Deck and Mac, what each folder holds, and how to back them up or move them between PCs."
order: 20
updated: 2026-10-02
related: sync-game-saves-across-pcs, restore-a-game-save, steam-cloud-alternative
---

On Windows, Cyberpunk 2077 keeps its saves in \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`, one folder per save. That's the short answer. The rest of this page covers the Steam Deck and Mac paths, what's actually in the folder, and how to keep it backed up.

## Where Cyberpunk 2077 keeps its saves

- **Windows** (Steam, GOG or Epic): \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`
- **Steam Deck and Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`
- **Mac:** \`~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves\`

The Windows and Mac paths are the ones CD Projekt Red gives in its own support pages. On a Steam Deck, the game runs inside a Proton prefix, which is a small Windows folder tree that Steam keeps per game; \`1091500\` is Cyberpunk's Steam app ID. If the game is installed on the microSD card, look for \`steamapps/compatdata/1091500\` on the card instead.

## What's in the folder

Cyberpunk doesn't write one save file. It writes **one folder per save**: \`AutoSave-0\`, \`AutoSave-1\` and so on, \`ManualSave-0\`, \`ManualSave-1\`, and \`QuickSave-0\`. Each holds the save itself (\`sav.dat\`) plus the screenshot and metadata the load menu shows.

Two things follow from that:

- **Back up the parent folder, not a single save.** Copying only the newest \`ManualSave\` leaves out the autosaves, which are often the most recent progress.
- **Autosaves rotate.** The game reuses a small set of \`AutoSave\` folders and overwrites the oldest. An autosave from three hours ago is usually already gone, which is why an external history is worth having.

Settings are not here. Graphics and controls live in \`UserSettings.json\` under \`%LOCALAPPDATA%\\CD Projekt Red\\Cyberpunk 2077\`, next to caches and logs. That's the folder a lot of people (and some tools) back up by mistake: it holds nothing you'd lose progress over.

## Does Cyberpunk 2077 have cloud saves?

Yes. The Steam version uses Steam Cloud, and the GOG version uses GOG Galaxy's cloud. Both keep the latest state of your saves in step between machines on the same store.

What neither does:

- **Keep older versions.** If a save gets corrupted, or a mod breaks it, the cloud holds the broken copy too.
- **Cross stores.** Steam Cloud and GOG's cloud don't talk to each other, even though PC saves from Steam, GOG and Epic load fine in any of them if you copy the folder over.

## Back it up by hand

1. Close the game completely.
2. Copy the whole \`Cyberpunk 2077\` folder from the path above to a USB stick, another drive or a cloud folder.
3. To restore, close the game and copy the folder back, replacing what's there.

It works, but only as often as you remember to do it, and only the copy you made last time.

## Keep it backed up and in sync with Hoard

[Hoard](/download) backs up the save folder every time you stop playing and keeps every version, so a corrupted save or an autosave that rotated away is one click back. It also syncs the folder between your PCs and a Steam Deck.

1. Install Hoard and sign in, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. Cyberpunk is detected from your Steam library and the community save database.
3. Check that the folder shown is the \`Saved Games\\CD Projekt Red\\Cyberpunk 2077\` one. If it shows the \`AppData\\Local\` folder instead, change it: that one only has settings.
4. Play. When you quit, the first version appears in the history.

Hoard tracks the whole folder, so every \`AutoSave\`, \`ManualSave\` and \`QuickSave\` goes into the same version. On a Deck and a desktop, the newest version is waiting on whichever one you pick up next — see [how syncing between PCs works](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Frequently asked questions

### Where are Cyberpunk 2077 saves on Steam Deck?

Inside the game's Proton prefix: \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`. If the game is on the microSD card, the \`compatdata\` folder is on the card.

### Can I move my Cyberpunk 2077 saves from GOG to Steam?

Yes. PC saves are the same across Steam, GOG and Epic. Copy the save folders into the same path on the other install with the game closed, and they show up in the load menu.

### Why are there so many AutoSave folders?

The game keeps a handful of autosave slots and overwrites the oldest one each time. They're normal saves; it's just that they get replaced on their own.

### Why did my older autosave disappear?

Because the slot it lived in was reused. The game only keeps a few. A backup tool that keeps versions is the only way to get one back after it rotates out.

### Does Hoard keep my settings in sync too?

No. Settings live in a separate folder that isn't part of the save, so each machine keeps its own, which is usually what you want: a Deck and a desktop need different graphics settings. More on that in [syncing saves across PCs](/guides/sync-game-saves-across-pcs).
`,ta=`---
title: "Dónde están las partidas de Cyberpunk 2077 (PC y Steam Deck)"
description: "Dónde guarda Cyberpunk 2077 sus partidas en Windows, Steam Deck y Mac, qué hay en cada carpeta y cómo copiarlas o llevarlas de un PC a otro."
order: 20
updated: 2026-10-02
---

En Windows, Cyberpunk 2077 guarda sus partidas en \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`, una carpeta por partida. Ésa es la respuesta corta. El resto de la página cubre las rutas de Steam Deck y Mac, qué hay de verdad en la carpeta y cómo tenerla siempre copiada.

## Dónde guarda Cyberpunk 2077 las partidas

- **Windows** (Steam, GOG o Epic): \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`
- **Steam Deck y Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`
- **Mac:** \`~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves\`

Las rutas de Windows y Mac son las que da CD Projekt Red en sus propias páginas de soporte. En una Steam Deck el juego corre dentro de un prefijo de Proton, un pequeño árbol de carpetas de Windows que Steam mantiene para cada juego; \`1091500\` es el ID de Cyberpunk en Steam. Si el juego está instalado en la microSD, busca \`steamapps/compatdata/1091500\` en la tarjeta.

## Qué hay en la carpeta

Cyberpunk no escribe un único fichero de partida. Escribe **una carpeta por partida**: \`AutoSave-0\`, \`AutoSave-1\` y siguientes, \`ManualSave-0\`, \`ManualSave-1\`, y \`QuickSave-0\`. Cada una guarda la partida en sí (\`sav.dat\`) y la captura y los metadatos que enseña el menú de carga.

De ahí salen dos cosas:

- **Copia la carpeta padre, no una partida suelta.** Copiar sólo el \`ManualSave\` más reciente deja fuera los autoguardados, que muchas veces tienen el progreso más nuevo.
- **Los autoguardados rotan.** El juego reutiliza unas pocas carpetas \`AutoSave\` y sobrescribe la más antigua. Un autoguardado de hace tres horas normalmente ya no existe, y por eso compensa tener un historial fuera del juego.

Los ajustes no están aquí. Los gráficos y controles viven en \`UserSettings.json\`, dentro de \`%LOCALAPPDATA%\\CD Projekt Red\\Cyberpunk 2077\`, junto a cachés y registros. Ésa es la carpeta que mucha gente (y alguna herramienta) copia por error: no guarda nada cuya pérdida te cueste progreso.

## ¿Cyberpunk 2077 tiene partidas en la nube?

Sí. La versión de Steam usa Steam Cloud y la de GOG, la nube de GOG Galaxy. Las dos mantienen al día el último estado de tus partidas entre máquinas de la misma tienda.

Lo que no hace ninguna:

- **Guardar versiones anteriores.** Si una partida se corrompe, o un mod la rompe, la nube también se queda con la copia rota.
- **Cruzar tiendas.** Steam Cloud y la nube de GOG no se hablan, aunque las partidas de PC de Steam, GOG y Epic cargan sin problema en cualquiera si copias la carpeta.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia la carpeta \`Cyberpunk 2077\` entera desde la ruta de arriba a un USB, otro disco o una carpeta en la nube.
3. Para restaurar, cierra el juego y vuelve a copiar la carpeta, sustituyendo lo que haya.

Funciona, pero sólo tan a menudo como te acuerdes, y sólo tienes la copia de la última vez.

## Copia y sincronización automáticas con Hoard

[Hoard](/download) copia la carpeta de partidas cada vez que dejas de jugar y guarda todas las versiones, así que una partida corrupta o un autoguardado que ya rotó están a un clic. También sincroniza la carpeta entre tus PC y una Steam Deck.

1. Instala Hoard e inicia sesión, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. Cyberpunk se detecta a partir de tu biblioteca de Steam y de la base de datos comunitaria de partidas.
3. Comprueba que la carpeta que aparece es la de \`Saved Games\\CD Projekt Red\\Cyberpunk 2077\`. Si aparece la de \`AppData\\Local\`, cámbiala: ésa sólo tiene ajustes.
4. Juega. Al salir, la primera versión aparece en el historial.

Hoard rastrea la carpeta entera, así que cada \`AutoSave\`, \`ManualSave\` y \`QuickSave\` entra en la misma versión. Con una Deck y un sobremesa, la versión más nueva te espera en el que cojas después; mira [cómo funciona la sincronización entre PC](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Preguntas frecuentes

### ¿Dónde están las partidas de Cyberpunk 2077 en Steam Deck?

Dentro del prefijo de Proton del juego: \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`. Si el juego está en la microSD, la carpeta \`compatdata\` está en la tarjeta.

### ¿Puedo pasar mis partidas de Cyberpunk 2077 de GOG a Steam?

Sí. Las partidas de PC son las mismas en Steam, GOG y Epic. Copia las carpetas de partida a la misma ruta en la otra instalación con el juego cerrado y aparecerán en el menú de carga.

### ¿Por qué hay tantas carpetas AutoSave?

El juego mantiene unas cuantas ranuras de autoguardado y sobrescribe la más antigua cada vez. Son partidas normales; sólo que se sustituyen solas.

### ¿Por qué ha desaparecido mi autoguardado antiguo?

Porque la ranura donde estaba se reutilizó. El juego sólo guarda unas pocas. Una herramienta de copias que guarde versiones es la única forma de recuperarlo cuando ya ha rotado.

### ¿Hoard sincroniza también mis ajustes?

No. Los ajustes viven en otra carpeta que no forma parte de la partida, así que cada máquina conserva los suyos, que normalmente es lo que quieres: una Deck y un sobremesa necesitan ajustes gráficos distintos. Más sobre esto en [sincronizar partidas entre PC](/guides/sync-game-saves-across-pcs).
`,ua=`---
title: "Emplacement des sauvegardes de Cyberpunk 2077 (PC et Steam Deck)"
description: "Où Cyberpunk 2077 range ses sauvegardes sous Windows, sur Steam Deck et sur Mac, ce que contient chaque dossier et comment les sauvegarder ou les transférer."
order: 20
updated: 2026-10-02
---

Sous Windows, Cyberpunk 2077 range ses sauvegardes dans \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`, un dossier par sauvegarde. Voilà la réponse courte. La suite couvre les chemins sur Steam Deck et Mac, ce que contient vraiment le dossier et comment le garder sauvegardé.

## Où Cyberpunk 2077 range ses sauvegardes

- **Windows** (Steam, GOG ou Epic) : \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`
- **Steam Deck et Linux** (Proton) : \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`
- **Mac :** \`~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves\`

Les chemins Windows et Mac sont ceux que donne CD Projekt Red dans ses propres pages d'assistance. Sur Steam Deck, le jeu tourne dans un préfixe Proton, une petite arborescence Windows que Steam garde pour chaque jeu ; \`1091500\` est l'identifiant Steam de Cyberpunk. Si le jeu est installé sur la carte microSD, cherchez \`steamapps/compatdata/1091500\` sur la carte.

## Ce que contient le dossier

Cyberpunk n'écrit pas un seul fichier de sauvegarde, mais **un dossier par sauvegarde** : \`AutoSave-0\`, \`AutoSave-1\` et ainsi de suite, \`ManualSave-0\`, \`ManualSave-1\` et \`QuickSave-0\`. Chacun contient la sauvegarde elle-même (\`sav.dat\`) ainsi que la capture et les métadonnées affichées dans le menu de chargement.

Deux conséquences :

- **Sauvegardez le dossier parent, pas une seule sauvegarde.** Copier seulement le \`ManualSave\` le plus récent laisse de côté les sauvegardes automatiques, qui contiennent souvent la progression la plus récente.
- **Les sauvegardes automatiques tournent.** Le jeu réutilise quelques dossiers \`AutoSave\` et écrase le plus ancien. Une sauvegarde automatique d'il y a trois heures a généralement déjà disparu, d'où l'intérêt d'un historique hors du jeu.

Les réglages ne sont pas là. Graphismes et commandes vivent dans \`UserSettings.json\`, sous \`%LOCALAPPDATA%\\CD Projekt Red\\Cyberpunk 2077\`, à côté des caches et des journaux. C'est le dossier que beaucoup de gens (et certains outils) sauvegardent par erreur : il ne contient rien dont la perte vous coûterait de la progression.

## Cyberpunk 2077 a-t-il des sauvegardes cloud ?

Oui. La version Steam utilise Steam Cloud, la version GOG le cloud de GOG Galaxy. Les deux gardent le dernier état de vos sauvegardes synchronisé entre les machines d'une même boutique.

Ce qu'aucun des deux ne fait :

- **Garder les anciennes versions.** Si une sauvegarde est corrompue, ou cassée par un mod, le cloud garde aussi la copie cassée.
- **Passer d'une boutique à l'autre.** Steam Cloud et le cloud de GOG ne communiquent pas, alors que les sauvegardes PC de Steam, GOG et Epic se chargent sans problème dans n'importe laquelle si vous copiez le dossier.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez tout le dossier \`Cyberpunk 2077\` depuis le chemin ci-dessus vers une clé USB, un autre disque ou un dossier cloud.
3. Pour restaurer, fermez le jeu et recopiez le dossier en remplaçant l'existant.

Ça marche, mais seulement aussi souvent que vous y pensez, et vous n'avez que la copie de la dernière fois.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) sauvegarde le dossier à chaque fois que vous arrêtez de jouer et garde toutes les versions : une sauvegarde corrompue ou une sauvegarde automatique écrasée est à un clic. Il synchronise aussi le dossier entre vos PC et un Steam Deck.

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Cyberpunk est détecté à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes.
3. Vérifiez que le dossier affiché est celui de \`Saved Games\\CD Projekt Red\\Cyberpunk 2077\`. S'il affiche celui d'\`AppData\\Local\`, changez-le : celui-là ne contient que des réglages.
4. Jouez. En quittant, la première version apparaît dans l'historique.

Hoard suit le dossier entier, donc chaque \`AutoSave\`, \`ManualSave\` et \`QuickSave\` entre dans la même version. Avec un Deck et un PC fixe, la version la plus récente vous attend sur celui que vous reprenez — voir [comment fonctionne la synchronisation entre PC](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Questions fréquentes

### Où sont les sauvegardes de Cyberpunk 2077 sur Steam Deck ?

Dans le préfixe Proton du jeu : \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`. Si le jeu est sur la carte microSD, le dossier \`compatdata\` est sur la carte.

### Puis-je passer mes sauvegardes de Cyberpunk 2077 de GOG à Steam ?

Oui. Les sauvegardes PC sont les mêmes sur Steam, GOG et Epic. Copiez les dossiers de sauvegarde au même chemin dans l'autre installation, jeu fermé, et ils apparaissent dans le menu de chargement.

### Pourquoi y a-t-il autant de dossiers AutoSave ?

Le jeu garde quelques emplacements de sauvegarde automatique et écrase le plus ancien à chaque fois. Ce sont des sauvegardes normales, simplement remplacées toutes seules.

### Pourquoi mon ancienne sauvegarde automatique a-t-elle disparu ?

Parce que son emplacement a été réutilisé. Le jeu n'en garde que quelques-uns. Un outil de sauvegarde qui conserve des versions est le seul moyen de la récupérer une fois écrasée.

### Hoard synchronise-t-il aussi mes réglages ?

Non. Les réglages vivent dans un autre dossier qui ne fait pas partie de la sauvegarde, donc chaque machine garde les siens, ce qui est en général ce que vous voulez : un Deck et un PC fixe ont besoin de réglages graphiques différents. Plus de détails dans [synchroniser ses parties entre PC](/guides/sync-game-saves-across-pcs).
`,da=`---
title: "Dove sono i salvataggi di Cyberpunk 2077 (PC e Steam Deck)"
description: "Dove Cyberpunk 2077 tiene i salvataggi su Windows, Steam Deck e Mac, cosa contiene ogni cartella e come farne il backup o spostarli da un PC all'altro."
order: 20
updated: 2026-10-02
---

Su Windows, Cyberpunk 2077 tiene i salvataggi in \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`, una cartella per salvataggio. Questa è la risposta breve. Il resto della pagina copre i percorsi su Steam Deck e Mac, cosa c'è davvero nella cartella e come tenerla sempre al sicuro.

## Dove Cyberpunk 2077 tiene i salvataggi

- **Windows** (Steam, GOG o Epic): \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`
- **Steam Deck e Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`
- **Mac:** \`~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves\`

I percorsi per Windows e Mac sono quelli indicati da CD Projekt Red nelle proprie pagine di supporto. Su Steam Deck il gioco gira dentro un prefisso Proton, un piccolo albero di cartelle Windows che Steam tiene per ogni gioco; \`1091500\` è l'ID Steam di Cyberpunk. Se il gioco è installato sulla microSD, cerca \`steamapps/compatdata/1091500\` sulla scheda.

## Cosa c'è nella cartella

Cyberpunk non scrive un unico file di salvataggio, ma **una cartella per salvataggio**: \`AutoSave-0\`, \`AutoSave-1\` e così via, \`ManualSave-0\`, \`ManualSave-1\` e \`QuickSave-0\`. Ognuna contiene il salvataggio vero e proprio (\`sav.dat\`) più lo screenshot e i metadati mostrati nel menu di caricamento.

Ne seguono due cose:

- **Fai il backup della cartella madre, non di un singolo salvataggio.** Copiare solo l'ultimo \`ManualSave\` lascia fuori i salvataggi automatici, che spesso hanno i progressi più recenti.
- **I salvataggi automatici ruotano.** Il gioco riusa poche cartelle \`AutoSave\` e sovrascrive la più vecchia. Un salvataggio automatico di tre ore fa di solito non c'è già più: per questo conviene una cronologia fuori dal gioco.

Le impostazioni non sono qui. Grafica e comandi stanno in \`UserSettings.json\`, sotto \`%LOCALAPPDATA%\\CD Projekt Red\\Cyberpunk 2077\`, accanto a cache e log. È la cartella che molti (e alcuni strumenti) salvano per errore: non contiene nulla la cui perdita ti costi progressi.

## Cyberpunk 2077 ha i salvataggi nel cloud?

Sì. La versione Steam usa Steam Cloud, quella GOG il cloud di GOG Galaxy. Entrambi tengono allineato l'ultimo stato dei salvataggi tra le macchine dello stesso store.

Cosa non fa nessuno dei due:

- **Tenere le versioni precedenti.** Se un salvataggio si corrompe, o una mod lo rompe, anche il cloud tiene la copia rotta.
- **Collegare store diversi.** Steam Cloud e il cloud di GOG non si parlano, anche se i salvataggi PC di Steam, GOG ed Epic si caricano senza problemi in qualsiasi versione copiando la cartella.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia l'intera cartella \`Cyberpunk 2077\` dal percorso qui sopra su una chiavetta USB, un altro disco o una cartella cloud.
3. Per ripristinare, chiudi il gioco e ricopia la cartella sostituendo quella esistente.

Funziona, ma solo quanto spesso te ne ricordi, e hai solo la copia dell'ultima volta.

## Backup e sync automatici con Hoard

[Hoard](/download) fa il backup della cartella ogni volta che smetti di giocare e tiene tutte le versioni, così un salvataggio corrotto o un salvataggio automatico ormai sovrascritto è a un clic. Sincronizza anche la cartella tra i tuoi PC e una Steam Deck.

1. Installa Hoard e accedi, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. Cyberpunk viene rilevato dalla tua libreria Steam e dal database comunitario dei salvataggi.
3. Controlla che la cartella mostrata sia quella in \`Saved Games\\CD Projekt Red\\Cyberpunk 2077\`. Se mostra quella in \`AppData\\Local\`, cambiala: contiene solo impostazioni.
4. Gioca. Quando esci, la prima versione compare nella cronologia.

Hoard segue l'intera cartella, quindi ogni \`AutoSave\`, \`ManualSave\` e \`QuickSave\` finisce nella stessa versione. Con una Deck e un fisso, la versione più recente ti aspetta su quello che prendi dopo — vedi [come funziona la sincronizzazione tra PC](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Domande frequenti

### Dove sono i salvataggi di Cyberpunk 2077 su Steam Deck?

Nel prefisso Proton del gioco: \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`. Se il gioco è sulla microSD, la cartella \`compatdata\` è sulla scheda.

### Posso spostare i salvataggi di Cyberpunk 2077 da GOG a Steam?

Sì. I salvataggi PC sono gli stessi su Steam, GOG ed Epic. Copia le cartelle di salvataggio nello stesso percorso dell'altra installazione a gioco chiuso e compariranno nel menu di caricamento.

### Perché ci sono così tante cartelle AutoSave?

Il gioco tiene alcuni slot di salvataggio automatico e sovrascrive ogni volta il più vecchio. Sono salvataggi normali, solo che vengono sostituiti da soli.

### Perché il mio vecchio salvataggio automatico è sparito?

Perché lo slot in cui si trovava è stato riusato. Il gioco ne tiene solo pochi. Uno strumento di backup con le versioni è l'unico modo per recuperarlo una volta ruotato via.

### Hoard sincronizza anche le mie impostazioni?

No. Le impostazioni stanno in un'altra cartella che non fa parte del salvataggio, quindi ogni macchina tiene le sue, che di solito è proprio ciò che vuoi: una Deck e un fisso hanno bisogno di impostazioni grafiche diverse. Maggiori dettagli in [sincronizzare i salvataggi tra PC](/guides/sync-game-saves-across-pcs).
`,la=`---
title: "サイバーパンク2077（Cyberpunk 2077）のセーブデータの場所（PC・Steam Deck）"
description: "Cyberpunk 2077のセーブデータがWindows、Steam Deck、Macのどこにあるか、各フォルダーの中身、バックアップやPC間での移し方を解説。"
order: 20
updated: 2026-10-02
---

Windows では、Cyberpunk 2077 のセーブデータは \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\` にあり、セーブ 1 つにつきフォルダーが 1 つ作られます。これが短い答えです。以下では Steam Deck と Mac のパス、フォルダーの実際の中身、そしてバックアップを保ち続ける方法を説明します。

## Cyberpunk 2077 のセーブデータの場所

- **Windows**（Steam、GOG、Epic）: \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`
- **Steam Deck と Linux**（Proton）: \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`
- **Mac:** \`~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves\`

Windows と Mac のパスは、CD Projekt Red が自社のサポートページで案内しているものです。Steam Deck ではゲームは Proton プレフィックスの中で動きます。これは Steam がゲームごとに用意する小さな Windows のフォルダーツリーで、\`1091500\` は Cyberpunk の Steam アプリ ID です。ゲームを microSD カードにインストールしている場合は、カード上の \`steamapps/compatdata/1091500\` を探してください。

## フォルダーの中身

Cyberpunk はセーブを 1 つのファイルとして書き込むのではなく、**セーブごとにフォルダー**を作ります。\`AutoSave-0\`、\`AutoSave-1\` と続き、\`ManualSave-0\`、\`ManualSave-1\`、\`QuickSave-0\` などです。それぞれにセーブ本体（\`sav.dat\`）と、ロード画面に表示されるスクリーンショットとメタデータが入っています。

ここから 2 つのことが言えます。

- **個別のセーブではなく、親フォルダーをバックアップする。** 最新の \`ManualSave\` だけをコピーすると、最新の進行が入っていることの多いオートセーブが漏れます。
- **オートセーブはローテーションする。** ゲームは少数の \`AutoSave\` フォルダーを使い回し、いちばん古いものを上書きします。3 時間前のオートセーブはたいてい消えているので、ゲームの外に履歴を持つ価値があります。

設定はここにはありません。グラフィックや操作の設定は \`%LOCALAPPDATA%\\CD Projekt Red\\Cyberpunk 2077\` の \`UserSettings.json\` にあり、キャッシュやログと一緒に置かれています。多くの人（や一部のツール）が誤ってバックアップするのがこのフォルダーですが、失っても進行に影響するものは入っていません。

## Cyberpunk 2077 にクラウドセーブはありますか？

あります。Steam 版は Steam クラウドを、GOG 版は GOG Galaxy のクラウドを使います。どちらも同じストアのマシン間で、セーブの最新状態をそろえてくれます。

どちらもしないこと:

- **古いバージョンを残すこと。** セーブが壊れたり Mod で壊れたりすると、クラウドにも壊れたコピーが残ります。
- **ストアをまたぐこと。** Steam クラウドと GOG のクラウドは連携しません。ただし Steam、GOG、Epic の PC 版セーブは、フォルダーをコピーすればどれでも問題なく読み込めます。

## 手動でバックアップする

1. ゲームを完全に終了します。
2. 上記のパスから \`Cyberpunk 2077\` フォルダーを丸ごと、USB メモリや別のドライブ、クラウドフォルダーにコピーします。
3. 復元するときは、ゲームを終了してフォルダーをコピーし戻し、既存のものと置き換えます。

これでも使えますが、覚えているときにしかできず、手元にあるのは前回のコピーだけです。

## Hoard で自動バックアップと同期

[Hoard](/download) は、プレイをやめるたびにセーブフォルダーをバックアップし、すべてのバージョンを保持します。壊れたセーブも、ローテーションで消えたオートセーブも、ワンクリックで戻せます。さらにフォルダーを PC と Steam Deck の間で同期します。

1. Hoard をインストールしてサインインするか、[自分のサーバー](/guides/self-host-hoard)を指定します。
2. **ライブラリ** を開きます。Cyberpunk は Steam ライブラリとコミュニティのセーブデータベースから検出されます。
3. 表示されているフォルダーが \`Saved Games\\CD Projekt Red\\Cyberpunk 2077\` であることを確認します。\`AppData\\Local\` のフォルダーが表示されていたら変更してください。そちらには設定しか入っていません。
4. プレイします。終了すると、最初のバージョンが履歴に表示されます。

Hoard はフォルダー全体を追跡するので、すべての \`AutoSave\`、\`ManualSave\`、\`QuickSave\` が同じバージョンに入ります。Deck とデスクトップなら、次に手に取ったほうで最新バージョンが待っています。詳しくは[PC 間の同期のしくみ](/guides/sync-game-saves-across-pcs)をご覧ください。

<!-- faq -->

## よくある質問

### Steam Deck での Cyberpunk 2077 のセーブデータはどこですか？

ゲームの Proton プレフィックスの中です: \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`。ゲームが microSD カードにある場合、\`compatdata\` フォルダーはカード上にあります。

### Cyberpunk 2077 のセーブを GOG から Steam に移せますか？

はい。PC 版のセーブは Steam、GOG、Epic で共通です。ゲームを閉じた状態で、もう一方のインストールの同じパスにセーブフォルダーをコピーすれば、ロード画面に表示されます。

### なぜ AutoSave フォルダーがたくさんあるのですか？

ゲームはいくつかのオートセーブ枠を持ち、毎回いちばん古いものを上書きします。普通のセーブですが、自動で置き換えられていくのです。

### 古いオートセーブが消えたのはなぜですか？

それが入っていた枠が再利用されたからです。ゲームが残すのは少数だけです。ローテーションで消えた後に取り戻すには、バージョンを保持するバックアップツールしかありません。

### Hoard は設定も同期しますか？

いいえ。設定はセーブに含まれない別のフォルダーにあるので、各マシンが自分の設定を保ちます。たいていはそれが望ましく、Deck とデスクトップでは必要なグラフィック設定が違います。詳しくは[PC 間でセーブを同期する](/guides/sync-game-saves-across-pcs)をご覧ください。
`,ca=`---
title: "Onde ficam os saves de Cyberpunk 2077 (PC e Steam Deck)"
description: "Onde o Cyberpunk 2077 guarda os saves no Windows, Steam Deck e Mac, o que há em cada pasta e como fazer backup ou levá-los de um PC para outro."
order: 20
updated: 2026-10-02
---

No Windows, o Cyberpunk 2077 guarda os saves em \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`, uma pasta por save. Essa é a resposta curta. O resto da página cobre os caminhos na Steam Deck e no Mac, o que há realmente na pasta e como mantê-la sempre com backup.

## Onde o Cyberpunk 2077 guarda os saves

- **Windows** (Steam, GOG ou Epic): \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`
- **Steam Deck e Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`
- **Mac:** \`~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves\`

Os caminhos do Windows e do Mac são os que a CD Projekt Red indica nas suas páginas de suporte. Na Steam Deck, o jogo corre dentro de um prefixo do Proton, uma pequena árvore de pastas do Windows que o Steam mantém para cada jogo; \`1091500\` é o ID do Cyberpunk no Steam. Se o jogo estiver instalado no cartão microSD, procura \`steamapps/compatdata/1091500\` no cartão.

## O que há na pasta

O Cyberpunk não escreve um único ficheiro de save. Escreve **uma pasta por save**: \`AutoSave-0\`, \`AutoSave-1\` e seguintes, \`ManualSave-0\`, \`ManualSave-1\` e \`QuickSave-0\`. Cada uma guarda o save em si (\`sav.dat\`) e a captura e os metadados que o menu de carregamento mostra.

Daqui saem duas coisas:

- **Faz backup da pasta mãe, não de um save solto.** Copiar só o \`ManualSave\` mais recente deixa de fora os autosaves, que muitas vezes têm o progresso mais novo.
- **Os autosaves rodam.** O jogo reutiliza umas poucas pastas \`AutoSave\` e escreve por cima da mais antiga. Um autosave de há três horas normalmente já não existe, e é por isso que compensa ter um histórico fora do jogo.

As definições não estão aqui. Gráficos e controlos vivem em \`UserSettings.json\`, em \`%LOCALAPPDATA%\\CD Projekt Red\\Cyberpunk 2077\`, junto a caches e logs. É a pasta que muita gente (e algumas ferramentas) copia por engano: não guarda nada cuja perda te custe progresso.

## O Cyberpunk 2077 tem saves na nuvem?

Sim. A versão Steam usa o Steam Cloud e a da GOG a nuvem do GOG Galaxy. Ambas mantêm em dia o último estado dos teus saves entre máquinas da mesma loja.

O que nenhuma faz:

- **Guardar versões anteriores.** Se um save se corromper, ou um mod o estragar, a nuvem também fica com a cópia estragada.
- **Atravessar lojas.** O Steam Cloud e a nuvem da GOG não falam um com o outro, embora os saves de PC do Steam, GOG e Epic carreguem sem problemas em qualquer um se copiares a pasta.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a pasta \`Cyberpunk 2077\` inteira do caminho acima para uma pen USB, outro disco ou uma pasta na nuvem.
3. Para restaurar, fecha o jogo e volta a copiar a pasta, substituindo o que lá estiver.

Funciona, mas só com a frequência com que te lembrares, e só tens a cópia da última vez.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) faz backup da pasta de saves sempre que deixas de jogar e guarda todas as versões, por isso um save corrompido ou um autosave que já rodou estão a um clique. Também sincroniza a pasta entre os teus PCs e uma Steam Deck.

1. Instala o Hoard e inicia sessão, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. O Cyberpunk é detetado a partir da tua biblioteca Steam e da base de dados comunitária de saves.
3. Confirma que a pasta mostrada é a de \`Saved Games\\CD Projekt Red\\Cyberpunk 2077\`. Se aparecer a de \`AppData\\Local\`, muda-a: essa só tem definições.
4. Joga. Ao sair, a primeira versão aparece no histórico.

O Hoard acompanha a pasta inteira, por isso cada \`AutoSave\`, \`ManualSave\` e \`QuickSave\` entra na mesma versão. Com uma Deck e um PC, a versão mais recente espera por ti no que pegares a seguir — vê [como funciona a sincronização entre PCs](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Perguntas frequentes

### Onde ficam os saves de Cyberpunk 2077 na Steam Deck?

Dentro do prefixo do Proton do jogo: \`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`. Se o jogo estiver no cartão microSD, a pasta \`compatdata\` está no cartão.

### Posso passar os meus saves de Cyberpunk 2077 da GOG para o Steam?

Sim. Os saves de PC são os mesmos no Steam, GOG e Epic. Copia as pastas de save para o mesmo caminho na outra instalação com o jogo fechado e aparecem no menu de carregamento.

### Porque há tantas pastas AutoSave?

O jogo mantém algumas ranhuras de autosave e escreve por cima da mais antiga de cada vez. São saves normais, só que são substituídos sozinhos.

### Porque desapareceu o meu autosave antigo?

Porque a ranhura onde estava foi reutilizada. O jogo só guarda uns poucos. Uma ferramenta de backup que guarde versões é a única forma de o recuperar depois de rodar.

### O Hoard também sincroniza as minhas definições?

Não. As definições vivem noutra pasta que não faz parte do save, por isso cada máquina mantém as suas, que normalmente é o que queres: uma Deck e um PC precisam de definições gráficas diferentes. Mais sobre isto em [sincronizar saves entre PCs](/guides/sync-game-saves-across-pcs).
`,ma=`---
title: "赛博朋克 2077（Cyberpunk 2077）存档位置（PC 与 Steam Deck）"
description: "Cyberpunk 2077 在 Windows、Steam Deck 和 Mac 上的存档位置，各文件夹里有什么，以及如何备份存档或在 PC 之间迁移。"
order: 20
updated: 2026-10-02
---

在 Windows 上，Cyberpunk 2077 把存档放在 \`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`，每个存档一个文件夹。这是简短的答案。本页其余部分介绍 Steam Deck 和 Mac 上的路径、文件夹里实际有什么，以及如何让它一直有备份。

## Cyberpunk 2077 的存档位置

- **Windows**（Steam、GOG 或 Epic）：\`%USERPROFILE%\\Saved Games\\CD Projekt Red\\Cyberpunk 2077\`
- **Steam Deck 和 Linux**（Proton）：\`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`
- **Mac：**\`~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves\`

Windows 和 Mac 的路径来自 CD Projekt Red 自己的支持页面。在 Steam Deck 上，游戏运行在一个 Proton 前缀里，也就是 Steam 为每款游戏单独维护的一套小型 Windows 目录树；\`1091500\` 是 Cyberpunk 的 Steam 应用 ID。如果游戏装在 microSD 卡上，请到卡上的 \`steamapps/compatdata/1091500\` 查找。

## 文件夹里有什么

Cyberpunk 不是写一个存档文件，而是**每个存档一个文件夹**：\`AutoSave-0\`、\`AutoSave-1\` 依次往后，\`ManualSave-0\`、\`ManualSave-1\`，以及 \`QuickSave-0\`。每个文件夹里都有存档本体（\`sav.dat\`），以及读档菜单显示的截图和元数据。

由此可以得出两点：

- **备份上一级文件夹，而不是单个存档。** 只复制最新的 \`ManualSave\` 会漏掉自动存档，而它们往往才是最新的进度。
- **自动存档会轮换。** 游戏只循环使用少数几个 \`AutoSave\` 文件夹，并覆盖最旧的那个。三小时前的自动存档通常已经没了，所以在游戏之外保留历史是值得的。

设置不在这里。画面和操作设置在 \`%LOCALAPPDATA%\\CD Projekt Red\\Cyberpunk 2077\` 下的 \`UserSettings.json\` 中，和缓存、日志放在一起。很多人（以及一些工具）会误备份这个文件夹：里面没有任何丢了会损失进度的东西。

## Cyberpunk 2077 有云存档吗？

有。Steam 版用 Steam 云，GOG 版用 GOG Galaxy 的云。两者都会在同一商店的设备之间同步存档的最新状态。

两者都做不到的：

- **保留旧版本。** 如果存档损坏或被 Mod 弄坏，云端也会是坏掉的那份。
- **跨商店。** Steam 云和 GOG 的云互不相通，尽管 Steam、GOG 和 Epic 的 PC 存档只要复制文件夹过去，在任何一个版本里都能正常读取。

## 手动备份

1. 完全关闭游戏。
2. 把上面路径中的整个 \`Cyberpunk 2077\` 文件夹复制到 U 盘、另一块硬盘或云盘文件夹。
3. 恢复时，关闭游戏，把文件夹复制回去并替换原有内容。

这样可行，但只能在你记得的时候做，而且手上只有上一次的那份。

## 用 Hoard 自动备份和同步

[Hoard](/download) 会在你每次停止游戏时备份存档文件夹，并保留每一个版本，所以损坏的存档或已经被轮换掉的自动存档只需点一下就能找回。它还会在你的 PC 和 Steam Deck 之间同步这个文件夹。

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**。Cyberpunk 会通过你的 Steam 库和社区存档数据库被识别出来。
3. 确认显示的文件夹是 \`Saved Games\\CD Projekt Red\\Cyberpunk 2077\`。如果显示的是 \`AppData\\Local\` 下的文件夹，请改掉：那里只有设置。
4. 开始游戏。退出后，第一个版本就会出现在历史记录里。

Hoard 跟踪整个文件夹，所以每个 \`AutoSave\`、\`ManualSave\` 和 \`QuickSave\` 都会进入同一个版本。有 Deck 和台式机时，最新版本会在你下一台拿起的设备上等着你——参见[PC 之间同步的原理](/guides/sync-game-saves-across-pcs)。

<!-- faq -->

## 常见问题

### Steam Deck 上的 Cyberpunk 2077 存档在哪里？

在游戏的 Proton 前缀里：\`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077\`。如果游戏装在 microSD 卡上，\`compatdata\` 文件夹就在卡上。

### 能把 Cyberpunk 2077 的存档从 GOG 转到 Steam 吗？

能。Steam、GOG 和 Epic 的 PC 存档是通用的。在游戏关闭时，把存档文件夹复制到另一个安装的相同路径下，它们就会出现在读档菜单里。

### 为什么有这么多 AutoSave 文件夹？

游戏保留几个自动存档槽位，每次覆盖最旧的那个。它们是普通存档，只是会被自动替换。

### 为什么我较早的自动存档不见了？

因为它所在的槽位被重新使用了。游戏只保留少数几个。一旦被轮换掉，只有能保留版本的备份工具才能把它找回来。

### Hoard 也会同步我的设置吗？

不会。设置位于另一个不属于存档的文件夹，所以每台设备保留各自的设置，这通常正是你想要的：Deck 和台式机需要不同的画面设置。详见[在 PC 之间同步存档](/guides/sync-game-saves-across-pcs)。
`,pa=`---
title: "Spielstand-Sync im Vergleich: Hoard gegen Ludusavi, Syncthing, OpenSave und die anderen"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager und mehr im Vergleich: Stärken, Schwächen und eine Tabelle direkt nebeneinander."
order: 4
updated: 2026-10-01
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

## GameSave Manager

Ein altgedientes, kostenloses Windows-Werkzeug, Closed Source, mit eigener Spieledatenbank. Es sichert und stellt wieder her, und seine bekannteste Funktion, **Sync & Link**, verschiebt einen Spielstand-Ordner in einen Cloud-Ordner wie Dropbox oder OneDrive und hinterlässt einen Link, sodass der Cloud-Client ihn synchron hält.

**Am besten, wenn:** du unter Windows bist, ohnehin in Dropbox oder OneDrive lebst und der Spielstand einfach dort liegen soll.

**Wo es aufhört:** nur Windows, und Sync & Link bedeutet, dass ein allgemeiner Cloud-Client den aktiven Ordner synchronisiert, während das Spiel hineinschreibt — dasselbe Muster, das [Syncthing für Spielstände riskant](/guides/syncthing-game-saves) macht. Der Versionsverlauf ist das, was dein Cloud-Speicher aufbewahrt.

## Aletheia

Das jüngste der Runde, AGPL, und es geht genau die Stelle an, die alle anderen halb abdecken: die Launcher. Heroic, itch.io, Lutris, Steam, GOG Galaxy und Xbox, unter Windows, Linux und macOS.

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

## Eine Anmerkung zu EmuDeck

EmuDeck kommt in diesen Gesprächen auf und ist kein Konkurrent im üblichen Sinn: Es ist ein Installer und Konfigurator für Emulatoren auf dem Steam Deck, und der angebotene Sync ist eine Bequemlichkeit, die an diese Aufgabe angeflanscht ist (Rclone gegen ein Cloud-Laufwerk, nur für Emulator-Spielstände). Es überschneidet sich mit den Tools oben, ohne dasselbe zu sein: EmuDeck richtet deine Emulatoren ein, die Tools hier kümmern sich um die Spielstände der ganzen Bibliothek. Manche betreiben EmuDeck neben einem davon, und das ist ein sinnvolles Setup, kein doppeltes.

## Hoard

Hoard nimmt die Spielsitzung als Einheit. Die Engine läuft als Hintergrunddienst — \`hoardd\`, ohne Fenster, also funktioniert sie im Game Mode von SteamOS —, merkt, dass du aufgehört hast zu spielen, und macht dann den Snapshot, statt mitten im Spiel auf jeden Schreibvorgang zu reagieren.

- **Versionshistorie pro Sitzung.** Jede Sitzung ist eine Version, zu der du zurückkannst, auch nach einem Plattenausfall oder einer Neuinstallation.
- **Deduplizierung über Inhalts-Hashes.** Zehn Versionen eines 2-GB-Spielstands kosten rund 2 GB, nicht 20 GB. Übertragungen sind zstd-komprimiert.
- **SHA-256 beim Hochladen und beim Herunterladen.** Beschädigungen werden erkannt, bevor sie einen guten Spielstand überschreiben können. Nichts wird stillschweigend überschrieben — darum geht es im Kern.
- **Cloud oder selbst gehostet, dasselbe Binary.** Hoard Cloud hat einen kostenlosen Tarif (2 GB, 3 Geräte, volle Historie). Oder du betreibst \`hoard-server\` selbst per Docker Compose gegen beliebigen S3-kompatiblen Speicher — MinIO, Garage, Backblaze B2 — ohne Konto und ohne Kontingent. AGPL-3.0.
- **Windows, Linux, macOS**, dazu eine headless CLI für ein Steam Deck oder einen Server.
- **Emulatoren in der Beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP und weitere als Voreinstellungen.

## Das Detail, an dem Steam Deck ↔ PC hängt

Gut zu wissen, egal welches Tool du nimmst. Der Cloud-Spielstand eines Steam-Spiels liegt in \`<AppID>/remote/\`, und der Ordner *darüber* enthält \`remotecache.vdf\`, Erfolgsstände, Statistiken und Spielzeitzähler — alles Dinge, die sich zwischen Deck und Desktop berechtigterweise unterscheiden.

Synchronisiere den übergeordneten Ordner, und du hast einen Dauerkonflikt zwischen zwei Maschinen, die sich über keinen einzigen Spielstand uneinig waren. Hoard verfolgt \`remote/\`, nicht den Elternordner. Jedem Tool, dem du einen Ordner von Hand zuweist, kann man dasselbe beibringen — und es ist das Erste, was man prüft, wenn ein Sync-Setup ohne sichtbaren Grund ständig Konflikte meldet.

## Wo Hoard verliert

- **Es will einen Server.** Cloud-Konto oder eigene Kiste, so oder so ist es Infrastruktur, und OpenSave oder Ludusavi brauchen keine.
- **Emulator-Unterstützung ist Beta.** Portable Installationen und die Eigenheiten einzelner Emulatoren erwischen es noch, und Aletheia und OpenSave decken manche Launcher- und Emulator-Sonderfälle heute besser ab.
- **macOS ist auf echter Hardware kaum getestet.** Es baut und läuft, aber niemand hat monatelang darauf gelebt.
- **Es ist jung.** Ludusavi und Game Backup Monitor haben Jahre an Fehlerberichten hinter sich. Hoard nicht, und das zählt bei etwas, das einen 200-Stunden-Spielstand hütet.
- **Es macht kein Koop-Teilen.** Wenn du einem Freund eine Welt geben willst, ist SaveSync dafür gebaut und Hoard nicht.

## Der Unterschied zwischen Hoard Cloud und Selbsthosten

Vergleiche zu Hoard werfen diese beiden fast immer in einen Topf, und das Ergebnis stimmt dann nicht. Deshalb klar gesagt:

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Stände liegen auf unseren Servern in der EU.
- **Ein selbst gehostetes Hoard gehört vollständig dir.** Du betreibst \`hoard-server\` auf deinem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir sehen weder Spielstand noch Spieltitel noch E-Mail-Adresse, weil davon nichts bei uns ankommt. Würde Hoard Cloud morgen abgeschaltet, liefe ein selbst gehostetes Setup unverändert weiter.

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
| **GameSave Manager** | Über deinen Cloud-Speicher (Sync & Link) | Lokal, plus Dropbox / OneDrive | Was der Cloud-Speicher behält | Windows | Kostenlos, Closed Source |
| **Aletheia** | Sichern und Wiederherstellen pro Launcher | Dein Speicher | Backups | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Ja, auch mit Freunden | Private Steam-Workshop-Einträge | Laut App | Windows | Kostenpflichtig, Closed Source |
| **Tachyon** | Ja, über die eigene Cloud | Tachyons Cloud | Versionen und Timelines | Windows (Beta) | Kostenlose Beta, kein veröffentlichter Code |

## Also welches

Willst du eine Maschine gesichert haben und sonst nichts, nimm Ludusavi oder Game Backup Monitor. Willst du unter keinen Umständen ein Konto und laufen deine Geräte meist gleichzeitig, OpenSave. Sollen die Spielstände in einem Drive-Ordner landen, für den du schon zahlst, OpenCloudSaves. Teilst du eine Koop-Welt mit Freunden, SaveSync.

Willst du, dass Backup *und* Sync zwischen PCs und einem Steam Deck einfach passieren, mit einer Version pro Sitzung, zu der du zurückkannst, und der Option, das Ganze selbst zu hosten, dann ist Hoard dafür da. [Lade es herunter](/download) oder lies vorher, [wie man es mit Docker selbst hostet](/guides/self-host-hoard). Es gibt außerdem einen [ausführlichen Ludusavi-Vergleich](/guides/ludusavi-alternative), falls du genau damit abwägst.

## Direkte Vergleiche

Jeder davon geht tiefer als der Abschnitt oben, samt der Punkte, an denen das andere Werkzeug gewinnt:

- [Hoard gegen Ludusavi](/guides/ludusavi-alternative)
- [Hoard als Steam-Cloud-Alternative](/guides/steam-cloud-alternative)
- [Peer-to-peer gegen einen eigenen Server](/guides/opensave-alternative)
- [Syncthing für Spielstände: was bricht](/guides/syncthing-game-saves)

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

Den Ordner über \`<AppID>/remote/\` in Steams \`userdata\` zu synchronisieren. Der übergeordnete Ordner enthält \`remotecache.vdf\` sowie Dateien für Erfolge und Spielzeit, die sich pro Rechner unterscheiden sollen — jeder Start sieht dann nach einem Konflikt aus, obwohl sich kein Stand bewegt hat.
`,ha=`---
title: "Game save sync compared: Hoard vs Ludusavi, Syncthing, OpenSave and the rest"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager and more compared: what each does best, where each falls short, and a side-by-side table."
order: 4
updated: 2026-10-01
related: ludusavi-alternative, steam-cloud-alternative, syncthing-game-saves
---

Steam Cloud only covers games you bought on Steam, and only when the developer bothered to switch it on. Emulators, GOG, Epic, itch.io, non-Steam games, anything modded — none of that is covered. If you play on more than one machine, a desktop and a Steam Deck say, you end up copying folders by hand and hoping you grabbed the newest one.

Several tools fix this, and they don't all do the same thing. Some make local backups, some mirror folders between devices, some upload to a cloud. This page goes through them and says what each one is genuinely best at. Hoard is my project, so the honest part comes at the end: a section on where Hoard loses, and a table you can read without trusting a word of the prose.

## Ludusavi

The best-known one, and deservedly so. Ludusavi (by mtkennerly) is a free, open-source backup tool with a GUI and a CLI, and it's built on the community save-location manifest that covers tens of thousands of games — the same manifest most of the tools here use, Hoard included. It keeps versioned local backups and can push them to your own cloud through Rclone.

**Best if:** you want local backups, full control, and no server anywhere. It's the safest default on this list and costs nothing.

**Where it stops:** cross-machine sync is a thing you assemble. Schedule a backup, configure an Rclone remote, remember to restore on the other PC *before* you play. It works, but nothing stops you forgetting the last step.

## Syncthing

Not a game tool at all — a general-purpose, peer-to-peer folder mirror, and a very good one. Point it at a save folder and it appears on your other devices.

**Best if:** you already run it and you want files in two places with no cloud in between.

**Where it stops:** it mirrors, it doesn't snapshot. A corrupted save reaches every device in seconds, exactly as fast as a good one. Its file versioning is per-file, with no idea what a play session is, so "roll back to how it was on Tuesday night" is something you reconstruct by hand. Two machines that both played offline give you conflict files, not a merge.

## OpenSave

Peer-to-peer sync built specifically for saves, in Go, MIT licensed, for Windows, Linux and Steam Deck. No account, no server: devices pair with each other and sync over the LAN or through a relay room code. It snapshots every change, has branches for parallel playthroughs, resolves conflicts by sync lineage rather than clock timestamps, and transfers only changed blocks. It can optionally mirror to Drive, Dropbox, OneDrive or WebDAV.

**Best if:** you refuse to have an account, and your devices are on together often enough to actually meet.

**Where it stops:** peer-to-peer means the save lives only on your devices. If the Deck holding the only recent copy dies and the mirror was never configured, that's it. Both devices have to be running for a sync to happen, and there's no macOS build.

## OpenCloudSaves

A cross-platform GUI that syncs your save folders into a cloud you already pay for — OneDrive, Google Drive, Dropbox, Nextcloud — using Rclone underneath.

**Best if:** you want your saves in a storage account you already have, with a UI instead of Rclone config files.

**Where it stops:** there's no content-level deduplication. Ten copies of a 2 GB save is 20 GB of your Drive quota, and cloud drives sync files, not play sessions, so what you get back is whatever the folder looked like at the time.

## Game Backup Monitor

Windows-first, and the original of this whole genre. GBM watches for a game process, and when you quit, it compresses the save with 7-Zip and keeps a numbered history.

**Best if:** you're on one Windows PC and want a compressed local archive with zero thinking.

**Where it stops:** it's a backup tool, not a sync tool. Getting the archive onto a second machine is your problem, and Steam Deck / SteamOS is not its home turf.

## GameSave Manager

A long-standing free Windows tool, closed source, with its own database of games. It backs up and restores, and its best-known feature, **Sync & Link**, moves a save folder into a cloud folder such as Dropbox or OneDrive and leaves a link behind, so the cloud client keeps it in sync.

**Best if:** you're on Windows, you already live in Dropbox or OneDrive, and you want the save simply to be in there.

**Where it stops:** Windows only, and Sync & Link means a general-purpose cloud client syncs the live folder while the game writes to it, the same pattern that makes [Syncthing risky for saves](/guides/syncthing-game-saves). Version history is whatever your cloud drive keeps.

## Aletheia

The newest of the bunch, AGPL, and it goes after the part everyone else half-covers: launchers. Heroic, itch.io, Lutris, Steam, GOG Galaxy and Xbox, across Windows, Linux and macOS.

**Best if:** your library is spread across launchers that other tools detect badly — especially Xbox/Game Pass and Heroic.

**Where it stops:** it's a young project with a deliberately narrow scope. Backup and restore is the feature set; there's no versioned cloud behind it.

## SaveSync

The commercial one, sold on Steam as a one-time purchase, Windows-focused. Its trick is that it isn't really aimed at you-on-two-PCs — it's aimed at co-op. Saves go into private, unlisted Steam Workshop entries so a friend can pull your Valheim or Factorio world, and there's LAN sync too.

**Best if:** the problem you're solving is "my friend hosts and I need their save", not "my saves follow me".

**Where it stops:** closed source, Windows, tied to Steam as the transport, and a set of supported co-op games rather than everything you own.

## Tachyon

The newest name here, in free beta on Windows. Tachyon detects saves for 5,000+ PC games, syncs them through its own cloud and keeps a version history with multiple "timelines".

**Best if:** you're on Windows, want something that works out of the box, and don't mind that it's a beta.

**Where it stops:** Windows only for now (Linux and macOS are listed as coming), so no Steam Deck yet; no published source code; no way to run your own server. Pricing after the beta hasn't been announced.

## A note on EmuDeck

EmuDeck comes up in these conversations, and it isn't a competitor in the normal sense — it's an emulator installer and configurator for Steam Deck, and the sync it offers is a convenience bolted onto that job (Rclone against a cloud drive, for emulator saves only). It overlaps with the tools above without being the same kind of thing: EmuDeck sets your emulators up, the tools here look after saves for the whole library. People do run EmuDeck alongside one of these, and that's a sensible setup, not a redundant one.

## Hoard

Hoard treats a play session as the unit. The engine runs as a background service — \`hoardd\`, no window, so it works in SteamOS game mode — notices you stopped playing, and takes a snapshot then, instead of reacting to every file write mid-game.

- **Version history per session.** Every session is a version you can roll back to, including after a disk failure or a fresh install.
- **Content-hash deduplication.** Ten versions of a 2 GB save cost about 2 GB, not 20 GB. Transfers are zstd-compressed.
- **SHA-256 on the way up and on the way down.** Corruption is caught before it can overwrite a good save. Nothing is ever silently overwritten — that's the whole design.
- **Cloud or self-hosted, same binary.** Hoard Cloud has a free tier (2 GB, 3 devices, full history). Or run \`hoard-server\` yourself with Docker Compose against any S3-compatible storage — MinIO, Garage, Backblaze B2 — with no account and no quota. AGPL-3.0.
- **Windows, Linux, macOS**, plus a headless CLI for a Steam Deck or a server.
- **Emulators in beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP and others as presets.

## The detail that decides Steam Deck ↔ PC sync

Worth knowing whichever tool you pick. A Steam game's cloud save lives in \`<AppID>/remote/\`, and the folder *above* it holds \`remotecache.vdf\`, achievement state, stats and playtime counters — all of which legitimately differ between your Deck and your desktop.

Sync the parent folder and you get a permanent conflict between two machines that never disagreed about a single save. Hoard tracks \`remote/\`, not the parent. Any tool pointed at a folder by hand can be told to do the same, and it's the first thing to check when a sync setup keeps flagging conflicts for no visible reason.

## Where Hoard loses

- **It wants a server.** Cloud account or your own box — either way it's infrastructure, and OpenSave or Ludusavi need none.
- **Emulator support is beta.** Portable installs and per-emulator quirks still catch it out; Aletheia and OpenSave cover some launcher/emulator edge cases better today.
- **macOS is barely tested on real hardware.** It builds and it runs, but nobody has lived on it for months.
- **It's young.** Ludusavi and Game Backup Monitor have years of bug reports behind them. Hoard doesn't, and that matters for something guarding a 200-hour save.
- **It doesn't do co-op sharing.** If you want to hand a world to a friend, SaveSync is built for that and Hoard isn't.

## The Hoard Cloud / self-host distinction

Comparisons of Hoard almost always collapse these two into one, and the result is wrong, so it's worth stating plainly:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **A self-hosted Hoard is entirely yours.** You run \`hoard-server\` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, because none of it ever reaches us. If Hoard Cloud shut down tomorrow, a self-hosted setup would carry on unchanged.

Same binary, same detection, same version history. The only thing that changes is who owns the storage. Being exact about one detail: your own server does have logins of its own — a user and a token per device — but they live in your database, not ours.

## The table

| Tool | Automatic sync between devices | Where saves live | History | Platforms | Licence |
|---|---|---|---|---|---|
| **Hoard** | Yes, per play session | Hoard Cloud or your own server (S3-compatible) | Versioned per session, deduplicated | Win · Linux · macOS · Deck | AGPL-3.0, free tier |
| **Ludusavi** | Manual, or Rclone that you wire up | Local, plus your Rclone remote | Versioned local backups | Win · Linux · macOS | Free, open source |
| **Syncthing** | Yes, continuous mirror | Your devices only | Per-file versioning | Everything | Free, open source |
| **OpenSave** | Yes, peer-to-peer | Your devices, optional cloud mirror | Snapshots and branches | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Yes, via your cloud drive | OneDrive / Drive / Dropbox / Nextcloud | Whatever the drive keeps | Win · Linux · macOS | Free, open source |
| **Game Backup Monitor** | No | Local 7-Zip archives | Numbered backups | Windows | Free, open source |
| **GameSave Manager** | Via your cloud drive (Sync & Link) | Local, plus Dropbox / OneDrive | Whatever the drive keeps | Windows | Free, closed source |
| **Aletheia** | Backup and restore per launcher | Your storage | Backups | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Yes, and with friends | Private Steam Workshop entries | Per the app | Windows | Paid, closed source |
| **Tachyon** | Yes, through its own cloud | Tachyon's cloud | Versions and timelines | Windows (beta) | Free beta, no published source |

## So which one

If you want one machine backed up and nothing else, take Ludusavi or Game Backup Monitor. If you want no account under any circumstances and your devices are usually on together, OpenSave. If your saves should be in a Drive folder you already pay for, OpenCloudSaves. If you're sharing a co-op world with friends, SaveSync.

If you want backups *and* automatic sync across PCs and a Steam Deck to just happen, with a version per session you can roll back to and the option to self-host the whole thing, that's what Hoard is for. [Download it](/download), or read [how to self-host it with Docker](/guides/self-host-hoard) first. There's also a longer [Ludusavi comparison](/guides/ludusavi-alternative) if that's the one you're weighing it against.

## One-on-one comparisons

Each of these goes deeper than the section above, including where the other tool wins:

- [Hoard vs Ludusavi](/guides/ludusavi-alternative)
- [Hoard as a Steam Cloud alternative](/guides/steam-cloud-alternative)
- [Peer-to-peer sync vs a server you own](/guides/opensave-alternative)
- [Syncthing for game saves: what breaks](/guides/syncthing-game-saves)

<!-- faq -->

## Frequently asked questions

### Which of these tools keeps a version history?

Hoard keeps every session as a version you can roll back to. Ludusavi keeps versioned local backups. Most of the rest sync or copy the current state, which means a corrupted save is faithfully propagated to your other machine.

### Which one works without any server or account?

Ludusavi with local backups, and any peer-to-peer tool. Hoard also qualifies if you self-host: no account with us, and nothing passing through our servers.

### Which one covers games that aren't on Steam?

All the save-manager tools here do, because they locate saves through the same community database rather than through a store. Steam Cloud is the one that doesn't: it only covers Steam games whose developer enabled it.

### Do I have to pick just one?

No, and plenty of people don't. A local backup tool and a sync tool solve different halves of the problem. The only rule is never to point one tool at another's backup folder, or you end up syncing a stale mirror instead of your live save.

### What's the single detail that breaks most DIY setups?

Syncing the folder above \`<AppID>/remote/\` in Steam's \`userdata\`. The parent holds \`remotecache.vdf\` plus achievement and playtime files that are supposed to differ per machine, so every launch looks like a conflict even though no save moved.
`,va=`---
title: "Comparativa de sincronización de partidas: Hoard frente a Ludusavi, Syncthing, OpenSave y las demás"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager y más, comparados: en qué destaca cada uno, dónde se queda corto y una tabla lado a lado."
order: 4
updated: 2026-10-01
---

Steam Cloud solo cubre los juegos que compraste en Steam, y solo cuando el desarrollador se molestó en activarlo. Emuladores, GOG, Epic, itch.io, juegos que no son de Steam, cualquier cosa con mods: nada de eso entra. Si juegas en más de un equipo, un sobremesa y una Steam Deck por ejemplo, acabas copiando carpetas a mano y confiando en haber cogido la más reciente.

Hay varias herramientas que resuelven esto y no todas hacen lo mismo. Unas hacen copias locales, otras replican carpetas entre dispositivos, otras suben a una nube. Esta página las repasa y dice en qué es buena de verdad cada una. Hoard es mi proyecto, así que la parte honesta va al final: un apartado sobre dónde pierde Hoard, y una tabla que puedes leer sin fiarte de una sola línea del texto.

## Ludusavi

La más conocida, y con razón. Ludusavi (de mtkennerly) es una herramienta de copia gratuita y open source, con interfaz y con CLI, construida sobre el manifiesto comunitario de ubicaciones de partidas que cubre decenas de miles de juegos: el mismo manifiesto que usan casi todas las de esta lista, Hoard incluido. Guarda copias locales versionadas y puede subirlas a una nube tuya configurando Rclone.

**Mejor si:** quieres copias locales, control total y ningún servidor en ninguna parte. Es la opción más segura de la lista y no cuesta nada.

**Dónde se queda:** la sincronización entre equipos es algo que montas tú. Programas una copia, configuras un remoto de Rclone y te acuerdas de restaurar en el otro PC *antes* de jugar. Funciona, pero nada te impide olvidarte del último paso.

## Syncthing

No es una herramienta de juegos: es un espejo de carpetas peer-to-peer de propósito general, y muy bueno. Le señalas una carpeta de partidas y aparece en tus otros dispositivos.

**Mejor si:** ya lo tienes montado y quieres los ficheros en dos sitios sin nube por medio.

**Dónde se queda:** replica, no fotografía. Una partida corrupta llega a todos los dispositivos en segundos, exactamente igual de rápido que una buena. Su versionado es por fichero, sin noción de qué es una sesión de juego, así que "volver a como estaba el martes por la noche" es algo que reconstruyes a mano. Dos máquinas que jugaron sin conexión te dan ficheros de conflicto, no una fusión.

## OpenSave

Sincronización peer-to-peer hecha específicamente para partidas, en Go, con licencia MIT, para Windows, Linux y Steam Deck. Sin cuenta y sin servidor: los dispositivos se emparejan entre ellos y sincronizan por la red local o a través de un código de sala en un relay. Fotografía cada cambio, tiene ramas para partidas paralelas, resuelve conflictos por linaje de sincronización en vez de por reloj, y transfiere solo los bloques que cambiaron. Opcionalmente puede replicar a Drive, Dropbox, OneDrive o WebDAV.

**Mejor si:** te niegas a tener una cuenta y tus dispositivos coinciden encendidos lo bastante a menudo.

**Dónde se queda:** peer-to-peer significa que la partida vive solo en tus dispositivos. Si muere la Deck que tenía la única copia reciente y nunca configuraste la réplica, se acabó. Los dos dispositivos tienen que estar en marcha para que haya sincronización, y no hay versión para macOS.

## OpenCloudSaves

Una interfaz multiplataforma que sincroniza tus carpetas de partidas contra una nube que ya pagas — OneDrive, Google Drive, Dropbox, Nextcloud — usando Rclone por debajo.

**Mejor si:** quieres tus partidas en una cuenta de almacenamiento que ya tienes, con una interfaz en vez de ficheros de configuración de Rclone.

**Dónde se queda:** no hay deduplicación por contenido. Diez copias de una partida de 2 GB son 20 GB de tu cuota de Drive, y las nubes de disco sincronizan ficheros, no sesiones de juego, así que lo que recuperas es como estuviera la carpeta en ese momento.

## Game Backup Monitor

Primero Windows, y el original de todo este género. GBM vigila el proceso del juego y, cuando sales, comprime la partida con 7-Zip y guarda un historial numerado.

**Mejor si:** estás en un solo PC con Windows y quieres un archivo comprimido local sin pensar en nada.

**Dónde se queda:** es una herramienta de copia, no de sincronización. Llevar el archivo a una segunda máquina es cosa tuya, y Steam Deck / SteamOS no es su terreno.

## GameSave Manager

Una herramienta veterana y gratuita para Windows, de código cerrado y con su propia base de datos de juegos. Hace copias y restaura, y su función más conocida, **Sync & Link**, mueve la carpeta de partidas a una carpeta en la nube como Dropbox u OneDrive y deja un enlace en su lugar, para que el cliente de la nube la mantenga sincronizada.

**Lo mejor si:** estás en Windows, ya vives en Dropbox u OneDrive y quieres que la partida simplemente esté ahí.

**Dónde se queda:** sólo Windows, y Sync & Link significa que un cliente de nube genérico sincroniza la carpeta viva mientras el juego escribe en ella, el mismo patrón que hace [arriesgado Syncthing para las partidas](/guides/syncthing-game-saves). El historial de versiones es el que guarde tu nube.

## Aletheia

La más nueva del grupo, AGPL, y va justo a la parte que las demás cubren a medias: los lanzadores. Heroic, itch.io, Lutris, Steam, GOG Galaxy y Xbox, en Windows, Linux y macOS.

**Mejor si:** tu biblioteca está repartida entre lanzadores que otras herramientas detectan mal, sobre todo Xbox/Game Pass y Heroic.

**Dónde se queda:** es un proyecto joven con un alcance deliberadamente estrecho. Copiar y restaurar es todo el conjunto de funciones; no hay una nube versionada detrás.

## SaveSync

La comercial, se vende en Steam como pago único y está centrada en Windows. Su truco es que no apunta a ti-en-dos-PC, sino al cooperativo: las partidas van a entradas privadas y no listadas del Steam Workshop para que un amigo pueda bajarse tu mundo de Valheim o de Factorio, y además hay sincronización por red local.

**Mejor si:** el problema que resuelves es "mi amigo hospeda y necesito su partida", no "que mis partidas me sigan".

**Dónde se queda:** código cerrado, Windows, atado a Steam como transporte, y una lista de juegos cooperativos soportados en vez de todo lo que tengas.

## Tachyon

El nombre más nuevo de la lista, en beta gratuita para Windows. Tachyon detecta las partidas de más de 5.000 juegos de PC, las sincroniza a través de su propia nube y guarda un historial de versiones con varias «líneas temporales».

**Lo mejor si:** estás en Windows, quieres algo que funcione nada más instalarlo y no te importa que sea una beta.

**Dónde se queda:** de momento sólo Windows (Linux y macOS aparecen como «próximamente»), así que aún no hay Steam Deck; sin código fuente publicado; sin forma de usar tu propio servidor. El precio tras la beta no se ha anunciado.

## Un apunte sobre EmuDeck

EmuDeck sale en estas conversaciones y no es un competidor en el sentido normal: es un instalador y configurador de emuladores para Steam Deck, y la sincronización que ofrece es una comodidad añadida a ese trabajo (Rclone contra una nube de disco, solo para partidas de emulador). Se solapa con las herramientas de arriba sin ser lo mismo: EmuDeck te deja los emuladores montados, y las de aquí cuidan las partidas de toda la biblioteca. Hay gente que usa EmuDeck junto a una de estas, y es un montaje sensato, no redundante.

## Hoard

Hoard toma la sesión de juego como unidad. El motor corre como servicio en segundo plano — \`hoardd\`, sin ventana, así que funciona en el modo juego de SteamOS —, se entera de que has dejado de jugar y hace la instantánea entonces, en vez de reaccionar a cada escritura de fichero en mitad de la partida.

- **Historial versionado por sesión.** Cada sesión es una versión a la que puedes volver, incluso después de un fallo de disco o una instalación limpia.
- **Deduplicación por hash de contenido.** Diez versiones de una partida de 2 GB ocupan unos 2 GB, no 20 GB. Las transferencias van comprimidas con zstd.
- **SHA-256 al subir y al bajar.** La corrupción se detecta antes de que pueda sobrescribir una partida buena. Nada se sobrescribe en silencio: ese es todo el diseño.
- **Nube o autoalojado, el mismo binario.** Hoard Cloud tiene plan gratuito (2 GB, 3 dispositivos, historial completo). O levantas \`hoard-server\` tú mismo con Docker Compose contra cualquier almacenamiento compatible con S3 — MinIO, Garage, Backblaze B2 — sin cuenta y sin cuota. AGPL-3.0.
- **Windows, Linux y macOS**, más una CLI sin interfaz para una Steam Deck o un servidor.
- **Emuladores en beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP y otros como preajustes.

## El detalle que decide la sincronización Steam Deck ↔ PC

Conviene saberlo elijas la herramienta que elijas. La partida en la nube de un juego de Steam vive en \`<AppID>/remote/\`, y la carpeta de *encima* guarda \`remotecache.vdf\`, el estado de logros, estadísticas y contadores de horas jugadas, cosas que legítimamente son distintas entre tu Deck y tu sobremesa.

Sincroniza la carpeta padre y tendrás un conflicto permanente entre dos máquinas que nunca discreparon sobre una sola partida. Hoard rastrea \`remote/\`, no la carpeta padre. A cualquier herramienta a la que le señales una carpeta a mano se le puede decir lo mismo, y es lo primero que hay que mirar cuando un montaje de sincronización marca conflictos sin motivo aparente.

## Dónde pierde Hoard

- **Quiere un servidor.** Cuenta en la nube o máquina tuya, en cualquier caso es infraestructura, y OpenSave o Ludusavi no necesitan ninguna.
- **El soporte de emuladores está en beta.** Las instalaciones portables y las manías de cada emulador todavía lo pillan, y hoy Aletheia y OpenSave cubren mejor algunos casos raros de lanzadores y emuladores.
- **macOS apenas está probado en hardware real.** Compila y funciona, pero nadie ha vivido ahí durante meses.
- **Es joven.** Ludusavi y Game Backup Monitor llevan años de informes de fallos a la espalda. Hoard no, y eso importa en algo que custodia una partida de 200 horas.
- **No hace cooperativo.** Si quieres pasarle un mundo a un amigo, SaveSync está hecho para eso y Hoard no.

## La distinción entre Hoard Cloud y autoalojarse

Las comparativas sobre Hoard casi siempre funden las dos en una, y el resultado sale mal, así que conviene decirlo claro:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Un Hoard autoalojado es tuyo por completo.** Levantas \`hoard-server\` en tu PC o en tu NAS y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, porque nada de eso nos llega. Si Hoard Cloud cerrara mañana, un montaje autoalojado seguiría igual.

El mismo binario, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento. Y siendo exactos en un detalle: tu servidor sí tiene sus propios accesos — un usuario y un token por dispositivo — pero viven en tu base de datos, no en la nuestra.

## La tabla

| Herramienta | Sincronización automática entre dispositivos | Dónde viven las partidas | Historial | Plataformas | Licencia |
|---|---|---|---|---|---|
| **Hoard** | Sí, por sesión de juego | Hoard Cloud o tu propio servidor (compatible con S3) | Versionado por sesión, deduplicado | Win · Linux · macOS · Deck | AGPL-3.0, plan gratuito |
| **Ludusavi** | Manual, o Rclone que montas tú | Local, más tu remoto de Rclone | Copias locales versionadas | Win · Linux · macOS | Gratis, open source |
| **Syncthing** | Sí, espejo continuo | Solo tus dispositivos | Versionado por fichero | Todo | Gratis, open source |
| **OpenSave** | Sí, peer-to-peer | Tus dispositivos, réplica opcional en nube | Instantáneas y ramas | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Sí, vía tu nube de disco | OneDrive / Drive / Dropbox / Nextcloud | Lo que guarde la nube | Win · Linux · macOS | Gratis, open source |
| **Game Backup Monitor** | No | Archivos 7-Zip locales | Copias numeradas | Windows | Gratis, open source |
| **GameSave Manager** | A través de tu nube (Sync & Link) | Local, más Dropbox / OneDrive | Lo que guarde la nube | Windows | Gratis, código cerrado |
| **Aletheia** | Copia y restauración por lanzador | Tu almacenamiento | Copias | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sí, y con amigos | Entradas privadas del Steam Workshop | Según la app | Windows | De pago, código cerrado |
| **Tachyon** | Sí, a través de su propia nube | La nube de Tachyon | Versiones y líneas temporales | Windows (beta) | Beta gratuita, sin código publicado |

## Entonces cuál

Si quieres una sola máquina respaldada y nada más, coge Ludusavi o Game Backup Monitor. Si no quieres una cuenta bajo ningún concepto y tus dispositivos suelen estar encendidos a la vez, OpenSave. Si tus partidas deben acabar en una carpeta de Drive que ya pagas, OpenCloudSaves. Si compartes un mundo cooperativo con amigos, SaveSync.

Si lo que quieres es que la copia *y* la sincronización entre PC y una Steam Deck pasen solas, con una versión por sesión a la que volver y la opción de autoalojarlo todo, para eso está Hoard. [Descárgalo](/download), o léete antes [cómo autoalojarlo con Docker](/guides/self-host-hoard). También hay una [comparativa larga con Ludusavi](/guides/ludusavi-alternative) si es esa la que estás sopesando.

## Comparativas una a una

Cada una entra más a fondo que el bloque de arriba, incluido dónde gana la otra herramienta:

- [Hoard frente a Ludusavi](/guides/ludusavi-alternative)
- [Hoard como alternativa a Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronización punto a punto frente a un servidor tuyo](/guides/opensave-alternative)
- [Syncthing para partidas: qué se rompe](/guides/syncthing-game-saves)

<!-- faq -->

## Preguntas frecuentes

### ¿Cuál de estas herramientas guarda historial de versiones?

Hoard conserva cada sesión como una versión a la que puedes volver. Ludusavi guarda copias locales versionadas. La mayoría del resto sincroniza o copia el estado actual, lo que significa que una partida corrupta se propaga fielmente a tu otra máquina.

### ¿Cuál funciona sin servidor ni cuenta?

Ludusavi con copias locales, y cualquier herramienta punto a punto. Hoard también entra si te autoalojas: sin cuenta con nosotros y sin nada que pase por nuestros servidores.

### ¿Cuál cubre juegos que no están en Steam?

Todas las herramientas de gestión de partidas de aquí, porque localizan los saves con la misma base de datos comunitaria y no a través de una tienda. La que no lo hace es Steam Cloud: sólo cubre juegos de Steam cuyo desarrollador lo activó.

### ¿Tengo que quedarme con una sola?

No, y mucha gente no lo hace. Una herramienta de copia local y una de sincronización resuelven mitades distintas del problema. La única regla es no apuntar nunca una a la carpeta de copias de la otra, o acabas sincronizando un espejo desfasado en vez de tu partida real.

### ¿Cuál es el detalle que rompe la mayoría de montajes caseros?

Sincronizar la carpeta que está por encima de \`<AppID>/remote/\` en el \`userdata\` de Steam. La padre guarda \`remotecache.vdf\` y ficheros de logros y tiempo jugado que deben ser distintos en cada máquina, así que cada arranque parece un conflicto aunque no se haya movido ninguna partida.
`,ga=`---
title: "Comparatif de synchronisation des sauvegardes : Hoard face à Ludusavi, Syncthing, OpenSave et les autres"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager et d'autres comparés : points forts, limites de chacun et un tableau côte à côte."
order: 4
updated: 2026-10-01
---

Steam Cloud ne couvre que les jeux achetés sur Steam, et seulement quand le développeur a pris la peine de l'activer. Émulateurs, GOG, Epic, itch.io, jeux hors Steam, tout ce qui est moddé : rien de tout ça n'est couvert. Si vous jouez sur plusieurs machines, un fixe et un Steam Deck par exemple, vous finissez par copier des dossiers à la main en espérant avoir pris le plus récent.

Plusieurs outils règlent le problème, et ils ne font pas tous la même chose. Certains font des sauvegardes locales, d'autres répliquent des dossiers entre appareils, d'autres envoient vers un cloud. Cette page les passe en revue et dit ce que chacun fait vraiment bien. Hoard est mon projet, donc la partie honnête arrive à la fin : une section sur les points faibles de Hoard, et un tableau lisible sans croire un mot du texte.

## Ludusavi

Le plus connu, et à juste titre. Ludusavi (de mtkennerly) est un outil de sauvegarde gratuit et open source, avec interface et ligne de commande, bâti sur le manifeste communautaire des emplacements de sauvegardes qui couvre des dizaines de milliers de jeux — le même manifeste qu'utilisent presque tous les outils d'ici, Hoard compris. Il conserve des sauvegardes locales versionnées et peut les pousser vers votre propre cloud via Rclone.

**Le meilleur si :** vous voulez des sauvegardes locales, le contrôle total et aucun serveur nulle part. C'est le choix le plus sûr de la liste, et il est gratuit.

**Là où il s'arrête :** la synchronisation entre machines, c'est vous qui l'assemblez. Planifier une sauvegarde, configurer un remote Rclone, puis penser à restaurer sur l'autre PC *avant* de jouer. Ça marche, mais rien ne vous empêche d'oublier la dernière étape.

## Syncthing

Pas du tout un outil de jeu : un miroir de dossiers pair-à-pair généraliste, et très bon. Vous lui désignez un dossier de sauvegardes et il apparaît sur vos autres appareils.

**Le meilleur si :** vous l'utilisez déjà et vous voulez les fichiers à deux endroits sans cloud entre les deux.

**Là où il s'arrête :** il réplique, il ne photographie pas. Une sauvegarde corrompue atteint tous les appareils en quelques secondes, exactement aussi vite qu'une bonne. Son versionnage est par fichier, sans notion de session de jeu, donc « revenir à mardi soir » se reconstruit à la main. Deux machines qui ont joué hors ligne vous donnent des fichiers de conflit, pas une fusion.

## OpenSave

Synchronisation pair-à-pair conçue spécifiquement pour les sauvegardes, en Go, sous licence MIT, pour Windows, Linux et Steam Deck. Pas de compte, pas de serveur : les appareils s'appairent entre eux et se synchronisent en réseau local ou via un code de salon sur un relais. Chaque changement devient un instantané, il y a des branches pour les parties parallèles, les conflits se résolvent par lignage de synchronisation plutôt que par horloge, et seuls les blocs modifiés circulent. Il peut, en option, répliquer vers Drive, Dropbox, OneDrive ou WebDAV.

**Le meilleur si :** vous refusez d'avoir un compte et vos appareils sont allumés en même temps assez souvent.

**Là où il s'arrête :** pair-à-pair veut dire que la sauvegarde ne vit que sur vos appareils. Si le Deck qui détenait la seule copie récente meurt et que la réplication n'a jamais été configurée, c'est terminé. Les deux appareils doivent tourner pour qu'une synchronisation ait lieu, et il n'y a pas de version macOS.

## OpenCloudSaves

Une interface multiplateforme qui synchronise vos dossiers de sauvegardes vers un cloud que vous payez déjà — OneDrive, Google Drive, Dropbox, Nextcloud — avec Rclone en dessous.

**Le meilleur si :** vous voulez vos sauvegardes dans un espace de stockage que vous avez déjà, avec une interface plutôt que des fichiers de configuration Rclone.

**Là où il s'arrête :** pas de déduplication au niveau du contenu. Dix copies d'une sauvegarde de 2 Go, ce sont 20 Go de votre quota Drive, et les clouds de fichiers synchronisent des fichiers, pas des sessions de jeu : vous récupérez l'état du dossier à un instant donné, rien de plus.

## Game Backup Monitor

D'abord Windows, et l'ancêtre de tout ce genre. GBM guette le processus du jeu et, à la fermeture, compresse la sauvegarde avec 7-Zip en gardant un historique numéroté.

**Le meilleur si :** vous êtes sur un seul PC Windows et voulez une archive locale compressée sans y penser.

**Là où il s'arrête :** c'est un outil de sauvegarde, pas de synchronisation. Amener l'archive sur une deuxième machine, c'est votre affaire, et Steam Deck / SteamOS n'est pas son terrain.

## GameSave Manager

Un outil Windows gratuit de longue date, propriétaire, avec sa propre base de jeux. Il sauvegarde et restaure, et sa fonction la plus connue, **Sync & Link**, déplace un dossier de sauvegarde dans un dossier cloud comme Dropbox ou OneDrive et laisse un lien à sa place, pour que le client cloud le garde synchronisé.

**Idéal si :** vous êtes sous Windows, vivez déjà dans Dropbox ou OneDrive et voulez simplement que la sauvegarde y soit.

**Où il s'arrête :** Windows uniquement, et Sync & Link signifie qu'un client cloud généraliste synchronise le dossier actif pendant que le jeu écrit dedans, le même schéma qui rend [Syncthing risqué pour les sauvegardes](/guides/syncthing-game-saves). L'historique est celui que garde votre cloud.

## Aletheia

Le plus récent du lot, sous AGPL, et il attaque précisément ce que les autres couvrent à moitié : les lanceurs. Heroic, itch.io, Lutris, Steam, GOG Galaxy et Xbox, sous Windows, Linux et macOS.

**Le meilleur si :** votre bibliothèque est éparpillée sur des lanceurs que les autres outils détectent mal, en particulier Xbox/Game Pass et Heroic.

**Là où il s'arrête :** projet jeune, au périmètre volontairement étroit. Sauvegarder et restaurer, c'est tout ; il n'y a pas de cloud versionné derrière.

## SaveSync

Le commercial, vendu sur Steam en achat unique, orienté Windows. Sa particularité : il ne vise pas vous-sur-deux-PC mais le coop. Les sauvegardes partent dans des entrées privées et non listées du Steam Workshop pour qu'un ami récupère votre monde Valheim ou Factorio, et il y a aussi une synchronisation en réseau local.

**Le meilleur si :** votre problème est « mon ami héberge et il me faut sa sauvegarde », pas « que mes sauvegardes me suivent ».

**Là où il s'arrête :** code fermé, Windows, dépendant de Steam comme transport, et une liste de jeux coop pris en charge plutôt que tout ce que vous possédez.

## Tachyon

Le nom le plus récent de la liste, en bêta gratuite sous Windows. Tachyon détecte les sauvegardes de plus de 5 000 jeux PC, les synchronise via son propre cloud et conserve un historique de versions avec plusieurs « timelines ».

**Idéal si :** vous êtes sous Windows, voulez quelque chose qui marche tout de suite et qu'une bêta ne vous gêne pas.

**Où il s'arrête :** Windows uniquement pour l'instant (Linux et macOS sont annoncés), donc pas encore de Steam Deck ; pas de code source publié ; impossible d'utiliser son propre serveur. Les tarifs après la bêta ne sont pas annoncés.

## Une note sur EmuDeck

EmuDeck revient dans ces discussions, et ce n'est pas un concurrent au sens habituel : c'est un installateur et configurateur d'émulateurs pour Steam Deck, et la synchronisation qu'il propose est un confort greffé sur cette mission (Rclone vers un cloud de fichiers, pour les sauvegardes d'émulateurs uniquement). Il recoupe les outils ci-dessus sans être de la même nature : EmuDeck installe vos émulateurs, les outils d'ici veillent sur les sauvegardes de toute la bibliothèque. Beaucoup font tourner EmuDeck à côté de l'un d'eux, et c'est une configuration sensée, pas une redondance.

## Hoard

Hoard prend la session de jeu comme unité. Le moteur tourne en service d'arrière-plan — \`hoardd\`, sans fenêtre, donc il fonctionne en mode jeu de SteamOS —, remarque que vous avez arrêté de jouer, et prend l'instantané à ce moment-là plutôt que de réagir à chaque écriture pendant la partie.

- **Historique versionné par session.** Chaque session est une version vers laquelle revenir, même après une panne de disque ou une réinstallation.
- **Déduplication par empreinte de contenu.** Dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20 Go. Les transferts sont compressés en zstd.
- **SHA-256 à la montée et à la descente.** La corruption est détectée avant de pouvoir écraser une bonne sauvegarde. Rien n'est jamais écrasé en silence : c'est tout le principe.
- **Cloud ou auto-hébergé, le même binaire.** Hoard Cloud a une offre gratuite (2 Go, 3 appareils, historique complet). Ou vous lancez \`hoard-server\` vous-même avec Docker Compose sur n'importe quel stockage compatible S3 — MinIO, Garage, Backblaze B2 — sans compte ni quota. AGPL-3.0.
- **Windows, Linux, macOS**, plus une CLI sans interface pour un Steam Deck ou un serveur.
- **Émulateurs en bêta :** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP et d'autres en préréglages.

## Le détail qui décide de la synchro Steam Deck ↔ PC

Bon à savoir quel que soit l'outil choisi. La sauvegarde cloud d'un jeu Steam vit dans \`<AppID>/remote/\`, et le dossier *au-dessus* contient \`remotecache.vdf\`, l'état des succès, les statistiques et les compteurs de temps de jeu — autant de choses qui diffèrent légitimement entre votre Deck et votre fixe.

Synchronisez le dossier parent et vous obtenez un conflit permanent entre deux machines qui n'ont jamais été en désaccord sur une seule sauvegarde. Hoard suit \`remote/\`, pas le dossier parent. N'importe quel outil auquel vous désignez un dossier à la main peut faire pareil, et c'est la première chose à vérifier quand une configuration de synchronisation signale des conflits sans raison visible.

## Là où Hoard perd

- **Il veut un serveur.** Compte cloud ou machine à vous, dans les deux cas c'est de l'infrastructure, alors qu'OpenSave ou Ludusavi n'en demandent aucune.
- **La prise en charge des émulateurs est en bêta.** Les installations portables et les manies de chaque émulateur le piègent encore, et Aletheia comme OpenSave couvrent aujourd'hui mieux certains cas particuliers de lanceurs et d'émulateurs.
- **macOS est à peine testé sur du matériel réel.** Ça compile et ça tourne, mais personne n'y a vécu pendant des mois.
- **C'est jeune.** Ludusavi et Game Backup Monitor ont des années de rapports de bugs derrière eux. Pas Hoard, et ça compte pour un logiciel qui garde une partie de 200 heures.
- **Il ne fait pas le partage coop.** Pour passer un monde à un ami, SaveSync est fait pour ça, Hoard non.

## La distinction entre Hoard Cloud et l'auto-hébergement

Les comparaisons sur Hoard confondent presque toujours les deux, et le résultat est faux. Autant le dire clairement :

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont stockées sur nos serveurs, dans l'UE.
- **Un Hoard auto-hébergé est entièrement le vôtre.** Vous faites tourner \`hoard-server\` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne voyons ni sauvegarde, ni nom de jeu, ni adresse e-mail, car rien de cela ne nous parvient. Si Hoard Cloud fermait demain, une installation auto-hébergée continuerait à l'identique.

Le même binaire, la même détection, le même historique. La seule chose qui change, c'est à qui appartient le stockage. Et pour être exact sur un point : votre serveur a bien ses propres accès — un utilisateur et un jeton par appareil — mais ils vivent dans votre base, pas dans la nôtre.

## Le tableau

| Outil | Synchro automatique entre appareils | Où vivent les sauvegardes | Historique | Plateformes | Licence |
|---|---|---|---|---|---|
| **Hoard** | Oui, par session de jeu | Hoard Cloud ou votre serveur (compatible S3) | Versionné par session, dédupliqué | Win · Linux · macOS · Deck | AGPL-3.0, offre gratuite |
| **Ludusavi** | Manuelle, ou Rclone que vous montez | Local, plus votre remote Rclone | Sauvegardes locales versionnées | Win · Linux · macOS | Gratuit, open source |
| **Syncthing** | Oui, miroir continu | Vos appareils seulement | Versionnage par fichier | Tout | Gratuit, open source |
| **OpenSave** | Oui, pair-à-pair | Vos appareils, réplication cloud optionnelle | Instantanés et branches | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Oui, via votre cloud | OneDrive / Drive / Dropbox / Nextcloud | Ce que garde le cloud | Win · Linux · macOS | Gratuit, open source |
| **Game Backup Monitor** | Non | Archives 7-Zip locales | Sauvegardes numérotées | Windows | Gratuit, open source |
| **GameSave Manager** | Via votre cloud (Sync & Link) | Local, plus Dropbox / OneDrive | Ce que garde le cloud | Windows | Gratuit, propriétaire |
| **Aletheia** | Sauvegarde et restauration par lanceur | Votre stockage | Sauvegardes | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Oui, et avec des amis | Entrées privées du Steam Workshop | Selon l'application | Windows | Payant, code fermé |
| **Tachyon** | Oui, via son propre cloud | Le cloud de Tachyon | Versions et timelines | Windows (bêta) | Bêta gratuite, code non publié |

## Alors lequel

Si vous voulez une seule machine sauvegardée et rien d'autre, prenez Ludusavi ou Game Backup Monitor. Si vous refusez tout compte et que vos appareils sont généralement allumés ensemble, OpenSave. Si vos sauvegardes doivent atterrir dans un dossier Drive que vous payez déjà, OpenCloudSaves. Si vous partagez un monde coop avec des amis, SaveSync.

Si vous voulez que la sauvegarde *et* la synchronisation entre PC et Steam Deck se fassent toutes seules, avec une version par session où revenir et la possibilité de tout auto-héberger, c'est à ça que sert Hoard. [Téléchargez-le](/download), ou lisez d'abord [comment l'auto-héberger avec Docker](/guides/self-host-hoard). Il y a aussi un [comparatif détaillé avec Ludusavi](/guides/ludusavi-alternative) si c'est celui que vous mettez dans la balance.

## Comparaisons en tête-à-tête

Chacune va plus loin que la section ci-dessus, y compris sur les points où l'autre outil l'emporte :

- [Hoard face à Ludusavi](/guides/ludusavi-alternative)
- [Hoard comme alternative à Steam Cloud](/guides/steam-cloud-alternative)
- [Synchro pair-à-pair face à un serveur qui vous appartient](/guides/opensave-alternative)
- [Syncthing pour les sauvegardes : ce qui casse](/guides/syncthing-game-saves)

<!-- faq -->

## Questions fréquentes

### Lequel de ces outils garde un historique de versions ?

Hoard conserve chaque session comme une version où revenir. Ludusavi garde des sauvegardes locales versionnées. La plupart des autres synchronisent ou copient l'état actuel : une sauvegarde corrompue est donc fidèlement propagée à votre autre machine.

### Lequel fonctionne sans serveur ni compte ?

Ludusavi en sauvegardes locales, et tout outil pair-à-pair. Hoard entre aussi dans cette catégorie si vous l'auto-hébergez : aucun compte chez nous, et rien qui passe par nos serveurs.

### Lequel couvre les jeux absents de Steam ?

Tous les gestionnaires de sauvegardes cités, car ils localisent les fichiers via la même base communautaire et non via une boutique. L'exception est Steam Cloud : il ne couvre que les jeux Steam dont le développeur l'a activé.

### Dois-je n'en choisir qu'un ?

Non, et beaucoup ne le font pas. Un outil de sauvegarde locale et un outil de synchro règlent deux moitiés différentes du problème. La seule règle : ne jamais pointer l'un vers le dossier de sauvegardes de l'autre, sinon vous synchronisez un miroir périmé au lieu de votre sauvegarde réelle.

### Quel est le détail qui casse la plupart des montages maison ?

Synchroniser le dossier situé au-dessus de \`<AppID>/remote/\` dans le \`userdata\` de Steam. Le parent contient \`remotecache.vdf\` et des fichiers de succès et de temps de jeu censés différer d'une machine à l'autre : chaque lancement ressemble alors à un conflit alors qu'aucune sauvegarde n'a bougé.
`,Sa=`---
title: "Sincronizzazione dei salvataggi a confronto: Hoard contro Ludusavi, Syncthing, OpenSave e le altre"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager e altri a confronto: punti di forza, limiti di ognuno e una tabella fianco a fianco."
order: 4
updated: 2026-10-01
---

Steam Cloud copre solo i giochi comprati su Steam, e solo quando lo sviluppatore si è preso la briga di attivarlo. Emulatori, GOG, Epic, itch.io, giochi non Steam, qualsiasi cosa con mod: niente di tutto questo rientra. Se giochi su più macchine, un fisso e uno Steam Deck per dire, finisci a copiare cartelle a mano sperando di aver preso la più recente.

Diversi strumenti risolvono la cosa, e non fanno tutti lo stesso. Alcuni fanno copie locali, altri replicano cartelle tra dispositivi, altri caricano su un cloud. Questa pagina li passa in rassegna e dice in cosa ciascuno è davvero bravo. Hoard è il mio progetto, quindi la parte onesta arriva alla fine: una sezione su dove Hoard perde, e una tabella che puoi leggere senza credere a una parola del testo.

## Ludusavi

Il più noto, e a ragione. Ludusavi (di mtkennerly) è uno strumento di backup gratuito e open source, con interfaccia e con CLI, costruito sul manifesto comunitario delle posizioni dei salvataggi che copre decine di migliaia di giochi: lo stesso manifesto che usano quasi tutti quelli di questa lista, Hoard compreso. Tiene copie locali versionate e può spingerle su un cloud tuo tramite Rclone.

**Il migliore se:** vuoi copie locali, controllo totale e nessun server da nessuna parte. È la scelta più sicura della lista e non costa nulla.

**Dove si ferma:** la sincronizzazione tra macchine è una cosa che monti tu. Pianifichi un backup, configuri un remote Rclone e ti ricordi di ripristinare sull'altro PC *prima* di giocare. Funziona, ma nulla ti impedisce di dimenticare l'ultimo passo.

## Syncthing

Non è affatto uno strumento per giochi: è uno specchio di cartelle peer-to-peer generico, e molto buono. Gli indichi una cartella di salvataggi e compare sugli altri dispositivi.

**Il migliore se:** lo usi già e vuoi i file in due posti senza cloud in mezzo.

**Dove si ferma:** replica, non fotografa. Un salvataggio corrotto raggiunge ogni dispositivo in pochi secondi, esattamente alla stessa velocità di uno buono. Il versionamento è per file, senza alcuna idea di cosa sia una sessione di gioco, quindi «torna a com'era martedì sera» te lo ricostruisci a mano. Due macchine che hanno giocato entrambe offline ti danno file di conflitto, non una fusione.

## OpenSave

Sincronizzazione peer-to-peer costruita apposta per i salvataggi, in Go, con licenza MIT, per Windows, Linux e Steam Deck. Nessun account, nessun server: i dispositivi si accoppiano tra loro e sincronizzano sulla rete locale o tramite un codice stanza su un relay. Ogni modifica diventa uno snapshot, ci sono i branch per partite parallele, i conflitti si risolvono per lignaggio di sincronizzazione invece che per orologio, e viaggiano solo i blocchi cambiati. Volendo può replicare su Drive, Dropbox, OneDrive o WebDAV.

**Il migliore se:** ti rifiuti di avere un account e i tuoi dispositivi sono accesi insieme abbastanza spesso.

**Dove si ferma:** peer-to-peer vuol dire che il salvataggio vive solo sui tuoi dispositivi. Se muore il Deck con l'unica copia recente e la replica non era configurata, è finita. Per sincronizzare devono essere accesi entrambi i dispositivi, e non c'è una build per macOS.

## OpenCloudSaves

Un'interfaccia multipiattaforma che sincronizza le cartelle dei salvataggi su un cloud che già paghi — OneDrive, Google Drive, Dropbox, Nextcloud — con Rclone sotto.

**Il migliore se:** vuoi i salvataggi in uno spazio di archiviazione che hai già, con un'interfaccia invece dei file di configurazione di Rclone.

**Dove si ferma:** non c'è deduplicazione a livello di contenuto. Dieci copie di un salvataggio da 2 GB sono 20 GB della tua quota Drive, e i cloud di file sincronizzano file, non sessioni di gioco: quel che recuperi è com'era la cartella in quel momento.

## Game Backup Monitor

Prima Windows, e il capostipite di tutto il genere. GBM sorveglia il processo del gioco e, quando esci, comprime il salvataggio con 7-Zip tenendo una cronologia numerata.

**Il migliore se:** sei su un solo PC Windows e vuoi un archivio locale compresso senza pensarci.

**Dove si ferma:** è uno strumento di backup, non di sincronizzazione. Portare l'archivio su una seconda macchina è affare tuo, e Steam Deck / SteamOS non è il suo terreno.

## GameSave Manager

Uno strumento Windows gratuito di lunga data, closed source, con un proprio database di giochi. Fa backup e ripristino, e la sua funzione più nota, **Sync & Link**, sposta una cartella di salvataggio in una cartella cloud come Dropbox o OneDrive e lascia un collegamento al suo posto, così il client cloud la tiene sincronizzata.

**Ideale se:** sei su Windows, vivi già in Dropbox o OneDrive e vuoi semplicemente che il salvataggio stia lì.

**Dove si ferma:** solo Windows, e Sync & Link significa che un client cloud generico sincronizza la cartella attiva mentre il gioco ci scrive, lo stesso schema che rende [Syncthing rischioso per i salvataggi](/guides/syncthing-game-saves). La cronologia è quella che tiene il tuo cloud.

## Aletheia

Il più nuovo del gruppo, AGPL, e va proprio sulla parte che gli altri coprono a metà: i launcher. Heroic, itch.io, Lutris, Steam, GOG Galaxy e Xbox, su Windows, Linux e macOS.

**Il migliore se:** la tua libreria è sparsa tra launcher che gli altri strumenti rilevano male, soprattutto Xbox/Game Pass e Heroic.

**Dove si ferma:** è un progetto giovane con un perimetro volutamente stretto. Copia e ripristino sono tutto il set di funzioni; dietro non c'è un cloud versionato.

## SaveSync

Quello commerciale, venduto su Steam con acquisto unico, centrato su Windows. Il suo trucco è che non punta a te-su-due-PC ma al cooperativo: i salvataggi finiscono in voci private e non elencate dello Steam Workshop così che un amico possa scaricarsi il tuo mondo di Valheim o di Factorio, e c'è anche la sincronizzazione in rete locale.

**Il migliore se:** il problema che risolvi è «ospita il mio amico e mi serve il suo salvataggio», non «che i miei salvataggi mi seguano».

**Dove si ferma:** codice chiuso, Windows, legato a Steam come mezzo di trasporto, e un elenco di giochi cooperativi supportati invece di tutto quello che possiedi.

## Tachyon

Il nome più recente della lista, in beta gratuita su Windows. Tachyon rileva i salvataggi di oltre 5.000 giochi PC, li sincronizza tramite il proprio cloud e tiene una cronologia delle versioni con più "timeline".

**Ideale se:** sei su Windows, vuoi qualcosa che funzioni subito e non ti dà fastidio che sia una beta.

**Dove si ferma:** per ora solo Windows (Linux e macOS sono annunciati), quindi niente Steam Deck; nessun codice sorgente pubblicato; nessun modo di usare il proprio server. I prezzi dopo la beta non sono stati annunciati.

## Una nota su EmuDeck

EmuDeck salta fuori in queste discussioni e non è un concorrente nel senso normale: è un installatore e configuratore di emulatori per Steam Deck, e la sincronizzazione che offre è una comodità innestata su quel lavoro (Rclone verso un cloud di file, solo per i salvataggi degli emulatori). Si sovrappone agli strumenti qui sopra senza essere la stessa cosa: EmuDeck ti sistema gli emulatori, quelli di qui si occupano dei salvataggi dell'intera libreria. C'è chi usa EmuDeck accanto a uno di questi, ed è una configurazione sensata, non ridondante.

## Hoard

Hoard prende la sessione di gioco come unità. Il motore gira come servizio in background — \`hoardd\`, senza finestra, quindi funziona in modalità gioco su SteamOS —, si accorge che hai smesso di giocare e scatta lo snapshot allora, invece di reagire a ogni scrittura di file mentre giochi.

- **Cronologia versionata per sessione.** Ogni sessione è una versione a cui tornare, anche dopo un guasto al disco o un'installazione pulita.
- **Deduplicazione per hash del contenuto.** Dieci versioni di un salvataggio da 2 GB costano circa 2 GB, non 20 GB. I trasferimenti sono compressi con zstd.
- **SHA-256 in salita e in discesa.** La corruzione viene intercettata prima che possa sovrascrivere un salvataggio buono. Niente viene mai sovrascritto in silenzio: è tutto il senso del progetto.
- **Cloud o self-hosted, lo stesso binario.** Hoard Cloud ha un piano gratuito (2 GB, 3 dispositivi, cronologia completa). Oppure avvii \`hoard-server\` da solo con Docker Compose su qualsiasi archiviazione compatibile S3 — MinIO, Garage, Backblaze B2 — senza account e senza quota. AGPL-3.0.
- **Windows, Linux, macOS**, più una CLI senza interfaccia per uno Steam Deck o un server.
- **Emulatori in beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP e altri come preimpostazioni.

## Il dettaglio che decide la sincronizzazione Steam Deck ↔ PC

Vale la pena saperlo qualunque strumento tu scelga. Il salvataggio cloud di un gioco Steam vive in \`<AppID>/remote/\`, e la cartella *sopra* contiene \`remotecache.vdf\`, lo stato degli obiettivi, le statistiche e i contatori delle ore giocate: tutte cose che legittimamente differiscono tra il Deck e il fisso.

Sincronizza la cartella padre e ottieni un conflitto permanente tra due macchine che non hanno mai discordato su un solo salvataggio. Hoard traccia \`remote/\`, non la cartella padre. A qualsiasi strumento a cui indichi una cartella a mano si può dire lo stesso, ed è la prima cosa da controllare quando una configurazione di sincronizzazione segnala conflitti senza motivo visibile.

## Dove Hoard perde

- **Vuole un server.** Account cloud o macchina tua, in ogni caso è infrastruttura, mentre OpenSave o Ludusavi non ne richiedono nessuna.
- **Il supporto agli emulatori è in beta.** Le installazioni portatili e le manie dei singoli emulatori lo colgono ancora in fallo, e oggi Aletheia e OpenSave coprono meglio certi casi limite di launcher ed emulatori.
- **macOS è provato pochissimo su hardware vero.** Compila e gira, ma nessuno ci ha vissuto per mesi.
- **È giovane.** Ludusavi e Game Backup Monitor hanno anni di segnalazioni alle spalle. Hoard no, e per qualcosa che custodisce una partita da 200 ore la differenza conta.
- **Non fa condivisione cooperativa.** Se vuoi passare un mondo a un amico, SaveSync è fatto per quello e Hoard no.

## La distinzione tra Hoard Cloud e self-hosting

I confronti su Hoard quasi sempre fondono i due in uno solo, e il risultato è sbagliato. Quindi, chiaramente:

- **Hoard Cloud** è l'opzione gestita: accedi e i tuoi salvataggi stanno sui nostri server, nell'UE.
- **Un Hoard self-hosted è interamente tuo.** Fai girare \`hoard-server\` sul tuo PC o NAS e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non vediamo un salvataggio, il nome di un gioco o un indirizzo email, perché niente di tutto ciò ci arriva. Se Hoard Cloud chiudesse domani, un'installazione self-hosted continuerebbe uguale.

Stesso binario, stesso rilevamento, stessa cronologia. L'unica cosa che cambia è di chi è lo spazio di archiviazione. E per essere esatti su un dettaglio: il tuo server ha eccome i suoi accessi — un utente e un token per dispositivo — ma vivono nel tuo database, non nel nostro.

## La tabella

| Strumento | Sincronizzazione automatica tra dispositivi | Dove vivono i salvataggi | Cronologia | Piattaforme | Licenza |
|---|---|---|---|---|---|
| **Hoard** | Sì, per sessione di gioco | Hoard Cloud o un tuo server (compatibile S3) | Versionata per sessione, deduplicata | Win · Linux · macOS · Deck | AGPL-3.0, piano gratuito |
| **Ludusavi** | Manuale, o Rclone montato da te | Locale, più il tuo remote Rclone | Copie locali versionate | Win · Linux · macOS | Gratis, open source |
| **Syncthing** | Sì, specchio continuo | Solo i tuoi dispositivi | Versionamento per file | Tutto | Gratis, open source |
| **OpenSave** | Sì, peer-to-peer | I tuoi dispositivi, replica cloud opzionale | Snapshot e branch | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Sì, tramite il tuo cloud | OneDrive / Drive / Dropbox / Nextcloud | Quello che tiene il cloud | Win · Linux · macOS | Gratis, open source |
| **Game Backup Monitor** | No | Archivi 7-Zip locali | Backup numerati | Windows | Gratis, open source |
| **GameSave Manager** | Tramite il tuo cloud (Sync & Link) | Locale, più Dropbox / OneDrive | Quello che tiene il cloud | Windows | Gratuito, closed source |
| **Aletheia** | Copia e ripristino per launcher | Il tuo spazio | Copie | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sì, e con gli amici | Voci private dello Steam Workshop | Secondo l'app | Windows | A pagamento, codice chiuso |
| **Tachyon** | Sì, tramite il proprio cloud | Il cloud di Tachyon | Versioni e timeline | Windows (beta) | Beta gratuita, codice non pubblicato |

## Quindi quale

Se vuoi una sola macchina messa al sicuro e nient'altro, prendi Ludusavi o Game Backup Monitor. Se non vuoi un account per nessun motivo e i tuoi dispositivi sono di solito accesi insieme, OpenSave. Se i salvataggi devono finire in una cartella di Drive che già paghi, OpenCloudSaves. Se condividi un mondo cooperativo con gli amici, SaveSync.

Se invece vuoi che copia *e* sincronizzazione tra PC e Steam Deck avvengano da sole, con una versione per sessione a cui tornare e la possibilità di ospitare tutto da te, è per questo che c'è Hoard. [Scaricalo](/download), o leggi prima [come ospitarlo da solo con Docker](/guides/self-host-hoard). C'è anche un [confronto approfondito con Ludusavi](/guides/ludusavi-alternative) se è quello che stai valutando.

## Confronti uno contro uno

Ognuno va più a fondo del blocco qui sopra, compresi i punti in cui vince l'altro strumento:

- [Hoard contro Ludusavi](/guides/ludusavi-alternative)
- [Hoard come alternativa a Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronizzazione peer-to-peer contro un server tuo](/guides/opensave-alternative)
- [Syncthing per i salvataggi: cosa si rompe](/guides/syncthing-game-saves)

<!-- faq -->

## Domande frequenti

### Quale di questi strumenti tiene una cronologia delle versioni?

Hoard conserva ogni sessione come una versione a cui tornare. Ludusavi tiene backup locali versionati. Quasi tutti gli altri sincronizzano o copiano lo stato attuale, quindi un salvataggio corrotto viene propagato fedelmente all'altra macchina.

### Quale funziona senza server né account?

Ludusavi con i backup locali, e qualsiasi strumento peer-to-peer. Ci rientra anche Hoard se fai self-hosting: nessun account con noi e niente che passi dai nostri server.

### Quale copre i giochi che non stanno su Steam?

Tutti i gestori di salvataggi elencati, perché individuano i file tramite lo stesso database comunitario e non attraverso un negozio. L'eccezione è Steam Cloud: copre solo i giochi Steam il cui sviluppatore l'ha attivata.

### Devo sceglierne uno solo?

No, e molti non lo fanno. Uno strumento di backup locale e uno di sincronizzazione risolvono metà diverse del problema. L'unica regola è non puntare mai uno alla cartella di backup dell'altro, o finisci per sincronizzare un mirror vecchio invece del salvataggio reale.

### Qual è il dettaglio che rompe quasi tutti i setup fai-da-te?

Sincronizzare la cartella sopra \`<AppID>/remote/\` dentro \`userdata\` di Steam. Quella superiore contiene \`remotecache.vdf\` e i file di obiettivi e tempo di gioco, che devono differire da macchina a macchina: ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso.
`,fa=`---
title: "セーブデータ同期ツール比較：Hoard と Ludusavi・Syncthing・OpenSave ほか"
description: "Hoard、Ludusavi、Syncthing、OpenSave、GameSave Managerなどを比較。それぞれの強みと弱点、並べて見られる比較表付き。"
order: 4
updated: 2026-10-01
---

Steam クラウドが守ってくれるのは Steam で買ったゲームだけ、しかも開発者が対応をオンにした場合に限られます。エミュレーター、GOG、Epic、itch.io、Steam 以外のゲーム、MOD を入れたもの——どれも対象外です。デスクトップと Steam Deck のように複数の環境で遊んでいると、結局フォルダーを手でコピーして、新しいほうを掴んだと信じるしかなくなります。

これを解決するツールはいくつもありますが、やっていることは同じではありません。ローカルにバックアップを取るもの、端末間でフォルダーをミラーするもの、クラウドへアップロードするもの。このページではそれぞれを見ていき、何が本当に得意なのかを書きます。Hoard は私のプロジェクトなので、正直な部分は最後に置きました。Hoard が負けている点の節と、本文を一切信じなくても読める比較表です。

## Ludusavi

いちばん有名で、それも当然の一本です。Ludusavi（作者は mtkennerly）は GUI と CLI を備えた無料のオープンソースのバックアップツールで、何万本ものゲームのセーブ位置を収録したコミュニティ製マニフェストの上に成り立っています。このページのほとんどのツール（Hoard も含む）が使っているのと同じマニフェストです。ローカルにバージョン付きのバックアップを保持し、Rclone を設定すれば自分のクラウドへ送れます。

**向いているのは：** ローカルのバックアップと完全な制御が欲しくて、サーバーはどこにも置きたくない人。このリストで最も安全な選択で、しかも無料です。

**足りないところ：** 端末間の同期は自分で組み立てるものになります。バックアップを予約し、Rclone のリモートを設定し、遊ぶ*前*に別の PC で復元するのを忘れない。動きはしますが、最後の一手を忘れるのを止めてくれるものは何もありません。

## Syncthing

そもそもゲーム用ではなく、汎用の P2P フォルダーミラーで、しかも良い出来です。セーブフォルダーを指定すれば、ほかの端末にも現れます。

**向いているのは：** すでに動かしていて、クラウドを挟まずにファイルを二か所に置きたい人。

**足りないところ：** ミラーであって、スナップショットではありません。壊れたセーブも、正常なセーブとまったく同じ速さで数秒のうちに全端末へ届きます。ファイル単位のバージョン管理はありますが、プレイセッションという概念はないので、「火曜の夜の状態に戻す」は手作業で組み直すことになります。両方の端末がオフラインで遊んでいれば、返ってくるのは競合ファイルであってマージではありません。

## OpenSave

セーブデータ専用に作られた P2P 同期。Go 製、MIT ライセンス、Windows・Linux・Steam Deck 対応です。アカウントもサーバーも不要で、端末同士をペアリングして LAN 経由、あるいはリレーのルームコード経由で同期します。変更のたびにスナップショットを取り、並行プレイ用のブランチがあり、競合は時計ではなく同期の系譜で解決し、転送は変化したブロックだけ。任意で Drive・Dropbox・OneDrive・WebDAV へのミラーもできます。

**向いているのは：** アカウントは絶対に作りたくなくて、端末が同時に起動している機会が十分にある人。

**足りないところ：** P2P である以上、セーブはあなたの端末の上にしか存在しません。最新のコピーを持っていた Deck が壊れ、ミラーを設定していなければそれで終わりです。同期には両方の端末が動いている必要があり、macOS 版はありません。

## OpenCloudSaves

すでに料金を払っているクラウド——OneDrive、Google Drive、Dropbox、Nextcloud——へセーブフォルダーを同期する、マルチプラットフォームの GUI です。中身は Rclone です。

**向いているのは：** すでに持っているストレージにセーブを置きたくて、Rclone の設定ファイルではなく画面で操作したい人。

**足りないところ：** 内容ベースの重複排除がありません。2 GB のセーブが 10 世代あれば Drive の容量を 20 GB 食いますし、クラウドドライブが同期するのはファイルであってプレイセッションではないので、戻ってくるのは「その時点のフォルダーの姿」だけです。

## Game Backup Monitor

Windows 中心で、このジャンルの元祖です。GBM はゲームのプロセスを見張り、終了した時点でセーブを 7-Zip で圧縮し、連番の履歴として残します。

**向いているのは：** Windows PC 一台で、何も考えずに圧縮済みのローカルアーカイブが欲しい人。

**足りないところ：** バックアップのツールであって同期のツールではありません。アーカイブを二台目に持っていくのは自分の仕事ですし、Steam Deck / SteamOS は得意分野ではありません。

## GameSave Manager

長く使われてきた無料の Windows 用ツールで、クローズドソース、独自のゲームデータベースを持っています。バックアップと復元ができ、最もよく知られた機能 **Sync & Link** は、セーブフォルダーを Dropbox や OneDrive などのクラウドフォルダーに移し、元の場所にリンクを残して、クラウドクライアントに同期させます。

**向いている人:** Windows を使っていて、すでに Dropbox や OneDrive 中心で暮らしており、セーブがそこにあればいい人。

**限界:** Windows のみ。また Sync & Link では、ゲームが書き込んでいる最中の使用中フォルダーを汎用クラウドクライアントが同期します。これは [Syncthing がセーブに危険な理由](/guides/syncthing-game-saves)と同じパターンです。バージョン履歴はクラウドドライブが保持する分だけです。

## Aletheia

この中では最も新しく、AGPL。ほかが中途半端にしか押さえていない部分、つまりランチャーを正面から狙っています。Heroic、itch.io、Lutris、Steam、GOG Galaxy、Xbox に、Windows・Linux・macOS 対応。

**向いているのは：** ライブラリが、ほかのツールでは検出しづらいランチャー——とくに Xbox / Game Pass と Heroic——に散らばっている人。

**足りないところ：** 意図的に範囲を絞った若いプロジェクトです。機能はバックアップと復元まで。背後にバージョン管理されたクラウドがあるわけではありません。

## SaveSync

唯一の商用で、Steam で買い切り販売、Windows 中心。特徴は、狙いが「二台の PC を使う自分」ではなく協力プレイにあることです。セーブは非公開・非掲載の Steam ワークショップの項目として保存され、友達があなたの Valheim や Factorio のワールドを持っていけます。LAN 同期もあります。

**向いているのは：** 解決したい問題が「自分のセーブについてきてほしい」ではなく「友達がホストで、その人のセーブが要る」である人。

**足りないところ：** クローズドソース、Windows、転送路として Steam に依存、そして対応するのは所有物すべてではなく協力プレイ向けの対応ゲーム一覧です。

## Tachyon

このリストで最も新しい名前で、Windows 向けの無料ベータ版です。5,000 本以上の PC ゲームのセーブを検出し、自社のクラウド経由で同期し、複数の「タイムライン」を持つバージョン履歴を保持します。

**向いている人:** Windows を使っていて、すぐに動くものがほしく、ベータ版でも気にならない人。

**限界:** 現時点では Windows のみ（Linux と macOS は「近日対応」とされています）なので Steam Deck はまだ使えません。ソースコードは公開されておらず、自分のサーバーで運用する方法もありません。ベータ終了後の価格は発表されていません。

## EmuDeck についての注記

この手の話題では EmuDeck も名前が挙がりますが、通常の意味での競合ではありません。Steam Deck 向けのエミュレーターのインストーラー兼設定ツールであり、備わっている同期はその仕事に付け足された利便機能です（クラウドドライブに対する Rclone、しかもエミュレーターのセーブ限定）。上のツール群と重なる部分はあっても、同じ種類のものではありません。EmuDeck はエミュレーター環境を整えるもの、ここで挙げたものはライブラリ全体のセーブを見守るもの。EmuDeck とどれか一つを併用している人もいて、それは重複ではなく理にかなった構成です。

## Hoard

Hoard はプレイセッションを単位として扱います。エンジンはバックグラウンドサービスとして動き（\`hoardd\`、ウィンドウを持たないので SteamOS のゲームモードでも動作します）、遊び終わったことを検知してからスナップショットを取ります。プレイ中のファイル書き込みに逐一反応するのではありません。

- **セッションごとのバージョン履歴。** どのセッションにも戻れます。ディスク故障のあとでも、クリーンインストールのあとでも。
- **内容ハッシュによる重複排除。** 2 GB のセーブが 10 世代あっても消費はおよそ 2 GB で、20 GB にはなりません。転送は zstd で圧縮されます。
- **アップロード時とダウンロード時の SHA-256。** 破損は、正常なセーブを上書きする前に検出されます。何も黙って上書きされない——設計の核はそこにあります。
- **クラウドでも自己ホストでも、同じバイナリ。** Hoard Cloud には無料プラン（2 GB、3 台、履歴は全部）があります。あるいは \`hoard-server\` を Docker Compose で自分で立て、S3 互換ストレージ（MinIO、Garage、Backblaze B2）に対して動かせば、アカウントも容量制限もありません。AGPL-3.0。
- **Windows・Linux・macOS**、加えて Steam Deck やサーバー向けのヘッドレス CLI。
- **エミュレーターはベータ：** PCSX2、RPCS3、Dolphin、Cemu、Ryujinx、RetroArch、DuckStation、PPSSPP ほかをプリセットで用意。

## Steam Deck ↔ PC の同期を左右する細部

どのツールを選ぶにしても知っておく価値があります。Steam のゲームのクラウドセーブは \`<AppID>/remote/\` にあり、その*一つ上*のフォルダーには \`remotecache.vdf\`、実績の状態、統計、プレイ時間のカウンターが入っています。これらは Deck とデスクトップとで違っていて当たり前のものです。

親フォルダーを同期すれば、セーブについては一度も食い違っていない二台の間で、恒久的な競合が起きます。Hoard が追いかけるのは \`remote/\` であって親フォルダーではありません。フォルダーを手動で指定できるツールなら同じ設定にできますし、同期の構成が理由もなく競合を出し続けるときに最初に確認すべき点でもあります。

## Hoard が負けている点

- **サーバーを欲しがる。** クラウドのアカウントか自前のマシンか、いずれにせよインフラです。OpenSave や Ludusavi はどちらも必要としません。
- **エミュレーター対応はベータ。** ポータブル構成や各エミュレーターの癖にまだ足をすくわれますし、ランチャーやエミュレーターの一部の特殊なケースは今日のところ Aletheia や OpenSave のほうがうまく扱えます。
- **macOS は実機での検証がほとんどない。** ビルドも起動もしますが、何か月も常用した人がいません。
- **歴史が浅い。** Ludusavi や Game Backup Monitor には何年分ものバグ報告が積み上がっています。Hoard にはそれがなく、200 時間のセーブを預かるものとしては軽くない差です。
- **協力プレイの共有はできない。** 友達にワールドを渡したいなら、それは SaveSync のための仕事で、Hoard の仕事ではありません。

## Hoard Cloud とセルフホストの違い

Hoard についての比較は、ほぼ必ずこの 2 つを一緒くたにし、その結果として誤った説明になります。はっきり書いておきます。

- **Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに保存されます。
- **セルフホストした Hoard は完全にあなたのものです。** 自分の PC や NAS で \`hoard-server\` を動かせば、セーブは自分のマシンから自分のディスクへ移ります。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。セーブもゲーム名もメールアドレスも見えません。届かないからです。仮に明日 Hoard Cloud が終了しても、セルフホスト構成はそのまま動き続けます。

同じバイナリ、同じ検出、同じ世代履歴。変わるのは保存先が誰のものかだけです。正確を期して 1 点だけ補うと、あなたのサーバーには確かに自前のログイン、つまりユーザーと端末ごとのトークンがありますが、それらはあなたのデータベースの中にあり、当方のデータベースにはありません。

## 比較表

| ツール | 端末間の自動同期 | セーブの置き場所 | 履歴 | 対応環境 | ライセンス |
|---|---|---|---|---|---|
| **Hoard** | あり（プレイセッション単位） | Hoard Cloud または自前サーバー（S3 互換） | セッション単位のバージョン、重複排除あり | Win · Linux · macOS · Deck | AGPL-3.0、無料プランあり |
| **Ludusavi** | 手動、または自分で組む Rclone | ローカル＋自分の Rclone リモート | バージョン付きローカルバックアップ | Win · Linux · macOS | 無料・オープンソース |
| **Syncthing** | あり（常時ミラー） | 自分の端末のみ | ファイル単位のバージョン | すべて | 無料・オープンソース |
| **OpenSave** | あり（P2P） | 自分の端末、任意でクラウドミラー | スナップショットとブランチ | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | あり（自分のクラウド経由） | OneDrive / Drive / Dropbox / Nextcloud | クラウド側が保持する範囲 | Win · Linux · macOS | 無料・オープンソース |
| **Game Backup Monitor** | なし | ローカルの 7-Zip アーカイブ | 連番バックアップ | Windows | 無料・オープンソース |
| **GameSave Manager** | クラウドドライブ経由（Sync & Link） | ローカル＋Dropbox / OneDrive | ドライブが保持する分 | Windows | 無料・クローズドソース |
| **Aletheia** | ランチャーごとのバックアップと復元 | 自分のストレージ | バックアップ | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | あり（友達とも） | 非公開の Steam ワークショップ項目 | アプリの仕様による | Windows | 有料・クローズドソース |
| **Tachyon** | あり（自社クラウド経由） | Tachyon のクラウド | バージョンとタイムライン | Windows（ベータ） | 無料ベータ・ソース非公開 |

## で、どれを選ぶか

一台だけ守れれば十分なら Ludusavi か Game Backup Monitor。アカウントだけは何があっても作りたくなくて、端末がだいたい同時に起動しているなら OpenSave。すでに料金を払っている Drive のフォルダーにセーブを置きたいなら OpenCloudSaves。友達と協力プレイのワールドを共有したいなら SaveSync。

バックアップ*と*、PC と Steam Deck をまたぐ同期が勝手に行われること、セッションごとに戻れるバージョンがあること、そして全部を自分でホストできる選択肢があること——それを求めるなら Hoard です。[ダウンロード](/download)するか、先に[Docker で自己ホストする方法](/guides/self-host-hoard)を読んでみてください。天秤にかけている相手が Ludusavi なら、[詳しい比較](/guides/ludusavi-alternative)もあります。

## 一対一の比較

以下はそれぞれ、上の節より踏み込んで扱っています。相手のほうが優れている点も含みます。

- [Hoard と Ludusavi](/guides/ludusavi-alternative)
- [Steam クラウドの代替としての Hoard](/guides/steam-cloud-alternative)
- [ピアツーピア同期と自分のサーバー](/guides/opensave-alternative)
- [Syncthing でセーブを同期すると何が壊れるか](/guides/syncthing-game-saves)

<!-- faq -->

## よくある質問

### この中で世代履歴を残すのはどれですか？

Hoard は 1 セッションを 1 世代として残し、そこへ戻れます。Ludusavi は世代管理されたローカルバックアップを保持します。その他の多くは現在の状態を同期・コピーするだけなので、壊れたセーブはそのまま忠実にもう 1 台へ伝わります。

### サーバーもアカウントもなしで使えるのはどれですか？

ローカルバックアップとしての Ludusavi と、ピアツーピアのツール全般です。セルフホストするなら Hoard も該当します。当方のアカウントはなく、当方のサーバーを通るものもありません。

### Steam にないゲームをカバーするのはどれですか？

ここに挙げたセーブ管理ツールはすべてカバーします。ストア経由ではなく、同じコミュニティのデータベースでセーブの場所を突き止めるからです。例外は Steam クラウドで、開発者が有効にした Steam のゲームしか対象になりません。

### 1 つだけ選ばないといけませんか？

いいえ。実際、多くの人は選んでいません。ローカルバックアップのツールと同期のツールは、問題の別々の半分を解いています。唯一の注意は、一方をもう一方のバックアップフォルダーに向けないこと。向けると、実際のセーブではなく古い写しを同期することになります。

### 自作構成がいちばん壊れる原因は何ですか？

Steam の \`userdata\` にある \`<AppID>/remote/\` の 1 つ上のフォルダーを同期することです。上のフォルダーには \`remotecache.vdf\` や、マシンごとに違って当然の実績・プレイ時間のファイルが入っているため、セーブが動いていなくても起動のたびに競合に見えます。
`,ba=`---
title: "Sincronização de saves comparada: Hoard frente a Ludusavi, Syncthing, OpenSave e as outras"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager e mais, comparados: onde cada um brilha, onde fica aquém e uma tabela lado a lado."
order: 4
updated: 2026-10-01
---

A Steam Cloud só cobre jogos comprados na Steam, e apenas quando o programador se deu ao trabalho de a ligar. Emuladores, GOG, Epic, itch.io, jogos fora da Steam, qualquer coisa com mods: nada disso entra. Se jogas em mais do que uma máquina, um desktop e uma Steam Deck por exemplo, acabas a copiar pastas à mão na esperança de teres apanhado a mais recente.

Há várias ferramentas que resolvem isto, e não fazem todas o mesmo. Umas fazem cópias locais, outras espelham pastas entre dispositivos, outras enviam para uma nuvem. Esta página passa por elas e diz em que é que cada uma é genuinamente boa. O Hoard é o meu projeto, por isso a parte honesta fica no fim: uma secção sobre onde o Hoard perde, e uma tabela que podes ler sem acreditar numa única linha do texto.

## Ludusavi

O mais conhecido, e com razão. O Ludusavi (de mtkennerly) é uma ferramenta de backup gratuita e open source, com interface e com CLI, construída sobre o manifesto comunitário de localizações de saves que cobre dezenas de milhares de jogos — o mesmo manifesto que quase todas as desta lista usam, o Hoard incluído. Guarda cópias locais versionadas e pode enviá-las para uma nuvem tua através do Rclone.

**Melhor se:** queres cópias locais, controlo total e nenhum servidor em lado nenhum. É a escolha mais segura da lista e não custa nada.

**Onde para:** a sincronização entre máquinas é algo que montas tu. Agendar um backup, configurar um remote do Rclone e lembrares-te de restaurar no outro PC *antes* de jogar. Funciona, mas nada te impede de esquecer o último passo.

## Syncthing

Não é sequer uma ferramenta de jogos: é um espelho de pastas peer-to-peer de uso geral, e muito bom. Apontas-lhe uma pasta de saves e ela aparece nos teus outros dispositivos.

**Melhor se:** já o tens a correr e queres os ficheiros em dois sítios sem nuvem pelo meio.

**Onde para:** espelha, não fotografa. Um save corrompido chega a todos os dispositivos em segundos, exatamente à mesma velocidade de um bom. O versionamento é por ficheiro, sem ideia nenhuma do que é uma sessão de jogo, por isso «voltar a como estava na terça à noite» é algo que reconstróis à mão. Duas máquinas que jogaram offline dão-te ficheiros de conflito, não uma fusão.

## OpenSave

Sincronização peer-to-peer feita de propósito para saves, em Go, com licença MIT, para Windows, Linux e Steam Deck. Sem conta e sem servidor: os dispositivos emparelham entre si e sincronizam pela rede local ou através de um código de sala num relay. Cada alteração vira um snapshot, há branches para partidas paralelas, os conflitos resolvem-se por linhagem de sincronização em vez de pelo relógio, e só viajam os blocos que mudaram. Opcionalmente pode espelhar para Drive, Dropbox, OneDrive ou WebDAV.

**Melhor se:** recusas ter uma conta e os teus dispositivos estão ligados ao mesmo tempo com frequência suficiente.

**Onde para:** peer-to-peer significa que o save só vive nos teus dispositivos. Se morre a Deck com a única cópia recente e o espelho nunca foi configurado, acabou. Os dois dispositivos têm de estar ligados para haver sincronização, e não há versão para macOS.

## OpenCloudSaves

Uma interface multiplataforma que sincroniza as tuas pastas de saves para uma nuvem que já pagas — OneDrive, Google Drive, Dropbox, Nextcloud — com o Rclone por baixo.

**Melhor se:** queres os saves numa conta de armazenamento que já tens, com interface em vez de ficheiros de configuração do Rclone.

**Onde para:** não há desduplicação ao nível do conteúdo. Dez cópias de um save de 2 GB são 20 GB da tua quota do Drive, e as nuvens de ficheiros sincronizam ficheiros, não sessões de jogo, por isso o que recuperas é como a pasta estava naquele momento.

## Game Backup Monitor

Windows primeiro, e o original de todo este género. O GBM vigia o processo do jogo e, quando sais, comprime o save com 7-Zip e guarda um histórico numerado.

**Melhor se:** estás num único PC com Windows e queres um arquivo local comprimido sem pensar nisso.

**Onde para:** é uma ferramenta de backup, não de sincronização. Levar o arquivo para uma segunda máquina é problema teu, e a Steam Deck / SteamOS não é o seu terreno.

## GameSave Manager

Uma ferramenta gratuita e veterana para Windows, de código fechado, com a sua própria base de dados de jogos. Faz backup e restauro, e a sua função mais conhecida, **Sync & Link**, move a pasta de saves para uma pasta na nuvem como o Dropbox ou o OneDrive e deixa um link no lugar, para que o cliente da nuvem a mantenha sincronizada.

**Melhor se:** estás no Windows, já vives no Dropbox ou no OneDrive e queres simplesmente que o save esteja lá.

**Onde para:** só Windows, e o Sync & Link significa que um cliente de nuvem genérico sincroniza a pasta ativa enquanto o jogo escreve nela, o mesmo padrão que torna [o Syncthing arriscado para saves](/guides/syncthing-game-saves). O histórico de versões é o que a tua nuvem guardar.

## Aletheia

O mais recente do grupo, AGPL, e vai exatamente à parte que os outros cobrem pela metade: os launchers. Heroic, itch.io, Lutris, Steam, GOG Galaxy e Xbox, em Windows, Linux e macOS.

**Melhor se:** a tua biblioteca está espalhada por launchers que as outras ferramentas detetam mal, sobretudo Xbox/Game Pass e Heroic.

**Onde para:** é um projeto jovem com um âmbito propositadamente estreito. Fazer cópia e restaurar é todo o conjunto de funcionalidades; não há uma nuvem versionada por trás.

## SaveSync

O comercial, vendido na Steam como compra única, virado para Windows. O truque dele é que não aponta a ti-em-dois-PC, mas ao cooperativo: os saves vão para entradas privadas e não listadas da Steam Workshop para que um amigo possa puxar o teu mundo de Valheim ou de Factorio, e também há sincronização por rede local.

**Melhor se:** o problema que resolves é «o meu amigo aloja e preciso do save dele», não «que os meus saves me sigam».

**Onde para:** código fechado, Windows, preso à Steam como meio de transporte, e uma lista de jogos cooperativos suportados em vez de tudo o que tens.

## Tachyon

O nome mais recente da lista, em beta gratuita para Windows. O Tachyon deteta os saves de mais de 5 000 jogos de PC, sincroniza-os através da sua própria nuvem e guarda um histórico de versões com várias «linhas temporais».

**Melhor se:** estás no Windows, queres algo que funcione logo à primeira e não te importas que seja uma beta.

**Onde para:** por agora só Windows (Linux e macOS aparecem como «em breve»), por isso ainda sem Steam Deck; sem código-fonte publicado; sem forma de usar o teu próprio servidor. O preço depois da beta não foi anunciado.

## Uma nota sobre o EmuDeck

O EmuDeck aparece nestas conversas e não é um concorrente no sentido normal: é um instalador e configurador de emuladores para a Steam Deck, e a sincronização que oferece é uma comodidade acoplada a esse trabalho (Rclone contra uma nuvem de ficheiros, só para saves de emulador). Sobrepõe-se às ferramentas acima sem ser a mesma coisa: o EmuDeck deixa-te os emuladores montados, e as daqui tomam conta dos saves da biblioteca toda. Há quem use o EmuDeck ao lado de uma destas, e é uma montagem sensata, não redundante.

## Hoard

O Hoard toma a sessão de jogo como unidade. O motor corre como serviço em segundo plano — \`hoardd\`, sem janela, por isso funciona no modo de jogo do SteamOS —, dá-se conta de que paraste de jogar e faz o snapshot nessa altura, em vez de reagir a cada escrita de ficheiro a meio da partida.

- **Histórico versionado por sessão.** Cada sessão é uma versão à qual podes voltar, mesmo depois de uma falha de disco ou de uma instalação limpa.
- **Desduplicação por hash de conteúdo.** Dez versões de um save de 2 GB custam cerca de 2 GB, não 20 GB. As transferências vão comprimidas com zstd.
- **SHA-256 à subida e à descida.** A corrupção é apanhada antes de poder sobrescrever um save bom. Nada é sobrescrito em silêncio: é esse o desenho todo.
- **Nuvem ou auto-alojado, o mesmo binário.** O Hoard Cloud tem plano gratuito (2 GB, 3 dispositivos, histórico completo). Ou levantas o \`hoard-server\` tu mesmo com Docker Compose contra qualquer armazenamento compatível com S3 — MinIO, Garage, Backblaze B2 — sem conta e sem quota. AGPL-3.0.
- **Windows, Linux, macOS**, mais uma CLI sem interface para uma Steam Deck ou um servidor.
- **Emuladores em beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP e outros como predefinições.

## O detalhe que decide a sincronização Steam Deck ↔ PC

Vale a pena saber, escolhas a ferramenta que escolheres. O save na nuvem de um jogo da Steam vive em \`<AppID>/remote/\`, e a pasta *acima* guarda o \`remotecache.vdf\`, o estado das conquistas, estatísticas e contadores de horas jogadas — coisas que legitimamente diferem entre a tua Deck e o teu desktop.

Sincroniza a pasta-mãe e ficas com um conflito permanente entre duas máquinas que nunca discordaram sobre um único save. O Hoard segue \`remote/\`, não a pasta-mãe. A qualquer ferramenta a que apontes uma pasta à mão pode dizer-se o mesmo, e é a primeira coisa a verificar quando uma configuração de sincronização assinala conflitos sem motivo visível.

## Onde o Hoard perde

- **Quer um servidor.** Conta na nuvem ou máquina tua, de qualquer forma é infraestrutura, e o OpenSave ou o Ludusavi não precisam de nenhuma.
- **O suporte a emuladores está em beta.** As instalações portáteis e as manias de cada emulador ainda o apanham, e hoje o Aletheia e o OpenSave cobrem melhor alguns casos limite de launchers e emuladores.
- **O macOS está mal testado em hardware real.** Compila e funciona, mas ninguém viveu lá durante meses.
- **É jovem.** O Ludusavi e o Game Backup Monitor têm anos de relatos de bugs atrás deles. O Hoard não, e isso pesa em algo que guarda um save de 200 horas.
- **Não faz partilha cooperativa.** Se queres passar um mundo a um amigo, o SaveSync foi feito para isso e o Hoard não.

## A distinção entre Hoard Cloud e self-hosting

As comparações sobre o Hoard quase sempre fundem os dois num só, e o resultado sai errado. Por isso, de forma clara:

- **O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.
- **Um Hoard self-hosted é inteiramente teu.** Corres o \`hoard-server\` no teu PC ou NAS e os saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não vemos um save, o nome de um jogo ou um email, porque nada disso nos chega. Se o Hoard Cloud fechasse amanhã, uma instalação self-hosted continuaria igual.

O mesmo binário, a mesma deteção, o mesmo histórico. A única coisa que muda é de quem é o armazenamento. E, para ser exato num detalhe: o teu servidor tem sim os seus próprios acessos — um utilizador e um token por dispositivo — mas vivem na tua base de dados, não na nossa.

## A tabela

| Ferramenta | Sincronização automática entre dispositivos | Onde vivem os saves | Histórico | Plataformas | Licença |
|---|---|---|---|---|---|
| **Hoard** | Sim, por sessão de jogo | Hoard Cloud ou servidor teu (compatível com S3) | Versionado por sessão, desduplicado | Win · Linux · macOS · Deck | AGPL-3.0, plano gratuito |
| **Ludusavi** | Manual, ou Rclone montado por ti | Local, mais o teu remote do Rclone | Cópias locais versionadas | Win · Linux · macOS | Grátis, open source |
| **Syncthing** | Sim, espelho contínuo | Só os teus dispositivos | Versionamento por ficheiro | Tudo | Grátis, open source |
| **OpenSave** | Sim, peer-to-peer | Os teus dispositivos, espelho opcional na nuvem | Snapshots e branches | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Sim, através da tua nuvem | OneDrive / Drive / Dropbox / Nextcloud | O que a nuvem guardar | Win · Linux · macOS | Grátis, open source |
| **Game Backup Monitor** | Não | Arquivos 7-Zip locais | Cópias numeradas | Windows | Grátis, open source |
| **GameSave Manager** | Através da tua nuvem (Sync & Link) | Local, mais Dropbox / OneDrive | O que a nuvem guardar | Windows | Grátis, código fechado |
| **Aletheia** | Cópia e restauro por launcher | O teu armazenamento | Cópias | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sim, e com amigos | Entradas privadas da Steam Workshop | Conforme a app | Windows | Pago, código fechado |
| **Tachyon** | Sim, através da sua nuvem | A nuvem do Tachyon | Versões e linhas temporais | Windows (beta) | Beta gratuita, sem código publicado |

## Então qual

Se queres uma máquina protegida e mais nada, leva o Ludusavi ou o Game Backup Monitor. Se não queres uma conta em circunstância alguma e os teus dispositivos costumam estar ligados ao mesmo tempo, o OpenSave. Se os saves devem ir parar a uma pasta do Drive que já pagas, o OpenCloudSaves. Se partilhas um mundo cooperativo com amigos, o SaveSync.

Se o que queres é que a cópia *e* a sincronização entre PC e Steam Deck aconteçam sozinhas, com uma versão por sessão à qual voltar e a opção de alojar tudo tu, é para isso que serve o Hoard. [Descarrega-o](/download), ou lê primeiro [como alojá-lo com Docker](/guides/self-host-hoard). Há também uma [comparação longa com o Ludusavi](/guides/ludusavi-alternative) se for essa a que estás a pesar.

## Comparações um para um

Cada uma vai mais fundo do que o bloco acima, incluindo onde a outra ferramenta ganha:

- [Hoard frente ao Ludusavi](/guides/ludusavi-alternative)
- [Hoard como alternativa à Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronização ponto a ponto frente a um servidor teu](/guides/opensave-alternative)
- [Syncthing para saves: o que parte](/guides/syncthing-game-saves)

<!-- faq -->

## Perguntas frequentes

### Qual destas ferramentas guarda histórico de versões?

O Hoard conserva cada sessão como uma versão à qual podes voltar. O Ludusavi guarda cópias locais versionadas. A maioria das restantes sincroniza ou copia o estado atual, o que significa que um save corrompido é propagado fielmente para a outra máquina.

### Qual funciona sem servidor nem conta?

O Ludusavi com cópias locais, e qualquer ferramenta ponto a ponto. O Hoard também entra se fizeres self-hosting: sem conta connosco e sem nada a passar pelos nossos servidores.

### Qual cobre jogos que não estão na Steam?

Todos os gestores de saves aqui listados, porque localizam os ficheiros pela mesma base de dados comunitária e não através de uma loja. A exceção é a Steam Cloud: só cobre jogos da Steam cujo programador a ativou.

### Tenho de escolher só uma?

Não, e muita gente não escolhe. Uma ferramenta de cópia local e uma de sincronização resolvem metades diferentes do problema. A única regra é nunca apontar uma para a pasta de cópias da outra, ou acabas a sincronizar um espelho desatualizado em vez do teu save real.

### Qual é o detalhe que parte a maioria das montagens caseiras?

Sincronizar a pasta acima de \`<AppID>/remote/\` no \`userdata\` da Steam. A de cima guarda \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo que devem ser diferentes em cada máquina, por isso cada arranque parece um conflito mesmo sem nenhum save se ter mexido.
`,ya=`---
title: "游戏存档同步工具对比：Hoard 与 Ludusavi、Syncthing、OpenSave 等"
description: "Hoard、Ludusavi、Syncthing、OpenSave、GameSave Manager 等工具对比：各自的长处与短板，附并排对比表。"
order: 4
updated: 2026-10-01
---

Steam 云存档只覆盖你在 Steam 上买的游戏，而且还得开发者愿意打开这个开关。模拟器、GOG、Epic、itch.io、非 Steam 的游戏、任何装了 MOD 的东西，统统不在其中。如果你在不止一台机器上玩——比如一台台式机加一台 Steam Deck——最后就是手动复制文件夹，然后祈祷自己拿的是最新的那一份。

有好几款工具在解决这件事，而它们做的并不是同一件事。有的做本地备份，有的在设备之间镜像文件夹，有的上传到云端。这一页把它们逐个过一遍，说清楚每一款真正擅长什么。Hoard 是我自己的项目，所以诚实的部分放在最后：一节讲 Hoard 输在哪里，再加一张即使你一个字都不信正文也能读的表。

## Ludusavi

最有名的一款，而且名副其实。Ludusavi（作者 mtkennerly）是一款免费开源的备份工具，有图形界面也有命令行，建立在收录了数万款游戏存档位置的社区清单之上——本页几乎所有工具用的都是同一份清单，Hoard 也不例外。它保留带版本的本地备份，并且可以通过配置 Rclone 推送到你自己的云。

**适合：** 想要本地备份、完全掌控，并且任何地方都不想有服务器的人。它是这份名单里最稳妥的选择，而且一分钱不花。

**止步之处：** 跨机器同步是要你自己拼出来的。安排备份、配置 Rclone 远端，然后记得在开玩*之前*到另一台电脑上还原。它确实可行，但没有任何东西会阻止你忘掉最后一步。

## Syncthing

它根本不是游戏工具，而是一个通用的点对点文件夹镜像，而且做得很好。把存档文件夹指给它，它就会出现在你其他设备上。

**适合：** 你本来就在用它，并且希望文件同时存在两处、中间不经过任何云。

**止步之处：** 它做的是镜像，不是快照。一个损坏的存档会在几秒内到达每一台设备，速度和一个完好的存档一模一样。它的版本保留是按文件的，完全不知道"一局游戏"是什么概念，所以"回到周二晚上的样子"得靠你手工拼回来。两台机器都离线玩过之后，你拿到的是冲突文件，不是合并结果。

## OpenSave

专为存档而做的点对点同步，用 Go 写成，MIT 许可，支持 Windows、Linux 和 Steam Deck。不需要账号也不需要服务器：设备之间互相配对，通过局域网或中继的房间码同步。每次改动都会生成快照，有分支可以放平行的存档进度，冲突按同步谱系而不是按时钟解决，传输只走变化的数据块。也可以选择镜像到 Drive、Dropbox、OneDrive 或 WebDAV。

**适合：** 说什么都不肯注册账号，而且设备同时开机的机会足够多的人。

**止步之处：** 点对点意味着存档只活在你自己的设备上。如果那台存着唯一一份最新存档的 Deck 坏了，而镜像又从来没配置过，那就到此为止。要同步，两台设备都得开着；另外没有 macOS 版本。

## OpenCloudSaves

一个跨平台的图形界面，把你的存档文件夹同步到你已经在付费的云上——OneDrive、Google Drive、Dropbox、Nextcloud——底层用的是 Rclone。

**适合：** 想把存档放进已经拥有的存储空间，并且宁可点界面也不想写 Rclone 配置文件的人。

**止步之处：** 没有基于内容的去重。一个 2 GB 存档保十份，就是吃掉你 Drive 配额的 20 GB；而且网盘同步的是文件而不是游戏会话，你取回来的只是文件夹当时的样子。

## Game Backup Monitor

以 Windows 为主，也是整个门类的鼻祖。GBM 盯着游戏进程，等你退出时用 7-Zip 压缩存档，并保留一份编号的历史。

**适合：** 只有一台 Windows 电脑，想要一份压缩好的本地归档、完全不用动脑的人。

**止步之处：** 它是备份工具，不是同步工具。把归档弄到第二台机器上是你自己的事，而 Steam Deck / SteamOS 也不是它的主场。

## GameSave Manager

一款历史悠久的免费 Windows 工具，闭源，自带游戏数据库。它能备份和恢复，最知名的功能 **Sync & Link** 会把存档文件夹移进 Dropbox 或 OneDrive 等云盘文件夹，并在原处留下链接，由云盘客户端保持同步。

**最适合：**你用 Windows，本来就离不开 Dropbox 或 OneDrive，只想让存档放在那里。

**局限：**仅限 Windows；而且 Sync & Link 意味着通用云盘客户端会在游戏写入时同步正在使用的文件夹，这正是[用 Syncthing 同步存档有风险](/guides/syncthing-game-saves)的同一种模式。版本历史取决于你的云盘保留多少。

## Aletheia

这一组里最新的一款，AGPL 许可，而且专攻别人都只覆盖了一半的那块：启动器。Heroic、itch.io、Lutris、Steam、GOG Galaxy 和 Xbox，覆盖 Windows、Linux 和 macOS。

**适合：** 游戏库散落在其他工具识别得不好的启动器上，尤其是 Xbox / Game Pass 和 Heroic。

**止步之处：** 这是一个年轻的项目，范围也是刻意收窄的。功能就是备份和还原，背后并没有一个带版本的云。

## SaveSync

唯一的商业产品，在 Steam 上买断制出售，以 Windows 为主。它的巧思在于：它瞄准的并不是"你和你的两台电脑"，而是联机合作。存档会存进私有且不公开列出的 Steam 创意工坊条目，好让朋友把你的《Valheim》或《Factorio》世界拉走；另外也有局域网同步。

**适合：** 你要解决的问题是"朋友开房，我需要他那份存档"，而不是"让我的存档跟着我走"。

**止步之处：** 闭源、限 Windows、把 Steam 当作传输通道，而且支持的是一份联机游戏清单，不是你拥有的一切。

## Tachyon

这份名单里最新的名字，目前在 Windows 上免费公测。Tachyon 能识别 5,000 多款 PC 游戏的存档，通过自家的云同步，并保留带有多条“时间线”的版本历史。

**最适合：**你用 Windows，想要开箱即用，也不介意它还是测试版。

**局限：**目前仅限 Windows（Linux 和 macOS 标注为即将推出），所以还不支持 Steam Deck；没有公开源代码；也无法使用自己的服务器。测试结束后的定价尚未公布。

## 关于 EmuDeck 的一点说明

这类讨论里常常会提到 EmuDeck，但它并不是通常意义上的竞品：它是 Steam Deck 上的模拟器安装与配置工具，所提供的同步只是附在这份工作上的便利功能（用 Rclone 对接网盘，而且仅限模拟器存档）。它和上面这些工具有重叠，却不是同一类东西：EmuDeck 负责把模拟器给你装好配好，这里的工具负责照看整个游戏库的存档。确实有人把 EmuDeck 和其中一款搭配着用，那是合理的组合，并不重复。

## Hoard

Hoard 以一次游戏会话作为单位。引擎作为后台服务运行——\`hoardd\`，没有窗口，所以在 SteamOS 的游戏模式下照样工作——它会察觉你已经不玩了，然后在那一刻拍下快照，而不是在游戏进行中对每一次文件写入做出反应。

- **按会话的版本历史。** 每一次会话都是一个可以回退到的版本，哪怕是在硬盘故障或者全新安装之后。
- **基于内容哈希的去重。** 一个 2 GB 存档的十个版本大约只占 2 GB，而不是 20 GB。传输使用 zstd 压缩。
- **上传和下载都做 SHA-256 校验。** 损坏会在覆盖掉一个完好存档之前被抓出来。任何东西都不会被悄悄覆盖——整个设计就是围着这一点转的。
- **云端或自托管，同一个二进制。** Hoard Cloud 有免费额度（2 GB、3 台设备、完整历史）。或者你用 Docker Compose 自己跑 \`hoard-server\`，对接任何兼容 S3 的存储——MinIO、Garage、Backblaze B2——不需要账号，也没有配额。AGPL-3.0。
- **Windows、Linux、macOS**，另外还有一个无界面的命令行版本，适合 Steam Deck 或服务器。
- **模拟器支持处于测试阶段：** PCSX2、RPCS3、Dolphin、Cemu、Ryujinx、RetroArch、DuckStation、PPSSPP 等以预设形式提供。

## 决定 Steam Deck ↔ PC 同步成败的那个细节

不管你最后选哪款工具，这一点都值得知道。Steam 游戏的云存档位于 \`<AppID>/remote/\`，而它*上一层*的文件夹里放着 \`remotecache.vdf\`、成就状态、统计数据和游戏时长计数器——这些东西在你的 Deck 和台式机上本来就应该不一样。

同步父文件夹，你就会在两台从未在任何一个存档上产生分歧的机器之间，制造出永久的冲突。Hoard 跟踪的是 \`remote/\`，不是父文件夹。任何允许你手动指定文件夹的工具都可以照此设置；当一套同步配置莫名其妙地不断报冲突时，这也是第一个该去检查的地方。

## Hoard 输在哪里

- **它需要一台服务器。** 云端账号也好，自己的机器也罢，总归是基础设施；而 OpenSave 或 Ludusavi 一台都不需要。
- **模拟器支持还在测试阶段。** 便携式安装和各家模拟器的怪癖仍然会绊到它，某些启动器和模拟器的边缘情况，今天 Aletheia 和 OpenSave 处理得更好。
- **macOS 几乎没在真机上验证过。** 能编译也能跑，但没有人在上面长期用过几个月。
- **它还年轻。** Ludusavi 和 Game Backup Monitor 背后有好几年的问题反馈积累，Hoard 没有；对于一个要守着 200 小时存档的软件来说，这个差距不轻。
- **它不做联机存档共享。** 想把一个世界递给朋友，那是 SaveSync 的活儿，不是 Hoard 的。

## Hoard Cloud 与自托管的区别

关于 Hoard 的比较几乎总把这两者混为一谈，结论也就跟着错了。所以直说：

- **Hoard Cloud** 是托管方案：你登录，存档保存在我们位于欧盟的服务器上。
- **自托管的 Hoard 完全属于你。** 你在自己的 PC 或 NAS 上运行 \`hoard-server\`，存档从你的机器走到你的磁盘。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。我们看不到任何存档、游戏名或邮箱地址，因为这些从未到达我们这里。就算 Hoard Cloud 明天关停，自托管的部署照常运行。

同一个二进制、同样的检测、同样的版本历史。唯一变化的是存储归谁所有。有一点要说准确：你自己的服务器确实有它的登录——一个用户和每台设备一个令牌——但它们在你的数据库里，不在我们的。

## 对比表

| 工具 | 设备之间自动同步 | 存档放在哪里 | 历史 | 平台 | 许可 |
|---|---|---|---|---|---|
| **Hoard** | 是，按游戏会话 | Hoard Cloud 或你自己的服务器（兼容 S3） | 按会话版本化，带去重 | Win · Linux · macOS · Deck | AGPL-3.0，有免费额度 |
| **Ludusavi** | 手动，或你自己搭的 Rclone | 本地，外加你的 Rclone 远端 | 带版本的本地备份 | Win · Linux · macOS | 免费开源 |
| **Syncthing** | 是，持续镜像 | 只在你的设备上 | 按文件的版本保留 | 全平台 | 免费开源 |
| **OpenSave** | 是，点对点 | 你的设备，可选云端镜像 | 快照与分支 | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | 是，经由你的网盘 | OneDrive / Drive / Dropbox / Nextcloud | 取决于网盘保留什么 | Win · Linux · macOS | 免费开源 |
| **Game Backup Monitor** | 否 | 本地 7-Zip 归档 | 编号备份 | Windows | 免费开源 |
| **GameSave Manager** | 通过你的云盘（Sync & Link） | 本地，外加 Dropbox / OneDrive | 取决于云盘保留多少 | Windows | 免费，闭源 |
| **Aletheia** | 按启动器备份与还原 | 你自己的存储 | 备份 | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | 是，还能和朋友同步 | 私有的 Steam 创意工坊条目 | 视应用而定 | Windows | 付费闭源 |
| **Tachyon** | 有，通过自家云 | Tachyon 的云 | 版本和时间线 | Windows（测试版） | 免费测试版，未公开源代码 |

## 那么选哪个

如果你只想保住一台机器，别的都不管，选 Ludusavi 或 Game Backup Monitor。如果你无论如何都不想要账号，而且设备通常同时开着，选 OpenSave。如果存档应该落进一个你已经在付费的 Drive 文件夹里，选 OpenCloudSaves。如果你要和朋友共享一个联机世界，选 SaveSync。

如果你想要的是备份*和*跨 PC 与 Steam Deck 的同步自己就发生，每一次会话都有一个可以回退的版本，并且保留把整套东西自托管的选项——那正是 Hoard 存在的理由。[下载它](/download)，或者先读一读[如何用 Docker 自托管](/guides/self-host-hoard)。如果你正在权衡的对手就是 Ludusavi，这里还有一篇[更详细的对比](/guides/ludusavi-alternative)。

## 一对一比较

下面每一篇都比上面的段落更深入，也包括对方胜出的地方：

- [Hoard 与 Ludusavi](/guides/ludusavi-alternative)
- [用 Hoard 替代 Steam 云存档](/guides/steam-cloud-alternative)
- [点对点同步与一台属于你的服务器](/guides/opensave-alternative)
- [用 Syncthing 同步存档会坏在哪里](/guides/syncthing-game-saves)

<!-- faq -->

## 常见问题

### 这些工具里，哪个保留版本历史？

Hoard 把每次游玩留成一个可回退的版本。Ludusavi 保留带版本的本地备份。其余大多数只是同步或复制当前状态，也就是说损坏的存档会被忠实地传到你的另一台机器上。

### 哪个不需要服务器也不需要账号？

用作本地备份的 Ludusavi，以及任何点对点工具。如果你自托管，Hoard 也算在内：没有我们这边的账号，也没有任何东西经过我们的服务器。

### 哪个能覆盖不在 Steam 上的游戏？

这里列出的存档管理工具都可以，因为它们通过同一份社区数据库定位存档，而不是通过商店。例外是 Steam 云存档：它只覆盖开发者启用了它的 Steam 游戏。

### 我必须只选一个吗？

不必，很多人也没有只选一个。本地备份工具和同步工具解决的是问题的不同一半。唯一的原则是：永远不要让其中一个指向另一个的备份文件夹，否则你同步的会是一份过期镜像，而不是真正的存档。

### 让多数自制方案翻车的，是哪个细节？

同步 Steam \`userdata\` 里 \`<AppID>/remote/\` 的上一层文件夹。上一层放着 \`remotecache.vdf\` 以及本就应该因机器而异的成就和游戏时长文件，于是每次启动都像冲突，尽管没有任何存档动过。
`,ka=`---
title: "Ludusavi-Alternative: automatische Cloud-Synchronisierung für deine Spielstände"
description: "Ludusavi ist top für lokale Backups. Hoard ergänzt automatischen Sync zwischen PCs und Steam Deck mit Versionsverlauf, auf derselben Spielstand-Datenbank."
order: 5
updated: 2026-10-01
---

Wenn du nach einer Möglichkeit suchst, deine Spielstände zu sichern und zu synchronisieren, bist du wahrscheinlich auf **Ludusavi** gestoßen — und es ist hervorragend. Diese Anleitung ist ein ehrlicher Vergleich, damit du das richtige Tool wählst, und erklärt, wo Hoard passt, wenn du automatische Cloud-Synchronisierung über mehrere Geräte willst.

## Was Ludusavi gut macht

Ludusavi ist ein kostenloses Open-Source-Tool (von mtkennerly), um PC-Spielstände unter Windows, macOS und Linux zu sichern und wiederherzustellen. Es hat eine aufgeräumte GUI und eine CLI, findet Stände für Tausende Spiele automatisch, führt versionierte lokale Backups und kann diese über **Rclone** in eine eigene Cloud übertragen (Google Drive, Dropbox und viele andere). Wenn du volle Kontrolle und ein Do-it-yourself-Setup willst, ist Ludusavi eine fantastische Wahl — und völlig kostenlos.

Hoard will das nicht ersetzen. Tatsächlich nutzt **Hoard dieselbe Community-Datenbank für Speicherorte, auf die sich auch Ludusavi stützt**, um zu finden, wo jedes Spiel seine Stände ablegt — die Erkennungsqualität ist also gleichwertig.

## Worin sich Hoard unterscheidet

Die Lücke, auf die die meisten bei jedem lokalen Tool stoßen, ist die **Synchronisierung über Geräte hinweg**. Mit Ludusavi machst du das selbst: Backup planen, Rclone-Remote konfigurieren, dann auf dem anderen PC wiederherstellen, bevor du spielst. Das funktioniert, ist aber manuell.

Hoard macht daraus **verwaltete Cloud-Synchronisierung**:

- **Anmelden und loslegen.** Keine Rclone-Remotes, keine Skripte. Hoard lädt deinen Stand nach dem Spielen hoch und vor dem Start die neueste Version herunter, auf jedem PC deines Kontos.
- **Versionierte Historie in der Cloud.** Jedes Backup bleibt erhalten, du kannst also zu jedem früheren Stand zurück — sogar nach einem Festplattenausfall oder einer Neuinstallation.
- **Konfliktbewusst.** Hoard vergleicht Zeitstempel und behält eine lokale Kopie von allem, was es ersetzt, sodass eine Synchronisierung nie stillschweigend Fortschritt zerstört.
- **Weiterhin Open Source und selbst hostbar.** Wie bei Ludusavi gibt es keine Bindung — nutze Hoard Cloud oder hoste den Server selbst.

## Direkter Vergleich

| | Ludusavi | Hoard |
|---|---|---|
| Lokale Backups | Ja | Ja |
| Erkennung der Stände | Community-Manifest | Dasselbe Manifest, dazu Steam-Bibliotheken, laufende Prozesse und ein Dateisystem-Scan |
| Cloud-Speicher | Eigener, über Rclone | Enthalten, oder dein eigener Server |
| Synchronisierung zwischen PCs | Manuell: hier sichern, dort wiederherstellen | Automatisch, nach dem Spielen und vor dem Start |
| Versionshistorie | Lokale Backups, die du selbst aufräumst | Jede Version in der Cloud, dedupliziert per Inhalts-Hash |
| Emulatoren | Ja | Ja |
| Oberflächen | Desktop-App und CLI | Desktop-App, CLI und ein Overlay im Spiel |
| Preis | Kostenlos | Kostenlos mit 2 GB und 3 Geräten, Pro darüber, ohne Limit beim Selbsthosten |
| Lizenz | MIT | AGPL-3.0 |

## Wann Ludusavi die bessere Wahl ist

Das ist der Teil, den die meisten Vergleichsseiten weglassen. Ludusavi ist das bessere Werkzeug, wenn:

- **Du nur an einem PC spielst.** Cloud-Synchronisierung löst dann ein Problem, das du nicht hast. Ein lokales Backup reicht, und darin ist Ludusavi sehr gut.
- **Du bereits ein Rclone-Remote hast, dem du vertraust.** Wenn dein Speicher eingerichtet ist und läuft, ist Hoards Hauptvorteil ein Einrichtungsschritt, den du längst hinter dir hast.
- **Du es im Spielmodus des Steam Deck nutzen willst.** Für Ludusavi gibt es ein Decky-Plugin, du kannst ein Backup also anstoßen, ohne die Konsolenoberfläche zu verlassen.
- **Du eine permissive Lizenz brauchst.** Ludusavi ist MIT, Hoard ist AGPL-3.0. Wenn du etwas darauf aufbauen und das Ergebnis nicht veröffentlichen willst, macht dieser Unterschied viel aus.
- **Du willst nichts laufen haben.** Hoard selbst zu hosten heißt, irgendwo einen kleinen Server am Laufen zu halten, und sei es derselbe PC. Ludusavi ist eine App, die du öffnest, wenn du sie brauchst.

## Ludusavi vs. GameSave Manager

Der andere Name, der neben Ludusavi fällt, ist **GameSave Manager**, ein altgedientes Windows-Werkzeug. Beide lösen dasselbe Problem auf unterschiedliche Weise:

- **Plattform.** Ludusavi läuft unter Windows, macOS und Linux, Steam Deck eingeschlossen. GameSave Manager nur unter Windows.
- **Lizenz.** Ludusavi ist quelloffen (MIT). GameSave Manager ist kostenlos, aber Closed Source.
- **Spielstände finden.** Ludusavi liest das Community-Manifest, das aus PCGamingWiki entsteht, rund 20.000 Spiele. GameSave Manager bringt eine eigene Datenbank mit.
- **Spielstände in die Cloud bringen.** Ludusavi kopiert seine Backups per Rclone auf ein Remote. „Sync & Link“ von GameSave Manager verschiebt einen Spielstand-Ordner in einen Cloud-Ordner wie Dropbox oder OneDrive und hinterlässt an seiner Stelle einen Link, sodass der Cloud-Client den aktiven Ordner synchronisiert.

Dieser letzte Punkt ist der, den man abwägen sollte. Ein aktiver Ordner, den ein allgemeiner Cloud-Client synchronisiert, ist genau das Setup, das einen halb geschriebenen Spielstand auf jedem PC zu einem kaputten macht; die [Syncthing-Anleitung](/guides/syncthing-game-saves) erklärt, warum. Hoard steht auf der anderen Seite dieser Linie: Es wartet, bis das Spiel geschlossen ist, lädt eine Version hoch und behält die alten.

## Von Ludusavi zu Hoard wechseln

Es gibt keinen Import, und das ist Absicht. Die Schritte:

1. **Lass deine Ludusavi-Backups genau dort, wo sie sind.** Es wird nichts migriert und nichts gelöscht. Behalte sie in den ersten Wochen als Sicherheitsnetz.
2. **Installiere Hoard und melde dich an**, oder richte es auf deinen eigenen Server.
3. **Lass es scannen.** Es liest dasselbe Manifest, die Liste der erkannten Spiele sollte dir also bekannt vorkommen.
4. **Richte Hoard nicht auf deinen Ludusavi-Backup-Ordner.** Verfolge den Ordner, in den das Spiel selbst schreibt. Ein Backup-Ordner ist eine Kopie, die sich nach Zeitplan ändert statt beim Spielen, und die Kopie einer Kopie zu synchronisieren ist der Weg, am Ende den Fortschritt von gestern wiederherzustellen. Hoard versucht das selbst zu erkennen — \`hoard doctor\` meldet einen verfolgten Ordner, der wie ein Backup-Spiegel aussieht — aber am einfachsten ist, ihn gar nicht erst aufzunehmen.
5. **Spiel einmal.** Beim Beenden erscheint die erste Version in der Historie.
6. **Wiederhole das am zweiten PC.** Dort anmelden, und die Versionen liegen schon bereit.

## Zwei Details, die man kennen sollte

**Steam-Spielstände liegen einen Ordner tiefer als gedacht.** Bei Steam-Spielen verfolgt Hoard \`<AppID>/remote/\` innerhalb von \`userdata\`, nicht den Ordner darüber. Der übergeordnete Ordner enthält auch \`remotecache.vdf\` sowie Dateien für Erfolge und Spielzeit, und die unterscheiden sich zu Recht von Rechner zu Rechner. Synchronisierst du den übergeordneten Ordner, sieht jeder Start nach einem Konflikt aus, obwohl sich kein einziger Spielstand bewegt hat. Das ist der häufigste Grund, warum ein selbstgebautes Setup zwischen Steam Deck und Desktop gegen sich selbst arbeitet.

**Versionen sind billig.** Snapshots werden per Inhalts-Hash gespeichert, unveränderte Dateien also nur einmal. Zehn Versionen eines 2 GB großen Spielstands kosten etwa 2 GB, nicht 20 — und genau das macht es praktikabel, die komplette Historie zu behalten, statt sie auszudünnen.

## Was Selbsthosten wirklich bedeutet

Genau hier liegen die meisten Vergleiche bei Hoard falsch, deshalb der Punkt im Detail. Es gibt zwei Betriebsarten, und sie unterscheiden sich wirklich:

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Spielstände liegen auf unseren Servern in der EU.
- **Selbsthosten gehört vollständig dir.** Du betreibst \`hoard-server\` auf deinem eigenen PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir können weder einen Spielstand noch einen Spieltitel noch eine E-Mail-Adresse sehen, schlicht weil davon nichts bei uns ankommt. Verschwände Hoard Cloud morgen, liefe ein selbst gehostetes Setup unverändert weiter.

Dasselbe Programm, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört.

## Was solltest du wählen?

- Wähle **Ludusavi**, wenn du ein kostenloses, lokal orientiertes Backup-Tool willst und gern deine eigene Cloud mit Rclone einrichtest.
- Wähle **Hoard**, wenn Backups *und* automatische Synchronisierung über PCs einfach funktionieren sollen, mit versionierter Cloud-Historie und der Option, selbst zu hosten.

Viele beginnen mit Ludusavi für lokale Backups und wechseln zu Hoard, sobald sie dieselben Spiele auf mehr als einem Gerät spielen. Wenn das auf dich zutrifft, siehe [wie du Spielstände über PCs synchronisierst](/guides/sync-game-saves-across-pcs) oder [lade einfach Hoard herunter](/download) und melde dich an. Einen Blick auf das ganze Feld gibt der [Vergleich aller Sync-Tools](/guides/game-save-sync-comparison).

<!-- faq -->

## Häufige Fragen

### Kann ich Ludusavi und Hoard gleichzeitig nutzen?

Ja. Beide lesen dieselben Speicherorte und keines hält die Dateien geöffnet. Viele behalten Ludusavi für lokale Archiv-Backups und überlassen Hoard die Synchronisierung zwischen Geräten. Die einzige Regel: richte keines der beiden Werkzeuge auf den Backup-Ordner des anderen.

### Importiert Hoard meine Ludusavi-Backups?

Nein, und das ist Absicht. Ein Backup-Ordner ist eine Kopie, die sich nach eigenem Zeitplan ändert; ihn zu verfolgen würde einen veralteten Spiegel synchronisieren statt deines echten Spielstands. Hoard verfolgt den Ordner, in den das Spiel schreibt, und beginnt seine eigene Historie mit deiner nächsten Sitzung. Behalte das Ludusavi-Archiv als Sicherheitsnetz.

### Ist Hoard kostenlos?

Hoard Cloud hat einen kostenlosen Tarif mit 2 GB Speicher und 3 Geräten, was für die meisten Sammlungen reicht; Pro hebt beides an. Den Server selbst zu hosten ist kostenlos und hat überhaupt kein Limit. Alles ist Open Source unter AGPL-3.0.

### Funktioniert Hoard auf dem Steam Deck?

Ja, auf dem Steam Deck und jedem Linux-Desktop, ebenso unter Windows und macOS. Das Deck ist genau der Fall, für den das \`remote/\`-Detail oben wichtig ist: Deck und Desktop schreiben neben demselben Spielstand unterschiedliche Dateien für Erfolge und Spielzeit.

### Brauche ich Rclone oder ein eigenes Cloud-Konto?

Nein. Das ist der wesentliche praktische Unterschied: Bei Hoard Cloud ist der Speicher schon eingerichtet, sobald du dich anmeldest. Wenn dir der Speicher lieber selbst gehört, betreibe den Server selbst gegen einen S3-kompatiblen Bucket oder einen gewöhnlichen Ordner auf deiner eigenen Maschine.

### Sendet Selbsthosten irgendetwas an Hoard?

Nein. Im selbst gehosteten Betrieb gibt es kein Konto bei uns und keine Telemetrie zu uns: deine Spielstände, deine Nutzer und deine Logs liegen auf deinem eigenen Server und berühren unseren nie. Das ist der ganze Sinn dieses Modus, und deshalb ist der Server dasselbe quelloffene Binary, das wir selbst betreiben, und keine abgespeckte Fassung.

### Ist Ludusavi sicher?

Ja. Es ist quelloffen, weit verbreitet und kopiert lediglich Spielstand-Dateien in einen Backup-Ordner und zurück; Spieldateien rührt es nicht an. Worauf du achten solltest, gilt für jedes Backup-Werkzeug: Ein altes Backup über einen neueren Spielstand wiederherzustellen ersetzt ihn, also prüfe vorher das Datum.

### Funktioniert Ludusavi auf dem Steam Deck?

Ja. Es gibt eine Linux-Version, die du im Desktop-Modus installierst, und ein Decky-Plugin, um Backups aus dem Spielmodus auszulösen. Was es allein nicht leistet, ist, Deck und Desktop auf demselben Stand zu halten: Du sicherst auf dem einen und stellst auf dem anderen wieder her. Hoard erledigt diesen Teil automatisch über einen Hintergrunddienst, ohne dass du etwas auslösen musst.

### Kann Ludusavi in Google Drive oder eine andere Cloud sichern?

Ja, über Rclone: Du richtest ein Remote für Google Drive, Dropbox, OneDrive oder einen anderen von Rclone unterstützten Anbieter ein, und Ludusavi kopiert seine Backups dorthin. Bei Hoard gibt es kein Remote einzurichten, und wenn dir der Speicher gehören soll, zeigst du es auf deinen eigenen Server.

### Sichert Ludusavi automatisch?

Nicht von selbst: Es sichert, wenn du es ausführst. Du kannst es mit der Kommandozeile und einer geplanten Aufgabe automatisieren oder den Startbefehl eines Spiels umhüllen, damit beim Beenden gesichert wird. Hoard bemerkt ohne jede Umhüllung, wenn ein Spiel geschlossen wird, und sichert dann.

### Unterstützt Ludusavi Spiele außerhalb von Steam?

Ja. Das Manifest deckt Spiele von GOG, Epic, Xbox und anderen Launchern ab, dazu viele ohne Store, und du kannst eigene Einträge für Fehlendes anlegen. Hoard liest dasselbe Manifest und ergänzt einen Dateisystem-Scan für Spiele, die dort nicht stehen.
`,Da=`---
title: "Ludusavi alternative: automatic cloud sync for your game saves"
description: "Ludusavi is great for local backups. Hoard adds automatic sync between PCs and Steam Deck with version history, built on the same save database."
order: 5
updated: 2026-10-01
related: game-save-sync-comparison, sync-game-saves-across-pcs, back-up-emulator-saves
---

If you're looking for a way to back up and sync your game saves, you've probably found **Ludusavi** — and it's excellent. This guide is an honest comparison so you can pick the right tool, and it explains where Hoard fits if you want automatic cloud sync across machines.

## What Ludusavi does well

Ludusavi is a free, open-source tool (made by mtkennerly) for backing up and restoring PC game saves on Windows, macOS and Linux. It has a clean GUI and a CLI, finds saves for thousands of games automatically, keeps versioned local backups, and can push those backups to a cloud you own by configuring **Rclone** (Google Drive, Dropbox, and many others). If you want full control and a do-it-yourself setup, Ludusavi is a fantastic choice — and it's completely free.

Hoard isn't here to replace that. In fact, **Hoard uses the same community save-location database that Ludusavi relies on** to locate where each game stores its saves, so detection quality is on par.

## Where Hoard is different

The gap most people hit with any local-first tool is **syncing across devices**. With Ludusavi you do it yourself: schedule a backup, configure an Rclone remote, then restore on the other PC before you play. That works, but it's manual.

Hoard turns that into **managed cloud sync**:

- **Sign in and go.** No Rclone remotes, no scripts. Hoard uploads your save after you finish playing and downloads the latest before you start, on every PC on your account.
- **Versioned history in the cloud.** Every backup is kept, so you can roll back to any earlier save — even after a disk failure or a fresh install.
- **Conflict-aware.** Hoard compares timestamps and keeps a local copy of anything it replaces, so a sync never silently destroys progress.
- **Still open source and self-hostable.** Like Ludusavi, you're not locked in — run Hoard Cloud or host the server yourself.

## Side by side

| | Ludusavi | Hoard |
|---|---|---|
| Local backups | Yes | Yes |
| Save detection | Community manifest | The same manifest, plus Steam libraries, running processes and a filesystem scan |
| Cloud storage | Bring your own, through Rclone | Included, or your own server |
| Sync between PCs | Manual: back up here, restore there | Automatic, after you stop playing and before you start |
| Version history | Local backups you prune yourself | Every version kept in the cloud, deduplicated by content hash |
| Emulators | Yes | Yes |
| Interfaces | Desktop app and CLI | Desktop app, CLI, and an in-game overlay |
| Price | Free | Free tier of 2 GB and 3 devices, Pro above that, no quota at all if you self-host |
| Licence | MIT | AGPL-3.0 |

## When Ludusavi is the better choice

This is the part most comparison pages skip. Ludusavi is the better tool when:

- **You only play on one PC.** Cloud sync solves a problem you don't have. A local backup is enough, and Ludusavi does local backups very well.
- **You already have an Rclone remote you trust.** If your storage is wired up and working, Hoard's main advantage is a setup step you've already paid for.
- **You want to run it from Game Mode on a Steam Deck.** Ludusavi has a Decky plugin, so you can trigger a backup without leaving the console interface.
- **You want a permissive licence.** Ludusavi is MIT, Hoard is AGPL-3.0. If you intend to build something on top and not publish the result, that difference matters.
- **You don't want anything running.** Self-hosting Hoard means keeping a small server up somewhere, even if it's the same PC. Ludusavi is an app you open when you want it.

## Ludusavi vs GameSave Manager

The other name that comes up next to Ludusavi is **GameSave Manager**, a long-standing Windows tool. They solve the same problem in different ways:

- **Platform.** Ludusavi runs on Windows, macOS and Linux, Steam Deck included. GameSave Manager is Windows only.
- **Licence.** Ludusavi is open source (MIT). GameSave Manager is free to use but closed source.
- **Finding saves.** Ludusavi reads the community manifest built from PCGamingWiki, around 20,000 games. GameSave Manager ships its own database.
- **Getting saves to the cloud.** Ludusavi copies its backups to a remote through Rclone. GameSave Manager's "Sync & Link" moves a save folder into a cloud folder such as Dropbox or OneDrive and leaves a link in its place, so the cloud client syncs the live folder.

That last point is the one to weigh. A live folder synced by a general-purpose cloud client is the setup that turns a half-written save into a broken save on every PC, and the [Syncthing guide](/guides/syncthing-game-saves) goes through why. Hoard sits on the other side of that line: it waits until the game has closed, uploads a version, and keeps the old ones.

## Moving from Ludusavi to Hoard

There's no importer, and that's on purpose. The steps:

1. **Leave your Ludusavi backups exactly where they are.** Nothing is migrated or deleted. Keep them as a safety net for the first few weeks.
2. **Install Hoard and sign in**, or point it at your own server.
3. **Let it scan.** It reads the same manifest, so the list of detected games should look familiar.
4. **Don't point Hoard at your Ludusavi backup folder.** Track the folder the game itself writes to. A backup folder is a copy that changes on a schedule rather than when you play, and syncing a copy of a copy is how you end up restoring yesterday's progress. Hoard tries to catch this on its own — \`hoard doctor\` flags a tracked folder that looks like a backup mirror — but it's easier never to track it.
5. **Play once.** When you quit, the first version appears in the history.
6. **Repeat on the second PC.** Sign in there and the versions are already waiting.

## Two details worth knowing

**Steam saves live one folder deeper than you think.** For Steam games, Hoard tracks \`<AppID>/remote/\` inside \`userdata\`, not the folder above it. The parent also holds \`remotecache.vdf\` and achievement and playtime files, and those legitimately differ from machine to machine. Sync the parent and every launch looks like a conflict even though no save actually moved. It's the most common reason a hand-rolled Steam Deck ↔ desktop setup ends up fighting itself.

**Versions are cheap.** Snapshots are stored by content hash, so unchanged files are stored once. Ten versions of a 2 GB save cost about 2 GB, not 20 — which is what makes keeping the full history practical instead of pruning it.

## What self-hosting actually means

This is the point most comparisons get wrong about Hoard, so it's worth being exact. There are two ways to run it, and they are genuinely different:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **Self-hosting is entirely yours.** You run \`hoard-server\` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, for the simple reason that none of it ever reaches us. If Hoard Cloud disappeared tomorrow, a self-hosted setup would carry on unchanged.

Same program, same detection, same version history. The only thing that changes is who owns the storage.

## Which should you choose?

- Choose **Ludusavi** if you want a free, local-first backup tool and you're happy to wire up your own cloud with Rclone.
- Choose **Hoard** if you want backups *and* automatic sync across PCs to just work, with a versioned cloud history, while keeping the option to self-host.

Many people start with Ludusavi for local backups and move to Hoard once they're playing the same games on more than one machine. If that's you, see [how to sync game saves across PCs](/guides/sync-game-saves-across-pcs) or just [download Hoard](/download) and sign in. For the wider field, there's a [comparison of every save sync tool](/guides/game-save-sync-comparison).

<!-- faq -->

## Frequently asked questions

### Can I use Ludusavi and Hoard at the same time?

Yes. They read the same save locations and neither one holds the files open. Plenty of people keep Ludusavi for local archive backups and let Hoard handle sync between machines. The only rule is not to point either tool at the other's backup folder.

### Does Hoard import my Ludusavi backups?

No, and that's deliberate. A backup folder is a copy that changes on its own schedule, so tracking it would sync a stale mirror instead of your live save. Hoard tracks the folder the game writes to and starts its own history from your next session. Keep the Ludusavi archive as a safety net.

### Is Hoard free?

Hoard Cloud has a free tier with 2 GB of storage and 3 devices, which covers most save collections; Pro raises both. Self-hosting the server is free and has no quota at all. Everything is open source under AGPL-3.0.

### Does Hoard work on Steam Deck?

Yes, on Steam Deck and any Linux desktop, as well as Windows and macOS. The Deck is exactly the case that needs the \`remote/\` detail above, because a Deck and a desktop write different achievement and playtime files next to the same save.

### Do I need Rclone or a cloud account of my own?

No. That's the main practical difference: with Hoard Cloud, storage is already set up when you sign in. If you'd rather own the storage, run the server yourself against an S3-compatible bucket or a plain folder on your own machine.

### Does self-hosting send anything to Hoard?

No. In self-hosted mode there is no account with us and no telemetry to us: your saves, your users and your logs live on your own server and never touch ours. That's the whole point of the mode, and it's why the server is the same open-source binary we run ourselves rather than a cut-down version.

### Is Ludusavi safe to use?

Yes. It's open source, widely used, and all it does is copy save files to a backup folder and back again; it doesn't touch game files. The one thing to watch is true of any backup tool: restoring an old backup over a newer save replaces it, so check the date before you restore.

### Does Ludusavi work on Steam Deck?

Yes. There's a Linux build you can install in Desktop Mode, and a Decky plugin to trigger backups from Game Mode. What it doesn't do on its own is keep the Deck and your desktop in step: you back up on one and restore on the other. Hoard does that part automatically, from a background service, so there's nothing to trigger.

### Can Ludusavi back up to Google Drive or another cloud?

Yes, through Rclone: you set up a remote for Google Drive, Dropbox, OneDrive or any other provider Rclone supports, and Ludusavi copies its backups there. With Hoard there's no remote to set up, and if you'd rather own the storage you point it at your own server.

### Does Ludusavi back up automatically?

Not by itself: it backs up when you run it. You can automate it with its command line and a scheduled task, or wrap a game's launch command so it backs up when the game exits. Hoard notices when a game closes, with no wrapping, and backs up then.

### Does Ludusavi support non-Steam games?

Yes. The manifest covers games from GOG, Epic, Xbox and other launchers, plus many sold without a store, and you can add custom entries for anything missing. Hoard reads the same manifest and adds a filesystem scan for games it doesn't list.
`,qa=`---
title: "Alternativa a Ludusavi: sincronización automática de partidas en la nube"
description: "Ludusavi es genial para copias locales. Hoard añade sync automático entre PC y Steam Deck con historial de versiones, sobre la misma base de datos de saves."
order: 5
updated: 2026-10-01
---

Si buscas una forma de hacer copia y sincronizar tus partidas guardadas, seguramente has encontrado **Ludusavi**, y es excelente. Esta guía es una comparativa honesta para que elijas la herramienta adecuada, y explica dónde encaja Hoard si quieres sincronización automática en la nube entre equipos.

## Qué hace bien Ludusavi

Ludusavi es una herramienta gratuita y open source (creada por mtkennerly) para hacer copias y restaurar partidas de PC en Windows, macOS y Linux. Tiene una interfaz limpia y una CLI, detecta automáticamente las partidas de miles de juegos, guarda copias locales versionadas y puede subir esas copias a una nube tuya configurando **Rclone** (Google Drive, Dropbox y muchas más). Si quieres control total y un montaje a tu medida, Ludusavi es una opción fantástica, y es completamente gratis.

Hoard no viene a reemplazar eso. De hecho, **Hoard usa la misma base de datos comunitaria de ubicación de partidas en la que se apoya Ludusavi** para localizar dónde guarda cada juego sus saves, así que la calidad de detección está a la par.

## En qué se diferencia Hoard

El punto donde la mayoría se atasca con cualquier herramienta local es **sincronizar entre dispositivos**. Con Ludusavi lo haces tú: programas una copia, configuras un remoto de Rclone y luego restauras en el otro PC antes de jugar. Funciona, pero es manual.

Hoard convierte eso en **sincronización gestionada en la nube**:

- **Inicia sesión y listo.** Sin remotos de Rclone, sin scripts. Hoard sube tu partida cuando terminas de jugar y descarga la última antes de empezar, en todos los PC de tu cuenta.
- **Historial versionado en la nube.** Se conserva cada copia, así que puedes volver a cualquier partida anterior, incluso tras un fallo de disco o una instalación limpia.
- **Tiene en cuenta los conflictos.** Hoard compara fechas y guarda una copia local de lo que reemplaza, así que una sincronización nunca destruye progreso en silencio.
- **Sigue siendo open source y autoalojable.** Como Ludusavi, no hay bloqueo: usa Hoard Cloud o aloja el servidor tú mismo.

## Cara a cara

| | Ludusavi | Hoard |
|---|---|---|
| Copias locales | Sí | Sí |
| Detección de partidas | Manifiesto comunitario | El mismo manifiesto, más bibliotecas de Steam, procesos en ejecución y un barrido del disco |
| Almacenamiento en la nube | El tuyo, vía Rclone | Incluido, o tu propio servidor |
| Sincronización entre PC | Manual: copia aquí, restaura allí | Automática, al dejar de jugar y antes de empezar |
| Historial de versiones | Copias locales que podas tú | Todas las versiones en la nube, deduplicadas por hash de contenido |
| Emuladores | Sí | Sí |
| Interfaces | App de escritorio y CLI | App de escritorio, CLI y overlay dentro del juego |
| Precio | Gratis | Plan gratis de 2 GB y 3 dispositivos, Pro por encima, sin cupo si te autoalojas |
| Licencia | MIT | AGPL-3.0 |

## Cuándo Ludusavi es la mejor opción

Ésta es la parte que casi ninguna comparativa incluye. Ludusavi es mejor herramienta cuando:

- **Sólo juegas en un PC.** La sincronización en la nube resuelve un problema que no tienes. Con una copia local basta, y Ludusavi hace copias locales muy bien.
- **Ya tienes un remoto de Rclone que funciona.** Si tu almacenamiento está montado y va fino, la ventaja principal de Hoard es un paso de configuración que tú ya has pagado.
- **Quieres usarlo desde el modo Juego de una Steam Deck.** Ludusavi tiene un plugin de Decky, así que puedes lanzar una copia sin salir de la interfaz de consola.
- **Quieres una licencia permisiva.** Ludusavi es MIT y Hoard es AGPL-3.0. Si piensas construir algo encima y no publicar el resultado, esa diferencia importa.
- **No quieres nada corriendo de fondo.** Autoalojar Hoard implica mantener un servidor en pie, aunque sea en el mismo PC. Ludusavi es una aplicación que abres cuando te hace falta.

## Ludusavi frente a GameSave Manager

El otro nombre que sale junto a Ludusavi es **GameSave Manager**, una herramienta veterana de Windows. Las dos resuelven el mismo problema de forma distinta:

- **Plataforma.** Ludusavi funciona en Windows, macOS y Linux, Steam Deck incluida. GameSave Manager sólo en Windows.
- **Licencia.** Ludusavi es de código abierto (MIT). GameSave Manager es gratis, pero de código cerrado.
- **Encontrar las partidas.** Ludusavi lee el manifiesto comunitario construido a partir de PCGamingWiki, unos 20.000 juegos. GameSave Manager trae su propia base de datos.
- **Llevar las partidas a la nube.** Ludusavi copia sus copias de seguridad a un remoto mediante Rclone. El «Sync & Link» de GameSave Manager mueve la carpeta de partidas a una carpeta en la nube, como Dropbox u OneDrive, y deja un enlace en su lugar, de modo que el cliente de la nube sincroniza la carpeta viva.

Ese último punto es el que hay que sopesar. Una carpeta viva sincronizada por un cliente de nube genérico es justo el montaje que convierte una partida a medio escribir en una partida rota en todos tus PC, y la [guía de Syncthing](/guides/syncthing-game-saves) explica por qué. Hoard está al otro lado de esa línea: espera a que el juego se cierre, sube una versión y conserva las anteriores.

## Pasar de Ludusavi a Hoard

No hay importador, y es a propósito. Los pasos:

1. **Deja tus copias de Ludusavi exactamente donde están.** No se migra ni se borra nada. Consérvalas como red de seguridad las primeras semanas.
2. **Instala Hoard e inicia sesión**, o apúntalo a tu propio servidor.
3. **Déjalo escanear.** Lee el mismo manifiesto, así que la lista de juegos detectados debería resultarte familiar.
4. **No apuntes Hoard a la carpeta de copias de Ludusavi.** Rastrea la carpeta en la que escribe el juego. Una carpeta de copias es un duplicado que cambia por horario y no cuando juegas, y sincronizar la copia de una copia es como acabas restaurando el progreso de ayer. Hoard intenta detectarlo solo — \`hoard doctor\` avisa de una carpeta rastreada que parece un espejo de copias — pero es más fácil no rastrearla nunca.
5. **Juega una vez.** Al salir, la primera versión aparece en el historial.
6. **Repite en el segundo PC.** Inicias sesión y las versiones ya están ahí.

## Dos detalles que conviene saber

**Las partidas de Steam viven una carpeta más adentro de lo que parece.** En los juegos de Steam, Hoard rastrea \`<AppID>/remote/\` dentro de \`userdata\`, no la carpeta de encima. La carpeta padre guarda además \`remotecache.vdf\` y ficheros de logros y de tiempo jugado, y ésos son legítimamente distintos en cada máquina. Si sincronizas la padre, cada arranque parece un conflicto aunque no se haya movido ninguna partida. Es el motivo más común de que un montaje casero entre Steam Deck y sobremesa acabe peleándose consigo mismo.

**Las versiones salen baratas.** Las instantáneas se guardan por hash de contenido, así que un fichero que no cambia se almacena una sola vez. Diez versiones de una partida de 2 GB ocupan unos 2 GB, no 20, y eso es lo que hace práctico conservar el historial entero en vez de ir podándolo.

## Qué significa realmente autoalojarse

Es el punto donde casi todas las comparativas se equivocan con Hoard, así que conviene ser exacto. Hay dos formas de usarlo, y son genuinamente distintas:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Autoalojarse es tuyo por completo.** Levantas \`hoard-server\` en tu PC o en tu NAS, y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, por la sencilla razón de que nada de eso nos llega. Si Hoard Cloud desapareciera mañana, un montaje autoalojado seguiría funcionando igual.

El mismo programa, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento.

## ¿Cuál elegir?

- Elige **Ludusavi** si quieres una herramienta de copia gratuita y local y no te importa montar tu propia nube con Rclone.
- Elige **Hoard** si quieres que la copia *y* la sincronización entre PC funcionen solas, con historial versionado en la nube, sin renunciar a poder autoalojarte.

Mucha gente empieza con Ludusavi para copias locales y pasa a Hoard cuando juega a los mismos juegos en más de un equipo. Si es tu caso, mira [cómo sincronizar partidas entre PC](/guides/sync-game-saves-across-pcs) o simplemente [descarga Hoard](/download) e inicia sesión. Y si quieres el panorama completo, hay una [comparativa de todas las herramientas de sincronización](/guides/game-save-sync-comparison).

<!-- faq -->

## Preguntas frecuentes

### ¿Puedo usar Ludusavi y Hoard a la vez?

Sí. Leen las mismas ubicaciones de partidas y ninguno de los dos bloquea los ficheros. Mucha gente conserva Ludusavi para copias de archivo locales y deja que Hoard se encargue de la sincronización entre equipos. La única regla es no apuntar una herramienta a la carpeta de copias de la otra.

### ¿Hoard importa mis copias de Ludusavi?

No, y es deliberado. Una carpeta de copias es un duplicado que cambia según su propio horario, así que rastrearla sincronizaría un espejo desfasado en lugar de tu partida real. Hoard rastrea la carpeta en la que escribe el juego y arranca su propio historial desde tu siguiente sesión. Guarda el archivo de Ludusavi como red de seguridad.

### ¿Hoard es gratis?

Hoard Cloud tiene un plan gratuito con 2 GB de almacenamiento y 3 dispositivos, que cubre la mayoría de colecciones de partidas; Pro sube ambos. Autoalojar el servidor es gratis y no tiene cupo ninguno. Todo es open source bajo AGPL-3.0.

### ¿Funciona en Steam Deck?

Sí, en Steam Deck y en cualquier escritorio Linux, además de Windows y macOS. La Deck es justo el caso que necesita el detalle de \`remote/\` de más arriba, porque una Deck y un sobremesa escriben ficheros de logros y de tiempo jugado distintos junto a la misma partida.

### ¿Necesito Rclone o una cuenta de nube propia?

No. Ésa es la diferencia práctica principal: con Hoard Cloud el almacenamiento ya está listo al iniciar sesión. Si prefieres ser dueño del almacenamiento, levanta el servidor tú mismo contra un bucket compatible con S3 o una carpeta normal de tu máquina.

### ¿Autoalojarse envía algo a Hoard?

No. En modo autoalojado no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus registros viven en tu propio servidor y nunca tocan el nuestro. Ése es el sentido del modo, y por eso el servidor es el mismo binario open source que usamos nosotros y no una versión recortada.

### ¿Es seguro usar Ludusavi?

Sí. Es de código abierto, lo usa mucha gente y lo único que hace es copiar los ficheros de partida a una carpeta de copia y de vuelta; no toca los ficheros del juego. Lo único a vigilar vale para cualquier herramienta de copias: restaurar una copia vieja encima de una partida más nueva la sustituye, así que mira la fecha antes de restaurar.

### ¿Ludusavi funciona en Steam Deck?

Sí. Hay una versión para Linux que se instala en el modo Escritorio y un plugin de Decky para lanzar copias desde el modo Juego. Lo que no hace por sí solo es mantener la Deck y el sobremesa al día: haces la copia en uno y restauras en el otro. Hoard hace esa parte automáticamente, desde un servicio en segundo plano, sin que tengas que lanzar nada.

### ¿Ludusavi puede hacer copias en Google Drive u otra nube?

Sí, mediante Rclone: configuras un remoto de Google Drive, Dropbox, OneDrive o cualquier otro proveedor que soporte Rclone, y Ludusavi copia allí sus copias de seguridad. Con Hoard no hay remoto que configurar, y si prefieres que el almacenamiento sea tuyo, lo apuntas a tu propio servidor.

### ¿Ludusavi hace copias automáticamente?

Por sí solo no: hace la copia cuando lo ejecutas. Puedes automatizarlo con su línea de comandos y una tarea programada, o envolver el comando de arranque de un juego para que haga la copia al salir. Hoard nota cuándo se cierra un juego, sin envolver nada, y hace la copia entonces.

### ¿Ludusavi soporta juegos que no son de Steam?

Sí. El manifiesto cubre juegos de GOG, Epic, Xbox y otros launchers, además de muchos que se venden sin tienda, y puedes añadir entradas propias para lo que falte. Hoard lee el mismo manifiesto y añade un escaneo del disco para los juegos que no aparecen.
`,Pa=`---
title: "Alternative à Ludusavi : synchronisation cloud automatique de vos parties"
description: "Ludusavi excelle en sauvegarde locale. Hoard ajoute la synchro automatique entre PC et Steam Deck avec historique, sur la même base de données."
order: 5
updated: 2026-10-01
---

Si vous cherchez un moyen de sauvegarder et synchroniser vos parties, vous avez sans doute trouvé **Ludusavi** — et il est excellent. Ce guide est une comparaison honnête pour vous aider à choisir le bon outil, et explique où Hoard s'inscrit si vous voulez une synchro cloud automatique entre machines.

## Ce que Ludusavi fait bien

Ludusavi est un outil gratuit et open source (créé par mtkennerly) pour sauvegarder et restaurer les parties PC sous Windows, macOS et Linux. Il a une interface soignée et une CLI, trouve automatiquement les sauvegardes de milliers de jeux, conserve des sauvegardes locales versionnées, et peut envoyer ces sauvegardes vers un cloud qui vous appartient en configurant **Rclone** (Google Drive, Dropbox et bien d'autres). Si vous voulez un contrôle total et un montage fait main, Ludusavi est un choix fantastique — et entièrement gratuit.

Hoard n'est pas là pour le remplacer. En fait, **Hoard utilise la même base de données communautaire d'emplacements que celle sur laquelle s'appuie Ludusavi** pour localiser où chaque jeu range ses sauvegardes : la qualité de détection est donc équivalente.

## En quoi Hoard est différent

Le point où la plupart bloquent avec tout outil local, c'est la **synchronisation entre appareils**. Avec Ludusavi, vous la faites vous-même : planifier une sauvegarde, configurer un distant Rclone, puis restaurer sur l'autre PC avant de jouer. Ça marche, mais c'est manuel.

Hoard transforme cela en **synchro cloud gérée** :

- **Connectez-vous et c'est parti.** Pas de distants Rclone, pas de scripts. Hoard envoie votre sauvegarde après le jeu et télécharge la dernière version avant que vous commenciez, sur chaque PC de votre compte.
- **Historique versionné dans le cloud.** Chaque sauvegarde est conservée, vous pouvez donc revenir à n'importe quelle sauvegarde antérieure — même après une panne de disque ou une installation neuve.
- **Gestion des conflits.** Hoard compare les horodatages et conserve une copie locale de tout ce qu'il remplace, donc une synchro ne détruit jamais la progression en silence.
- **Toujours open source et auto-hébergeable.** Comme Ludusavi, pas de verrouillage — utilisez Hoard Cloud ou hébergez le serveur vous-même.

## Face à face

| | Ludusavi | Hoard |
|---|---|---|
| Sauvegardes locales | Oui | Oui |
| Détection des sauvegardes | Manifeste communautaire | Le même manifeste, plus les bibliothèques Steam, les processus en cours et un balayage du disque |
| Stockage cloud | Le vôtre, via Rclone | Inclus, ou votre propre serveur |
| Synchro entre PC | Manuelle : sauvegarder ici, restaurer là-bas | Automatique, après avoir joué et avant de commencer |
| Historique des versions | Sauvegardes locales que vous élaguez vous-même | Toutes les versions dans le cloud, dédupliquées par empreinte de contenu |
| Émulateurs | Oui | Oui |
| Interfaces | Application de bureau et CLI | Application de bureau, CLI et surcouche en jeu |
| Prix | Gratuit | Offre gratuite de 2 Go et 3 appareils, Pro au-delà, sans quota en auto-hébergement |
| Licence | MIT | AGPL-3.0 |

## Quand Ludusavi est le meilleur choix

C'est la partie que presque aucune page de comparaison n'inclut. Ludusavi est le meilleur outil quand :

- **Vous ne jouez que sur un seul PC.** La synchro cloud résout alors un problème que vous n'avez pas. Une sauvegarde locale suffit, et Ludusavi les fait très bien.
- **Vous avez déjà un distant Rclone en qui vous avez confiance.** Si votre stockage est configuré et fonctionne, l'avantage principal de Hoard est une étape que vous avez déjà payée.
- **Vous voulez l'utiliser depuis le mode Jeu d'un Steam Deck.** Ludusavi a un plugin Decky : vous pouvez lancer une sauvegarde sans quitter l'interface console.
- **Vous voulez une licence permissive.** Ludusavi est en MIT, Hoard en AGPL-3.0. Si vous comptez bâtir quelque chose par-dessus sans publier le résultat, cette différence compte.
- **Vous ne voulez rien qui tourne en fond.** Auto-héberger Hoard veut dire garder un petit serveur allumé quelque part, même sur le même PC. Ludusavi est une application que vous ouvrez au besoin.

## Ludusavi vs GameSave Manager

L'autre nom qui revient à côté de Ludusavi est **GameSave Manager**, un outil Windows de longue date. Les deux résolvent le même problème de manières différentes :

- **Plateforme.** Ludusavi tourne sous Windows, macOS et Linux, Steam Deck compris. GameSave Manager uniquement sous Windows.
- **Licence.** Ludusavi est open source (MIT). GameSave Manager est gratuit mais propriétaire.
- **Trouver les sauvegardes.** Ludusavi lit le manifeste communautaire issu de PCGamingWiki, environ 20 000 jeux. GameSave Manager embarque sa propre base de données.
- **Envoyer les sauvegardes dans le cloud.** Ludusavi copie ses sauvegardes vers un remote via Rclone. Le « Sync & Link » de GameSave Manager déplace un dossier de sauvegarde dans un dossier cloud comme Dropbox ou OneDrive et laisse un lien à sa place, si bien que le client cloud synchronise le dossier actif.

C'est ce dernier point qu'il faut peser. Un dossier actif synchronisé par un client cloud généraliste est précisément la configuration qui transforme une sauvegarde à moitié écrite en sauvegarde cassée sur tous vos PC ; le [guide Syncthing](/guides/syncthing-game-saves) explique pourquoi. Hoard se place de l'autre côté de cette ligne : il attend que le jeu soit fermé, envoie une version et garde les anciennes.

## Passer de Ludusavi à Hoard

Il n'y a pas d'importateur, et c'est volontaire. Les étapes :

1. **Laissez vos sauvegardes Ludusavi exactement où elles sont.** Rien n'est migré ni supprimé. Gardez-les comme filet de sécurité les premières semaines.
2. **Installez Hoard et connectez-vous**, ou pointez-le vers votre propre serveur.
3. **Laissez-le analyser.** Il lit le même manifeste : la liste des jeux détectés devrait vous sembler familière.
4. **Ne pointez pas Hoard vers votre dossier de sauvegardes Ludusavi.** Suivez le dossier dans lequel le jeu écrit lui-même. Un dossier de sauvegardes est une copie qui change selon un horaire et non quand vous jouez, et synchroniser la copie d'une copie, c'est ainsi qu'on finit par restaurer la progression d'hier. Hoard essaie de le repérer tout seul — \`hoard doctor\` signale un dossier suivi qui ressemble à un miroir de sauvegardes — mais le plus simple est de ne jamais l'ajouter.
5. **Jouez une fois.** En quittant, la première version apparaît dans l'historique.
6. **Recommencez sur le second PC.** Connectez-vous et les versions sont déjà là.

## Deux détails à connaître

**Les sauvegardes Steam sont un dossier plus bas qu'on ne croit.** Pour les jeux Steam, Hoard suit \`<AppID>/remote/\` dans \`userdata\`, pas le dossier au-dessus. Le dossier parent contient aussi \`remotecache.vdf\` ainsi que des fichiers de succès et de temps de jeu, qui diffèrent légitimement d'une machine à l'autre. Synchronisez le parent et chaque lancement ressemble à un conflit alors qu'aucune sauvegarde n'a bougé. C'est la raison la plus fréquente pour laquelle un montage maison entre Steam Deck et PC de bureau finit par se battre contre lui-même.

**Les versions coûtent peu.** Les instantanés sont stockés par empreinte de contenu : un fichier inchangé n'est stocké qu'une fois. Dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20 — c'est ce qui rend viable de garder tout l'historique au lieu de l'élaguer.

## Ce que l'auto-hébergement veut vraiment dire

C'est le point sur lequel presque toutes les comparaisons se trompent au sujet de Hoard, autant être précis. Il y a deux façons de l'utiliser, et elles sont réellement différentes :

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont stockées sur nos serveurs, dans l'UE.
- **L'auto-hébergement est entièrement le vôtre.** Vous faites tourner \`hoard-server\` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne pouvons voir ni une sauvegarde, ni un nom de jeu, ni une adresse e-mail, pour la simple raison que rien de tout cela ne nous parvient. Si Hoard Cloud disparaissait demain, une installation auto-hébergée continuerait à l'identique.

Le même programme, la même détection, le même historique de versions. La seule chose qui change, c'est à qui appartient le stockage.

## Lequel choisir ?

- Choisissez **Ludusavi** si vous voulez un outil de sauvegarde gratuit et local et que configurer votre propre cloud avec Rclone ne vous dérange pas.
- Choisissez **Hoard** si vous voulez que la sauvegarde *et* la synchro entre PC fonctionnent toutes seules, avec un historique cloud versionné, tout en gardant l'option de l'auto-hébergement.

Beaucoup commencent avec Ludusavi pour les sauvegardes locales et passent à Hoard dès qu'ils jouent aux mêmes jeux sur plus d'une machine. Si c'est votre cas, voir [comment synchroniser vos parties entre PC](/guides/sync-game-saves-across-pcs) ou simplement [téléchargez Hoard](/download) et connectez-vous. Pour l'ensemble du paysage, il y a une [comparaison de tous les outils de synchro](/guides/game-save-sync-comparison).

<!-- faq -->

## Questions fréquentes

### Puis-je utiliser Ludusavi et Hoard en même temps ?

Oui. Ils lisent les mêmes emplacements et aucun des deux ne verrouille les fichiers. Beaucoup gardent Ludusavi pour les sauvegardes d'archive locales et laissent Hoard gérer la synchro entre machines. La seule règle : ne pointez pas un outil vers le dossier de sauvegardes de l'autre.

### Hoard importe-t-il mes sauvegardes Ludusavi ?

Non, et c'est délibéré. Un dossier de sauvegardes est une copie qui change selon son propre horaire ; le suivre synchroniserait un miroir périmé au lieu de votre sauvegarde réelle. Hoard suit le dossier dans lequel le jeu écrit et démarre son propre historique à votre prochaine session. Gardez l'archive Ludusavi comme filet de sécurité.

### Hoard est-il gratuit ?

Hoard Cloud a une offre gratuite de 2 Go de stockage et 3 appareils, ce qui couvre la plupart des collections ; Pro augmente les deux. Auto-héberger le serveur est gratuit et sans aucun quota. Tout est open source sous AGPL-3.0.

### Hoard fonctionne-t-il sur Steam Deck ?

Oui, sur Steam Deck et sur n'importe quel bureau Linux, ainsi que sous Windows et macOS. Le Deck est précisément le cas qui exige le détail \`remote/\` ci-dessus, car un Deck et un PC de bureau écrivent des fichiers de succès et de temps de jeu différents à côté de la même sauvegarde.

### Ai-je besoin de Rclone ou d'un compte cloud à moi ?

Non. C'est la principale différence pratique : avec Hoard Cloud, le stockage est déjà en place dès la connexion. Si vous préférez posséder le stockage, faites tourner le serveur vous-même sur un bucket compatible S3 ou un simple dossier de votre machine.

### L'auto-hébergement envoie-t-il quoi que ce soit à Hoard ?

Non. En mode auto-hébergé il n'y a aucun compte chez nous ni aucune télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux vivent sur votre propre serveur et ne touchent jamais le nôtre. C'est tout l'intérêt de ce mode, et c'est pourquoi le serveur est le même binaire open source que celui que nous faisons tourner, pas une version allégée.

### Ludusavi est-il sûr ?

Oui. Il est open source, largement utilisé, et se contente de copier les fichiers de sauvegarde vers un dossier de backup et inversement ; il ne touche pas aux fichiers du jeu. Le seul point de vigilance vaut pour tout outil de sauvegarde : restaurer un ancien backup par-dessus une sauvegarde plus récente la remplace, vérifiez donc la date avant.

### Ludusavi fonctionne-t-il sur Steam Deck ?

Oui. Il existe une version Linux à installer en mode Bureau, et un plugin Decky pour lancer des sauvegardes depuis le mode Jeu. Ce qu'il ne fait pas seul, c'est garder le Deck et le PC fixe au même niveau : vous sauvegardez sur l'un et restaurez sur l'autre. Hoard s'en charge automatiquement, depuis un service en arrière-plan, sans rien à déclencher.

### Ludusavi peut-il sauvegarder vers Google Drive ou un autre cloud ?

Oui, via Rclone : vous configurez un remote Google Drive, Dropbox, OneDrive ou tout autre fournisseur pris en charge par Rclone, et Ludusavi y copie ses sauvegardes. Avec Hoard, aucun remote à configurer, et si vous préférez posséder le stockage, vous le pointez vers votre propre serveur.

### Ludusavi sauvegarde-t-il automatiquement ?

Pas tout seul : il sauvegarde quand vous le lancez. Vous pouvez l'automatiser avec sa ligne de commande et une tâche planifiée, ou envelopper la commande de lancement d'un jeu pour qu'il sauvegarde à sa fermeture. Hoard remarque la fermeture d'un jeu, sans rien envelopper, et sauvegarde à ce moment-là.

### Ludusavi prend-il en charge les jeux hors Steam ?

Oui. Le manifeste couvre les jeux de GOG, Epic, Xbox et d'autres launchers, ainsi que beaucoup de jeux vendus sans boutique, et vous pouvez ajouter vos propres entrées pour ce qui manque. Hoard lit le même manifeste et ajoute une analyse du disque pour les jeux qui n'y figurent pas.
`,Ca=`---
title: "Alternativa a Ludusavi: sincronizzazione cloud automatica dei salvataggi"
description: "Ludusavi è ottimo per i backup locali. Hoard aggiunge il sync automatico tra PC e Steam Deck con cronologia, sullo stesso database dei salvataggi."
order: 5
updated: 2026-10-01
---

Se cerchi un modo per fare backup e sincronizzare i tuoi salvataggi, probabilmente hai trovato **Ludusavi** — ed è eccellente. Questa guida è un confronto onesto per aiutarti a scegliere lo strumento giusto, e spiega dove si inserisce Hoard se vuoi sincronizzazione cloud automatica tra macchine.

## Cosa fa bene Ludusavi

Ludusavi è uno strumento gratuito e open source (creato da mtkennerly) per fare backup e ripristinare i salvataggi PC su Windows, macOS e Linux. Ha una GUI pulita e una CLI, trova automaticamente i salvataggi di migliaia di giochi, conserva backup locali versionati e può inviare quei backup a un cloud tuo configurando **Rclone** (Google Drive, Dropbox e molti altri). Se vuoi pieno controllo e un setup fai-da-te, Ludusavi è una scelta fantastica — e completamente gratuita.

Hoard non vuole sostituirlo. Anzi, **Hoard usa lo stesso database comunitario di posizioni su cui si basa Ludusavi** per individuare dove ogni gioco conserva i salvataggi, quindi la qualità del rilevamento è alla pari.

## In cosa Hoard è diverso

Il punto in cui quasi tutti si bloccano con qualsiasi strumento locale è la **sincronizzazione tra dispositivi**. Con Ludusavi la fai tu: programmare un backup, configurare un remoto Rclone, poi ripristinare sull'altro PC prima di giocare. Funziona, ma è manuale.

Hoard la trasforma in **sincronizzazione cloud gestita**:

- **Accedi e via.** Niente remoti Rclone, niente script. Hoard carica il salvataggio dopo che giochi e scarica l'ultima versione prima che inizi, su ogni PC del tuo account.
- **Cronologia versionata nel cloud.** Ogni backup viene conservato, quindi puoi tornare a qualsiasi salvataggio precedente — anche dopo un guasto del disco o un'installazione pulita.
- **Consapevole dei conflitti.** Hoard confronta i timestamp e conserva una copia locale di tutto ciò che sostituisce, così una sincronizzazione non distrugge mai i progressi in silenzio.
- **Sempre open source e self-hostable.** Come Ludusavi, nessun vincolo — usa Hoard Cloud o ospita il server tu stesso.

## Testa a testa

| | Ludusavi | Hoard |
|---|---|---|
| Backup locali | Sì | Sì |
| Rilevamento dei salvataggi | Manifest comunitario | Lo stesso manifest, più le librerie Steam, i processi in esecuzione e una scansione del disco |
| Spazio cloud | Il tuo, tramite Rclone | Incluso, oppure il tuo server |
| Sincronizzazione tra PC | Manuale: backup qui, ripristino là | Automatica, dopo che smetti di giocare e prima che inizi |
| Cronologia versioni | Backup locali che poti tu | Ogni versione nel cloud, deduplicata per hash del contenuto |
| Emulatori | Sì | Sì |
| Interfacce | App desktop e CLI | App desktop, CLI e overlay in gioco |
| Prezzo | Gratuito | Piano gratis da 2 GB e 3 dispositivi, Pro oltre, nessuna quota in self-hosting |
| Licenza | MIT | AGPL-3.0 |

## Quando Ludusavi è la scelta migliore

È la parte che quasi nessuna pagina di confronto include. Ludusavi è lo strumento migliore quando:

- **Giochi su un solo PC.** La sincronizzazione cloud risolve un problema che non hai. Basta un backup locale, e Ludusavi li fa molto bene.
- **Hai già un remoto Rclone di cui ti fidi.** Se il tuo spazio è configurato e funziona, il vantaggio principale di Hoard è un passaggio che hai già pagato.
- **Vuoi usarlo dalla modalità gioco di uno Steam Deck.** Ludusavi ha un plugin Decky, quindi puoi lanciare un backup senza uscire dall'interfaccia console.
- **Vuoi una licenza permissiva.** Ludusavi è MIT, Hoard è AGPL-3.0. Se hai in mente di costruirci sopra qualcosa senza pubblicare il risultato, quella differenza pesa.
- **Non vuoi niente che giri in sottofondo.** Ospitare Hoard da soli significa tenere in piedi un piccolo server da qualche parte, anche sullo stesso PC. Ludusavi è un'app che apri quando ti serve.

## Ludusavi contro GameSave Manager

L'altro nome che salta fuori accanto a Ludusavi è **GameSave Manager**, uno strumento Windows di lunga data. Risolvono lo stesso problema in modi diversi:

- **Piattaforma.** Ludusavi gira su Windows, macOS e Linux, Steam Deck compresa. GameSave Manager solo su Windows.
- **Licenza.** Ludusavi è open source (MIT). GameSave Manager è gratuito ma closed source.
- **Trovare i salvataggi.** Ludusavi legge il manifest comunitario ricavato da PCGamingWiki, circa 20.000 giochi. GameSave Manager ha un database proprio.
- **Portare i salvataggi nel cloud.** Ludusavi copia i suoi backup su un remote tramite Rclone. Il "Sync & Link" di GameSave Manager sposta una cartella di salvataggio in una cartella cloud come Dropbox o OneDrive e lascia un collegamento al suo posto, così il client cloud sincronizza la cartella attiva.

È quest'ultimo punto quello da valutare. Una cartella attiva sincronizzata da un client cloud generico è proprio la configurazione che trasforma un salvataggio scritto a metà in un salvataggio rotto su ogni PC; la [guida a Syncthing](/guides/syncthing-game-saves) spiega perché. Hoard sta dall'altra parte di quella linea: aspetta che il gioco sia chiuso, carica una versione e tiene le precedenti.

## Passare da Ludusavi a Hoard

Non c'è un importatore, ed è voluto. I passaggi:

1. **Lascia i backup di Ludusavi esattamente dove sono.** Non viene migrato né cancellato nulla. Tienili come rete di sicurezza per le prime settimane.
2. **Installa Hoard e accedi**, oppure puntalo al tuo server.
3. **Lascia che faccia la scansione.** Legge lo stesso manifest, quindi l'elenco dei giochi rilevati dovrebbe esserti familiare.
4. **Non puntare Hoard alla cartella dei backup di Ludusavi.** Traccia la cartella in cui scrive il gioco. Una cartella di backup è una copia che cambia secondo un orario e non quando giochi, e sincronizzare la copia di una copia è il modo in cui si finisce per ripristinare i progressi di ieri. Hoard prova a rilevarlo da solo — \`hoard doctor\` segnala una cartella tracciata che sembra un mirror di backup — ma è più semplice non tracciarla affatto.
5. **Gioca una volta.** All'uscita, la prima versione compare nella cronologia.
6. **Ripeti sul secondo PC.** Accedi lì e le versioni sono già pronte.

## Due dettagli da sapere

**I salvataggi di Steam stanno una cartella più in profondità di quanto sembri.** Per i giochi Steam, Hoard traccia \`<AppID>/remote/\` dentro \`userdata\`, non la cartella superiore. Quella superiore contiene anche \`remotecache.vdf\` e i file di obiettivi e tempo di gioco, che legittimamente cambiano da macchina a macchina. Se sincronizzi la cartella superiore, ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso. È il motivo più comune per cui un setup artigianale tra Steam Deck e desktop finisce per combattere contro sé stesso.

**Le versioni costano poco.** Gli snapshot sono archiviati per hash del contenuto, quindi un file che non cambia viene salvato una volta sola. Dieci versioni di un salvataggio da 2 GB occupano circa 2 GB, non 20 — ed è questo che rende pratico conservare tutta la cronologia invece di potarla.

## Cosa significa davvero il self-hosting

È il punto su cui quasi tutti i confronti sbagliano riguardo a Hoard, quindi vale la pena essere precisi. Ci sono due modi di usarlo, e sono davvero diversi:

- **Hoard Cloud** è l'opzione gestita: accedi e i tuoi salvataggi stanno sui nostri server, nell'UE.
- **Il self-hosting è interamente tuo.** Fai girare \`hoard-server\` sul tuo PC o sul tuo NAS, e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non possiamo vedere un salvataggio, il nome di un gioco o un indirizzo email, per il semplice motivo che niente di tutto ciò ci arriva. Se Hoard Cloud sparisse domani, un'installazione self-hosted continuerebbe uguale.

Stesso programma, stesso rilevamento, stessa cronologia delle versioni. L'unica cosa che cambia è di chi è lo spazio di archiviazione.

## Quale scegliere?

- Scegli **Ludusavi** se vuoi uno strumento di backup gratuito e locale e non ti dispiace montare il tuo cloud con Rclone.
- Scegli **Hoard** se vuoi che backup *e* sincronizzazione tra PC funzionino da soli, con una cronologia cloud versionata, mantenendo l'opzione del self-hosting.

Molti iniziano con Ludusavi per i backup locali e passano a Hoard quando giocano agli stessi giochi su più di una macchina. Se è il tuo caso, vedi [come sincronizzare i salvataggi tra PC](/guides/sync-game-saves-across-pcs) o semplicemente [scarica Hoard](/download) e accedi. Per il quadro completo c'è un [confronto di tutti gli strumenti di sincronizzazione](/guides/game-save-sync-comparison).

<!-- faq -->

## Domande frequenti

### Posso usare Ludusavi e Hoard insieme?

Sì. Leggono le stesse posizioni e nessuno dei due tiene i file bloccati. Molti tengono Ludusavi per i backup di archivio locali e lasciano a Hoard la sincronizzazione tra macchine. L'unica regola è non puntare uno dei due alla cartella di backup dell'altro.

### Hoard importa i miei backup di Ludusavi?

No, ed è deliberato. Una cartella di backup è una copia che cambia secondo il proprio orario: tracciarla sincronizzerebbe un mirror vecchio invece del salvataggio reale. Hoard traccia la cartella in cui scrive il gioco e avvia la propria cronologia dalla sessione successiva. Tieni l'archivio di Ludusavi come rete di sicurezza.

### Hoard è gratuito?

Hoard Cloud ha un piano gratuito con 2 GB di spazio e 3 dispositivi, che copre la maggior parte delle collezioni; Pro alza entrambi. Ospitare il server per conto proprio è gratis e non ha alcuna quota. Tutto è open source sotto AGPL-3.0.

### Hoard funziona su Steam Deck?

Sì, su Steam Deck e su qualsiasi desktop Linux, oltre che su Windows e macOS. Il Deck è proprio il caso che richiede il dettaglio su \`remote/\` qui sopra, perché un Deck e un desktop scrivono file di obiettivi e tempo di gioco diversi accanto allo stesso salvataggio.

### Mi serve Rclone o un account cloud mio?

No. È la differenza pratica principale: con Hoard Cloud lo spazio è già pronto quando accedi. Se preferisci essere padrone dello spazio, fai girare il server tu stesso su un bucket compatibile con S3 o una normale cartella della tua macchina.

### Il self-hosting manda qualcosa a Hoard?

No. In modalità self-hosted non c'è alcun account con noi né telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log stanno sul tuo server e non toccano mai il nostro. È tutto il senso di questa modalità, ed è il motivo per cui il server è lo stesso binario open source che usiamo noi e non una versione ridotta.

### Ludusavi è sicuro?

Sì. È open source, molto usato, e non fa altro che copiare i file di salvataggio in una cartella di backup e viceversa; non tocca i file del gioco. L'unica cosa a cui fare attenzione vale per qualsiasi strumento di backup: ripristinare un backup vecchio sopra un salvataggio più recente lo sostituisce, quindi controlla la data prima di ripristinare.

### Ludusavi funziona su Steam Deck?

Sì. C'è una versione Linux da installare in modalità Desktop e un plugin Decky per avviare i backup dalla modalità Gioco. Quello che non fa da solo è tenere allineati la Deck e il fisso: fai il backup su uno e ripristini sull'altro. Hoard se ne occupa in automatico, da un servizio in background, senza nulla da avviare.

### Ludusavi può fare il backup su Google Drive o un altro cloud?

Sì, tramite Rclone: configuri un remote per Google Drive, Dropbox, OneDrive o qualsiasi altro provider supportato da Rclone, e Ludusavi ci copia i suoi backup. Con Hoard non c'è nessun remote da configurare, e se preferisci che lo spazio sia tuo lo punti al tuo server.

### Ludusavi fa il backup in automatico?

Da solo no: fa il backup quando lo avvii. Puoi automatizzarlo con la riga di comando e un'attività pianificata, oppure avvolgere il comando di avvio di un gioco perché faccia il backup all'uscita. Hoard si accorge da solo quando un gioco si chiude, senza avvolgere nulla, e fa il backup in quel momento.

### Ludusavi supporta i giochi non Steam?

Sì. Il manifest copre giochi di GOG, Epic, Xbox e altri launcher, oltre a molti venduti senza store, e puoi aggiungere voci personalizzate per ciò che manca. Hoard legge lo stesso manifest e aggiunge una scansione del disco per i giochi che non vi compaiono.
`,wa=`---
title: "Ludusavi の代替：セーブデータの自動クラウド同期"
description: "Ludusaviはローカルバックアップに最適。HoardはPCとSteam Deck間の自動同期とバージョン履歴を、同じセーブ場所データベースの上に追加します。"
order: 5
updated: 2026-10-01
---

セーブデータをバックアップして同期する方法を探しているなら、おそらく **Ludusavi** にたどり着いたはずです――そして優れたツールです。このガイドは適切なツールを選べるよう正直に比較し、端末間での自動クラウド同期が欲しい場合に Hoard がどこに位置づくかを説明します。

## Ludusavi の優れている点

Ludusavi は Windows、macOS、Linux で PC のセーブをバックアップ・復元する無料のオープンソースツール（mtkennerly 作）です。すっきりした GUI と CLI を備え、数千のゲームのセーブを自動で見つけ、世代管理されたローカルバックアップを保持し、**Rclone** を設定すれば自分のクラウド（Google Drive、Dropbox など）へバックアップを送れます。完全な制御と自前のセットアップが欲しいなら、Ludusavi は素晴らしい選択肢で、しかも完全に無料です。

Hoard はそれを置き換えるためのものではありません。実際、**Hoard は Ludusavi が依拠しているのと同じコミュニティのセーブ位置データベース** を使って各ゲームのセーブ場所を特定するため、検出の品質は同等です。

## Hoard が異なる点

ローカル中心のツールで多くの人がぶつかる壁が、**端末間の同期** です。Ludusavi では自分で行います。バックアップをスケジュールし、Rclone のリモートを設定し、プレイ前にもう一方の PC で復元する。動作はしますが、手作業です。

Hoard はこれを **マネージドなクラウド同期** に変えます。

- **サインインするだけ。** Rclone のリモートもスクリプトも不要。Hoard はプレイ後にセーブをアップロードし、開始前に最新版をダウンロードします。アカウント内のすべての PC で行われます。
- **クラウド上の世代履歴。** すべてのバックアップが保持されるので、以前のどのセーブにも巻き戻せます――ディスク故障やクリーンインストールの後でも。
- **競合を認識。** Hoard はタイムスタンプを比較し、置き換えるものすべてのローカルコピーを保持するため、同期が黙って進行を壊すことはありません。
- **引き続きオープンソースでセルフホスト可能。** Ludusavi と同様にロックインはありません――Hoard Cloud を使うか、サーバーを自分でホストできます。

## 一覧で比較

| | Ludusavi | Hoard |
|---|---|---|
| ローカルバックアップ | あり | あり |
| セーブの検出 | コミュニティのマニフェスト | 同じマニフェストに加え、Steam ライブラリ、実行中プロセス、ファイルシステムの走査 |
| クラウドの保存先 | 自前、Rclone 経由 | 同梱、または自分のサーバー |
| PC 間の同期 | 手動：こちらでバックアップ、あちらで復元 | 自動：プレイ終了後と開始前 |
| 世代履歴 | 自分で整理するローカルバックアップ | すべての世代をクラウドに保持し、内容ハッシュで重複排除 |
| エミュレーター | 対応 | 対応 |
| インターフェース | デスクトップアプリと CLI | デスクトップアプリ、CLI、ゲーム内オーバーレイ |
| 価格 | 無料 | 無料枠は 2 GB・3 台、それ以上は Pro、セルフホストなら上限なし |
| ライセンス | MIT | AGPL-3.0 |

## Ludusavi のほうが向いている場合

ほとんどの比較ページが省く部分です。次の場合は Ludusavi のほうが適しています。

- **PC 1 台でしか遊ばない。** クラウド同期は存在しない問題を解決することになります。ローカルバックアップで十分で、Ludusavi はそれが得意です。
- **信頼している Rclone リモートがすでにある。** 保存先が設定済みで動いているなら、Hoard の主な利点はすでに済ませた手間の肩代わりです。
- **Steam Deck のゲームモードから使いたい。** Ludusavi には Decky プラグインがあり、コンソール画面を離れずにバックアップを実行できます。
- **緩やかなライセンスが必要。** Ludusavi は MIT、Hoard は AGPL-3.0 です。上に何かを作って結果を公開しないつもりなら、この違いは大きく効きます。
- **常駐するものを増やしたくない。** Hoard のセルフホストは、同じ PC 上であっても小さなサーバーを動かし続けることを意味します。Ludusavi は必要なときに開くアプリです。

## Ludusavi と GameSave Manager の比較

Ludusavi と並んでよく名前が挙がるのが、長く使われてきた Windows 用ツールの **GameSave Manager** です。同じ問題を別のやり方で解決しています。

- **対応プラットフォーム:** Ludusavi は Windows、macOS、Linux（Steam Deck を含む）で動きます。GameSave Manager は Windows のみです。
- **ライセンス:** Ludusavi はオープンソース（MIT）。GameSave Manager は無料ですがクローズドソースです。
- **セーブの検出:** Ludusavi は PCGamingWiki を元にしたコミュニティのマニフェスト（約 2 万タイトル）を読みます。GameSave Manager は独自のデータベースを持っています。
- **クラウドへの送り方:** Ludusavi はバックアップを Rclone 経由でリモートにコピーします。GameSave Manager の「Sync & Link」はセーブフォルダーを Dropbox や OneDrive などのクラウドフォルダーに移し、元の場所にリンクを残すため、クラウドクライアントが使用中のフォルダーをそのまま同期します。

考えるべきは最後の点です。汎用のクラウドクライアントが使用中のフォルダーを同期する構成は、書きかけのセーブをすべての PC で壊れたセーブに変えてしまう典型例で、その理由は [Syncthing のガイド](/guides/syncthing-game-saves)で説明しています。Hoard はその反対側にいます。ゲームが終了するのを待ってからバージョンをアップロードし、古いバージョンも残します。

## Ludusavi から Hoard へ移る

インポート機能はなく、それは意図的です。手順は次のとおりです。

1. **Ludusavi のバックアップはそのままの場所に残す。** 何も移行せず、何も削除しません。最初の数週間は安全網として保管してください。
2. **Hoard をインストールしてサインインする。** あるいは自分のサーバーを指定します。
3. **スキャンさせる。** 同じマニフェストを読むので、検出されるゲームの一覧は見覚えのあるものになるはずです。
4. **Hoard を Ludusavi のバックアップフォルダーに向けない。** ゲーム自身が書き込むフォルダーを追跡してください。バックアップフォルダーはプレイ時ではなくスケジュールで変わる複製であり、複製の複製を同期すると昨日の進行を復元することになります。Hoard は自力で気づこうとし、\`hoard doctor\` がバックアップの写しに見える追跡フォルダーを警告しますが、最初から追跡しないほうが簡単です。
5. **一度プレイする。** 終了すると最初の世代が履歴に現れます。
6. **2 台目の PC でも同じことを。** サインインすれば、世代はもうそこにあります。

## 知っておく価値のある 2 点

**Steam のセーブは思っているより 1 階層深い。** Steam のゲームでは、Hoard は \`userdata\` の中の \`<AppID>/remote/\` を追跡し、その上のフォルダーは追跡しません。上のフォルダーには \`remotecache.vdf\` や実績・プレイ時間のファイルもあり、これらはマシンごとに違って当然です。上を同期すると、セーブが動いていなくても起動のたびに競合に見えます。Steam Deck とデスクトップの自作構成が自分自身と喧嘩する、いちばん多い原因がこれです。

**世代は安い。** スナップショットは内容ハッシュで保存されるため、変わっていないファイルは一度だけ保存されます。2 GB のセーブの 10 世代は約 20 GB ではなく約 2 GB です。履歴を間引かずに丸ごと残せるのはこのためです。

## セルフホストの本当の意味

Hoard について多くの比較が誤解している点なので、正確に書きます。動かし方は 2 通りあり、両者は本当に別物です。

- **Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに保存されます。
- **セルフホストは完全にあなたのものです。** 自分の PC や NAS で \`hoard-server\` を動かし、セーブは自分のマシンから自分のディスクへ移ります。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。セーブもゲーム名もメールアドレスも見えません。そもそも届かないからです。仮に明日 Hoard Cloud が消えても、セルフホスト構成はそのまま動き続けます。

同じプログラム、同じ検出、同じ世代履歴。変わるのは保存先が誰のものかだけです。

## どちらを選ぶべきか

- 無料でローカル中心のバックアップツールが欲しく、Rclone で自分のクラウドを組むのが苦でないなら **Ludusavi** を選びましょう。
- バックアップ *と* PC 間の同期が手間なく動き、世代付きのクラウド履歴を持ちつつ、セルフホストの選択肢も残したいなら **Hoard** を選びましょう。

多くの人はローカルバックアップに Ludusavi で始め、複数のマシンで同じゲームをプレイするようになると Hoard に移行します。あなたがそうなら、[PC 間でセーブを同期する方法](/guides/sync-game-saves-across-pcs) をご覧いただくか、[Hoard をダウンロード](/download) してサインインしてください。全体像は [セーブ同期ツールの比較](/guides/game-save-sync-comparison) にまとめてあります。

<!-- faq -->

## よくある質問

### Ludusavi と Hoard を同時に使えますか？

はい。両者は同じセーブ位置を読み、どちらもファイルを掴んだままにしません。ローカルの保管用バックアップは Ludusavi に任せ、マシン間の同期は Hoard に任せている人は多くいます。唯一の注意は、一方をもう一方のバックアップフォルダーに向けないことです。

### Hoard は Ludusavi のバックアップを取り込みますか？

いいえ。これは意図的です。バックアップフォルダーは独自のスケジュールで変わる複製なので、追跡すると実際のセーブではなく古い写しを同期してしまいます。Hoard はゲームが書き込むフォルダーを追跡し、次のセッションから独自の履歴を始めます。Ludusavi の保管分は安全網として残してください。

### Hoard は無料ですか？

Hoard Cloud には 2 GB・3 台の無料枠があり、多くのセーブ環境はこれで足ります。Pro は両方を引き上げます。サーバーを自分でホストする場合は無料で、容量制限もありません。すべて AGPL-3.0 のオープンソースです。

### Steam Deck で動きますか？

はい。Steam Deck と任意の Linux デスクトップ、加えて Windows と macOS で動きます。Deck はまさに上の \`remote/\` の話が効く場面です。Deck とデスクトップは同じセーブの隣に、異なる実績・プレイ時間のファイルを書くからです。

### Rclone や自分のクラウドアカウントは必要ですか？

いいえ。そこが実用上の最大の違いです。Hoard Cloud ならサインインした時点で保存先が用意されています。保存先を自分で所有したい場合は、S3 互換のバケットか自分のマシンの普通のフォルダーを指定してサーバーを動かしてください。

### セルフホストは Hoard に何かを送信しますか？

いいえ。セルフホストでは当方のアカウントも当方へのテレメトリもありません。セーブもユーザーもログも自分のサーバーの中にとどまり、当方のサーバーには一切触れません。それがこのモードの目的であり、サーバーが機能を削った別物ではなく、当方自身が動かしているものと同じオープンソースのバイナリである理由です。

### Ludusavi は安全ですか？

はい。オープンソースで広く使われており、セーブファイルをバックアップフォルダーにコピーし、戻すだけで、ゲームファイルには触れません。注意点はどのバックアップツールでも同じで、古いバックアップを新しいセーブの上に復元すると置き換わってしまうため、復元前に日付を確認してください。

### Ludusavi は Steam Deck で使えますか？

はい。デスクトップモードでインストールできる Linux 版と、ゲームモードからバックアップを実行できる Decky プラグインがあります。ただし、Deck とデスクトップを単独で同じ状態に保つことはしません。片方でバックアップし、もう片方で復元する必要があります。Hoard はその部分をバックグラウンドサービスで自動的に行うので、何も操作する必要がありません。

### Ludusavi は Google Drive などのクラウドにバックアップできますか？

はい、Rclone 経由で可能です。Google Drive、Dropbox、OneDrive など Rclone が対応するプロバイダーのリモートを設定すれば、Ludusavi がそこへバックアップをコピーします。Hoard では設定するリモートはなく、ストレージを自分で持ちたい場合は自分のサーバーを指定します。

### Ludusavi は自動でバックアップしますか？

単独ではしません。実行したときにバックアップします。コマンドラインとスケジュールタスクで自動化したり、ゲームの起動コマンドをラップして終了時にバックアップさせたりできます。Hoard はラップなしでゲームの終了を検知し、そのタイミングでバックアップします。

### Ludusavi は Steam 以外のゲームに対応していますか？

はい。マニフェストは GOG、Epic、Xbox などのランチャーのゲームに加え、ストアを介さずに販売されている多くのゲームもカバーしており、足りないものは独自のエントリーを追加できます。Hoard は同じマニフェストを読み、載っていないゲームのためにファイルシステムのスキャンを加えています。
`,za=`---
title: "Alternativa ao Ludusavi: sincronização automática de saves na nuvem"
description: "O Ludusavi é ótimo para backups locais. O Hoard junta sincronização automática entre PC e Steam Deck com histórico, sobre a mesma base de dados de saves."
order: 5
updated: 2026-10-01
---

Se procuras uma forma de fazer backup e sincronizar os teus saves, é provável que tenhas encontrado o **Ludusavi** — e é excelente. Este guia é uma comparação honesta para te ajudar a escolher a ferramenta certa, e explica onde o Hoard se encaixa se quiseres sincronização na nuvem automática entre máquinas.

## O que o Ludusavi faz bem

O Ludusavi é uma ferramenta gratuita e open source (criada por mtkennerly) para fazer backup e restaurar saves de PC em Windows, macOS e Linux. Tem uma GUI limpa e uma CLI, encontra automaticamente os saves de milhares de jogos, guarda backups locais versionados e pode enviar esses backups para uma nuvem tua configurando o **Rclone** (Google Drive, Dropbox e muitos outros). Se queres controlo total e uma configuração faz-tu-mesmo, o Ludusavi é uma escolha fantástica — e completamente gratuita.

O Hoard não vem substituir isso. Na verdade, **o Hoard usa a mesma base de dados comunitária de localizações em que o Ludusavi se apoia** para localizar onde cada jogo guarda os saves, por isso a qualidade da deteção está ao mesmo nível.

## Em que o Hoard é diferente

O ponto onde a maioria esbarra com qualquer ferramenta local é a **sincronização entre dispositivos**. Com o Ludusavi fá-lo tu: agendar um backup, configurar um remoto Rclone, e depois restaurar no outro PC antes de jogar. Funciona, mas é manual.

O Hoard transforma isso em **sincronização na nuvem gerida**:

- **Inicia sessão e pronto.** Sem remotos Rclone, sem scripts. O Hoard envia o teu save depois de jogares e descarrega a versão mais recente antes de começares, em cada PC da tua conta.
- **Histórico versionado na nuvem.** Cada backup é guardado, por isso podes voltar a qualquer save anterior — mesmo depois de uma falha de disco ou de uma instalação limpa.
- **Tem em conta os conflitos.** O Hoard compara os timestamps e guarda uma cópia local de tudo o que substitui, por isso uma sincronização nunca destrói progresso em silêncio.
- **Continua open source e self-hostable.** Como o Ludusavi, não há aprisionamento — usa o Hoard Cloud ou aloja o servidor tu mesmo.

## Frente a frente

| | Ludusavi | Hoard |
|---|---|---|
| Backups locais | Sim | Sim |
| Deteção de saves | Manifesto comunitário | O mesmo manifesto, mais bibliotecas Steam, processos em execução e uma varredura do disco |
| Armazenamento na nuvem | O teu, via Rclone | Incluído, ou o teu próprio servidor |
| Sincronização entre PCs | Manual: backup aqui, restauro ali | Automática, depois de jogares e antes de começares |
| Histórico de versões | Backups locais que limpas tu | Todas as versões na nuvem, deduplicadas por hash de conteúdo |
| Emuladores | Sim | Sim |
| Interfaces | App de ambiente de trabalho e CLI | App de ambiente de trabalho, CLI e overlay dentro do jogo |
| Preço | Gratuito | Plano gratuito de 2 GB e 3 dispositivos, Pro acima disso, sem qualquer quota em self-hosting |
| Licença | MIT | AGPL-3.0 |

## Quando o Ludusavi é a melhor escolha

Esta é a parte que quase nenhuma página de comparação inclui. O Ludusavi é a melhor ferramenta quando:

- **Só jogas num PC.** A sincronização na nuvem resolve um problema que não tens. Um backup local chega, e o Ludusavi faz backups locais muito bem.
- **Já tens um remoto Rclone em que confias.** Se o teu armazenamento está montado e a funcionar, a principal vantagem do Hoard é um passo de configuração que já pagaste.
- **Queres usá-lo a partir do modo de jogo de uma Steam Deck.** O Ludusavi tem um plugin Decky, por isso podes lançar um backup sem sair da interface de consola.
- **Queres uma licença permissiva.** O Ludusavi é MIT e o Hoard é AGPL-3.0. Se pensas construir algo por cima e não publicar o resultado, essa diferença pesa.
- **Não queres nada a correr em pano de fundo.** Alojar o Hoard tu mesmo implica manter um pequeno servidor de pé, mesmo que seja no próprio PC. O Ludusavi é uma aplicação que abres quando precisas.

## Ludusavi vs GameSave Manager

O outro nome que aparece ao lado do Ludusavi é o **GameSave Manager**, uma ferramenta veterana para Windows. Os dois resolvem o mesmo problema de formas diferentes:

- **Plataforma.** O Ludusavi corre em Windows, macOS e Linux, Steam Deck incluída. O GameSave Manager só em Windows.
- **Licença.** O Ludusavi é de código aberto (MIT). O GameSave Manager é gratuito, mas de código fechado.
- **Encontrar os saves.** O Ludusavi lê o manifesto comunitário construído a partir da PCGamingWiki, cerca de 20 000 jogos. O GameSave Manager traz a sua própria base de dados.
- **Levar os saves para a nuvem.** O Ludusavi copia os seus backups para um remoto através do Rclone. O «Sync & Link» do GameSave Manager move a pasta de saves para uma pasta na nuvem, como o Dropbox ou o OneDrive, e deixa um link no seu lugar, para que o cliente da nuvem sincronize a pasta ativa.

É este último ponto que convém pesar. Uma pasta ativa sincronizada por um cliente de nuvem genérico é exatamente a configuração que transforma um save escrito a meio num save estragado em todos os teus PCs, e o [guia do Syncthing](/guides/syncthing-game-saves) explica porquê. O Hoard está do outro lado dessa linha: espera que o jogo feche, envia uma versão e guarda as anteriores.

## Passar do Ludusavi para o Hoard

Não há importador, e é de propósito. Os passos:

1. **Deixa os teus backups do Ludusavi exatamente onde estão.** Nada é migrado nem apagado. Guarda-os como rede de segurança nas primeiras semanas.
2. **Instala o Hoard e inicia sessão**, ou aponta-o ao teu próprio servidor.
3. **Deixa-o analisar.** Lê o mesmo manifesto, por isso a lista de jogos detetados deve ser-te familiar.
4. **Não apontes o Hoard para a pasta de backups do Ludusavi.** Segue a pasta onde o jogo escreve. Uma pasta de backups é uma cópia que muda por horário e não quando jogas, e sincronizar a cópia de uma cópia é como se acaba a restaurar o progresso de ontem. O Hoard tenta detetá-lo sozinho — \`hoard doctor\` assinala uma pasta seguida que parece um espelho de backups — mas é mais simples nunca a seguir.
5. **Joga uma vez.** Ao sair, a primeira versão aparece no histórico.
6. **Repete no segundo PC.** Inicias sessão e as versões já lá estão.

## Dois detalhes que vale a pena saber

**Os saves da Steam vivem uma pasta mais abaixo do que parece.** Nos jogos da Steam, o Hoard segue \`<AppID>/remote/\` dentro de \`userdata\`, não a pasta acima. A pasta acima guarda também \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo, que são legitimamente diferentes em cada máquina. Se sincronizares a pasta acima, cada arranque parece um conflito mesmo sem nenhum save se ter mexido. É o motivo mais comum para uma montagem caseira entre Steam Deck e desktop acabar a lutar contra si própria.

**As versões são baratas.** Os snapshots são guardados por hash de conteúdo, por isso um ficheiro que não muda é guardado uma só vez. Dez versões de um save de 2 GB ocupam cerca de 2 GB, não 20 — e é isso que torna prático manter o histórico inteiro em vez de o ir cortando.

## O que self-hosting quer mesmo dizer

É o ponto em que quase todas as comparações se enganam sobre o Hoard, por isso convém ser exato. Há duas formas de o usar, e são genuinamente diferentes:

- **O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.
- **O self-hosting é inteiramente teu.** Corres o \`hoard-server\` no teu PC ou no teu NAS, e os teus saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não conseguimos ver um save, o nome de um jogo ou um endereço de email, pela simples razão de que nada disso nos chega. Se o Hoard Cloud desaparecesse amanhã, uma instalação self-hosted continuaria igual.

O mesmo programa, a mesma deteção, o mesmo histórico de versões. A única coisa que muda é de quem é o armazenamento.

## Qual deves escolher?

- Escolhe o **Ludusavi** se queres uma ferramenta de backup gratuita e local e não te importas de montar a tua própria nuvem com o Rclone.
- Escolhe o **Hoard** se queres que o backup *e* a sincronização entre PCs simplesmente funcionem, com um histórico na nuvem versionado, mantendo a opção de self-hosting.

Muita gente começa com o Ludusavi para backups locais e passa para o Hoard quando joga os mesmos jogos em mais de uma máquina. Se é o teu caso, vê [como sincronizar saves entre PCs](/guides/sync-game-saves-across-pcs) ou simplesmente [descarrega o Hoard](/download) e inicia sessão. Para o panorama completo, há uma [comparação de todas as ferramentas de sincronização](/guides/game-save-sync-comparison).

<!-- faq -->

## Perguntas frequentes

### Posso usar o Ludusavi e o Hoard ao mesmo tempo?

Sim. Leem as mesmas localizações e nenhum dos dois bloqueia os ficheiros. Muita gente mantém o Ludusavi para backups de arquivo locais e deixa o Hoard tratar da sincronização entre máquinas. A única regra é não apontar uma ferramenta para a pasta de backups da outra.

### O Hoard importa os meus backups do Ludusavi?

Não, e é deliberado. Uma pasta de backups é uma cópia que muda segundo o seu próprio horário, por isso segui-la sincronizaria um espelho desatualizado em vez do teu save real. O Hoard segue a pasta onde o jogo escreve e começa o seu próprio histórico a partir da tua sessão seguinte. Guarda o arquivo do Ludusavi como rede de segurança.

### O Hoard é gratuito?

O Hoard Cloud tem um plano gratuito com 2 GB de armazenamento e 3 dispositivos, o que cobre a maioria das coleções; o Pro sobe ambos. Alojar o servidor tu mesmo é gratuito e não tem quota nenhuma. Tudo é open source sob AGPL-3.0.

### O Hoard funciona na Steam Deck?

Sim, na Steam Deck e em qualquer ambiente de trabalho Linux, além de Windows e macOS. A Deck é exatamente o caso que precisa do detalhe do \`remote/\` acima, porque uma Deck e um desktop escrevem ficheiros de proezas e tempo de jogo diferentes ao lado do mesmo save.

### Preciso de Rclone ou de uma conta de nuvem minha?

Não. É essa a principal diferença prática: com o Hoard Cloud o armazenamento já está pronto quando inicias sessão. Se preferes ser dono do armazenamento, corre o servidor tu mesmo contra um bucket compatível com S3 ou uma pasta normal da tua máquina.

### O self-hosting envia alguma coisa para o Hoard?

Não. Em modo self-hosted não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus registos vivem no teu próprio servidor e nunca tocam no nosso. É esse o sentido do modo, e é por isso que o servidor é o mesmo binário open source que nós corremos e não uma versão reduzida.

### O Ludusavi é seguro?

Sim. É de código aberto, muito usado, e limita-se a copiar os ficheiros de save para uma pasta de backup e de volta; não mexe nos ficheiros do jogo. O único cuidado vale para qualquer ferramenta de backup: restaurar um backup antigo por cima de um save mais recente substitui-o, por isso confirma a data antes de restaurar.

### O Ludusavi funciona na Steam Deck?

Sim. Há uma versão para Linux que se instala no modo Ambiente de Trabalho e um plugin Decky para lançar backups a partir do modo Jogo. O que não faz sozinho é manter a Deck e o PC em sintonia: fazes o backup num e restauras no outro. O Hoard trata dessa parte automaticamente, a partir de um serviço em segundo plano, sem nada para lançar.

### O Ludusavi consegue fazer backup para o Google Drive ou outra nuvem?

Sim, através do Rclone: configuras um remoto do Google Drive, Dropbox, OneDrive ou outro fornecedor suportado pelo Rclone, e o Ludusavi copia para lá os seus backups. Com o Hoard não há remoto para configurar e, se preferires que o armazenamento seja teu, apontas para o teu próprio servidor.

### O Ludusavi faz backup automaticamente?

Sozinho não: faz backup quando o corres. Podes automatizá-lo com a linha de comandos e uma tarefa agendada, ou envolver o comando de arranque de um jogo para que faça backup quando o jogo fecha. O Hoard repara quando um jogo fecha, sem envolver nada, e faz o backup nesse momento.

### O Ludusavi suporta jogos fora do Steam?

Sim. O manifesto cobre jogos da GOG, Epic, Xbox e outros launchers, além de muitos vendidos sem loja, e podes adicionar entradas próprias para o que faltar. O Hoard lê o mesmo manifesto e junta uma análise do disco para os jogos que lá não estão.
`,Aa=`---
title: "Ludusavi 替代方案：游戏存档的自动云同步"
description: "Ludusavi 擅长本地备份。Hoard 基于同一份存档位置数据库，增加了 PC 与 Steam Deck 之间的自动同步和版本历史。"
order: 5
updated: 2026-10-01
---

如果你在寻找备份和同步游戏存档的方法，那你很可能已经找到了 **Ludusavi**——它非常出色。本指南是一份诚实的对比，帮助你选对工具，并说明当你需要跨机器自动云同步时，Hoard 的定位在哪里。

## Ludusavi 的优点

Ludusavi 是一款免费的开源工具（由 mtkennerly 开发），可在 Windows、macOS 和 Linux 上备份与还原 PC 游戏存档。它有简洁的图形界面和命令行，能自动找到数千款游戏的存档，保留带版本的本地备份，并可通过配置 **Rclone** 把这些备份推送到你自己的云端（Google Drive、Dropbox 等）。如果你想要完全掌控和自己动手的方案，Ludusavi 是绝佳选择——而且完全免费。

Hoard 并非来取代它。事实上，**Hoard 使用与 Ludusavi 所依赖的相同的社区存档位置数据库**来定位每款游戏存档的位置，因此检测质量不相上下。

## Hoard 的不同之处

大多数人在任何以本地为主的工具上都会遇到的瓶颈，是**跨设备同步**。用 Ludusavi 时你得自己来：安排备份、配置 Rclone 远端，然后在玩之前在另一台 PC 上还原。这能行，但是手动的。

Hoard 把它变成**托管式云同步**：

- **登录即用。** 无需 Rclone 远端，无需脚本。Hoard 会在你玩完后上传存档，并在你开始前下载最新版本，覆盖你账号下的每台 PC。
- **云端版本历史。** 每个备份都会保留，因此你可以回退到任意较早的存档——即使在磁盘故障或全新安装之后。
- **冲突感知。** Hoard 会比较时间戳，并为它替换的一切保留本地副本，因此同步绝不会悄无声息地破坏进度。
- **依然开源且可自托管。** 与 Ludusavi 一样，没有锁定——使用 Hoard Cloud，或自己托管服务器。

## 逐项对比

| | Ludusavi | Hoard |
|---|---|---|
| 本地备份 | 有 | 有 |
| 存档检测 | 社区清单 | 同一份清单，另加 Steam 库、运行中的进程与文件系统扫描 |
| 云端存储 | 自备，通过 Rclone | 内置，或你自己的服务器 |
| 多台 PC 同步 | 手动：这边备份，那边还原 | 自动：玩完之后、开始之前 |
| 版本历史 | 需要你自己清理的本地备份 | 每个版本都留在云端，按内容哈希去重 |
| 模拟器 | 支持 | 支持 |
| 界面 | 桌面应用与命令行 | 桌面应用、命令行与游戏内浮层 |
| 价格 | 免费 | 免费额度 2 GB、3 台设备，超出走 Pro，自托管则完全没有配额 |
| 许可证 | MIT | AGPL-3.0 |

## 什么时候 Ludusavi 更合适

这是几乎所有对比页面都会略过的部分。以下情况 Ludusavi 是更好的工具：

- **你只在一台 PC 上玩。** 云同步解决的是你没有的问题。本地备份就够了，而 Ludusavi 的本地备份做得很好。
- **你已经有一个用着放心的 Rclone 远端。** 如果存储已经配好并正常运行，Hoard 的主要优势正是你已经付出过的那一步。
- **你想在 Steam Deck 的游戏模式里用。** Ludusavi 有 Decky 插件，不必离开主机界面就能触发备份。
- **你需要宽松的许可证。** Ludusavi 是 MIT，Hoard 是 AGPL-3.0。如果你打算在其之上做东西且不公开成果，这个差别很关键。
- **你不想有东西常驻运行。** 自托管 Hoard 意味着要让一个小服务器一直开着，哪怕就在同一台 PC 上。Ludusavi 是你需要时才打开的应用。

## Ludusavi 与 GameSave Manager 对比

和 Ludusavi 一起常被提到的另一个名字是 **GameSave Manager**，一款历史悠久的 Windows 工具。两者用不同的方式解决同一个问题：

- **平台：**Ludusavi 支持 Windows、macOS 和 Linux（包括 Steam Deck）。GameSave Manager 只支持 Windows。
- **许可证：**Ludusavi 是开源的（MIT）。GameSave Manager 免费但闭源。
- **查找存档：**Ludusavi 读取基于 PCGamingWiki 的社区清单，约 2 万款游戏。GameSave Manager 自带数据库。
- **把存档送上云端：**Ludusavi 通过 Rclone 把备份复制到远程存储。GameSave Manager 的“Sync & Link”会把存档文件夹移进 Dropbox 或 OneDrive 等云盘文件夹，并在原位置留下链接，于是云盘客户端同步的是正在使用的文件夹。

最后这一点值得掂量。由通用云盘客户端同步正在使用的文件夹，正是会把一个写到一半的存档变成所有 PC 上都损坏的存档的那种配置，原因见 [Syncthing 指南](/guides/syncthing-game-saves)。Hoard 站在这条线的另一边：它等游戏关闭后才上传一个版本，并保留旧版本。

## 从 Ludusavi 迁到 Hoard

没有导入功能，这是有意为之。步骤如下：

1. **让 Ludusavi 的备份原地不动。** 不迁移也不删除任何东西。头几周把它们留作安全网。
2. **安装 Hoard 并登录**，或把它指向你自己的服务器。
3. **让它扫描。** 它读取同一份清单，因此检测出的游戏列表应该很眼熟。
4. **不要把 Hoard 指向 Ludusavi 的备份文件夹。** 请追踪游戏本身写入的那个文件夹。备份文件夹是一份按计划变化、而非随你游玩变化的副本，同步副本的副本正是最终还原到昨天进度的原因。Hoard 会尝试自行识别——\`hoard doctor\` 会对看起来像备份镜像的被追踪文件夹发出提示——但更省事的办法是从一开始就别追踪它。
5. **先玩一次。** 退出时，第一个版本会出现在历史里。
6. **在第二台 PC 上重复一遍。** 登录之后，版本已经在那里等着了。

## 两个值得知道的细节

**Steam 存档比你以为的深一层。** 对 Steam 游戏，Hoard 追踪 \`userdata\` 里的 \`<AppID>/remote/\`，而不是它上一层的文件夹。上一层还放着 \`remotecache.vdf\` 以及成就和游戏时长文件，这些在不同机器上本就应该不同。同步上一层，每次启动都像是冲突，尽管没有任何存档动过。这正是 Steam Deck 与台式机的手工方案最终自己跟自己打架的最常见原因。

**版本很便宜。** 快照按内容哈希存储，未改动的文件只存一份。一个 2 GB 存档的十个版本大约占 2 GB，而不是 20 GB——正因如此，保留完整历史才是可行的，不必不断修剪。

## 自托管到底意味着什么

这是多数对比在 Hoard 上弄错的地方，所以说得精确些。它有两种运行方式，而且两者确实不同：

- **Hoard Cloud** 是托管方案：你登录，存档保存在我们位于欧盟的服务器上。
- **自托管完全属于你。** 你在自己的 PC 或 NAS 上运行 \`hoard-server\`，存档从你的机器走到你的硬盘。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。我们看不到任何存档、游戏名或邮箱地址，原因很简单：这些从未到达我们这里。就算 Hoard Cloud 明天消失，自托管的部署照常运行。

同一个程序，同样的检测，同样的版本历史。唯一变化的是存储归谁所有。

## 你该选哪个？

- 如果你想要一款免费、以本地为主的备份工具，并且不介意用 Rclone 搭建自己的云端，就选 **Ludusavi**。
- 如果你想让备份*和*跨 PC 同步都自动生效，拥有带版本的云端历史，同时保留自托管的选项，就选 **Hoard**。

很多人先用 Ludusavi 做本地备份，等到在不止一台机器上玩同样的游戏时再转向 Hoard。如果这就是你，请见[如何在多台 PC 之间同步存档](/guides/sync-game-saves-across-pcs)，或直接[下载 Hoard](/download) 并登录。想看完整的横向对比，可以读[所有存档同步工具的比较](/guides/game-save-sync-comparison)。

<!-- faq -->

## 常见问题

### 可以同时使用 Ludusavi 和 Hoard 吗？

可以。两者读取相同的存档位置，也都不会长期占用文件。很多人用 Ludusavi 做本地归档备份，把机器之间的同步交给 Hoard。唯一的原则是：不要让其中一个指向另一个的备份文件夹。

### Hoard 会导入我的 Ludusavi 备份吗？

不会，这是刻意的。备份文件夹是一份按自己节奏变化的副本，追踪它会同步一份过期镜像，而不是你真正的存档。Hoard 追踪游戏写入的那个文件夹，并从你的下一次游玩开始建立自己的历史。请把 Ludusavi 的归档留作安全网。

### Hoard 是免费的吗？

Hoard Cloud 提供 2 GB 存储和 3 台设备的免费额度，足够覆盖大多数存档收藏；Pro 会同时提高这两项。自己托管服务器是免费的，而且完全没有配额。全部代码以 AGPL-3.0 开源。

### Hoard 支持 Steam Deck 吗？

支持，包括 Steam Deck 和任何 Linux 桌面，以及 Windows 和 macOS。Deck 正是上面 \`remote/\` 那个细节要解决的场景：Deck 和台式机会在同一份存档旁边写入不同的成就与游戏时长文件。

### 我需要 Rclone 或自己的云账号吗？

不需要。这正是最主要的实际差别：使用 Hoard Cloud，登录时存储就已经准备好了。如果你更想自己拥有存储，可以让服务器对接兼容 S3 的存储桶，或你机器上的一个普通文件夹。

### 自托管会向 Hoard 发送任何东西吗？

不会。在自托管模式下，没有我们这边的账号，也没有发往我们的遥测：你的存档、你的用户和你的日志都留在你自己的服务器上，从不接触我们的服务器。这正是这一模式的意义，也是服务器用的是我们自己在跑的同一个开源二进制、而不是删减版的原因。

### Ludusavi 安全吗？

安全。它是开源的，用户众多，所做的只是把存档文件复制到备份文件夹再复制回来，不会动游戏文件。唯一需要注意的一点对任何备份工具都成立：把旧备份恢复到较新的存档上会将其替换，所以恢复前先看一下日期。

### Ludusavi 能在 Steam Deck 上用吗？

能。有可以在桌面模式下安装的 Linux 版本，还有一个 Decky 插件可以在游戏模式下触发备份。它自己做不到的是让 Deck 和台式机保持同步：你得在一台上备份，再到另一台上恢复。Hoard 通过后台服务自动完成这一步，无需手动触发。

### Ludusavi 能备份到 Google Drive 或其他云盘吗？

能，通过 Rclone：为 Google Drive、Dropbox、OneDrive 或其他 Rclone 支持的服务配置一个远程，Ludusavi 就会把备份复制过去。用 Hoard 则不需要配置远程；如果你想自己掌握存储，就把它指向你自己的服务器。

### Ludusavi 会自动备份吗？

它本身不会：你运行它时才备份。你可以用它的命令行加计划任务实现自动化，或者包装游戏的启动命令，让它在游戏退出时备份。Hoard 无需任何包装就能察觉游戏关闭，并在那时备份。

### Ludusavi 支持非 Steam 游戏吗？

支持。清单涵盖 GOG、Epic、Xbox 等启动器的游戏，以及许多不通过商店销售的游戏，缺少的还可以自行添加条目。Hoard 读取同一份清单，并为清单里没有的游戏额外进行文件系统扫描。
`,La=`---
title: "OpenSave-Alternative: direkt zwischen Geräten oder über einen eigenen Server"
description: "OpenSave synct Peer-to-Peer, Hoard über einen Server, unseren oder deinen, und behält jede Version. Ein ehrlicher Blick, wann welches Design gewinnt."
order: 8
updated: 2026-09-01
---

Beide Werkzeuge lösen dasselbe Problem und sind sich über die Architektur uneinig, und genau das ist das Einzige, was einen Vergleich lohnt. Diese Seite legt die zwei Ansätze nebeneinander, samt der Fälle, in denen der andere die bessere Antwort ist.

## Der eigentliche Unterschied: direkt oder über einen Server

**OpenSave** arbeitet peer-to-peer. Deine Maschinen reden direkt miteinander, dazwischen sitzt nichts. Kein Konto, kein Speicher, den man bezahlt, und optional lässt sich eine Kopie in eine Cloud spiegeln, die du ohnehin hast.

**Hoard** synchronisiert über einen Server. Dieser Server ist entweder Hoard Cloud, von uns betrieben, oder \`hoard-server\` auf deinem eigenen PC oder NAS. Dein Stand geht hoch, wenn du aufhörst, und kommt herunter, wenn eine andere Maschine danach fragt.

Alles Weitere folgt aus dieser einen Entscheidung.

## Was dir ein Server bringt

- **Die andere Maschine muss nicht laufen.** Du hörst am Desktop auf, der Laptop bleibt eine Woche zu, und beim Aufklappen wartet der neueste Stand. Peer-to-peer braucht beide Enden gleichzeitig wach — am Schreibtisch kein Problem, mit einem Handheld, das du zweimal im Monat anfasst, schon.
- **Eine Versionshistorie statt nur des letzten Zustands.** Jede Sitzung wird eine Version, zu der du zurückkannst. Das zählt an dem Tag, an dem ein Mod deine Welt frisst oder ein Stand halb geschrieben landet: eine direkte Synchronisierung kopiert die kaputte Datei getreulich auf den anderen PC.
- **Eine Kopie, die die Hardware überlebt.** Dass beide PCs in derselben Wohnung sterben, ist kein exotisches Szenario. Ein Spielstand, den es nur auf diesen zwei Maschinen gab, stirbt mit ihnen.
- **Nichts am Netzwerk zu regeln.** Kein NAT zu durchqueren, kein Port zu öffnen, keine Bedingung, dass beide im selben LAN hängen.

## Was dir peer-to-peer bringt

Fairerweise die andere Seite:

- **Nie Speicher zu bezahlen.** Es gibt kein Limit zu erreichen, weil es keinen Speicherort gibt. Hoards kostenloser Tarif sind 2 GB, darüber zahlst du oder hostest selbst.
- **Von Natur aus nichts dazwischen.** Wenn das Ziel ist, dass eine Datei nie die Platte eines Dritten berührt, ist direkte Übertragung die kürzestmögliche Antwort.
- **Nichts zu betreiben.** Kein Server, der laufen muss, nicht einmal ein eigener.

Wenn du an zwei Desktops spielst, die beide eingeschaltet sind, nie zurückrollen willst und über Speicher gar nicht nachdenken möchtest, passt dieses Design sauber, und Hoard ist mehr Maschinerie als nötig.

## Die Datenschutzfrage, präzise beantwortet

Hier gehen Vergleiche zu Hoard meist schief, deshalb genau: es gibt zwei Betriebsarten, und sie unterscheiden sich wirklich.

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Stände liegen auf unseren Servern in der EU.
- **Selbsthosten gehört vollständig dir.** Du betreibst \`hoard-server\` auf deinem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir sehen weder Spielstand noch Spieltitel noch E-Mail-Adresse, weil davon nichts bei uns ankommt. Würde Hoard Cloud morgen abgeschaltet, liefe ein selbst gehostetes Setup unverändert weiter.

"Server" heißt also nicht "der Computer von jemand anderem", außer du willst es so. Ein selbst gehostetes Hoard hält deine Stände auf deiner eigenen Hardware, genau wie eine direkte Übertragung, und gibt dir zusätzlich Historie und den Fall der ausgeschalteten Maschine.

## Erkennung und Abdeckung

Beide Werkzeuge finden Spielstände für einen großen Katalog automatisch. Hoard liest dasselbe Community-Manifest für Speicherorte, das im Open-Source-Umfeld geteilt wird und über 20.000 Titel abdeckt, und legt Steam-Bibliotheken, laufende Prozesse und einen Dateisystem-Scan obendrauf. Bei Steam-Spielen verfolgt es \`<AppID>/remote/\` in \`userdata\` statt des Ordners darüber, denn der enthält \`remotecache.vdf\` und gerätebezogene Dateien für Erfolge und Spielzeit — synchronisiert man die, sieht jeder Start nach einem Konflikt aus. Ungewöhnliches richtest du von Hand ein.

## Was solltest du nehmen?

- **Peer-to-peer**, wenn deine Maschinen gleichzeitig laufen, Speicher gar nicht vorkommen soll und der letzte Stand alles ist, was du je gebraucht hast.
- **Hoard**, wenn du eine Historie zum Zurückrollen willst, eine Maschine eine Woche aus sein darf und eine Kopie beide PCs überleben soll — wahlweise über unsere Cloud oder deinen eigenen Server.

Es gibt einen breiteren [Vergleich aller Sync-Tools](/guides/game-save-sync-comparison) und einen [Ludusavi-Vergleich](/guides/ludusavi-alternative) für die Seite der lokalen Backups.

<!-- faq -->

## Häufige Fragen

### Braucht Hoard ein Konto?

Für Hoard Cloud ja, daran hängt die Synchronisierung. Selbst gehostet gibt es gar kein Konto bei uns: dein Server hat eigene Benutzer und ein Token je Gerät, und die verlassen deine Maschine nie.

### Funktioniert Hoard ganz ohne Cloud?

Ja. Betreibe \`hoard-server\` auf einem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte, ohne dass etwas über unsere Server läuft.

### Müssen beide PCs gleichzeitig online sein?

Nein, und das ist der praktische Vorteil der Synchronisierung über einen Server. Dein Stand wird hochgeladen, wenn du aufhörst, und heruntergeladen, sobald die andere Maschine das nächste Mal danach fragt.

### Führt eine Direktübertragung eine Versionshistorie?

Von sich aus nicht — eine Datei auf eine andere Maschine zu kopieren gibt dir den aktuellen Zustand auf beiden. Hoard sichert jede Sitzung als Version, und genau das macht das Zurückrollen eines beschädigten Stands möglich.

### Ist Hoard ebenfalls Open Source?

Ja, AGPL-3.0, Server inklusive. Der selbst gehostete Server ist dasselbe Binary, das wir betreiben, keine abgespeckte Edition.
`,Ha=`---
title: "OpenSave alternative: peer-to-peer or a server you own"
description: "OpenSave syncs saves peer-to-peer; Hoard syncs through a server, ours or yours, and keeps every version. An honest look at when each design wins."
order: 8
updated: 2026-09-01
related: game-save-sync-comparison, self-host-hoard, syncthing-game-saves
---

Both tools solve the same problem and disagree about the architecture, which is the only thing worth comparing. This page lays the two designs side by side, including the cases where the other one is the better answer.

## The actual difference: peer-to-peer or a server

**OpenSave** is peer-to-peer. Your machines talk to each other directly, and nothing sits in between. There's no account and no storage to pay for, and it can optionally mirror a copy to a cloud drive you already have.

**Hoard** syncs through a server. That server is either Hoard Cloud, managed by us, or \`hoard-server\` running on your own PC or NAS. Your save goes up when you stop playing and comes down when another machine asks for it.

Everything else follows from that one choice.

## What a server buys you

- **The other machine doesn't have to be on.** You finish on the desktop, the laptop stays shut for a week, and the latest save is waiting when you open it. Peer-to-peer needs both ends awake at the same time, which is fine at a desk and awkward with a handheld you pick up twice a month.
- **A version history, not just the latest state.** Every session becomes a version you can roll back to. This is the part that matters the day a mod eats your world or a save is written half-corrupt: direct sync faithfully copies the broken file to your other PC.
- **A copy that survives the hardware.** Both your PCs dying in the same flat is not an exotic scenario. A save that only ever existed on those two machines dies with them.
- **Nothing to arrange on the network.** No NAT to traverse, no port to open, no both-devices-on-the-same-LAN caveat.

## What peer-to-peer buys you

Being fair about the other side:

- **No storage to pay for, ever.** There's no quota to hit, because there's no bucket. Hoard's free tier is 2 GB, and above that you either pay or self-host.
- **Nothing in the middle by design.** If the goal is that a file never touches a third party's disk, direct transfer is the shortest possible answer.
- **Nothing to run.** No server to keep up, not even your own.

If you play on two desktops that are both switched on, you never want to roll back, and you'd rather not think about storage at all, that design is a clean fit and Hoard is more machinery than you need.

## The privacy question, answered precisely

This is where comparisons of Hoard usually go wrong, so it's worth being exact. There are two ways to run Hoard, and they are genuinely different:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **Self-hosting is entirely yours.** You run \`hoard-server\` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, because none of it ever reaches us. If Hoard Cloud shut down tomorrow, a self-hosted setup would carry on unchanged.

So "server" doesn't mean "someone else's computer" unless you choose that. A self-hosted Hoard keeps your saves on hardware you own, exactly like a direct transfer does, and still gives you the history and the offline-machine case.

## Detection and coverage

Both tools find saves for a large catalogue automatically. Hoard reads the same community save-location manifest that the open-source ecosystem shares, covering 20,000+ titles, and adds Steam library scanning, running processes and a filesystem sweep on top. For Steam games it tracks \`<AppID>/remote/\` inside \`userdata\` rather than the folder above, because the parent holds \`remotecache.vdf\` and per-machine achievement and playtime files — sync those and every launch looks like a conflict. Anything unusual you can point it at by hand.

## Which one should you use?

- **Peer-to-peer** if your machines are on at the same time, you don't want storage in the picture at all, and the latest save is all you've ever needed.
- **Hoard** if you want a version history you can roll back, a machine that can be off for a week, and a copy that outlives both PCs — with the choice of our cloud or your own server.

There's a wider [comparison of every save sync tool](/guides/game-save-sync-comparison) if you want the whole field, and a [Ludusavi comparison](/guides/ludusavi-alternative) for the local-backup end of it.

<!-- faq -->

## Frequently asked questions

### Does Hoard need an account?

For Hoard Cloud, yes — that's what the sync is tied to. Self-hosted, there's no account with us at all; your server has its own users and a token per device, and they never leave your machine.

### Can Hoard work without any cloud?

Yes. Run \`hoard-server\` on a PC or a NAS and your saves go from your machine to your disk, with nothing passing through our servers.

### Do both PCs need to be online at the same time?

No, and that's the practical advantage of syncing through a server. Your save is uploaded when you stop playing and downloaded whenever the other machine next asks for it.

### Does a direct transfer keep a version history?

Not inherently — copying a file to another machine gives you the current state on both. Hoard captures every session as a version, which is what makes rolling back a corrupted save possible.

### Is Hoard open source too?

Yes, AGPL-3.0, server included. The self-hosted server is the same binary we run, not a cut-down edition.
`,ja=`---
title: "Alternativa a OpenSave: entre equipos o con un servidor tuyo"
description: "OpenSave sincroniza de igual a igual; Hoard lo hace a través de un servidor, el nuestro o el tuyo, y guarda cada versión. Cuándo gana cada diseño."
order: 8
updated: 2026-09-01
---

Las dos herramientas resuelven el mismo problema y discrepan en la arquitectura, que es lo único que merece compararse. Esta página pone los dos diseños uno al lado del otro, incluidos los casos en los que el otro es mejor respuesta.

## La diferencia de verdad: entre equipos o con servidor

**OpenSave** es punto a punto. Tus máquinas hablan entre ellas directamente y no hay nada en medio. No hay cuenta ni almacenamiento que pagar, y opcionalmente puede espejar una copia en una nube que ya tengas.

**Hoard** sincroniza a través de un servidor. Ese servidor es Hoard Cloud, gestionado por nosotros, o \`hoard-server\` corriendo en tu propio PC o NAS. Tu partida sube cuando dejas de jugar y baja cuando otra máquina la pide.

Todo lo demás sale de esa única decisión.

## Qué te da tener un servidor

- **La otra máquina no tiene que estar encendida.** Terminas en el sobremesa, el portátil sigue cerrado una semana, y la última partida está esperando cuando lo abres. Lo punto a punto necesita los dos extremos despiertos a la vez, que es perfecto en un escritorio e incómodo con una consola de mano que coges dos veces al mes.
- **Un historial de versiones, no sólo el último estado.** Cada sesión es una versión a la que puedes volver. Es la parte que importa el día que un mod se come tu mundo o una partida se escribe a medias: una sincronización directa copia fielmente el fichero roto al otro PC.
- **Una copia que sobrevive al hardware.** Que tus dos PC mueran en el mismo piso no es un escenario exótico. Una partida que sólo existió en esas dos máquinas se muere con ellas.
- **Nada que preparar en la red.** Ningún NAT que atravesar, ningún puerto que abrir, ninguna condición de estar los dos en la misma LAN.

## Qué te da lo punto a punto

Siendo justos con el otro lado:

- **Ningún almacenamiento que pagar, nunca.** No hay cupo que agotar porque no hay depósito. El plan gratuito de Hoard son 2 GB, y por encima pagas o te autoalojas.
- **Nada en medio por diseño.** Si el objetivo es que un fichero no toque nunca el disco de un tercero, la transferencia directa es la respuesta más corta posible.
- **Nada que mantener.** Ningún servidor en pie, ni siquiera el tuyo.

Si juegas en dos sobremesas que están los dos encendidos, nunca quieres volver atrás y prefieres no pensar en almacenamiento, ese diseño encaja limpio y Hoard es más maquinaria de la que necesitas.

## La cuestión de la privacidad, con precisión

Aquí es donde las comparativas de Hoard suelen equivocarse, así que conviene ser exacto. Hay dos formas de usar Hoard, y son genuinamente distintas:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Autoalojarse es tuyo por completo.** Levantas \`hoard-server\` en tu PC o en tu NAS y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, porque nada de eso nos llega. Si Hoard Cloud cerrara mañana, un montaje autoalojado seguiría igual.

O sea que «servidor» no significa «el ordenador de otro» salvo que tú lo elijas. Un Hoard autoalojado mantiene tus partidas en hardware tuyo, exactamente igual que una transferencia directa, y encima te da el historial y el caso de la máquina apagada.

## Detección y cobertura

Las dos herramientas encuentran partidas de un catálogo grande de forma automática. Hoard lee el mismo manifiesto comunitario de ubicaciones que comparte el ecosistema open source, con más de 20.000 títulos, y le suma el barrido de bibliotecas de Steam, los procesos en ejecución y un escaneo del disco. En los juegos de Steam rastrea \`<AppID>/remote/\` dentro de \`userdata\` y no la carpeta de encima, porque la padre guarda \`remotecache.vdf\` y ficheros de logros y tiempo jugado propios de cada máquina: si sincronizas eso, cada arranque parece un conflicto. Lo raro se lo señalas a mano.

## ¿Cuál deberías usar?

- **Punto a punto** si tus máquinas están encendidas a la vez, no quieres almacenamiento en la ecuación y la última partida es todo lo que has necesitado nunca.
- **Hoard** si quieres un historial al que volver, una máquina que pueda estar apagada una semana y una copia que sobreviva a los dos PC, con la opción de usar nuestra nube o tu propio servidor.

Hay una [comparativa de todas las herramientas de sincronización](/guides/game-save-sync-comparison) si quieres el panorama completo, y una [comparativa con Ludusavi](/guides/ludusavi-alternative) para la parte de copias locales.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard necesita cuenta?

Para Hoard Cloud sí, porque es a lo que está atada la sincronización. Autoalojado no hay ninguna cuenta con nosotros: tu servidor tiene sus propios usuarios y un token por dispositivo, y no salen de tu máquina.

### ¿Puede funcionar Hoard sin ninguna nube?

Sí. Levanta \`hoard-server\` en un PC o en un NAS y tus partidas van de tu máquina a tu disco, sin que nada pase por nuestros servidores.

### ¿Tienen que estar los dos PC encendidos a la vez?

No, y ésa es la ventaja práctica de sincronizar a través de un servidor. Tu partida sube cuando dejas de jugar y baja cuando la otra máquina la pida.

### ¿Una transferencia directa guarda historial de versiones?

No de por sí: copiar un fichero a otra máquina te deja el estado actual en las dos. Hoard captura cada sesión como una versión, y eso es lo que hace posible volver atrás desde una partida corrupta.

### ¿Hoard también es open source?

Sí, AGPL-3.0, servidor incluido. El servidor autoalojado es el mismo binario que usamos nosotros, no una edición recortada.
`,xa=`---
title: "Alternative à OpenSave : direct entre machines ou serveur qui vous appartient"
description: "OpenSave synchronise en pair-à-pair ; Hoard passe par un serveur, le nôtre ou le vôtre, et garde chaque version. Quand chaque approche l'emporte."
order: 8
updated: 2026-09-01
---

Les deux outils résolvent le même problème et divergent sur l'architecture, et c'est bien la seule chose qui mérite comparaison. Cette page met les deux approches côte à côte, y compris les cas où l'autre est la meilleure réponse.

## La vraie différence : direct ou via un serveur

**OpenSave** est pair-à-pair. Vos machines se parlent directement, sans rien entre elles. Pas de compte, pas de stockage à payer, et la possibilité de refléter une copie vers un cloud que vous avez déjà.

**Hoard** synchronise via un serveur. Ce serveur est soit Hoard Cloud, géré par nous, soit \`hoard-server\` sur votre propre PC ou NAS. Votre sauvegarde monte quand vous arrêtez de jouer et redescend quand une autre machine la demande.

Tout le reste découle de ce seul choix.

## Ce qu'un serveur vous apporte

- **L'autre machine n'a pas besoin d'être allumée.** Vous finissez sur le fixe, le portable reste fermé une semaine, et la dernière sauvegarde attend quand vous l'ouvrez. Le pair-à-pair exige les deux bouts éveillés en même temps : parfait à un bureau, pénible avec une console portable que vous sortez deux fois par mois.
- **Un historique de versions, pas seulement le dernier état.** Chaque session devient une version où revenir. C'est ce qui compte le jour où un mod dévore votre monde ou qu'une sauvegarde s'écrit à moitié : une synchro directe recopie fidèlement le fichier cassé sur l'autre PC.
- **Une copie qui survit au matériel.** Que vos deux PC meurent dans le même appartement n'a rien d'exotique. Une sauvegarde qui n'a existé que sur ces deux machines meurt avec elles.
- **Rien à préparer côté réseau.** Pas de NAT à traverser, pas de port à ouvrir, pas de condition d'être sur le même réseau local.

## Ce que le pair-à-pair vous apporte

Pour être juste avec l'autre camp :

- **Jamais de stockage à payer.** Aucun quota à atteindre, puisqu'il n'y a pas d'espace de stockage. L'offre gratuite de Hoard, c'est 2 Go ; au-delà, vous payez ou vous auto-hébergez.
- **Rien au milieu, par construction.** Si l'objectif est qu'un fichier ne touche jamais le disque d'un tiers, le transfert direct est la réponse la plus courte possible.
- **Rien à faire tourner.** Aucun serveur à maintenir, pas même le vôtre.

Si vous jouez sur deux fixes allumés tous les deux, que vous ne voulez jamais revenir en arrière et que le stockage ne doit pas entrer dans l'équation, cette approche convient parfaitement et Hoard est plus de machinerie qu'il n'en faut.

## La question de la vie privée, précisément

C'est là que les comparaisons de Hoard se trompent d'habitude, alors soyons exacts : il y a deux façons de faire tourner Hoard, et elles sont réellement différentes.

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont sur nos serveurs, dans l'UE.
- **L'auto-hébergement est entièrement le vôtre.** Vous faites tourner \`hoard-server\` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. **Aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne voyons ni sauvegarde, ni nom de jeu, ni adresse e-mail, car rien de tout cela ne nous parvient. Si Hoard Cloud fermait demain, une installation auto-hébergée continuerait à l'identique.

Donc « serveur » ne veut pas dire « l'ordinateur de quelqu'un d'autre », sauf si vous le choisissez. Un Hoard auto-hébergé garde vos sauvegardes sur du matériel qui vous appartient, exactement comme un transfert direct, et vous donne en plus l'historique et le cas de la machine éteinte.

## Détection et couverture

Les deux outils trouvent automatiquement les sauvegardes d'un large catalogue. Hoard lit le même manifeste communautaire d'emplacements que partage l'écosystème open source, couvrant plus de 20 000 titres, et y ajoute l'analyse des bibliothèques Steam, les processus en cours et un balayage du disque. Pour les jeux Steam, il suit \`<AppID>/remote/\` dans \`userdata\` et non le dossier au-dessus, car le parent contient \`remotecache.vdf\` et des fichiers de succès et de temps de jeu propres à chaque machine : les synchroniser, et chaque lancement ressemble à un conflit. Pour les cas particuliers, vous lui désignez le dossier.

## Lequel choisir ?

- **Le pair-à-pair** si vos machines sont allumées en même temps, que le stockage ne doit pas entrer en jeu et que la dernière sauvegarde vous a toujours suffi.
- **Hoard** si vous voulez un historique où revenir, une machine qui peut rester éteinte une semaine et une copie qui survive aux deux PC — au choix via notre cloud ou votre propre serveur.

Il existe une [comparaison de tous les outils de synchro](/guides/game-save-sync-comparison) pour le paysage complet, et une [comparaison avec Ludusavi](/guides/ludusavi-alternative) pour le versant sauvegarde locale.

<!-- faq -->

## Questions fréquentes

### Hoard exige-t-il un compte ?

Pour Hoard Cloud, oui : la synchro y est rattachée. En auto-hébergé, aucun compte chez nous ; votre serveur a ses propres utilisateurs et un jeton par appareil, et ils ne quittent jamais votre machine.

### Hoard peut-il fonctionner sans aucun cloud ?

Oui. Faites tourner \`hoard-server\` sur un PC ou un NAS et vos sauvegardes vont de votre machine à votre disque, sans que rien passe par nos serveurs.

### Les deux PC doivent-ils être en ligne en même temps ?

Non, et c'est l'avantage pratique de passer par un serveur. Votre sauvegarde est envoyée quand vous arrêtez de jouer et téléchargée dès que l'autre machine la réclame.

### Un transfert direct garde-t-il un historique de versions ?

Pas en soi : copier un fichier vers une autre machine vous donne l'état actuel des deux côtés. Hoard capture chaque session comme une version, ce qui rend possible le retour en arrière après une sauvegarde corrompue.

### Hoard est-il open source lui aussi ?

Oui, AGPL-3.0, serveur compris. Le serveur auto-hébergé est le même binaire que celui que nous faisons tourner, pas une édition allégée.
`,Oa=`---
title: "Alternativa a OpenSave: diretto tra macchine o con un server tuo"
description: "OpenSave sincronizza peer-to-peer; Hoard passa da un server, il nostro o il tuo, e tiene ogni versione. Uno sguardo onesto su quando vince ciascuno."
order: 8
updated: 2026-09-01
---

I due strumenti risolvono lo stesso problema e non sono d'accordo sull'architettura, che è l'unica cosa che valga la pena confrontare. Questa pagina mette i due approcci uno accanto all'altro, compresi i casi in cui l'altro è la risposta migliore.

## La differenza vera: diretto o con un server

**OpenSave** è peer-to-peer. Le tue macchine si parlano direttamente e in mezzo non c'è nulla. Nessun account e nessuno spazio da pagare, e in opzione può replicare una copia su un cloud che hai già.

**Hoard** sincronizza attraverso un server. Quel server è Hoard Cloud, gestito da noi, oppure \`hoard-server\` sul tuo PC o sul tuo NAS. Il salvataggio sale quando smetti di giocare e scende quando un'altra macchina lo chiede.

Tutto il resto discende da questa singola scelta.

## Cosa ti dà un server

- **L'altra macchina non deve essere accesa.** Finisci sul fisso, il portatile resta chiuso una settimana, e all'apertura l'ultimo salvataggio è lì ad aspettare. Il peer-to-peer vuole entrambi i capi svegli nello stesso momento: ottimo alla scrivania, scomodo con una portatile che prendi in mano due volte al mese.
- **Una cronologia, non solo l'ultimo stato.** Ogni sessione diventa una versione a cui tornare. È la parte che conta il giorno in cui una mod ti mangia il mondo o un salvataggio finisce scritto a metà: una sincronizzazione diretta copia fedelmente il file rotto sull'altro PC.
- **Una copia che sopravvive all'hardware.** Che entrambi i PC muoiano nella stessa casa non è uno scenario esotico. Un salvataggio esistito solo su quelle due macchine muore con loro.
- **Niente da sistemare sulla rete.** Nessun NAT da attraversare, nessuna porta da aprire, nessun vincolo di stare sulla stessa LAN.

## Cosa ti dà il peer-to-peer

Per essere onesti con l'altra parte:

- **Nessuno spazio da pagare, mai.** Non c'è quota da esaurire perché non c'è un archivio. Il piano gratuito di Hoard è 2 GB, sopra si paga o si fa self-hosting.
- **Niente in mezzo per progetto.** Se l'obiettivo è che un file non tocchi mai il disco di terzi, il trasferimento diretto è la risposta più breve possibile.
- **Niente da mandare avanti.** Nessun server da tenere in piedi, nemmeno il tuo.

Se giochi su due fissi entrambi accesi, non vuoi mai tornare indietro e preferisci non pensare allo spazio, quell'approccio calza perfettamente e Hoard è più macchinario di quanto ti serva.

## La questione privacy, detta con precisione

È qui che i confronti su Hoard di solito sbagliano, quindi siamo esatti: ci sono due modi di usarlo e sono davvero diversi.

- **Hoard Cloud** è l'opzione gestita: accedi e i salvataggi stanno sui nostri server, nell'UE.
- **Il self-hosting è interamente tuo.** Fai girare \`hoard-server\` sul tuo PC o NAS e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non vediamo un salvataggio, il nome di un gioco o un indirizzo email, perché niente di tutto ciò ci arriva. Se Hoard Cloud chiudesse domani, un'installazione self-hosted continuerebbe uguale.

Quindi "server" non vuol dire "il computer di qualcun altro", a meno che tu non lo scelga. Un Hoard self-hosted tiene i salvataggi su hardware tuo, esattamente come un trasferimento diretto, e in più ti dà la cronologia e il caso della macchina spenta.

## Rilevamento e copertura

Entrambi trovano automaticamente i salvataggi di un catalogo ampio. Hoard legge lo stesso manifest comunitario delle posizioni condiviso dall'ecosistema open source, oltre 20.000 titoli, e ci aggiunge le librerie Steam, i processi in esecuzione e una scansione del disco. Per i giochi Steam traccia \`<AppID>/remote/\` dentro \`userdata\` e non la cartella superiore, perché quella contiene \`remotecache.vdf\` e file di obiettivi e tempo di gioco propri di ogni macchina: sincronizzarli significa vedere un conflitto a ogni avvio. Per i casi insoliti gli indichi tu la cartella.

## Quale usare?

- **Peer-to-peer** se le tue macchine sono accese insieme, non vuoi che lo spazio entri nel discorso e l'ultimo salvataggio è tutto ciò che ti è mai servito.
- **Hoard** se vuoi una cronologia a cui tornare, una macchina che possa restare spenta una settimana e una copia che sopravviva a entrambi i PC, con la scelta tra il nostro cloud e il tuo server.

C'è un [confronto di tutti gli strumenti di sincronizzazione](/guides/game-save-sync-comparison) per il quadro completo, e un [confronto con Ludusavi](/guides/ludusavi-alternative) per il versante dei backup locali.

<!-- faq -->

## Domande frequenti

### Hoard richiede un account?

Per Hoard Cloud sì, perché la sincronizzazione è legata a quello. In self-hosting non c'è alcun account con noi: il tuo server ha i suoi utenti e un token per dispositivo, e non escono dalla tua macchina.

### Hoard può funzionare senza alcun cloud?

Sì. Fai girare \`hoard-server\` su un PC o un NAS e i salvataggi vanno dalla tua macchina al tuo disco, senza che nulla passi dai nostri server.

### Servono entrambi i PC online nello stesso momento?

No, ed è il vantaggio pratico di passare da un server. Il salvataggio viene caricato quando smetti di giocare e scaricato quando l'altra macchina lo richiede.

### Un trasferimento diretto tiene una cronologia?

Non di per sé: copiare un file su un'altra macchina ti dà lo stato attuale su entrambe. Hoard cattura ogni sessione come una versione, ed è questo a rendere possibile tornare indietro da un salvataggio corrotto.

### Anche Hoard è open source?

Sì, AGPL-3.0, server incluso. Il server self-hosted è lo stesso binario che usiamo noi, non un'edizione ridotta.
`,Ga=`---
title: "OpenSave の代替：端末間の直接転送か、自分のサーバーか"
description: "OpenSaveはP2Pで同期、Hoardは自前または当社のサーバー経由で同期し全バージョンを保持。どちらの設計が向くかを公平に解説。"
order: 8
updated: 2026-09-01
---

どちらのツールも同じ問題を解こうとしていて、設計思想だけが食い違っています。比べる価値があるのはそこだけです。このページでは 2 つの設計を並べ、相手のほうが良い答えになる場面も含めて説明します。

## 本当の違い：直接転送かサーバー経由か

**OpenSave** はピアツーピアです。あなたのマシン同士が直接やり取りし、あいだには何も入りません。アカウントも、支払うストレージもなく、必要なら既に持っているクラウドドライブへ複製することもできます。

**Hoard** はサーバー経由で同期します。そのサーバーは、当方が運用する Hoard Cloud か、自分の PC や NAS で動かす \`hoard-server\` のどちらかです。プレイを終えるとセーブが上がり、別のマシンが求めたときに下ります。

あとはすべて、この 1 つの選択から派生します。

## サーバーが与えてくれるもの

- **もう 1 台が起動している必要がありません。** デスクトップで遊び終え、ノート PC は 1 週間閉じたままでも、開いたときには最新のセーブが待っています。ピアツーピアは両端が同時に起きている必要があり、机の上なら問題なくても、月に 2 回しか触らない携帯機では厄介です。
- **最新の状態だけでなく、世代履歴が残ります。** 1 セッションが 1 つの世代になり、そこへ戻れます。Mod がワールドを食べた日や、セーブが途中まで書かれた日に効いてくる部分です。直接同期は、壊れたファイルを忠実にもう 1 台へコピーします。
- **ハードウェアより長生きするコピー。** 2 台の PC が同じ部屋で同時に壊れるのは、珍しい話ではありません。その 2 台にしか存在しなかったセーブは、一緒に消えます。
- **ネットワーク側の準備が不要。** NAT 越えも、開けるポートも、両方が同じ LAN にいる必要もありません。

## ピアツーピアが与えてくれるもの

相手側にも公平に。

- **ストレージ料金が一切ない。** 保管場所そのものがないので、使い切る上限もありません。Hoard の無料枠は 2 GB で、それを超えると支払うかセルフホストするかになります。
- **設計上、あいだに何も入らない。** ファイルが第三者のディスクに一度も触れないことが目的なら、直接転送はいちばん短い答えです。
- **動かし続けるものがない。** サーバーは不要で、自分のものすら要りません。

常時起動のデスクトップ 2 台で遊び、巻き戻す必要を感じたことがなく、ストレージのことを考えたくないのなら、その設計はきれいに噛み合い、Hoard は必要以上の仕掛けになります。

## プライバシーの話を、正確に

Hoard についての比較がいちばん誤りやすいのがここなので、正確に書きます。Hoard には 2 つの動かし方があり、両者は本当に別物です。

- **Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに保存されます。
- **セルフホストは完全にあなたのものです。** 自分の PC や NAS で \`hoard-server\` を動かせば、セーブは自分のマシンから自分のディスクへ移ります。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。セーブもゲーム名もメールアドレスも見えません。届かないからです。仮に明日 Hoard Cloud が終了しても、セルフホスト構成はそのまま動き続けます。

つまり「サーバー」は、あなたがそう選ばない限り「他人のコンピューター」を意味しません。セルフホストした Hoard は、直接転送とまったく同じように、セーブを自分のハードウェアの中にとどめたうえで、履歴と「電源が入っていないマシン」への対応を追加してくれます。

## 検出と対応範囲

どちらのツールも、広いカタログのセーブを自動で見つけます。Hoard はオープンソースの世界で共有されているのと同じ、2 万本以上を収録したコミュニティのセーブ位置マニフェストを読み、そこに Steam ライブラリの走査、実行中プロセス、ファイルシステムの掃引を重ねます。Steam のゲームでは、上のフォルダーではなく \`userdata\` 内の \`<AppID>/remote/\` を追跡します。上のフォルダーには \`remotecache.vdf\` や、実績・プレイ時間といったマシンごとのファイルがあり、それを同期すると起動のたびに競合に見えるからです。変わったものは手動で指定できます。

## どちらを使うべきか

- 2 台が同時に起動していて、ストレージを話に入れたくなく、最新のセーブだけで足りてきたのなら **ピアツーピア**。
- 巻き戻せる履歴が欲しい、1 週間電源を切っていてよいマシンがある、2 台の PC より長生きするコピーが欲しい——そのいずれかなら **Hoard**。当方のクラウドでも、自分のサーバーでも選べます。

全体像が知りたい場合は [セーブ同期ツールの比較](/guides/game-save-sync-comparison) を、ローカルバックアップ側については [Ludusavi との比較](/guides/ludusavi-alternative) をご覧ください。

<!-- faq -->

## よくある質問

### Hoard にはアカウントが必要ですか？

Hoard Cloud では必要です。同期がそこに結びついているためです。セルフホストなら当方のアカウントは一切ありません。あなたのサーバーが自分のユーザーと端末ごとのトークンを持ち、それらがマシンの外に出ることはありません。

### クラウドをまったく使わずに運用できますか？

はい。PC か NAS で \`hoard-server\` を動かせば、セーブは自分のマシンから自分のディスクへ移り、当方のサーバーを何も通りません。

### 2 台の PC を同時にオンラインにする必要はありますか？

ありません。それがサーバー経由で同期する実用上の利点です。プレイ終了時にアップロードされ、もう 1 台が次に求めたときにダウンロードされます。

### 直接転送でも世代履歴は残りますか？

そのままでは残りません。ファイルをもう 1 台へコピーすれば、両方が現在の状態になるだけです。Hoard は 1 セッションを 1 世代として取り込み、それが壊れたセーブから戻れる理由になります。

### Hoard もオープンソースですか？

はい。サーバーを含め AGPL-3.0 です。セルフホスト用のサーバーは当方が運用しているものと同じバイナリで、機能を削った版ではありません。
`,Ea=`---
title: "Alternativa ao OpenSave: direto entre máquinas ou com um servidor teu"
description: "O OpenSave sincroniza ponto a ponto; o Hoard passa por um servidor, o nosso ou o teu, e guarda cada versão. Quando ganha cada abordagem."
order: 8
updated: 2026-09-01
---

As duas ferramentas resolvem o mesmo problema e discordam quanto à arquitetura, que é a única coisa que vale a pena comparar. Esta página põe os dois desenhos lado a lado, incluindo os casos em que o outro é a melhor resposta.

## A diferença a sério: direto ou com servidor

**O OpenSave** é ponto a ponto. As tuas máquinas falam diretamente umas com as outras e no meio não há nada. Sem conta e sem armazenamento a pagar, e opcionalmente pode espelhar uma cópia para uma nuvem que já tenhas.

**O Hoard** sincroniza através de um servidor. Esse servidor é o Hoard Cloud, gerido por nós, ou o \`hoard-server\` a correr no teu PC ou no teu NAS. O teu save sobe quando paras de jogar e desce quando outra máquina o pede.

Tudo o resto sai dessa única escolha.

## O que um servidor te dá

- **A outra máquina não tem de estar ligada.** Acabas no fixo, o portátil fica fechado uma semana, e o save mais recente está à espera quando o abres. O ponto a ponto precisa das duas pontas acordadas ao mesmo tempo: perfeito numa secretária, chato com uma consola portátil que pegas duas vezes por mês.
- **Um histórico de versões, não só o último estado.** Cada sessão passa a ser uma versão à qual podes voltar. É a parte que conta no dia em que uma mod te come o mundo ou um save fica escrito a meio: uma sincronização direta copia fielmente o ficheiro partido para o outro PC.
- **Uma cópia que sobrevive ao hardware.** Os dois PCs morrerem na mesma casa não é um cenário exótico. Um save que só existiu nessas duas máquinas morre com elas.
- **Nada para preparar na rede.** Sem NAT para atravessar, sem porta para abrir, sem a condição de estarem os dois na mesma LAN.

## O que o ponto a ponto te dá

Sendo justos com o outro lado:

- **Nunca há armazenamento a pagar.** Não há quota para esgotar porque não há depósito. O plano gratuito do Hoard são 2 GB; acima disso pagas ou alojas tu.
- **Nada pelo meio, por desenho.** Se o objetivo é que um ficheiro nunca toque no disco de terceiros, a transferência direta é a resposta mais curta possível.
- **Nada para manter.** Nenhum servidor de pé, nem sequer o teu.

Se jogas em dois fixos ambos ligados, nunca queres voltar atrás e preferes não pensar em armazenamento, esse desenho encaixa bem e o Hoard é mais maquinaria do que precisas.

## A questão da privacidade, com precisão

É aqui que as comparações ao Hoard costumam falhar, por isso sejamos exatos: há duas formas de o usar e são genuinamente diferentes.

- **O Hoard Cloud** é a opção gerida: inicias sessão e os saves ficam nos nossos servidores, na UE.
- **O self-hosting é inteiramente teu.** Corres o \`hoard-server\` no teu PC ou NAS e os saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não vemos um save, o nome de um jogo ou um email, porque nada disso nos chega. Se o Hoard Cloud fechasse amanhã, uma instalação self-hosted continuaria igual.

Ou seja, "servidor" não significa "o computador de outra pessoa" a não ser que o escolhas. Um Hoard self-hosted mantém os saves em hardware teu, tal como uma transferência direta, e ainda te dá o histórico e o caso da máquina desligada.

## Deteção e cobertura

Ambas as ferramentas encontram automaticamente os saves de um catálogo grande. O Hoard lê o mesmo manifesto comunitário de localizações que o ecossistema open source partilha, com mais de 20.000 títulos, e junta-lhe as bibliotecas da Steam, os processos em execução e uma varredura do disco. Nos jogos da Steam segue \`<AppID>/remote/\` dentro de \`userdata\` e não a pasta acima, porque a de cima guarda \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo próprios de cada máquina: sincronizá-los é ver um conflito a cada arranque. Para o que for invulgar, apontas-lhe a pasta.

## Qual deves usar?

- **Ponto a ponto** se as tuas máquinas estão ligadas ao mesmo tempo, não queres armazenamento na equação e o último save é tudo o que alguma vez precisaste.
- **O Hoard** se queres um histórico ao qual voltar, uma máquina que possa estar desligada uma semana e uma cópia que sobreviva aos dois PCs — com a escolha entre a nossa nuvem e o teu próprio servidor.

Há uma [comparação de todas as ferramentas de sincronização](/guides/game-save-sync-comparison) para o panorama completo, e uma [comparação com o Ludusavi](/guides/ludusavi-alternative) para o lado das cópias locais.

<!-- faq -->

## Perguntas frequentes

### O Hoard precisa de conta?

Para o Hoard Cloud sim, é a isso que a sincronização está ligada. Em self-hosted não há conta nenhuma connosco: o teu servidor tem os seus utilizadores e um token por dispositivo, e não saem da tua máquina.

### O Hoard funciona sem nuvem nenhuma?

Sim. Corre o \`hoard-server\` num PC ou num NAS e os teus saves vão da tua máquina para o teu disco, sem nada a passar pelos nossos servidores.

### Os dois PCs têm de estar online ao mesmo tempo?

Não, e essa é a vantagem prática de sincronizar através de um servidor. O save sobe quando paras de jogar e desce quando a outra máquina o pedir.

### Uma transferência direta guarda histórico de versões?

Por si só não: copiar um ficheiro para outra máquina dá-te o estado atual nas duas. O Hoard captura cada sessão como uma versão, e é isso que torna possível voltar atrás a partir de um save corrompido.

### O Hoard também é open source?

Sim, AGPL-3.0, servidor incluído. O servidor self-hosted é o mesmo binário que nós corremos, não uma edição reduzida.
`,Ma=`---
title: "OpenSave 的替代方案：机器之间直连，还是一台属于你的服务器"
description: "OpenSave 点对点同步；Hoard 通过服务器（我们的或你自己的）同步并保留每个版本。客观分析两种设计各自适合的场景。"
order: 8
updated: 2026-09-01
---

两个工具解决的是同一个问题，分歧只在架构上，而这也是唯一值得比较的地方。本页把两种设计并排摆开，包括另一种才是更好答案的情形。

## 真正的差别：直连还是走服务器

**OpenSave** 是点对点的。你的机器彼此直接通信，中间不隔任何东西。没有账号，也没有要付费的存储，还可以选择把副本镜像到你已有的云盘。

**Hoard** 通过服务器同步。这台服务器要么是我们运营的 Hoard Cloud，要么是跑在你自己 PC 或 NAS 上的 \`hoard-server\`。你停止游玩时存档上传，另一台机器需要时再下载。

其余的一切，都从这一个选择衍生而来。

## 服务器带来什么

- **另一台机器不必开着。** 你在台式机上玩完，笔记本关着放一周，打开时最新存档就在等你。点对点要求两端同时醒着——在书桌前没问题，但对一个月只拿两次的掌机就很别扭。
- **版本历史，而不只是最新状态。** 每次游玩都成为一个可回退的版本。模组吞掉你的世界、或存档写到一半的那天，这一点才见真章：直连同步会忠实地把损坏的文件复制到另一台 PC。
- **一份比硬件活得久的副本。** 两台 PC 在同一间屋子里一起完蛋并不算离奇。只在这两台机器上存在过的存档，会跟着它们一起消失。
- **网络上无需张罗。** 不用穿透 NAT，不用开端口，也没有"两台必须在同一局域网"的前提。

## 点对点带来什么

对另一边也要公平：

- **永远不必为存储付费。** 没有配额可撞，因为根本没有存储桶。Hoard 的免费额度是 2 GB，超出就要付费或自托管。
- **从设计上中间就没有东西。** 如果目标是让文件永远不碰第三方的磁盘，直接传输就是最短的答案。
- **没有需要维护的东西。** 不必让任何服务器常驻，连你自己的也不用。

如果你在两台都开着的台式机上玩，从不想回退，也不愿把存储纳入考虑，那种设计干净利落，Hoard 对你而言是多余的机械。

## 隐私问题，说得精确一点

这正是关于 Hoard 的比较最常出错的地方，所以说准确些：Hoard 有两种运行方式，而且确实不同。

- **Hoard Cloud** 是托管方案：你登录，存档保存在我们位于欧盟的服务器上。
- **自托管完全属于你。** 你在自己的 PC 或 NAS 上运行 \`hoard-server\`，存档从你的机器走到你的磁盘。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。我们看不到任何存档、游戏名或邮箱地址，因为这些从未到达我们这里。就算 Hoard Cloud 明天关停，自托管的部署照常运行。

所以除非你自己选择，"服务器"并不意味着"别人的电脑"。自托管的 Hoard 把存档留在属于你的硬件上，和直接传输一样，同时还多给你历史，以及那台可以关机的机器。

## 检测与覆盖

两个工具都能自动找到大量游戏的存档。Hoard 读取开源生态共享的同一份社区存档位置清单，覆盖两万余款游戏，并在此之上加了 Steam 库扫描、运行中进程和文件系统扫描。对 Steam 游戏，它追踪 \`userdata\` 里的 \`<AppID>/remote/\` 而不是上一层文件夹，因为上一层放着 \`remotecache.vdf\` 以及各机器各自的成就和游戏时长文件——同步它们，每次启动都会像冲突。特殊情况可以手动指定文件夹。

## 你该用哪个？

- **点对点**：如果你的机器同时开着，不希望存储进入这个话题，而最新存档也一直够用。
- **Hoard**：如果你想要能回退的历史、一台可以关一周的机器，以及一份比两台 PC 都活得久的副本——并且可以在我们的云和你自己的服务器之间选择。

想看完整的横向比较，可以读[所有存档同步工具的比较](/guides/game-save-sync-comparison)；本地备份那一侧，参见[与 Ludusavi 的比较](/guides/ludusavi-alternative)。

<!-- faq -->

## 常见问题

### Hoard 需要账号吗？

用 Hoard Cloud 需要，同步就是绑定在账号上的。自托管则完全没有我们这边的账号：你的服务器有它自己的用户和每台设备一个的令牌，它们从不离开你的机器。

### Hoard 能完全不用云吗？

能。在 PC 或 NAS 上运行 \`hoard-server\`，你的存档就从你的机器走到你的磁盘，没有任何东西经过我们的服务器。

### 两台 PC 需要同时在线吗？

不需要，这正是走服务器的实际好处。你停止游玩时存档上传，另一台机器下次索取时再下载。

### 直接传输会保留版本历史吗？

本身不会——把文件复制到另一台机器，只是让两边都停在当前状态。Hoard 把每次游玩都抓成一个版本，这才让从损坏存档中回退成为可能。

### Hoard 也是开源的吗？

是的，AGPL-3.0，服务器也包含在内。自托管服务器就是我们自己在跑的那个二进制，不是删减版。
`,Wa=`---
title: "Palworld: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Palworld seine Welten auf PC und Steam Deck ablegt, was jede Datei ist, wie Koop-Welten funktionieren und wie du Spielstände sicherst oder umziehst."
order: 24
updated: 2026-10-02
---

Auf dem PC (Steam) legt Palworld seine Spielstände in \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<deine Steam-ID>\` ab, darin ein Ordner pro Welt. Diesen Pfad nennt Pocketpair in seiner offiziellen FAQ. Darunter findest du den Pfad auf dem Steam Deck, was jede Datei tut, was sich im Koop ändert und wie du deine Welten gesichert hältst.

## Wo Palworld seine Spielstände ablegt

- **Windows, Steam-Version:** \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<deine Steam-ID>\\<Welt-ID>\`
- **Steam Deck und Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`

Der Ordner mit der Steam-ID ist eine lange Zahl, die an dein Steam-Konto gebunden ist. Darin hat jede Welt, die du angelegt hast, einen eigenen Ordner mit einem langen Hexadezimalnamen. Auf dem Steam Deck läuft das Spiel über Proton, die Spielstände liegen also im Proton-Präfix, das Steam dafür anlegt; \`1623730\` ist die Steam-App-ID des Spiels.

Die Version aus der Xbox-App / dem Game Pass legt ihre Spielstände an einem anderen, verpackten Ort ab, und es sind nicht dieselben Dateien, die du zwischen Steam-Installationen kopieren würdest.

## Was im Ordner einer Welt liegt

- **\`Level.sav\`** ist die Welt selbst: deine Basis, die Karte, die dort platzierten Pals.
- **\`LevelMeta.sav\`** enthält Name und Zusammenfassung der Welt für das Menü.
- **\`Players\\\`** enthält eine \`.sav\` pro Spieler, der in dieser Welt war.
- **\`LocalData.sav\`** und **\`WorldOption.sav\`** enthalten lokale Daten und die Einstellungen der Welt.
- **\`backup\\\`** sind die automatischen Backups der Welt, die das Spiel selbst anlegt.

Die Einstellungen liegen woanders: \`Pal\\Saved\\Config\\Windows\\GameUserSettings.ini\` enthält Grafik und Steuerung, \`Pal\\Saved\\Logs\` die Logs. Beides gehört nicht zu deinem Fortschritt.

Sichere beim Backup **den ganzen Weltordner**, nicht nur \`Level.sav\`. Welt und Spielerdateien gehören zusammen, und wer eines ohne das andere wiederherstellt, hat am Ende Figuren, die nicht zu ihrer Welt passen.

## Koop und dedizierte Server

Im Koop **lebt die Welt auf dem PC des Hosts**. Deine Figur in dieser Welt ist eine Datei im \`Players\`-Ordner des Hosts, nicht auf deinem Rechner. Verliert der Host seinen Spielstand, geht der Fortschritt aller in dieser Welt mit. Auf einem dedizierten Server lebt die Welt auf dem Server.

Bei einer geteilten Welt braucht also der Ordner des Hosts das Backup.

## Hat Palworld Cloud-Saves?

Die Steam-Version nutzt Steam Cloud, die den neuesten Stand deiner Welten zwischen Rechnern mit demselben Konto synchron hält. Ältere Versionen behält sie nicht, und der \`backup\\\`-Ordner des Spiels liegt auf derselben Platte wie der Spielstand — eine kaputte Platte nimmt beide mit.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere deinen Steam-ID-Ordner aus \`SaveGames\` (er enthält alle deine Welten) an einen sicheren Ort.
3. Zum Wiederherstellen das Spiel schließen und den Ordner an dieselbe Stelle zurückkopieren.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) sichert deine Welten jedes Mal, wenn du aufhörst zu spielen, und behält jede Version außerhalb des Rechners — eine beschädigte Welt oder eine verlorene Platte ist also nicht das Ende einer Basis, an der du wochenlang gebaut hast. Außerdem synchronisiert es die Welten zwischen deinen PCs und einem Steam Deck.

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Palworld wird über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Hostest du Koop-Partien, ist das der Rechner, auf den es ankommt: Sichere den Host, und die geteilte Welt ist abgedeckt. Um eine Welt zurückzusetzen, [stelle eine ältere Version wieder her](/guides/restore-a-game-save).

<!-- faq -->

## Häufige Fragen

### Wo liegen die Spielstände von Palworld auf dem Steam Deck?

Im Proton-Präfix: \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`, im Ordner mit deiner Steam-ID.

### Wo wird meine Figur in der Welt eines Freundes gespeichert?

Auf dem PC des Hosts, im \`Players\`-Ordner dieser Welt. Dein PC behält keine Kopie von Welten, die jemand anderes hostet.

### Welche Datei ist meine Welt?

\`Level.sav\`, aber sichere den ganzen Weltordner: Spielerdateien und Welt gehören zusammen.

### Sichert Palworld meine Welt von selbst?

Es legt automatische Backups im \`backup\`-Ordner der Welt an. Die liegen auf derselben Platte wie der Spielstand, schützen also vor einem schlechten Spielstand, nicht vor dem Verlust der Platte.
`,Ia=`---
title: "Palworld save location (PC & Steam Deck)"
description: "Where Palworld keeps its worlds on PC and Steam Deck, what each file is, how co-op worlds work, and how to back up your saves or move them between PCs."
order: 24
updated: 2026-10-02
related: sync-game-saves-across-pcs, restore-a-game-save, steam-cloud-alternative
---

On PC (Steam), Palworld keeps its saves in \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<your Steam ID>\`, with one folder per world inside. That's the path Pocketpair gives in its official FAQ. Below is the Steam Deck path, what each file does, how co-op changes things, and how to keep your worlds backed up.

## Where Palworld keeps its saves

- **Windows, Steam version:** \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<your Steam ID>\\<world ID>\`
- **Steam Deck and Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`

The Steam ID folder is a long number tied to your Steam account. Inside it, each world you've created has its own folder with a long hexadecimal name. On a Steam Deck the game runs through Proton, so the saves sit inside the Proton prefix Steam keeps for it; \`1623730\` is the game's Steam app ID.

The Xbox app / Game Pass version keeps its saves in a different, packaged location, and they aren't the same files you'd copy between Steam installs.

## What's in a world folder

- **\`Level.sav\`** is the world itself: your base, the map, the Pals placed in it.
- **\`LevelMeta.sav\`** holds the world's name and summary for the menu.
- **\`Players\\\`** holds one \`.sav\` per player who has been in that world.
- **\`LocalData.sav\`** and **\`WorldOption.sav\`** hold local data and the world's settings.
- **\`backup\\\`** is the game's own automatic backups of the world.

Settings are elsewhere: \`Pal\\Saved\\Config\\Windows\\GameUserSettings.ini\` holds graphics and controls, and \`Pal\\Saved\\Logs\` holds logs. Neither is part of your progress.

When you back up, take the **whole world folder**, not just \`Level.sav\`. The world and the player files belong together, and restoring one without the other is how characters end up out of step with the world they're in.

## Co-op and dedicated servers

In co-op, **the world lives on the host's PC**. Your character in that world is a file in the host's \`Players\` folder, not on your machine. If the host loses their save, everyone's progress in that world goes with it. On a dedicated server, the world lives on the server.

So for a shared world, it's the host's folder that needs the backup.

## Does Palworld have cloud saves?

The Steam version uses Steam Cloud, which keeps the latest state of your worlds in step between machines on the same account. It doesn't keep older versions, and the game's own \`backup\\\` folder lives on the same disk as the save, so a dead drive takes both.

## Back it up by hand

1. Close the game completely.
2. Copy your Steam ID folder from \`SaveGames\` (it contains all your worlds) somewhere safe.
3. To restore, close the game and copy it back to the same place.

## Keep it backed up and in sync with Hoard

[Hoard](/download) backs up your worlds every time you stop playing and keeps every version off the machine, so a corrupted world or a lost drive isn't the end of a base you spent weeks on. It also syncs the worlds between your PCs and a Steam Deck.

1. Install Hoard and sign in, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. Palworld is detected from your Steam library and the community save database.
3. Play. When you quit, the first version appears in the history.

If you host co-op, this is the machine that matters: back up the host, and the shared world is covered. To roll a world back, [restore an earlier version](/guides/restore-a-game-save).

<!-- faq -->

## Frequently asked questions

### Where are Palworld saves on Steam Deck?

Inside the Proton prefix: \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`, in the folder named after your Steam ID.

### Where is my character in a friend's world saved?

On the host's PC, in that world's \`Players\` folder. Your own PC doesn't keep a copy of worlds hosted by someone else.

### Which file is my world?

\`Level.sav\`, but back up the whole world folder: the player files and the world are meant to stay together.

### Does Palworld back up my world on its own?

It keeps automatic backups in the world's \`backup\` folder. They're on the same disk as the save, so they protect against a bad save, not against losing the drive.
`,Ra=`---
title: "Dónde están las partidas de Palworld (PC y Steam Deck)"
description: "Dónde guarda Palworld sus mundos en PC y Steam Deck, qué es cada fichero, cómo funcionan los mundos cooperativos y cómo copiar tus partidas o moverlas de PC."
order: 24
updated: 2026-10-02
---

En PC (Steam), Palworld guarda sus partidas en \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<tu ID de Steam>\`, con una carpeta por mundo dentro. Es la ruta que da Pocketpair en su FAQ oficial. Debajo tienes la ruta de Steam Deck, qué hace cada fichero, cómo cambia el cooperativo y cómo tener tus mundos siempre copiados.

## Dónde guarda Palworld las partidas

- **Windows, versión de Steam:** \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<tu ID de Steam>\\<ID del mundo>\`
- **Steam Deck y Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`

La carpeta del ID de Steam es un número largo ligado a tu cuenta de Steam. Dentro, cada mundo que has creado tiene su propia carpeta con un nombre hexadecimal largo. En una Steam Deck el juego corre con Proton, así que las partidas están dentro del prefijo de Proton que Steam mantiene para él; \`1623730\` es el ID del juego en Steam.

La versión de la app de Xbox / Game Pass guarda sus partidas en otro sitio, empaquetado, y no son los mismos ficheros que copiarías entre instalaciones de Steam.

## Qué hay en la carpeta de un mundo

- **\`Level.sav\`** es el mundo en sí: tu base, el mapa, los Pals colocados en él.
- **\`LevelMeta.sav\`** guarda el nombre y el resumen del mundo para el menú.
- **\`Players\\\`** guarda un \`.sav\` por cada jugador que ha estado en ese mundo.
- **\`LocalData.sav\`** y **\`WorldOption.sav\`** guardan datos locales y los ajustes del mundo.
- **\`backup\\\`** son las copias automáticas del mundo que hace el propio juego.

Los ajustes están en otro sitio: \`Pal\\Saved\\Config\\Windows\\GameUserSettings.ini\` guarda gráficos y controles, y \`Pal\\Saved\\Logs\` los registros. Ninguno forma parte de tu progreso.

Al hacer la copia, llévate **la carpeta del mundo entera**, no sólo \`Level.sav\`. El mundo y los ficheros de jugador van juntos, y restaurar uno sin el otro es como acaban los personajes desacompasados con el mundo en el que están.

## Cooperativo y servidores dedicados

En cooperativo, **el mundo vive en el PC del anfitrión**. Tu personaje en ese mundo es un fichero en la carpeta \`Players\` del anfitrión, no en tu máquina. Si el anfitrión pierde su partida, el progreso de todos en ese mundo se va con ella. En un servidor dedicado, el mundo vive en el servidor.

Así que, en un mundo compartido, la copia que importa es la de la carpeta del anfitrión.

## ¿Palworld tiene partidas en la nube?

La versión de Steam usa Steam Cloud, que mantiene al día el último estado de tus mundos entre máquinas con la misma cuenta. No guarda versiones anteriores, y la carpeta \`backup\\\` del juego está en el mismo disco que la partida, así que un disco muerto se lleva las dos.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia tu carpeta del ID de Steam desde \`SaveGames\` (contiene todos tus mundos) a un sitio seguro.
3. Para restaurar, cierra el juego y vuelve a copiarla al mismo sitio.

## Copia y sincronización automáticas con Hoard

[Hoard](/download) copia tus mundos cada vez que dejas de jugar y guarda todas las versiones fuera de la máquina, así que un mundo corrupto o un disco perdido no es el final de una base en la que llevas semanas. También sincroniza los mundos entre tus PC y una Steam Deck.

1. Instala Hoard e inicia sesión, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. Palworld se detecta a partir de tu biblioteca de Steam y de la base de datos comunitaria de partidas.
3. Juega. Al salir, la primera versión aparece en el historial.

Si eres anfitrión en cooperativo, ésta es la máquina que importa: copia la del anfitrión y el mundo compartido queda cubierto. Para devolver un mundo atrás, [restaura una versión anterior](/guides/restore-a-game-save).

<!-- faq -->

## Preguntas frecuentes

### ¿Dónde están las partidas de Palworld en Steam Deck?

Dentro del prefijo de Proton: \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`, en la carpeta con tu ID de Steam.

### ¿Dónde se guarda mi personaje en el mundo de un amigo?

En el PC del anfitrión, en la carpeta \`Players\` de ese mundo. Tu PC no guarda copia de los mundos que aloja otra persona.

### ¿Qué fichero es mi mundo?

\`Level.sav\`, pero copia la carpeta del mundo entera: los ficheros de jugador y el mundo tienen que ir juntos.

### ¿Palworld hace copia de mi mundo por su cuenta?

Guarda copias automáticas en la carpeta \`backup\` del mundo. Están en el mismo disco que la partida, así que te protegen de una partida mala, no de perder el disco.
`,_a=`---
title: "Emplacement des sauvegardes de Palworld (PC et Steam Deck)"
description: "Où Palworld range ses mondes sur PC et Steam Deck, à quoi sert chaque fichier, comment marchent les mondes en coop et comment sauvegarder ou transférer vos parties."
order: 24
updated: 2026-10-02
---

Sur PC (Steam), Palworld range ses sauvegardes dans \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<votre identifiant Steam>\`, avec un dossier par monde. C'est le chemin que donne Pocketpair dans sa FAQ officielle. Vous trouverez ci-dessous le chemin sur Steam Deck, le rôle de chaque fichier, ce que change la coop et comment garder vos mondes sauvegardés.

## Où Palworld range ses sauvegardes

- **Windows, version Steam :** \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<votre identifiant Steam>\\<identifiant du monde>\`
- **Steam Deck et Linux** (Proton) : \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`

Le dossier de l'identifiant Steam est un long numéro lié à votre compte Steam. À l'intérieur, chaque monde créé a son propre dossier au long nom hexadécimal. Sur Steam Deck, le jeu passe par Proton, donc les sauvegardes sont dans le préfixe Proton que Steam garde pour lui ; \`1623730\` est l'identifiant Steam du jeu.

La version de l'application Xbox / Game Pass range ses sauvegardes ailleurs, dans un emplacement empaqueté, et ce ne sont pas les mêmes fichiers que ceux que vous copieriez entre installations Steam.

## Ce que contient le dossier d'un monde

- **\`Level.sav\`** est le monde lui-même : votre base, la carte, les Pals qui y sont placés.
- **\`LevelMeta.sav\`** contient le nom et le résumé du monde pour le menu.
- **\`Players\\\`** contient un \`.sav\` par joueur passé dans ce monde.
- **\`LocalData.sav\`** et **\`WorldOption.sav\`** contiennent des données locales et les réglages du monde.
- **\`backup\\\`** contient les sauvegardes automatiques du monde faites par le jeu.

Les réglages sont ailleurs : \`Pal\\Saved\\Config\\Windows\\GameUserSettings.ini\` pour les graphismes et les commandes, \`Pal\\Saved\\Logs\` pour les journaux. Aucun ne fait partie de votre progression.

Pour sauvegarder, prenez **tout le dossier du monde**, pas seulement \`Level.sav\`. Le monde et les fichiers des joueurs vont ensemble, et en restaurer un sans l'autre, c'est ainsi que les personnages se retrouvent décalés par rapport à leur monde.

## Coop et serveurs dédiés

En coop, **le monde vit sur le PC de l'hôte**. Votre personnage dans ce monde est un fichier du dossier \`Players\` de l'hôte, pas sur votre machine. Si l'hôte perd sa sauvegarde, la progression de tous dans ce monde disparaît avec. Sur un serveur dédié, le monde vit sur le serveur.

Pour un monde partagé, c'est donc le dossier de l'hôte qui doit être sauvegardé.

## Palworld a-t-il des sauvegardes cloud ?

La version Steam utilise Steam Cloud, qui garde le dernier état de vos mondes synchronisé entre les machines d'un même compte. Il ne garde pas les anciennes versions, et le dossier \`backup\\\` du jeu est sur le même disque que la sauvegarde : un disque mort emporte les deux.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez votre dossier d'identifiant Steam depuis \`SaveGames\` (il contient tous vos mondes) vers un endroit sûr.
3. Pour restaurer, fermez le jeu et recopiez-le au même endroit.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) sauvegarde vos mondes à chaque fois que vous arrêtez de jouer et garde toutes les versions hors de la machine : un monde corrompu ou un disque perdu n'est pas la fin d'une base construite pendant des semaines. Il synchronise aussi les mondes entre vos PC et un Steam Deck.

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Palworld est détecté à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes.
3. Jouez. En quittant, la première version apparaît dans l'historique.

Si vous hébergez en coop, c'est cette machine qui compte : sauvegardez l'hôte, et le monde partagé est couvert. Pour ramener un monde en arrière, [restaurez une version antérieure](/guides/restore-a-game-save).

<!-- faq -->

## Questions fréquentes

### Où sont les sauvegardes de Palworld sur Steam Deck ?

Dans le préfixe Proton : \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`, dans le dossier à votre identifiant Steam.

### Où est sauvegardé mon personnage dans le monde d'un ami ?

Sur le PC de l'hôte, dans le dossier \`Players\` de ce monde. Votre PC ne garde pas de copie des mondes hébergés par quelqu'un d'autre.

### Quel fichier est mon monde ?

\`Level.sav\`, mais sauvegardez tout le dossier du monde : les fichiers des joueurs et le monde vont ensemble.

### Palworld sauvegarde-t-il mon monde tout seul ?

Il garde des sauvegardes automatiques dans le dossier \`backup\` du monde. Elles sont sur le même disque que la sauvegarde : elles protègent d'une mauvaise sauvegarde, pas de la perte du disque.
`,Ta=`---
title: "Dove sono i salvataggi di Palworld (PC e Steam Deck)"
description: "Dove Palworld tiene i mondi su PC e Steam Deck, a cosa serve ogni file, come funzionano i mondi in co-op e come fare il backup dei salvataggi o spostarli tra PC."
order: 24
updated: 2026-10-02
---

Su PC (Steam), Palworld tiene i salvataggi in \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<il tuo ID Steam>\`, con una cartella per mondo all'interno. È il percorso che Pocketpair indica nella sua FAQ ufficiale. Qui sotto trovi il percorso su Steam Deck, a cosa serve ogni file, cosa cambia in co-op e come tenere al sicuro i tuoi mondi.

## Dove Palworld tiene i salvataggi

- **Windows, versione Steam:** \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<il tuo ID Steam>\\<ID del mondo>\`
- **Steam Deck e Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`

La cartella dell'ID Steam è un numero lungo legato al tuo account Steam. Al suo interno, ogni mondo che hai creato ha una propria cartella con un lungo nome esadecimale. Su Steam Deck il gioco gira con Proton, quindi i salvataggi stanno nel prefisso Proton che Steam tiene per lui; \`1623730\` è l'ID Steam del gioco.

La versione dell'app Xbox / Game Pass tiene i salvataggi in un'altra posizione, impacchettata, e non sono gli stessi file che copieresti tra installazioni Steam.

## Cosa c'è nella cartella di un mondo

- **\`Level.sav\`** è il mondo vero e proprio: la tua base, la mappa, i Pal piazzati lì.
- **\`LevelMeta.sav\`** contiene nome e riepilogo del mondo per il menu.
- **\`Players\\\`** contiene un \`.sav\` per ogni giocatore che è stato in quel mondo.
- **\`LocalData.sav\`** e **\`WorldOption.sav\`** contengono dati locali e le impostazioni del mondo.
- **\`backup\\\`** sono i backup automatici del mondo fatti dal gioco stesso.

Le impostazioni sono altrove: \`Pal\\Saved\\Config\\Windows\\GameUserSettings.ini\` per grafica e comandi, \`Pal\\Saved\\Logs\` per i log. Nessuno dei due fa parte dei tuoi progressi.

Per il backup prendi **l'intera cartella del mondo**, non solo \`Level.sav\`. Il mondo e i file dei giocatori vanno insieme, e ripristinarne uno senza l'altro è il modo in cui i personaggi finiscono fuori sincrono con il mondo in cui si trovano.

## Co-op e server dedicati

In co-op **il mondo vive sul PC dell'host**. Il tuo personaggio in quel mondo è un file nella cartella \`Players\` dell'host, non sulla tua macchina. Se l'host perde il salvataggio, i progressi di tutti in quel mondo se ne vanno con lui. Su un server dedicato, il mondo vive sul server.

Per un mondo condiviso, quindi, è la cartella dell'host a dover avere il backup.

## Palworld ha i salvataggi nel cloud?

La versione Steam usa Steam Cloud, che tiene allineato l'ultimo stato dei tuoi mondi tra le macchine con lo stesso account. Non tiene le versioni precedenti, e la cartella \`backup\\\` del gioco sta sullo stesso disco del salvataggio: un disco morto si porta via entrambi.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia la cartella del tuo ID Steam da \`SaveGames\` (contiene tutti i tuoi mondi) in un posto sicuro.
3. Per ripristinare, chiudi il gioco e ricopiala nello stesso posto.

## Backup e sync automatici con Hoard

[Hoard](/download) fa il backup dei tuoi mondi ogni volta che smetti di giocare e tiene tutte le versioni fuori dalla macchina, così un mondo corrotto o un disco perso non sono la fine di una base costruita in settimane. Sincronizza anche i mondi tra i tuoi PC e una Steam Deck.

1. Installa Hoard e accedi, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. Palworld viene rilevato dalla tua libreria Steam e dal database comunitario dei salvataggi.
3. Gioca. Quando esci, la prima versione compare nella cronologia.

Se fai da host in co-op, è questa la macchina che conta: fai il backup dell'host e il mondo condiviso è coperto. Per riportare indietro un mondo, [ripristina una versione precedente](/guides/restore-a-game-save).

<!-- faq -->

## Domande frequenti

### Dove sono i salvataggi di Palworld su Steam Deck?

Nel prefisso Proton: \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`, nella cartella col tuo ID Steam.

### Dove viene salvato il mio personaggio nel mondo di un amico?

Sul PC dell'host, nella cartella \`Players\` di quel mondo. Il tuo PC non tiene una copia dei mondi ospitati da altri.

### Quale file è il mio mondo?

\`Level.sav\`, ma fai il backup dell'intera cartella del mondo: i file dei giocatori e il mondo vanno insieme.

### Palworld fa il backup del mio mondo da solo?

Tiene backup automatici nella cartella \`backup\` del mondo. Stanno sullo stesso disco del salvataggio, quindi proteggono da un salvataggio rovinato, non dalla perdita del disco.
`,Ba=`---
title: "パルワールド（Palworld）のセーブデータの場所（PC・Steam Deck）"
description: "PalworldのワールドがPCとSteam Deckのどこに保存されるか、各ファイルの役割、協力プレイのワールドのしくみ、バックアップやPC間での移し方を解説。"
order: 24
updated: 2026-10-02
---

PC（Steam）版の Palworld は、セーブデータを \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<あなたの Steam ID>\` に保存し、その中にワールドごとのフォルダーがあります。Pocketpair が公式 FAQ で案内しているパスです。以下では Steam Deck でのパス、各ファイルの役割、協力プレイでの違い、そしてワールドのバックアップを保つ方法を説明します。

## Palworld のセーブデータの場所

- **Windows、Steam 版:** \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<あなたの Steam ID>\\<ワールド ID>\`
- **Steam Deck と Linux**（Proton）: \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`

Steam ID のフォルダーは、Steam アカウントに結びついた長い数字です。その中に、作成したワールドごとに長い 16 進数の名前のフォルダーがあります。Steam Deck ではゲームは Proton で動くため、セーブは Steam が用意する Proton プレフィックスの中にあります。\`1623730\` はこのゲームの Steam アプリ ID です。

Xbox アプリ / Game Pass 版は、別のパッケージ化された場所にセーブを保存しており、Steam のインストール間でコピーするファイルとは別物です。

## ワールドフォルダーの中身

- **\`Level.sav\`** はワールドそのものです。拠点、マップ、そこに配置したパル。
- **\`LevelMeta.sav\`** には、メニューに表示するワールド名と概要が入っています。
- **\`Players\\\`** には、そのワールドに入ったプレイヤーごとに \`.sav\` が 1 つずつあります。
- **\`LocalData.sav\`** と **\`WorldOption.sav\`** には、ローカルデータとワールドの設定が入っています。
- **\`backup\\\`** は、ゲーム自身が作るワールドの自動バックアップです。

設定は別の場所にあります。\`Pal\\Saved\\Config\\Windows\\GameUserSettings.ini\` にグラフィックや操作の設定、\`Pal\\Saved\\Logs\` にログがあります。どちらも進行の一部ではありません。

バックアップするときは、\`Level.sav\` だけでなく **ワールドフォルダー全体** をコピーしてください。ワールドとプレイヤーファイルはセットで、片方だけを復元すると、キャラクターとワールドの状態がずれてしまいます。

## 協力プレイと専用サーバー

協力プレイでは、**ワールドはホストの PC にあります**。そのワールドでのあなたのキャラクターは、あなたのマシンではなく、ホストの \`Players\` フォルダーにあるファイルです。ホストがセーブを失うと、そのワールドでの全員の進行も失われます。専用サーバーの場合、ワールドはサーバー上にあります。

つまり共有ワールドでバックアップが必要なのは、ホストのフォルダーです。

## Palworld にクラウドセーブはありますか？

Steam 版は Steam クラウドを使い、同じアカウントのマシン間でワールドの最新状態をそろえます。古いバージョンは残さず、ゲームの \`backup\\\` フォルダーもセーブと同じディスクにあるため、ディスクが壊れると両方とも失われます。

## 手動でバックアップする

1. ゲームを完全に終了します。
2. \`SaveGames\` の中の Steam ID フォルダー（すべてのワールドを含みます）を安全な場所にコピーします。
3. 復元するときは、ゲームを終了して同じ場所にコピーし戻します。

## Hoard で自動バックアップと同期

[Hoard](/download) は、プレイをやめるたびにワールドをバックアップし、すべてのバージョンをマシンの外に保持します。ワールドが壊れても、ドライブを失っても、何週間もかけた拠点が終わりになることはありません。さらにワールドを PC と Steam Deck の間で同期します。

1. Hoard をインストールしてサインインするか、[自分のサーバー](/guides/self-host-hoard)を指定します。
2. **ライブラリ** を開きます。Palworld は Steam ライブラリとコミュニティのセーブデータベースから検出されます。
3. プレイします。終了すると、最初のバージョンが履歴に表示されます。

協力プレイのホストをしているなら、大事なのはそのマシンです。ホストをバックアップすれば、共有ワールドもカバーされます。ワールドを巻き戻すには、[以前のバージョンを復元](/guides/restore-a-game-save)してください。

<!-- faq -->

## よくある質問

### Steam Deck での Palworld のセーブデータはどこですか？

Proton プレフィックスの中です: \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\` の、Steam ID の名前のフォルダーです。

### 友達のワールドでの自分のキャラクターはどこに保存されますか？

ホストの PC の、そのワールドの \`Players\` フォルダーです。他人がホストするワールドのコピーは、あなたの PC には残りません。

### どのファイルが自分のワールドですか？

\`Level.sav\` ですが、ワールドフォルダー全体をバックアップしてください。プレイヤーファイルとワールドはセットです。

### Palworld は自動でワールドをバックアップしますか？

ワールドの \`backup\` フォルダーに自動バックアップを残します。ただしセーブと同じディスクにあるので、守れるのは壊れたセーブからであって、ディスクの故障からではありません。
`,Na=`---
title: "Onde ficam os saves de Palworld (PC e Steam Deck)"
description: "Onde o Palworld guarda os mundos no PC e na Steam Deck, o que é cada ficheiro, como funcionam os mundos em co-op e como fazer backup dos saves ou mudá-los de PC."
order: 24
updated: 2026-10-02
---

No PC (Steam), o Palworld guarda os saves em \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<o teu ID do Steam>\`, com uma pasta por mundo lá dentro. É o caminho que a Pocketpair indica no seu FAQ oficial. Abaixo tens o caminho na Steam Deck, o que faz cada ficheiro, o que muda no co-op e como manter os teus mundos com backup.

## Onde o Palworld guarda os saves

- **Windows, versão Steam:** \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<o teu ID do Steam>\\<ID do mundo>\`
- **Steam Deck e Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`

A pasta do ID do Steam é um número longo ligado à tua conta Steam. Lá dentro, cada mundo que criaste tem a sua pasta com um nome hexadecimal longo. Na Steam Deck o jogo corre com o Proton, por isso os saves estão dentro do prefixo do Proton que o Steam mantém para ele; \`1623730\` é o ID do jogo no Steam.

A versão da app Xbox / Game Pass guarda os saves noutro sítio, empacotado, e não são os mesmos ficheiros que copiarias entre instalações do Steam.

## O que há na pasta de um mundo

- **\`Level.sav\`** é o mundo em si: a tua base, o mapa, os Pals lá colocados.
- **\`LevelMeta.sav\`** guarda o nome e o resumo do mundo para o menu.
- **\`Players\\\`** guarda um \`.sav\` por cada jogador que esteve nesse mundo.
- **\`LocalData.sav\`** e **\`WorldOption.sav\`** guardam dados locais e as definições do mundo.
- **\`backup\\\`** são os backups automáticos do mundo feitos pelo próprio jogo.

As definições estão noutro sítio: \`Pal\\Saved\\Config\\Windows\\GameUserSettings.ini\` guarda gráficos e controlos, e \`Pal\\Saved\\Logs\` os logs. Nenhum faz parte do teu progresso.

Ao fazer o backup, leva **a pasta do mundo inteira**, não só o \`Level.sav\`. O mundo e os ficheiros de jogador andam juntos, e restaurar um sem o outro é como as personagens acabam dessincronizadas do mundo em que estão.

## Co-op e servidores dedicados

No co-op, **o mundo vive no PC do anfitrião**. A tua personagem nesse mundo é um ficheiro na pasta \`Players\` do anfitrião, não na tua máquina. Se o anfitrião perder o save, o progresso de todos nesse mundo vai com ele. Num servidor dedicado, o mundo vive no servidor.

Por isso, num mundo partilhado, é a pasta do anfitrião que precisa de backup.

## O Palworld tem saves na nuvem?

A versão Steam usa o Steam Cloud, que mantém em dia o último estado dos teus mundos entre máquinas com a mesma conta. Não guarda versões anteriores, e a pasta \`backup\\\` do jogo está no mesmo disco que o save, por isso um disco morto leva os dois.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a tua pasta do ID do Steam de \`SaveGames\` (contém todos os teus mundos) para um sítio seguro.
3. Para restaurar, fecha o jogo e volta a copiá-la para o mesmo sítio.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) faz backup dos teus mundos sempre que deixas de jogar e guarda todas as versões fora da máquina, por isso um mundo corrompido ou um disco perdido não são o fim de uma base em que andas há semanas. Também sincroniza os mundos entre os teus PCs e uma Steam Deck.

1. Instala o Hoard e inicia sessão, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. O Palworld é detetado a partir da tua biblioteca Steam e da base de dados comunitária de saves.
3. Joga. Ao sair, a primeira versão aparece no histórico.

Se és anfitrião no co-op, esta é a máquina que importa: faz backup do anfitrião e o mundo partilhado fica coberto. Para recuar um mundo, [restaura uma versão anterior](/guides/restore-a-game-save).

<!-- faq -->

## Perguntas frequentes

### Onde ficam os saves de Palworld na Steam Deck?

Dentro do prefixo do Proton: \`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`, na pasta com o teu ID do Steam.

### Onde fica guardada a minha personagem no mundo de um amigo?

No PC do anfitrião, na pasta \`Players\` desse mundo. O teu PC não guarda cópia dos mundos alojados por outra pessoa.

### Que ficheiro é o meu mundo?

O \`Level.sav\`, mas faz backup da pasta do mundo inteira: os ficheiros de jogador e o mundo andam juntos.

### O Palworld faz backup do meu mundo sozinho?

Guarda backups automáticos na pasta \`backup\` do mundo. Estão no mesmo disco que o save, por isso protegem-te de um save mau, não de perder o disco.
`,Va=`---
title: "幻兽帕鲁（Palworld）存档位置（PC 与 Steam Deck）"
description: "Palworld 的世界在 PC 和 Steam Deck 上存放在哪里，每个文件的作用，联机世界如何运作，以及如何备份存档或在 PC 之间迁移。"
order: 24
updated: 2026-10-02
---

在 PC（Steam）上，Palworld 把存档放在 \`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<你的 Steam ID>\`，里面每个世界一个文件夹。这是 Pocketpair 在官方 FAQ 中给出的路径。下面是 Steam Deck 上的路径、每个文件的作用、联机时有什么不同，以及如何让你的世界一直有备份。

## Palworld 的存档位置

- **Windows，Steam 版：**\`%LOCALAPPDATA%\\Pal\\Saved\\SaveGames\\<你的 Steam ID>\\<世界 ID>\`
- **Steam Deck 和 Linux**（Proton）：\`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\`

Steam ID 文件夹是一个与你的 Steam 账号绑定的长数字。里面每个你创建的世界都有自己的文件夹，名字是一长串十六进制字符。在 Steam Deck 上游戏通过 Proton 运行，所以存档位于 Steam 为它维护的 Proton 前缀中；\`1623730\` 是游戏的 Steam 应用 ID。

Xbox 应用 / Game Pass 版把存档放在另一个打包好的位置，与在 Steam 安装之间复制的文件不是一回事。

## 世界文件夹里有什么

- **\`Level.sav\`** 就是世界本身：你的据点、地图、放置在其中的帕鲁。
- **\`LevelMeta.sav\`** 保存世界在菜单中显示的名称和概要。
- **\`Players\\\`** 为每个进入过该世界的玩家保存一个 \`.sav\`。
- **\`LocalData.sav\`** 和 **\`WorldOption.sav\`** 保存本地数据和世界设置。
- **\`backup\\\`** 是游戏自己为世界做的自动备份。

设置在别处：\`Pal\\Saved\\Config\\Windows\\GameUserSettings.ini\` 保存画面和操作设置，\`Pal\\Saved\\Logs\` 保存日志。它们都不属于你的进度。

备份时请复制**整个世界文件夹**，而不只是 \`Level.sav\`。世界和玩家文件是一体的，只恢复其中一个，角色就会和所在的世界对不上。

## 联机与专用服务器

联机时，**世界保存在房主的 PC 上**。你在那个世界里的角色是房主 \`Players\` 文件夹中的一个文件，而不在你的设备上。如果房主丢了存档，所有人在那个世界的进度都会一起丢失。在专用服务器上，世界保存在服务器上。

所以对于共享的世界，需要备份的是房主的文件夹。

## Palworld 有云存档吗？

Steam 版使用 Steam 云，会在同一账号的设备之间同步世界的最新状态。它不保留旧版本，而游戏自带的 \`backup\\\` 文件夹和存档在同一块硬盘上，硬盘坏了两者会一起没。

## 手动备份

1. 完全关闭游戏。
2. 把 \`SaveGames\` 中你的 Steam ID 文件夹（包含你所有的世界）复制到安全的地方。
3. 恢复时，关闭游戏，把它复制回原来的位置。

## 用 Hoard 自动备份和同步

[Hoard](/download) 会在你每次停止游戏时备份你的世界，并把每个版本保存在本机之外，所以世界损坏或硬盘丢失，都不会让你花了几周建起来的据点就此完蛋。它还会在你的 PC 和 Steam Deck 之间同步这些世界。

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**。Palworld 会通过你的 Steam 库和社区存档数据库被识别出来。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

如果你是联机房主，最重要的就是这台设备：备份好房主，共享的世界也就有了保障。要把世界回滚，请[恢复旧版本](/guides/restore-a-game-save)。

<!-- faq -->

## 常见问题

### Steam Deck 上的 Palworld 存档在哪里？

在 Proton 前缀里：\`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/\` 下以你的 Steam ID 命名的文件夹。

### 我在朋友世界里的角色保存在哪里？

在房主的 PC 上，那个世界的 \`Players\` 文件夹里。你的 PC 不会保留别人托管的世界的副本。

### 哪个文件是我的世界？

\`Level.sav\`，但请备份整个世界文件夹：玩家文件和世界是一体的。

### Palworld 会自动备份我的世界吗？

它会在世界的 \`backup\` 文件夹里保留自动备份。但它们和存档在同一块硬盘上，所以只能防存档损坏，防不了硬盘丢失。
`,Ua=`---
title: "So stellst du einen alten Spielstand wieder her"
description: "Kaputter Spielstand, fehlerhafte Mod oder eine bereute Entscheidung? Setz deinen Spielstand Schritt für Schritt zurück, ohne den aktuellen zu verlieren."
order: 3
updated: 2026-09-01
---

Eine schlechte Entscheidung im Spiel, eine beschädigte Datei oder ein verpfuschter Mod — manchmal musst du einfach zurück. Da Hoard eine vollständige Versionshistorie jedes Stands führt, dauert die Wiederherstellung eines früheren nur Sekunden.

## Eine frühere Version wiederherstellen

1. Öffne **Hoard** und gehe zum Spiel in deiner **Bibliothek**.
2. Öffne den Reiter **Historie**. Du siehst jedes Backup mit Datum und Größe.
3. Wähle die gewünschte Version und klicke auf **Wiederherstellen**.
4. Hoard schreibt diesen Snapshot zurück in den Speicherordner des Spiels. Dein aktueller Stand wird zuerst gesichert, die Wiederherstellung ist also umkehrbar.

## Auf einem neuen oder neu installierten PC wiederherstellen

1. Installiere Hoard und melde dich mit deinem Konto an.
2. Füge das Spiel zu deiner Bibliothek hinzu — Hoard findet das passende Cloud-Backup.
3. Stelle die neueste Version oder eine ältere wieder her und spiele weiter.

Da Hoard Speicherordner mit derselben Community-Datenbank wie Ludusavi findet, weiß es selbst bei einer Neuinstallation, wohin ein wiederhergestellter Stand gehört — ohne manuelle Pfadsuche.

## Wenn ein Spielstand beschädigt ist oder ein Mod ihn zerlegt hat

Ein Spiel, das beim Laden abstürzt, ein Mod, der etwas überschrieben hat, ein Autosave mitten im Schreibvorgang: die Lösung ist dieselbe. Öffne die **Historie** des Spiels, wähle die letzte Version von vor dem Problem und stelle sie wieder her. Datum und Größe reichen meist, um den Moment zu finden — ein plötzlicher Größensturz ist ein gutes Zeichen dafür, dass ein Stand abgeschnitten wurde.

Wenn du nicht sicher bist, welche die richtige ist, stelle die wahrscheinlichste wieder her und prüfe es im Spiel. Ein zweiter Versuch kostet nichts, denn die eben ersetzte Version wurde ebenfalls behalten.

## Was beim Wiederherstellen tatsächlich passiert

Drei Dinge, die man wissen sollte, denn sie machen einen Versuch gefahrlos:

1. **Dein aktueller Stand wird zuerst gesichert.** Die Wiederherstellung ist umkehrbar: das Ersetzte wird eine Version in der Historie wie jede andere.
2. **Es wird nur geladen, was fehlt.** Dateien, die mit dem richtigen Inhalt schon auf der Platte liegen, werden so verwendet — einen großen Spielstand nach einer kleinen Änderung wiederherzustellen bewegt ein paar Megabyte statt des ganzen Ordners.
3. **Dateien dieses Rechners bleiben unangetastet.** Konfiguration und Logs neben dem Spielstand werden gesichert, aber nicht über deine lokalen Kopien geschrieben: Tastenbelegung und Grafikeinstellungen überleben eine Wiederherstellung von einem anderen PC.

## Wiederherstellen ohne unsere Server

Wenn du deinen eigenen \`hoard-server\` betreibst, funktioniert das Wiederherstellen genauso, nur kommen die Versionen von deiner Maschine statt von unserer. Es gibt kein Konto bei uns, keine Telemetrie zu uns und nichts, was über unsere Server läuft. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

## Tipp

Wiederherstellungen sind nie zerstörerisch: Der ersetzte Stand wird zuerst als neue Version erfasst, du kannst eine Wiederherstellung also immer rückgängig machen, indem du den vorherigen Eintrag wiederherstellst. Hast du bisher nur lokale Backups geführt (etwa mit Ludusavi), ergänzt der Wechsel zu Hoard eine geräteunabhängige, versionierte Historie, aus der du selbst nach einem Festplattenausfall wiederherstellen kannst.

<!-- faq -->

## Häufige Fragen

### Überschreibt eine Wiederherstellung meinen aktuellen Fortschritt?

Erst nachdem dein aktueller Stand als neue Version gesichert wurde. Hast du die falsche gewählt, stelle den vorherigen Eintrag wieder her und du bist zurück am Ausgangspunkt.

### Wie weit reicht die Historie zurück?

So weit, wie das Versionslimit deines Tarifs erlaubt, und eine angeheftete Version wird nie weggeräumt, um Platz zu schaffen. Auf einem selbst gehosteten Server ist die einzige Grenze deine Platte.

### Kann ich auf einen PC wiederherstellen, auf dem das Spiel noch nicht installiert ist?

Installiere zuerst das Spiel, damit sein Speicherordner existiert, und stelle dann wieder her. Hoard weiß, wo jedes Spiel seine Stände erwartet, und schreibt den Snapshot an die richtige Stelle, ohne dass du den Pfad suchen musst.

### Klappt das zwischen Windows und einem Steam Deck?

Ja. Dasselbe Spiel legt seinen Stand auf beiden Geräten woanders ab — auf dem Deck im Proton-Prefix — und Hoard schreibt die wiederhergestellte Version dorthin, wo diese Maschine sie erwartet.

### Ist die Wiederherstellung auf einem selbst gehosteten Server anders?

Nein. Gleiche App, gleiche Historie, gleiche Wiederherstellung per Klick. Nur der Speicher gehört dir.
`,Fa=`---
title: "How to restore an old game save"
description: "Corrupted save, bad mod or a choice you regret? Roll a game save back to an earlier version, step by step, without losing what's on your PC now."
order: 3
updated: 2026-09-01
related: back-up-game-saves, steam-cloud-alternative, sync-game-saves-across-pcs
---

A bad decision in-game, a corrupted file, or a botched mod — sometimes you just need to go back. Because Hoard keeps a full version history of every save, restoring an earlier one takes seconds.

## Restore a previous version

1. Open **Hoard** and go to the game in your **Library**.
2. Open its **History** tab. You'll see every backup with its date and size.
3. Pick the version you want and choose **Restore**.
4. Hoard writes that snapshot back into the game's save folder. Your current save is backed up first, so the restore itself is reversible.

## Restore on a new or reinstalled PC

1. Install Hoard and sign in with your account.
2. Add the game to your Library — Hoard finds the matching cloud backup.
3. Restore the latest version, or any older one, and keep playing.

Because Hoard locates save folders using the same community database as Ludusavi, it knows where to put a restored save even on a fresh install — no manual path hunting.

## When a save is corrupted or a mod broke it

A game that crashes on load, a mod that rewrote something it shouldn't, an autosave that landed halfway through a write: the fix is the same. Open the game's **History**, pick the last version from before the problem started, and restore it. Dates and sizes are usually enough to spot the moment things went wrong — a sudden drop in size is a good sign that a save got truncated.

If you're not sure which version is the good one, restore the most likely candidate and check in-game. Trying again costs nothing, because the version you just replaced was kept too.

## What a restore actually does

Three things worth knowing, because they are what make a restore safe to try:

1. **Your current save is captured first.** The restore is reversible: whatever you replaced becomes a version in the history like any other.
2. **Only what's missing is downloaded.** Files already on disk with the right content are used as they are, so restoring a large save after a small change moves a few megabytes instead of the whole folder.
3. **Files that belong to this machine are left alone.** Configuration and logs sitting next to the save are backed up, but not written over your local copies — your key bindings and graphics settings survive a restore that came from another PC.

## Restoring without our servers

If you run your own \`hoard-server\`, restores work exactly the same way, except the versions come from your machine instead of ours. There is no account with us, no telemetry to us and nothing passing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

## Tip

Restores are never destructive: the save you replace is captured as a new version first, so you can always undo a restore by restoring the previous entry. If you've only ever kept local backups (for example with Ludusavi), moving to Hoard adds an off-machine, versioned history you can restore from even after a disk failure.

<!-- faq -->

## Frequently asked questions

### Will restoring overwrite my current progress?

Only after your current save has been captured as a new version. If you restore the wrong one, restore the previous entry and you're back where you started.

### How far back does the history go?

As far as the version limit on your plan allows, and a version you pin is never pruned to make room. On a self-hosted server the only limit is your disk.

### Can I restore to a PC where the game isn't installed yet?

Install the game first so its save folder exists, then restore. Hoard knows where each game expects its saves, so it writes the snapshot to the right place without you hunting for the path.

### Does restoring work between Windows and a Steam Deck?

Yes. The same game keeps its save in different places on each — on the Deck, inside the Proton prefix — and Hoard writes the restored version wherever that machine expects it.

### Is a restore any different on a self-hosted server?

No. Same app, same history, same one-click restore. Only the storage is yours.
`,Ka=`---
title: "Cómo restaurar una partida guardada anterior"
description: "¿Partida corrupta, un mod roto o una decisión que lamentas? Vuelve a una versión anterior de tu partida, paso a paso, sin perder lo que tienes ahora."
order: 3
updated: 2026-09-01
---

Una mala decisión en el juego, un archivo corrupto o un mod que lo rompe todo: a veces solo necesitas volver atrás. Como Hoard guarda un historial completo de versiones de cada partida, restaurar una anterior lleva segundos.

## Restaurar una versión anterior

1. Abre **Hoard** y ve al juego en tu **Biblioteca**.
2. Abre su pestaña **Historial**. Verás cada copia con su fecha y tamaño.
3. Elige la versión que quieras y pulsa **Restaurar**.
4. Hoard vuelve a escribir esa instantánea en la carpeta de guardado del juego. Tu partida actual se respalda primero, así que la restauración es reversible.

## Restaurar en un PC nuevo o reinstalado

1. Instala Hoard e inicia sesión con tu cuenta.
2. Añade el juego a tu Biblioteca: Hoard encuentra la copia en la nube correspondiente.
3. Restaura la última versión, o cualquiera anterior, y sigue jugando.

Como Hoard localiza las carpetas de guardado con la misma base de datos comunitaria que Ludusavi, sabe dónde colocar una partida restaurada incluso en una instalación limpia, sin que busques rutas a mano.

## Cuando una partida se corrompe o un mod la rompe

Un juego que se cierra al cargar, un mod que reescribió lo que no debía, un autoguardado que cayó a mitad de escritura: la solución es la misma. Abre el **Historial** del juego, elige la última versión anterior al problema y restáurala. Las fechas y los tamaños suelen bastar para ver dónde se torció: una caída brusca de tamaño es buena señal de que una partida quedó truncada.

Si no tienes claro cuál es la buena, restaura la candidata más probable y compruébalo dentro del juego. Volver a intentarlo no cuesta nada, porque la versión que acabas de reemplazar también se guardó.

## Qué hace realmente una restauración

Tres cosas que conviene saber, porque son las que hacen que restaurar sea seguro:

1. **Tu partida actual se captura primero.** La restauración es reversible: lo que reemplazaste pasa a ser una versión más del historial.
2. **Sólo se descarga lo que falta.** Los ficheros que ya están en disco con el contenido correcto se aprovechan tal cual, así que restaurar una partida grande después de un cambio pequeño mueve unos megas y no la carpeta entera.
3. **Los ficheros propios de esta máquina no se tocan.** La configuración y los registros que viven junto a la partida se copian, pero no se escriben encima de los tuyos: tus controles y tus ajustes gráficos sobreviven a una restauración que venga de otro PC.

## Restaurar sin pasar por nuestros servidores

Si levantas tu propio \`hoard-server\`, las restauraciones funcionan exactamente igual, sólo que las versiones vienen de tu máquina y no de la nuestra. No hay cuenta con nosotros, ni telemetría hacia nosotros, ni nada que pase por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

## Consejo

Las restauraciones nunca son destructivas: la partida que reemplazas se guarda antes como una nueva versión, así que siempre puedes deshacer una restauración volviendo a la entrada anterior. Si hasta ahora solo guardabas copias en local (por ejemplo con Ludusavi), pasar a Hoard añade un historial versionado y fuera del equipo desde el que puedes restaurar incluso tras un fallo de disco.

<!-- faq -->

## Preguntas frecuentes

### ¿Restaurar sobrescribe mi progreso actual?

Sólo después de que tu partida actual se haya capturado como una versión nueva. Si restauras la equivocada, restaura la entrada anterior y vuelves al punto de partida.

### ¿Hasta dónde llega el historial?

Hasta donde permita el tope de versiones de tu plan, y una versión que fijes no se poda nunca para hacer sitio. En un servidor autoalojado el único límite es tu disco.

### ¿Puedo restaurar en un PC donde el juego todavía no está instalado?

Instala primero el juego para que exista su carpeta de partidas, y luego restaura. Hoard sabe dónde espera cada juego sus saves, así que escribe la instantánea en el sitio correcto sin que tengas que buscar la ruta.

### ¿Funciona restaurar entre Windows y una Steam Deck?

Sí. El mismo juego guarda en sitios distintos en cada uno — en la Deck, dentro del prefijo de Proton — y Hoard escribe la versión restaurada donde esa máquina la espera.

### ¿Cambia algo restaurar en un servidor autoalojado?

No. Misma aplicación, mismo historial, misma restauración de un clic. Lo único tuyo es el almacenamiento.
`,Qa=`---
title: "Comment restaurer une ancienne sauvegarde"
description: "Sauvegarde corrompue, mod cassé ou choix regretté ? Revenez à une version antérieure de votre partie, pas à pas, sans perdre l'état actuel."
order: 3
updated: 2026-09-01
---

Une mauvaise décision en jeu, un fichier corrompu ou un mod qui casse tout — parfois, il faut juste revenir en arrière. Comme Hoard conserve un historique complet des versions de chaque sauvegarde, en restaurer une plus ancienne prend quelques secondes.

## Restaurer une version précédente

1. Ouvrez **Hoard** et allez au jeu dans votre **Bibliothèque**.
2. Ouvrez son onglet **Historique**. Vous verrez chaque sauvegarde avec sa date et sa taille.
3. Choisissez la version voulue et cliquez sur **Restaurer**.
4. Hoard réécrit cet instantané dans le dossier de sauvegarde du jeu. Votre sauvegarde actuelle est d'abord sauvegardée, la restauration est donc réversible.

## Restaurer sur un PC neuf ou réinstallé

1. Installez Hoard et connectez-vous avec votre compte.
2. Ajoutez le jeu à votre Bibliothèque — Hoard trouve la sauvegarde cloud correspondante.
3. Restaurez la dernière version, ou une plus ancienne, et continuez à jouer.

Comme Hoard localise les dossiers de sauvegarde avec la même base communautaire que Ludusavi, il sait où placer une sauvegarde restaurée même sur une installation neuve — sans chasse manuelle au chemin.

## Quand une sauvegarde est corrompue ou qu'un mod l'a cassée

Un jeu qui plante au chargement, un mod qui a réécrit ce qu'il ne fallait pas, une sauvegarde automatique tombée en plein milieu d'une écriture : le remède est le même. Ouvrez l'**Historique** du jeu, choisissez la dernière version d'avant le problème et restaurez-la. Les dates et les tailles suffisent en général à repérer le moment où ça a dérapé — une chute brutale de taille indique souvent une sauvegarde tronquée.

Si vous ne savez pas laquelle est la bonne, restaurez la candidate la plus probable et vérifiez en jeu. Recommencer ne coûte rien, puisque la version que vous venez de remplacer a été conservée elle aussi.

## Ce que fait réellement une restauration

Trois choses à savoir, car ce sont elles qui rendent l'essai sans risque :

1. **Votre sauvegarde actuelle est capturée d'abord.** La restauration est réversible : ce que vous avez remplacé devient une version de l'historique comme une autre.
2. **Seul ce qui manque est téléchargé.** Les fichiers déjà présents avec le bon contenu sont réutilisés tels quels : restaurer une grosse sauvegarde après une petite modification déplace quelques mégaoctets, pas tout le dossier.
3. **Les fichiers propres à cette machine ne sont pas touchés.** La configuration et les journaux voisins de la sauvegarde sont sauvegardés, mais pas réécrits par-dessus vos copies locales : vos touches et vos réglages graphiques survivent à une restauration venue d'un autre PC.

## Restaurer sans passer par nos serveurs

Si vous faites tourner votre propre \`hoard-server\`, les restaurations fonctionnent exactement pareil, sauf que les versions viennent de votre machine et non de la nôtre. Aucun compte chez nous, aucune télémétrie vers nous, rien qui passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

## Astuce

Les restaurations ne sont jamais destructrices : la sauvegarde remplacée est d'abord capturée comme nouvelle version, vous pouvez donc toujours annuler une restauration en restaurant l'entrée précédente. Si vous n'aviez que des sauvegardes locales (par exemple avec Ludusavi), passer à Hoard ajoute un historique versionné hors machine, depuis lequel vous pouvez restaurer même après une panne de disque.

<!-- faq -->

## Questions fréquentes

### Une restauration écrase-t-elle ma progression actuelle ?

Seulement après que votre sauvegarde actuelle a été capturée comme nouvelle version. Si vous restaurez la mauvaise, restaurez l'entrée précédente et vous revoilà au point de départ.

### Jusqu'où remonte l'historique ?

Aussi loin que le permet la limite de versions de votre offre, et une version épinglée n'est jamais supprimée pour faire de la place. Sur un serveur auto-hébergé, la seule limite est votre disque.

### Puis-je restaurer sur un PC où le jeu n'est pas encore installé ?

Installez d'abord le jeu pour que son dossier de sauvegarde existe, puis restaurez. Hoard sait où chaque jeu attend ses sauvegardes et écrit l'instantané au bon endroit, sans chasse au chemin.

### Est-ce que ça marche entre Windows et un Steam Deck ?

Oui. Le même jeu range sa sauvegarde à des endroits différents sur chacun — sur le Deck, dans le préfixe Proton — et Hoard écrit la version restaurée là où cette machine l'attend.

### Une restauration est-elle différente sur un serveur auto-hébergé ?

Non. Même application, même historique, même restauration en un clic. Seul le stockage est à vous.
`,Xa=`---
title: "Come ripristinare un vecchio salvataggio"
description: "Salvataggio corrotto, mod difettosa o una scelta di cui ti penti? Torna a una versione precedente, passo dopo passo, senza perdere quella attuale."
order: 3
updated: 2026-09-01
---

Una brutta decisione nel gioco, un file corrotto o una mod che rompe tutto — a volte devi solo tornare indietro. Poiché Hoard conserva una cronologia completa delle versioni di ogni salvataggio, ripristinarne uno precedente richiede pochi secondi.

## Ripristinare una versione precedente

1. Apri **Hoard** e vai al gioco nella tua **Libreria**.
2. Apri la scheda **Cronologia**. Vedrai ogni backup con data e dimensione.
3. Scegli la versione che vuoi e premi **Ripristina**.
4. Hoard riscrive quello snapshot nella cartella di salvataggio del gioco. Il salvataggio attuale viene salvato prima, quindi il ripristino è reversibile.

## Ripristinare su un PC nuovo o reinstallato

1. Installa Hoard e accedi con il tuo account.
2. Aggiungi il gioco alla Libreria — Hoard trova il backup cloud corrispondente.
3. Ripristina l'ultima versione, o una più vecchia, e continua a giocare.

Poiché Hoard individua le cartelle di salvataggio con lo stesso database comunitario di Ludusavi, sa dove mettere un salvataggio ripristinato anche su un'installazione pulita — senza cercare percorsi a mano.

## Quando un salvataggio è corrotto o l'ha rotto una mod

Un gioco che crasha al caricamento, una mod che ha riscritto ciò che non doveva, un salvataggio automatico caduto a metà scrittura: il rimedio è lo stesso. Apri la **Cronologia** del gioco, scegli l'ultima versione precedente al problema e ripristinala. Date e dimensioni bastano di solito a individuare il momento in cui è andata storta: un calo improvviso di dimensione è un buon indizio di un salvataggio troncato.

Se non sai quale sia quella buona, ripristina la candidata più probabile e verifica nel gioco. Riprovare non costa nulla, perché anche la versione appena sostituita è stata conservata.

## Cosa fa davvero un ripristino

Tre cose da sapere, perché sono quelle che rendono sicuro provarci:

1. **Il salvataggio attuale viene catturato per primo.** Il ripristino è reversibile: ciò che hai sostituito diventa una versione della cronologia come tutte le altre.
2. **Si scarica solo ciò che manca.** I file già su disco con il contenuto giusto vengono usati così come sono, quindi ripristinare un salvataggio grande dopo una piccola modifica sposta qualche megabyte e non l'intera cartella.
3. **I file che appartengono a questa macchina restano intatti.** Configurazione e log accanto al salvataggio vengono salvati, ma non riscritti sopra le tue copie locali: i tuoi comandi e le tue impostazioni grafiche sopravvivono a un ripristino arrivato da un altro PC.

## Ripristinare senza passare dai nostri server

Se fai girare il tuo \`hoard-server\`, i ripristini funzionano esattamente allo stesso modo, solo che le versioni arrivano dalla tua macchina invece che dalla nostra. Nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

## Suggerimento

I ripristini non sono mai distruttivi: il salvataggio che sostituisci viene prima catturato come nuova versione, quindi puoi sempre annullare un ripristino ripristinando la voce precedente. Se finora hai tenuto solo backup locali (ad esempio con Ludusavi), passare a Hoard aggiunge una cronologia versionata fuori dalla macchina, da cui puoi ripristinare anche dopo un guasto del disco.

<!-- faq -->

## Domande frequenti

### Il ripristino sovrascrive i miei progressi attuali?

Solo dopo che il salvataggio attuale è stato catturato come nuova versione. Se ripristini quella sbagliata, ripristina la voce precedente e sei di nuovo al punto di partenza.

### Fin dove arriva la cronologia?

Fin dove lo consente il limite di versioni del tuo piano, e una versione che fissi non viene mai eliminata per fare spazio. Su un server self-hosted l'unico limite è il tuo disco.

### Posso ripristinare su un PC dove il gioco non è ancora installato?

Installa prima il gioco, così esiste la sua cartella dei salvataggi, poi ripristina. Hoard sa dove ogni gioco si aspetta i salvataggi e scrive lo snapshot nel posto giusto senza che tu debba cercare il percorso.

### Funziona tra Windows e una Steam Deck?

Sì. Lo stesso gioco tiene il salvataggio in posti diversi sui due — sulla Deck, dentro il prefisso Proton — e Hoard scrive la versione ripristinata dove quella macchina se l'aspetta.

### Il ripristino cambia su un server self-hosted?

No. Stessa app, stessa cronologia, stesso ripristino in un clic. L'unica cosa tua è lo spazio di archiviazione.
`,$a=`---
title: "古いセーブデータを復元する方法"
description: "セーブ破損、壊れたMod、後悔した選択。今のデータを失わずに、セーブを以前のバージョンへ戻す手順を解説します。"
order: 3
updated: 2026-09-01
---

ゲーム内での悪い決断、壊れたファイル、失敗した MOD――時にはただ巻き戻したいだけのことがあります。Hoard はすべてのセーブの完全なバージョン履歴を保持しているので、以前のものへの復元は数秒で済みます。

## 以前のバージョンを復元する

1. **Hoard** を開き、**ライブラリ** で対象のゲームに移動します。
2. その **履歴** タブを開きます。各バックアップが日付とサイズ付きで表示されます。
3. 復元したいバージョンを選び、**復元** を選択します。
4. Hoard はそのスナップショットをゲームのセーブフォルダーに書き戻します。現在のセーブが先にバックアップされるため、復元自体も元に戻せます。

## 新しい PC や再インストールした PC で復元する

1. Hoard をインストールし、自分のアカウントでサインインします。
2. ゲームをライブラリに追加します――Hoard が対応するクラウドバックアップを見つけます。
3. 最新版、または任意の古い版を復元して、プレイを続けます。

Hoard は Ludusavi と同じコミュニティデータベースでセーブフォルダーを特定するため、クリーンインストールでも復元先を把握しています。手動でパスを探す必要はありません。

## セーブが壊れたとき、Mod が壊したとき

読み込みで落ちるゲーム、書き換えてはいけないものを書き換えた Mod、書き込みの途中で保存されたオートセーブ。対処はどれも同じです。そのゲームの **履歴** を開き、問題が起きる前の最後の世代を選んで復元します。日付とサイズだけで、どこでおかしくなったかはたいてい分かります。サイズが急に落ちていれば、セーブが途中で切れた良い手がかりです。

どれが無事な世代か分からないときは、いちばんそれらしいものを復元してゲームで確かめてください。やり直しに費用はかかりません。いま置き換えた世代も残っているからです。

## 復元で実際に起きること

知っておく価値のある 3 点です。ここが、気軽に試せる理由になります。

1. **いまのセーブが先に取り込まれます。** 復元は取り消せます。置き換えたものは、履歴の中のひとつの世代になります。
2. **足りないものだけをダウンロードします。** 正しい内容ですでにディスクにあるファイルはそのまま使われるため、小さな変更のあとに大きなセーブを復元しても、動くのはフォルダー全体ではなく数メガバイトです。
3. **そのマシンに属するファイルには触れません。** セーブの隣にある設定やログはバックアップされますが、あなたのローカルの内容を上書きすることはありません。別の PC から復元しても、キー割り当てやグラフィック設定はそのまま残ります。

## 当方のサーバーを介さない復元

自分で \`hoard-server\` を動かしている場合も、復元の動きはまったく同じで、世代の出どころが当方ではなく自分のマシンになるだけです。当方のアカウントも、当方へのテレメトリも、当方のサーバーを通るものもありません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

## ヒント

復元が破壊的になることはありません。置き換えるセーブは先に新しいバージョンとして取り込まれるので、直前のエントリを復元すればいつでも復元を取り消せます。これまでローカルバックアップ（たとえば Ludusavi）しか持っていなかった場合、Hoard に移行するとマシン外の世代履歴が加わり、ディスク故障の後でもそこから復元できます。

<!-- faq -->

## よくある質問

### 復元すると今の進行は上書きされますか？

いまのセーブが新しい世代として取り込まれたあとに限り、上書きされます。選び間違えたら、ひとつ前の項目を復元すれば元の状態に戻ります。

### 履歴はどこまで遡れますか？

プランの世代数の上限まで遡れます。ピン留めした世代は、空きを作るために削除されることはありません。セルフホストのサーバーなら、上限はディスクの容量だけです。

### ゲームがまだ入っていない PC に復元できますか？

先にゲームをインストールしてセーブ用フォルダーを作ってから復元してください。Hoard は各ゲームがセーブをどこに置くかを把握しているので、パスを探さなくても正しい場所に書き込みます。

### Windows と Steam Deck のあいだでも復元できますか？

はい。同じゲームでも保存場所は両者で異なり、Deck では Proton のプレフィックスの中にあります。Hoard は復元した世代を、そのマシンが想定する場所に書き込みます。

### セルフホストのサーバーだと復元は変わりますか？

いいえ。同じアプリ、同じ履歴、同じワンクリックの復元です。自分のものになるのは保存先だけです。
`,Ja=`---
title: "Como restaurar um save antigo"
description: "Save corrompido, mod estragado ou uma escolha de que te arrependes? Volta a uma versão anterior, passo a passo, sem perder o que tens agora."
order: 3
updated: 2026-09-01
---

Uma má decisão no jogo, um ficheiro corrompido ou um mod que parte tudo — às vezes só precisas de voltar atrás. Como o Hoard guarda um histórico completo de versões de cada save, restaurar um anterior leva segundos.

## Restaurar uma versão anterior

1. Abre o **Hoard** e vai ao jogo na tua **Biblioteca**.
2. Abre o separador **Histórico**. Verás cada backup com data e tamanho.
3. Escolhe a versão que queres e carrega em **Restaurar**.
4. O Hoard volta a escrever esse snapshot na pasta de save do jogo. O teu save atual é guardado primeiro, por isso a restauração é reversível.

## Restaurar num PC novo ou reinstalado

1. Instala o Hoard e inicia sessão com a tua conta.
2. Adiciona o jogo à Biblioteca — o Hoard encontra o backup na nuvem correspondente.
3. Restaura a versão mais recente, ou uma mais antiga, e continua a jogar.

Como o Hoard localiza as pastas de save com a mesma base de dados comunitária do Ludusavi, sabe onde colocar um save restaurado mesmo numa instalação limpa — sem procurares caminhos à mão.

## Quando um save fica corrompido ou uma mod o parte

Um jogo que rebenta ao carregar, uma mod que reescreveu o que não devia, um autosave que caiu a meio de uma escrita: a solução é a mesma. Abre o **Histórico** do jogo, escolhe a última versão anterior ao problema e restaura-a. As datas e os tamanhos costumam chegar para ver onde correu mal — uma queda súbita de tamanho é bom sinal de que um save ficou truncado.

Se não tens a certeza de qual é a boa, restaura a candidata mais provável e confirma dentro do jogo. Tentar de novo não custa nada, porque a versão que acabaste de substituir também ficou guardada.

## O que uma restauração faz mesmo

Três coisas que vale a pena saber, porque são as que tornam seguro experimentar:

1. **O teu save atual é capturado primeiro.** A restauração é reversível: o que substituíste passa a ser uma versão do histórico como outra qualquer.
2. **Só se descarrega o que falta.** Os ficheiros já em disco com o conteúdo certo são aproveitados tal como estão, por isso restaurar um save grande depois de uma pequena alteração move alguns megabytes e não a pasta inteira.
3. **Os ficheiros próprios desta máquina ficam intactos.** A configuração e os registos ao lado do save são copiados, mas não escritos por cima das tuas cópias locais: os teus controlos e as tuas definições gráficas sobrevivem a uma restauração vinda de outro PC.

## Restaurar sem passar pelos nossos servidores

Se corres o teu próprio \`hoard-server\`, as restaurações funcionam exatamente da mesma maneira, só que as versões vêm da tua máquina e não da nossa. Não há conta connosco, nem telemetria para nós, nem nada que passe pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

## Dica

As restaurações nunca são destrutivas: o save que substituis é primeiro capturado como nova versão, por isso podes sempre desfazer uma restauração restaurando a entrada anterior. Se até agora só guardavas backups locais (por exemplo com o Ludusavi), passar para o Hoard acrescenta um histórico versionado fora da máquina, a partir do qual podes restaurar mesmo depois de uma falha de disco.

<!-- faq -->

## Perguntas frequentes

### Restaurar sobrescreve o meu progresso atual?

Só depois de o teu save atual ter sido capturado como uma nova versão. Se restaurares a errada, restaura a entrada anterior e ficas onde estavas.

### Até onde vai o histórico?

Até onde permitir o limite de versões do teu plano, e uma versão que fixes nunca é apagada para abrir espaço. Num servidor self-hosted o único limite é o teu disco.

### Posso restaurar num PC onde o jogo ainda não está instalado?

Instala primeiro o jogo para que a pasta de saves exista, e depois restaura. O Hoard sabe onde cada jogo espera os seus saves, por isso escreve o snapshot no sítio certo sem teres de procurar o caminho.

### Funciona entre Windows e uma Steam Deck?

Sim. O mesmo jogo guarda em sítios diferentes em cada um — na Deck, dentro do prefixo Proton — e o Hoard escreve a versão restaurada onde essa máquina a espera.

### Restaurar é diferente num servidor self-hosted?

Não. Mesma aplicação, mesmo histórico, mesma restauração num clique. Só o armazenamento é teu.
`,Za=`---
title: "如何还原旧的游戏存档"
description: "存档损坏、Mod 出错或做了后悔的选择？一步步把游戏存档回滚到早先版本，同时不丢失电脑上当前的存档。"
order: 3
updated: 2026-09-01
---

游戏中的错误决定、损坏的文件，或一个搞砸的 MOD——有时你只是需要回到从前。由于 Hoard 保留每个存档的完整版本历史，还原较早的版本只需几秒。

## 还原先前版本

1. 打开 **Hoard**，在你的**库**中找到该游戏。
2. 打开它的**历史**标签。你会看到每个备份及其日期和大小。
3. 选择你想要的版本，然后选择**还原**。
4. Hoard 会把该快照写回游戏的存档文件夹。你当前的存档会先被备份，因此还原本身也可撤销。

## 在新的或重装的 PC 上还原

1. 安装 Hoard 并用你的账号登录。
2. 把游戏添加到你的库——Hoard 会找到对应的云端备份。
3. 还原最新版本，或任意较早的版本，然后继续游戏。

由于 Hoard 使用与 Ludusavi 相同的社区数据库来定位存档文件夹，即使在全新安装上，它也知道把还原的存档放到哪里——无需你手动查找路径。

## 当存档损坏，或被模组弄坏时

读档就崩溃的游戏、改写了不该改的模组、写到一半就落地的自动存档——处理方式都一样。打开该游戏的**历史**，选择出问题之前的最后一个版本，还原它。日期和大小通常就足以看出是哪一刻出了岔子：体积突然变小，往往说明存档被截断了。

如果拿不准哪个版本是好的，就先还原最可能的那个，再进游戏确认。重来一次没有代价，因为你刚刚替换掉的版本同样被保留了下来。

## 还原到底做了什么

有三点值得知道，正是它们让"先试一次"变得安全：

1. **你当前的存档会先被抓取。** 还原是可逆的：被替换掉的内容会成为历史里的一个版本，和其他版本没有区别。
2. **只下载缺少的部分。** 磁盘上内容正确的文件会被直接沿用，因此在一次小改动之后还原一个大存档，搬动的是几兆字节，而不是整个文件夹。
3. **属于这台机器的文件不会被动。** 存档旁边的配置和日志会被备份，但不会覆盖你本地的副本——从另一台 PC 还原之后，你的按键绑定和画质设置依然还在。

## 不经过我们服务器的还原

如果你运行的是自己的 \`hoard-server\`，还原的方式完全一样，只是版本来自你自己的机器而不是我们的。没有我们这边的账号，没有发往我们的遥测，也没有任何东西经过我们的服务器。参见[如何自托管 Hoard](/guides/self-host-hoard)。

## 提示

还原绝不是破坏性的：被替换的存档会先作为新版本被捕获，因此你总能通过还原上一条记录来撤销一次还原。如果你过去只保留本地备份（例如用 Ludusavi），迁移到 Hoard 会增加一份脱离本机的版本历史，即使在磁盘故障之后，你也能从中还原。

<!-- faq -->

## 常见问题

### 还原会覆盖我当前的进度吗？

只有在你当前的存档已被抓取为一个新版本之后才会。如果还原错了，把上一条记录再还原一次，就回到了原点。

### 历史能回溯多久？

取决于你所在方案的版本数上限；被你固定的版本永远不会为了腾空间而被清理。在自托管的服务器上，唯一的限制是你的磁盘。

### 可以还原到还没安装游戏的 PC 上吗？

先安装游戏，让它的存档文件夹存在，然后再还原。Hoard 知道每款游戏把存档放在哪里，会直接写到正确的位置，不必你去找路径。

### Windows 和 Steam Deck 之间能互相还原吗？

可以。同一款游戏在两边的存档位置不同——在 Deck 上位于 Proton 前缀内——Hoard 会把还原的版本写到那台机器期望的位置。

### 在自托管服务器上还原有区别吗？

没有。同样的应用、同样的历史、同样一键还原。只有存储归你所有。
`,Ya=`---
title: "Hoard mit Docker selbst hosten (Self-Hosting)"
description: "Betreibe deinen eigenen Hoard-Server mit Docker Compose: kostenlos, quelloffen, auf deiner Hardware, ohne Konto bei uns und ohne Kontingent."
order: 0
featured: true
updated: 2026-09-29
---

Hoard ist Open Source und selbst hostbar. Statt Hoard Cloud zu nutzen, kannst du denselben \`hoard-server\` auf deiner eigenen Maschine betreiben und jedes Gerät darauf verweisen – ohne Konto und ohne Speicherlimit außer der Festplatte, die du ihm gibst. Diese Anleitung bringt einen Server in wenigen Minuten mit Docker zum Laufen.

## Warum Hoard selbst hosten

- **Volle Kontrolle.** Deine Spielstände liegen auf Hardware, die du kontrollierst, nicht in fremder Cloud.
- **Kein Limit.** Der Speicher wird nur von deiner eigenen Festplatte begrenzt.
- **Gleiche App, gleiche Funktionen.** Versionierter Verlauf und Hintergrund-Sync funktionieren genau wie mit Hoard Cloud – nur das Backend ändert sich.
- **Open Source.** Du kannst den Server lesen, prüfen und anpassen.

Das ist der entscheidende Unterschied zu Tools wie [Ludusavi](/guides/ludusavi-alternative): Ludusavi ist großartig für lokale Backups und eigene Cloud per Rclone, aber den Sync richtest du selbst ein. Hoard bietet dir einen verwalteten Sync-Server, den du einmal startest und mit dem sich jedes Gerät verbindet.

## Was Selbsthosten für deine Daten bedeutet

Das gehört klar gesagt, denn genau hier liegen die meisten Vergleiche bei Hoard falsch.

**Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Spielstände liegen auf unseren Servern in der EU.

**Ein selbst gehostetes Hoard gehört vollständig dir.** Deine Geräte sprechen mit deinem Server und mit sonst nichts. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir können weder einen Spielstand noch einen Spieltitel noch eine E-Mail-Adresse sehen, schlicht weil davon nichts bei uns ankommt. Würde Hoard Cloud morgen abgeschaltet, liefe dein Setup unverändert weiter.

Eine Sache der Genauigkeit halber: dein Server hat sehr wohl eigene Zugänge — den Benutzer, den du unten anlegst, und ein Token je Gerät. Die gehören dir, auf deiner Maschine, in deiner Datenbank. Was es nicht gibt, ist ein Konto bei uns.

## Was du brauchst

- Eine Maschine, die durchläuft (Heimserver, NAS mit Docker oder ein kleiner VPS).
- Docker und Docker Compose installiert (auf einem Synology-NAS das Paket Container Manager).
- Optional eine Domain und ein Reverse-Proxy für HTTPS (empfohlen für alles außerhalb deines LAN).

## Installation mit Docker Compose

Du musst das Repository nicht klonen. Der Server ist ein fertiges Image (\`ghcr.io/rleeon/hoard\`, amd64 und arm64), und herunterladen musst du nur seine \`docker-compose.yml\`:

\`\`\`sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
\`\`\`

Beim ersten Start schreibt der Container eine funktionierende \`config.toml\` nach \`./config/\`, neben die Compose-Datei; mit einfachem \`docker compose\` gibt es also nichts vorzubereiten. Die Daten liegen in einem benannten Docker-Volume (\`hoard-data\`) – sichere es wie jedes andere Volume. Der Container lauscht intern auf Port \`12421\`; einen anderen Host-Port setzt du mit \`HOARD_PORT=9000 docker compose up -d\`.

Willst du die Konfiguration lesen, bevor irgendetwas startet, oder das Image selbst bauen, erklärt die [Self-Hosting-Anleitung im Repository](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md), wie du es klonst.

### Auf einem Synology-NAS (Container Manager)

Container Manager hat keine Kommandozeile, über die du diese beiden Variablen übergeben könntest, und startet kein Projekt, solange ein eingebundener Ordner fehlt: Das ist der Fehler \`Bind mount failed: '…/config' does not exist\`. Vier Schritte lösen beides:

1. Lege in der File Station einen Ordner für Hoard an (zum Beispiel \`docker/hoard\`) und darin einen leeren Ordner \`config\`.
2. Öffne im Container Manager **Project** → **Create**, wähle diesen Ordner als Pfad, lass eine \`docker-compose.yml\` anlegen und füge dort den Inhalt der Datei ein (öffne dazu die URL aus der \`curl\`-Zeile oben im Browser).
3. Ersetze in der eingefügten Datei \`\${HOARD_ADMIN_USERNAME:-}\` und \`\${HOARD_ADMIN_PASSWORD:-}\` durch Benutzernamen und Passwort deines Admins und lass den Rest jeder Zeile stehen, sodass die beiden Zeilen so aussehen:

   \`\`\`yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   \`\`\`

4. Schließe den Assistenten ab, um ihn zu starten, und öffne unter **Container** das Protokoll von \`hoard-server\`: Dort steht das Geräte-Token, ein einziges Mal.

Mit der Datei, wie sie ist, liegen die Spielstände im eigenen Speicher von Docker, außerhalb deiner freigegebenen Ordner. Sollen sie in einem freigegebenen Ordner liegen, den du ohnehin sicherst, lege neben \`config\` auch einen Ordner \`data\` an und ändere die Zeile \`hoard-data:/var/lib/hoard\` in \`./data:/var/lib/hoard\`.

## Benutzer und Geräte-Token anlegen

Hast du ihn mit \`HOARD_ADMIN_USERNAME\` und \`HOARD_ADMIN_PASSWORD\` gestartet, ist das schon erledigt: Der Benutzer existiert und sein Token steht im Log. Andernfalls legst du beides auf der Kommandozeile an, denn der Server hat keine Registrierungsseite:

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

Das Token wird nur einmal angezeigt und **kann später nicht wiederhergestellt werden**, also kopiere es jetzt. Für jedes weitere Gerät brauchst du kein Terminal: Öffne die Adresse deines Servers im Browser, melde dich mit diesem Benutzernamen und Passwort im Web-Panel an und nutze **Benutzer** → **Neues Token**.

## Die Desktop-App verbinden

Installiere die [Hoard-Desktop-App](/download) auf jedem Rechner. Wähle im Onboarding **Self-Host** und füge deine Server-URL und das eben erstellte Token ein. Ab da verhält es sich genau wie Hoard Cloud: Es erkennt deine Spiele, sichert Spielstände automatisch und führt einen versionierten Verlauf. Siehe [Spielstände zwischen PCs synchronisieren](/guides/sync-game-saves-across-pcs) für den Alltag.

## Halte deinen Server aktuell

Wie du aktualisierst, hängt davon ab, wie du installiert hast — und der falsche Befehl liefert keinen Fehler, sondern tut schlicht nichts. Es lohnt sich also zu wissen, welcher deiner ist.

**Docker Compose.** Neues Image holen und den Container neu erstellen. Beide Hälften, in dieser Reihenfolge:

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Hörst du nach der ersten auf, läuft der alte Container unberührt weiter: \`/v1/health\` meldet weiterhin die alte Version, und das Update sieht aus, als wäre es still gescheitert. \`git pull\` aktualisiert weder das eine noch das andere — was läuft, ist das veröffentlichte Image, nicht dein Checkout. Nagle eine Version fest (\`ghcr.io/rleeon/hoard:1.1\`) statt \`:latest\`, wenn du lieber selbst entscheidest, wann eine neue kommt.

**Unraid.** Reiter *Docker* → Hoard → *Apply update*, sobald eines angeboten wird. Nichts zu tippen.

**Bare Metal (systemd).** \`sudo hoard-server upgrade\`, danach \`sudo systemctl restart hoard-server\`. Der Befehl tauscht die Binärdatei atomar aus und startet den Dienst absichtlich nicht selbst neu, damit eine laufende Synchronisierung nicht abgeschnitten wird.

\`hoard-server upgrade\` gilt nur für die Bare-Metal-Installation. In einem Container verweigert er sich absichtlich — der Binärtausch würde das nächste \`docker compose up -d\` nicht überleben — und gibt stattdessen die beiden Befehle von oben aus; führe \`docker compose exec server hoard-server upgrade\` aus, wenn du es selbst sehen willst. Datenbankmigrationen wendet der Server beim Start an, dafür gibt es also nie einen eigenen Schritt.

## Im Produktivbetrieb

Für alles, was über dein lokales Netz hinausgeht, beende TLS an einem Reverse-Proxy (Caddy, nginx oder Traefik). Lieber Bare Metal? Das Repo liefert auch ein \`systemd\`-Installationsskript und einen Befehl \`hoard-server upgrade\`, der die Binärdatei atomar austauscht, ohne einen laufenden Sync abzubrechen.

## Selbst hosten oder Hoard Cloud?

Selbst-Hosting ist ideal, wenn du schon einen Server betreibst und volle Kontrolle ohne Limit willst. Wenn du keine Infrastruktur pflegen möchtest, bietet dir [Hoard Cloud](/pricing) denselben Sync verwaltet, mit einem kostenlosen Einstieg. So oder so bleiben App und Spielstände portabel – du kannst später wechseln.

<!-- faq -->

## Häufige Fragen

### Funkt ein selbst gehostetes Hoard nach Hause?

Nein. Die Desktop-App spricht mit der Serveradresse, die du ihr gibst. Deine Stände, deine Nutzer und deine Logs bleiben auf deiner Maschine, und nichts davon erreicht uns.

### Ist der selbst gehostete Server derselbe Code wie Hoard Cloud?

Ja, dasselbe \`hoard-server\`-Binary unter AGPL-3.0. Es gibt keine abgespeckte Community-Edition und keine Funktion, die der gehosteten Version vorbehalten wäre.

### Wo liegen die Spielstände tatsächlich?

Standardmäßig in dem Docker-Volume, das du dem Container gibst, auf deiner eigenen Platte. Wenn du bereits Objektspeicher betreibst, spricht der Server auch S3 — MinIO, Garage oder Backblaze B2 funktionieren als Ablage. So oder so sprechen deine Geräte ausschließlich mit deinem Server.

### Läuft das auf einem NAS?

Ja, auf jedem NAS mit Docker. Auf einem Synology folgst du den Schritten für Container Manager oben; für Unraid enthält das Repository eine Vorlage. In beiden Fällen wechselt das Image auf die \`PUID\`/\`PGID\`, die du angibst, damit eingebundene Ordner dem richtigen Benutzer gehören statt root.

### Brauche ich Domain und HTTPS?

Im eigenen LAN nicht. Sobald der Server von außen erreichbar ist, gehört ein Reverse Proxy davor, der TLS terminiert — Caddy, nginx oder Traefik.

### Was, wenn mein Server aus ist, wenn ich aufhöre zu spielen?

Der Snapshot entsteht lokal, es geht also nichts verloren. Er wird von selbst hochgeladen, sobald der Server wieder antwortet.

### Kann ich mit Hoard Cloud anfangen und später wechseln?

Ja, in beide Richtungen. Über die Kontoseite lässt sich alles exportieren, und die App kann ohne Neuinstallation auf einen anderen Server zeigen.
`,en=`---
title: "How to self-host Hoard with Docker"
description: "Run your own Hoard server with Docker Compose: free, open source, on your hardware, with no account with us and no quota. Your saves stay with you."
order: 0
featured: true
updated: 2026-09-29
related: sync-game-saves-across-pcs, opensave-alternative, back-up-game-saves
---

Hoard is open source and self-hostable. Instead of using Hoard Cloud, you can run the same \`hoard-server\` on your own machine and point every device at it — no account, no storage quota beyond the disk you give it. This guide gets a server running with Docker in a few minutes.

## Why self-host Hoard

- **Full ownership.** Your game saves live on hardware you control, not someone else's cloud.
- **No quota.** Storage is limited only by your own disk.
- **Same app, same features.** Versioned history and background sync work exactly as they do with Hoard Cloud — only the backend changes.
- **Open source.** You can read, audit and modify the server.

This is the key difference from tools like [Ludusavi](/guides/ludusavi-alternative): Ludusavi is great for local backups and bring-your-own-cloud via Rclone, but you wire up the sync yourself. Hoard gives you a managed sync server you run once and every device connects to.

## What self-hosting means for your data

Worth stating plainly, because it's the thing most comparisons get wrong about Hoard.

**Hoard Cloud** is the managed option: you sign in, and your saves sit on our servers, in the EU.

**A self-hosted Hoard is entirely yours.** Your devices talk to your server and to nothing else. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, for the simple reason that none of it ever reaches us. If Hoard Cloud shut down tomorrow, your setup would carry on unchanged.

To be exact about one thing: your server does have logins of its own — the user you create below, and a token per device. Those are yours, on your machine, in your database. What doesn't exist is an account with us.

## What you need

- A machine that stays on (a home server, NAS that runs Docker, or a small VPS).
- Docker and Docker Compose installed (on a Synology NAS, the Container Manager package).
- Optionally a domain name and a reverse proxy for HTTPS (recommended for anything beyond your LAN).

## Install with Docker Compose

You don't need to clone the repository. The server is a prebuilt image (\`ghcr.io/rleeon/hoard\`, amd64 and arm64), and the only file you download is its \`docker-compose.yml\`:

\`\`\`sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
\`\`\`

On its first start the container writes a working \`config.toml\` into \`./config/\`, next to the compose file, so with plain \`docker compose\` there is nothing to prepare. Data lives in a named Docker volume (\`hoard-data\`) — back it up like any other volume. The container listens on port \`12421\` internally; map a different host port with \`HOARD_PORT=9000 docker compose up -d\`.

If you'd rather read the config before anything starts, or build the image yourself, the [self-hosting guide in the repository](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md) covers cloning it.

### On a Synology NAS (Container Manager)

Container Manager has no command line to pass those two variables, and it refuses to start a project while a bind-mounted folder is missing: that is the \`Bind mount failed: '…/config' does not exist\` error. Four steps cover both:

1. In File Station, create a folder for Hoard (for example \`docker/hoard\`) and an empty \`config\` folder inside it.
2. In Container Manager, open **Project** → **Create**, set the path to that folder, choose to create a \`docker-compose.yml\` and paste in the file's contents (open the URL from the \`curl\` line above in a browser to get them).
3. In the pasted file, replace \`\${HOARD_ADMIN_USERNAME:-}\` and \`\${HOARD_ADMIN_PASSWORD:-}\` with your admin's username and password, leaving the rest of each line as it is, so they read:

   \`\`\`yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   \`\`\`

4. Finish the wizard to start it, then open the log of \`hoard-server\` under **Container**: the device token is printed there, once.

With the file as it is, the saves live in Docker's own storage, outside your shared folders. To keep them in a shared folder you already back up, also create a \`data\` folder next to \`config\` and change the line \`hoard-data:/var/lib/hoard\` to \`./data:/var/lib/hoard\`.

## Create your user and a device token

If you started it with \`HOARD_ADMIN_USERNAME\` and \`HOARD_ADMIN_PASSWORD\`, this is already done: the user exists and its token is in the log. Otherwise, create them from the command line; the server has no signup screen:

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

The token is printed once and **cannot be retrieved later**, so copy it now. For each device you add later there is no need for a terminal: open your server's address in a browser, sign in to the web panel with that username and password, and use **Users** → **New token**.

## Connect the desktop app

Install the [Hoard desktop app](/download) on each machine. In the onboarding flow, pick **Self-Host**, then paste your server URL and the token you just created. From there it behaves exactly like Hoard Cloud: it detects your games, backs up saves automatically, and keeps versioned history. See [syncing saves across PCs](/guides/sync-game-saves-across-pcs) for the day-to-day flow.

## Keep your server up to date

How you update depends on how you installed it, and the wrong command is a no-op rather than an error — so it is worth knowing which one is yours.

**Docker Compose.** Pull the new image and recreate the container. Both halves, in order:

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Stop after the first and the old container keeps running untouched: \`/v1/health\` goes on reporting the old version and the update looks as if it silently failed. \`git pull\` updates neither — what runs is the published image, not your checkout. Pin a version (\`ghcr.io/rleeon/hoard:1.1\`) instead of \`:latest\` if you would rather choose when a new one lands.

**Unraid.** *Docker* tab → Hoard → *Apply update* when one is offered. Nothing to type.

**Bare metal (systemd).** \`sudo hoard-server upgrade\`, then \`sudo systemctl restart hoard-server\`. It swaps the binary atomically and deliberately does not restart the service itself, so an in-flight sync is not killed.

\`hoard-server upgrade\` is for the bare-metal install only. Inside a container it refuses on purpose — the binary swap would not survive the next \`docker compose up -d\` — and prints the two commands above instead; run \`docker compose exec server hoard-server upgrade\` if you want to see it say so. Database migrations are applied by the server when it starts, so there is never a separate step for them.

## Run it in production

For anything exposed beyond your local network, terminate TLS at a reverse proxy (Caddy, nginx or Traefik). Prefer bare metal? The repo also ships a \`systemd\` install script and a \`hoard-server upgrade\` command that swaps the binary atomically without killing an in-flight sync.

## Self-host or Hoard Cloud?

Self-hosting is ideal if you already run a server and want full control with no quota. If you'd rather not maintain infrastructure, [Hoard Cloud](/pricing) gives you the same sync managed for you, with a free tier to start. Either way the app and your saves stay portable — you can switch later.

<!-- faq -->

## Frequently asked questions

### Does a self-hosted Hoard phone home?

No. The desktop app talks to the server address you give it. Your saves, your users and your logs stay on your machine, and nothing about them reaches us.

### Is the self-hosted server the same code as Hoard Cloud?

Yes, the same \`hoard-server\` binary, under AGPL-3.0. There is no cut-down community edition and no feature held back for the hosted version.

### Where are the saves actually stored?

By default in the Docker volume you gave the container, on your own disk. If you already run object storage, the server also speaks S3, so MinIO, Garage or Backblaze B2 work as the backing store. Either way, your devices only ever talk to your server.

### Can I run it on a NAS?

Yes, on any NAS that runs Docker. On Synology, follow the Container Manager steps above; for Unraid, the repository ships a template. Either way the image drops to the \`PUID\`/\`PGID\` you give it, so bind-mounted folders end up owned by the right user instead of root.

### Do I need a domain and HTTPS?

Not on your own LAN. The moment the server is reachable from outside it, put a reverse proxy in front of it and terminate TLS there — Caddy, nginx or Traefik all work.

### What if my server is down when I finish playing?

The snapshot is taken locally, so nothing is lost. It uploads on its own once the server answers again.

### Can I start on Hoard Cloud and move later?

Yes, in both directions. You can export everything from your account page, and the app can be pointed at a different server without reinstalling.
`,an=`---
title: "Cómo autoalojar Hoard con Docker (self-hosted)"
description: "Monta tu propio servidor de Hoard con Docker Compose: gratis, de código abierto, en tu hardware, sin cuenta con nosotros y sin cupo. Tus partidas, contigo."
order: 0
featured: true
updated: 2026-09-29
---

Hoard es de código abierto y se puede autoalojar. En lugar de usar Hoard Cloud, puedes ejecutar el mismo \`hoard-server\` en tu propia máquina y apuntar todos tus dispositivos a él: sin cuenta y sin más límite de espacio que el disco que le des. Esta guía deja un servidor funcionando con Docker en pocos minutos.

## Por qué autoalojar Hoard

- **Control total.** Tus partidas viven en hardware que tú controlas, no en la nube de otro.
- **Sin cuota.** El espacio solo lo limita tu propio disco.
- **La misma app, las mismas funciones.** El historial versionado y la sincronización en segundo plano funcionan igual que con Hoard Cloud; solo cambia el backend.
- **Código abierto.** Puedes leer, auditar y modificar el servidor.

Esta es la diferencia clave frente a herramientas como [Ludusavi](/guides/ludusavi-alternative): Ludusavi es excelente para copias locales y para usar tu propia nube vía Rclone, pero la sincronización la montas tú. Hoard te da un servidor de sincronización gestionado que arrancas una vez y al que se conectan todos los dispositivos.

## Qué significa autoalojarse para tus datos

Conviene decirlo sin rodeos, porque es lo que casi todas las comparativas se equivocan sobre Hoard.

**Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas están en nuestros servidores, en la UE.

**Un Hoard autoalojado es tuyo por completo.** Tus dispositivos hablan con tu servidor y con nada más. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, por la sencilla razón de que nada de eso nos llega. Si Hoard Cloud cerrara mañana, tu montaje seguiría funcionando igual.

Y para ser exactos en una cosa: tu servidor sí tiene sus propios accesos — el usuario que crearás más abajo y un token por dispositivo. Son tuyos, en tu máquina, en tu base de datos. Lo que no existe es una cuenta con nosotros.

## Qué necesitas

- Una máquina que esté siempre encendida (un servidor casero, un NAS que ejecute Docker o un VPS pequeño).
- Docker y Docker Compose instalados (en un NAS Synology, el paquete Container Manager).
- Opcionalmente un dominio y un proxy inverso para HTTPS (recomendado para cualquier cosa fuera de tu red local).

## Instalación con Docker Compose

No hace falta clonar el repositorio. El servidor es una imagen ya compilada (\`ghcr.io/rleeon/hoard\`, amd64 y arm64), y lo único que descargas es su \`docker-compose.yml\`:

\`\`\`sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
\`\`\`

En el primer arranque el contenedor escribe un \`config.toml\` funcional en \`./config/\`, junto al fichero de compose, así que con \`docker compose\` a secas no hay nada que preparar. Los datos se guardan en un volumen de Docker (\`hoard-data\`); haz copia de seguridad como con cualquier otro volumen. El contenedor escucha internamente en el puerto \`12421\`; usa otro puerto del host con \`HOARD_PORT=9000 docker compose up -d\`.

Si prefieres leer la configuración antes de arrancar nada, o compilar la imagen tú mismo, la [guía de self-hosting del repositorio](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md) explica cómo clonarlo.

### En un NAS Synology (Container Manager)

Container Manager no tiene línea de comandos donde pasar esas dos variables, y se niega a arrancar un proyecto mientras falte alguna de las carpetas que monta: es el error \`Bind mount failed: '…/config' does not exist\`. Cuatro pasos resuelven las dos cosas:

1. En File Station, crea una carpeta para Hoard (por ejemplo \`docker/hoard\`) y, dentro, una carpeta \`config\` vacía.
2. En Container Manager, abre **Project** → **Create**, elige esa carpeta como ruta, indica que quieres crear un \`docker-compose.yml\` y pega el contenido del fichero (para verlo, abre en el navegador la URL de la línea \`curl\` de arriba).
3. En el fichero pegado, sustituye \`\${HOARD_ADMIN_USERNAME:-}\` y \`\${HOARD_ADMIN_PASSWORD:-}\` por el usuario y la contraseña de tu administrador, sin tocar el resto de cada línea, para que queden así:

   \`\`\`yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   \`\`\`

4. Termina el asistente para arrancarlo y abre el registro de \`hoard-server\` en **Container**: ahí aparece el token de dispositivo, una sola vez.

Con el fichero tal cual, las partidas se guardan en el almacenamiento propio de Docker, fuera de tus carpetas compartidas. Para tenerlas en una carpeta compartida de la que ya haces copia, crea también una carpeta \`data\` junto a \`config\` y cambia la línea \`hoard-data:/var/lib/hoard\` por \`./data:/var/lib/hoard\`.

## Crea tu usuario y un token de dispositivo

Si lo arrancaste con \`HOARD_ADMIN_USERNAME\` y \`HOARD_ADMIN_PASSWORD\`, esto ya está hecho: el usuario existe y su token está en el log. Si no, créalos por línea de comandos (el servidor no tiene pantalla de registro):

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

El token se muestra una sola vez y **no se puede recuperar después**, así que cópialo ahora. Para cada dispositivo que añadas más adelante no hace falta terminal: abre la dirección de tu servidor en el navegador, entra en el panel web con ese usuario y esa contraseña, y usa **Usuarios** → **Nuevo token**.

## Conecta la aplicación de escritorio

Instala la [app de escritorio de Hoard](/download) en cada equipo. En el asistente inicial elige **Self-Host**, y pega la URL de tu servidor y el token que acabas de crear. A partir de ahí se comporta igual que Hoard Cloud: detecta tus juegos, copia las partidas automáticamente y mantiene el historial versionado. Consulta [sincronizar partidas entre varios PC](/guides/sync-game-saves-across-pcs) para el día a día.

## Mantén tu servidor al día

Cómo se actualiza depende de cómo lo instalaste, y equivocarse de comando no da error: simplemente no hace nada. Merece la pena saber cuál es el tuyo.

**Docker Compose.** Baja la imagen nueva y recrea el contenedor. Las dos mitades, en orden:

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Si te quedas en la primera, el contenedor viejo sigue corriendo intacto: \`/v1/health\` sigue informando de la versión antigua y la actualización parece haber fallado en silencio. \`git pull\` no actualiza ninguna de las dos: lo que corre es la imagen publicada, no tu copia del repositorio. Fija una versión (\`ghcr.io/rleeon/hoard:1.1\`) en lugar de \`:latest\` si prefieres elegir tú cuándo llega una nueva.

**Unraid.** Pestaña *Docker* → Hoard → *Apply update* cuando aparezca. No hay nada que teclear.

**Bare metal (systemd).** \`sudo hoard-server upgrade\` y después \`sudo systemctl restart hoard-server\`. Cambia el binario de forma atómica y a propósito no reinicia el servicio por su cuenta, para no cortar una sincronización en marcha.

\`hoard-server upgrade\` es sólo para la instalación bare metal. Dentro de un contenedor se niega a propósito —el cambio de binario no sobreviviría al siguiente \`docker compose up -d\`— e imprime los dos comandos de arriba; ejecuta \`docker compose exec server hoard-server upgrade\` si quieres verlo decirlo. Las migraciones de la base de datos las aplica el servidor al arrancar, así que nunca hay un paso aparte para ellas.

## Llevarlo a producción

Para cualquier cosa expuesta fuera de tu red local, termina el TLS en un proxy inverso (Caddy, nginx o Traefik). ¿Prefieres bare metal? El repositorio también incluye un script de instalación con \`systemd\` y un comando \`hoard-server upgrade\` que cambia el binario de forma atómica sin cortar una sincronización en curso.

## ¿Self-hosted o Hoard Cloud?

Autoalojar es ideal si ya tienes un servidor y quieres control total sin límites. Si prefieres no mantener infraestructura, [Hoard Cloud](/pricing) te da la misma sincronización gestionada por nosotros, con un plan gratuito para empezar. En cualquier caso, la app y tus partidas siguen siendo portables: puedes cambiar más adelante.

<!-- faq -->

## Preguntas frecuentes

### ¿Un Hoard autoalojado llama a casa?

No. La aplicación de escritorio habla con la dirección de servidor que tú le des. Tus partidas, tus usuarios y tus registros se quedan en tu máquina, y nada de eso nos llega.

### ¿El servidor autoalojado es el mismo código que Hoard Cloud?

Sí, el mismo binario \`hoard-server\`, bajo AGPL-3.0. No hay una edición comunitaria recortada ni funciones reservadas para la versión alojada.

### ¿Dónde se guardan realmente las partidas?

Por defecto, en el volumen de Docker que le des al contenedor, en tu propio disco. Si ya tienes almacenamiento de objetos, el servidor también habla S3, así que MinIO, Garage o Backblaze B2 sirven como respaldo. En cualquier caso, tus dispositivos sólo hablan con tu servidor.

### ¿Puedo montarlo en un NAS?

Sí, en cualquier NAS que ejecute Docker. En Synology, sigue los pasos de Container Manager de arriba; para Unraid, el repositorio incluye una plantilla. En los dos casos la imagen cambia al \`PUID\`/\`PGID\` que le indiques, para que las carpetas montadas sean del usuario correcto y no de root.

### ¿Necesito dominio y HTTPS?

En tu propia red local, no. En cuanto el servidor sea accesible desde fuera, pon un proxy inverso delante y termina ahí el TLS: Caddy, nginx o Traefik valen igual.

### ¿Y si mi servidor está caído cuando termino de jugar?

La instantánea se toma en local, así que no se pierde nada. Se sube sola en cuanto el servidor vuelve a responder.

### ¿Puedo empezar en Hoard Cloud y mudarme después?

Sí, y en los dos sentidos. Puedes exportarlo todo desde la página de tu cuenta, y la aplicación se puede apuntar a otro servidor sin reinstalar nada.
`,nn=`---
title: "Comment auto-héberger Hoard avec Docker (self-hosted)"
description: "Hébergez votre propre serveur Hoard avec Docker Compose : gratuit, open source, sur votre matériel, sans compte chez nous ni quota."
order: 0
featured: true
updated: 2026-09-29
---

Hoard est open source et auto-hébergeable. Au lieu d'utiliser Hoard Cloud, vous pouvez exécuter le même \`hoard-server\` sur votre propre machine et y connecter chaque appareil — sans compte, sans quota au-delà du disque que vous lui donnez. Ce guide met un serveur en route avec Docker en quelques minutes.

## Pourquoi auto-héberger Hoard

- **Maîtrise totale.** Vos sauvegardes vivent sur du matériel que vous contrôlez, pas sur le cloud d'un autre.
- **Aucun quota.** L'espace n'est limité que par votre propre disque.
- **Même app, mêmes fonctions.** L'historique versionné et la synchro en arrière-plan fonctionnent comme avec Hoard Cloud — seul le backend change.
- **Open source.** Vous pouvez lire, auditer et modifier le serveur.

C'est la différence clé avec des outils comme [Ludusavi](/guides/ludusavi-alternative) : Ludusavi est excellent pour les sauvegardes locales et le cloud « apportez le vôtre » via Rclone, mais c'est à vous de câbler la synchro. Hoard vous donne un serveur de synchro géré que vous lancez une fois et auquel chaque appareil se connecte.

## Ce que l'auto-hébergement veut dire pour vos données

Autant le dire franchement, car c'est là que presque toutes les comparaisons se trompent au sujet de Hoard.

**Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes se trouvent sur nos serveurs, dans l'UE.

**Un Hoard auto-hébergé est entièrement le vôtre.** Vos appareils parlent à votre serveur et à rien d'autre. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne pouvons voir ni une sauvegarde, ni un nom de jeu, ni une adresse e-mail, pour la simple raison que rien de tout cela ne nous parvient. Si Hoard Cloud fermait demain, votre installation continuerait à l'identique.

Une précision, pour être exact : votre serveur a bel et bien ses propres accès — l'utilisateur que vous créez plus bas, et un jeton par appareil. Ils sont à vous, sur votre machine, dans votre base. Ce qui n'existe pas, c'est un compte chez nous.

## Ce qu'il vous faut

- Une machine qui reste allumée (serveur maison, NAS exécutant Docker ou petit VPS).
- Docker et Docker Compose installés (sur un NAS Synology, le paquet Container Manager).
- Éventuellement un nom de domaine et un reverse proxy pour le HTTPS (recommandé au-delà de votre réseau local).

## Installation avec Docker Compose

Inutile de cloner le dépôt. Le serveur est une image prête à l'emploi (\`ghcr.io/rleeon/hoard\`, amd64 et arm64), et le seul fichier à télécharger est son \`docker-compose.yml\` :

\`\`\`sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
\`\`\`

Au premier démarrage, le conteneur écrit un \`config.toml\` fonctionnel dans \`./config/\`, à côté du fichier compose : avec un simple \`docker compose\`, il n'y a donc rien à préparer. Les données vivent dans un volume Docker nommé (\`hoard-data\`) — sauvegardez-le comme n'importe quel volume. Le conteneur écoute en interne sur le port \`12421\` ; choisissez un autre port hôte avec \`HOARD_PORT=9000 docker compose up -d\`.

Vous préférez lire la configuration avant tout démarrage, ou construire l'image vous-même ? Le [guide d'auto-hébergement du dépôt](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md) explique comment le cloner.

### Sur un NAS Synology (Container Manager)

Container Manager n'a pas de ligne de commande pour passer ces deux variables, et refuse de démarrer un projet tant qu'un dossier monté manque : c'est l'erreur \`Bind mount failed: '…/config' does not exist\`. Quatre étapes règlent les deux :

1. Dans File Station, créez un dossier pour Hoard (par exemple \`docker/hoard\`) et, dedans, un dossier \`config\` vide.
2. Dans Container Manager, ouvrez **Project** → **Create**, choisissez ce dossier comme chemin, optez pour la création d'un \`docker-compose.yml\` et collez-y le contenu du fichier (ouvrez l'URL de la ligne \`curl\` ci-dessus dans un navigateur pour l'obtenir).
3. Dans le fichier collé, remplacez \`\${HOARD_ADMIN_USERNAME:-}\` et \`\${HOARD_ADMIN_PASSWORD:-}\` par le nom d'utilisateur et le mot de passe de votre administrateur, sans toucher au reste de chaque ligne, pour obtenir :

   \`\`\`yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   \`\`\`

4. Terminez l'assistant pour le démarrer, puis ouvrez le journal de \`hoard-server\` dans **Container** : le jeton d'appareil y est affiché, une seule fois.

Avec le fichier tel quel, les sauvegardes vivent dans le stockage propre de Docker, hors de vos dossiers partagés. Pour les garder dans un dossier partagé que vous sauvegardez déjà, créez aussi un dossier \`data\` à côté de \`config\` et remplacez la ligne \`hoard-data:/var/lib/hoard\` par \`./data:/var/lib/hoard\`.

## Créez votre utilisateur et un jeton d'appareil

Si vous l'avez démarré avec \`HOARD_ADMIN_USERNAME\` et \`HOARD_ADMIN_PASSWORD\`, c'est déjà fait : l'utilisateur existe et son jeton est dans les logs. Sinon, créez-les en ligne de commande, car le serveur n'a pas d'écran d'inscription :

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

Le jeton n'est affiché qu'une fois et **ne peut pas être récupéré ensuite**, copiez-le maintenant. Pour chaque appareil ajouté plus tard, pas besoin de terminal : ouvrez l'adresse de votre serveur dans un navigateur, connectez-vous au panneau web avec ce nom d'utilisateur et ce mot de passe, puis utilisez **Utilisateurs** → **Nouveau jeton**.

## Connectez l'application de bureau

Installez l'[app de bureau Hoard](/download) sur chaque machine. Dans l'assistant, choisissez **Self-Host**, puis collez l'URL de votre serveur et le jeton que vous venez de créer. Ensuite, le comportement est identique à Hoard Cloud : détection des jeux, sauvegarde automatique et historique versionné. Voir [synchroniser ses sauvegardes entre PC](/guides/sync-game-saves-across-pcs) pour l'usage quotidien.

## Gardez votre serveur à jour

La façon de mettre à jour dépend de la façon dont vous l'avez installé, et se tromper de commande ne produit pas d'erreur : cela ne fait tout simplement rien. Autant savoir laquelle est la vôtre.

**Docker Compose.** Récupérez la nouvelle image et recréez le conteneur. Les deux moitiés, dans cet ordre :

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Si vous vous arrêtez à la première, l'ancien conteneur continue de tourner intact : \`/v1/health\` annonce toujours l'ancienne version et la mise à jour a l'air d'avoir échoué en silence. \`git pull\` ne met à jour ni l'un ni l'autre — ce qui tourne, c'est l'image publiée, pas votre copie du dépôt. Épinglez une version (\`ghcr.io/rleeon/hoard:1.1\`) au lieu de \`:latest\` si vous préférez choisir quand une nouvelle arrive.

**Unraid.** Onglet *Docker* → Hoard → *Apply update* quand une mise à jour est proposée. Rien à taper.

**Bare metal (systemd).** \`sudo hoard-server upgrade\`, puis \`sudo systemctl restart hoard-server\`. La commande remplace le binaire de façon atomique et ne redémarre volontairement pas le service elle-même, pour ne pas couper une synchro en cours.

\`hoard-server upgrade\` ne concerne que l'installation bare metal. Dans un conteneur, elle refuse volontairement — le remplacement du binaire ne survivrait pas au prochain \`docker compose up -d\` — et affiche les deux commandes ci-dessus ; lancez \`docker compose exec server hoard-server upgrade\` si vous voulez le constater. Les migrations de base de données sont appliquées par le serveur au démarrage : il n'y a jamais d'étape séparée pour elles.

## En production

Pour tout ce qui dépasse votre réseau local, terminez le TLS sur un reverse proxy (Caddy, nginx ou Traefik). Plutôt bare metal ? Le dépôt fournit aussi un script d'installation \`systemd\` et une commande \`hoard-server upgrade\` qui remplace le binaire de façon atomique sans interrompre une synchro en cours.

## Auto-hébergement ou Hoard Cloud ?

L'auto-hébergement est idéal si vous avez déjà un serveur et voulez un contrôle total sans quota. Si vous préférez ne pas gérer d'infrastructure, [Hoard Cloud](/pricing) vous offre la même synchro gérée pour vous, avec une offre gratuite pour démarrer. Dans les deux cas, l'app et vos sauvegardes restent portables — vous pouvez changer plus tard.

<!-- faq -->

## Questions fréquentes

### Un Hoard auto-hébergé communique-t-il avec vous ?

Non. L'application de bureau parle à l'adresse de serveur que vous lui donnez. Vos sauvegardes, vos utilisateurs et vos journaux restent sur votre machine, et rien de tout cela ne nous parvient.

### Le serveur auto-hébergé est-il le même code que Hoard Cloud ?

Oui, le même binaire \`hoard-server\`, sous AGPL-3.0. Il n'y a pas d'édition communautaire allégée ni de fonction réservée à la version hébergée.

### Où sont réellement stockées les sauvegardes ?

Par défaut dans le volume Docker que vous donnez au conteneur, sur votre propre disque. Si vous avez déjà du stockage objet, le serveur parle aussi S3 : MinIO, Garage ou Backblaze B2 font l'affaire. Dans tous les cas, vos appareils ne parlent qu'à votre serveur.

### Puis-je le faire tourner sur un NAS ?

Oui, sur n'importe quel NAS qui exécute Docker. Sur Synology, suivez les étapes Container Manager ci-dessus ; pour Unraid, le dépôt fournit un modèle. Dans les deux cas, l'image bascule sur les \`PUID\`/\`PGID\` que vous indiquez, pour que les dossiers montés appartiennent au bon utilisateur plutôt qu'à root.

### Ai-je besoin d'un domaine et de HTTPS ?

Pas sur votre réseau local. Dès que le serveur est joignable de l'extérieur, placez un reverse proxy devant et terminez-y le TLS : Caddy, nginx ou Traefik conviennent.

### Et si mon serveur est éteint quand j'arrête de jouer ?

L'instantané est pris localement, rien n'est perdu. Il s'envoie tout seul dès que le serveur répond à nouveau.

### Puis-je commencer sur Hoard Cloud et migrer plus tard ?

Oui, dans les deux sens. Vous pouvez tout exporter depuis la page de votre compte, et l'application peut pointer vers un autre serveur sans réinstallation.
`,on=`---
title: "Come self-hostare Hoard con Docker"
description: "Avvia il tuo server Hoard con Docker Compose: gratis, open source, sul tuo hardware, senza account presso di noi e senza quota. I salvataggi restano tuoi."
order: 0
featured: true
updated: 2026-09-29
---

Hoard è open source e self-hostabile. Invece di usare Hoard Cloud, puoi eseguire lo stesso \`hoard-server\` sulla tua macchina e puntarci ogni dispositivo — senza account e senza limiti di spazio oltre al disco che gli dai. Questa guida mette in piedi un server con Docker in pochi minuti.

## Perché self-hostare Hoard

- **Controllo totale.** I tuoi salvataggi vivono su hardware che controlli tu, non sul cloud altrui.
- **Nessun limite.** Lo spazio è limitato solo dal tuo disco.
- **Stessa app, stesse funzioni.** Cronologia versionata e sync in background funzionano come con Hoard Cloud — cambia solo il backend.
- **Open source.** Puoi leggere, verificare e modificare il server.

È la differenza chiave rispetto a strumenti come [Ludusavi](/guides/ludusavi-alternative): Ludusavi è ottimo per i backup locali e per il cloud «porta il tuo» tramite Rclone, ma la sincronizzazione la configuri tu. Hoard ti dà un server di sync gestito che avvii una volta e a cui si collega ogni dispositivo.

## Cosa significa il self-hosting per i tuoi dati

Vale la pena dirlo chiaramente, perché è il punto su cui quasi tutti i confronti sbagliano riguardo a Hoard.

**Hoard Cloud** è l'opzione gestita: accedi e i tuoi salvataggi stanno sui nostri server, nell'UE.

**Un Hoard self-hosted è interamente tuo.** I tuoi dispositivi parlano con il tuo server e con nient'altro. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non possiamo vedere un salvataggio, il nome di un gioco o un indirizzo email, per il semplice motivo che niente di tutto ciò ci arriva. Se Hoard Cloud chiudesse domani, la tua installazione andrebbe avanti identica.

Una precisazione, per essere esatti: il tuo server ha eccome i suoi accessi — l'utente che crei più sotto e un token per dispositivo. Sono tuoi, sulla tua macchina, nel tuo database. Quello che non esiste è un account con noi.

## Cosa ti serve

- Una macchina sempre accesa (un server domestico, un NAS che esegue Docker o un piccolo VPS).
- Docker e Docker Compose installati (su un NAS Synology, il pacchetto Container Manager).
- Facoltativamente un dominio e un reverse proxy per l'HTTPS (consigliato per tutto ciò che esce dalla rete locale).

## Installazione con Docker Compose

Non serve clonare il repository. Il server è un'immagine già pronta (\`ghcr.io/rleeon/hoard\`, amd64 e arm64), e l'unico file da scaricare è il suo \`docker-compose.yml\`:

\`\`\`sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
\`\`\`

Al primo avvio il container scrive un \`config.toml\` funzionante in \`./config/\`, accanto al file compose, quindi con un semplice \`docker compose\` non c'è niente da preparare. I dati vivono in un volume Docker (\`hoard-data\`): eseguine il backup come per qualsiasi volume. Il container ascolta internamente sulla porta \`12421\`; usa un'altra porta host con \`HOARD_PORT=9000 docker compose up -d\`.

Preferisci leggere la configurazione prima che parta qualcosa, o compilare l'immagine da te? La [guida al self-hosting nel repository](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md) spiega come clonarlo.

### Su un NAS Synology (Container Manager)

Container Manager non ha una riga di comando da cui passare quelle due variabili, e si rifiuta di avviare un progetto finché manca una cartella montata: è l'errore \`Bind mount failed: '…/config' does not exist\`. Quattro passaggi risolvono entrambe le cose:

1. In File Station, crea una cartella per Hoard (per esempio \`docker/hoard\`) e, al suo interno, una cartella \`config\` vuota.
2. In Container Manager, apri **Project** → **Create**, imposta quella cartella come percorso, scegli di creare un \`docker-compose.yml\` e incolla il contenuto del file (per ottenerlo, apri nel browser l'URL della riga \`curl\` qui sopra).
3. Nel file incollato, sostituisci \`\${HOARD_ADMIN_USERNAME:-}\` e \`\${HOARD_ADMIN_PASSWORD:-}\` con nome utente e password del tuo amministratore, lasciando il resto di ogni riga com'è, in modo che le due righe diventino:

   \`\`\`yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   \`\`\`

4. Completa la procedura guidata per avviarlo, poi apri il log di \`hoard-server\` in **Container**: il token del dispositivo è stampato lì, una sola volta.

Con il file così com'è, i salvataggi vivono nello spazio di archiviazione di Docker, fuori dalle tue cartelle condivise. Per tenerli in una cartella condivisa di cui fai già il backup, crea anche una cartella \`data\` accanto a \`config\` e cambia la riga \`hoard-data:/var/lib/hoard\` in \`./data:/var/lib/hoard\`.

## Crea il tuo utente e un token dispositivo

Se l'hai avviato con \`HOARD_ADMIN_USERNAME\` e \`HOARD_ADMIN_PASSWORD\`, è già fatto: l'utente esiste e il suo token è nel log. Altrimenti creali da riga di comando, perché il server non ha una schermata di registrazione:

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

Il token viene mostrato una sola volta e **non può essere recuperato in seguito**, quindi copialo ora. Per ogni dispositivo che aggiungi in seguito non serve un terminale: apri l'indirizzo del server nel browser, accedi al pannello web con quel nome utente e quella password e usa **Utenti** → **Nuovo token**.

## Collega l'app desktop

Installa l'[app desktop di Hoard](/download) su ogni macchina. Nella procedura iniziale scegli **Self-Host**, poi incolla l'URL del server e il token appena creato. Da lì si comporta esattamente come Hoard Cloud: rileva i giochi, salva automaticamente e mantiene la cronologia versionata. Vedi [sincronizzare i salvataggi tra più PC](/guides/sync-game-saves-across-pcs) per l'uso quotidiano.

## Tieni aggiornato il tuo server

Come si aggiorna dipende da come l'hai installato, e sbagliare comando non dà errore: semplicemente non fa nulla. Vale la pena sapere qual è il tuo caso.

**Docker Compose.** Scarica la nuova immagine e ricrea il container. Entrambe le metà, in quest'ordine:

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Se ti fermi alla prima, il vecchio container continua a girare intatto: \`/v1/health\` riporta ancora la versione precedente e l'aggiornamento sembra fallito in silenzio. \`git pull\` non aggiorna né l'uno né l'altro: quello che gira è l'immagine pubblicata, non la tua copia del repository. Fissa una versione (\`ghcr.io/rleeon/hoard:1.1\`) al posto di \`:latest\` se preferisci scegliere tu quando ne arriva una nuova.

**Unraid.** Scheda *Docker* → Hoard → *Apply update* quando compare. Niente da digitare.

**Bare metal (systemd).** \`sudo hoard-server upgrade\`, poi \`sudo systemctl restart hoard-server\`. Sostituisce il binario in modo atomico e di proposito non riavvia il servizio da solo, per non troncare una sincronizzazione in corso.

\`hoard-server upgrade\` vale solo per l'installazione bare metal. Dentro un container si rifiuta di proposito — la sostituzione del binario non sopravvivrebbe al prossimo \`docker compose up -d\` — e stampa i due comandi qui sopra; esegui \`docker compose exec server hoard-server upgrade\` se vuoi sentirglielo dire. Le migrazioni del database le applica il server all'avvio, quindi non c'è mai un passaggio separato.

## In produzione

Per tutto ciò che è esposto oltre la rete locale, termina il TLS su un reverse proxy (Caddy, nginx o Traefik). Preferisci il bare metal? Il repository include anche uno script di installazione \`systemd\` e un comando \`hoard-server upgrade\` che sostituisce il binario in modo atomico senza interrompere una sync in corso.

## Self-host o Hoard Cloud?

Il self-hosting è ideale se hai già un server e vuoi controllo totale senza limiti. Se preferisci non gestire infrastruttura, [Hoard Cloud](/pricing) ti dà la stessa sincronizzazione gestita da noi, con un piano gratuito per iniziare. In ogni caso app e salvataggi restano portabili: puoi cambiare in seguito.

<!-- faq -->

## Domande frequenti

### Un Hoard self-hosted comunica con voi?

No. L'app desktop parla con l'indirizzo del server che le indichi tu. I tuoi salvataggi, i tuoi utenti e i tuoi log restano sulla tua macchina, e niente di tutto ciò ci arriva.

### Il server self-hosted è lo stesso codice di Hoard Cloud?

Sì, lo stesso binario \`hoard-server\`, sotto AGPL-3.0. Non c'è una community edition ridotta né funzioni tenute da parte per la versione ospitata.

### Dove finiscono davvero i salvataggi?

Per impostazione predefinita nel volume Docker che assegni al container, sul tuo disco. Se hai già uno storage a oggetti, il server parla anche S3: MinIO, Garage o Backblaze B2 vanno bene come archivio. In ogni caso i tuoi dispositivi parlano soltanto con il tuo server.

### Posso farlo girare su un NAS?

Sì, su qualsiasi NAS che esegua Docker. Su Synology segui i passaggi per Container Manager qui sopra; per Unraid il repository include un template. In entrambi i casi l'immagine scende ai \`PUID\`/\`PGID\` che indichi, così le cartelle montate risultano dell'utente giusto e non di root.

### Servono un dominio e HTTPS?

Sulla tua rete locale no. Non appena il server è raggiungibile dall'esterno, mettici davanti un reverse proxy e termina lì il TLS: vanno bene Caddy, nginx o Traefik.

### E se il server è spento quando smetto di giocare?

Lo snapshot viene preso in locale, quindi non si perde nulla. Sale da solo appena il server torna a rispondere.

### Posso iniziare con Hoard Cloud e spostarmi dopo?

Sì, in entrambe le direzioni. Puoi esportare tutto dalla pagina del tuo account, e l'app può essere puntata su un altro server senza reinstallare niente.
`,sn=`---
title: "DockerでHoardをセルフホストする方法"
description: "Docker ComposeでHoardサーバーを自前で運用。無料・オープンソースで自分のハードウェア上に。当社のアカウントも容量制限も不要です。"
order: 0
featured: true
updated: 2026-09-29
---

Hoard はオープンソースでセルフホスト可能です。Hoard Cloud を使う代わりに、同じ \`hoard-server\` を自分のマシンで動かし、すべての端末をそこへ接続できます。アカウントは不要で、容量制限は与えたディスク容量だけです。このガイドでは Docker を使って数分でサーバーを立ち上げます。

## なぜ Hoard をセルフホストするのか

- **完全な所有権。** セーブデータは他人のクラウドではなく、自分が管理するハードウェアに保存されます。
- **容量制限なし。** 容量は自分のディスクだけが上限です。
- **同じアプリ、同じ機能。** バージョン履歴とバックグラウンド同期は Hoard Cloud とまったく同じように動作し、変わるのはバックエンドだけです。
- **オープンソース。** サーバーを読み、監査し、改変できます。

これが [Ludusavi](/guides/ludusavi-alternative) のようなツールとの決定的な違いです。Ludusavi はローカルバックアップや Rclone 経由の「自分のクラウドを持ち込む」方式に優れていますが、同期は自分で組む必要があります。Hoard は一度立ち上げればすべての端末が接続できる、管理された同期サーバーを提供します。

## セルフホストがデータにとって何を意味するか

多くの比較が Hoard について誤解している点なので、はっきり書きます。

**Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに置かれます。

**セルフホストした Hoard は完全にあなたのものです。** あなたの端末は自分のサーバーとだけ通信し、他のどこにも接続しません。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。セーブもゲーム名もメールアドレスも見えません。そもそも届かないからです。仮に明日 Hoard Cloud が終了しても、あなたの構成はそのまま動き続けます。

正確を期して 1 点だけ。あなたのサーバーには確かにログインがあります。下で作成するユーザーと、端末ごとのトークンです。それらはあなたのもので、あなたのマシンの、あなたのデータベースの中にあります。存在しないのは「当方のアカウント」です。

## 必要なもの

- 常時稼働するマシン（自宅サーバー、Docker が動く NAS、または小さな VPS）。
- Docker と Docker Compose がインストール済みであること（Synology NAS なら Container Manager パッケージ）。
- 任意で、HTTPS 用のドメインとリバースプロキシ（LAN を越える用途では推奨）。

## Docker Compose でインストール

リポジトリをクローンする必要はありません。サーバーはビルド済みのイメージ（\`ghcr.io/rleeon/hoard\`、amd64 と arm64）なので、ダウンロードするのは \`docker-compose.yml\` だけです。

\`\`\`sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
\`\`\`

初回起動時に、コンテナは動作する \`config.toml\` を compose ファイルの隣の \`./config/\` に書き出します。素の \`docker compose\` なら事前の準備は何もいりません。データは名前付き Docker ボリューム（\`hoard-data\`）に保存されるので、他のボリュームと同様にバックアップしてください。コンテナは内部でポート \`12421\` を待ち受けます。別のホストポートを使うには \`HOARD_PORT=9000 docker compose up -d\` とします。

起動前に設定を読んでおきたい場合や、イメージを自分でビルドしたい場合は、[リポジトリのセルフホストガイド](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md)にクローンする手順があります。

### Synology NAS の場合（Container Manager）

Container Manager には、この 2 つの変数を渡すコマンドラインがありません。また、バインドマウントするフォルダーが存在しないとプロジェクトを起動しません。これが \`Bind mount failed: '…/config' does not exist\` エラーの原因です。次の 4 ステップで両方とも解決します。

1. File Station で Hoard 用のフォルダー（例：\`docker/hoard\`）を作り、その中に空の \`config\` フォルダーを作ります。
2. Container Manager で **Project** → **Create** を開き、パスにそのフォルダーを指定して \`docker-compose.yml\` の作成を選び、ファイルの内容を貼り付けます（内容は、上の \`curl\` 行の URL をブラウザーで開くと確認できます）。
3. 貼り付けたファイルで \`\${HOARD_ADMIN_USERNAME:-}\` と \`\${HOARD_ADMIN_PASSWORD:-}\` を管理者のユーザー名とパスワードに置き換えます。各行のそれ以外の部分はそのままにして、次のようにします。

   \`\`\`yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   \`\`\`

4. ウィザードを完了して起動したら、**Container** で \`hoard-server\` のログを開きます。端末トークンはそこに一度だけ表示されます。

ファイルをそのまま使うと、セーブは共有フォルダーの外にある Docker 自身のストレージに保存されます。すでにバックアップしている共有フォルダーに置きたい場合は、\`config\` の隣に \`data\` フォルダーも作り、\`hoard-data:/var/lib/hoard\` の行を \`./data:/var/lib/hoard\` に変更してください。

## ユーザーと端末トークンを作成

\`HOARD_ADMIN_USERNAME\` と \`HOARD_ADMIN_PASSWORD\` を付けて起動した場合、これはもう済んでいます。ユーザーは作成済みで、トークンはログにあります。そうでない場合は、コマンドラインで作成します（サーバーにサインアップ画面はありません）。

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

トークンは一度だけ表示され、**後から取得することはできません**。今すぐコピーしてください。あとから端末を追加するときはターミナル不要です。ブラウザーでサーバーのアドレスを開き、そのユーザー名とパスワードで Web パネルにログインして、**ユーザー** → **トークンを発行** を使います。

## デスクトップアプリを接続

各マシンに [Hoard デスクトップアプリ](/download) をインストールします。オンボーディングで **セルフホスト** を選び、サーバーの URL と作成したトークンを貼り付けます。あとは Hoard Cloud とまったく同じで、ゲームを検出し、自動でバックアップし、バージョン履歴を保持します。日常的な使い方は [複数の PC 間でセーブを同期する](/guides/sync-game-saves-across-pcs) を参照してください。

## サーバーを最新に保つ

更新の方法はインストールの仕方によって変わります。しかも間違ったコマンドはエラーにならず、ただ何も起きないだけなので、自分がどれに当てはまるかを知っておく価値があります。

**Docker Compose.** 新しいイメージを取得し、コンテナを作り直します。次の順番で、両方とも実行してください。

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

最初のコマンドで止めると、古いコンテナがそのまま動き続けます。\`/v1/health\` は古いバージョンを返し続け、更新が黙って失敗したように見えます。\`git pull\` はどちらも更新しません。動いているのは公開イメージであって、あなたのチェックアウトではないからです。新しいイメージが来るタイミングを自分で決めたい場合は、\`:latest\` の代わりにバージョンを固定してください（\`ghcr.io/rleeon/hoard:1.1\`）。

**Unraid.** *Docker* タブ → Hoard → 更新が出たら *Apply update*。入力するものはありません。

**ベアメタル（systemd）.** \`sudo hoard-server upgrade\` を実行し、続けて \`sudo systemctl restart hoard-server\`。バイナリをアトミックに入れ替えますが、進行中の同期を切らないよう、サービスの再起動は意図的に行いません。

\`hoard-server upgrade\` はベアメタルのインストール専用です。コンテナ内では意図的に実行を拒否し（入れ替えたバイナリは次の \`docker compose up -d\` で消えてしまうため）、代わりに上の 2 つのコマンドを表示します。実際に確かめたい場合は \`docker compose exec server hoard-server upgrade\` を実行してください。データベースのマイグレーションは起動時にサーバーが適用するので、そのための別の手順はありません。

## 本番運用

ローカルネットワークを越えて公開する場合は、リバースプロキシ（Caddy、nginx、Traefik）で TLS を終端します。ベアメタルがよい場合は、リポジトリに \`systemd\` インストールスクリプトと、進行中の同期を止めずにバイナリをアトミックに入れ替える \`hoard-server upgrade\` コマンドも含まれています。

## セルフホストと Hoard Cloud のどちら？

すでにサーバーを運用していて容量制限なしの完全な管理を望むなら、セルフホストが最適です。インフラの保守をしたくない場合は、[Hoard Cloud](/pricing) が同じ同期をこちらで管理して提供し、無料プランから始められます。どちらでもアプリとセーブデータは可搬性を保つので、後から切り替えられます。

<!-- faq -->

## よくある質問

### セルフホストした Hoard は外部に通信しますか？

いいえ。デスクトップアプリは、あなたが指定したサーバーのアドレスとだけ通信します。セーブもユーザーもログもあなたのマシンにとどまり、そのいずれも当方には届きません。

### セルフホストのサーバーは Hoard Cloud と同じコードですか？

はい。AGPL-3.0 の同じ \`hoard-server\` バイナリです。機能を削ったコミュニティ版もなければ、ホスト版だけの機能もありません。

### セーブは実際どこに保存されますか？

既定では、コンテナに与えた Docker ボリューム、つまりあなた自身のディスクです。すでにオブジェクトストレージを運用しているなら、サーバーは S3 も話せるので、MinIO、Garage、Backblaze B2 を保存先にできます。いずれの場合も、端末が通信する相手はあなたのサーバーだけです。

### NAS で動かせますか？

はい、Docker が動く NAS なら動きます。Synology では上の Container Manager の手順に従ってください。Unraid 用にはリポジトリにテンプレートが同梱されています。いずれの場合も、イメージは指定した \`PUID\`/\`PGID\` に降格するので、バインドマウントしたフォルダーの所有者が root ではなく適切なユーザーになります。

### ドメインと HTTPS は必要ですか？

自宅の LAN 内だけなら不要です。サーバーが外部から到達可能になった時点で、前段にリバースプロキシを置いて TLS を終端してください。Caddy、nginx、Traefik のいずれでも構いません。

### プレイ終了時にサーバーが落ちていたら？

スナップショットはローカルで作られるので、失われるものはありません。サーバーが応答を再開すると自動でアップロードされます。

### Hoard Cloud で始めて、後から移れますか？

はい、双方向に移れます。アカウントページからすべてをエクスポートでき、アプリは再インストールなしで別のサーバーを指すように変更できます。
`,rn=`---
title: "Como auto-hospedar o Hoard com Docker (self-hosted)"
description: "Corre o teu próprio servidor Hoard com Docker Compose: grátis, código aberto, no teu hardware, sem conta connosco e sem quota. Os saves ficam contigo."
order: 0
featured: true
updated: 2026-09-29
---

O Hoard é de código aberto e pode ser auto-hospedado. Em vez de usar o Hoard Cloud, você pode rodar o mesmo \`hoard-server\` na sua própria máquina e apontar todos os dispositivos para ele — sem conta e sem limite de espaço além do disco que você der a ele. Este guia coloca um servidor no ar com Docker em poucos minutos.

## Por que auto-hospedar o Hoard

- **Controle total.** Seus saves ficam em hardware que você controla, não na nuvem de outra pessoa.
- **Sem cota.** O espaço é limitado apenas pelo seu próprio disco.
- **Mesmo app, mesmos recursos.** Histórico versionado e sincronização em segundo plano funcionam igual ao Hoard Cloud — só muda o backend.
- **Código aberto.** Você pode ler, auditar e modificar o servidor.

Essa é a diferença principal em relação a ferramentas como o [Ludusavi](/guides/ludusavi-alternative): o Ludusavi é ótimo para backups locais e para usar sua própria nuvem via Rclone, mas a sincronização você mesmo monta. O Hoard oferece um servidor de sincronização gerenciado que você sobe uma vez e ao qual cada dispositivo se conecta.

## O que o self-hosting significa para os teus dados

Vale a pena dizê-lo sem rodeios, porque é o ponto em que quase todas as comparações se enganam sobre o Hoard.

**O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.

**Um Hoard self-hosted é inteiramente teu.** Os teus dispositivos falam com o teu servidor e com mais nada. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não conseguimos ver um save, o nome de um jogo ou um endereço de email, pela simples razão de que nada disso nos chega. Se o Hoard Cloud fechasse amanhã, a tua instalação continuaria igual.

E, para ser exato numa coisa: o teu servidor tem sim os seus próprios acessos — o utilizador que crias mais abaixo e um token por dispositivo. São teus, na tua máquina, na tua base de dados. O que não existe é uma conta connosco.

## O que você precisa

- Uma máquina que fique ligada (um servidor doméstico, um NAS que rode Docker ou um VPS pequeno).
- Docker e Docker Compose instalados (em um NAS Synology, o pacote Container Manager).
- Opcionalmente um domínio e um proxy reverso para HTTPS (recomendado para qualquer coisa fora da sua rede local).

## Instalação com Docker Compose

Não é preciso clonar o repositório. O servidor é uma imagem pronta (\`ghcr.io/rleeon/hoard\`, amd64 e arm64), e o único arquivo que você baixa é o \`docker-compose.yml\` dela:

\`\`\`sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
\`\`\`

Na primeira inicialização, o contêiner grava um \`config.toml\` funcional em \`./config/\`, ao lado do arquivo compose; com o \`docker compose\` puro, não há nada a preparar. Os dados ficam em um volume nomeado do Docker (\`hoard-data\`) — faça backup como em qualquer outro volume. O contêiner escuta internamente na porta \`12421\`; use outra porta do host com \`HOARD_PORT=9000 docker compose up -d\`.

Prefere ler a configuração antes de iniciar qualquer coisa, ou compilar a imagem você mesmo? O [guia de self-hosting do repositório](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md) explica como cloná-lo.

### Em um NAS Synology (Container Manager)

O Container Manager não tem linha de comando para passar essas duas variáveis, e se recusa a iniciar um projeto enquanto faltar uma pasta montada: é o erro \`Bind mount failed: '…/config' does not exist\`. Quatro passos resolvem as duas coisas:

1. No File Station, crie uma pasta para o Hoard (por exemplo \`docker/hoard\`) e, dentro dela, uma pasta \`config\` vazia.
2. No Container Manager, abra **Project** → **Create**, defina essa pasta como caminho, escolha criar um \`docker-compose.yml\` e cole o conteúdo do arquivo (para obtê-lo, abra no navegador a URL da linha \`curl\` acima).
3. No arquivo colado, substitua \`\${HOARD_ADMIN_USERNAME:-}\` e \`\${HOARD_ADMIN_PASSWORD:-}\` pelo usuário e pela senha do seu administrador, sem mexer no resto de cada linha, para que fiquem assim:

   \`\`\`yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   \`\`\`

4. Conclua o assistente para iniciá-lo e abra o log de \`hoard-server\` em **Container**: o token do dispositivo aparece ali, uma única vez.

Com o arquivo como está, os saves ficam no armazenamento do próprio Docker, fora das suas pastas compartilhadas. Para mantê-los em uma pasta compartilhada da qual você já faz backup, crie também uma pasta \`data\` ao lado de \`config\` e troque a linha \`hoard-data:/var/lib/hoard\` por \`./data:/var/lib/hoard\`.

## Crie seu usuário e um token de dispositivo

Se você o iniciou com \`HOARD_ADMIN_USERNAME\` e \`HOARD_ADMIN_PASSWORD\`, isso já está feito: o usuário existe e o token dele está no log. Caso contrário, crie-os pela linha de comando, já que o servidor não tem tela de cadastro:

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

O token é exibido uma única vez e **não pode ser recuperado depois**, então copie-o agora. Para cada dispositivo que você adicionar depois, não é preciso terminal: abra o endereço do servidor no navegador, entre no painel web com esse usuário e essa senha e use **Utilizadores** → **Novo token**.

## Conecte o app de desktop

Instale o [app de desktop do Hoard](/download) em cada máquina. No fluxo inicial, escolha **Self-Host** e cole a URL do seu servidor e o token recém-criado. A partir daí ele se comporta exatamente como o Hoard Cloud: detecta seus jogos, faz backup dos saves automaticamente e mantém o histórico versionado. Veja [sincronizar saves entre vários PCs](/guides/sync-game-saves-across-pcs) para o uso no dia a dia.

## Mantenha seu servidor atualizado

Como atualizar depende de como você instalou, e errar o comando não dá erro: simplesmente não faz nada. Vale saber qual é o seu caso.

**Docker Compose.** Baixe a imagem nova e recrie o contêiner. As duas metades, nesta ordem:

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Se parar na primeira, o contêiner antigo continua rodando intacto: \`/v1/health\` segue informando a versão antiga e a atualização parece ter falhado em silêncio. \`git pull\` não atualiza nenhum dos dois — o que roda é a imagem publicada, não o seu clone do repositório. Fixe uma versão (\`ghcr.io/rleeon/hoard:1.1\`) no lugar de \`:latest\` se preferir escolher quando uma nova chega.

**Unraid.** Aba *Docker* → Hoard → *Apply update* quando aparecer. Nada para digitar.

**Bare metal (systemd).** \`sudo hoard-server upgrade\` e depois \`sudo systemctl restart hoard-server\`. Ele troca o binário de forma atômica e de propósito não reinicia o serviço sozinho, para não cortar uma sincronização em andamento.

\`hoard-server upgrade\` é só para a instalação bare metal. Dentro de um contêiner ele se recusa de propósito — a troca de binário não sobreviveria ao próximo \`docker compose up -d\` — e imprime os dois comandos acima; rode \`docker compose exec server hoard-server upgrade\` se quiser vê-lo dizer isso. As migrações do banco de dados são aplicadas pelo servidor ao iniciar, então nunca há um passo separado para elas.

## Em produção

Para qualquer coisa exposta além da rede local, termine o TLS em um proxy reverso (Caddy, nginx ou Traefik). Prefere bare metal? O repositório também traz um script de instalação \`systemd\` e um comando \`hoard-server upgrade\` que troca o binário de forma atômica sem matar uma sincronização em andamento.

## Self-hosted ou Hoard Cloud?

Auto-hospedar é ideal se você já tem um servidor e quer controle total sem cota. Se preferir não manter infraestrutura, o [Hoard Cloud](/pricing) oferece a mesma sincronização gerenciada por nós, com um plano gratuito para começar. De qualquer forma, o app e seus saves continuam portáteis — você pode trocar depois.

<!-- faq -->

## Perguntas frequentes

### Um Hoard self-hosted comunica convosco?

Não. A aplicação de ambiente de trabalho fala com o endereço de servidor que lhe deres. Os teus saves, os teus utilizadores e os teus registos ficam na tua máquina, e nada disso nos chega.

### O servidor self-hosted é o mesmo código do Hoard Cloud?

Sim, o mesmo binário \`hoard-server\`, sob AGPL-3.0. Não há uma edição comunitária reduzida nem funcionalidades guardadas para a versão alojada.

### Onde ficam realmente guardados os saves?

Por omissão, no volume Docker que deres ao contentor, no teu próprio disco. Se já tens armazenamento de objetos, o servidor também fala S3, por isso MinIO, Garage ou Backblaze B2 servem de repositório. Em qualquer dos casos, os teus dispositivos só falam com o teu servidor.

### Posso pô-lo a correr num NAS?

Sim, em qualquer NAS que corra Docker. No Synology, segue os passos do Container Manager acima; para o Unraid, o repositório inclui um template. Em ambos os casos a imagem desce para os \`PUID\`/\`PGID\` que indicares, para que as pastas montadas fiquem do utilizador certo em vez de root.

### Preciso de domínio e HTTPS?

Na tua própria rede local, não. A partir do momento em que o servidor é acessível de fora, põe um proxy inverso à frente e termina aí o TLS: Caddy, nginx ou Traefik servem.

### E se o meu servidor estiver em baixo quando acabo de jogar?

O snapshot é tirado localmente, por isso não se perde nada. Sobe sozinho assim que o servidor voltar a responder.

### Posso começar no Hoard Cloud e mudar mais tarde?

Sim, nos dois sentidos. Podes exportar tudo a partir da página da tua conta, e a aplicação pode ser apontada a outro servidor sem reinstalar nada.
`,tn=`---
title: "如何用 Docker 自托管 Hoard"
description: "用 Docker Compose 运行你自己的 Hoard 服务端：免费开源，跑在你的硬件上，无需我们的账号，也没有配额限制。"
order: 0
featured: true
updated: 2026-09-29
---

Hoard 是开源且可自托管的。你可以不使用 Hoard Cloud，而是在自己的机器上运行同一个 \`hoard-server\`，让每台设备都连接到它——无需账号，容量只受你分配的磁盘大小限制。本指南用 Docker 在几分钟内把服务器跑起来。

## 为什么自托管 Hoard

- **完全掌控。** 你的存档保存在你自己掌控的硬件上，而不是别人的云端。
- **没有容量限制。** 空间仅受你自己的磁盘限制。
- **同一个应用，同样的功能。** 版本历史和后台同步与 Hoard Cloud 完全一致，改变的只有后端。
- **开源。** 你可以阅读、审计并修改服务器代码。

这正是它与 [Ludusavi](/guides/ludusavi-alternative) 这类工具的关键区别：Ludusavi 在本地备份和通过 Rclone「自带云」方面很出色，但同步需要你自己搭建。Hoard 则提供一个托管式的同步服务器，启动一次后每台设备都能连接。

## 自托管对你的数据意味着什么

这一点值得直说，因为多数对比在 Hoard 上正是弄错了这里。

**Hoard Cloud** 是托管方案：你登录，存档存放在我们位于欧盟的服务器上。

**自托管的 Hoard 完全属于你。** 你的设备只与你自己的服务器通信，不与任何其他地方通信。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。我们看不到任何存档、游戏名或邮箱地址，原因很简单：这些从未到达我们这里。就算 Hoard Cloud 明天关停，你的部署照常运行。

有一点需要说准确：你的服务器确实有它自己的登录——下面你要创建的用户，以及每台设备一个令牌。它们是你的，在你的机器上、你的数据库里。不存在的是"我们这边的账号"。

## 你需要准备

- 一台保持开机的机器（家庭服务器、运行 Docker 的 NAS，或一台小型 VPS）。
- 已安装 Docker 和 Docker Compose（群晖 NAS 上即 Container Manager 套件）。
- 可选：一个域名和用于 HTTPS 的反向代理（超出本地局域网的场景推荐）。

## 用 Docker Compose 安装

不需要克隆仓库。服务器是预先构建好的镜像（\`ghcr.io/rleeon/hoard\`，支持 amd64 和 arm64），你只需下载它的 \`docker-compose.yml\`：

\`\`\`sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
\`\`\`

首次启动时，容器会在 compose 文件旁边的 \`./config/\` 里写入一份可用的 \`config.toml\`，所以直接用 \`docker compose\` 时无需任何准备。数据保存在一个命名的 Docker 卷（\`hoard-data\`）中——像备份其他卷一样备份它。容器内部监听 \`12421\` 端口；用 \`HOARD_PORT=9000 docker compose up -d\` 可映射到其他主机端口。

如果你想在启动前先阅读配置，或自己构建镜像，[仓库里的自托管指南](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md)介绍了克隆仓库的做法。

### 在群晖（Synology）NAS 上（Container Manager）

Container Manager 没有命令行可以传入这两个变量；而且只要有一个绑定挂载的文件夹不存在，它就拒绝启动项目——这就是 \`Bind mount failed: '…/config' does not exist\` 错误。以下四步可以同时解决这两个问题：

1. 在 File Station 中为 Hoard 新建一个文件夹（例如 \`docker/hoard\`），并在其中新建一个空的 \`config\` 文件夹。
2. 在 Container Manager 中打开 **Project** → **Create**，把路径设为该文件夹，选择创建 \`docker-compose.yml\`，然后粘贴文件内容（在浏览器中打开上面 \`curl\` 那一行里的 URL 即可获取）。
3. 在粘贴的文件中，把 \`\${HOARD_ADMIN_USERNAME:-}\` 和 \`\${HOARD_ADMIN_PASSWORD:-}\` 换成你的管理员用户名和密码，每行其余部分保持不变，使这两行变成：

   \`\`\`yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   \`\`\`

4. 完成向导以启动它，然后在 **Container** 中打开 \`hoard-server\` 的日志：设备令牌会在那里显示，仅此一次。

按文件原样使用时，存档保存在 Docker 自己的存储中，不在你的共享文件夹里。若想把它们放进你已经在备份的共享文件夹，请在 \`config\` 旁再新建一个 \`data\` 文件夹，并把 \`hoard-data:/var/lib/hoard\` 这一行改为 \`./data:/var/lib/hoard\`。

## 创建用户和设备令牌

如果你启动时设置了 \`HOARD_ADMIN_USERNAME\` 和 \`HOARD_ADMIN_PASSWORD\`，这一步已经完成：用户已存在，令牌就在日志里。否则，请通过命令行创建（服务器没有注册页面）：

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

令牌只显示一次，**之后无法找回**，请立即复制。之后再添加设备时无需终端：在浏览器中打开服务器地址，用该用户名和密码登录网页面板，然后使用 **用户** → **新建令牌**。

## 连接桌面应用

在每台机器上安装 [Hoard 桌面应用](/download)。在初始引导中选择 **自托管**，然后粘贴你的服务器 URL 和刚创建的令牌。之后它的行为与 Hoard Cloud 完全相同：检测你的游戏、自动备份存档、保留版本历史。日常用法请参见[在多台 PC 之间同步存档](/guides/sync-game-saves-across-pcs)。

## 保持服务器更新

怎么更新取决于你是怎么安装的，而且用错命令不会报错，只是什么都不做 —— 所以值得先弄清楚哪一种是你的情况。

**Docker Compose.** 拉取新镜像并重建容器。两条都要执行，按顺序：

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

只执行第一条的话，旧容器会原封不动地继续运行：\`/v1/health\` 仍然报告旧版本，看起来就像更新悄悄失败了。\`git pull\` 两者都更新不了 —— 运行的是已发布的镜像，不是你的代码副本。如果你想自己决定什么时候用上新版本，把 \`:latest\` 换成固定版本（\`ghcr.io/rleeon/hoard:1.1\`）。

**Unraid.** *Docker* 标签页 → Hoard → 出现更新时点 *Apply update*。不需要输入任何命令。

**裸机（systemd）.** 先 \`sudo hoard-server upgrade\`，再 \`sudo systemctl restart hoard-server\`。它会原子地替换二进制文件，并且故意不自己重启服务，以免中断正在进行的同步。

\`hoard-server upgrade\` 只适用于裸机安装。在容器里它会故意拒绝执行 —— 替换后的二进制文件撑不过下一次 \`docker compose up -d\` —— 并改为打印上面那两条命令；想亲眼看看的话，执行 \`docker compose exec server hoard-server upgrade\`。数据库迁移由服务器在启动时应用，所以永远不需要单独的步骤。

## 在生产环境中运行

对于任何暴露到本地网络之外的部署，请在反向代理（Caddy、nginx 或 Traefik）上终止 TLS。更喜欢裸机部署？仓库还提供了 \`systemd\` 安装脚本，以及一个 \`hoard-server upgrade\` 命令，它会原子地替换二进制文件而不会中断正在进行的同步。

## 自托管还是 Hoard Cloud？

如果你已经在运行服务器并希望完全掌控、没有容量限制，自托管是理想选择。如果你不想维护基础设施，[Hoard Cloud](/pricing) 提供由我们托管的同样同步功能，并有免费档可供起步。无论哪种方式，应用和你的存档都保持可迁移——以后可以随时切换。

<!-- faq -->

## 常见问题

### 自托管的 Hoard 会回连你们吗？

不会。桌面应用只与你给它的服务器地址通信。你的存档、你的用户和你的日志都留在你的机器上，其中没有任何内容会到达我们这里。

### 自托管服务器和 Hoard Cloud 是同一份代码吗？

是的，同一个 \`hoard-server\` 二进制，采用 AGPL-3.0。没有功能删减的社区版，也没有只留给托管版的功能。

### 存档实际保存在哪里？

默认在你分配给容器的 Docker 卷里，也就是你自己的磁盘上。如果你已经在跑对象存储，服务器同样支持 S3，MinIO、Garage 或 Backblaze B2 都可以作为后端。无论哪种方式，你的设备始终只与你的服务器通信。

### 可以跑在 NAS 上吗？

可以，任何能运行 Docker 的 NAS 都行。群晖请按照上文 Container Manager 的步骤操作；Unraid 可使用仓库里附带的模板。无论哪种方式，镜像都会降权到你指定的 \`PUID\`/\`PGID\`，这样绑定挂载的文件夹归属正确的用户，而不是 root。

### 需要域名和 HTTPS 吗？

在自家局域网里不需要。一旦服务器可以从外部访问，就在前面放一个反向代理并在那里终止 TLS——Caddy、nginx 或 Traefik 都可以。

### 如果我玩完时服务器正好没开呢？

快照是在本地生成的，不会丢失任何东西。等服务器重新响应，它会自行上传。

### 可以先用 Hoard Cloud，以后再迁移吗？

可以，双向都行。你能在账号页面导出全部数据，应用也可以指向另一台服务器，无需重装。
`,un=`---
title: "Marvel's Spider-Man 2: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Marvel's Spider-Man 2 seine PC-Spielstände ablegt, was der Ordner mit der langen Zahl ist, die OneDrive-Falle, der Pfad auf dem Steam Deck und Backups."
order: 22
updated: 2026-10-02
---

Auf dem PC legt Marvel's Spider-Man 2 seine Spielstände in \`Dokumente\\Marvel's Spider-Man 2\\\` ab, in einem Unterordner mit einer langen Zahl. Bei Steam ist diese Zahl deine Steam-ID. Darunter erfährst du, was das praktisch bedeutet, die OneDrive-Falle, den Pfad auf dem Steam Deck und wie du die Spielstände gesichert hältst.

## Wo Marvel's Spider-Man 2 seine Spielstände ablegt

- **Windows:** \`%USERPROFILE%\\Documents\\Marvel's Spider-Man 2\\<lange Zahl>\`
- **Steam Deck und Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<lange Zahl>\`

Die PC-Version gibt es nur für Windows, also läuft sie auf dem Steam Deck über Proton, und die Spielstände liegen im Proton-Präfix, das Steam für das Spiel anlegt; \`2651280\` ist seine Steam-App-ID. Ist es auf der microSD-Karte installiert, liegt der \`compatdata\`-Ordner auf der Karte.

Nixxes, das Studio hinter der PC-Fassung, beschreibt den Spielstand-Ordner als „einen Unterordner mit einer langen Zahl oder einer Kombination aus Buchstaben und Zahlen“ unter \`Dokumente\\Marvel's Spider-Man 2\\\`.

## Der Ordner mit der langen Zahl

Der Unterordner ist nach deinem Konto benannt: Bei Steam ist es deine **64-Bit-Steam-ID**, die Epic-Version nutzt stattdessen eine Mischung aus Buchstaben und Zahlen. So oder so ist er für jedes Konto anders. Zwei Folgen:

- Spielen zwei Personen auf demselben PC mit verschiedenen Steam-Konten, hat jede ihren eigenen Spielstand-Ordner.
- Kopierst du Spielstände von Hand auf einen anderen PC, gehören sie in den Ordner des Steam-Kontos auf **diesem** Rechner. In einem Ordner mit anderer ID sieht das Spiel sie nicht.

Der übergeordnete Ordner \`Marvel's Spider-Man 2\` enthält außerdem das Log und Absturzberichte des Spiels (\`.log\`, \`.mdmp\`). Das sind keine Spielstände und müssen nicht gesichert werden.

## Die OneDrive-Falle

Viele Windows-PCs leiten \`Dokumente\` in OneDrive um. Ist das bei dir so, lautet der echte Pfad \`%USERPROFILE%\\OneDrive\\Documents\\Marvel's Spider-Man 2\\\`, und OneDrive synchronisiert den Ordner eigenständig, während du spielst. Das bringt zwei Probleme: OneDrive lädt womöglich einen halb geschriebenen Spielstand hoch, und „Speicherplatz freigeben“ kann den Spielstand in einen reinen Online-Platzhalter verwandeln. Verlässt du dich hier auf OneDrive, markiere den Ordner mit **Immer auf diesem Gerät behalten**.

## Hat Spider-Man 2 Cloud-Saves?

Ja, Steam Cloud, die die neuesten Spielstände zwischen Rechnern mit demselben Steam-Konto synchron hält. Ältere Versionen behält sie nicht: Geht ein Spielstand kaputt, wird der kaputte synchronisiert.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere den Ordner \`Marvel's Spider-Man 2\` aus \`Dokumente\` an einen sicheren Ort.
3. Zum Wiederherstellen das Spiel schließen und den Ordner mit der langen Zahl an dieselbe Stelle zurückkopieren, unter demselben Steam-Konto.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) sichert den Spielstand-Ordner jedes Mal, wenn du aufhörst zu spielen, und behält jede Version. Außerdem synchronisiert es ihn zwischen deinen PCs und einem Steam Deck, sodass das Spiel auf beiden dort weitermacht, wo du aufgehört hast.

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek** und prüfe, dass der für Spider-Man 2 angezeigte Ordner der unter \`Dokumente\` (oder \`OneDrive\\Documents\`) ist. Zeigt er woandershin, ändere ihn auf diesen Ordner.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Geht später ein Spielstand kaputt, holt ihn [das Wiederherstellen einer älteren Version](/guides/restore-a-game-save) zurück.

<!-- faq -->

## Häufige Fragen

### Was ist die lange Zahl im Spielstand-Ordner?

Bei Steam deine 64-Bit-Steam-ID, bei Epic die ID deines Kontos. Jedes Konto hat seinen eigenen Ordner, und das Spiel liest nur den des angemeldeten Kontos.

### Wo liegen die Spielstände von Spider-Man 2 auf dem Steam Deck?

Im Proton-Präfix: \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/\`, im Ordner mit der langen Zahl.

### Ich finde den Ordner in Dokumente nicht. Wo ist er?

Schau unter \`OneDrive\\Documents\\Marvel's Spider-Man 2\`. Bei den meisten neuen Windows-Installationen liegt Dokumente in OneDrive.

### Kann ich meine Spielstände auf den PC eines Freundes kopieren?

Die Dateien lassen sich kopieren, gehören aber in den Ordner mit der Steam-ID des Kontos auf jenem PC. Ob das Spiel Spielstände eines anderen Kontos annimmt, entscheidet das Spiel — behalte vor dem Versuch eine Kopie des Originals.
`,dn=`---
title: "Marvel's Spider-Man 2 save location (PC & Steam Deck)"
description: "Where Marvel's Spider-Man 2 keeps its PC saves, what the long-number folder is, the OneDrive trap, the Steam Deck path, and how to back saves up."
order: 22
updated: 2026-10-02
related: sync-game-saves-across-pcs, restore-a-game-save, steam-cloud-alternative
---

On PC, Marvel's Spider-Man 2 keeps its saves in \`Documents\\Marvel's Spider-Man 2\\\`, inside a subfolder named with a long number. On Steam, that number is your Steam ID. Below is what that means in practice, the OneDrive trap, the Steam Deck path, and how to keep the saves backed up.

## Where Marvel's Spider-Man 2 keeps its saves

- **Windows:** \`%USERPROFILE%\\Documents\\Marvel's Spider-Man 2\\<long number>\`
- **Steam Deck and Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<long number>\`

The PC version is Windows only, so on a Steam Deck it runs through Proton, and the saves sit inside the Proton prefix Steam keeps for the game; \`2651280\` is its Steam app ID. If the game is installed on the microSD card, the \`compatdata\` folder is on the card.

Nixxes, the studio behind the PC port, describes the save folder as "a subfolder with a long number or a combination of letters and numbers" under \`Documents\\Marvel's Spider-Man 2\\\`.

## The long-number folder

The subfolder is named after your account: on Steam it's your **64-bit Steam ID**; the Epic version uses a mix of letters and numbers instead. Either way it's different for every account. Two consequences:

- If two people play on the same PC with different Steam accounts, each has their own save folder.
- If you copy saves to another PC by hand, put them in the folder for **that** machine's Steam account. Dropped into a folder with a different ID, the game won't see them.

The parent \`Marvel's Spider-Man 2\` folder also holds the game's log and crash dumps (\`.log\`, \`.mdmp\`). Those aren't saves and don't need backing up.

## The OneDrive trap

Many Windows PCs redirect \`Documents\` into OneDrive. If yours does, the real path is \`%USERPROFILE%\\OneDrive\\Documents\\Marvel's Spider-Man 2\\\`, and OneDrive syncs the folder on its own while you play. That causes two problems: OneDrive may upload a save halfway through being written, and "Free up space" can turn the save into an online-only placeholder. If you rely on OneDrive here, mark the folder **Always keep on this device**.

## Does Spider-Man 2 have cloud saves?

Yes, Steam Cloud, which keeps the latest saves in step between machines on the same Steam account. It doesn't keep older versions: if a save breaks, the broken one is what syncs.

## Back it up by hand

1. Close the game completely.
2. Copy the \`Marvel's Spider-Man 2\` folder from \`Documents\` somewhere safe.
3. To restore, close the game and copy the long-number folder back into the same place, under the same Steam account.

## Keep it backed up and in sync with Hoard

[Hoard](/download) backs up the save folder every time you stop playing and keeps every version. It also syncs it between your PCs and a Steam Deck, so the game picks up where you left off on either one.

1. Install Hoard and sign in, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library** and check the folder it shows for Spider-Man 2 is the one under \`Documents\` (or \`OneDrive\\Documents\`). If it points somewhere else, change it to that folder.
3. Play. When you quit, the first version appears in the history.

If a save goes wrong later, [restoring an older version](/guides/restore-a-game-save) puts it back.

<!-- faq -->

## Frequently asked questions

### What is the long number in the save folder?

On Steam, your 64-bit Steam ID; on Epic, your account's ID. Every account gets its own folder, and the game only reads the one for the account that's signed in.

### Where are Spider-Man 2 saves on Steam Deck?

Inside the Proton prefix: \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/\`, in the long-number folder.

### I can't find the folder in Documents. Where is it?

Check \`OneDrive\\Documents\\Marvel's Spider-Man 2\`. On most new Windows installs, Documents lives inside OneDrive.

### Can I copy my saves to a friend's PC?

The files will copy, but they go in the folder named after the Steam ID of the account on that PC. Whether the game accepts saves made on a different account is up to the game, so keep a copy of the original before you try.
`,ln=`---
title: "Dónde están las partidas de Marvel's Spider-Man 2 (PC y Steam Deck)"
description: "Dónde guarda Marvel's Spider-Man 2 sus partidas en PC, qué es la carpeta del número largo, la trampa de OneDrive, la ruta en Steam Deck y cómo copiarlas."
order: 22
updated: 2026-10-02
---

En PC, Marvel's Spider-Man 2 guarda sus partidas en \`Documentos\\Marvel's Spider-Man 2\\\`, dentro de una subcarpeta con un número largo. En Steam, ese número es tu ID de Steam. Debajo tienes qué significa eso en la práctica, la trampa de OneDrive, la ruta en Steam Deck y cómo tener las partidas siempre copiadas.

## Dónde guarda Marvel's Spider-Man 2 las partidas

- **Windows:** \`%USERPROFILE%\\Documents\\Marvel's Spider-Man 2\\<número largo>\`
- **Steam Deck y Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<número largo>\`

La versión de PC es sólo para Windows, así que en una Steam Deck corre con Proton y las partidas están dentro del prefijo de Proton que Steam mantiene para el juego; \`2651280\` es su ID en Steam. Si está instalado en la microSD, la carpeta \`compatdata\` está en la tarjeta.

Nixxes, el estudio del port de PC, describe la carpeta de partidas como «una subcarpeta con un número largo o una combinación de letras y números» dentro de \`Documentos\\Marvel's Spider-Man 2\\\`.

## La carpeta del número largo

La subcarpeta lleva el nombre de tu cuenta: en Steam es tu **ID de Steam de 64 bits**; la versión de Epic usa en su lugar una mezcla de letras y números. En cualquier caso es distinta para cada cuenta. Dos consecuencias:

- Si dos personas juegan en el mismo PC con cuentas de Steam distintas, cada una tiene su propia carpeta de partidas.
- Si copias partidas a otro PC a mano, ponlas en la carpeta de la cuenta de Steam de **esa** máquina. En una carpeta con otro ID, el juego no las ve.

La carpeta padre \`Marvel's Spider-Man 2\` también guarda el registro del juego y los volcados de errores (\`.log\`, \`.mdmp\`). No son partidas y no hace falta copiarlos.

## La trampa de OneDrive

Muchos PC con Windows redirigen \`Documentos\` a OneDrive. Si es tu caso, la ruta real es \`%USERPROFILE%\\OneDrive\\Documents\\Marvel's Spider-Man 2\\\`, y OneDrive sincroniza la carpeta por su cuenta mientras juegas. Eso trae dos problemas: OneDrive puede subir una partida a medio escribir, y «Liberar espacio» puede convertir la partida en un marcador que sólo está en línea. Si dependes de OneDrive aquí, marca la carpeta como **Mantener siempre en este dispositivo**.

## ¿Spider-Man 2 tiene partidas en la nube?

Sí, Steam Cloud, que mantiene al día las últimas partidas entre máquinas con la misma cuenta de Steam. No guarda versiones anteriores: si una partida se rompe, lo que se sincroniza es la rota.

## Cópiala a mano

1. Cierra el juego del todo.
2. Copia la carpeta \`Marvel's Spider-Man 2\` de \`Documentos\` a un sitio seguro.
3. Para restaurar, cierra el juego y vuelve a copiar la carpeta del número largo al mismo sitio, con la misma cuenta de Steam.

## Copia y sincronización automáticas con Hoard

[Hoard](/download) copia la carpeta de partidas cada vez que dejas de jugar y guarda todas las versiones. También la sincroniza entre tus PC y una Steam Deck, así que el juego sigue donde lo dejaste en cualquiera de los dos.

1. Instala Hoard e inicia sesión, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca** y comprueba que la carpeta que aparece para Spider-Man 2 es la de \`Documentos\` (o \`OneDrive\\Documents\`). Si apunta a otro sitio, cámbiala a esa carpeta.
3. Juega. Al salir, la primera versión aparece en el historial.

Si más adelante una partida se estropea, [restaurar una versión anterior](/guides/restore-a-game-save) la devuelve.

<!-- faq -->

## Preguntas frecuentes

### ¿Qué es el número largo de la carpeta de partidas?

En Steam, tu ID de Steam de 64 bits; en Epic, el ID de tu cuenta. Cada cuenta tiene su propia carpeta, y el juego sólo lee la de la cuenta con la que has iniciado sesión.

### ¿Dónde están las partidas de Spider-Man 2 en Steam Deck?

Dentro del prefijo de Proton: \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/\`, en la carpeta del número largo.

### No encuentro la carpeta en Documentos. ¿Dónde está?

Mira en \`OneDrive\\Documents\\Marvel's Spider-Man 2\`. En la mayoría de instalaciones nuevas de Windows, Documentos vive dentro de OneDrive.

### ¿Puedo copiar mis partidas al PC de un amigo?

Los ficheros se copian, pero van en la carpeta con el ID de Steam de la cuenta de ese PC. Que el juego acepte partidas hechas con otra cuenta depende del juego, así que guarda una copia del original antes de probar.
`,cn=`---
title: "Emplacement des sauvegardes de Marvel's Spider-Man 2 (PC et Steam Deck)"
description: "Où Marvel's Spider-Man 2 range ses sauvegardes sur PC, ce qu'est le dossier au long numéro, le piège OneDrive, le chemin sur Steam Deck et comment sauvegarder."
order: 22
updated: 2026-10-02
---

Sur PC, Marvel's Spider-Man 2 range ses sauvegardes dans \`Documents\\Marvel's Spider-Man 2\\\`, dans un sous-dossier au long numéro. Sur Steam, ce numéro est votre identifiant Steam. Voici ce que cela implique, le piège OneDrive, le chemin sur Steam Deck et comment garder vos sauvegardes à l'abri.

## Où Marvel's Spider-Man 2 range ses sauvegardes

- **Windows :** \`%USERPROFILE%\\Documents\\Marvel's Spider-Man 2\\<long numéro>\`
- **Steam Deck et Linux** (Proton) : \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<long numéro>\`

La version PC n'existe que sous Windows : sur Steam Deck, elle passe par Proton, et les sauvegardes se trouvent dans le préfixe Proton que Steam garde pour le jeu ; \`2651280\` est son identifiant Steam. S'il est installé sur la carte microSD, le dossier \`compatdata\` est sur la carte.

Nixxes, le studio derrière le portage PC, décrit le dossier de sauvegarde comme « un sous-dossier avec un long numéro ou une combinaison de lettres et de chiffres » sous \`Documents\\Marvel's Spider-Man 2\\\`.

## Le dossier au long numéro

Le sous-dossier porte le nom de votre compte : sur Steam, c'est votre **identifiant Steam 64 bits** ; la version Epic utilise plutôt un mélange de lettres et de chiffres. Dans tous les cas, il diffère pour chaque compte. Deux conséquences :

- Si deux personnes jouent sur le même PC avec des comptes Steam différents, chacune a son propre dossier de sauvegarde.
- Si vous copiez des sauvegardes à la main sur un autre PC, mettez-les dans le dossier du compte Steam de **cette** machine. Dans un dossier avec un autre identifiant, le jeu ne les voit pas.

Le dossier parent \`Marvel's Spider-Man 2\` contient aussi le journal du jeu et des rapports de plantage (\`.log\`, \`.mdmp\`). Ce ne sont pas des sauvegardes et inutile de les sauvegarder.

## Le piège OneDrive

Beaucoup de PC Windows redirigent \`Documents\` vers OneDrive. Si c'est votre cas, le vrai chemin est \`%USERPROFILE%\\OneDrive\\Documents\\Marvel's Spider-Man 2\\\`, et OneDrive synchronise le dossier de lui-même pendant que vous jouez. Deux problèmes : OneDrive peut envoyer une sauvegarde à moitié écrite, et « Libérer de l'espace » peut transformer la sauvegarde en fichier disponible uniquement en ligne. Si vous comptez sur OneDrive ici, marquez le dossier **Toujours conserver sur cet appareil**.

## Spider-Man 2 a-t-il des sauvegardes cloud ?

Oui, Steam Cloud, qui garde les dernières sauvegardes synchronisées entre les machines d'un même compte Steam. Il ne garde pas les anciennes versions : si une sauvegarde casse, c'est la version cassée qui se synchronise.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez le dossier \`Marvel's Spider-Man 2\` de \`Documents\` vers un endroit sûr.
3. Pour restaurer, fermez le jeu et recopiez le dossier au long numéro au même endroit, sous le même compte Steam.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) sauvegarde le dossier à chaque fois que vous arrêtez de jouer et garde toutes les versions. Il le synchronise aussi entre vos PC et un Steam Deck, pour que le jeu reprenne là où vous l'avez laissé sur l'un comme sur l'autre.

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque** et vérifiez que le dossier affiché pour Spider-Man 2 est celui sous \`Documents\` (ou \`OneDrive\\Documents\`). S'il pointe ailleurs, changez-le pour ce dossier.
3. Jouez. En quittant, la première version apparaît dans l'historique.

Si une sauvegarde tourne mal plus tard, [restaurer une version antérieure](/guides/restore-a-game-save) la remet en place.

<!-- faq -->

## Questions fréquentes

### Qu'est-ce que le long numéro du dossier de sauvegarde ?

Sur Steam, votre identifiant Steam 64 bits ; sur Epic, l'identifiant de votre compte. Chaque compte a son propre dossier, et le jeu ne lit que celui du compte connecté.

### Où sont les sauvegardes de Spider-Man 2 sur Steam Deck ?

Dans le préfixe Proton : \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/\`, dans le dossier au long numéro.

### Je ne trouve pas le dossier dans Documents. Où est-il ?

Regardez dans \`OneDrive\\Documents\\Marvel's Spider-Man 2\`. Sur la plupart des installations récentes de Windows, Documents se trouve dans OneDrive.

### Puis-je copier mes sauvegardes sur le PC d'un ami ?

Les fichiers se copient, mais ils vont dans le dossier portant l'identifiant Steam du compte de ce PC. Que le jeu accepte des sauvegardes créées sur un autre compte dépend du jeu : gardez une copie de l'original avant d'essayer.
`,mn=`---
title: "Dove sono i salvataggi di Marvel's Spider-Man 2 (PC e Steam Deck)"
description: "Dove Marvel's Spider-Man 2 tiene i salvataggi su PC, cos'è la cartella col numero lungo, la trappola di OneDrive, il percorso su Steam Deck e come farne il backup."
order: 22
updated: 2026-10-02
---

Su PC, Marvel's Spider-Man 2 tiene i salvataggi in \`Documenti\\Marvel's Spider-Man 2\\\`, dentro una sottocartella con un numero lungo. Su Steam, quel numero è il tuo ID Steam. Qui sotto trovi cosa significa in pratica, la trappola di OneDrive, il percorso su Steam Deck e come tenere i salvataggi al sicuro.

## Dove Marvel's Spider-Man 2 tiene i salvataggi

- **Windows:** \`%USERPROFILE%\\Documents\\Marvel's Spider-Man 2\\<numero lungo>\`
- **Steam Deck e Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<numero lungo>\`

La versione PC è solo per Windows, quindi su Steam Deck gira con Proton e i salvataggi stanno nel prefisso Proton che Steam tiene per il gioco; \`2651280\` è il suo ID Steam. Se è installato sulla microSD, la cartella \`compatdata\` è sulla scheda.

Nixxes, lo studio dietro la conversione per PC, descrive la cartella dei salvataggi come "una sottocartella con un numero lungo o una combinazione di lettere e numeri" sotto \`Documenti\\Marvel's Spider-Man 2\\\`.

## La cartella col numero lungo

La sottocartella prende il nome dal tuo account: su Steam è il tuo **ID Steam a 64 bit**; la versione Epic usa invece un misto di lettere e numeri. In ogni caso è diversa per ogni account. Due conseguenze:

- Se due persone giocano sullo stesso PC con account Steam diversi, ognuna ha la propria cartella di salvataggio.
- Se copi i salvataggi a mano su un altro PC, mettili nella cartella dell'account Steam di **quella** macchina. In una cartella con un altro ID, il gioco non li vede.

La cartella madre \`Marvel's Spider-Man 2\` contiene anche il log del gioco e i crash dump (\`.log\`, \`.mdmp\`). Non sono salvataggi e non serve farne il backup.

## La trappola di OneDrive

Molti PC Windows reindirizzano \`Documenti\` su OneDrive. Se è il tuo caso, il percorso reale è \`%USERPROFILE%\\OneDrive\\Documents\\Marvel's Spider-Man 2\\\`, e OneDrive sincronizza la cartella da solo mentre giochi. Ne nascono due problemi: OneDrive può caricare un salvataggio scritto a metà, e "Libera spazio" può trasformare il salvataggio in un segnaposto solo online. Se qui ti affidi a OneDrive, imposta la cartella su **Mantieni sempre su questo dispositivo**.

## Spider-Man 2 ha i salvataggi nel cloud?

Sì, Steam Cloud, che tiene allineati gli ultimi salvataggi tra le macchine con lo stesso account Steam. Non tiene le versioni precedenti: se un salvataggio si rompe, si sincronizza quello rotto.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia la cartella \`Marvel's Spider-Man 2\` da \`Documenti\` in un posto sicuro.
3. Per ripristinare, chiudi il gioco e ricopia la cartella col numero lungo nello stesso posto, con lo stesso account Steam.

## Backup e sync automatici con Hoard

[Hoard](/download) fa il backup della cartella ogni volta che smetti di giocare e tiene tutte le versioni. La sincronizza anche tra i tuoi PC e una Steam Deck, così il gioco riprende da dove l'hai lasciato su entrambi.

1. Installa Hoard e accedi, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria** e controlla che la cartella mostrata per Spider-Man 2 sia quella sotto \`Documenti\` (o \`OneDrive\\Documents\`). Se punta altrove, cambiala con quella cartella.
3. Gioca. Quando esci, la prima versione compare nella cronologia.

Se più avanti un salvataggio si rompe, [ripristinare una versione precedente](/guides/restore-a-game-save) lo rimette a posto.

<!-- faq -->

## Domande frequenti

### Cos'è il numero lungo nella cartella dei salvataggi?

Su Steam, il tuo ID Steam a 64 bit; su Epic, l'ID del tuo account. Ogni account ha la propria cartella, e il gioco legge solo quella dell'account connesso.

### Dove sono i salvataggi di Spider-Man 2 su Steam Deck?

Nel prefisso Proton: \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/\`, nella cartella col numero lungo.

### Non trovo la cartella in Documenti. Dov'è?

Guarda in \`OneDrive\\Documents\\Marvel's Spider-Man 2\`. Nella maggior parte delle installazioni recenti di Windows, Documenti sta dentro OneDrive.

### Posso copiare i miei salvataggi sul PC di un amico?

I file si copiano, ma vanno nella cartella con l'ID Steam dell'account di quel PC. Se il gioco accetti salvataggi creati con un altro account dipende dal gioco: tieni una copia dell'originale prima di provare.
`,pn=`---
title: "Marvel's Spider-Man 2 のセーブデータの場所（PC・Steam Deck）"
description: "Marvel's Spider-Man 2のPC版セーブデータの場所、長い数字のフォルダーの正体、OneDriveの落とし穴、Steam Deckでのパス、バックアップ方法を解説。"
order: 22
updated: 2026-10-02
---

PC 版の Marvel's Spider-Man 2 は、セーブデータを \`ドキュメント\\Marvel's Spider-Man 2\\\` の中の、長い数字の名前のサブフォルダーに保存します。Steam では、その数字はあなたの Steam ID です。以下では、それが実際に意味すること、OneDrive の落とし穴、Steam Deck でのパス、そしてセーブのバックアップを保つ方法を説明します。

## Marvel's Spider-Man 2 のセーブデータの場所

- **Windows:** \`%USERPROFILE%\\Documents\\Marvel's Spider-Man 2\\<長い数字>\`
- **Steam Deck と Linux**（Proton）: \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<長い数字>\`

PC 版は Windows 専用なので、Steam Deck では Proton で動き、セーブは Steam がこのゲーム用に用意する Proton プレフィックスの中にあります。\`2651280\` はこのゲームの Steam アプリ ID です。microSD カードにインストールしている場合、\`compatdata\` フォルダーはカード上にあります。

PC 版を手がけたスタジオ Nixxes は、セーブフォルダーを \`ドキュメント\\Marvel's Spider-Man 2\\\` の下にある「長い数字、または英数字の組み合わせのサブフォルダー」と説明しています。

## 長い数字のフォルダー

サブフォルダーの名前はアカウントによって決まります。Steam では **64 ビットの Steam ID**、Epic 版では英数字の組み合わせです。いずれにせよ、アカウントごとに異なります。ここから 2 つのことが言えます。

- 同じ PC で 2 人が別々の Steam アカウントで遊ぶ場合、それぞれに専用のセーブフォルダーがあります。
- 手動で別の PC にセーブをコピーするときは、**その**マシンの Steam アカウントのフォルダーに入れてください。別の ID のフォルダーに置いても、ゲームは認識しません。

親フォルダーの \`Marvel's Spider-Man 2\` には、ゲームのログやクラッシュダンプ（\`.log\`、\`.mdmp\`）も入っています。これらはセーブではないので、バックアップは不要です。

## OneDrive の落とし穴

多くの Windows PC では \`ドキュメント\` が OneDrive にリダイレクトされています。その場合、実際のパスは \`%USERPROFILE%\\OneDrive\\Documents\\Marvel's Spider-Man 2\\\` で、プレイ中に OneDrive がフォルダーを勝手に同期します。問題は 2 つです。書き込み途中のセーブを OneDrive がアップロードすることがあり、「空き領域を増やす」でセーブがオンライン専用のプレースホルダーになることがあります。ここで OneDrive に頼るなら、フォルダーを **このデバイス上に常に保持する** に設定してください。

## Spider-Man 2 にクラウドセーブはありますか？

あります。Steam クラウドが、同じ Steam アカウントのマシン間で最新のセーブをそろえます。古いバージョンは残しません。セーブが壊れると、壊れたものが同期されます。

## 手動でバックアップする

1. ゲームを完全に終了します。
2. \`ドキュメント\` の \`Marvel's Spider-Man 2\` フォルダーを安全な場所にコピーします。
3. 復元するときは、ゲームを終了し、長い数字のフォルダーを同じ場所、同じ Steam アカウントにコピーし戻します。

## Hoard で自動バックアップと同期

[Hoard](/download) は、プレイをやめるたびにセーブフォルダーをバックアップし、すべてのバージョンを保持します。さらに PC と Steam Deck の間で同期するので、どちらでも続きから遊べます。

1. Hoard をインストールしてサインインするか、[自分のサーバー](/guides/self-host-hoard)を指定します。
2. **ライブラリ** を開き、Spider-Man 2 に表示されているフォルダーが \`ドキュメント\`（または \`OneDrive\\Documents\`）の下のものか確認します。別の場所を指していたら、そのフォルダーに変更してください。
3. プレイします。終了すると、最初のバージョンが履歴に表示されます。

後でセーブがおかしくなっても、[以前のバージョンを復元](/guides/restore-a-game-save)すれば元に戻せます。

<!-- faq -->

## よくある質問

### セーブフォルダーの長い数字は何ですか？

Steam では 64 ビットの Steam ID、Epic ではアカウントの ID です。アカウントごとにフォルダーがあり、ゲームはサインイン中のアカウントのものだけを読み込みます。

### Steam Deck での Spider-Man 2 のセーブデータはどこですか？

Proton プレフィックスの中です: \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/\` の、長い数字のフォルダーです。

### ドキュメントにフォルダーが見つかりません。どこにありますか？

\`OneDrive\\Documents\\Marvel's Spider-Man 2\` を確認してください。最近の Windows の多くでは、ドキュメントは OneDrive の中にあります。

### 友達の PC にセーブをコピーできますか？

ファイル自体はコピーできますが、その PC のアカウントの Steam ID のフォルダーに入れる必要があります。別のアカウントで作ったセーブを受け付けるかはゲーム次第なので、試す前に元のデータのコピーを取っておいてください。
`,hn=`---
title: "Onde ficam os saves de Marvel's Spider-Man 2 (PC e Steam Deck)"
description: "Onde o Marvel's Spider-Man 2 guarda os saves no PC, o que é a pasta do número longo, a armadilha do OneDrive, o caminho na Steam Deck e como fazer backup."
order: 22
updated: 2026-10-02
---

No PC, o Marvel's Spider-Man 2 guarda os saves em \`Documentos\\Marvel's Spider-Man 2\\\`, dentro de uma subpasta com um número longo. No Steam, esse número é o teu ID do Steam. Abaixo tens o que isso significa na prática, a armadilha do OneDrive, o caminho na Steam Deck e como manter os saves sempre com backup.

## Onde o Marvel's Spider-Man 2 guarda os saves

- **Windows:** \`%USERPROFILE%\\Documents\\Marvel's Spider-Man 2\\<número longo>\`
- **Steam Deck e Linux** (Proton): \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<número longo>\`

A versão de PC é só para Windows, por isso na Steam Deck corre com o Proton e os saves estão dentro do prefixo do Proton que o Steam mantém para o jogo; \`2651280\` é o seu ID no Steam. Se estiver instalado no cartão microSD, a pasta \`compatdata\` está no cartão.

A Nixxes, o estúdio responsável pela versão de PC, descreve a pasta de saves como «uma subpasta com um número longo ou uma combinação de letras e números» dentro de \`Documentos\\Marvel's Spider-Man 2\\\`.

## A pasta do número longo

A subpasta tem o nome da tua conta: no Steam é o teu **ID do Steam de 64 bits**; a versão da Epic usa antes uma mistura de letras e números. Em qualquer caso, é diferente para cada conta. Duas consequências:

- Se duas pessoas jogarem no mesmo PC com contas Steam diferentes, cada uma tem a sua pasta de saves.
- Se copiares saves à mão para outro PC, põe-nos na pasta da conta Steam **dessa** máquina. Numa pasta com outro ID, o jogo não os vê.

A pasta mãe \`Marvel's Spider-Man 2\` também guarda o log do jogo e os crash dumps (\`.log\`, \`.mdmp\`). Não são saves e não precisam de backup.

## A armadilha do OneDrive

Muitos PCs com Windows redirecionam \`Documentos\` para o OneDrive. Se for o teu caso, o caminho real é \`%USERPROFILE%\\OneDrive\\Documents\\Marvel's Spider-Man 2\\\`, e o OneDrive sincroniza a pasta por conta própria enquanto jogas. Isso traz dois problemas: o OneDrive pode enviar um save escrito a meio, e «Libertar espaço» pode transformar o save num marcador só online. Se dependes do OneDrive aqui, marca a pasta como **Manter sempre neste dispositivo**.

## O Spider-Man 2 tem saves na nuvem?

Sim, Steam Cloud, que mantém em dia os últimos saves entre máquinas com a mesma conta Steam. Não guarda versões anteriores: se um save se estragar, é o estragado que se sincroniza.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a pasta \`Marvel's Spider-Man 2\` de \`Documentos\` para um sítio seguro.
3. Para restaurar, fecha o jogo e volta a copiar a pasta do número longo para o mesmo sítio, com a mesma conta Steam.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) faz backup da pasta de saves sempre que deixas de jogar e guarda todas as versões. Também a sincroniza entre os teus PCs e uma Steam Deck, para que o jogo continue onde o deixaste em qualquer um deles.

1. Instala o Hoard e inicia sessão, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca** e confirma que a pasta mostrada para o Spider-Man 2 é a de \`Documentos\` (ou \`OneDrive\\Documents\`). Se apontar para outro sítio, muda-a para essa pasta.
3. Joga. Ao sair, a primeira versão aparece no histórico.

Se mais tarde um save se estragar, [restaurar uma versão anterior](/guides/restore-a-game-save) devolve-o.

<!-- faq -->

## Perguntas frequentes

### O que é o número longo da pasta de saves?

No Steam, o teu ID do Steam de 64 bits; na Epic, o ID da tua conta. Cada conta tem a sua pasta, e o jogo só lê a da conta com sessão iniciada.

### Onde ficam os saves de Spider-Man 2 na Steam Deck?

Dentro do prefixo do Proton: \`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/\`, na pasta do número longo.

### Não encontro a pasta em Documentos. Onde está?

Procura em \`OneDrive\\Documents\\Marvel's Spider-Man 2\`. Na maioria das instalações recentes do Windows, Documentos fica dentro do OneDrive.

### Posso copiar os meus saves para o PC de um amigo?

Os ficheiros copiam-se, mas vão para a pasta com o ID do Steam da conta desse PC. Se o jogo aceita saves feitos noutra conta depende do jogo, por isso guarda uma cópia do original antes de experimentar.
`,vn=`---
title: "漫威蜘蛛侠 2（Marvel's Spider-Man 2）存档位置（PC 与 Steam Deck）"
description: "Marvel's Spider-Man 2 的 PC 存档位置、那个长数字文件夹是什么、OneDrive 的陷阱、Steam Deck 上的路径，以及如何备份存档。"
order: 22
updated: 2026-10-02
---

在 PC 上，Marvel's Spider-Man 2 把存档放在 \`文档\\Marvel's Spider-Man 2\\\` 里一个以长数字命名的子文件夹中。在 Steam 上，这个数字就是你的 Steam ID。下面介绍这在实际中意味着什么、OneDrive 的陷阱、Steam Deck 上的路径，以及如何让存档一直有备份。

## Marvel's Spider-Man 2 的存档位置

- **Windows：**\`%USERPROFILE%\\Documents\\Marvel's Spider-Man 2\\<长数字>\`
- **Steam Deck 和 Linux**（Proton）：\`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<长数字>\`

PC 版只支持 Windows，所以在 Steam Deck 上通过 Proton 运行，存档位于 Steam 为该游戏维护的 Proton 前缀中；\`2651280\` 是它的 Steam 应用 ID。如果装在 microSD 卡上，\`compatdata\` 文件夹就在卡上。

负责 PC 移植的工作室 Nixxes 把存档文件夹描述为 \`文档\\Marvel's Spider-Man 2\\\` 下“一个以长数字或字母数字组合命名的子文件夹”。

## 长数字文件夹

子文件夹以你的账号命名：在 Steam 上是你的 **64 位 Steam ID**；Epic 版则用字母和数字的组合。无论哪种，每个账号都不一样。由此有两个后果：

- 如果两个人在同一台 PC 上用不同的 Steam 账号玩，每人都有自己的存档文件夹。
- 如果你手动把存档复制到另一台 PC，要放进**那台**设备上 Steam 账号对应的文件夹。放进 ID 不同的文件夹，游戏是看不到的。

上一级的 \`Marvel's Spider-Man 2\` 文件夹里还有游戏日志和崩溃转储（\`.log\`、\`.mdmp\`）。它们不是存档，不需要备份。

## OneDrive 的陷阱

很多 Windows PC 会把“文档”重定向到 OneDrive。如果你也是这样，真实路径是 \`%USERPROFILE%\\OneDrive\\Documents\\Marvel's Spider-Man 2\\\`，而且在你游玩时 OneDrive 会自行同步这个文件夹。这会带来两个问题：OneDrive 可能上传写到一半的存档，“释放空间”还可能把存档变成仅在线的占位文件。如果你在这里依赖 OneDrive，请把文件夹设为**始终保留在此设备上**。

## Spider-Man 2 有云存档吗？

有，Steam 云会在同一 Steam 账号的设备之间同步最新的存档。它不保留旧版本：存档一旦损坏，同步过去的就是损坏的那份。

## 手动备份

1. 完全关闭游戏。
2. 把“文档”里的 \`Marvel's Spider-Man 2\` 文件夹复制到安全的地方。
3. 恢复时，关闭游戏，把长数字文件夹复制回同一位置，使用同一个 Steam 账号。

## 用 Hoard 自动备份和同步

[Hoard](/download) 会在你每次停止游戏时备份存档文件夹，并保留每一个版本。它还会在你的 PC 和 Steam Deck 之间同步，让游戏在任意一台上都能接着玩。

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**，确认 Spider-Man 2 显示的文件夹是“文档”（或 \`OneDrive\\Documents\`）下的那个。如果指向别处，请改成该文件夹。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

如果之后存档出了问题，[恢复旧版本](/guides/restore-a-game-save)就能把它找回来。

<!-- faq -->

## 常见问题

### 存档文件夹里的长数字是什么？

在 Steam 上是你的 64 位 Steam ID；在 Epic 上是你的账号 ID。每个账号都有自己的文件夹，游戏只读取当前登录账号的那个。

### Steam Deck 上的 Spider-Man 2 存档在哪里？

在 Proton 前缀里：\`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/\` 下的长数字文件夹。

### 我在“文档”里找不到这个文件夹，它在哪里？

去 \`OneDrive\\Documents\\Marvel's Spider-Man 2\` 找找。在大多数新装的 Windows 上，“文档”位于 OneDrive 里。

### 我能把存档复制到朋友的 PC 上吗？

文件可以复制，但要放进那台 PC 上账号的 Steam ID 文件夹里。游戏是否接受用其他账号创建的存档取决于游戏本身，所以尝试前先保留一份原始存档。
`,gn=`---
title: "Steam-Cloud-Alternative: sichere die Spielstände, die Steam nicht sichert"
description: "Steam Cloud lässt viele Spiele aus und hat keinen Verlauf. Sichere jedes Spiel aus jedem Launcher, mit Versionen zum Zurücksetzen. Cloud oder selbst gehostet."
order: 7
updated: 2026-09-01
---

Steam Cloud macht die eng umrissene Aufgabe, die sie hat, wirklich gut, und die meisten stoßen erst an dem Tag an ihre Grenzen, an dem etwas verloren geht. Diese Anleitung zeigt, wo diese Grenzen liegen und was mit den Spielen zu tun ist, die dahinter liegen.

## Was Steam Cloud tatsächlich abdeckt

Steam Cloud synchronisiert den Ordner eines Spiels, wenn **der Entwickler es eingerichtet hat** — indem er angibt, welche Dateien zu synchronisieren sind, oder indem das Spiel die Steam-API aufruft. Das ist das ganze Modell, und daraus folgen drei Dinge:

- Es funktioniert nur für Spiele, die über Steam gekauft und gestartet werden.
- Ob es überhaupt funktioniert, entscheidet der Entwickler, pro Spiel und manchmal pro Plattform.
- Jedes Spiel hat sein eigenes Speicherkontingent, festgelegt von diesem Entwickler.

Wenn es funktioniert, ist es unsichtbar und hervorragend: Spiel auf dem einen PC schließen, auf dem anderen öffnen, Fortschritt ist da.

## Wo es dich im Regen stehen lässt

- **Alles, was kein Steam-Spiel ist.** GOG, Epic, itch, Battle.net, die Xbox-App, Emulatoren, alles von Hand Installierte. Steam weiß nicht, dass es existiert.
- **Steam-Spiele, bei denen es nie aktiviert wurde.** Viele Titel, gerade ältere oder kleinere, haben es schlicht nicht. Die Shop-Seite sagt es, aber niemand schaut nach, bevor er 60 Stunden investiert.
- **Es gibt kein Zurück.** Das ist der große Punkt. Steam hält den aktuellen Zustand deines Spielstands, nicht dessen Geschichte. Wird die Datei beschädigt, frisst ein Mod deine Welt, oder überschreibst du einen guten Stand mit einem schlechten, dann ist die Cloud-Kopie bereits der schlechte. Du kannst die Dateien ansehen, die Steam für ein Spiel hält, aber es gibt keine frühere Version zum Wiederherstellen.
- **Der Konfliktdialog.** Wenn Steam local und remote für uneinig hält, sollst du wählen — mit kaum mehr als zwei Zeitstempeln als Grundlage. Wählst du falsch, ist die andere Kopie weg.

## Was Hoard ergänzt

Hoard beobachtet den Ordner, in den ein Spiel wirklich schreibt, und sichert **nach jedem Spielen eine neue Version**:

- **Woher das Spiel stammt, ist egal.** Steam, GOG, Epic, itch, Emulatoren oder ein Ordner, auf den du es von Hand richtest.
- **Jede Version bleibt erhalten**, ein beschädigter Stand oder eine schlechte Entscheidung kosten also zwei Klicks statt eines Spieldurchgangs.
- **Es synchronisiert zwischen deinen Geräten**, Steam Deck und Desktop eingeschlossen.
- **Nichts wird stillschweigend zerstört.** Der ersetzte Stand wird zuerst gesichert, selbst eine falsche Wiederherstellung ist also umkehrbar.

Snapshots werden per Inhalts-Hash gespeichert, zehn Versionen eines 2 GB großen Stands kosten also etwa 2 GB, nicht 20 — und das macht die komplette Historie überhaupt praktikabel.

## Beides gleichzeitig nutzen

Sie stören sich nicht, du musst dich nicht entscheiden. Bei einem Steam-Spiel mit Cloud-Unterstützung lass Steam synchronisieren, was es ohnehin tut; Hoards Beitrag dort ist die Historie — genau das, was Steam nicht führt. Für alles andere übernimmt Hoard auch die Synchronisierung.

Ein Detail, das zählt, wenn du neben dem Desktop ein Steam Deck hast: Hoard verfolgt \`<AppID>/remote/\` innerhalb von \`userdata\`, nicht den Ordner darüber, denn der enthält \`remotecache.vdf\` und gerätebezogene Dateien für Erfolge und Spielzeit. Genau diese Unterscheidung geht bei selbstgebauter Synchronisierung meist schief, weshalb solche Setups bei jedem Start zu kollidieren scheinen.

## Wann Steam Cloud reicht

Ehrlich gesagt: wenn alle deine Spiele Steam-Spiele mit Cloud-Unterstützung sind, du an einem PC spielst und noch nie einen Spielstand zurücknehmen musstest, erledigt Steam Cloud die Aufgabe und du brauchst nichts weiter. Für Hoard sprechen die Versionshistorie, Spiele außerhalb von Steam und Geräte, die Steam Cloud nicht erreicht.

## Ganz ohne fremde Cloud

Wenn der Reiz darin liegt, von keiner Plattform abzuhängen: Hoard läuft komplett auf deiner eigenen Hardware — \`hoard-server\` auf einem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört.

<!-- faq -->

## Häufige Fragen

### Ersetzt Hoard Steam Cloud?

Muss es nicht. Steam Cloud hält deinen aktuellen Stand für die unterstützten Spiele synchron; Hoard ergänzt die Versionshistorie und deckt die übrigen Spiele ab. Beides parallel zu nutzen ist der Normalfall.

### Kann Steam Cloud zu einem älteren Spielstand zurück?

Nein. Steam hält den aktuellen Zustand der Dateien, nicht deren Geschichte. Ist ein schlechter Stand einmal synchronisiert, steht genau der in der Cloud. Zurück geht es nur mit einem versionierenden Werkzeug.

### Warum synchronisieren nicht alle meine Steam-Spiele?

Weil der Entwickler es aktiviert, pro Spiel und manchmal pro Plattform. Die Shop-Seite führt Steam Cloud unter den Features auf, wenn es unterstützt wird — und viele Titel tun das schlicht nicht.

### Funktioniert Hoard mit Nicht-Steam-Spielen?

Ja, das ist ein Großteil des Sinns. Es findet Spielstände über eine Community-Datenbank mit über 20.000 Titeln, aus jedem Store, und für Ungewöhnliches richtest du es von Hand auf einen Ordner.

### Gibt es Konflikte, wenn beides läuft?

Nein. Hoard sichert eine Version, nachdem du aufgehört hast und der Ordner zur Ruhe kommt, und überschreibt nie, ohne das Ersetzte vorher zu sichern.

### Kann ich meine Stände aus beiden Clouds heraushalten?

Ja. Hoste den Server selbst, dann verlassen deine Spielstände nie deine eigene Hardware — ohne Konto und ohne Telemetrie an irgendwen.
`,Sn=`---
title: "Steam Cloud alternative: back up the saves Steam doesn't"
description: "Steam Cloud skips many games and keeps no history. Back up every game from any launcher, with versions you can roll back. In the cloud or self-hosted."
order: 7
updated: 2026-09-01
related: sync-game-saves-across-pcs, restore-a-game-save, game-save-sync-comparison
---

Steam Cloud is genuinely good at the narrow job it does, and most people only find its edges the day they lose something. This guide explains exactly where those edges are, and what to do about the games that fall outside them.

## What Steam Cloud actually covers

Steam Cloud syncs a folder for a game when **the developer set it up** — either by declaring which files to sync, or by calling the Steam API from inside the game. That's the whole model, and three things follow from it:

- It only works for games bought and launched through Steam.
- Whether it works at all is the developer's decision, per game, and sometimes per platform.
- Each game has its own storage allowance, set by that developer.

When it works, it's invisible and excellent: you close the game on one PC, open it on another, and your progress is there.

## Where it leaves you exposed

- **Everything that isn't a Steam game.** GOG, Epic, itch, Battle.net, the Xbox app, emulators, anything installed by hand. Steam doesn't know they exist.
- **Steam games where it was never switched on.** Plenty of titles, especially older or smaller ones, simply don't have it. The store page tells you, but nobody checks before starting a 60-hour run.
- **There is no going back.** This is the big one. Steam holds the current state of your save, not a history of it. Corrupt the file, let a mod eat your world, or overwrite a good save with a bad one, and the cloud copy is already the bad one. You can browse the files Steam is holding for a game, but there's no earlier version to restore.
- **The conflict dialog.** When Steam thinks the local and remote saves disagree, it asks you to choose, with little more than two timestamps to go on. Choose wrong and the other copy is gone.

## What Hoard adds

Hoard watches the folder each game actually writes to and captures a **new version every time you finish playing**:

- **It doesn't care where a game came from.** Steam, GOG, Epic, itch, emulators, a folder you pointed it at by hand.
- **Every version is kept**, so rolling back a corrupted save or a bad decision is two clicks rather than a lost run.
- **It syncs between your machines** the same way, including a Steam Deck and a desktop.
- **Nothing is destroyed silently.** The save being replaced is captured first, so even a wrong restore is reversible.

Snapshots are stored by content hash, so ten versions of a 2 GB save cost about 2 GB, not 20 — which is what makes keeping the whole history practical.

## Using both at once

They don't fight, and you don't have to pick. For a Steam game with cloud support, let Steam do the syncing it's already doing; Hoard's contribution there is the history — the thing Steam doesn't keep. For everything else, Hoard is doing the syncing too.

One detail that matters if you're on a Steam Deck as well as a desktop: Hoard tracks \`<AppID>/remote/\` inside \`userdata\`, not the folder above it, because the parent holds \`remotecache.vdf\` and per-machine achievement and playtime files. That's the distinction a hand-rolled sync usually gets wrong, and it's why those setups seem to conflict on every launch.

## When Steam Cloud is enough

Worth saying plainly: if every game you play is a Steam game with cloud support, you play on one PC, and you've never needed to undo a save, Steam Cloud already does the job and you don't need anything else. The case for adding Hoard is version history, games from outside Steam, and machines Steam Cloud doesn't reach.

## Without anyone's cloud

If the appeal is not depending on a platform at all, Hoard can be run entirely on your own hardware: \`hoard-server\` on a PC or a NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us and no relay** — nothing passes through our servers, because there is nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same version history. The only thing that changes is who owns the storage.

<!-- faq -->

## Frequently asked questions

### Does Hoard replace Steam Cloud?

It doesn't have to. Steam Cloud keeps your current save in sync for the games that support it; Hoard adds a version history and covers the games it doesn't. Running both is normal.

### Can Steam Cloud roll back to an older save?

No. Steam holds the current state of the files, not a history of them. Once a bad save has synced, that's what's in the cloud. A versioned tool is the only way to go back.

### Why don't all my Steam games sync?

Because it's the developer who enables it, per game and sometimes per platform. A game's store page lists Steam Cloud among its features when it's supported — and plenty of titles simply don't.

### Does Hoard work with non-Steam games?

Yes, that's most of the point. It locates saves through a community database covering 20,000+ titles, from any launcher, and you can point it at a folder by hand for anything unusual.

### Will running both cause conflicts?

No. Hoard captures a version after you stop playing, once the folder goes quiet, and never overwrites without capturing what it replaces first.

### Can I keep my saves off both clouds?

Yes. Self-host the server and your saves never leave hardware you own, with no account and no telemetry going anywhere.
`,fn=`---
title: "Alternativa a Steam Cloud: copia las partidas que Steam no guarda"
description: "Steam Cloud se salta muchos juegos y no guarda historial. Copia todos, de cualquier launcher, con versiones a las que volver. En la nube o en tu servidor."
order: 7
updated: 2026-09-01
---

Steam Cloud hace muy bien el trabajo concreto que hace, y la mayoría de la gente descubre sus límites justo el día que pierde algo. Esta guía explica dónde están esos límites y qué hacer con los juegos que se quedan fuera.

## Qué cubre realmente Steam Cloud

Steam Cloud sincroniza la carpeta de un juego cuando **el desarrollador lo configuró**, ya sea declarando qué ficheros sincronizar o llamando a la API de Steam desde dentro del juego. Ése es todo el modelo, y de ahí salen tres consecuencias:

- Sólo funciona con juegos comprados y lanzados desde Steam.
- Que funcione o no lo decide el desarrollador, juego por juego, y a veces por plataforma.
- Cada juego tiene su propio cupo de almacenamiento, fijado por ese desarrollador.

Cuando funciona es invisible y excelente: cierras el juego en un PC, lo abres en otro y tu progreso está ahí.

## Dónde te deja expuesto

- **Todo lo que no sea un juego de Steam.** GOG, Epic, itch, Battle.net, la app de Xbox, emuladores, cualquier cosa instalada a mano. Steam ni sabe que existen.
- **Juegos de Steam donde nunca se activó.** Bastantes títulos, sobre todo antiguos o pequeños, sencillamente no lo tienen. La ficha de la tienda lo dice, pero nadie lo mira antes de empezar una partida de 60 horas.
- **No hay marcha atrás.** Éste es el grande. Steam guarda el estado actual de tu partida, no su historial. Si el fichero se corrompe, si un mod se come tu mundo o si machacas una partida buena con una mala, la copia de la nube ya es la mala. Puedes ver los ficheros que Steam guarda de un juego, pero no hay una versión anterior a la que volver.
- **El diálogo de conflicto.** Cuando Steam cree que la partida local y la remota no cuadran, te pide que elijas con poco más que dos fechas delante. Si eliges mal, la otra copia desaparece.

## Qué añade Hoard

Hoard vigila la carpeta en la que escribe cada juego y captura una **versión nueva cada vez que terminas de jugar**:

- **Le da igual de dónde venga el juego.** Steam, GOG, Epic, itch, emuladores o una carpeta que le señales a mano.
- **Se conservan todas las versiones**, así que recuperarte de una partida corrupta o de una mala decisión son dos clics y no una partida perdida.
- **Sincroniza entre tus máquinas** igual, incluidas una Steam Deck y un sobremesa.
- **Nada se destruye en silencio.** La partida que se reemplaza se captura antes, así que hasta una restauración equivocada es reversible.

Las instantáneas se guardan por hash de contenido, así que diez versiones de una partida de 2 GB ocupan unos 2 GB y no 20, que es lo que hace práctico conservar el historial entero.

## Usar los dos a la vez

No se pelean, y no tienes que elegir. En un juego de Steam con soporte de nube, deja que Steam siga sincronizando lo que ya sincroniza; lo que aporta Hoard ahí es el historial, que es justo lo que Steam no guarda. Para todo lo demás, Hoard se encarga también de la sincronización.

Un detalle que importa si tienes Steam Deck además de sobremesa: Hoard rastrea \`<AppID>/remote/\` dentro de \`userdata\`, no la carpeta de encima, porque la padre guarda \`remotecache.vdf\` y ficheros de logros y tiempo jugado propios de cada máquina. Ésa es la distinción que suele fallar en una sincronización casera, y por eso esos montajes parecen entrar en conflicto en cada arranque.

## Cuándo basta con Steam Cloud

Conviene decirlo claro: si todos los juegos a los que juegas son de Steam y con soporte de nube, juegas en un solo PC y nunca has necesitado deshacer una partida, Steam Cloud ya hace el trabajo y no necesitas nada más. Lo que justifica añadir Hoard es el historial de versiones, los juegos de fuera de Steam y las máquinas a las que Steam Cloud no llega.

## Sin la nube de nadie

Si lo que te atrae es no depender de ninguna plataforma, Hoard se puede usar entero sobre tu propio hardware: \`hoard-server\` en un PC o en un NAS, y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

El mismo programa, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard sustituye a Steam Cloud?

No tiene por qué. Steam Cloud mantiene sincronizada tu partida actual en los juegos que lo soportan; Hoard añade el historial de versiones y cubre los juegos que no. Usar los dos es lo normal.

### ¿Steam Cloud puede volver a una partida anterior?

No. Steam guarda el estado actual de los ficheros, no su historial. Una vez que una partida mala se ha sincronizado, eso es lo que hay en la nube. Para volver atrás hace falta una herramienta con versiones.

### ¿Por qué no se sincronizan todos mis juegos de Steam?

Porque quien lo activa es el desarrollador, juego por juego y a veces por plataforma. La ficha del juego en la tienda incluye Steam Cloud entre sus características cuando está soportado, y muchos títulos sencillamente no lo están.

### ¿Hoard funciona con juegos que no son de Steam?

Sí, y es buena parte del sentido que tiene. Localiza las partidas con una base de datos comunitaria que cubre más de 20.000 títulos, de cualquier tienda, y para lo raro puedes señalarle la carpeta a mano.

### ¿Usar los dos provoca conflictos?

No. Hoard captura una versión cuando dejas de jugar y la carpeta se queda quieta, y nunca sobrescribe sin capturar antes lo que reemplaza.

### ¿Puedo mantener mis partidas fuera de las dos nubes?

Sí. Autoaloja el servidor y tus partidas no salen nunca de hardware tuyo, sin cuenta y sin telemetría hacia ningún sitio.
`,bn=`---
title: "Alternative à Steam Cloud : sauvegardez les parties que Steam ignore"
description: "Steam Cloud ignore beaucoup de jeux et ne garde aucun historique. Sauvegardez tous vos jeux, de tout launcher, avec des versions à restaurer."
order: 7
updated: 2026-09-01
---

Steam Cloud fait très bien le travail précis qu'il fait, et la plupart des gens en découvrent les limites le jour où ils perdent quelque chose. Ce guide explique où sont ces limites, et quoi faire des jeux qui restent en dehors.

## Ce que Steam Cloud couvre réellement

Steam Cloud synchronise le dossier d'un jeu quand **le développeur l'a configuré** : soit en déclarant les fichiers à synchroniser, soit en appelant l'API Steam depuis le jeu. C'est tout le modèle, et trois conséquences en découlent :

- Ça ne marche que pour des jeux achetés et lancés via Steam.
- Que ça marche ou non est la décision du développeur, jeu par jeu, parfois par plateforme.
- Chaque jeu a son propre quota de stockage, fixé par ce développeur.

Quand ça marche, c'est invisible et excellent : vous fermez le jeu sur un PC, vous l'ouvrez sur un autre, votre progression est là.

## Là où ça vous laisse exposé

- **Tout ce qui n'est pas un jeu Steam.** GOG, Epic, itch, Battle.net, l'application Xbox, les émulateurs, tout ce qui est installé à la main. Steam ignore leur existence.
- **Les jeux Steam où ça n'a jamais été activé.** Beaucoup de titres, surtout anciens ou modestes, ne l'ont tout simplement pas. La fiche boutique le dit, mais personne ne vérifie avant de lancer une partie de 60 heures.
- **Il n'y a pas de retour en arrière.** C'est le point majeur. Steam conserve l'état actuel de votre sauvegarde, pas son histoire. Fichier corrompu, mod qui dévore votre monde, bonne sauvegarde écrasée par une mauvaise : la copie du cloud est déjà la mauvaise. Vous pouvez consulter les fichiers que Steam détient pour un jeu, mais il n'y a aucune version antérieure à restaurer.
- **La boîte de dialogue de conflit.** Quand Steam estime que local et distant divergent, il vous demande de choisir avec guère plus que deux horodatages. Mauvais choix, l'autre copie a disparu.

## Ce que Hoard ajoute

Hoard surveille le dossier dans lequel le jeu écrit vraiment et capture **une nouvelle version chaque fois que vous arrêtez de jouer** :

- **La provenance du jeu lui est égale.** Steam, GOG, Epic, itch, émulateurs, ou un dossier que vous lui désignez.
- **Toutes les versions sont conservées** : se remettre d'une sauvegarde corrompue ou d'une mauvaise décision coûte deux clics, pas une partie entière.
- **Il synchronise entre vos machines**, Steam Deck et PC de bureau compris.
- **Rien n'est détruit en silence.** La sauvegarde remplacée est capturée d'abord : même une restauration malheureuse est réversible.

Les instantanés sont stockés par empreinte de contenu : dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20 — c'est ce qui rend viable de garder tout l'historique.

## Utiliser les deux ensemble

Ils ne se marchent pas dessus, et vous n'avez pas à choisir. Pour un jeu Steam qui gère le cloud, laissez Steam synchroniser ce qu'il synchronise déjà ; l'apport de Hoard est l'historique — précisément ce que Steam ne garde pas. Pour tout le reste, Hoard assure aussi la synchro.

Un détail qui compte si vous avez un Steam Deck en plus d'un fixe : Hoard suit \`<AppID>/remote/\` dans \`userdata\`, et non le dossier au-dessus, car le parent contient \`remotecache.vdf\` et des fichiers de succès et de temps de jeu propres à chaque machine. C'est la distinction qu'une synchro maison rate le plus souvent, et la raison pour laquelle ces montages semblent en conflit à chaque lancement.

## Quand Steam Cloud suffit

Disons-le franchement : si tous vos jeux sont des jeux Steam avec support cloud, que vous jouez sur un seul PC et que vous n'avez jamais eu besoin d'annuler une sauvegarde, Steam Cloud fait le travail et vous n'avez besoin de rien d'autre. Ce qui justifie d'ajouter Hoard, c'est l'historique de versions, les jeux hors Steam et les machines que Steam Cloud n'atteint pas.

## Sans le cloud de personne

Si l'attrait est de ne dépendre d'aucune plateforme, Hoard tourne entièrement sur votre matériel : \`hoard-server\` sur un PC ou un NAS, et vos sauvegardes vont de votre machine à votre disque. **Aucun compte chez nous, aucune télémétrie vers nous, aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

Le même programme, la même détection, le même historique. La seule chose qui change, c'est à qui appartient le stockage.

<!-- faq -->

## Questions fréquentes

### Hoard remplace-t-il Steam Cloud ?

Ce n'est pas obligatoire. Steam Cloud garde votre sauvegarde courante synchronisée pour les jeux compatibles ; Hoard ajoute l'historique de versions et couvre les jeux qui ne le sont pas. Faire tourner les deux est courant.

### Steam Cloud peut-il revenir à une sauvegarde plus ancienne ?

Non. Steam conserve l'état actuel des fichiers, pas leur histoire. Une fois qu'une mauvaise sauvegarde est synchronisée, c'est elle qui est dans le cloud. Revenir en arrière exige un outil qui versionne.

### Pourquoi tous mes jeux Steam ne se synchronisent-ils pas ?

Parce que c'est le développeur qui l'active, jeu par jeu et parfois par plateforme. La fiche du jeu mentionne Steam Cloud dans ses fonctionnalités quand c'est pris en charge — et beaucoup de titres ne le sont pas.

### Hoard fonctionne-t-il avec des jeux hors Steam ?

Oui, et c'est une bonne part de son intérêt. Il localise les sauvegardes via une base communautaire couvrant plus de 20 000 titres, toutes boutiques confondues, et vous pouvez lui désigner un dossier à la main pour les cas particuliers.

### Faire tourner les deux crée-t-il des conflits ?

Non. Hoard capture une version après que vous avez arrêté et que le dossier s'est calmé, et n'écrase jamais sans avoir d'abord capturé ce qu'il remplace.

### Puis-je garder mes sauvegardes hors des deux clouds ?

Oui. Auto-hébergez le serveur : vos sauvegardes ne quittent jamais du matériel qui vous appartient, sans compte et sans télémétrie vers qui que ce soit.
`,yn=`---
title: "Alternativa a Steam Cloud: salva i salvataggi che Steam non copre"
description: "Steam Cloud salta molti giochi e non tiene la cronologia. Fai il backup di ogni gioco, da qualsiasi launcher, con versioni da ripristinare. Cloud o self-host."
order: 7
updated: 2026-09-01
---

Steam Cloud fa molto bene il compito ristretto che ha, e quasi tutti ne scoprono i limiti proprio il giorno in cui perdono qualcosa. Questa guida spiega dove sono quei limiti e cosa fare con i giochi che restano fuori.

## Cosa copre davvero Steam Cloud

Steam Cloud sincronizza la cartella di un gioco quando **lo sviluppatore l'ha configurato**: dichiarando quali file sincronizzare, oppure chiamando l'API di Steam dall'interno del gioco. È tutto qui, e ne discendono tre cose:

- Funziona solo per giochi comprati e avviati tramite Steam.
- Che funzioni o no lo decide lo sviluppatore, gioco per gioco e a volte per piattaforma.
- Ogni gioco ha la sua quota di spazio, fissata da quello sviluppatore.

Quando funziona è invisibile ed eccellente: chiudi il gioco su un PC, lo apri su un altro, i progressi sono lì.

## Dove ti lascia scoperto

- **Tutto ciò che non è un gioco Steam.** GOG, Epic, itch, Battle.net, l'app Xbox, gli emulatori, qualsiasi cosa installata a mano. Steam non sa che esistono.
- **Giochi Steam dove non è mai stato attivato.** Parecchi titoli, soprattutto vecchi o piccoli, semplicemente non ce l'hanno. La pagina del negozio lo dice, ma nessuno la controlla prima di iniziare una partita da 60 ore.
- **Non si torna indietro.** Questo è il punto grosso. Steam conserva lo stato attuale del salvataggio, non la sua storia. Se il file si corrompe, se una mod ti mangia il mondo o se sovrascrivi un salvataggio buono con uno rotto, la copia nel cloud è già quella rotta. Puoi vedere i file che Steam tiene per un gioco, ma non c'è una versione precedente da ripristinare.
- **La finestra di conflitto.** Quando Steam ritiene che locale e remoto non coincidano, ti chiede di scegliere con poco più di due date davanti. Se sbagli, l'altra copia è persa.

## Cosa aggiunge Hoard

Hoard sorveglia la cartella in cui il gioco scrive davvero e cattura una **nuova versione ogni volta che smetti di giocare**:

- **Non gli importa da dove venga il gioco.** Steam, GOG, Epic, itch, emulatori o una cartella che gli indichi a mano.
- **Tutte le versioni vengono conservate**, quindi rimediare a un salvataggio corrotto o a una scelta sbagliata sono due clic e non una partita persa.
- **Sincronizza tra le tue macchine** allo stesso modo, Steam Deck e desktop inclusi.
- **Niente viene distrutto in silenzio.** Il salvataggio sostituito viene catturato prima, quindi anche un ripristino sbagliato è reversibile.

Gli snapshot sono archiviati per hash del contenuto, così dieci versioni di un salvataggio da 2 GB occupano circa 2 GB e non 20: è questo a rendere pratico conservare tutta la cronologia.

## Usarli insieme

Non litigano, e non devi scegliere. Per un gioco Steam con supporto cloud, lascia che Steam sincronizzi quello che già sincronizza; il contributo di Hoard lì è la cronologia, cioè proprio ciò che Steam non tiene. Per tutto il resto, alla sincronizzazione pensa Hoard.

Un dettaglio che conta se oltre al desktop hai una Steam Deck: Hoard traccia \`<AppID>/remote/\` dentro \`userdata\`, non la cartella superiore, perché quella contiene \`remotecache.vdf\` e file di obiettivi e tempo di gioco propri di ogni macchina. È la distinzione che una sincronizzazione artigianale sbaglia più spesso, ed è il motivo per cui quei setup sembrano andare in conflitto a ogni avvio.

## Quando Steam Cloud basta

Vale la pena dirlo chiaramente: se tutti i giochi a cui giochi sono giochi Steam con supporto cloud, giochi su un solo PC e non hai mai avuto bisogno di annullare un salvataggio, Steam Cloud fa già il suo e non ti serve altro. Ad aggiungere Hoard convincono la cronologia delle versioni, i giochi fuori da Steam e le macchine che Steam Cloud non raggiunge.

## Senza il cloud di nessuno

Se quello che ti attira è non dipendere da nessuna piattaforma, Hoard può girare interamente sul tuo hardware: \`hoard-server\` su un PC o su un NAS, e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia. L'unica cosa che cambia è di chi è lo spazio di archiviazione.

<!-- faq -->

## Domande frequenti

### Hoard sostituisce Steam Cloud?

Non deve per forza. Steam Cloud tiene sincronizzato il salvataggio attuale per i giochi supportati; Hoard aggiunge la cronologia e copre i giochi che non lo sono. Tenerli entrambi è normale.

### Steam Cloud può tornare a un salvataggio più vecchio?

No. Steam conserva lo stato attuale dei file, non la loro storia. Una volta che un salvataggio rotto è stato sincronizzato, è quello che sta nel cloud. Per tornare indietro serve uno strumento che versiona.

### Perché non tutti i miei giochi Steam si sincronizzano?

Perché è lo sviluppatore ad attivarlo, gioco per gioco e a volte per piattaforma. La pagina del negozio elenca Steam Cloud tra le caratteristiche quando è supportato, e molti titoli semplicemente non lo sono.

### Hoard funziona con giochi non Steam?

Sì, ed è buona parte del punto. Individua i salvataggi tramite un database comunitario che copre oltre 20.000 titoli, da qualsiasi store, e per i casi insoliti puoi indicargli la cartella a mano.

### Usarli entrambi crea conflitti?

No. Hoard cattura una versione dopo che hai smesso e la cartella si è calmata, e non sovrascrive mai senza aver prima catturato ciò che sostituisce.

### Posso tenere i salvataggi fuori da entrambi i cloud?

Sì. Ospita il server da solo: i salvataggi non lasciano mai hardware tuo, senza account e senza telemetria verso nessuno.
`,kn=`---
title: "Steam クラウドの代替：Steam が守らないセーブを守る"
description: "Steam Cloudは多くのゲームに非対応で履歴もありません。どのランチャーのゲームも、巻き戻せる履歴付きでバックアップ。クラウドでもセルフホストでも。"
order: 7
updated: 2026-09-01
---

Steam クラウドは、その限られた役割を本当にうまくこなします。そして多くの人は、何かを失った日に初めてその境界を知ります。このガイドでは、その境界がどこにあるのか、そして境界の外に残るゲームをどうするのかを説明します。

## Steam クラウドが実際にカバーする範囲

Steam クラウドは、**開発者が設定した場合に限り** ゲームのフォルダーを同期します。同期するファイルを宣言するか、ゲーム内から Steam の API を呼ぶかのどちらかです。仕組みはこれだけで、そこから 3 つのことが導かれます。

- Steam で購入し、Steam から起動したゲームでしか働きません。
- そもそも働くかどうかは開発者の判断で、ゲームごと、ときにはプラットフォームごとに異なります。
- 各ゲームには、その開発者が決めた保存容量の枠があります。

うまく働いているときは、目に見えないほど快適です。1 台目でゲームを閉じ、2 台目で開けば、進行はそこにあります。

## どこが穴になるのか

- **Steam のゲームでないものすべて。** GOG、Epic、itch、Battle.net、Xbox アプリ、エミュレーター、手動で入れたもの。Steam はその存在を知りません。
- **有効化されていない Steam のゲーム。** 特に古いものや小規模なものには、そもそも搭載されていない例が多くあります。ストアページには書いてありますが、60 時間の周回を始める前に確認する人はいません。
- **戻る手段がない。** これが最大の点です。Steam が保持しているのはセーブの現在の状態であって、その履歴ではありません。ファイルが壊れても、Mod がワールドを食べても、良いセーブを悪いセーブで上書きしても、クラウドにあるのはすでに悪いほうです。ゲームごとに Steam が保持しているファイルを見ることはできますが、復元できる過去の世代はありません。
- **競合のダイアログ。** ローカルとリモートが食い違うと Steam は選択を求めますが、判断材料は 2 つの日時程度です。選び間違えれば、もう片方は消えます。

## Hoard が足すもの

Hoard はゲームが実際に書き込むフォルダーを監視し、**プレイを終えるたびに新しい世代** を取り込みます。

- **ゲームの入手元を問いません。** Steam、GOG、Epic、itch、エミュレーター、手動で指定したフォルダー。
- **すべての世代が残る** ので、壊れたセーブや判断ミスからの復帰は、周回のやり直しではなく 2 クリックです。
- **マシン間の同期も同じ仕組み** で、Steam Deck とデスクトップも含みます。
- **黙って壊れるものはありません。** 置き換えられるセーブは先に取り込まれるため、復元を間違えても元に戻せます。

スナップショットは内容ハッシュで保存されるため、2 GB のセーブの 10 世代は約 20 GB ではなく約 2 GB です。履歴を丸ごと残しておけるのはこのためです。

## 両方を同時に使う

両者はぶつかりませんし、どちらかを選ぶ必要もありません。クラウド対応の Steam ゲームでは、Steam に今までどおり同期させてください。そこで Hoard が足すのは履歴、つまり Steam が持たないものです。それ以外のすべてでは、同期も Hoard が担います。

デスクトップに加えて Steam Deck を使うなら、重要な細部がひとつあります。Hoard は \`userdata\` の中の \`<AppID>/remote/\` を追跡し、その上のフォルダーは追跡しません。上のフォルダーには \`remotecache.vdf\` や、実績・プレイ時間といったマシンごとのファイルが入っているからです。自作の同期がいちばん間違えるのがこの区別で、そうした構成が起動のたびに競合しているように見える理由でもあります。

## Steam クラウドで足りる場合

はっきり書いておきます。遊ぶゲームがすべてクラウド対応の Steam タイトルで、PC は 1 台、セーブを巻き戻したいと思ったことがないのなら、Steam クラウドで用は足りており、ほかに何も要りません。Hoard を足す理由になるのは、世代履歴、Steam の外にあるゲーム、そして Steam クラウドが届かないマシンです。

## 誰のクラウドも使わない

どのプラットフォームにも依存したくないのであれば、Hoard は自分のハードウェアだけで動かせます。PC か NAS で \`hoard-server\` を動かせば、セーブは自分のマシンから自分のディスクへ移ります。**当方のアカウントも、当方へのテレメトリも、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

同じプログラム、同じ検出、同じ世代履歴。変わるのは保存先が誰のものかだけです。

<!-- faq -->

## よくある質問

### Hoard は Steam クラウドの置き換えですか？

置き換える必要はありません。Steam クラウドは対応ゲームの現在のセーブを同期し続け、Hoard はそこに世代履歴を足し、対応していないゲームを引き受けます。両方を使うのが普通です。

### Steam クラウドで古いセーブに戻せますか？

いいえ。Steam が保持するのはファイルの現在の状態であって、その履歴ではありません。壊れたセーブが同期されてしまえば、クラウドにあるのはそれです。戻るには世代を残すツールが必要です。

### Steam のゲームなのに同期されないのはなぜですか？

有効にするのが開発者だからです。ゲームごと、ときにはプラットフォームごとに決まります。対応している場合はストアページの機能欄に Steam クラウドが並びますが、載っていないタイトルも数多くあります。

### Steam 以外のゲームでも使えますか？

はい。むしろそこが要点のひとつです。2 万本以上を収録したコミュニティのデータベースからセーブの場所を割り出し、入手元は問いません。変わったものは手動でフォルダーを指定できます。

### 両方動かすと競合しませんか？

しません。Hoard はプレイ終了後、フォルダーが静かになってから世代を取り込み、置き換える前に必ず現物を取り込みます。

### セーブをどちらのクラウドにも置かずに済みますか？

はい。サーバーをセルフホストすれば、セーブが自分のハードウェアから出ることはありません。アカウントもなく、どこへもテレメトリを送りません。
`,Dn=`---
title: "Alternativa à Steam Cloud: guarda os saves que a Steam não guarda"
description: "O Steam Cloud deixa muitos jogos de fora e não guarda histórico. Faz backup de todos, de qualquer launcher, com versões para recuperar. Nuvem ou self-host."
order: 7
updated: 2026-09-01
---

A Steam Cloud faz muito bem o trabalho estreito que faz, e a maioria das pessoas só lhe descobre os limites no dia em que perde alguma coisa. Este guia explica onde estão esses limites e o que fazer com os jogos que ficam de fora.

## O que a Steam Cloud cobre mesmo

A Steam Cloud sincroniza a pasta de um jogo quando **o programador a configurou**: ou declarando que ficheiros sincronizar, ou chamando a API da Steam de dentro do jogo. É todo o modelo, e daí saem três consequências:

- Só funciona com jogos comprados e lançados pela Steam.
- Se funciona ou não é decisão do programador, jogo a jogo, e às vezes por plataforma.
- Cada jogo tem a sua própria quota de espaço, definida por esse programador.

Quando funciona é invisível e excelente: fechas o jogo num PC, abres noutro, e o progresso está lá.

## Onde te deixa exposto

- **Tudo o que não seja um jogo da Steam.** GOG, Epic, itch, Battle.net, a app da Xbox, emuladores, tudo o que instalaste à mão. A Steam nem sabe que existem.
- **Jogos da Steam onde nunca foi ativada.** Muitos títulos, sobretudo antigos ou pequenos, simplesmente não a têm. A página da loja di-lo, mas ninguém verifica antes de começar uma campanha de 60 horas.
- **Não há volta atrás.** É o ponto grande. A Steam guarda o estado atual do save, não o seu histórico. Se o ficheiro se corrompe, se uma mod te come o mundo, ou se escreves por cima de um save bom com um mau, a cópia na nuvem já é a má. Podes ver os ficheiros que a Steam guarda de um jogo, mas não há versão anterior para restaurar.
- **A janela de conflito.** Quando a Steam acha que o local e o remoto divergem, pede-te para escolher com pouco mais do que duas datas à frente. Escolhes mal e a outra cópia desapareceu.

## O que o Hoard acrescenta

O Hoard vigia a pasta onde o jogo realmente escreve e captura uma **versão nova sempre que acabas de jogar**:

- **Não lhe interessa de onde veio o jogo.** Steam, GOG, Epic, itch, emuladores ou uma pasta que lhe apontes à mão.
- **Todas as versões ficam guardadas**, por isso recuperar de um save corrompido ou de uma má decisão são dois cliques e não uma campanha perdida.
- **Sincroniza entre as tuas máquinas** da mesma forma, Steam Deck e desktop incluídos.
- **Nada é destruído em silêncio.** O save substituído é capturado primeiro, por isso até uma restauração errada é reversível.

Os snapshots são guardados por hash de conteúdo, por isso dez versões de um save de 2 GB ocupam cerca de 2 GB e não 20 — é isso que torna prático manter o histórico inteiro.

## Usar os dois ao mesmo tempo

Não se atropelam, e não tens de escolher. Num jogo da Steam com suporte de nuvem, deixa a Steam sincronizar o que já sincroniza; o que o Hoard acrescenta aí é o histórico, exatamente aquilo que a Steam não guarda. Para tudo o resto, é o Hoard que também trata da sincronização.

Um detalhe que conta se tens uma Steam Deck além do fixo: o Hoard segue \`<AppID>/remote/\` dentro de \`userdata\`, e não a pasta acima, porque a de cima guarda \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo próprios de cada máquina. É a distinção que uma sincronização caseira falha com mais frequência, e a razão pela qual essas montagens parecem entrar em conflito a cada arranque.

## Quando a Steam Cloud chega

Convém dizê-lo com clareza: se todos os jogos a que jogas são da Steam e com suporte de nuvem, jogas num só PC e nunca precisaste de desfazer um save, a Steam Cloud já faz o trabalho e não precisas de mais nada. O que justifica juntar o Hoard é o histórico de versões, os jogos de fora da Steam e as máquinas onde a Steam Cloud não chega.

## Sem a nuvem de ninguém

Se o que te atrai é não depender de plataforma nenhuma, o Hoard pode correr inteiramente no teu hardware: \`hoard-server\` num PC ou num NAS, e os teus saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico. A única coisa que muda é de quem é o armazenamento.

<!-- faq -->

## Perguntas frequentes

### O Hoard substitui a Steam Cloud?

Não tem de substituir. A Steam Cloud mantém o teu save atual sincronizado nos jogos que a suportam; o Hoard acrescenta o histórico de versões e cobre os jogos que não. Ter os dois é o normal.

### A Steam Cloud consegue voltar a um save mais antigo?

Não. A Steam guarda o estado atual dos ficheiros, não o histórico deles. Assim que um save mau sincroniza, é esse que está na nuvem. Para voltar atrás é preciso uma ferramenta com versões.

### Porque é que nem todos os meus jogos da Steam sincronizam?

Porque quem a ativa é o programador, jogo a jogo e às vezes por plataforma. A página do jogo na loja indica a Steam Cloud entre as funcionalidades quando é suportada — e muitos títulos simplesmente não são.

### O Hoard funciona com jogos que não são da Steam?

Sim, e é boa parte do sentido. Localiza os saves através de uma base de dados comunitária que cobre mais de 20.000 títulos, de qualquer loja, e para o que for invulgar podes apontar-lhe a pasta à mão.

### Ter os dois a correr provoca conflitos?

Não. O Hoard captura uma versão depois de parares e de a pasta ficar quieta, e nunca escreve por cima sem capturar primeiro aquilo que substitui.

### Posso manter os meus saves fora das duas nuvens?

Sim. Aloja o servidor tu mesmo e os teus saves nunca saem de hardware teu, sem conta e sem telemetria para lado nenhum.
`,qn=`---
title: "Steam 云存档的替代方案：备份 Steam 管不到的存档"
description: "Steam Cloud 不支持很多游戏，也不保留历史。备份任何启动器里的每款游戏，附可回滚的版本历史。云端或自托管均可。"
order: 7
updated: 2026-09-01
---

Steam 云存档把它那件狭窄的事做得相当好，而多数人是在丢东西的那一天才发现它的边界。本文说明这些边界在哪里，以及落在边界之外的游戏该怎么办。

## Steam 云存档实际覆盖什么

Steam 云存档只在**开发者做了配置**时才同步某款游戏的文件夹——要么声明需要同步哪些文件，要么在游戏内调用 Steam 的接口。整个模型就是这样，由此引出三件事：

- 它只对通过 Steam 购买并启动的游戏有效。
- 它到底能不能用，由开发者逐款决定，有时还分平台。
- 每款游戏有各自的存储配额，由该开发者设定。

它生效的时候是无形而出色的：在一台 PC 上关掉游戏，在另一台上打开，进度就在那里。

## 它把你晾在哪里

- **一切不是 Steam 的游戏。** GOG、Epic、itch、Battle.net、Xbox 应用、模拟器，以及你手动安装的一切。Steam 根本不知道它们存在。
- **从未启用它的 Steam 游戏。** 相当多的游戏，尤其是较老或较小的作品，压根就没有。商店页面会写明，但没人会在开一档 60 小时的存档前先去看。
- **没有回头路。** 这是最关键的一点。Steam 保存的是存档的当前状态，而不是它的历史。文件损坏、模组吞掉你的世界、或者用坏档覆盖了好档，云端那份已经是坏的了。你可以查看 Steam 为某款游戏保存的文件，但没有更早的版本可供还原。
- **冲突对话框。** 当 Steam 认为本地和云端不一致时，它让你选择，而你手上几乎只有两个时间戳。选错了，另一份就没了。

## Hoard 补上了什么

Hoard 盯着游戏真正写入的那个文件夹，并在**你每次玩完之后**抓取一个新版本：

- **它不在乎游戏从哪来。** Steam、GOG、Epic、itch、模拟器，或者你手动指给它的文件夹。
- **每个版本都会保留**，因此从损坏的存档或一次错误决定中脱身是两次点击，而不是一整轮重来。
- **同样负责在你的机器之间同步**，包括 Steam Deck 和台式机。
- **不会有东西悄悄消失。** 被替换掉的存档会先被抓取，所以连还原错了都能撤销。

快照按内容哈希存储，因此一个 2 GB 存档的十个版本大约占 2 GB，而不是 20 GB——正是这一点让保留完整历史变得现实。

## 两者同时使用

它们不会打架，你也不必二选一。对于支持云存档的 Steam 游戏，让 Steam 继续做它已经在做的同步；Hoard 在那里补上的是历史，也正是 Steam 不保留的东西。至于其他一切，同步也由 Hoard 负责。

如果你除了台式机还有 Steam Deck，有个细节很重要：Hoard 追踪的是 \`userdata\` 里的 \`<AppID>/remote/\`，而不是它上一层的文件夹，因为上一层放着 \`remotecache.vdf\` 以及各机器各自的成就和游戏时长文件。手工搭建的同步最常弄错的就是这个区别，这也是那类方案看起来每次启动都在冲突的原因。

## 什么时候 Steam 云存档就够了

不妨直说：如果你玩的每款游戏都是支持云存档的 Steam 游戏，只在一台 PC 上玩，也从没需要撤销过某个存档，那么 Steam 云存档已经把事情办了，你不需要别的。值得加上 Hoard 的理由是版本历史、Steam 之外的游戏，以及 Steam 云存档够不到的机器。

## 不用任何人的云

如果你在意的是不依赖任何平台，Hoard 可以完全跑在你自己的硬件上：在 PC 或 NAS 上运行 \`hoard-server\`，你的存档就从你的机器走到你的磁盘。**没有我们这边的账号，没有发往我们的遥测，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。参见[如何自托管 Hoard](/guides/self-host-hoard)。

同一个程序，同样的检测，同样的版本历史。唯一变化的是存储归谁所有。

<!-- faq -->

## 常见问题

### Hoard 是要取代 Steam 云存档吗？

不必如此。Steam 云存档为支持它的游戏同步当前存档；Hoard 补上版本历史，并覆盖那些不支持的游戏。两者同时使用很常见。

### Steam 云存档能回退到更早的存档吗？

不能。Steam 保存的是文件的当前状态，不是它们的历史。一旦坏档同步上去，云端就是那一份。要回退，只能靠会做版本管理的工具。

### 为什么我的 Steam 游戏不是每款都同步？

因为启用它的是开发者，逐款决定，有时还分平台。支持时，游戏商店页面会把 Steam 云存档列在功能里——而很多游戏根本就没有。

### Hoard 支持非 Steam 的游戏吗？

支持，这正是它的意义所在。它通过覆盖两万余款游戏的社区数据库定位存档，不限平台；遇到特别的情况，你也可以手动指定文件夹。

### 两个一起用会冲突吗？

不会。Hoard 会在你停止游玩、文件夹安静之后才抓取版本，并且在覆盖之前一定会先把被替换的内容抓取下来。

### 我能让存档不进这两朵云吗？

可以。自托管服务器，你的存档就永远不会离开属于你的硬件，没有账号，也不向任何地方发送遥测。
`,Pn=`---
title: "So synchronisierst du Spielstände über mehrere PCs"
description: "Spiele auf Desktop, Laptop und Steam Deck ohne Fortschrittsverlust: Spielstände automatisch zwischen PCs synchronisieren, mit Versionsverlauf."
order: 2
updated: 2026-10-01
---

Wenn du an mehr als einem Computer spielst — ein Desktop zu Hause und ein Laptop unterwegs — hält Hoard deine Stände synchron, damit du immer dort weitermachst, wo du aufgehört hast.

## So funktioniert die Synchronisierung

Hoard sichert jeden Stand in deine Cloud und lädt die neueste Version auf deinen anderen Geräten herunter. Wenn du auf einem PC fertig bist, wartet der neueste Stand auf dem nächsten.

## Synchronisierung einrichten

1. Installiere **Hoard** auf jedem PC, auf dem du spielst (Windows, macOS oder Linux).
2. Melde dich mit **demselben Konto** auf jedem Gerät an oder verbinde sie mit demselben selbst gehosteten Server.
3. Füge auf jedem PC dieselben Spiele zur **Bibliothek** hinzu. Hoard ordnet sie nach Spiel zu, sodass ein auf einem Gerät gesicherter Stand auf den anderen erscheint.
4. Lass den **Automatikmodus** an. Hoard lädt nach dem Spielen hoch und vor dem Start die neueste Version herunter.

## Wechsel von Ludusavi?

Ludusavi ist ein großartiges Open-Source-Tool, um Stände lokal zu sichern und wiederherzustellen, und es kann diese Backups in eine selbst konfigurierte Cloud mit Rclone übertragen. Aber die Synchronisierung über Geräte hinweg richtest du manuell ein: Backup planen, Remote einrichten, dann auf dem anderen PC wiederherstellen, bevor du spielst.

Hoard macht daraus verwaltete Synchronisierung. Es nutzt dieselben Community-Daten für Speicherorte wie Ludusavi, um deine Stände zu finden, lädt dann nach jeder Sitzung hoch und vor der nächsten die neueste Version herunter — auf jedem PC deines Kontos, mit versionierter Historie in der Cloud. Keine Rclone-Remotes, keine Skripte. Und wie Ludusavi ist Hoard Open Source und selbst hostbar. Siehe den vollständigen [Ludusavi-Alternative-Vergleich](/guides/ludusavi-alternative).

## Konflikte vermeiden

Hoard ist konfliktbewusst: Es vergleicht Änderungszeiten und behält eine lokale Kopie jedes ersetzten Stands, sodass eine Synchronisierung nie stillschweigend Fortschritt zerstört. Läuft ein Spiel noch oder wurde ein Stand in den letzten Minuten berührt, wartet Hoard.

## Steam Deck und Desktop

Das häufigste Zwei-Geräte-Setup ist auch das, was von Hand gebaut am öftesten kaputtgeht, und fast immer aus demselben Grund.

Unter Windows liegt der Spielstand vielleicht in \`Dokumente\\My Games\\…\` oder in Steams \`userdata\`. Auf einem Steam Deck läuft dasselbe Windows-Spiel über Proton, sein Stand liegt also in einem Kompatibilitäts-Prefix: \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Zwei sehr verschiedene Pfade, ein Spiel, ein Spielfortschritt. Hoard liest die Proton-Prefixes ebenso wie die nativen Orte und ordnet Gefundenes dem Spiel zu, sodass Deck-Stand und Desktop-Stand zwei Versionen einer Historie werden statt zweier zusammenhangloser Ordner.

Das Detail, an dem alles hängt: Bei Steam-Spielen verfolgt Hoard \`<AppID>/remote/\` innerhalb von \`userdata\`, **nicht** den Ordner darüber. Der übergeordnete Ordner enthält auch \`remotecache.vdf\` sowie gerätebezogene Dateien für Erfolge und Spielzeit, die sich zwischen Deck und Desktop unterscheiden sollen. Synchronisierst du den übergeordneten Ordner, sieht jeder Start nach einem Konflikt aus, obwohl sich kein Stand bewegt hat. Genau dieser eine Fehler lässt die meisten selbstgebauten Deck-PC-Setups defekt wirken.

## Spiele, die Steam Cloud nicht abdeckt

Würden alle deine Spiele Steam Cloud unterstützen, bräuchtest du nichts davon. In der Praxis:

- **Spiele von überall außer Steam.** GOG, Epic, itch, Battle.net, die Xbox-App und alles von Hand Installierte.
- **Steam-Spiele, bei denen die Entwickler es nie aktiviert haben**, oder nur für eine Plattform.
- **Emulatoren.** RetroArch, Dolphin, PCSX2, RPCS3 und der Rest speichern, wo sie wollen, und Steam weiß nichts davon.
- **Spiele, die außerhalb des von Steam beobachteten Ordners schreiben**, und das sind mehr, als man denkt.

Hoard ist egal, wer ein Spiel veröffentlicht hat oder woher es kommt. Es verfolgt den Ordner, der sich beim Spielen ändert.

## Wenn zwei PCs denselben Stand ändern

Du spielst am Laptop, ohne den Desktop zu Ende synchronisieren zu lassen, und hast das klassische Problem: zwei Stände, beide neuer als die letzte gemeinsame Version.

Hoard überschreibt nie blind. Es vergleicht Änderungszeiten, behält eine lokale Kopie von allem, was es ersetzt, und wartet, solange ein Spiel läuft oder der Stand in den letzten Minuten angefasst wurde — eine Datei, die gerade geschrieben wird, will man nicht halb hochladen. Alle früheren Versionen bleiben in der Cloud-Historie, die falsche Wahl kostet dich also zwei Klicks statt eines Wochenendes.

Die ehrliche Grenze: **Hoard führt zwei auseinandergelaufene Stände nicht zusammen.** Das kann kein Werkzeug — eine Speicherdatei ist undurchsichtig, und es gibt keinen richtigen Weg, zwei verschiedene Spielnachmittage zu vermischen. Was du stattdessen bekommst: jede Version, auf jedem Gerät, und die Wahl.

## Werden Einstellungen zwischen PCs synchronisiert?

Spielstände ja. Einstellungen standardmäßig nicht, und das ist Absicht. Das Steam Deck zeigt, warum: Es läuft mit 1280×800 auf einer Handheld-GPU, dein Desktop vielleicht mit 4K auf etwas viel Größerem. Kopierst du die \`graphics.ini\` des Desktops auf das Deck, startet das Spiel in einer Auflösung, die der Bildschirm nicht darstellen kann, mit Einstellungen, die die Hardware nicht schafft.

Deshalb sortiert Hoard, was es in einem Spielstand-Ordner findet:

- **Spielstände** werden in jeder Session zwischen den Rechnern synchronisiert.
- **Einstellungsdateien** (\`graphics.ini\`, \`settings.cfg\` und Ähnliches) landen in jedem Backup und gehen nie verloren, aber eine Wiederherstellung schreibt sie nicht über die Dateien eines anderen Rechners. Sagt der Eintrag des Spiels in der Spielstand-Datenbank, dass eine \`.ini\` *der* Spielstand ist, wird sie als Spielstand behandelt.
- **Ballast** wie Absturzberichte, temporäre Dateien, Engine-Logs wie \`Player.log\` und Steams eigene Buchhaltung wird gar nicht gesichert, damit das bloße Öffnen eines Spiels keine neue Version erzeugt.

Wenn die Einstellungen doch mitreisen sollen, setz beim Wiederherstellen einer Version aus dem Verlauf den Haken bei **Auch Einstellungsdateien wiederherstellen**, oder stell das Spiel auf **Einstellungen mitwiederherstellen**, damit jede Wiederherstellung sie mitbringt, automatische eingeschlossen. Praktisch, wenn beide PCs denselben Bildschirm haben und du das Spiel einmal eingestellt hast.

## Synchronisieren ohne unsere Server

Das gehört ausdrücklich gesagt, weil die meisten Vergleiche genau hier danebenliegen. Es gibt zwei Betriebsarten:

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Stände liegen auf unseren Servern in der EU.
- **Selbsthosten gehört vollständig dir.** Du betreibst \`hoard-server\` auf deinem eigenen PC oder NAS, und deine Geräte synchronisieren darüber. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört.

## Tipp

Gib jedem Gerät einen Moment, um die Synchronisierung abzuschließen, bevor du ein Spiel startest — das Dashboard zeigt den Live-Status, damit du weißt, dass der neueste Stand bereit ist.

<!-- faq -->

## Häufige Fragen

### Wie viele PCs kann ich synchronisieren?

Drei im kostenlosen Tarif, unbegrenzt mit Pro und unbegrenzt beim Selbsthosten — dein Server, deine Regeln.

### Müssen beide Geräte gleichzeitig online sein?

Nein. Dein Stand geht nach dem Spielen zum Server und kommt herunter, wenn das andere Gerät danach fragt. Der zweite PC kann also eine Woche ausgeschaltet sein und bekommt beim Einschalten trotzdem die neueste Version.

### Was, wenn ich offline spiele?

Kein Problem. Der Snapshot entsteht lokal, wenn du aufhörst zu spielen, und wird von selbst hochgeladen, sobald die Maschine wieder Verbindung hat.

### Werden auch Mods und Einstellungen synchronisiert?

Spielstände ja. Dateien, die zu einem bestimmten Rechner gehören — Konfiguration, Logs und Ähnliches — werden hochgeladen, damit sie im Backup sind, aber nicht über die Kopie eines anderen PCs geschrieben: eine Grafikeinstellung, die zu deinem Desktop passt, ist selten die, die dein Laptop will.

### Sendet Selbsthosten irgendetwas an Hoard?

Nein. Im selbst gehosteten Betrieb gibt es kein Konto bei uns und keine Telemetrie zu uns: deine Stände, deine Nutzer und deine Logs liegen auf deinem eigenen Server und berühren unseren nie.

### Kann ich meine Einstellungen trotzdem auf den anderen PC übernehmen?

Ja. Setz beim Wiederherstellen einer Version den Haken bei **Auch Einstellungsdateien wiederherstellen**, oder stell das Spiel auf **Einstellungen mitwiederherstellen**, damit sie bei jeder Wiederherstellung mitkommen. Auf der Kommandozeile macht \`hoard restore --allow-ini\` dasselbe.
`,Cn=`---
title: "How to sync game saves across multiple PCs"
description: "Play on your desktop, laptop and Steam Deck without losing progress: sync game saves between PCs automatically, with version history. Step by step."
order: 2
updated: 2026-10-01
related: steam-cloud-alternative, back-up-emulator-saves, self-host-hoard
---

If you play on more than one computer — a desktop at home and a laptop on the go — Hoard keeps your saves in sync so you always pick up where you left off.

## How sync works

Hoard backs up each save to your cloud and pulls the latest version down on your other machines. When you finish playing on one PC, the newest save is waiting on the next one.

## Set up sync

1. Install **Hoard** on every PC you play on (Windows, macOS or Linux).
2. Sign in with the **same account** on each machine, or connect them to the same self-hosted server.
3. Add the same games to your **Library** on each PC. Hoard matches them by game, so a save backed up on one shows up on the others.
4. Keep **automatic mode** on. Hoard uploads after you play and downloads the latest before you start.

## Coming from Ludusavi?

Ludusavi is a great open-source tool for backing up and restoring saves locally, and it can push those backups to a cloud you configure yourself with Rclone. But syncing across devices is something you wire up manually: schedule the backup, set up the remote, then restore on the other PC before you play.

Hoard turns that into managed sync. It uses the same community save-location data as Ludusavi to find your saves, then uploads after each session and downloads the latest before the next one — across every PC on your account, with versioned history in the cloud. No Rclone remotes, no scripts. And like Ludusavi, Hoard is open source and can be self-hosted. See the full [Ludusavi alternative comparison](/guides/ludusavi-alternative).

## Avoiding conflicts

Hoard is conflict-aware: it compares modification times and keeps a local copy of any replaced save, so a sync never silently destroys progress. If a game is still running or a save was touched in the last few minutes, Hoard waits.

## Steam Deck and desktop

The most common two-machine setup is also the one that breaks most often when it's wired by hand, and nearly always for the same reason.

On Windows, a game's save might sit in \`Documents\\My Games\\…\` or inside Steam's \`userdata\`. On a Steam Deck, that same Windows game runs through Proton, so its save lives inside a compatibility prefix: \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Two very different paths, one game, one run of progress. Hoard reads the Proton prefixes as well as the native locations and matches what it finds by game, so the Deck save and the desktop save become two versions of one history instead of two unrelated folders.

The detail that decides whether any of this works: for Steam games Hoard tracks \`<AppID>/remote/\` inside \`userdata\`, **not** the folder above it. The parent also holds \`remotecache.vdf\` and per-machine achievement and playtime files, which are supposed to differ between your Deck and your desktop. Sync the parent and every launch looks like a conflict even though no save actually moved. That single mistake is what makes most hand-rolled Deck ↔ PC setups feel broken.

## Games Steam Cloud doesn't cover

If every game you played supported Steam Cloud, you wouldn't need any of this. In practice:

- **Games from anywhere but Steam.** GOG, Epic, itch, Battle.net, the Xbox app, and anything you installed by hand.
- **Steam games where the developer never turned it on**, or turned it on for one platform only.
- **Emulators.** RetroArch, Dolphin, PCSX2, RPCS3 and the rest save where they like, and Steam knows nothing about it.
- **Games that write outside the folder Steam watches**, which is more of them than you'd expect.

Hoard doesn't care who published a game or where it came from. It tracks the folder that changes when you play.

## When two PCs edit the same save

Play on the laptop without letting the desktop finish syncing and you get the classic problem: two saves, both newer than the last common version.

Hoard never overwrites blind. It compares modification times, keeps a local copy of whatever it replaces, and holds off while a game is running or the save was touched in the last few minutes — a save file being written is not a save you want to upload halfway. Every earlier version stays in the cloud history, so picking the wrong one costs you two clicks, not a weekend.

The honest limit: **Hoard does not merge two divergent saves.** No tool can — a save file is opaque, and there is no correct way to blend two different afternoons of play. What you get instead is every version, on every machine, and the ability to choose.

## Do settings sync across PCs?

Saves do. Settings, by default, don't, and that's on purpose. The Steam Deck shows why: it runs at 1280×800 on a handheld GPU, while your desktop may run at 4K on something far bigger. Copy the desktop's \`graphics.ini\` onto the Deck and the game starts at a resolution the screen can't show, with settings the hardware can't hold.

So Hoard sorts what it finds in a save folder:

- **Saves** sync between machines, every session.
- **Settings files** (\`graphics.ini\`, \`settings.cfg\` and the like) are kept in every backup, so they're never lost, but a restore doesn't write them over another machine's copy. If the game's entry in the save database says a \`.ini\` *is* the save, it's treated as a save.
- **Clutter** such as crash dumps, temporary files, engine logs like \`Player.log\` and Steam's own bookkeeping isn't backed up at all, so simply opening a game doesn't create a new version.

When you do want the settings to travel, tick **Also restore settings files** when you restore a version from the history, or set the game to **Also restore settings** so every restore brings them, automatic ones included. Useful when both PCs share the same screen and you've tuned the game once.

## Syncing without our servers

Worth being explicit, because it's the part most comparisons get wrong. There are two ways to run this:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **Self-hosting is entirely yours.** You run \`hoard-server\` on your own PC or NAS and your machines sync through it. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same version history. The only thing that changes is who owns the storage.

## Tip

Give each machine a moment to finish syncing before you launch a game — the dashboard shows live status, so you know the latest save is in place.

<!-- faq -->

## Frequently asked questions

### How many PCs can I sync?

Three on the free tier, unlimited on Pro, and unlimited when you self-host — your server, your rules.

### Do both machines have to be online at the same time?

No. Your save goes up to the server when you finish playing and comes down when the other machine asks for it, so the second PC can be switched off for a week and still get the latest version when it wakes up.

### What if I play offline?

Fine. The snapshot is taken locally when you stop playing, and it uploads on its own once the machine has a connection again.

### Does it sync my mods and settings too?

Saves, yes. Files that belong to one machine — configuration, logs, and similar — are uploaded so they're in the backup, but are not written back over another PC's copy, because a graphics setting that suits your desktop is rarely the one your laptop wants.

### Does self-hosting send anything to Hoard?

No. In self-hosted mode there is no account with us and no telemetry to us: your saves, your users and your logs live on your own server and never touch ours.

### Can I copy my settings to the other PC anyway?

Yes. Tick **Also restore settings files** when restoring a version, or set the game to **Also restore settings** to bring them on every restore. On the command line, \`hoard restore --allow-ini\` does the same.
`,wn=`---
title: "Cómo sincronizar partidas guardadas entre varios PC"
description: "Juega en tu sobremesa, tu portátil y tu Steam Deck sin perder progreso: sincroniza tus partidas entre PC automáticamente, con historial. Paso a paso."
order: 2
updated: 2026-10-01
---

Si juegas en más de un ordenador —un sobremesa en casa y un portátil de viaje— Hoard mantiene tus partidas sincronizadas para que siempre retomes donde lo dejaste.

## Cómo funciona la sincronización

Hoard sube cada partida a tu nube y descarga la última versión en tus otros equipos. Cuando terminas de jugar en un PC, la partida más reciente te espera en el siguiente.

## Configura la sincronización

1. Instala **Hoard** en cada PC en el que juegues (Windows, macOS o Linux).
2. Inicia sesión con la **misma cuenta** en cada equipo, o conéctalos al mismo servidor autoalojado.
3. Añade los mismos juegos a tu **Biblioteca** en cada PC. Hoard los empareja por juego, así que una partida guardada en uno aparece en los demás.
4. Mantén el **modo automático** activado. Hoard sube cuando terminas de jugar y descarga la última versión antes de empezar.

## ¿Vienes de Ludusavi?

Ludusavi es una gran herramienta open source para hacer copias y restaurar partidas en local, y puede subir esas copias a una nube que configures tú mismo con Rclone. Pero sincronizar entre dispositivos es algo que montas a mano: programas la copia, configuras el remoto y luego restauras en el otro PC antes de jugar.

Hoard convierte eso en sincronización gestionada. Usa los mismos datos comunitarios de ubicación de partidas que Ludusavi para encontrar tus saves, y luego sube tras cada sesión y descarga la última versión antes de la siguiente, en todos los PC de tu cuenta y con historial versionado en la nube. Sin remotos de Rclone, sin scripts. Y, como Ludusavi, Hoard es open source y se puede autoalojar. Mira la [comparativa completa con Ludusavi](/guides/ludusavi-alternative).

## Evitar conflictos

Hoard tiene en cuenta los conflictos: compara las fechas de modificación y guarda una copia local de cualquier partida que reemplaza, así que una sincronización nunca destruye progreso en silencio. Si un juego sigue abierto o la partida se tocó hace pocos minutos, Hoard espera.

## Steam Deck y sobremesa

El montaje de dos máquinas más habitual es también el que más se rompe cuando se monta a mano, y casi siempre por el mismo motivo.

En Windows, la partida de un juego puede estar en \`Documentos\\My Games\\…\` o dentro del \`userdata\` de Steam. En una Steam Deck, ese mismo juego de Windows corre bajo Proton, así que su partida vive dentro de un prefijo de compatibilidad: \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Dos rutas muy distintas, un solo juego, un solo progreso. Hoard lee los prefijos de Proton además de las ubicaciones nativas y empareja lo que encuentra por juego, así que la partida de la Deck y la del sobremesa pasan a ser dos versiones de un mismo historial en vez de dos carpetas sin relación.

El detalle que decide si esto funciona: en los juegos de Steam, Hoard rastrea \`<AppID>/remote/\` dentro de \`userdata\`, **no** la carpeta de encima. La carpeta padre guarda además \`remotecache.vdf\` y ficheros de logros y de tiempo jugado propios de cada máquina, que deben ser distintos entre tu Deck y tu sobremesa. Si sincronizas la padre, cada arranque parece un conflicto aunque no se haya movido ninguna partida. Ese único error es lo que hace que la mayoría de los montajes caseros entre Deck y PC parezcan estropeados.

## Juegos que Steam Cloud no cubre

Si todos los juegos a los que juegas soportaran Steam Cloud, no necesitarías nada de esto. En la práctica:

- **Juegos de cualquier sitio que no sea Steam.** GOG, Epic, itch, Battle.net, la app de Xbox y todo lo que hayas instalado a mano.
- **Juegos de Steam en los que el desarrollador nunca lo activó**, o lo activó sólo para una plataforma.
- **Emuladores.** RetroArch, Dolphin, PCSX2, RPCS3 y compañía guardan donde les parece, y Steam no sabe nada de eso.
- **Juegos que escriben fuera de la carpeta que vigila Steam**, que son más de los que imaginas.

A Hoard le da igual quién publicara el juego o de dónde venga: rastrea la carpeta que cambia cuando juegas.

## Cuando dos PC tocan la misma partida

Juegas en el portátil sin dejar que el sobremesa termine de sincronizar y tienes el problema clásico: dos partidas, las dos más nuevas que la última versión común.

Hoard nunca sobrescribe a ciegas. Compara fechas de modificación, guarda una copia local de lo que reemplaza, y espera mientras haya un juego abierto o la partida se haya tocado en los últimos minutos: un fichero que se está escribiendo no es un fichero que quieras subir a medias. Todas las versiones anteriores siguen en el historial de la nube, así que equivocarte de versión cuesta dos clics y no un fin de semana.

El límite honesto: **Hoard no fusiona dos partidas divergentes.** Ninguna herramienta puede — un fichero de partida es opaco, y no existe una forma correcta de mezclar dos tardes distintas de juego. Lo que te da a cambio es todas las versiones, en todas las máquinas, y la posibilidad de elegir.

## ¿Se sincronizan los ajustes entre PC?

Las partidas sí. Los ajustes, por defecto, no, y es a propósito. La Steam Deck lo deja claro: funciona a 1280×800 con una GPU de portátil, mientras que tu sobremesa quizá vaya a 4K con algo mucho más grande. Copia el \`graphics.ini\` del sobremesa a la Deck y el juego arranca con una resolución que la pantalla no puede mostrar y unos ajustes que el hardware no aguanta.

Por eso Hoard clasifica lo que encuentra en una carpeta de partidas:

- **Las partidas** se sincronizan entre máquinas, en cada sesión.
- **Los ficheros de ajustes** (\`graphics.ini\`, \`settings.cfg\` y similares) se guardan en cada copia, así que nunca se pierden, pero una restauración no los escribe encima de los de otra máquina. Si la entrada del juego en la base de datos de partidas dice que un \`.ini\` *es* la partida, se trata como partida.
- **La basura**, como volcados de errores, ficheros temporales, registros del motor tipo \`Player.log\` y la contabilidad propia de Steam, no se copia, para que abrir un juego sin más no cree una versión nueva.

Cuando sí quieras que los ajustes viajen, marca **Restaurar también los ficheros de ajustes** al restaurar una versión desde el historial, o configura el juego con **Restaurar también los ajustes** para que cada restauración los traiga, también las automáticas. Útil cuando los dos PC tienen la misma pantalla y ya dejaste el juego afinado una vez.

## Sincronizar sin pasar por nuestros servidores

Conviene decirlo explícitamente, porque es la parte que casi todas las comparativas se equivocan. Hay dos formas de usar esto:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Autoalojarse es tuyo por completo.** Levantas \`hoard-server\` en tu PC o en tu NAS y tus máquinas sincronizan a través de él. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

El mismo programa, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento.

## Consejo

Deja que cada equipo termine de sincronizar antes de abrir un juego: el panel muestra el estado en vivo, así sabes que la última partida ya está en su sitio.

<!-- faq -->

## Preguntas frecuentes

### ¿Cuántos PC puedo sincronizar?

Tres en el plan gratuito, ilimitados en Pro, e ilimitados si te autoalojas: tu servidor, tus reglas.

### ¿Tienen que estar las dos máquinas encendidas a la vez?

No. Tu partida sube al servidor cuando terminas de jugar y baja cuando la otra máquina la pide, así que el segundo PC puede estar apagado una semana y aun así recibir la última versión al encenderse.

### ¿Y si juego sin conexión?

Sin problema. La instantánea se toma en local al dejar de jugar, y se sube sola en cuanto la máquina vuelve a tener conexión.

### ¿Sincroniza también mods y ajustes?

Las partidas, sí. Los ficheros que son de una máquina concreta — configuración, registros y similares — se suben para que estén en la copia, pero no se escriben encima de la copia de otro PC, porque un ajuste gráfico que le va bien a tu sobremesa rara vez es el que quiere tu portátil.

### ¿Autoalojarse envía algo a Hoard?

No. En modo autoalojado no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus registros viven en tu propio servidor y nunca tocan el nuestro.

### ¿Puedo copiar mis ajustes al otro PC de todas formas?

Sí. Marca **Restaurar también los ficheros de ajustes** al restaurar una versión, o configura el juego con **Restaurar también los ajustes** para traerlos en cada restauración. Desde la línea de comandos, \`hoard restore --allow-ini\` hace lo mismo.
`,zn=`---
title: "Comment synchroniser vos parties entre plusieurs PC"
description: "Jouez sur votre fixe, votre portable et votre Steam Deck sans perdre votre progression : synchro automatique des parties entre PC, avec historique."
order: 2
updated: 2026-10-01
---

Si vous jouez sur plus d'un ordinateur — un fixe à la maison et un portable en déplacement — Hoard garde vos sauvegardes synchronisées pour que vous repreniez toujours là où vous en étiez.

## Comment fonctionne la synchronisation

Hoard sauvegarde chaque partie vers votre cloud et récupère la dernière version sur vos autres machines. Quand vous finissez de jouer sur un PC, la sauvegarde la plus récente vous attend sur le suivant.

## Configurer la synchronisation

1. Installez **Hoard** sur chaque PC où vous jouez (Windows, macOS ou Linux).
2. Connectez-vous avec le **même compte** sur chaque machine, ou reliez-les au même serveur auto-hébergé.
3. Ajoutez les mêmes jeux à votre **Bibliothèque** sur chaque PC. Hoard les associe par jeu, donc une sauvegarde faite sur l'un apparaît sur les autres.
4. Gardez le **mode automatique** activé. Hoard envoie après que vous jouez et télécharge la dernière version avant que vous commenciez.

## Vous venez de Ludusavi ?

Ludusavi est un excellent outil open source pour sauvegarder et restaurer des parties en local, et il peut envoyer ces sauvegardes vers un cloud que vous configurez vous-même avec Rclone. Mais la synchro entre appareils, vous la montez à la main : planifier la sauvegarde, configurer le distant, puis restaurer sur l'autre PC avant de jouer.

Hoard transforme cela en synchro gérée. Il utilise les mêmes données communautaires d'emplacements que Ludusavi pour trouver vos sauvegardes, puis envoie après chaque session et télécharge la dernière version avant la suivante — sur chaque PC de votre compte, avec un historique versionné dans le cloud. Pas de distants Rclone, pas de scripts. Et comme Ludusavi, Hoard est open source et peut être auto-hébergé. Voir la [comparaison complète avec Ludusavi](/guides/ludusavi-alternative).

## Éviter les conflits

Hoard gère les conflits : il compare les dates de modification et conserve une copie locale de toute sauvegarde remplacée, donc une synchro ne détruit jamais la progression en silence. Si un jeu tourne encore ou qu'une sauvegarde a été modifiée il y a quelques minutes, Hoard attend.

## Steam Deck et PC de bureau

Le montage à deux machines le plus courant est aussi celui qui casse le plus souvent quand on le fait à la main, et presque toujours pour la même raison.

Sous Windows, la sauvegarde d'un jeu peut se trouver dans \`Documents\\My Games\\…\` ou dans le \`userdata\` de Steam. Sur un Steam Deck, ce même jeu Windows tourne via Proton : sa sauvegarde vit donc dans un préfixe de compatibilité, \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Deux chemins très différents, un seul jeu, une seule progression. Hoard lit les préfixes Proton comme les emplacements natifs et rapproche ce qu'il trouve par jeu : la sauvegarde du Deck et celle du bureau deviennent deux versions d'un même historique au lieu de deux dossiers sans rapport.

Le détail dont tout dépend : pour les jeux Steam, Hoard suit \`<AppID>/remote/\` dans \`userdata\`, et **non** le dossier au-dessus. Le dossier parent contient aussi \`remotecache.vdf\` ainsi que des fichiers de succès et de temps de jeu propres à chaque machine, qui doivent différer entre votre Deck et votre bureau. Synchronisez le parent et chaque lancement ressemble à un conflit alors qu'aucune sauvegarde n'a bougé. Cette seule erreur suffit à faire paraître cassés la plupart des montages maison Deck ↔ PC.

## Les jeux que Steam Cloud ne couvre pas

Si tous vos jeux géraient Steam Cloud, vous n'auriez besoin de rien de tout cela. En pratique :

- **Les jeux venus d'ailleurs que Steam.** GOG, Epic, itch, Battle.net, l'application Xbox, et tout ce que vous avez installé à la main.
- **Les jeux Steam où le développeur ne l'a jamais activé**, ou seulement pour une plateforme.
- **Les émulateurs.** RetroArch, Dolphin, PCSX2, RPCS3 et les autres écrivent où bon leur semble, et Steam n'en sait rien.
- **Les jeux qui écrivent hors du dossier surveillé par Steam**, et il y en a plus qu'on ne croit.

Hoard se moque de qui a publié un jeu et d'où il vient : il suit le dossier qui change quand vous jouez.

## Quand deux PC modifient la même sauvegarde

Vous jouez sur le portable sans laisser le fixe finir sa synchro, et voilà le problème classique : deux sauvegardes, toutes deux plus récentes que la dernière version commune.

Hoard n'écrase jamais à l'aveugle. Il compare les dates de modification, conserve une copie locale de ce qu'il remplace, et attend tant qu'un jeu tourne ou que la sauvegarde a été touchée dans les dernières minutes : un fichier en cours d'écriture n'est pas un fichier qu'on veut envoyer à moitié. Toutes les versions antérieures restent dans l'historique cloud : se tromper de version coûte deux clics, pas un week-end.

La limite honnête : **Hoard ne fusionne pas deux sauvegardes divergentes.** Aucun outil ne le peut — un fichier de sauvegarde est opaque, et il n'existe aucune façon correcte de mélanger deux après-midi de jeu différents. Ce que vous obtenez à la place, c'est toutes les versions, sur toutes les machines, et le choix.

## Les réglages se synchronisent-ils entre PC ?

Les sauvegardes, oui. Les réglages, par défaut, non, et c'est voulu. Le Steam Deck montre pourquoi : il tourne en 1280×800 avec un GPU de console portable, alors que votre PC fixe tourne peut-être en 4K sur une machine bien plus puissante. Copiez le \`graphics.ini\` du PC fixe sur le Deck et le jeu démarre dans une résolution que l'écran ne peut pas afficher, avec des réglages que le matériel ne tient pas.

Hoard trie donc ce qu'il trouve dans un dossier de sauvegarde :

- **Les sauvegardes** se synchronisent entre machines, à chaque session.
- **Les fichiers de réglages** (\`graphics.ini\`, \`settings.cfg\` et autres) sont conservés dans chaque sauvegarde, donc jamais perdus, mais une restauration ne les écrit pas par-dessus ceux d'une autre machine. Si l'entrée du jeu dans la base de données des sauvegardes indique qu'un \`.ini\` *est* la sauvegarde, il est traité comme tel.
- **Le superflu**, comme les rapports de plantage, les fichiers temporaires, les journaux du moteur type \`Player.log\` et la comptabilité propre à Steam, n'est pas sauvegardé du tout, pour qu'ouvrir un jeu ne crée pas à lui seul une nouvelle version.

Si vous voulez que les réglages voyagent, cochez **Restaurer aussi les fichiers de réglages** en restaurant une version depuis l'historique, ou réglez le jeu sur **Restaurer aussi les réglages** pour qu'ils suivent à chaque restauration, automatiques comprises. Pratique quand les deux PC ont le même écran et que vous avez déjà peaufiné le jeu.

## Synchroniser sans passer par nos serveurs

Autant le dire franchement, car c'est le point sur lequel presque toutes les comparaisons se trompent. Il y a deux façons de l'utiliser :

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont stockées sur nos serveurs, dans l'UE.
- **L'auto-hébergement est entièrement le vôtre.** Vous faites tourner \`hoard-server\` sur votre PC ou votre NAS et vos machines se synchronisent à travers lui. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

Le même programme, la même détection, le même historique de versions. La seule chose qui change, c'est à qui appartient le stockage.

## Astuce

Laissez chaque machine finir de synchroniser avant de lancer un jeu — le tableau de bord affiche l'état en direct, vous savez donc que la dernière sauvegarde est en place.

<!-- faq -->

## Questions fréquentes

### Combien de PC puis-je synchroniser ?

Trois avec l'offre gratuite, un nombre illimité avec Pro, et illimité en auto-hébergement : votre serveur, vos règles.

### Les deux machines doivent-elles être allumées en même temps ?

Non. Votre sauvegarde monte vers le serveur quand vous finissez de jouer et redescend quand l'autre machine la demande : le second PC peut rester éteint une semaine et recevoir quand même la dernière version à l'allumage.

### Et si je joue hors ligne ?

Aucun souci. L'instantané est pris localement quand vous arrêtez de jouer, et il part tout seul dès que la machine retrouve une connexion.

### Est-ce que ça synchronise aussi mes mods et réglages ?

Les sauvegardes, oui. Les fichiers propres à une machine — configuration, journaux et compagnie — sont envoyés pour figurer dans la sauvegarde, mais ne sont pas réécrits par-dessus la copie d'un autre PC : un réglage graphique qui convient à votre fixe est rarement celui que veut votre portable.

### L'auto-hébergement envoie-t-il quoi que ce soit à Hoard ?

Non. En mode auto-hébergé il n'y a aucun compte chez nous ni aucune télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux vivent sur votre propre serveur et ne touchent jamais le nôtre.

### Puis-je quand même copier mes réglages sur l'autre PC ?

Oui. Cochez **Restaurer aussi les fichiers de réglages** en restaurant une version, ou réglez le jeu sur **Restaurer aussi les réglages** pour les ramener à chaque restauration. En ligne de commande, \`hoard restore --allow-ini\` fait la même chose.
`,An=`---
title: "Come sincronizzare i salvataggi tra più PC"
description: "Gioca su fisso, portatile e Steam Deck senza perdere progressi: sincronizza i salvataggi tra PC in automatico, con cronologia. Passo dopo passo."
order: 2
updated: 2026-10-01
---

Se giochi su più di un computer — un fisso a casa e un portatile in giro — Hoard mantiene i salvataggi sincronizzati così riprendi sempre da dove avevi lasciato.

## Come funziona la sincronizzazione

Hoard fa il backup di ogni salvataggio sul tuo cloud e scarica l'ultima versione sulle altre macchine. Quando finisci di giocare su un PC, il salvataggio più recente ti aspetta sul successivo.

## Imposta la sincronizzazione

1. Installa **Hoard** su ogni PC su cui giochi (Windows, macOS o Linux).
2. Accedi con lo **stesso account** su ogni macchina, o collegale allo stesso server self-hosted.
3. Aggiungi gli stessi giochi alla **Libreria** su ogni PC. Hoard li abbina per gioco, così un salvataggio fatto su uno appare sugli altri.
4. Tieni attiva la **modalità automatica**. Hoard carica dopo che giochi e scarica l'ultima versione prima che inizi.

## Arrivi da Ludusavi?

Ludusavi è un ottimo strumento open source per fare backup e ripristinare salvataggi in locale, e può inviare quei backup a un cloud che configuri tu stesso con Rclone. Ma la sincronizzazione tra dispositivi la imposti a mano: programmare il backup, configurare il remoto, poi ripristinare sull'altro PC prima di giocare.

Hoard trasforma tutto questo in sincronizzazione gestita. Usa gli stessi dati comunitari di posizione di Ludusavi per trovare i tuoi salvataggi, poi carica dopo ogni sessione e scarica l'ultima versione prima della successiva — su ogni PC del tuo account, con cronologia versionata nel cloud. Niente remoti Rclone, niente script. E come Ludusavi, Hoard è open source e può essere self-hosted. Vedi il [confronto completo con Ludusavi](/guides/ludusavi-alternative).

## Evitare i conflitti

Hoard è consapevole dei conflitti: confronta le date di modifica e conserva una copia locale di ogni salvataggio sostituito, così una sincronizzazione non distrugge mai i progressi in silenzio. Se un gioco è ancora aperto o un salvataggio è stato toccato negli ultimi minuti, Hoard aspetta.

## Steam Deck e desktop

Il setup a due macchine più comune è anche quello che si rompe più spesso quando lo si monta a mano, e quasi sempre per lo stesso motivo.

Su Windows il salvataggio di un gioco può stare in \`Documenti\\My Games\\…\` oppure dentro \`userdata\` di Steam. Su una Steam Deck lo stesso gioco Windows gira con Proton, quindi il salvataggio vive dentro un prefisso di compatibilità: \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Due percorsi molto diversi, un gioco solo, un solo progresso. Hoard legge i prefissi Proton oltre alle posizioni native e abbina quello che trova per gioco, così il salvataggio della Deck e quello del desktop diventano due versioni della stessa cronologia invece di due cartelle scollegate.

Il dettaglio da cui dipende tutto: per i giochi Steam, Hoard traccia \`<AppID>/remote/\` dentro \`userdata\`, **non** la cartella superiore. Quella superiore contiene anche \`remotecache.vdf\` e i file di obiettivi e tempo di gioco propri di ogni macchina, che tra Deck e desktop devono essere diversi. Se sincronizzi la cartella superiore, ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso. È quell'unico errore a far sembrare rotti quasi tutti i setup artigianali tra Deck e PC.

## I giochi che Steam Cloud non copre

Se tutti i giochi a cui giochi supportassero Steam Cloud non ti servirebbe niente di tutto questo. Nella pratica:

- **Giochi che non vengono da Steam.** GOG, Epic, itch, Battle.net, l'app Xbox e tutto ciò che hai installato a mano.
- **Giochi Steam in cui lo sviluppatore non l'ha mai attivato**, o l'ha attivato per una sola piattaforma.
- **Emulatori.** RetroArch, Dolphin, PCSX2, RPCS3 e gli altri salvano dove preferiscono, e Steam non ne sa nulla.
- **Giochi che scrivono fuori dalla cartella sorvegliata da Steam**, e sono più di quanti immagini.

A Hoard non importa chi abbia pubblicato un gioco né da dove arrivi: traccia la cartella che cambia quando giochi.

## Quando due PC toccano lo stesso salvataggio

Giochi sul portatile senza lasciare che il fisso finisca di sincronizzare ed ecco il problema classico: due salvataggi, entrambi più recenti dell'ultima versione comune.

Hoard non sovrascrive mai alla cieca. Confronta le date di modifica, conserva una copia locale di ciò che sostituisce e aspetta finché un gioco è aperto o il salvataggio è stato toccato negli ultimi minuti: un file in scrittura non è un file da caricare a metà. Tutte le versioni precedenti restano nella cronologia cloud, quindi sbagliare versione costa due clic e non un fine settimana.

Il limite onesto: **Hoard non fonde due salvataggi divergenti.** Nessuno strumento può farlo — un file di salvataggio è opaco e non esiste un modo corretto di mescolare due pomeriggi di gioco diversi. Quello che ottieni invece è ogni versione, su ogni macchina, e la possibilità di scegliere.

## Le impostazioni si sincronizzano tra PC?

I salvataggi sì. Le impostazioni, di default, no, ed è voluto. La Steam Deck mostra perché: gira a 1280×800 con una GPU da portatile, mentre il tuo fisso magari va in 4K su qualcosa di molto più grosso. Copia il \`graphics.ini\` del fisso sulla Deck e il gioco parte a una risoluzione che lo schermo non può mostrare, con impostazioni che l'hardware non regge.

Per questo Hoard classifica ciò che trova in una cartella di salvataggio:

- **I salvataggi** si sincronizzano tra le macchine, a ogni sessione.
- **I file di impostazioni** (\`graphics.ini\`, \`settings.cfg\` e simili) restano in ogni backup, quindi non si perdono mai, ma un ripristino non li scrive sopra quelli di un'altra macchina. Se la voce del gioco nel database dei salvataggi dice che un \`.ini\` *è* il salvataggio, viene trattato come salvataggio.
- **Il superfluo**, come crash dump, file temporanei, log del motore tipo \`Player.log\` e la contabilità di Steam, non viene salvato affatto, così aprire un gioco non crea da solo una nuova versione.

Se invece vuoi che le impostazioni viaggino, spunta **Ripristina anche i file di impostazioni** quando ripristini una versione dalla cronologia, oppure imposta il gioco su **Ripristina anche le impostazioni** perché ogni ripristino le porti, compresi quelli automatici. Utile quando i due PC hanno lo stesso schermo e hai già messo a punto il gioco una volta.

## Sincronizzare senza passare dai nostri server

Vale la pena dirlo chiaramente, perché è il punto su cui quasi tutti i confronti sbagliano. Ci sono due modi di usarlo:

- **Hoard Cloud** è l'opzione gestita: accedi e i salvataggi stanno sui nostri server, nell'UE.
- **Il self-hosting è interamente tuo.** Fai girare \`hoard-server\` sul tuo PC o sul tuo NAS e le tue macchine si sincronizzano attraverso quello. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia delle versioni. L'unica cosa che cambia è di chi è lo spazio di archiviazione.

## Suggerimento

Lascia che ogni macchina finisca di sincronizzare prima di avviare un gioco — la dashboard mostra lo stato in tempo reale, così sai che l'ultimo salvataggio è al suo posto.

<!-- faq -->

## Domande frequenti

### Quanti PC posso sincronizzare?

Tre nel piano gratuito, illimitati con Pro e illimitati in self-hosting: il tuo server, le tue regole.

### Le due macchine devono essere accese nello stesso momento?

No. Il salvataggio sale al server quando smetti di giocare e scende quando l'altra macchina lo chiede: il secondo PC può restare spento una settimana e ricevere comunque l'ultima versione all'accensione.

### E se gioco offline?

Nessun problema. Lo snapshot viene preso in locale quando smetti di giocare e parte da solo appena la macchina torna online.

### Sincronizza anche mod e impostazioni?

I salvataggi sì. I file che appartengono a una macchina specifica — configurazione, log e simili — vengono caricati per essere nel backup, ma non riscritti sopra la copia di un altro PC: un'impostazione grafica che va bene al fisso è raramente quella che vuole il portatile.

### Il self-hosting manda qualcosa a Hoard?

No. In modalità self-hosted non c'è alcun account con noi né telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log stanno sul tuo server e non toccano mai il nostro.

### Posso comunque copiare le mie impostazioni sull'altro PC?

Sì. Spunta **Ripristina anche i file di impostazioni** quando ripristini una versione, oppure imposta il gioco su **Ripristina anche le impostazioni** per portarle a ogni ripristino. Da riga di comando, \`hoard restore --allow-ini\` fa lo stesso.
`,Ln=`---
title: "複数の PC 間でセーブデータを同期する方法"
description: "デスクトップ、ノートPC、Steam Deckで進行を失わずにプレイ。セーブをPC間で自動同期し、バージョン履歴も残す方法を手順で解説。"
order: 2
updated: 2026-10-01
---

複数のコンピューター（自宅のデスクトップと外出先のノート PC など）でプレイするなら、Hoard がセーブデータを同期し続けるので、いつでも続きから再開できます。

## 同期の仕組み

Hoard は各セーブをクラウドにバックアップし、ほかのマシンに最新バージョンをダウンロードします。ある PC でプレイを終えると、最新のセーブが次の PC で待っています。

## 同期を設定する

1. プレイするすべての PC に **Hoard をインストール** します（Windows、macOS、Linux）。
2. 各マシンで **同じアカウント** でサインインするか、同じセルフホストサーバーに接続します。
3. 各 PC の **ライブラリ** に同じゲームを追加します。Hoard はゲーム単位で対応付けるので、一方でバックアップしたセーブが他方にも表示されます。
4. **自動モード** をオンのままにします。Hoard はプレイ後にアップロードし、開始前に最新版をダウンロードします。

## Ludusavi から移行しますか？

Ludusavi はローカルでセーブをバックアップ・復元する優れたオープンソースツールで、Rclone で自分で設定したクラウドへバックアップを送ることもできます。ただし端末間の同期は自分で組む必要があります。バックアップをスケジュールし、リモートを設定し、プレイ前にもう一方の PC で復元する、という流れです。

Hoard はこれをマネージドな同期に変えます。Ludusavi と同じコミュニティのセーブ位置データを使ってセーブを見つけ、各セッション後にアップロードし、次の前に最新版をダウンロードします。アカウント内のすべての PC で、クラウド上に世代履歴を保ちながら行われます。Rclone のリモートもスクリプトも不要です。そして Ludusavi と同様に、Hoard もオープンソースでセルフホスト可能です。詳しくは [Ludusavi 代替の比較](/guides/ludusavi-alternative) をご覧ください。

## 競合を避ける

Hoard は競合を認識します。更新時刻を比較し、置き換えるセーブのローカルコピーを保持するため、同期が黙って進行を壊すことはありません。ゲームがまだ起動中だったり、直近数分でセーブが変更されていたりする場合、Hoard は待機します。

## Steam Deck とデスクトップ

2 台構成として最も多い組み合わせは、手作業で組んだときに最も壊れやすい組み合わせでもあり、原因はほぼ毎回同じです。

Windows では、セーブは \`ドキュメント\\My Games\\…\` か Steam の \`userdata\` にあります。Steam Deck では同じ Windows 版ゲームが Proton 経由で動くため、セーブは互換プレフィックスの中、\`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` に置かれます。まるで違うパス、同じゲーム、ひとつづきの進行です。Hoard はネイティブの場所に加えて Proton のプレフィックスも読み、見つけたものをゲーム単位で結びつけます。こうして Deck のセーブとデスクトップのセーブは、無関係な 2 つのフォルダーではなく、ひとつの履歴の 2 つの世代になります。

すべてを左右する細部があります。Steam のゲームでは、Hoard は \`userdata\` の中の \`<AppID>/remote/\` を追跡し、その **上のフォルダーは追跡しません**。上のフォルダーには \`remotecache.vdf\` や、実績・プレイ時間といったマシンごとに異なって当然のファイルが入っています。上を同期すると、セーブが動いていなくても起動のたびに競合に見えます。自作の Deck と PC の構成が壊れているように感じられる原因は、たいていこの一点です。

## Steam クラウドが面倒を見ないゲーム

遊ぶゲームがすべて Steam クラウドに対応していれば、こうした仕組みは要りません。現実には:

- **Steam 以外から来たゲーム。** GOG、Epic、itch、Battle.net、Xbox アプリ、そして手動で入れたもの全部。
- **開発者が有効にしなかった Steam のゲーム。** あるいは片方のプラットフォームでしか有効にしていないもの。
- **エミュレーター。** RetroArch、Dolphin、PCSX2、RPCS3 などは好きな場所に保存し、Steam はそれを知りません。
- **Steam が見ているフォルダーの外に書き込むゲーム。** 思っているより多くあります。

Hoard は誰が出したゲームかも、どこから来たかも問いません。プレイすると変化するフォルダーを追跡するだけです。

## 2 台の PC が同じセーブを触ったとき

デスクトップの同期が終わらないうちにノート PC で遊ぶと、古典的な問題が起きます。最後の共通世代より新しいセーブが 2 つある状態です。

Hoard は決して無言で上書きしません。更新時刻を比べ、置き換えるものはローカルに控えを残し、ゲームが動作中か、セーブが直前の数分に触られていれば待ちます。書き込み途中のファイルは、半端な状態でアップロードしたくないからです。以前の世代はすべてクラウドの履歴に残るので、選び間違えても週末ではなく 2 クリックで済みます。

正直な限界を書いておきます。**Hoard は分岐した 2 つのセーブを統合しません。** どのツールにもできません。セーブファイルは中身が読めず、異なる 2 回のプレイを正しく混ぜる方法は存在しないからです。代わりに手に入るのは、すべての世代がすべてのマシンにあり、選べるという状態です。

## 設定は PC 間で同期されますか？

セーブは同期されます。設定は既定では同期されず、これは意図的なものです。理由は Steam Deck を見るとわかります。Deck は携帯機の GPU で 1280×800 で動き、デスクトップはもっと強力なマシンで 4K かもしれません。デスクトップの \`graphics.ini\` を Deck にコピーすると、画面が表示できない解像度、ハードウェアが耐えられない設定でゲームが起動してしまいます。

そこで Hoard はセーブフォルダーの中身を分類します。

- **セーブ**は毎セッション、マシン間で同期されます。
- **設定ファイル**（\`graphics.ini\`、\`settings.cfg\` など）はすべてのバックアップに含まれるので失われることはありませんが、復元時に別のマシンのものを上書きしません。セーブデータベースのそのゲームの項目で \`.ini\` *が* セーブだとされている場合は、セーブとして扱います。
- **不要なもの**（クラッシュダンプ、一時ファイル、\`Player.log\` のようなエンジンのログ、Steam 自身の管理ファイル）はバックアップしません。ゲームを起動しただけで新しいバージョンができることはありません。

設定も移したいときは、履歴からバージョンを復元する際に **設定ファイルも復元する** にチェックを入れるか、ゲームを **設定も復元する** に設定して、自動復元を含むすべての復元で設定を持ってくるようにします。2 台の PC が同じ画面で、一度ゲームを調整済みなら便利です。

## 当方のサーバーを介さない同期

多くの比較が誤解している点なので、はっきり書きます。動かし方は 2 通りあります。

- **Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに保存されます。
- **セルフホストは完全にあなたのものです。** 自分の PC や NAS で \`hoard-server\` を動かし、各マシンはそれを介して同期します。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

同じプログラム、同じ検出、同じ世代履歴。変わるのは保存先が誰のものかだけです。

## ヒント

ゲームを起動する前に、各マシンの同期が完了するまで少し待ちましょう。ダッシュボードがリアルタイムの状態を表示するので、最新のセーブが揃っているか分かります。

<!-- faq -->

## よくある質問

### 何台の PC を同期できますか？

無料枠は 3 台、Pro は無制限、セルフホストでも無制限です。自分のサーバーなら台数は自分で決められます。

### 2 台を同時に起動しておく必要はありますか？

いいえ。セーブはプレイ終了時にサーバーへ上がり、もう一方のマシンが求めたときに下ります。2 台目は 1 週間電源を切っていても、次に起動したときに最新の世代を受け取れます。

### オフラインで遊んだ場合は？

問題ありません。スナップショットはプレイ終了時にローカルで作られ、回線が戻ったときに自動でアップロードされます。

### Mod や設定も同期されますか？

セーブは同期されます。特定のマシンに属するファイル、つまり設定やログなどはバックアップに含めるためアップロードされますが、他の PC の同じファイルを上書きすることはありません。デスクトップに合うグラフィック設定が、ノート PC にも合うとは限らないからです。

### セルフホストは Hoard に何かを送信しますか？

いいえ。セルフホストでは当方のアカウントも当方へのテレメトリもありません。セーブもユーザーもログも自分のサーバーの中にとどまり、当方のサーバーには一切触れません。

### それでも設定を別の PC にコピーできますか？

はい。バージョンを復元する際に **設定ファイルも復元する** にチェックを入れるか、ゲームを **設定も復元する** に設定すれば、毎回の復元で設定も持ってきます。コマンドラインでは \`hoard restore --allow-ini\` で同じことができます。
`,Hn=`---
title: "Como sincronizar saves entre vários PCs"
description: "Joga no fixo, no portátil e na Steam Deck sem perder progresso: sincroniza os saves entre PCs automaticamente, com histórico. Passo a passo."
order: 2
updated: 2026-10-01
---

Se jogas em mais de um computador — um fixo em casa e um portátil em viagem — o Hoard mantém os teus saves sincronizados para que retomes sempre onde paraste.

## Como funciona a sincronização

O Hoard faz backup de cada save para a tua nuvem e descarrega a versão mais recente nas tuas outras máquinas. Quando acabas de jogar num PC, o save mais recente espera-te no seguinte.

## Configurar a sincronização

1. Instala o **Hoard** em cada PC onde jogas (Windows, macOS ou Linux).
2. Inicia sessão com a **mesma conta** em cada máquina, ou liga-as ao mesmo servidor self-hosted.
3. Adiciona os mesmos jogos à **Biblioteca** em cada PC. O Hoard associa-os por jogo, por isso um save feito num aparece nos outros.
4. Mantém o **modo automático** ligado. O Hoard envia depois de jogares e descarrega a versão mais recente antes de começares.

## Vens do Ludusavi?

O Ludusavi é uma excelente ferramenta open source para fazer backup e restaurar saves localmente, e pode enviar esses backups para uma nuvem que configuras tu mesmo com o Rclone. Mas a sincronização entre dispositivos montas tu à mão: agendar o backup, configurar o remoto, e depois restaurar no outro PC antes de jogar.

O Hoard transforma isso em sincronização gerida. Usa os mesmos dados comunitários de localização do Ludusavi para encontrar os teus saves, depois envia após cada sessão e descarrega a versão mais recente antes da seguinte — em cada PC da tua conta, com histórico versionado na nuvem. Sem remotos de Rclone, sem scripts. E como o Ludusavi, o Hoard é open source e pode ser self-hosted. Vê a [comparação completa com o Ludusavi](/guides/ludusavi-alternative).

## Evitar conflitos

O Hoard tem em conta os conflitos: compara as datas de modificação e guarda uma cópia local de qualquer save substituído, por isso uma sincronização nunca destrói progresso em silêncio. Se um jogo ainda estiver aberto ou um save foi tocado nos últimos minutos, o Hoard espera.

## Steam Deck e desktop

A montagem de duas máquinas mais comum é também a que mais se estraga quando é feita à mão, e quase sempre pelo mesmo motivo.

No Windows, o save de um jogo pode estar em \`Documentos\\My Games\\…\` ou dentro do \`userdata\` da Steam. Numa Steam Deck, esse mesmo jogo de Windows corre com Proton, por isso o save vive dentro de um prefixo de compatibilidade: \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Dois caminhos muito diferentes, um só jogo, um só progresso. O Hoard lê os prefixos Proton além das localizações nativas e associa o que encontra por jogo, por isso o save da Deck e o do desktop passam a ser duas versões do mesmo histórico em vez de duas pastas sem relação.

O detalhe de que tudo depende: nos jogos da Steam, o Hoard segue \`<AppID>/remote/\` dentro de \`userdata\`, e **não** a pasta acima. A pasta acima guarda também \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo próprios de cada máquina, que devem ser diferentes entre a tua Deck e o teu desktop. Se sincronizares a de cima, cada arranque parece um conflito mesmo sem nenhum save se ter mexido. É esse único erro que faz parecerem avariadas quase todas as montagens caseiras entre Deck e PC.

## Jogos que a Steam Cloud não cobre

Se todos os jogos que jogas suportassem Steam Cloud, não precisarias de nada disto. Na prática:

- **Jogos vindos de qualquer sítio que não a Steam.** GOG, Epic, itch, Battle.net, a app da Xbox e tudo o que instalaste à mão.
- **Jogos da Steam em que o programador nunca a ativou**, ou ativou só para uma plataforma.
- **Emuladores.** RetroArch, Dolphin, PCSX2, RPCS3 e companhia guardam onde lhes apetece, e a Steam não sabe nada disso.
- **Jogos que escrevem fora da pasta vigiada pela Steam**, e são mais do que se imagina.

Ao Hoard tanto lhe faz quem publicou o jogo ou de onde veio: segue a pasta que muda quando jogas.

## Quando dois PCs mexem no mesmo save

Jogas no portátil sem deixar o fixo acabar de sincronizar e tens o problema clássico: dois saves, ambos mais recentes do que a última versão comum.

O Hoard nunca escreve por cima às cegas. Compara datas de modificação, guarda uma cópia local do que substitui, e espera enquanto houver um jogo aberto ou o save tiver sido tocado nos últimos minutos: um ficheiro a ser escrito não é um ficheiro que queiras enviar a meio. Todas as versões anteriores ficam no histórico da nuvem, por isso enganares-te na versão custa dois cliques e não um fim de semana.

O limite honesto: **o Hoard não funde dois saves divergentes.** Nenhuma ferramenta o consegue — um ficheiro de save é opaco, e não há forma correta de misturar duas tardes de jogo diferentes. O que tens em troca é todas as versões, em todas as máquinas, e a possibilidade de escolher.

## As definições sincronizam entre PCs?

Os saves sim. As definições, por predefinição, não, e é de propósito. A Steam Deck mostra porquê: corre a 1280×800 com uma GPU de portátil, enquanto o teu PC talvez corra a 4K em algo muito maior. Copia o \`graphics.ini\` do PC para a Deck e o jogo arranca numa resolução que o ecrã não consegue mostrar, com definições que o hardware não aguenta.

Por isso o Hoard classifica o que encontra numa pasta de saves:

- **Os saves** sincronizam entre máquinas, em cada sessão.
- **Os ficheiros de definições** (\`graphics.ini\`, \`settings.cfg\` e afins) ficam em cada backup, por isso nunca se perdem, mas um restauro não os escreve por cima dos de outra máquina. Se a entrada do jogo na base de dados de saves disser que um \`.ini\` *é* o save, é tratado como save.
- **O lixo**, como crash dumps, ficheiros temporários, logs do motor tipo \`Player.log\` e a contabilidade própria do Steam, nem sequer entra no backup, para que abrir um jogo não crie por si só uma versão nova.

Quando quiseres mesmo que as definições viajem, marca **Restaurar também os ficheiros de definições** ao restaurar uma versão a partir do histórico, ou configura o jogo com **Restaurar também as definições** para que cada restauro as traga, incluindo os automáticos. Útil quando os dois PCs têm o mesmo ecrã e já afinaste o jogo uma vez.

## Sincronizar sem passar pelos nossos servidores

Vale a pena dizê-lo de forma explícita, porque é o ponto em que quase todas as comparações se enganam. Há duas formas de o usar:

- **O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.
- **O self-hosting é inteiramente teu.** Corres o \`hoard-server\` no teu PC ou no teu NAS e as tuas máquinas sincronizam através dele. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico de versões. A única coisa que muda é de quem é o armazenamento.

## Dica

Dá a cada máquina um momento para terminar a sincronização antes de abrires um jogo — o painel mostra o estado em tempo real, por isso sabes que o save mais recente já está no sítio.

<!-- faq -->

## Perguntas frequentes

### Quantos PCs posso sincronizar?

Três no plano gratuito, ilimitados no Pro e ilimitados em self-hosting: o teu servidor, as tuas regras.

### As duas máquinas têm de estar ligadas ao mesmo tempo?

Não. O teu save sobe para o servidor quando acabas de jogar e desce quando a outra máquina o pede, por isso o segundo PC pode estar desligado uma semana e mesmo assim receber a versão mais recente ao ligar.

### E se jogar sem ligação?

Sem problema. O snapshot é tirado localmente quando páras de jogar, e sobe sozinho assim que a máquina volta a ter ligação.

### Também sincroniza mods e definições?

Os saves, sim. Os ficheiros que pertencem a uma máquina em concreto — configuração, registos e afins — são enviados para ficarem no backup, mas não são escritos por cima da cópia de outro PC: uma definição gráfica que serve ao teu fixo raramente é a que o teu portátil quer.

### O self-hosting envia alguma coisa para o Hoard?

Não. Em modo self-hosted não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus registos vivem no teu próprio servidor e nunca tocam no nosso.

### Posso copiar as minhas definições para o outro PC mesmo assim?

Sim. Marca **Restaurar também os ficheiros de definições** ao restaurar uma versão, ou configura o jogo com **Restaurar também as definições** para as trazer em cada restauro. Na linha de comandos, \`hoard restore --allow-ini\` faz o mesmo.
`,jn=`---
title: "如何在多台 PC 之间同步游戏存档"
description: "在台式机、笔记本和 Steam Deck 上游玩而不丢进度：在多台 PC 之间自动同步游戏存档并保留版本历史，分步讲解。"
order: 2
updated: 2026-10-01
---

如果你在不止一台电脑上玩游戏——家里的台式机和外出用的笔记本——Hoard 会让你的存档保持同步，让你总能从上次离开的地方继续。

## 同步的原理

Hoard 会把每个存档备份到你的云端，并在你的其他机器上拉取最新版本。当你在一台 PC 上玩完，最新的存档就已在下一台等着你。

## 设置同步

1. 在你玩游戏的每台 PC 上**安装 Hoard**（Windows、macOS 或 Linux）。
2. 在每台机器上用**同一账号**登录，或把它们连接到同一台自托管服务器。
3. 在每台 PC 的**库**中添加相同的游戏。Hoard 按游戏进行匹配，因此在一台上备份的存档会出现在其他机器上。
4. 保持**自动模式**开启。Hoard 会在你玩完后上传，并在你开始前下载最新版本。

## 从 Ludusavi 迁移？

Ludusavi 是一款出色的开源工具，可在本地备份和还原存档，并能通过你自己用 Rclone 配置的云端推送这些备份。但跨设备同步需要你自己搭建：安排备份、配置远端，然后在玩之前在另一台 PC 上还原。

Hoard 把这一切变成托管式同步。它使用与 Ludusavi 相同的社区存档位置数据来找到你的存档，然后在每次会话后上传、在下一次之前下载最新版本——覆盖你账号下的每台 PC，并在云端保留版本历史。无需 Rclone 远端，无需脚本。而且与 Ludusavi 一样，Hoard 同样开源且可自托管。请见完整的 [Ludusavi 替代方案对比](/guides/ludusavi-alternative)。

## 避免冲突

Hoard 具备冲突感知：它会比较修改时间，并为任何被替换的存档保留一份本地副本，因此同步绝不会悄无声息地破坏进度。如果某款游戏仍在运行，或某个存档在最近几分钟内被改动过，Hoard 会等待。

## Steam Deck 与台式机

最常见的双机组合，也正是手工搭建时最容易出问题的组合，而且原因几乎每次都一样。

在 Windows 上，存档可能在 \`文档\\My Games\\…\`，也可能在 Steam 的 \`userdata\` 里。在 Steam Deck 上，同一款 Windows 游戏通过 Proton 运行，存档因此位于兼容层前缀内：\`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`。两条完全不同的路径，同一款游戏，同一份进度。Hoard 除了原生位置之外也会读取 Proton 前缀，并按游戏把找到的内容对应起来，于是 Deck 的存档和台式机的存档成为同一段历史的两个版本，而不是两个毫不相干的文件夹。

决定成败的细节：对 Steam 游戏，Hoard 追踪 \`userdata\` 里的 \`<AppID>/remote/\`，而**不是**它上一层的文件夹。上一层还放着 \`remotecache.vdf\` 以及成就和游戏时长这类本就属于各台机器的文件，它们在 Deck 和台式机之间理应不同。同步上一层，每次启动都像冲突，尽管没有任何存档动过。绝大多数手工搭建的 Deck 与 PC 方案让人觉得"坏掉了"，就坏在这一点上。

## Steam 云存档管不到的游戏

如果你玩的每款游戏都支持 Steam 云存档，这一切都不需要。但实际上：

- **不是来自 Steam 的游戏。** GOG、Epic、itch、Battle.net、Xbox 应用，以及你手动安装的一切。
- **开发者从未开启云存档的 Steam 游戏**，或者只为某一个平台开启。
- **模拟器。** RetroArch、Dolphin、PCSX2、RPCS3 等等想存哪儿就存哪儿，Steam 对此一无所知。
- **写在 Steam 监视范围之外的游戏**，而且比你想的要多。

Hoard 不在乎游戏由谁发行、从哪儿来：它追踪的是你游玩时会变化的那个文件夹。

## 当两台 PC 改动同一份存档

没等台式机同步完就在笔记本上玩，就会遇到经典问题：两份存档，都比上一次共同的版本更新。

Hoard 从不盲目覆盖。它比较修改时间，为被替换的内容保留本地副本，并在游戏仍在运行、或存档在最近几分钟内被改动过时先等一等——正在写入的文件，不是你想传到一半的文件。所有更早的版本都留在云端历史里，因此选错版本的代价是两次点击，而不是一个周末。

坦白说出限制：**Hoard 不会合并两份已经分叉的存档。** 任何工具都做不到——存档文件是不透明的，把两个不同下午的游玩正确地揉在一起并不存在。你得到的是每一个版本、每一台机器上都有，以及自己选择的余地。

## 设置会在 PC 之间同步吗？

存档会。设置默认不会，而且是有意为之。Steam Deck 就说明了原因：它用掌机 GPU 以 1280×800 运行，而你的台式机可能在强得多的硬件上跑 4K。把台式机的 \`graphics.ini\` 复制到 Deck 上，游戏就会以屏幕无法显示的分辨率、硬件扛不住的设置启动。

所以 Hoard 会对存档文件夹里的内容进行分类：

- **存档**每次游玩后都会在设备之间同步。
- **设置文件**（\`graphics.ini\`、\`settings.cfg\` 等）会保存在每次备份中，绝不会丢失，但恢复时不会覆盖另一台设备上的同名文件。如果存档数据库中该游戏的条目表明某个 \`.ini\` *就是* 存档，它会被当作存档处理。
- **杂项**，例如崩溃转储、临时文件、\`Player.log\` 这类引擎日志以及 Steam 自己的记录文件，根本不会备份，所以仅仅打开游戏不会产生新版本。

如果你确实希望设置也跟着走，在从历史记录恢复某个版本时勾选**同时恢复设置文件**，或者把该游戏设为**同时恢复设置**，这样每次恢复（包括自动恢复）都会带上设置。两台 PC 屏幕相同、你又已经调好过一次游戏时，这很方便。

## 不经过我们服务器的同步

值得说明白，因为这正是多数对比弄错的地方。它有两种运行方式：

- **Hoard Cloud** 是托管方案：你登录，存档保存在我们位于欧盟的服务器上。
- **自托管完全属于你。** 你在自己的 PC 或 NAS 上运行 \`hoard-server\`，各台机器通过它同步。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。参见[如何自托管 Hoard](/guides/self-host-hoard)。

同一个程序，同样的检测，同样的版本历史。唯一变化的是存储归谁所有。

## 提示

在启动游戏前，给每台机器一点时间完成同步——仪表盘会显示实时状态，让你知道最新存档已经就位。

<!-- faq -->

## 常见问题

### 我可以同步多少台 PC？

免费额度三台，Pro 不限台数，自托管同样不限——你的服务器，你说了算。

### 两台机器需要同时开机吗？

不需要。你玩完之后存档会上传到服务器，另一台机器需要时再取下来，所以第二台 PC 关机一周，开机后照样能拿到最新版本。

### 离线玩怎么办？

没问题。快照是在你停止游玩时于本地生成的，等机器重新联网后会自行上传。

### 它也会同步模组和设置吗？

存档会。属于某一台机器的文件——配置、日志之类——会上传以便进入备份，但不会覆盖另一台 PC 上的同名文件：适合你台式机的画质设置，通常并不是笔记本想要的。

### 自托管会向 Hoard 发送任何东西吗？

不会。在自托管模式下，没有我们这边的账号，也没有发往我们的遥测：你的存档、你的用户和你的日志都留在你自己的服务器上，从不接触我们的服务器。

### 我还是想把设置复制到另一台 PC，可以吗？

可以。恢复某个版本时勾选**同时恢复设置文件**，或者把游戏设为**同时恢复设置**，让每次恢复都带上设置。在命令行中，\`hoard restore --allow-ini\` 效果相同。
`,xn=`---
title: "Syncthing für Spielstände: was klappt und was bricht"
description: "Syncthing ist ein toller Datei-Sync, aber Spielstände brechen drei seiner Annahmen. Was schiefgeht, die Workarounds und wann ein Save-Tool besser passt."
order: 9
updated: 2026-09-01
---

Syncthing ist die Antwort, zu der viele zuerst greifen, und das aus gutem Grund: kostenlos, quelloffen, peer-to-peer, und es funktioniert. Doch Spielstände brechen drei Annahmen, auf denen ein universeller Datei-Sync aufbaut, und die Fehler sind leise. Diese Anleitung handelt davon, was wirklich schiefgeht, und wann sich ein Werkzeug lohnt, das weiß, was ein Spielstand ist.

## Warum man dort landet

Es ist wirklich gute Software. Kein Konto, kein Abo, deine Dateien liegen nie auf der Platte einer Firma, und es synchronisiert alles: Dokumente, Fotos, einen Ordner mit Spielständen. Wenn du es ohnehin betreibst, kostet dich ein zusätzlicher Ordner dreißig Sekunden. Das ist ein echtes Argument, und für manche Setups das richtige.

## Die drei Dinge, die brechen

**Es synchronisiert, während das Spiel läuft.** Syncthing reagiert darauf, dass sich eine Datei ändert — für ein Dokument genau richtig. Ein Spiel schreibt seinen Stand mitten in der Sitzung, manchmal in mehreren Durchgängen, und eine Datei, die mitten im Schreiben erwischt wird, verbreitet sich halbfertig. Die andere Maschine hat dann einen Stand, den das Spiel womöglich nicht lädt.

**Konflikte werden zu Dateien statt zu Entscheidungen.** Ändern beide Maschinen denselben Stand, tut Syncthing das Sichere und behält beide, indem es einen in \`etwas.sync-conflict-20260901-143022-ABCDEFG.sav\` umbenennt. Verloren geht nichts — aber das Spiel weiß nicht, was diese Datei ist, und du vergleichst Zeitstempel im Dateimanager, um zu entscheiden, welchen Spielnachmittag du behältst. Ein paar Mal, und der Ordner füllt sich mit Konfliktdateien, die niemand zu löschen wagt.

**Versionierung ist pro Datei, nicht pro Sitzung.** Syncthing kann alte Kopien in \`.stversions\` aufheben, besser als nichts. Aber ein Spielstand besteht oft aus mehreren Dateien, die nur zusammen Sinn ergeben, und Wiederherstellen heißt, für jede den richtigen Zeitstempel von Hand zu finden. Ein "setz dieses Spiel auf Dienstag zurück" gibt es nicht.

Und ein vierter Punkt, speziell für Steam: richtest du es auf \`userdata/<UserID>/<AppID>/\` statt auf den \`remote/\`-Ordner darin, synchronisierst du auch \`remotecache.vdf\` sowie Dateien für Erfolge und Spielzeit, die sich zwischen Maschinen unterscheiden **sollen**. Dann sieht jeder Start nach einem Konflikt aus, obwohl sich kein Stand bewegt hat. Das ist der häufigste Grund, warum ein selbstgebautes Setup zwischen Steam Deck und Desktop kaputt wirkt.

## Was du am Ende selbst baust

Nichts davon ist unlösbar. Man behilft sich mit Ignore-Mustern je Spiel, einer Versionierungsrichtlinie und der Gewohnheit, das Spiel zu schließen und zu warten, bevor man den anderen PC anfasst. Das funktioniert, und es ist Pflege, die dir für immer gehört: ein neues Spiel heißt neue Pfade, und der Tag, an dem du das Warten vergisst, ist der Tag, an dem du es merkst.

## Was ein spielstandbewusstes Werkzeug stattdessen tut

Hoard sichert **nachdem du aufgehört hast**, sobald der Ordner zur Ruhe kommt, ein Snapshot ist also nie eine halb geschriebene Datei. Jede Sicherung ist eine Version des ganzen Spielstands, nicht einzelner Dateien, das Wiederherstellen ist ein Klick und setzt alles gemeinsam zurück. Es weiß, welcher Ordner zu welchem Spiel gehört — es liest dasselbe Community-Manifest für Speicherorte, das im Open-Source-Umfeld geteilt wird, mit über 20.000 Titeln — es gibt also keine Pfade zu pflegen, und es verfolgt \`<AppID>/remote/\` statt des Ordners darüber.

## Wann Syncthing die bessere Antwort ist

Fairerweise:

- **Du betreibst es ohnehin**, ein Ordner mehr ist gratis.
- **Du willst peer-to-peer ganz ohne Server**, nicht einmal einen eigenen.
- **Du synchronisierst weit mehr als Spielstände** und hättest lieber ein Werkzeug für alles.
- **Du rollst nie zurück.** Wenn der letzte Stand immer gereicht hat, ist eine Versionshistorie Maschinerie, die du nicht nutzt.

## Beides nutzen

Sie vertragen sich, und das ist ein vernünftiges Setup: der universelle Sync übernimmt Dokumente und den Rest, ein spielstandbewusstes Werkzeug die Speicherordner. Die einzige Regel: richte nicht beide auf denselben Ordner — zwei Programme, die dieselben Dateien schreiben, erzeugen genau die Konflikte, die du vermeiden wolltest.

## Auch ohne unsere Server

Wenn ein Teil des Reizes ist, dass nichts die Platte einer Firma berührt: Hoard geht genauso. \`hoard-server\` auf deinem eigenen PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

Dasselbe Binary, dieselbe Erkennung, dieselbe Historie. Es ändert sich nur, wem der Speicher gehört. Es gibt außerdem einen vollständigen [Vergleich aller Sync-Tools](/guides/game-save-sync-comparison).

<!-- faq -->

## Häufige Fragen

### Kann Syncthing Spielstände überhaupt synchronisieren?

Ja, und in einfachen Fällen tut es das gut. Schwierig wird es bei Spielen, die während des Spielens schreiben, bei Spielständen aus mehreren Dateien, und überall dort, wo beide Maschinen zwischen zwei Synchronisierungen bearbeitet werden.

### Was sind die .sync-conflict-Dateien in meinem Speicherordner?

Das ist der Sync, der nach einem Konflikt beide Fassungen behält, statt eine zu wählen. Verloren geht nichts, aber das Spiel kann sie nicht lesen, und die Entscheidung ist jedes Mal Handarbeit.

### Warum kollidiert mein Steam-Spielstand bei jedem Start?

Fast immer, weil der synchronisierte Ordner der über \`remote/\` ist. Er enthält \`remotecache.vdf\` sowie Dateien für Erfolge und Spielzeit, die sich zu Recht je Rechner unterscheiden — die beiden Enden werden sich also nie einig.

### Muss ich das Spiel vor dem Synchronisieren schließen?

Mit einem universellen Sync ja, das ist die Gewohnheit, die halb geschriebene Stände verhindert. Ein spielstandbewusstes Werkzeug wartet von selbst, bis der Ordner ruhig ist.

### Kann ich beide zusammen nutzen?

Ja. Richte sie nur nicht auf denselben Ordner, sonst streiten sie sich um dieselben Dateien.
`,On=`---
title: "Syncthing for game saves: what works and what breaks"
description: "Syncthing is a great file syncer, but game saves break three of its assumptions. What goes wrong, the workarounds, and when a save-aware tool fits better."
order: 9
updated: 2026-09-01
related: game-save-sync-comparison, sync-game-saves-across-pcs, opensave-alternative
---

Syncthing is the answer a lot of people reach for first, and for good reason: it's free, open source, peer-to-peer, and it works. But game saves break three of the assumptions a general-purpose file syncer is built on, and the failures are quiet ones. This guide is about what actually goes wrong, and when it's worth using something that knows what a save is.

## Why people reach for it

It's genuinely good software. No account, no subscription, your files never sit on a company's disk, and it syncs anything: documents, photos, a folder of saves. If you already run it for other things, pointing it at a save folder costs you thirty seconds. That's a real argument, and for some setups it's the right one.

## The three things that break

**It syncs while the game is running.** Syncthing reacts to a file changing, because that's the correct behaviour for a document. A game writes its save in the middle of a session, sometimes in several passes, and a file caught mid-write is a file that propagates half-finished. The other machine now holds a save the game may refuse to load.

**Conflicts become files, not decisions.** When both machines change the same save, Syncthing does the safe thing and keeps both, renaming one to \`something.sync-conflict-20260901-143022-ABCDEFG.sav\`. Nothing is lost — but the game doesn't know what that file is, and you're left comparing timestamps in a file manager to work out which afternoon of play to keep. Do this a few times and the folder fills with conflict files nobody dares delete.

**Versioning is per file, not per session.** Syncthing can keep old copies in \`.stversions\`, and that's better than nothing. But a save is often several files that only make sense together, and restoring means finding the right timestamp for each one by hand. There's no "put this game back the way it was on Tuesday".

And a fourth, specific to Steam: point it at \`userdata/<UserID>/<AppID>/\` instead of the \`remote/\` folder inside, and you're also syncing \`remotecache.vdf\` plus achievement and playtime files that are *supposed* to differ between machines. Every launch then looks like a conflict even though no save actually moved. This is the single most common reason a hand-rolled Steam Deck and desktop setup feels broken.

## What you end up building

None of the above is unfixable. People handle it with ignore patterns per game, a versioning policy, and the habit of closing the game and waiting before touching the other PC. That works, and it's a maintenance job you own forever: a new game means new paths, and the day you forget to wait is the day you find out.

## What a save-aware tool does instead

Hoard captures **after you stop playing**, once the folder goes quiet, so a snapshot is never a half-written file. Each capture is a version of the whole save, not of individual files, so restoring is one click and puts everything back together. It knows which folder belongs to which game — reading the same community save-location manifest the open-source ecosystem shares, covering 20,000+ titles — so there are no paths to maintain, and it tracks \`<AppID>/remote/\` rather than the folder above it.

## When Syncthing is the better answer

Being fair about it:

- **You already run it**, and adding a folder is free.
- **You want peer-to-peer with no server at all**, not even your own.
- **You're syncing much more than saves** and would rather have one tool for everything.
- **You never roll back.** If the latest save is all you've ever needed, a version history is machinery you won't use.

## Using both

They coexist without a fight, and it's a reasonable setup: let the general syncer handle your documents and whatever else, and let a save-aware tool handle the save folders. The only rule is not to point both at the same folder — two tools writing the same files is how you manufacture the conflicts you were trying to avoid.

## Without our servers either

If part of the appeal is that nothing touches a company's disk, Hoard can be run the same way: \`hoard-server\` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us and no relay** — nothing passes through our servers, because there is nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same binary, same detection, same history. The only thing that changes is who owns the storage. There's also a full [comparison of every save sync tool](/guides/game-save-sync-comparison).

<!-- faq -->

## Frequently asked questions

### Can Syncthing sync game saves at all?

Yes, and for simple cases it does it fine. The trouble starts with games that write while you play, saves made of several files, and any setup where both machines get edited between syncs.

### What are the .sync-conflict files in my save folder?

That's the syncer keeping both versions after a conflict instead of choosing one. Nothing is lost, but the game can't read them, and deciding which to keep is manual work every time.

### Why does my Steam save conflict on every launch?

Almost always because the synced folder is the one above \`remote/\`. It contains \`remotecache.vdf\` and achievement and playtime files that legitimately differ per machine, so the two ends never agree.

### Do I need to close the game before syncing?

With a general-purpose syncer, yes — that's the habit that prevents half-written saves. A save-aware tool waits for the folder to go quiet on its own.

### Can I keep using both together?

Yes. Just don't point both at the same folder, or the two of them will fight over the same files.
`,Gn=`---
title: "Syncthing para partidas guardadas: qué funciona y qué se rompe"
description: "Syncthing es un gran sincronizador, pero las partidas rompen tres de sus supuestos. Qué falla, cómo se esquiva y cuándo encaja mejor una herramienta de saves."
order: 9
updated: 2026-09-01
---

Syncthing es la respuesta a la que mucha gente llega primero, y con razón: es gratis, open source, punto a punto, y funciona. Pero las partidas guardadas rompen tres de los supuestos sobre los que se construye un sincronizador de ficheros genérico, y los fallos son silenciosos. Esta guía va de qué se rompe de verdad, y de cuándo merece la pena usar algo que sepa lo que es una partida.

## Por qué la gente acaba ahí

Es software genuinamente bueno. Sin cuenta, sin suscripción, tus ficheros no se quedan en el disco de ninguna empresa, y sincroniza cualquier cosa: documentos, fotos, una carpeta de partidas. Si ya lo tienes montado para otras cosas, apuntarlo a una carpeta de saves te cuesta treinta segundos. Ése es un argumento real, y en algunos montajes es el correcto.

## Las tres cosas que se rompen

**Sincroniza con el juego abierto.** Syncthing reacciona a que un fichero cambie, porque eso es lo correcto para un documento. Un juego escribe su partida en mitad de la sesión, a veces en varias pasadas, y un fichero pillado a medio escribir es un fichero que se propaga incompleto. La otra máquina se queda con una partida que el juego puede negarse a cargar.

**Los conflictos se convierten en ficheros, no en decisiones.** Cuando las dos máquinas cambian la misma partida, Syncthing hace lo seguro y conserva las dos, renombrando una a \`algo.sync-conflict-20260901-143022-ABCDEFG.sav\`. No se pierde nada, pero el juego no sabe qué es ese fichero y tú acabas comparando fechas en un explorador para decidir qué tarde de juego te quedas. Repítelo unas cuantas veces y la carpeta se llena de ficheros de conflicto que nadie se atreve a borrar.

**El versionado es por fichero, no por sesión.** Syncthing puede guardar copias viejas en \`.stversions\`, y eso es mejor que nada. Pero una partida suele ser varios ficheros que sólo tienen sentido juntos, y restaurar significa buscar a mano la fecha correcta de cada uno. No existe un «deja este juego como estaba el martes».

Y una cuarta, específica de Steam: si lo apuntas a \`userdata/<UserID>/<AppID>/\` en vez de a la carpeta \`remote/\` de dentro, también estás sincronizando \`remotecache.vdf\` y ficheros de logros y tiempo jugado que **deben** ser distintos entre máquinas. Entonces cada arranque parece un conflicto aunque no se haya movido ninguna partida. Es el motivo más común de que un montaje casero entre Steam Deck y sobremesa parezca estropeado.

## Lo que acabas construyendo

Nada de lo anterior es irresoluble. La gente lo apaña con patrones de exclusión por juego, una política de versionado, y la costumbre de cerrar el juego y esperar antes de tocar el otro PC. Funciona, y es un mantenimiento que te llevas de por vida: un juego nuevo son rutas nuevas, y el día que se te olvide esperar es el día que te enteras.

## Qué hace en su lugar una herramienta que entiende de partidas

Hoard captura **cuando dejas de jugar**, una vez que la carpeta se queda quieta, así que una instantánea nunca es un fichero a medio escribir. Cada captura es una versión de la partida entera, no de ficheros sueltos, así que restaurar es un clic y lo devuelve todo junto. Sabe qué carpeta es de qué juego — leyendo el mismo manifiesto comunitario de ubicaciones que comparte el ecosistema open source, con más de 20.000 títulos — así que no hay rutas que mantener, y rastrea \`<AppID>/remote/\` y no la carpeta de encima.

## Cuándo Syncthing es la mejor respuesta

Siendo justos:

- **Ya lo tienes corriendo**, y añadir una carpeta te sale gratis.
- **Quieres punto a punto sin servidor ninguno**, ni siquiera el tuyo.
- **Sincronizas mucho más que partidas** y prefieres una sola herramienta para todo.
- **Nunca vuelves atrás.** Si la última partida es todo lo que has necesitado, un historial de versiones es maquinaria que no vas a usar.

## Usar los dos

Conviven sin pelearse, y es un montaje razonable: que el sincronizador genérico se ocupe de tus documentos y de lo que sea, y que de las carpetas de partidas se ocupe una herramienta que las entienda. La única regla es no apuntar los dos a la misma carpeta: dos programas escribiendo los mismos ficheros es la forma de fabricar justo los conflictos que querías evitar.

## Sin nuestros servidores tampoco

Si parte del atractivo es que nada toque el disco de una empresa, Hoard se puede usar igual: \`hoard-server\` en tu propio PC o NAS, y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

El mismo binario, la misma detección, el mismo historial. Lo único que cambia es de quién es el almacenamiento. También hay una [comparativa de todas las herramientas de sincronización](/guides/game-save-sync-comparison).

<!-- faq -->

## Preguntas frecuentes

### ¿Syncthing sirve para sincronizar partidas?

Sí, y en casos sencillos lo hace bien. El problema empieza con juegos que escriben mientras juegas, partidas hechas de varios ficheros, y cualquier montaje donde las dos máquinas se editen entre sincronizaciones.

### ¿Qué son los ficheros .sync-conflict de mi carpeta de partidas?

Es el sincronizador conservando las dos versiones tras un conflicto en vez de elegir una. No se pierde nada, pero el juego no puede leerlos, y decidir cuál te quedas es trabajo manual cada vez.

### ¿Por qué mi partida de Steam da conflicto en cada arranque?

Casi siempre porque la carpeta sincronizada es la que está por encima de \`remote/\`. Contiene \`remotecache.vdf\` y ficheros de logros y tiempo jugado que son legítimamente distintos en cada máquina, así que los dos extremos nunca coinciden.

### ¿Tengo que cerrar el juego antes de sincronizar?

Con un sincronizador genérico, sí: ésa es la costumbre que evita las partidas a medio escribir. Una herramienta que entiende de saves espera sola a que la carpeta se quede quieta.

### ¿Puedo seguir usando los dos a la vez?

Sí. Sólo que no apuntes los dos a la misma carpeta, o se pelearán por los mismos ficheros.
`,En=`---
title: "Syncthing pour les sauvegardes de jeux : ce qui marche et ce qui casse"
description: "Syncthing est un excellent outil de synchro, mais les sauvegardes de jeu cassent trois de ses hypothèses. Ce qui casse, les parades, et quand changer d'outil."
order: 9
updated: 2026-09-01
---

Syncthing est la réponse vers laquelle beaucoup se tournent d'abord, et à raison : gratuit, open source, pair-à-pair, et ça marche. Mais les sauvegardes de jeux brisent trois hypothèses sur lesquelles repose un synchroniseur généraliste, et les échecs sont silencieux. Ce guide parle de ce qui déraille vraiment, et du moment où un outil qui sait ce qu'est une sauvegarde devient utile.

## Pourquoi on y arrive

C'est un très bon logiciel. Pas de compte, pas d'abonnement, vos fichiers ne dorment jamais sur le disque d'une entreprise, et il synchronise n'importe quoi : documents, photos, un dossier de sauvegardes. Si vous l'utilisez déjà pour autre chose, ajouter un dossier vous coûte trente secondes. C'est un argument réel, et pour certains montages c'est le bon.

## Les trois choses qui cassent

**Il synchronise pendant que le jeu tourne.** Syncthing réagit à la modification d'un fichier, ce qui est le comportement correct pour un document. Un jeu écrit sa sauvegarde en pleine session, parfois en plusieurs passes, et un fichier attrapé en cours d'écriture se propage à moitié fini. L'autre machine se retrouve avec une sauvegarde que le jeu peut refuser de charger.

**Les conflits deviennent des fichiers, pas des décisions.** Quand les deux machines modifient la même sauvegarde, Syncthing fait le choix sûr et garde les deux, en renommant l'une en \`truc.sync-conflict-20260901-143022-ABCDEFG.sav\`. Rien n'est perdu, mais le jeu ignore ce qu'est ce fichier, et vous voilà à comparer des horodatages dans un explorateur pour décider quel après-midi de jeu garder. Répétez quelques fois et le dossier se remplit de fichiers de conflit que personne n'ose supprimer.

**Le versionnage est par fichier, pas par session.** Syncthing peut garder d'anciennes copies dans \`.stversions\`, ce qui vaut mieux que rien. Mais une sauvegarde est souvent plusieurs fichiers qui n'ont de sens qu'ensemble, et restaurer signifie retrouver à la main le bon horodatage pour chacun. Il n'y a pas de « remets ce jeu comme il était mardi ».

Et un quatrième, propre à Steam : pointez-le sur \`userdata/<UserID>/<AppID>/\` au lieu du dossier \`remote/\` à l'intérieur, et vous synchronisez aussi \`remotecache.vdf\` ainsi que des fichiers de succès et de temps de jeu qui **doivent** différer d'une machine à l'autre. Chaque lancement ressemble alors à un conflit alors qu'aucune sauvegarde n'a bougé. C'est la raison la plus fréquente pour laquelle un montage maison entre Steam Deck et PC de bureau paraît cassé.

## Ce que vous finissez par construire

Rien de tout cela n'est insoluble. On s'en sort avec des motifs d'exclusion par jeu, une politique de versionnage, et l'habitude de fermer le jeu et d'attendre avant de toucher l'autre PC. Ça marche, et c'est un entretien qui vous appartient pour toujours : un nouveau jeu, ce sont de nouveaux chemins, et le jour où vous oubliez d'attendre est le jour où vous l'apprenez.

## Ce que fait à la place un outil qui connaît les sauvegardes

Hoard capture **après que vous avez arrêté de jouer**, une fois le dossier calmé : un instantané n'est donc jamais un fichier à moitié écrit. Chaque capture est une version de la sauvegarde entière, pas de fichiers isolés, donc restaurer se fait en un clic et remet tout ensemble. Il sait quel dossier appartient à quel jeu — il lit le même manifeste communautaire d'emplacements que partage l'écosystème open source, couvrant plus de 20 000 titres — donc aucun chemin à maintenir, et il suit \`<AppID>/remote/\` plutôt que le dossier au-dessus.

## Quand Syncthing est la meilleure réponse

Pour être juste :

- **Vous l'utilisez déjà**, et ajouter un dossier est gratuit.
- **Vous voulez du pair-à-pair sans aucun serveur**, pas même le vôtre.
- **Vous synchronisez bien plus que des sauvegardes** et préférez un seul outil pour tout.
- **Vous ne revenez jamais en arrière.** Si la dernière sauvegarde vous a toujours suffi, un historique de versions est une mécanique que vous n'utiliserez pas.

## Utiliser les deux

Ils cohabitent sans se battre, et c'est un montage raisonnable : le synchroniseur généraliste s'occupe de vos documents et du reste, un outil qui connaît les sauvegardes s'occupe des dossiers de sauvegarde. La seule règle : ne pointez pas les deux sur le même dossier — deux programmes qui écrivent les mêmes fichiers, c'est fabriquer exactement les conflits que vous vouliez éviter.

## Sans nos serveurs non plus

Si une partie de l'attrait est que rien ne touche le disque d'une entreprise, Hoard se prête au même usage : \`hoard-server\` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. **Aucun compte chez nous, aucune télémétrie vers nous, aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

Le même binaire, la même détection, le même historique. La seule chose qui change, c'est à qui appartient le stockage. Il existe aussi une [comparaison complète des outils de synchro](/guides/game-save-sync-comparison).

<!-- faq -->

## Questions fréquentes

### Syncthing peut-il synchroniser des sauvegardes de jeux ?

Oui, et pour les cas simples il le fait très bien. Les ennuis commencent avec les jeux qui écrivent pendant que vous jouez, les sauvegardes faites de plusieurs fichiers, et tout montage où les deux machines sont modifiées entre deux synchros.

### Que sont les fichiers .sync-conflict dans mon dossier de sauvegardes ?

C'est le synchroniseur qui garde les deux versions après un conflit au lieu d'en choisir une. Rien n'est perdu, mais le jeu ne sait pas les lire, et décider laquelle garder est un travail manuel à chaque fois.

### Pourquoi ma sauvegarde Steam entre-t-elle en conflit à chaque lancement ?

Presque toujours parce que le dossier synchronisé est celui au-dessus de \`remote/\`. Il contient \`remotecache.vdf\` et des fichiers de succès et de temps de jeu qui diffèrent légitimement selon la machine : les deux bouts ne seront jamais d'accord.

### Dois-je fermer le jeu avant de synchroniser ?

Avec un synchroniseur généraliste, oui : c'est l'habitude qui évite les sauvegardes à moitié écrites. Un outil qui connaît les sauvegardes attend tout seul que le dossier se calme.

### Puis-je continuer à utiliser les deux ?

Oui. Ne les pointez simplement pas sur le même dossier, sinon ils se disputeront les mêmes fichiers.
`,Mn=`---
title: "Syncthing per i salvataggi: cosa funziona e cosa si rompe"
description: "Syncthing è un ottimo sincronizzatore, ma i salvataggi rompono tre sue ipotesi. Cosa va storto, come aggirarlo e quando serve uno strumento dedicato."
order: 9
updated: 2026-09-01
---

Syncthing è la risposta a cui molti arrivano per primi, e per buone ragioni: è gratuito, open source, peer-to-peer e funziona. Ma i salvataggi infrangono tre presupposti su cui si regge un sincronizzatore generico, e i guasti sono silenziosi. Questa guida parla di cosa va storto davvero, e di quando vale la pena usare qualcosa che sappia cos'è un salvataggio.

## Perché ci si finisce

È software genuinamente buono. Nessun account, nessun abbonamento, i tuoi file non stanno mai sul disco di un'azienda, e sincronizza qualsiasi cosa: documenti, foto, una cartella di salvataggi. Se già lo usi per altro, puntarlo a una cartella di salvataggi ti costa trenta secondi. È un argomento vero, e per certi setup è quello giusto.

## Le tre cose che si rompono

**Sincronizza mentre il gioco è aperto.** Syncthing reagisce al cambiamento di un file, che è il comportamento corretto per un documento. Un gioco scrive il salvataggio a metà sessione, a volte in più passaggi, e un file colto durante la scrittura è un file che si propaga a metà. L'altra macchina si ritrova un salvataggio che il gioco può rifiutarsi di caricare.

**I conflitti diventano file, non decisioni.** Quando entrambe le macchine cambiano lo stesso salvataggio, Syncthing fa la cosa sicura e li tiene entrambi, rinominandone uno in \`qualcosa.sync-conflict-20260901-143022-ABCDEFG.sav\`. Non si perde nulla, ma il gioco non sa cosa sia quel file, e tu finisci a confrontare date in un gestore file per decidere quale pomeriggio di gioco tenere. Ripetilo qualche volta e la cartella si riempie di file di conflitto che nessuno osa cancellare.

**Il versionamento è per file, non per sessione.** Syncthing può conservare copie vecchie in \`.stversions\`, ed è meglio di niente. Ma un salvataggio è spesso fatto di più file che hanno senso solo insieme, e ripristinare significa trovare a mano la data giusta per ciascuno. Non esiste un "rimetti questo gioco com'era martedì".

E un quarto punto, specifico di Steam: se lo punti a \`userdata/<UserID>/<AppID>/\` invece che alla cartella \`remote/\` al suo interno, stai sincronizzando anche \`remotecache.vdf\` e i file di obiettivi e tempo di gioco che **devono** essere diversi tra le macchine. A quel punto ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso. È il motivo più comune per cui un setup artigianale tra Steam Deck e desktop sembra rotto.

## Cosa finisci per costruire

Niente di tutto ciò è irrisolvibile. Ci si arrangia con pattern di esclusione per gioco, una politica di versionamento e l'abitudine di chiudere il gioco e aspettare prima di toccare l'altro PC. Funziona, ed è manutenzione che ti porti dietro per sempre: un gioco nuovo sono percorsi nuovi, e il giorno in cui dimentichi di aspettare è il giorno in cui lo scopri.

## Cosa fa invece uno strumento che conosce i salvataggi

Hoard cattura **dopo che hai smesso di giocare**, quando la cartella si è calmata, quindi uno snapshot non è mai un file scritto a metà. Ogni cattura è una versione dell'intero salvataggio, non dei singoli file, quindi ripristinare è un clic e rimette tutto insieme. Sa quale cartella appartiene a quale gioco — legge lo stesso manifest comunitario delle posizioni condiviso dall'ecosistema open source, oltre 20.000 titoli — quindi non ci sono percorsi da mantenere, e traccia \`<AppID>/remote/\` invece della cartella superiore.

## Quando Syncthing è la risposta migliore

Per essere onesti:

- **Lo hai già in funzione**, e aggiungere una cartella è gratis.
- **Vuoi peer-to-peer senza alcun server**, nemmeno il tuo.
- **Sincronizzi molto più dei salvataggi** e preferisci un solo strumento per tutto.
- **Non torni mai indietro.** Se l'ultimo salvataggio è tutto ciò che ti è servito, una cronologia è macchinario che non userai.

## Usarli entrambi

Convivono senza litigare, ed è un setup ragionevole: il sincronizzatore generico si occupa dei documenti e del resto, uno strumento che conosce i salvataggi si occupa delle cartelle di salvataggio. L'unica regola è non puntarli entrambi alla stessa cartella: due programmi che scrivono gli stessi file sono il modo di fabbricare proprio i conflitti che volevi evitare.

## Nemmeno dai nostri server

Se parte dell'attrattiva è che nulla tocchi il disco di un'azienda, Hoard si può usare allo stesso modo: \`hoard-server\` sul tuo PC o NAS, e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

Stesso binario, stesso rilevamento, stessa cronologia. L'unica cosa che cambia è di chi è lo spazio di archiviazione. C'è anche un [confronto completo di tutti gli strumenti di sincronizzazione](/guides/game-save-sync-comparison).

<!-- faq -->

## Domande frequenti

### Syncthing può sincronizzare i salvataggi?

Sì, e nei casi semplici lo fa bene. I problemi iniziano con i giochi che scrivono mentre giochi, con i salvataggi fatti di più file e con qualsiasi situazione in cui entrambe le macchine vengano modificate tra una sincronizzazione e l'altra.

### Cosa sono i file .sync-conflict nella mia cartella dei salvataggi?

È il sincronizzatore che dopo un conflitto tiene entrambe le versioni invece di sceglierne una. Non si perde nulla, ma il gioco non sa leggerli, e decidere quale tenere è lavoro manuale ogni volta.

### Perché il mio salvataggio Steam va in conflitto a ogni avvio?

Quasi sempre perché la cartella sincronizzata è quella sopra \`remote/\`. Contiene \`remotecache.vdf\` e file di obiettivi e tempo di gioco che sono legittimamente diversi su ogni macchina, quindi i due capi non andranno mai d'accordo.

### Devo chiudere il gioco prima di sincronizzare?

Con un sincronizzatore generico sì: è l'abitudine che evita i salvataggi scritti a metà. Uno strumento che conosce i salvataggi aspetta da solo che la cartella si calmi.

### Posso continuare a usarli insieme?

Sì. Solo, non puntarli entrambi alla stessa cartella, o si contenderanno gli stessi file.
`,Wn=`---
title: "Syncthing でセーブデータを同期する：うまくいく点と壊れる点"
description: "Syncthingは優れた同期ツールですが、セーブデータはその3つの前提を崩します。何が起きるか、回避策、専用ツールが向く場面を解説。"
order: 9
updated: 2026-09-01
---

Syncthing は多くの人が最初にたどり着く答えで、それには十分な理由があります。無料、オープンソース、ピアツーピアで、ちゃんと動きます。しかしゲームのセーブは、汎用のファイル同期が前提にしていることを 3 つ壊します。しかも壊れ方が静かです。このガイドでは、実際に何が起きるのか、そしてセーブを理解したツールを使う価値が出るのはいつかを扱います。

## なぜそこにたどり着くのか

本当に良いソフトウェアだからです。アカウントも購読もなく、ファイルが企業のディスクに置かれることもなく、何でも同期できます。書類でも、写真でも、セーブのフォルダーでも。すでに別の用途で動かしているなら、セーブのフォルダーを足すのは 30 秒の作業です。これは本物の利点で、構成によってはそれが正解です。

## 壊れる 3 つのこと

**ゲームが動いている最中に同期します。** Syncthing はファイルが変わったことに反応します。書類にとってはそれが正しい振る舞いです。ゲームはセッションの途中で、ときには複数回に分けてセーブを書きます。書き込み途中で捕まえられたファイルは、半端なまま伝播します。もう 1 台には、ゲームが読み込みを拒むかもしれないセーブが残ります。

**競合が「判断」ではなく「ファイル」になります。** 両方のマシンが同じセーブを変更すると、Syncthing は安全側に倒して両方を残し、片方を \`something.sync-conflict-20260901-143022-ABCDEFG.sav\` のような名前に変えます。失われるものはありませんが、ゲームはそのファイルが何なのかを知りません。結局、ファイルマネージャーで日時を見比べて、どちらのプレイを残すか決めることになります。何度か繰り返せば、フォルダーは誰も消す勇気のない競合ファイルで埋まります。

**世代管理はファイル単位で、セッション単位ではありません。** Syncthing は古いコピーを \`.stversions\` に残せます。何もないよりはずっと良い。ただ、セーブはしばしば複数のファイルがそろって初めて意味を持ちます。復元するには、それぞれについて正しい日時を手で探す必要があります。「このゲームを火曜の状態に戻す」に当たるものはありません。

そして 4 つ目、Steam に特有の点です。中の \`remote/\` ではなく \`userdata/<UserID>/<AppID>/\` を指定すると、\`remotecache.vdf\` や、マシンごとに **違って当然の** 実績・プレイ時間のファイルまで同期されます。こうなると、セーブが動いていなくても起動のたびに競合に見えます。自作の Steam Deck とデスクトップの構成が壊れているように感じられる、いちばん多い原因がこれです。

## 結局あなたが組み立てるもの

以上はどれも解決不能ではありません。ゲームごとの除外パターン、世代管理の方針、そして「ゲームを閉じて少し待ってから、もう 1 台に触る」という習慣で、みんな回しています。それで動きますし、その保守はずっとあなたのものです。ゲームが増えればパスが増え、待つのを忘れた日にそれを知ることになります。

## セーブを理解したツールは代わりに何をするか

Hoard は **プレイを終えたあと**、フォルダーが静かになってから取り込みます。だからスナップショットが書き込み途中のファイルになることはありません。取り込みの単位は個々のファイルではなくセーブ全体の 1 世代なので、復元はワンクリックで、まとめて元に戻ります。どのフォルダーがどのゲームのものかも把握しています。オープンソースの世界で共有されている、2 万本以上を収録した同じコミュニティのセーブ位置マニフェストを読むためです。保守するパスはなく、上のフォルダーではなく \`<AppID>/remote/\` を追跡します。

## Syncthing のほうが良い場合

公平に書いておきます。

- **すでに動かしている。** フォルダーを 1 つ足すのは無料です。
- **サーバーをまったく置きたくない。** 自分のものすら含めて、ピアツーピアで済ませたい。
- **セーブ以外もたくさん同期している。** 何でも 1 つのツールで済ませたい。
- **巻き戻したことがない。** 最新のセーブで足りてきたのなら、世代履歴は使わない仕掛けです。

## 両方を使う

両者は衝突せず、これは理にかなった構成です。汎用の同期には書類やその他を任せ、セーブのフォルダーはセーブを理解したツールに任せる。唯一の注意は、両方を同じフォルダーに向けないこと。同じファイルを 2 つのプログラムが書けば、避けたかった競合を自分で作り出すことになります。

## 当方のサーバーも通さずに

「企業のディスクに何も触れさせない」ことが魅力の一部なら、Hoard も同じように使えます。自分の PC や NAS で \`hoard-server\` を動かせば、セーブは自分のマシンから自分のディスクへ移ります。**当方のアカウントも、当方へのテレメトリも、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

同じバイナリ、同じ検出、同じ履歴。変わるのは保存先が誰のものかだけです。[セーブ同期ツールの比較](/guides/game-save-sync-comparison) もあります。

<!-- faq -->

## よくある質問

### Syncthing でセーブデータは同期できますか？

できますし、単純なケースなら問題なく動きます。困り始めるのは、プレイ中に書き込むゲーム、複数ファイルで構成されるセーブ、そして同期のあいだに両方のマシンが編集される構成です。

### セーブのフォルダーにある .sync-conflict ファイルは何ですか？

競合が起きたときに、どちらかを選ばず両方を残した結果です。失われるものはありませんが、ゲームはそれを読めず、どちらを残すかの判断は毎回手作業になります。

### Steam のセーブが起動のたびに競合するのはなぜですか？

ほぼ確実に、同期しているフォルダーが \`remote/\` の 1 つ上だからです。そこには \`remotecache.vdf\` や、マシンごとに違って当然の実績・プレイ時間のファイルが入っているため、両端が一致することはありません。

### 同期の前にゲームを閉じる必要はありますか？

汎用の同期なら必要です。それが書き込み途中のセーブを防ぐ習慣になります。セーブを理解したツールは、フォルダーが静かになるまで自分で待ちます。

### 両方を併用し続けられますか？

はい。ただし両方を同じフォルダーに向けないでください。同じファイルを取り合うことになります。
`,In=`---
title: "Syncthing para saves de jogos: o que funciona e o que parte"
description: "O Syncthing é um ótimo sincronizador, mas os saves quebram três das suas premissas. O que corre mal, como contornar e quando usar uma ferramenta de saves."
order: 9
updated: 2026-09-01
---

O Syncthing é a resposta a que muita gente chega primeiro, e com razão: é gratuito, open source, ponto a ponto, e funciona. Mas os saves de jogos partem três dos pressupostos em que assenta um sincronizador genérico, e as falhas são silenciosas. Este guia é sobre o que corre mal a sério, e sobre quando vale a pena usar algo que saiba o que é um save.

## Porque se acaba aí

É software genuinamente bom. Sem conta, sem subscrição, os teus ficheiros nunca ficam no disco de uma empresa, e sincroniza qualquer coisa: documentos, fotos, uma pasta de saves. Se já o tens a correr para outras coisas, apontá-lo a uma pasta de saves custa-te trinta segundos. É um argumento real, e para certas montagens é o correto.

## As três coisas que partem

**Sincroniza com o jogo aberto.** O Syncthing reage à alteração de um ficheiro, que é o comportamento certo para um documento. Um jogo escreve o save a meio da sessão, às vezes em várias passagens, e um ficheiro apanhado a meio da escrita propaga-se incompleto. A outra máquina fica com um save que o jogo pode recusar carregar.

**Os conflitos tornam-se ficheiros, não decisões.** Quando ambas as máquinas mudam o mesmo save, o Syncthing faz o seguro e guarda os dois, renomeando um para \`algo.sync-conflict-20260901-143022-ABCDEFG.sav\`. Não se perde nada, mas o jogo não sabe o que é esse ficheiro, e ficas a comparar datas num explorador para decidir que tarde de jogo manténs. Repete umas quantas vezes e a pasta enche-se de ficheiros de conflito que ninguém se atreve a apagar.

**O versionamento é por ficheiro, não por sessão.** O Syncthing pode guardar cópias antigas em \`.stversions\`, e é melhor do que nada. Mas um save é muitas vezes vários ficheiros que só fazem sentido juntos, e restaurar significa encontrar à mão a data certa de cada um. Não existe um "põe este jogo como estava na terça".

E um quarto ponto, específico da Steam: se o apontares a \`userdata/<UserID>/<AppID>/\` em vez da pasta \`remote/\` lá dentro, também estás a sincronizar \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo que **devem** ser diferentes entre máquinas. A partir daí cada arranque parece um conflito mesmo sem nenhum save se ter mexido. É o motivo mais comum para uma montagem caseira entre Steam Deck e desktop parecer avariada.

## O que acabas por construir

Nada disto é insolúvel. As pessoas safam-se com padrões de exclusão por jogo, uma política de versionamento, e o hábito de fechar o jogo e esperar antes de tocar no outro PC. Funciona, e é manutenção que passa a ser tua para sempre: um jogo novo são caminhos novos, e o dia em que te esqueces de esperar é o dia em que dás por isso.

## O que faz em vez disso uma ferramenta que percebe de saves

O Hoard captura **quando paras de jogar**, assim que a pasta fica quieta, por isso um snapshot nunca é um ficheiro escrito a meio. Cada captura é uma versão do save inteiro, e não de ficheiros soltos, por isso restaurar é um clique e devolve tudo junto. Sabe que pasta é de que jogo — lê o mesmo manifesto comunitário de localizações que o ecossistema open source partilha, com mais de 20.000 títulos — por isso não há caminhos para manter, e segue \`<AppID>/remote/\` em vez da pasta acima.

## Quando o Syncthing é a melhor resposta

Sendo justos:

- **Já o tens a correr**, e acrescentar uma pasta sai grátis.
- **Queres ponto a ponto sem servidor nenhum**, nem sequer o teu.
- **Sincronizas muito mais do que saves** e preferes uma só ferramenta para tudo.
- **Nunca voltas atrás.** Se o último save é tudo o que alguma vez precisaste, um histórico de versões é maquinaria que não vais usar.

## Usar os dois

Convivem sem se atropelar, e é uma montagem razoável: o sincronizador genérico trata dos teus documentos e do resto, e das pastas de saves trata uma ferramenta que as perceba. A única regra é não apontar os dois à mesma pasta — dois programas a escrever os mesmos ficheiros é a forma de fabricar precisamente os conflitos que querias evitar.

## Sem os nossos servidores também

Se parte do apelo é que nada toque no disco de uma empresa, o Hoard pode ser usado da mesma forma: \`hoard-server\` no teu PC ou NAS, e os teus saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo binário, a mesma deteção, o mesmo histórico. A única coisa que muda é de quem é o armazenamento. Há também uma [comparação completa de todas as ferramentas de sincronização](/guides/game-save-sync-comparison).

<!-- faq -->

## Perguntas frequentes

### O Syncthing consegue sincronizar saves de jogos?

Consegue, e em casos simples fá-lo bem. Os problemas começam com jogos que escrevem enquanto jogas, saves feitos de vários ficheiros, e qualquer montagem em que as duas máquinas sejam editadas entre sincronizações.

### O que são os ficheiros .sync-conflict na minha pasta de saves?

É o sincronizador a guardar as duas versões depois de um conflito, em vez de escolher uma. Não se perde nada, mas o jogo não os consegue ler, e decidir qual ficar é trabalho manual de cada vez.

### Porque é que o meu save da Steam dá conflito a cada arranque?

Quase sempre porque a pasta sincronizada é a que está acima de \`remote/\`. Contém \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo que são legitimamente diferentes em cada máquina, por isso as duas pontas nunca coincidem.

### Tenho de fechar o jogo antes de sincronizar?

Com um sincronizador genérico, sim: é esse o hábito que evita saves escritos a meio. Uma ferramenta que percebe de saves espera sozinha que a pasta fique quieta.

### Posso continuar a usar os dois?

Sim. Só não apontes os dois à mesma pasta, ou vão andar à luta pelos mesmos ficheiros.
`,Rn=`---
title: "用 Syncthing 同步游戏存档：哪些可行，哪些会坏"
description: "Syncthing 是出色的文件同步工具，但游戏存档会打破它的三个前提。会出什么问题、如何绕过，以及何时更适合用专门的存档工具。"
order: 9
updated: 2026-09-01
---

Syncthing 是很多人最先想到的答案，理由也很充分：免费、开源、点对点，而且确实好用。但游戏存档打破了通用同步工具赖以成立的三个前提，而且失败得很安静。本文讲的是实际会出什么问题，以及什么时候值得换一个懂存档是什么的工具。

## 为什么大家会走到这一步

它确实是好软件。没有账号，没有订阅，你的文件从不停留在某家公司的磁盘上，而且什么都能同步：文档、照片、一个存档文件夹。如果你本来就在用它做别的事，再加一个文件夹只花三十秒。这是实打实的理由，对某些配置来说也是正确的选择。

## 会坏掉的三件事

**它会在游戏运行时同步。** Syncthing 对"文件发生变化"作出反应，对文档而言这完全正确。但游戏是在游玩过程中写存档的，有时还分几次写，一个在写入途中被抓到的文件，会以残缺的状态传过去。另一台机器于是拿到一个游戏可能拒绝读取的存档。

**冲突变成了文件，而不是决定。** 当两台机器都改了同一个存档，Syncthing 会做安全的事——两个都留下，把其中一个重命名为 \`something.sync-conflict-20260901-143022-ABCDEFG.sav\`。什么都没丢，但游戏不知道那个文件是什么，于是你只能在文件管理器里比对时间戳，决定保留哪一个下午的游玩。重复几次，文件夹里就堆满了没人敢删的冲突文件。

**版本是按文件算的，不是按一次游玩算的。** Syncthing 可以把旧副本留在 \`.stversions\` 里，这比没有强。但一个存档往往由多个只有凑在一起才有意义的文件组成，恢复就意味着为每一个手动找出正确的时间点。并不存在"把这个游戏恢复到周二的样子"。

还有第四点，是 Steam 独有的：如果你指的是 \`userdata/<UserID>/<AppID>/\` 而不是里面的 \`remote/\`，那你连 \`remotecache.vdf\` 以及那些**本就应该**因机器而异的成就和游戏时长文件也一起同步了。于是每次启动都像冲突，尽管没有任何存档动过。这正是手工搭建的 Steam Deck 与台式机方案让人觉得"坏掉了"的最常见原因。

## 你最后会自己搭出什么

上面这些都不是无解的。大家用逐个游戏的排除规则、一套版本策略，以及"先关游戏、等一会儿再碰另一台 PC"的习惯来应付。这行得通，而这份维护从此归你所有：多一款游戏就是多几条路径，而你忘记等待的那一天，就是你发现问题的那一天。

## 一个懂存档的工具会怎么做

Hoard 在**你停止游玩之后**、文件夹安静下来时才抓取，所以快照永远不会是写到一半的文件。每次抓取都是整个存档的一个版本，而不是单个文件的版本，因此还原只需一次点击，并且是整体复原。它知道哪个文件夹属于哪款游戏——读取开源生态共享的同一份社区存档位置清单，覆盖两万余款游戏——所以没有路径需要你维护，而且它追踪的是 \`<AppID>/remote/\`，不是它上一层。

## 什么时候 Syncthing 才是更好的答案

公平地说：

- **你本来就在跑它**，再加一个文件夹是白得的。
- **你想要完全没有服务器的点对点**，连自己的也不要。
- **你同步的远不止存档**，宁愿一个工具管所有事。
- **你从不回退。** 如果最新存档一直就够用，版本历史就是你用不上的机械。

## 两个一起用

它们可以共存，而且这是个合理的配置：通用同步负责文档和其他一切，懂存档的工具负责存档文件夹。唯一的原则是别把两者指向同一个文件夹——两个程序写同一批文件，正是在亲手制造你想避免的冲突。

## 也不经过我们的服务器

如果吸引你的一部分正是"不让任何东西碰到公司的磁盘"，Hoard 也可以这样用：在自己的 PC 或 NAS 上运行 \`hoard-server\`，存档就从你的机器走到你的磁盘。**没有我们这边的账号，没有发往我们的遥测，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。参见[如何自托管 Hoard](/guides/self-host-hoard)。

同一个二进制、同样的检测、同样的历史。唯一变化的是存储归谁所有。也可以看[所有存档同步工具的完整比较](/guides/game-save-sync-comparison)。

<!-- faq -->

## 常见问题

### Syncthing 到底能不能同步游戏存档？

能，简单场景下也做得不错。麻烦出在会在游玩过程中写入的游戏、由多个文件组成的存档，以及两台机器在两次同步之间都被改动过的情况。

### 我存档文件夹里的 .sync-conflict 文件是什么？

那是同步工具在冲突后保留了两个版本，而不是替你选一个。什么都没丢，但游戏读不了它们，而且每次都得靠你手动决定留哪个。

### 为什么我的 Steam 存档每次启动都冲突？

几乎总是因为被同步的是 \`remote/\` 的上一层文件夹。它包含 \`remotecache.vdf\` 以及本就应该因机器而异的成就和游戏时长文件，所以两端永远达不成一致。

### 同步前必须先关掉游戏吗？

用通用同步工具的话，是的——正是这个习惯避免了写到一半的存档。懂存档的工具会自己等到文件夹安静下来。

### 我可以两个继续一起用吗？

可以。只是别把两者指向同一个文件夹，否则它们会为同一批文件打架。
`;function X(){return{async:!1,breaks:!1,extensions:null,gfm:!0,hooks:null,pedantic:!1,renderer:null,silent:!1,tokenizer:null,walkTokens:null}}var L=X();function me(n){L=n}var w={exec:()=>null};function H(n){let e=[];return a=>{let s=Math.max(0,Math.min(3,a-1)),o=e[s];return o||(o=n(s),e[s]=o),o}}function h(n,e=""){let a=typeof n=="string"?n:n.source,s={replace:(o,i)=>{let t=typeof i=="string"?i:i.source;return t=t.replace(b.caret,"$1"),a=a.replace(o,t),s},getRegex:()=>new RegExp(a,e)};return s}var _n=((n="")=>{try{return!!new RegExp("(?<=1)(?<!1)"+n)}catch{return!1}})(),b={codeRemoveIndent:/^(?: {1,4}| {0,3}\t)/gm,outputLinkReplace:/\\([\[\]])/g,indentCodeCompensation:/^(\s+)(?:```)/,beginningSpace:/^\s+/,endingHash:/#$/,startingSpaceChar:/^ /,endingSpaceChar:/ $/,nonSpaceChar:/[^ ]/,newLineCharGlobal:/\n/g,tabCharGlobal:/\t/g,multipleSpaceGlobal:/\s+/g,blankLine:/^[ \t]*$/,doubleBlankLine:/\n[ \t]*\n[ \t]*$/,blockquoteStart:/^ {0,3}>/,blockquoteSetextReplace:/\n {0,3}((?:=+|-+) *)(?=\n|$)/g,blockquoteSetextReplace2:/^ {0,3}>[ \t]?/gm,listReplaceNesting:/^ {1,4}(?=( {4})*[^ ])/g,listIsTask:/^\[[ xX]\] +\S/,listReplaceTask:/^\[[ xX]\] +/,listTaskCheckbox:/\[[ xX]\]/,anyLine:/\n.*\n/,hrefBrackets:/^<(.*)>$/,tableDelimiter:/[:|]/,tableAlignChars:/^\||\| *$/g,tableRowBlankLine:/\n[ \t]*$/,tableAlignRight:/^ *-+: *$/,tableAlignCenter:/^ *:-+: *$/,tableAlignLeft:/^ *:-+ *$/,startATag:/^<a /i,endATag:/^<\/a>/i,startPreScriptTag:/^<(pre|code|kbd|script)(\s|>)/i,endPreScriptTag:/^<\/(pre|code|kbd|script)(\s|>)/i,startAngleBracket:/^</,endAngleBracket:/>$/,pedanticHrefTitle:/^([^'"]*[^\s])\s+(['"])(.*)\2/,unicodeAlphaNumeric:/[\p{L}\p{N}]/u,escapeTest:/[&<>"']/,escapeReplace:/[&<>"']/g,escapeTestNoEncode:/[<>"']|&(?!(#\d{1,7}|#[Xx][a-fA-F0-9]{1,6}|\w+);)/,escapeReplaceNoEncode:/[<>"']|&(?!(#\d{1,7}|#[Xx][a-fA-F0-9]{1,6}|\w+);)/g,caret:/(^|[^\[])\^/g,percentDecode:/%25/g,findPipe:/\|/g,splitPipe:/ \|/,slashPipe:/\\\|/g,carriageReturn:/\r\n|\r/g,spaceLine:/^ +$/gm,notSpaceStart:/^\S*/,endingNewline:/\n$/,listItemRegex:n=>new RegExp(`^( {0,3}${n})((?:[	 ][^\\n]*)?(?:\\n|$))`),nextBulletRegex:H(n=>new RegExp(`^ {0,${n}}(?:[*+-]|\\d{1,9}[.)])((?:[ 	][^\\n]*)?(?:\\n|$))`)),hrRegex:H(n=>new RegExp(`^ {0,${n}}((?:- *){3,}|(?:_ *){3,}|(?:\\* *){3,})(?:\\n+|$)`)),fencesBeginRegex:H(n=>new RegExp(`^ {0,${n}}(?:\`\`\`|~~~)`)),headingBeginRegex:H(n=>new RegExp(`^ {0,${n}}#`)),htmlBeginRegex:H(n=>new RegExp(`^ {0,${n}}<(?:[a-z].*>|!--)`,"i")),blockquoteBeginRegex:H(n=>new RegExp(`^ {0,${n}}>`))},Tn=/^(?:[ \t]*(?:\n|$))+/,Bn=/^((?: {4}| {0,3}\t)[^\n]+(?:\n(?:[ \t]*(?:\n|$))*)?)+/,Nn=/^ {0,3}(`{3,}(?=[^`\n]*(?:\n|$))|~{3,})([^\n]*)(?:\n|$)(?:|([\s\S]*?)(?:\n|$))(?: {0,3}\1[~`]* *(?=\n|$)|$)/,E=/^ {0,3}((?:-[\t ]*){3,}|(?:_[ \t]*){3,}|(?:\*[ \t]*){3,})(?:\n+|$)/,Vn=/^ {0,3}(#{1,6})(?=\s|$)(.*)(?:\n+|$)/,$=/ {0,3}(?:[*+-]|\d{1,9}[.)])/,pe=/^(?!bull |blockCode|fences|blockquote|heading|html|table)((?:.|\n(?!\s*?\n|bull |blockCode|fences|blockquote|heading|html|table))+?)\n {0,3}(=+|-+) *(?:\n+|$)/,he=h(pe).replace(/bull/g,$).replace(/blockCode/g,/(?: {4}| {0,3}\t)/).replace(/fences/g,/ {0,3}(?:`{3,}|~{3,})/).replace(/blockquote/g,/ {0,3}>/).replace(/heading/g,/ {0,3}#{1,6}/).replace(/html/g,/ {0,3}<[^\n>]+>\n/).replace(/\|table/g,"").getRegex(),Un=h(pe).replace(/bull/g,$).replace(/blockCode/g,/(?: {4}| {0,3}\t)/).replace(/fences/g,/ {0,3}(?:`{3,}|~{3,})/).replace(/blockquote/g,/ {0,3}>/).replace(/heading/g,/ {0,3}#{1,6}/).replace(/html/g,/ {0,3}<[^\n>]+>\n/).replace(/table/g,/ {0,3}\|?(?:[:\- ]*\|)+[\:\- ]*\n/).getRegex(),J=/^([^\n]+(?:\n(?!hr|heading|lheading|blockquote|fences|list|html|table| +\n)[^\n]+)*)/,Fn=/^[^\n]+/,Z=/(?!\s*\])(?:\\[\s\S]|[^\[\]\\])+/,Kn=h(/^ {0,3}\[(label)\]: *(?:\n[ \t]*)?([^<\s][^\s]*|<.*?>)(?:(?: +(?:\n[ \t]*)?| *\n[ \t]*)(title))? *(?:\n+|$)/).replace("label",Z).replace("title",/(?:"(?:\\"?|[^"\\])*"|'[^'\n]*(?:\n[^'\n]+)*\n?'|\([^()]*\))/).getRegex(),Qn=h(/^(bull)([ \t][^\n]*?)?(?:\n|$)/).replace(/bull/g,$).getRegex(),B="address|article|aside|base|basefont|blockquote|body|caption|center|col|colgroup|dd|details|dialog|dir|div|dl|dt|fieldset|figcaption|figure|footer|form|frame|frameset|h[1-6]|head|header|hr|html|iframe|legend|li|link|main|menu|menuitem|meta|nav|noframes|ol|optgroup|option|p|param|search|section|summary|table|tbody|td|tfoot|th|thead|title|tr|track|ul",Y=/<!--(?:-?>|[\s\S]*?(?:-->|$))/,Xn=h("^ {0,3}(?:<(script|pre|style|textarea)[\\s>][\\s\\S]*?(?:</\\1>[^\\n]*\\n+|$)|comment[^\\n]*(\\n+|$)|<\\?[\\s\\S]*?(?:\\?>\\n*|$)|<![A-Z][\\s\\S]*?(?:>\\n*|$)|<!\\[CDATA\\[[\\s\\S]*?(?:\\]\\]>\\n*|$)|</?(tag)(?: +|\\n|/?>)[\\s\\S]*?(?:(?:\\n[ 	]*)+\\n|$)|<(?!script|pre|style|textarea)([a-z][\\w-]*)(?:attribute)*? */?>(?=[ \\t]*(?:\\n|$))[\\s\\S]*?(?:(?:\\n[ 	]*)+\\n|$)|</(?!script|pre|style|textarea)[a-z][\\w-]*\\s*>(?=[ \\t]*(?:\\n|$))[\\s\\S]*?(?:(?:\\n[ 	]*)+\\n|$))","i").replace("comment",Y).replace("tag",B).replace("attribute",/ +[a-zA-Z:_][\w.:-]*(?: *= *"[^"\n]*"| *= *'[^'\n]*'| *= *[^\s"'=<>`]+)?/).getRegex(),ve=h(J).replace("hr",E).replace("heading"," {0,3}#{1,6}(?:\\s|$)").replace("|lheading","").replace("|table","").replace("blockquote"," {0,3}>").replace("fences"," {0,3}(?:`{3,}(?=[^`\\n]*\\n)|~{3,})[^\\n]*\\n").replace("list"," {0,3}(?:[*+-]|1[.)])[ \\t]+[^ \\t\\n]").replace("html","</?(?:tag)(?: +|\\n|/?>)|<(?:script|pre|style|textarea|!--)").replace("tag",B).getRegex(),$n=h(/^( {0,3}> ?(paragraph|[^\n]*)(?:\n|$))+/).replace("paragraph",ve).getRegex(),ee={blockquote:$n,code:Bn,def:Kn,fences:Nn,heading:Vn,hr:E,html:Xn,lheading:he,list:Qn,newline:Tn,paragraph:ve,table:w,text:Fn},se=h("^ *([^\\n ].*)\\n {0,3}((?:\\| *)?:?-+:? *(?:\\| *:?-+:? *)*(?:\\| *)?)(?:\\n((?:(?! *\\n|hr|heading|blockquote|code|fences|list|html).*(?:\\n|$))*)\\n*|$)").replace("hr",E).replace("heading"," {0,3}#{1,6}(?:\\s|$)").replace("blockquote"," {0,3}>").replace("code","(?: {4}| {0,3}	)[^\\n]").replace("fences"," {0,3}(?:`{3,}(?=[^`\\n]*\\n)|~{3,})[^\\n]*\\n").replace("list"," {0,3}(?:[*+-]|1[.)])[ \\t]").replace("html","</?(?:tag)(?: +|\\n|/?>)|<(?:script|pre|style|textarea|!--)").replace("tag",B).getRegex(),Jn={...ee,lheading:Un,table:se,paragraph:h(J).replace("hr",E).replace("heading"," {0,3}#{1,6}(?:\\s|$)").replace("|lheading","").replace("table",se).replace("blockquote"," {0,3}>").replace("fences"," {0,3}(?:`{3,}(?=[^`\\n]*\\n)|~{3,})[^\\n]*\\n").replace("list"," {0,3}(?:[*+-]|1[.)])[ \\t]+[^ \\t\\n]").replace("html","</?(?:tag)(?: +|\\n|/?>)|<(?:script|pre|style|textarea|!--)").replace("tag",B).getRegex()},Zn={...ee,html:h(`^ *(?:comment *(?:\\n|\\s*$)|<(tag)[\\s\\S]+?</\\1> *(?:\\n{2,}|\\s*$)|<tag(?:"[^"]*"|'[^']*'|\\s[^'"/>\\s]*)*?/?> *(?:\\n{2,}|\\s*$))`).replace("comment",Y).replace(/tag/g,"(?!(?:a|em|strong|small|s|cite|q|dfn|abbr|data|time|code|var|samp|kbd|sub|sup|i|b|u|mark|ruby|rt|rp|bdi|bdo|span|br|wbr|ins|del|img)\\b)\\w+(?!:|[^\\w\\s@]*@)\\b").getRegex(),def:/^ *\[([^\]]+)\]: *<?([^\s>]+)>?(?: +(["(][^\n]+[")]))? *(?:\n+|$)/,heading:/^(#{1,6})(.*)(?:\n+|$)/,fences:w,lheading:/^(.+?)\n {0,3}(=+|-+) *(?:\n+|$)/,paragraph:h(J).replace("hr",E).replace("heading",` *#{1,6} *[^
]`).replace("lheading",he).replace("|table","").replace("blockquote"," {0,3}>").replace("|fences","").replace("|list","").replace("|html","").replace("|tag","").getRegex()},Yn=/^\\([!"#$%&'()*+,\-./:;<=>?@\[\]\\^_`{|}~])/,eo=/^(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/,ge=/^( {2,}|\\)\n(?!\s*$)/,ao=/^(`+|[^`])(?:(?= {2,}\n)|[\s\S]*?(?:(?=[\\<!\[`*_]|\b_|$)|[^ ](?= {2,}\n)))/,j=/[\p{P}\p{S}]/u,N=/[\s\p{P}\p{S}]/u,ae=/[^\s\p{P}\p{S}]/u,no=h(/^((?![*_])punctSpace)/,"u").replace(/punctSpace/g,N).getRegex(),Se=/(?!~)[\p{P}\p{S}]/u,oo=/(?!~)[\s\p{P}\p{S}]/u,so=/(?:[^\s\p{P}\p{S}]|~)/u,io=h(/link|precode-code|html/,"g").replace("link",/\[(?:[^\[\]`]|(?<a>`+)[^`]+\k<a>(?!`))*?\]\((?:\\[\s\S]|[^\\\(\)]|\((?:\\[\s\S]|[^\\\(\)])*\))*\)/).replace("precode-",_n?"(?<!`)()":"(^^|[^`])").replace("code",/(?<b>`+)[^`]+\k<b>(?!`)/).replace("html",/<(?! )[^<>]*?>/).getRegex(),fe=/^(?:\*+(?:((?!\*)punct)|([^\s*]))?)|^_+(?:((?!_)punct)|([^\s_]))?/,ro=h(fe,"u").replace(/punct/g,j).getRegex(),to=h(fe,"u").replace(/punct/g,Se).getRegex(),be="^[^_*]*?__[^_*]*?\\*[^_*]*?(?=__)|[^*]+(?=[^*])|(?!\\*)punct(\\*+)(?=[\\s]|$)|notPunctSpace(\\*+)(?!\\*)(?=punctSpace|$)|(?!\\*)punctSpace(\\*+)(?=notPunctSpace)|[\\s](\\*+)(?!\\*)(?=punct)|(?!\\*)punct(\\*+)(?!\\*)(?=punct)|notPunctSpace(\\*+)(?=notPunctSpace)",uo=h(be,"gu").replace(/notPunctSpace/g,ae).replace(/punctSpace/g,N).replace(/punct/g,j).getRegex(),lo=h(be,"gu").replace(/notPunctSpace/g,so).replace(/punctSpace/g,oo).replace(/punct/g,Se).getRegex(),co=h("^[^_*]*?\\*\\*[^_*]*?_[^_*]*?(?=\\*\\*)|[^_]+(?=[^_])|(?!_)punct(_+)(?=[\\s]|$)|notPunctSpace(_+)(?!_)(?=punctSpace|$)|(?!_)punctSpace(_+)(?=notPunctSpace)|[\\s](_+)(?!_)(?=punct)|(?!_)punct(_+)(?!_)(?=punct)","gu").replace(/notPunctSpace/g,ae).replace(/punctSpace/g,N).replace(/punct/g,j).getRegex(),mo=h(/^~~?(?:((?!~)punct)|[^\s~])/,"u").replace(/punct/g,j).getRegex(),po="^[^~]+(?=[^~])|(?!~)punct(~~?)(?=[\\s]|$)|notPunctSpace(~~?)(?!~)(?=punctSpace|$)|(?!~)punctSpace(~~?)(?=notPunctSpace)|[\\s](~~?)(?!~)(?=punct)|(?!~)punct(~~?)(?!~)(?=punct)|notPunctSpace(~~?)(?=notPunctSpace)",ho=h(po,"gu").replace(/notPunctSpace/g,ae).replace(/punctSpace/g,N).replace(/punct/g,j).getRegex(),vo=h(/\\(punct)/,"gu").replace(/punct/g,j).getRegex(),go=h(/^<(scheme:[^\s\x00-\x1f<>]*|email)>/).replace("scheme",/[a-zA-Z][a-zA-Z0-9+.-]{1,31}/).replace("email",/[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+(@)[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+(?![-_])/).getRegex(),So=h(Y).replace("(?:-->|$)","-->").getRegex(),fo=h("^comment|^</[a-zA-Z][\\w:-]*\\s*>|^<[a-zA-Z][\\w-]*(?:attribute)*?\\s*/?>|^<\\?[\\s\\S]*?\\?>|^<![a-zA-Z]+\\s[\\s\\S]*?>|^<!\\[CDATA\\[[\\s\\S]*?\\]\\]>").replace("comment",So).replace("attribute",/\s+[a-zA-Z:_][\w.:-]*(?:\s*=\s*"[^"]*"|\s*=\s*'[^']*'|\s*=\s*[^\s"'=<>`]+)?/).getRegex(),R=/(?:\[(?:\\[\s\S]|[^\[\]\\])*\]|\\[\s\S]|`+(?!`)[^`]*?`+(?!`)|``+(?=\])|[^\[\]\\`])*?/,bo=h(/^!?\[(label)\]\(\s*(href)(?:(?:[ \t]+(?:\n[ \t]*)?|\n[ \t]*)(title))?\s*\)/).replace("label",R).replace("href",/<(?:\\.|[^\n<>\\])+>|[^ \t\n\x00-\x1f]*/).replace("title",/"(?:\\"?|[^"\\])*"|'(?:\\'?|[^'\\])*'|\((?:\\\)?|[^)\\])*\)/).getRegex(),ye=h(/^!?\[(label)\]\[(ref)\]/).replace("label",R).replace("ref",Z).getRegex(),ke=h(/^!?\[(ref)\](?:\[\])?/).replace("ref",Z).getRegex(),yo=h("reflink|nolink(?!\\()","g").replace("reflink",ye).replace("nolink",ke).getRegex(),ie=/[hH][tT][tT][pP][sS]?|[fF][tT][pP]/,ne={_backpedal:w,anyPunctuation:vo,autolink:go,blockSkip:io,br:ge,code:eo,del:w,delLDelim:w,delRDelim:w,emStrongLDelim:ro,emStrongRDelimAst:uo,emStrongRDelimUnd:co,escape:Yn,link:bo,nolink:ke,punctuation:no,reflink:ye,reflinkSearch:yo,tag:fo,text:ao,url:w},ko={...ne,link:h(/^!?\[(label)\]\((.*?)\)/).replace("label",R).getRegex(),reflink:h(/^!?\[(label)\]\s*\[([^\]]*)\]/).replace("label",R).getRegex()},F={...ne,emStrongRDelimAst:lo,emStrongLDelim:to,delLDelim:mo,delRDelim:ho,url:h(/^((?:protocol):\/\/|www\.)(?:[a-zA-Z0-9\-]+\.?)+[^\s<]*|^email/).replace("protocol",ie).replace("email",/[A-Za-z0-9._+-]+(@)[a-zA-Z0-9-_]+(?:\.[a-zA-Z0-9-_]*[a-zA-Z0-9])+(?![-_])/).getRegex(),_backpedal:/(?:[^?!.,:;*_'"~()&]+|\([^)]*\)|&(?![a-zA-Z0-9]+;$)|[?!.,:;*_'"~)]+(?!$))+/,del:/^(~~?)(?=[^\s~])((?:\\[\s\S]|[^\\])*?(?:\\[\s\S]|[^\s~\\]))\1(?=[^~]|$)/,text:h(/^([`~]+|[^`~])(?:(?= {2,}\n)|(?=[a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-]+@)|[\s\S]*?(?:(?=[\\<!\[`*~_]|\b_|protocol:\/\/|www\.|$)|[^ ](?= {2,}\n)|[^a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-](?=[a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-]+@)))/).replace("protocol",ie).getRegex()},Do={...F,br:h(ge).replace("{2,}","*").getRegex(),text:h(F.text).replace("\\b_","\\b_| {2,}\\n").replace(/\{2,\}/g,"*").getRegex()},W={normal:ee,gfm:Jn,pedantic:Zn},O={normal:ne,gfm:F,breaks:Do,pedantic:ko},qo={"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"},re=n=>qo[n];function q(n,e){if(e){if(b.escapeTest.test(n))return n.replace(b.escapeReplace,re)}else if(b.escapeTestNoEncode.test(n))return n.replace(b.escapeReplaceNoEncode,re);return n}function te(n){try{n=encodeURI(n).replace(b.percentDecode,"%")}catch{return null}return n}function ue(n,e){var i;let a=n.replace(b.findPipe,(t,u,r)=>{let l=!1,d=u;for(;--d>=0&&r[d]==="\\";)l=!l;return l?"|":" |"}),s=a.split(b.splitPipe),o=0;if(s[0].trim()||s.shift(),s.length>0&&!((i=s.at(-1))!=null&&i.trim())&&s.pop(),e)if(s.length>e)s.splice(e);else for(;s.length<e;)s.push("");for(;o<s.length;o++)s[o]=s[o].trim().replace(b.slashPipe,"|");return s}function P(n,e,a){let s=n.length;if(s===0)return"";let o=0;for(;o<s&&n.charAt(s-o-1)===e;)o++;return n.slice(0,s-o)}function de(n){let e=n.split(`
`),a=e.length-1;for(;a>=0&&b.blankLine.test(e[a]);)a--;return e.length-a<=2?n:e.slice(0,a+1).join(`
`)}function Po(n,e){if(n.indexOf(e[1])===-1)return-1;let a=0;for(let s=0;s<n.length;s++)if(n[s]==="\\")s++;else if(n[s]===e[0])a++;else if(n[s]===e[1]&&(a--,a<0))return s;return a>0?-2:-1}function Co(n,e=0){let a=e,s="";for(let o of n)if(o==="	"){let i=4-a%4;s+=" ".repeat(i),a+=i}else s+=o,a++;return s}function le(n,e,a,s,o){let i=e.href,t=e.title||null,u=n[1].replace(o.other.outputLinkReplace,"$1");s.state.inLink=!0;let r={type:n[0].charAt(0)==="!"?"image":"link",raw:a,href:i,title:t,text:u,tokens:s.inlineTokens(u)};return s.state.inLink=!1,r}function wo(n,e,a){let s=n.match(a.other.indentCodeCompensation);if(s===null)return e;let o=s[1];return e.split(`
`).map(i=>{let t=i.match(a.other.beginningSpace);if(t===null)return i;let[u]=t;return u.length>=o.length?i.slice(o.length):i}).join(`
`)}var _=class{constructor(n){g(this,"options");g(this,"rules");g(this,"lexer");this.options=n||L}space(n){let e=this.rules.block.newline.exec(n);if(e&&e[0].length>0)return{type:"space",raw:e[0]}}code(n){let e=this.rules.block.code.exec(n);if(e){let a=this.options.pedantic?e[0]:de(e[0]),s=a.replace(this.rules.other.codeRemoveIndent,"");return{type:"code",raw:a,codeBlockStyle:"indented",text:s}}}fences(n){let e=this.rules.block.fences.exec(n);if(e){let a=e[0],s=wo(a,e[3]||"",this.rules);return{type:"code",raw:a,lang:e[2]?e[2].trim().replace(this.rules.inline.anyPunctuation,"$1"):e[2],text:s}}}heading(n){let e=this.rules.block.heading.exec(n);if(e){let a=e[2].trim();if(this.rules.other.endingHash.test(a)){let s=P(a,"#");(this.options.pedantic||!s||this.rules.other.endingSpaceChar.test(s))&&(a=s.trim())}return{type:"heading",raw:P(e[0],`
`),depth:e[1].length,text:a,tokens:this.lexer.inline(a)}}}hr(n){let e=this.rules.block.hr.exec(n);if(e)return{type:"hr",raw:P(e[0],`
`)}}blockquote(n){let e=this.rules.block.blockquote.exec(n);if(e){let a=P(e[0],`
`).split(`
`),s="",o="",i=[];for(;a.length>0;){let t=!1,u=[],r;for(r=0;r<a.length;r++)if(this.rules.other.blockquoteStart.test(a[r]))u.push(a[r]),t=!0;else if(!t)u.push(a[r]);else break;a=a.slice(r);let l=u.join(`
`),d=l.replace(this.rules.other.blockquoteSetextReplace,`
    $1`).replace(this.rules.other.blockquoteSetextReplace2,"");s=s?`${s}
${l}`:l,o=o?`${o}
${d}`:d;let m=this.lexer.state.top;if(this.lexer.state.top=!0,this.lexer.blockTokens(d,i,!0),this.lexer.state.top=m,a.length===0)break;let p=i.at(-1);if((p==null?void 0:p.type)==="code")break;if((p==null?void 0:p.type)==="blockquote"){let f=p,c=f.raw+`
`+a.join(`
`),y=this.blockquote(c);i[i.length-1]=y,s=s.substring(0,s.length-f.raw.length)+y.raw,o=o.substring(0,o.length-f.text.length)+y.text;break}else if((p==null?void 0:p.type)==="list"){let f=p,c=f.raw+`
`+a.join(`
`),y=this.list(c);i[i.length-1]=y,s=s.substring(0,s.length-p.raw.length)+y.raw,o=o.substring(0,o.length-f.raw.length)+y.raw,a=c.substring(i.at(-1).raw.length).split(`
`);continue}}return{type:"blockquote",raw:s,tokens:i,text:o}}}list(n){let e=this.rules.block.list.exec(n);if(e){let a=e[1].trim(),s=a.length>1,o={type:"list",raw:"",ordered:s,start:s?+a.slice(0,-1):"",loose:!1,items:[]};a=s?`\\d{1,9}\\${a.slice(-1)}`:`\\${a}`,this.options.pedantic&&(a=s?a:"[*+-]");let i=this.rules.other.listItemRegex(a),t=!1;for(;n;){let r=!1,l="",d="";if(!(e=i.exec(n))||this.rules.block.hr.test(n))break;l=e[0],n=n.substring(l.length);let m=Co(e[2].split(`
`,1)[0],e[1].length),p=n.split(`
`,1)[0],f=!m.trim(),c=0;if(this.options.pedantic?(c=2,d=m.trimStart()):f?c=e[1].length+1:(c=m.search(this.rules.other.nonSpaceChar),c=c>4?1:c,d=m.slice(c),c+=e[1].length),f&&this.rules.other.blankLine.test(p)&&(l+=p+`
`,n=n.substring(p.length+1),r=!0),!r){let y=this.rules.other.nextBulletRegex(c),S=this.rules.other.hrRegex(c),M=this.rules.other.fencesBeginRegex(c),C=this.rules.other.headingBeginRegex(c),V=this.rules.other.htmlBeginRegex(c),qe=this.rules.other.blockquoteBeginRegex(c);for(;n;){let U=n.split(`
`,1)[0],x;if(p=U,this.options.pedantic?(p=p.replace(this.rules.other.listReplaceNesting,"  "),x=p):x=p.replace(this.rules.other.tabCharGlobal,"    "),M.test(p)||C.test(p)||V.test(p)||qe.test(p)||y.test(p)||S.test(p))break;if(x.search(this.rules.other.nonSpaceChar)>=c||!p.trim())d+=`
`+x.slice(c);else{if(f||m.replace(this.rules.other.tabCharGlobal,"    ").search(this.rules.other.nonSpaceChar)>=4||M.test(m)||C.test(m)||S.test(m))break;d+=`
`+p}f=!p.trim(),l+=U+`
`,n=n.substring(U.length+1),m=x.slice(c)}}o.loose||(t?o.loose=!0:this.rules.other.doubleBlankLine.test(l)&&(t=!0)),o.items.push({type:"list_item",raw:l,task:!!this.options.gfm&&this.rules.other.listIsTask.test(d),loose:!1,text:d,tokens:[]}),o.raw+=l}let u=o.items.at(-1);if(u)u.raw=u.raw.trimEnd(),u.text=u.text.trimEnd();else return;o.raw=o.raw.trimEnd();for(let r of o.items){this.lexer.state.top=!1,r.tokens=this.lexer.blockTokens(r.text,[]);let l=r.tokens[0];if(r.task&&((l==null?void 0:l.type)==="text"||(l==null?void 0:l.type)==="paragraph")){r.text=r.text.replace(this.rules.other.listReplaceTask,""),l.raw=l.raw.replace(this.rules.other.listReplaceTask,""),l.text=l.text.replace(this.rules.other.listReplaceTask,"");for(let m=this.lexer.inlineQueue.length-1;m>=0;m--)if(this.rules.other.listIsTask.test(this.lexer.inlineQueue[m].src)){this.lexer.inlineQueue[m].src=this.lexer.inlineQueue[m].src.replace(this.rules.other.listReplaceTask,"");break}let d=this.rules.other.listTaskCheckbox.exec(r.raw);if(d){let m={type:"checkbox",raw:d[0]+" ",checked:d[0]!=="[ ]"};r.checked=m.checked,o.loose?r.tokens[0]&&["paragraph","text"].includes(r.tokens[0].type)&&"tokens"in r.tokens[0]&&r.tokens[0].tokens?(r.tokens[0].raw=m.raw+r.tokens[0].raw,r.tokens[0].text=m.raw+r.tokens[0].text,r.tokens[0].tokens.unshift(m)):r.tokens.unshift({type:"paragraph",raw:m.raw,text:m.raw,tokens:[m]}):r.tokens.unshift(m)}}else r.task&&(r.task=!1);if(!o.loose){let d=r.tokens.filter(p=>p.type==="space"),m=d.length>0&&d.some(p=>this.rules.other.anyLine.test(p.raw));o.loose=m}}if(o.loose)for(let r of o.items){r.loose=!0;for(let l of r.tokens)l.type==="text"&&(l.type="paragraph")}return o}}html(n){let e=this.rules.block.html.exec(n);if(e){let a=de(e[0]);return{type:"html",block:!0,raw:a,pre:e[1]==="pre"||e[1]==="script"||e[1]==="style",text:a}}}def(n){let e=this.rules.block.def.exec(n);if(e){let a=e[1].toLowerCase().replace(this.rules.other.multipleSpaceGlobal," "),s=e[2]?e[2].replace(this.rules.other.hrefBrackets,"$1").replace(this.rules.inline.anyPunctuation,"$1"):"",o=e[3]?e[3].substring(1,e[3].length-1).replace(this.rules.inline.anyPunctuation,"$1"):e[3];return{type:"def",tag:a,raw:P(e[0],`
`),href:s,title:o}}}table(n){var t;let e=this.rules.block.table.exec(n);if(!e||!this.rules.other.tableDelimiter.test(e[2]))return;let a=ue(e[1]),s=e[2].replace(this.rules.other.tableAlignChars,"").split("|"),o=(t=e[3])!=null&&t.trim()?e[3].replace(this.rules.other.tableRowBlankLine,"").split(`
`):[],i={type:"table",raw:P(e[0],`
`),header:[],align:[],rows:[]};if(a.length===s.length){for(let u of s)this.rules.other.tableAlignRight.test(u)?i.align.push("right"):this.rules.other.tableAlignCenter.test(u)?i.align.push("center"):this.rules.other.tableAlignLeft.test(u)?i.align.push("left"):i.align.push(null);for(let u=0;u<a.length;u++)i.header.push({text:a[u],tokens:this.lexer.inline(a[u]),header:!0,align:i.align[u]});for(let u of o)i.rows.push(ue(u,i.header.length).map((r,l)=>({text:r,tokens:this.lexer.inline(r),header:!1,align:i.align[l]})));return i}}lheading(n){let e=this.rules.block.lheading.exec(n);if(e){let a=e[1].trim();return{type:"heading",raw:P(e[0],`
`),depth:e[2].charAt(0)==="="?1:2,text:a,tokens:this.lexer.inline(a)}}}paragraph(n){let e=this.rules.block.paragraph.exec(n);if(e){let a=e[1].charAt(e[1].length-1)===`
`?e[1].slice(0,-1):e[1];return{type:"paragraph",raw:e[0],text:a,tokens:this.lexer.inline(a)}}}text(n){let e=this.rules.block.text.exec(n);if(e)return{type:"text",raw:e[0],text:e[0],tokens:this.lexer.inline(e[0])}}escape(n){let e=this.rules.inline.escape.exec(n);if(e)return{type:"escape",raw:e[0],text:e[1]}}tag(n){let e=this.rules.inline.tag.exec(n);if(e)return!this.lexer.state.inLink&&this.rules.other.startATag.test(e[0])?this.lexer.state.inLink=!0:this.lexer.state.inLink&&this.rules.other.endATag.test(e[0])&&(this.lexer.state.inLink=!1),!this.lexer.state.inRawBlock&&this.rules.other.startPreScriptTag.test(e[0])?this.lexer.state.inRawBlock=!0:this.lexer.state.inRawBlock&&this.rules.other.endPreScriptTag.test(e[0])&&(this.lexer.state.inRawBlock=!1),{type:"html",raw:e[0],inLink:this.lexer.state.inLink,inRawBlock:this.lexer.state.inRawBlock,block:!1,text:e[0]}}link(n){let e=this.rules.inline.link.exec(n);if(e){let a=e[2].trim();if(!this.options.pedantic&&this.rules.other.startAngleBracket.test(a)){if(!this.rules.other.endAngleBracket.test(a))return;let i=P(a.slice(0,-1),"\\");if((a.length-i.length)%2===0)return}else{let i=Po(e[2],"()");if(i===-2)return;if(i>-1){let t=(e[0].indexOf("!")===0?5:4)+e[1].length+i;e[2]=e[2].substring(0,i),e[0]=e[0].substring(0,t).trim(),e[3]=""}}let s=e[2],o="";if(this.options.pedantic){let i=this.rules.other.pedanticHrefTitle.exec(s);i&&(s=i[1],o=i[3])}else o=e[3]?e[3].slice(1,-1):"";return s=s.trim(),this.rules.other.startAngleBracket.test(s)&&(this.options.pedantic&&!this.rules.other.endAngleBracket.test(a)?s=s.slice(1):s=s.slice(1,-1)),le(e,{href:s&&s.replace(this.rules.inline.anyPunctuation,"$1"),title:o&&o.replace(this.rules.inline.anyPunctuation,"$1")},e[0],this.lexer,this.rules)}}reflink(n,e){let a;if((a=this.rules.inline.reflink.exec(n))||(a=this.rules.inline.nolink.exec(n))){let s=(a[2]||a[1]).replace(this.rules.other.multipleSpaceGlobal," "),o=e[s.toLowerCase()];if(!o){let i=a[0].charAt(0);return{type:"text",raw:i,text:i}}return le(a,o,a[0],this.lexer,this.rules)}}emStrong(n,e,a=""){let s=this.rules.inline.emStrongLDelim.exec(n);if(!(!s||!s[1]&&!s[2]&&!s[3]&&!s[4]||s[4]&&a.match(this.rules.other.unicodeAlphaNumeric))&&(!(s[1]||s[3])||!a||this.rules.inline.punctuation.exec(a))){let o=[...s[0]].length-1,i,t,u=o,r=0,l=s[0][0]==="*"?this.rules.inline.emStrongRDelimAst:this.rules.inline.emStrongRDelimUnd;for(l.lastIndex=0,e=e.slice(-1*n.length+o);(s=l.exec(e))!==null;){if(i=s[1]||s[2]||s[3]||s[4]||s[5]||s[6],!i)continue;if(t=[...i].length,s[3]||s[4]){u+=t;continue}else if((s[5]||s[6])&&o%3&&!((o+t)%3)){r+=t;continue}if(u-=t,u>0)continue;t=Math.min(t,t+u+r);let d=[...s[0]][0].length,m=n.slice(0,o+s.index+d+t);if(Math.min(o,t)%2){let f=m.slice(1,-1);return{type:"em",raw:m,text:f,tokens:this.lexer.inlineTokens(f)}}let p=m.slice(2,-2);return{type:"strong",raw:m,text:p,tokens:this.lexer.inlineTokens(p)}}}}codespan(n){let e=this.rules.inline.code.exec(n);if(e){let a=e[2].replace(this.rules.other.newLineCharGlobal," "),s=this.rules.other.nonSpaceChar.test(a),o=this.rules.other.startingSpaceChar.test(a)&&this.rules.other.endingSpaceChar.test(a);return s&&o&&(a=a.substring(1,a.length-1)),{type:"codespan",raw:e[0],text:a}}}br(n){let e=this.rules.inline.br.exec(n);if(e)return{type:"br",raw:e[0]}}del(n,e,a=""){let s=this.rules.inline.delLDelim.exec(n);if(s&&(!s[1]||!a||this.rules.inline.punctuation.exec(a))){let o=[...s[0]].length-1,i,t,u=o,r=this.rules.inline.delRDelim;for(r.lastIndex=0,e=e.slice(-1*n.length+o);(s=r.exec(e))!==null;){if(i=s[1]||s[2]||s[3]||s[4]||s[5]||s[6],!i||(t=[...i].length,t!==o))continue;if(s[3]||s[4]){u+=t;continue}if(u-=t,u>0)continue;t=Math.min(t,t+u);let l=[...s[0]][0].length,d=n.slice(0,o+s.index+l+t),m=d.slice(o,-o);return{type:"del",raw:d,text:m,tokens:this.lexer.inlineTokens(m)}}}}autolink(n){let e=this.rules.inline.autolink.exec(n);if(e){let a,s;return e[2]==="@"?(a=e[1],s="mailto:"+a):(a=e[1],s=a),{type:"link",raw:e[0],text:a,href:s,tokens:[{type:"text",raw:a,text:a}]}}}url(n){var a;let e;if(e=this.rules.inline.url.exec(n)){let s,o;if(e[2]==="@")s=e[0],o="mailto:"+s;else{let i;do i=e[0],e[0]=((a=this.rules.inline._backpedal.exec(e[0]))==null?void 0:a[0])??"";while(i!==e[0]);s=e[0],e[1]==="www."?o="http://"+e[0]:o=e[0]}return{type:"link",raw:e[0],text:s,href:o,tokens:[{type:"text",raw:s,text:s}]}}}inlineText(n){let e=this.rules.inline.text.exec(n);if(e){let a=this.lexer.state.inRawBlock;return{type:"text",raw:e[0],text:e[0],escaped:a}}}},k=class K{constructor(e){g(this,"tokens");g(this,"options");g(this,"state");g(this,"inlineQueue");g(this,"tokenizer");this.tokens=[],this.tokens.links=Object.create(null),this.options=e||L,this.options.tokenizer=this.options.tokenizer||new _,this.tokenizer=this.options.tokenizer,this.tokenizer.options=this.options,this.tokenizer.lexer=this,this.inlineQueue=[],this.state={inLink:!1,inRawBlock:!1,top:!0};let a={other:b,block:W.normal,inline:O.normal};this.options.pedantic?(a.block=W.pedantic,a.inline=O.pedantic):this.options.gfm&&(a.block=W.gfm,this.options.breaks?a.inline=O.breaks:a.inline=O.gfm),this.tokenizer.rules=a}static get rules(){return{block:W,inline:O}}static lex(e,a){return new K(a).lex(e)}static lexInline(e,a){return new K(a).inlineTokens(e)}lex(e){e=e.replace(b.carriageReturn,`
`),this.blockTokens(e,this.tokens);for(let a=0;a<this.inlineQueue.length;a++){let s=this.inlineQueue[a];this.inlineTokens(s.src,s.tokens)}return this.inlineQueue=[],this.tokens}blockTokens(e,a=[],s=!1){var i,t,u;this.tokenizer.lexer=this,this.options.pedantic&&(e=e.replace(b.tabCharGlobal,"    ").replace(b.spaceLine,""));let o=1/0;for(;e;){if(e.length<o)o=e.length;else{this.infiniteLoopError(e.charCodeAt(0));break}let r;if((t=(i=this.options.extensions)==null?void 0:i.block)!=null&&t.some(d=>(r=d.call({lexer:this},e,a))?(e=e.substring(r.raw.length),a.push(r),!0):!1))continue;if(r=this.tokenizer.space(e)){e=e.substring(r.raw.length);let d=a.at(-1);r.raw.length===1&&d!==void 0?d.raw+=`
`:a.push(r);continue}if(r=this.tokenizer.code(e)){e=e.substring(r.raw.length);let d=a.at(-1);(d==null?void 0:d.type)==="paragraph"||(d==null?void 0:d.type)==="text"?(d.raw+=(d.raw.endsWith(`
`)?"":`
`)+r.raw,d.text+=`
`+r.text,this.inlineQueue.at(-1).src=d.text):a.push(r);continue}if(r=this.tokenizer.fences(e)){e=e.substring(r.raw.length),a.push(r);continue}if(r=this.tokenizer.heading(e)){e=e.substring(r.raw.length),a.push(r);continue}if(r=this.tokenizer.hr(e)){e=e.substring(r.raw.length),a.push(r);continue}if(r=this.tokenizer.blockquote(e)){e=e.substring(r.raw.length),a.push(r);continue}if(r=this.tokenizer.list(e)){e=e.substring(r.raw.length),a.push(r);continue}if(r=this.tokenizer.html(e)){e=e.substring(r.raw.length),a.push(r);continue}if(r=this.tokenizer.def(e)){e=e.substring(r.raw.length);let d=a.at(-1);(d==null?void 0:d.type)==="paragraph"||(d==null?void 0:d.type)==="text"?(d.raw+=(d.raw.endsWith(`
`)?"":`
`)+r.raw,d.text+=`
`+r.raw,this.inlineQueue.at(-1).src=d.text):this.tokens.links[r.tag]||(this.tokens.links[r.tag]={href:r.href,title:r.title},a.push(r));continue}if(r=this.tokenizer.table(e)){e=e.substring(r.raw.length),a.push(r);continue}if(r=this.tokenizer.lheading(e)){e=e.substring(r.raw.length),a.push(r);continue}let l=e;if((u=this.options.extensions)!=null&&u.startBlock){let d=1/0,m=e.slice(1),p;this.options.extensions.startBlock.forEach(f=>{p=f.call({lexer:this},m),typeof p=="number"&&p>=0&&(d=Math.min(d,p))}),d<1/0&&d>=0&&(l=e.substring(0,d+1))}if(this.state.top&&(r=this.tokenizer.paragraph(l))){let d=a.at(-1);s&&(d==null?void 0:d.type)==="paragraph"?(d.raw+=(d.raw.endsWith(`
`)?"":`
`)+r.raw,d.text+=`
`+r.text,this.inlineQueue.pop(),this.inlineQueue.at(-1).src=d.text):a.push(r),s=l.length!==e.length,e=e.substring(r.raw.length);continue}if(r=this.tokenizer.text(e)){e=e.substring(r.raw.length);let d=a.at(-1);(d==null?void 0:d.type)==="text"?(d.raw+=(d.raw.endsWith(`
`)?"":`
`)+r.raw,d.text+=`
`+r.text,this.inlineQueue.pop(),this.inlineQueue.at(-1).src=d.text):a.push(r);continue}if(e){this.infiniteLoopError(e.charCodeAt(0));break}}return this.state.top=!0,a}inline(e,a=[]){return this.inlineQueue.push({src:e,tokens:a}),a}inlineTokens(e,a=[]){var l,d,m,p,f;this.tokenizer.lexer=this;let s=e,o=null;if(this.tokens.links){let c=Object.keys(this.tokens.links);if(c.length>0)for(;(o=this.tokenizer.rules.inline.reflinkSearch.exec(s))!==null;)c.includes(o[0].slice(o[0].lastIndexOf("[")+1,-1))&&(s=s.slice(0,o.index)+"["+"a".repeat(o[0].length-2)+"]"+s.slice(this.tokenizer.rules.inline.reflinkSearch.lastIndex))}for(;(o=this.tokenizer.rules.inline.anyPunctuation.exec(s))!==null;)s=s.slice(0,o.index)+"++"+s.slice(this.tokenizer.rules.inline.anyPunctuation.lastIndex);let i;for(;(o=this.tokenizer.rules.inline.blockSkip.exec(s))!==null;)i=o[2]?o[2].length:0,s=s.slice(0,o.index+i)+"["+"a".repeat(o[0].length-i-2)+"]"+s.slice(this.tokenizer.rules.inline.blockSkip.lastIndex);s=((d=(l=this.options.hooks)==null?void 0:l.emStrongMask)==null?void 0:d.call({lexer:this},s))??s;let t=!1,u="",r=1/0;for(;e;){if(e.length<r)r=e.length;else{this.infiniteLoopError(e.charCodeAt(0));break}t||(u=""),t=!1;let c;if((p=(m=this.options.extensions)==null?void 0:m.inline)!=null&&p.some(S=>(c=S.call({lexer:this},e,a))?(e=e.substring(c.raw.length),a.push(c),!0):!1))continue;if(c=this.tokenizer.escape(e)){e=e.substring(c.raw.length),a.push(c);continue}if(c=this.tokenizer.tag(e)){e=e.substring(c.raw.length),a.push(c);continue}if(c=this.tokenizer.link(e)){e=e.substring(c.raw.length),a.push(c);continue}if(c=this.tokenizer.reflink(e,this.tokens.links)){e=e.substring(c.raw.length);let S=a.at(-1);c.type==="text"&&(S==null?void 0:S.type)==="text"?(S.raw+=c.raw,S.text+=c.text):a.push(c);continue}if(c=this.tokenizer.emStrong(e,s,u)){e=e.substring(c.raw.length),a.push(c);continue}if(c=this.tokenizer.codespan(e)){e=e.substring(c.raw.length),a.push(c);continue}if(c=this.tokenizer.br(e)){e=e.substring(c.raw.length),a.push(c);continue}if(c=this.tokenizer.del(e,s,u)){e=e.substring(c.raw.length),a.push(c);continue}if(c=this.tokenizer.autolink(e)){e=e.substring(c.raw.length),a.push(c);continue}if(!this.state.inLink&&(c=this.tokenizer.url(e))){e=e.substring(c.raw.length),a.push(c);continue}let y=e;if((f=this.options.extensions)!=null&&f.startInline){let S=1/0,M=e.slice(1),C;this.options.extensions.startInline.forEach(V=>{C=V.call({lexer:this},M),typeof C=="number"&&C>=0&&(S=Math.min(S,C))}),S<1/0&&S>=0&&(y=e.substring(0,S+1))}if(c=this.tokenizer.inlineText(y)){e=e.substring(c.raw.length),c.raw.slice(-1)!=="_"&&(u=c.raw.slice(-1)),t=!0;let S=a.at(-1);(S==null?void 0:S.type)==="text"?(S.raw+=c.raw,S.text+=c.text):a.push(c);continue}if(e){this.infiniteLoopError(e.charCodeAt(0));break}}return a}infiniteLoopError(e){let a="Infinite loop on byte: "+e;if(this.options.silent)console.error(a);else throw new Error(a)}},T=class{constructor(n){g(this,"options");g(this,"parser");this.options=n||L}space(n){return""}code({text:n,lang:e,escaped:a}){var i;let s=(i=(e||"").match(b.notSpaceStart))==null?void 0:i[0],o=n.replace(b.endingNewline,"")+`
`;return s?'<pre><code class="language-'+q(s)+'">'+(a?o:q(o,!0))+`</code></pre>
`:"<pre><code>"+(a?o:q(o,!0))+`</code></pre>
`}blockquote({tokens:n}){return`<blockquote>
${this.parser.parse(n)}</blockquote>
`}html({text:n}){return n}def(n){return""}heading({tokens:n,depth:e}){return`<h${e}>${this.parser.parseInline(n)}</h${e}>
`}hr(n){return`<hr>
`}list(n){let e=n.ordered,a=n.start,s="";for(let t=0;t<n.items.length;t++){let u=n.items[t];s+=this.listitem(u)}let o=e?"ol":"ul",i=e&&a!==1?' start="'+a+'"':"";return"<"+o+i+`>
`+s+"</"+o+`>
`}listitem(n){return`<li>${this.parser.parse(n.tokens)}</li>
`}checkbox({checked:n}){return"<input "+(n?'checked="" ':"")+'disabled="" type="checkbox"> '}paragraph({tokens:n}){return`<p>${this.parser.parseInline(n)}</p>
`}table(n){let e="",a="";for(let o=0;o<n.header.length;o++)a+=this.tablecell(n.header[o]);e+=this.tablerow({text:a});let s="";for(let o=0;o<n.rows.length;o++){let i=n.rows[o];a="";for(let t=0;t<i.length;t++)a+=this.tablecell(i[t]);s+=this.tablerow({text:a})}return s&&(s=`<tbody>${s}</tbody>`),`<table>
<thead>
`+e+`</thead>
`+s+`</table>
`}tablerow({text:n}){return`<tr>
${n}</tr>
`}tablecell(n){let e=this.parser.parseInline(n.tokens),a=n.header?"th":"td";return(n.align?`<${a} align="${n.align}">`:`<${a}>`)+e+`</${a}>
`}strong({tokens:n}){return`<strong>${this.parser.parseInline(n)}</strong>`}em({tokens:n}){return`<em>${this.parser.parseInline(n)}</em>`}codespan({text:n}){return`<code>${q(n,!0)}</code>`}br(n){return"<br>"}del({tokens:n}){return`<del>${this.parser.parseInline(n)}</del>`}link({href:n,title:e,tokens:a}){let s=this.parser.parseInline(a),o=te(n);if(o===null)return s;n=o;let i='<a href="'+n+'"';return e&&(i+=' title="'+q(e)+'"'),i+=">"+s+"</a>",i}image({href:n,title:e,text:a,tokens:s}){s&&(a=this.parser.parseInline(s,this.parser.textRenderer));let o=te(n);if(o===null)return q(a);n=o;let i=`<img src="${n}" alt="${q(a)}"`;return e&&(i+=` title="${q(e)}"`),i+=">",i}text(n){return"tokens"in n&&n.tokens?this.parser.parseInline(n.tokens):"escaped"in n&&n.escaped?n.text:q(n.text)}},oe=class{strong({text:n}){return n}em({text:n}){return n}codespan({text:n}){return n}del({text:n}){return n}html({text:n}){return n}text({text:n}){return n}link({text:n}){return""+n}image({text:n}){return""+n}br(){return""}checkbox({raw:n}){return n}},D=class Q{constructor(e){g(this,"options");g(this,"renderer");g(this,"textRenderer");this.options=e||L,this.options.renderer=this.options.renderer||new T,this.renderer=this.options.renderer,this.renderer.options=this.options,this.renderer.parser=this,this.textRenderer=new oe}static parse(e,a){return new Q(a).parse(e)}static parseInline(e,a){return new Q(a).parseInline(e)}parse(e){var s,o;this.renderer.parser=this;let a="";for(let i=0;i<e.length;i++){let t=e[i];if((o=(s=this.options.extensions)==null?void 0:s.renderers)!=null&&o[t.type]){let r=t,l=this.options.extensions.renderers[r.type].call({parser:this},r);if(l!==!1||!["space","hr","heading","code","table","blockquote","list","html","def","paragraph","text"].includes(r.type)){a+=l||"";continue}}let u=t;switch(u.type){case"space":{a+=this.renderer.space(u);break}case"hr":{a+=this.renderer.hr(u);break}case"heading":{a+=this.renderer.heading(u);break}case"code":{a+=this.renderer.code(u);break}case"table":{a+=this.renderer.table(u);break}case"blockquote":{a+=this.renderer.blockquote(u);break}case"list":{a+=this.renderer.list(u);break}case"checkbox":{a+=this.renderer.checkbox(u);break}case"html":{a+=this.renderer.html(u);break}case"def":{a+=this.renderer.def(u);break}case"paragraph":{a+=this.renderer.paragraph(u);break}case"text":{a+=this.renderer.text(u);break}default:{let r='Token with "'+u.type+'" type was not found.';if(this.options.silent)return console.error(r),"";throw new Error(r)}}}return a}parseInline(e,a=this.renderer){var o,i;this.renderer.parser=this;let s="";for(let t=0;t<e.length;t++){let u=e[t];if((i=(o=this.options.extensions)==null?void 0:o.renderers)!=null&&i[u.type]){let l=this.options.extensions.renderers[u.type].call({parser:this},u);if(l!==!1||!["escape","html","link","image","strong","em","codespan","br","del","text"].includes(u.type)){s+=l||"";continue}}let r=u;switch(r.type){case"escape":{s+=a.text(r);break}case"html":{s+=a.html(r);break}case"link":{s+=a.link(r);break}case"image":{s+=a.image(r);break}case"checkbox":{s+=a.checkbox(r);break}case"strong":{s+=a.strong(r);break}case"em":{s+=a.em(r);break}case"codespan":{s+=a.codespan(r);break}case"br":{s+=a.br(r);break}case"del":{s+=a.del(r);break}case"text":{s+=a.text(r);break}default:{let l='Token with "'+r.type+'" type was not found.';if(this.options.silent)return console.error(l),"";throw new Error(l)}}}return s}},I,G=(I=class{constructor(n){g(this,"options");g(this,"block");this.options=n||L}preprocess(n){return n}postprocess(n){return n}processAllTokens(n){return n}emStrongMask(n){return n}provideLexer(n=this.block){return n?k.lex:k.lexInline}provideParser(n=this.block){return n?D.parse:D.parseInline}},g(I,"passThroughHooks",new Set(["preprocess","postprocess","processAllTokens","emStrongMask"])),g(I,"passThroughHooksRespectAsync",new Set(["preprocess","postprocess","processAllTokens"])),I),zo=class{constructor(...n){g(this,"defaults",X());g(this,"options",this.setOptions);g(this,"parse",this.parseMarkdown(!0));g(this,"parseInline",this.parseMarkdown(!1));g(this,"Parser",D);g(this,"Renderer",T);g(this,"TextRenderer",oe);g(this,"Lexer",k);g(this,"Tokenizer",_);g(this,"Hooks",G);this.use(...n)}walkTokens(n,e){var s,o;let a=[];for(let i of n)switch(a=a.concat(e.call(this,i)),i.type){case"table":{let t=i;for(let u of t.header)a=a.concat(this.walkTokens(u.tokens,e));for(let u of t.rows)for(let r of u)a=a.concat(this.walkTokens(r.tokens,e));break}case"list":{let t=i;a=a.concat(this.walkTokens(t.items,e));break}default:{let t=i;(o=(s=this.defaults.extensions)==null?void 0:s.childTokens)!=null&&o[t.type]?this.defaults.extensions.childTokens[t.type].forEach(u=>{let r=t[u].flat(1/0);a=a.concat(this.walkTokens(r,e))}):t.tokens&&(a=a.concat(this.walkTokens(t.tokens,e)))}}return a}use(...n){let e=this.defaults.extensions||{renderers:{},childTokens:{}};return n.forEach(a=>{let s={...a};if(s.async=this.defaults.async||s.async||!1,a.extensions&&(a.extensions.forEach(o=>{if(!o.name)throw new Error("extension name required");if("renderer"in o){let i=e.renderers[o.name];i?e.renderers[o.name]=function(...t){let u=o.renderer.apply(this,t);return u===!1&&(u=i.apply(this,t)),u}:e.renderers[o.name]=o.renderer}if("tokenizer"in o){if(!o.level||o.level!=="block"&&o.level!=="inline")throw new Error("extension level must be 'block' or 'inline'");let i=e[o.level];i?i.unshift(o.tokenizer):e[o.level]=[o.tokenizer],o.start&&(o.level==="block"?e.startBlock?e.startBlock.push(o.start):e.startBlock=[o.start]:o.level==="inline"&&(e.startInline?e.startInline.push(o.start):e.startInline=[o.start]))}"childTokens"in o&&o.childTokens&&(e.childTokens[o.name]=o.childTokens)}),s.extensions=e),a.renderer){let o=this.defaults.renderer||new T(this.defaults);for(let i in a.renderer){if(!(i in o))throw new Error(`renderer '${i}' does not exist`);if(["options","parser"].includes(i))continue;let t=i,u=a.renderer[t],r=o[t];o[t]=(...l)=>{let d=u.apply(o,l);return d===!1&&(d=r.apply(o,l)),d||""}}s.renderer=o}if(a.tokenizer){let o=this.defaults.tokenizer||new _(this.defaults);for(let i in a.tokenizer){if(!(i in o))throw new Error(`tokenizer '${i}' does not exist`);if(["options","rules","lexer"].includes(i))continue;let t=i,u=a.tokenizer[t],r=o[t];o[t]=(...l)=>{let d=u.apply(o,l);return d===!1&&(d=r.apply(o,l)),d}}s.tokenizer=o}if(a.hooks){let o=this.defaults.hooks||new G;for(let i in a.hooks){if(!(i in o))throw new Error(`hook '${i}' does not exist`);if(["options","block"].includes(i))continue;let t=i,u=a.hooks[t],r=o[t];G.passThroughHooks.has(i)?o[t]=l=>{if(this.defaults.async&&G.passThroughHooksRespectAsync.has(i))return(async()=>{let m=await u.call(o,l);return r.call(o,m)})();let d=u.call(o,l);return r.call(o,d)}:o[t]=(...l)=>{if(this.defaults.async)return(async()=>{let m=await u.apply(o,l);return m===!1&&(m=await r.apply(o,l)),m})();let d=u.apply(o,l);return d===!1&&(d=r.apply(o,l)),d}}s.hooks=o}if(a.walkTokens){let o=this.defaults.walkTokens,i=a.walkTokens;s.walkTokens=function(t){let u=[];return u.push(i.call(this,t)),o&&(u=u.concat(o.call(this,t))),u}}this.defaults={...this.defaults,...s}}),this}setOptions(n){return this.defaults={...this.defaults,...n},this}lexer(n,e){return k.lex(n,e??this.defaults)}parser(n,e){return D.parse(n,e??this.defaults)}parseMarkdown(n){return(e,a)=>{let s={...a},o={...this.defaults,...s},i=this.onError(!!o.silent,!!o.async);if(this.defaults.async===!0&&s.async===!1)return i(new Error("marked(): The async option was set to true by an extension. Remove async: false from the parse options object to return a Promise."));if(typeof e>"u"||e===null)return i(new Error("marked(): input parameter is undefined or null"));if(typeof e!="string")return i(new Error("marked(): input parameter is of type "+Object.prototype.toString.call(e)+", string expected"));if(o.hooks&&(o.hooks.options=o,o.hooks.block=n),o.async)return(async()=>{let t=o.hooks?await o.hooks.preprocess(e):e,u=await(o.hooks?await o.hooks.provideLexer(n):n?k.lex:k.lexInline)(t,o),r=o.hooks?await o.hooks.processAllTokens(u):u;o.walkTokens&&await Promise.all(this.walkTokens(r,o.walkTokens));let l=await(o.hooks?await o.hooks.provideParser(n):n?D.parse:D.parseInline)(r,o);return o.hooks?await o.hooks.postprocess(l):l})().catch(i);try{o.hooks&&(e=o.hooks.preprocess(e));let t=(o.hooks?o.hooks.provideLexer(n):n?k.lex:k.lexInline)(e,o);o.hooks&&(t=o.hooks.processAllTokens(t)),o.walkTokens&&this.walkTokens(t,o.walkTokens);let u=(o.hooks?o.hooks.provideParser(n):n?D.parse:D.parseInline)(t,o);return o.hooks&&(u=o.hooks.postprocess(u)),u}catch(t){return i(t)}}}onError(n,e){return a=>{if(a.message+=`
Please report this to https://github.com/markedjs/marked.`,n){let s="<p>An error occurred:</p><pre>"+q(a.message+"",!0)+"</pre>";return e?Promise.resolve(s):s}if(e)return Promise.reject(a);throw a}}},A=new zo;function v(n,e){return A.parse(n,e)}v.options=v.setOptions=function(n){return A.setOptions(n),v.defaults=A.defaults,me(v.defaults),v};v.getDefaults=X;v.defaults=L;v.use=function(...n){return A.use(...n),v.defaults=A.defaults,me(v.defaults),v};v.walkTokens=function(n,e){return A.walkTokens(n,e)};v.parseInline=A.parseInline;v.Parser=D;v.parser=D.parse;v.Renderer=T;v.TextRenderer=oe;v.Lexer=k;v.lexer=k.lex;v.Tokenizer=_;v.Hooks=G;v.parse=v;v.options;v.setOptions;v.use;v.walkTokens;v.parseInline;D.parse;k.lex;v.setOptions({gfm:!0,breaks:!1});const Ao=/^(?:https?:|mailto:|tel:|#|\/|\.\/|\.\.\/)/i;v.use({walkTokens(n){if(n.type==="html")n.text="";else if(n.type==="link"||n.type==="image"){const e=n;(!e.href||!Ao.test(e.href.trim()))&&(e.href="#")}}});const Lo=Object.assign({"./content/back-up-emulator-saves/de.md":ze,"./content/back-up-emulator-saves/en.md":Ae,"./content/back-up-emulator-saves/es.md":Le,"./content/back-up-emulator-saves/fr.md":He,"./content/back-up-emulator-saves/it.md":je,"./content/back-up-emulator-saves/ja.md":xe,"./content/back-up-emulator-saves/pt.md":Oe,"./content/back-up-emulator-saves/zh.md":Ge,"./content/back-up-game-saves/de.md":Ee,"./content/back-up-game-saves/en.md":Me,"./content/back-up-game-saves/es.md":We,"./content/back-up-game-saves/fr.md":Ie,"./content/back-up-game-saves/it.md":Re,"./content/back-up-game-saves/ja.md":_e,"./content/back-up-game-saves/pt.md":Te,"./content/back-up-game-saves/zh.md":Be,"./content/baldurs-gate-3-save-location/de.md":Ne,"./content/baldurs-gate-3-save-location/en.md":Ve,"./content/baldurs-gate-3-save-location/es.md":Ue,"./content/baldurs-gate-3-save-location/fr.md":Fe,"./content/baldurs-gate-3-save-location/it.md":Ke,"./content/baldurs-gate-3-save-location/ja.md":Qe,"./content/baldurs-gate-3-save-location/pt.md":Xe,"./content/baldurs-gate-3-save-location/zh.md":$e,"./content/crimson-desert-save-location/de.md":Je,"./content/crimson-desert-save-location/en.md":Ze,"./content/crimson-desert-save-location/es.md":Ye,"./content/crimson-desert-save-location/fr.md":ea,"./content/crimson-desert-save-location/it.md":aa,"./content/crimson-desert-save-location/ja.md":na,"./content/crimson-desert-save-location/pt.md":oa,"./content/crimson-desert-save-location/zh.md":sa,"./content/cyberpunk-2077-save-location/de.md":ia,"./content/cyberpunk-2077-save-location/en.md":ra,"./content/cyberpunk-2077-save-location/es.md":ta,"./content/cyberpunk-2077-save-location/fr.md":ua,"./content/cyberpunk-2077-save-location/it.md":da,"./content/cyberpunk-2077-save-location/ja.md":la,"./content/cyberpunk-2077-save-location/pt.md":ca,"./content/cyberpunk-2077-save-location/zh.md":ma,"./content/game-save-sync-comparison/de.md":pa,"./content/game-save-sync-comparison/en.md":ha,"./content/game-save-sync-comparison/es.md":va,"./content/game-save-sync-comparison/fr.md":ga,"./content/game-save-sync-comparison/it.md":Sa,"./content/game-save-sync-comparison/ja.md":fa,"./content/game-save-sync-comparison/pt.md":ba,"./content/game-save-sync-comparison/zh.md":ya,"./content/ludusavi-alternative/de.md":ka,"./content/ludusavi-alternative/en.md":Da,"./content/ludusavi-alternative/es.md":qa,"./content/ludusavi-alternative/fr.md":Pa,"./content/ludusavi-alternative/it.md":Ca,"./content/ludusavi-alternative/ja.md":wa,"./content/ludusavi-alternative/pt.md":za,"./content/ludusavi-alternative/zh.md":Aa,"./content/opensave-alternative/de.md":La,"./content/opensave-alternative/en.md":Ha,"./content/opensave-alternative/es.md":ja,"./content/opensave-alternative/fr.md":xa,"./content/opensave-alternative/it.md":Oa,"./content/opensave-alternative/ja.md":Ga,"./content/opensave-alternative/pt.md":Ea,"./content/opensave-alternative/zh.md":Ma,"./content/palworld-save-location/de.md":Wa,"./content/palworld-save-location/en.md":Ia,"./content/palworld-save-location/es.md":Ra,"./content/palworld-save-location/fr.md":_a,"./content/palworld-save-location/it.md":Ta,"./content/palworld-save-location/ja.md":Ba,"./content/palworld-save-location/pt.md":Na,"./content/palworld-save-location/zh.md":Va,"./content/restore-a-game-save/de.md":Ua,"./content/restore-a-game-save/en.md":Fa,"./content/restore-a-game-save/es.md":Ka,"./content/restore-a-game-save/fr.md":Qa,"./content/restore-a-game-save/it.md":Xa,"./content/restore-a-game-save/ja.md":$a,"./content/restore-a-game-save/pt.md":Ja,"./content/restore-a-game-save/zh.md":Za,"./content/self-host-hoard/de.md":Ya,"./content/self-host-hoard/en.md":en,"./content/self-host-hoard/es.md":an,"./content/self-host-hoard/fr.md":nn,"./content/self-host-hoard/it.md":on,"./content/self-host-hoard/ja.md":sn,"./content/self-host-hoard/pt.md":rn,"./content/self-host-hoard/zh.md":tn,"./content/spider-man-2-save-location/de.md":un,"./content/spider-man-2-save-location/en.md":dn,"./content/spider-man-2-save-location/es.md":ln,"./content/spider-man-2-save-location/fr.md":cn,"./content/spider-man-2-save-location/it.md":mn,"./content/spider-man-2-save-location/ja.md":pn,"./content/spider-man-2-save-location/pt.md":hn,"./content/spider-man-2-save-location/zh.md":vn,"./content/steam-cloud-alternative/de.md":gn,"./content/steam-cloud-alternative/en.md":Sn,"./content/steam-cloud-alternative/es.md":fn,"./content/steam-cloud-alternative/fr.md":bn,"./content/steam-cloud-alternative/it.md":yn,"./content/steam-cloud-alternative/ja.md":kn,"./content/steam-cloud-alternative/pt.md":Dn,"./content/steam-cloud-alternative/zh.md":qn,"./content/sync-game-saves-across-pcs/de.md":Pn,"./content/sync-game-saves-across-pcs/en.md":Cn,"./content/sync-game-saves-across-pcs/es.md":wn,"./content/sync-game-saves-across-pcs/fr.md":zn,"./content/sync-game-saves-across-pcs/it.md":An,"./content/sync-game-saves-across-pcs/ja.md":Ln,"./content/sync-game-saves-across-pcs/pt.md":Hn,"./content/sync-game-saves-across-pcs/zh.md":jn,"./content/syncthing-game-saves/de.md":xn,"./content/syncthing-game-saves/en.md":On,"./content/syncthing-game-saves/es.md":Gn,"./content/syncthing-game-saves/fr.md":En,"./content/syncthing-game-saves/it.md":Mn,"./content/syncthing-game-saves/ja.md":Wn,"./content/syncthing-game-saves/pt.md":In,"./content/syncthing-game-saves/zh.md":Rn});function Ho(n){const e=n.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/);if(!e)return{meta:{},body:n};const a={};for(const s of e[1].split(/\r?\n/)){const o=s.indexOf(":");if(o===-1)continue;const i=s.slice(0,o).trim();let t=s.slice(o+1).trim();(t.startsWith('"')&&t.endsWith('"')||t.startsWith("'")&&t.endsWith("'"))&&(t=t.slice(1,-1)),a[i]=t}return{meta:a,body:e[2]}}const jo=n=>n.replace(/\[([^\]]+)\]\([^)]*\)/g,"$1").replace(/[*_`]/g,"").trim();function xo(n){const e=n.indexOf("<!-- faq -->");return e===-1?[]:n.slice(e).split(/^###[ \t]+/m).slice(1).map(a=>{const s=a.indexOf(`
`),o=jo(s===-1?a:a.slice(0,s)),t=(s===-1?"":a.slice(s+1)).split(/^##[ \t]/m)[0].trim();return{question:o,answer:t?v.parse(t):""}}).filter(a=>a.question&&a.answer)}const z={};for(const[n,e]of Object.entries(Lo)){const a=n.match(/\/content\/([^/]+)\/([^/]+)\.md$/);if(!a)continue;const[,s,o]=a;if(!we.includes(o))continue;const{meta:i,body:t}=Ho(e);(z[s]??(z[s]={}))[o]={slug:s,title:i.title??s,description:i.description??"",order:Number(i.order??999),featured:i.featured==="true",updated:i.updated??"",related:(i.related??"").split(",").map(u=>u.trim()).filter(Boolean),html:v.parse(t.trim()),faq:xo(t)}}function De(n,e){const a=z[n];return a?a[e]??a[ce]??null:null}function Eo(n){return Object.keys(z).map(e=>De(e,n)).filter(e=>e!==null).sort((e,a)=>e.order-a.order||e.title.localeCompare(a.title))}function Mo(n,e){var s,o;return(((o=(s=z[n])==null?void 0:s[ce])==null?void 0:o.related)??[]).filter(i=>i!==n).map(i=>De(i,e)).filter(i=>i!==null)}function Wo(){return Object.keys(z)}export{Wo as a,De as g,Eo as l,Mo as r};
