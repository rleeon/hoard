---
title: "How to back up and sync emulator saves (RetroArch, Dolphin, PCSX2)"
description: "Back up and sync your emulator save files and save states across PCs — RetroArch, Dolphin, PCSX2, DuckStation and more — automatically with Hoard."
order: 6
updated: 2026-09-01
---

Emulator saves are easy to lose: save files and save states live in scattered folders, and a reinstall or a new PC can wipe years of progress. Hoard backs them up automatically and keeps them in sync across machines.

## Emulators Hoard works with

Hoard handles standard emulator save files (`.srm`, `.sav`, memory cards) and save states for the popular emulators, including:

- **RetroArch** — per-core saves and states
- **Dolphin** (GameCube / Wii) — memory cards and GCI files
- **PCSX2** (PS2) — memory cards
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA**, and more

Because Hoard locates save folders using the same community database that powers Ludusavi, many emulator paths are detected automatically. For anything custom, you can point Hoard at a folder by hand.

## Set up emulator save backups

1. **Install Hoard** for Windows, macOS or Linux and sign in.
2. Open the **Library** and add your emulator, or add its saves/states folder manually if you've changed the default location.
3. Keep **automatic mode** on. Hoard backs up after each session and keeps a versioned history.
4. Install Hoard on your other PCs with the same account to sync those saves everywhere — see [syncing saves across PCs](/guides/sync-game-saves-across-pcs).

## Ludusavi for emulators?

Ludusavi can back up emulator saves locally too, and it's a great free option for that. If you also want those emulator saves to sync automatically between machines and keep a cloud version history without configuring Rclone, that's where Hoard helps — read the full [Ludusavi vs Hoard comparison](/guides/ludusavi-alternative).

## Where each emulator keeps its saves

Useful to know, because a portable install puts all of this somewhere else entirely:

- **RetroArch** — `saves/` and `states/` under the config folder: `%APPDATA%\RetroArch` on Windows, `~/.config/retroarch` on Linux.
- **Dolphin** — memory cards under `GC/`, Wii saves in the emulated NAND, inside `Documents\Dolphin Emulator` or `~/.local/share/dolphin-emu`.
- **PCSX2** — `memcards/`, under `Documents\PCSX2` or `~/.config/PCSX2`.
- **DuckStation** — `memcards/` and `savestates/` in its own data folder.
- **PPSSPP** — `PSP/SAVEDATA` for saves and `PSP/PPSSPP_STATE` for states.
- **RPCS3** — `dev_hdd0/home/00000001/savedata`.
- **Cemu** — `mlc01/usr/save`.
- **mGBA and most standalone cores** — a `.sav` next to the ROM, unless you told them otherwise.

A **portable install** — the norm on handhelds and USB sticks — keeps every one of those next to the executable instead. If that's your setup, point Hoard at that folder and it tracks it like any other save.

## Save files and save states are not the same thing

Worth separating, because they behave differently when they travel:

- A **save file** (`.srm`, a memory card, a `SAVEDATA` folder) is the game's own save, written by the emulated console. It moves between machines and between emulator versions without complaint.
- A **save state** is a dump of emulator memory. It's tied to the emulator build, and often to the exact core, so a state written by one version may refuse to load in another.

Hoard backs up both. Just don't be surprised when a state from an updated machine won't open on a stale one — keep your emulators on matching versions, and lean on save files for anything you care about.

## One emulator, many games

An emulator is a single process hosting dozens of titles, which is what makes emulator saves awkward for a tool that thinks in terms of "the running game". Hoard keeps the titles apart rather than treating the whole emulator as one blob, so each game gets its own history instead of a single pile that changes every time you launch anything.

## Emulator saves without our servers

Everything here works the same against your own server: run `hoard-server`, point the app at it, and your saves go from your machine to your disk. No account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

## Tip

Save states are tied to a specific emulator version. Keep your emulators updated consistently across PCs so a synced state loads cleanly everywhere.

<!-- faq -->

## Frequently asked questions

### Does Hoard back up my ROMs too?

No. It tracks save folders, not game files. ROMs are large, they don't change, and you already have them — there's nothing to version.

### My emulator is a portable install. Does that work?

Yes. Add the folder next to the executable by hand and Hoard tracks it like any other save location. This is the usual setup on handhelds.

### Can I sync save states between two PCs?

You can, and Hoard will. Whether a state loads depends on the emulators being the same version on both machines, which is an emulator limitation rather than a sync one. Save files don't have that problem.

### Will it work with an emulator that isn't on the list?

Almost certainly. Detection covers the common ones automatically, and anything else you can add by pointing Hoard at its saves folder.

### Does self-hosting change anything for emulators?

No. Same detection, same versions, same sync. Only the storage is yours.
