---
title: "How to back up and sync emulator saves (RetroArch, Dolphin, PCSX2)"
description: "Back up and sync emulator saves between PCs and Steam Deck: RetroArch, Dolphin, PCSX2, DuckStation and more, with version history and where each one saves."
order: 6
updated: 2026-10-01
related: sync-game-saves-across-pcs, back-up-game-saves, ludusavi-alternative
---

Emulator saves are easy to lose: save files and save states live in scattered folders, and a reinstall or a new PC can wipe years of progress. Hoard backs them up automatically and keeps them in sync across machines, including a Steam Deck.

## Emulators Hoard works with

Hoard handles standard emulator save files (`.srm`, `.sav`, memory cards, per-title save folders) and save states. It knows where these keep their saves out of the box:

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

PCSX2 writes memory cards (`.ps2` files) to `memcards/`:

- Windows: `Documents\PCSX2\memcards`
- Linux: `~/.config/PCSX2/memcards`
- Steam Deck: `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`

One memory card holds the saves of every game you've played on it, so it travels as one item: restoring an older version rolls back the whole card, not a single game.

### Dolphin cloud saves (GameCube and Wii)

GameCube saves live under `GC/` (memory card images or one folder per card), Wii saves in the emulated NAND under `Wii/`:

- Windows: `Documents\Dolphin Emulator\GC` and `\Wii`
- Linux: `~/.local/share/dolphin-emu/GC` and `/Wii`
- Steam Deck: `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/`

### DuckStation cloud saves (PS1)

DuckStation keeps memory cards in `memcards/`, and by default it makes a separate card for each game, which suits syncing well:

- Windows: `Documents\DuckStation\memcards` (newer builds use `%LOCALAPPDATA%\DuckStation\memcards`)
- Linux: `~/.local/share/duckstation/memcards`
- Steam Deck: `~/.var/app/org.duckstation.DuckStation/` under `data/` or `config/`

### RetroArch save sync

RetroArch splits `saves/` (the in-game saves) from `states/` (save states). Hoard tracks the saves folder; add `states/` as its own entry if you play with states:

- Windows: `%APPDATA%\RetroArch`, or next to `retroarch.exe` on a portable install
- Linux: `~/.config/retroarch`
- Steam Deck: `~/.var/app/org.libretro.RetroArch/config/retroarch`, or `~/Emulation/saves/retroarch` if you set it up with EmuDeck

RetroArch also has a built-in Cloud Sync that talks to a WebDAV server you provide. It's a reasonable choice if you only use RetroArch and already run WebDAV. Hoard needs no WebDAV, keeps a version history you can roll back, and covers the standalone emulators too.

### PPSSPP (PSP)

Saves go to `PSP/SAVEDATA`, states to `PSP/PPSSPP_STATE`:

- Windows: `Documents\PPSSPP\PSP\SAVEDATA`, or `memstick\PSP\SAVEDATA` next to the executable on a portable install
- Linux: `~/.config/ppsspp/PSP/SAVEDATA`
- Steam Deck: `~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA`

### RPCS3 (PS3)

Saves live in `dev_hdd0/home/00000001/savedata`, inside the RPCS3 folder on Windows and under `~/.config/rpcs3/` on Linux and Steam Deck.

### Switch emulators: Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

Ryujinx keeps saves in `bis/user/save` (under `%APPDATA%\Ryujinx` or `~/.config/Ryujinx`). The yuzu family uses `nand/user/save` under its own folder in `%APPDATA%` or `~/.local/share`.

There's a trap here. The yuzu-style tree goes `save/<account>/<profile>/<title-id>/`, and the profile ID is generated the first time the emulator runs, so it's different on every install. Sync the whole `save/` folder between two machines and each one ends up with the other's profile next to its own, and neither game sees the other's progress. Hoard steps down to each game's own folder instead, so the same title matches across machines no matter what the profile is called.

### Citra and Azahar (3DS)

Saves sit deep under `sdmc/Nintendo 3DS/<id0>/<id1>/title/…`, and `id0`/`id1` come from the emulated console's keys, so they also differ per install. Hoard handles it the same way as the Switch tree: one entry per game, matched across machines.

### The rest

- **Cemu (Wii U):** `mlc01/usr/save`, under `%APPDATA%\Cemu` or `~/.local/share/Cemu`.
- **shadPS4 (PS4):** `savedata`, under `%APPDATA%\shadPS4` or `~/.local/share/shadPS4`.
- **Vita3K (PS Vita):** `ux0/user/00/savedata` inside its data folder.
- **mGBA, melonDS and most cartridge-era emulators:** a `.sav` next to the ROM, unless you told them otherwise. Add the ROM folder's saves by hand.

## Emulator saves on a Steam Deck

On a Steam Deck the emulators usually come from Flatpak, so their folders sit under `~/.var/app/<id>/` rather than the usual `~/.config` or `~/.local/share`. EmuDeck gathers everything under `~/Emulation/saves/`, one folder per emulator. Either way, add the folder once and Hoard watches it.

The part that matters on a handheld: Hoard's engine runs as a background service, so it backs up after you quit a game in Game Mode without any window open. Pick the Deck up after a session on the desktop and the save is already there.

## Save files and save states are not the same thing

Worth separating, because they behave differently when they travel:

- A **save file** (`.srm`, a memory card, a `SAVEDATA` folder) is the game's own save, written by the emulated console. It moves between machines and between emulator versions without complaint.
- A **save state** is a dump of emulator memory. It's tied to the emulator build, and often to the exact core, so a state written by one version may refuse to load in another.

Hoard backs up both. Just don't be surprised when a state from an updated machine won't open on a stale one — keep your emulators on matching versions, and lean on save files for anything you care about.

## One emulator, many games

An emulator is a single process hosting dozens of titles, which is what makes emulator saves awkward for a tool that thinks in terms of "the running game". Hoard keeps the titles apart rather than treating the whole emulator as one blob, so each game gets its own history instead of a single pile that changes every time you launch anything. If a save does go wrong, you can [roll it back to an earlier version](/guides/restore-a-game-save).

## Emulator saves without our servers

Everything here works the same against your own server: run `hoard-server`, point the app at it, and your saves go from your machine to your disk. No account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

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
