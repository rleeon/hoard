---
title: "RetroArch save sync: cloud saves across PC and Steam Deck"
description: "Sync RetroArch saves and states between PC, Steam Deck and laptop: where .srm files live, built-in Cloud Sync vs automatic sync, and the traps."
order: 14
updated: 2026-10-09
related: back-up-emulator-saves, sync-saves-steam-deck-pc, sync-game-saves-across-pcs, pcsx2-cloud-saves, dolphin-cloud-saves
---

RetroArch keeps in-game saves as `.srm` files in a `saves` folder and save states in a `states` folder. To sync them between devices you can use RetroArch's built-in Cloud Sync with a WebDAV server you provide, or a tool that watches both folders. Hoard does the second automatically: it backs up both folders when you quit RetroArch, brings them down on your other machines, keeps every version, and understands EmuDeck setups.

## Where RetroArch keeps saves

- **Windows:** `%APPDATA%\RetroArch\saves` and `\states`, or `saves` and `states` next to `retroarch.exe` if you installed it to its own folder.
- **Linux:** `~/.config/retroarch/saves`. The Flatpak uses `~/.var/app/org.libretro.RetroArch/config/retroarch/saves`.
- **Steam Deck with EmuDeck:** `~/Emulation/saves/retroarch/`, where `saves` and `states` are links to the real folders. Hoard reads `retroarch.cfg` to find where they really point.
- **RetroDECK:** `~/retrodeck/saves` and `~/retrodeck/states` by default.
- **Anywhere else:** **Settings → Directory** shows the folders RetroArch is actually using.

## When RetroArch actually writes the save

This catches people out. RetroArch keeps the in-game save in memory and only writes the `.srm` when you close the game or quit RetroArch, unless **Settings → Saving → SaveRAM Autosave Interval** is set. Until then, nothing is on disk, so a crash or a flat battery loses everything since the last write, and no sync tool can move a save that hasn't been written.

Set an autosave interval of a few seconds. And before you switch devices, **quit RetroArch**, not just the game: Hoard backs up once RetroArch has closed, so it never copies a save mid-write. On a Deck, suspending doesn't count as quitting.

## Keep every device configured the same

- **Sorting options.** **Settings → Saving** can sort saves and states into subfolders by core name or by content folder. If one device sorts and the other doesn't, the synced file lands in a folder RetroArch isn't looking in. Use the same settings everywhere.
- **ROM file names.** The `.srm` is named after the ROM: `Super Metroid (USA).sfc` saves to `Super Metroid (USA).srm`. A differently named ROM on the other device won't find it.
- **The same core.** Two cores for the same console don't always read each other's saves. Pick one per system and use it everywhere.
- **Core versions, for states.** A save state is a snapshot of the core's memory and often won't load in a different core version. Save files don't have that problem.

One more trap with states: a state includes the game's memory, in-game save included. Load an old state and the next `.srm` write puts that older save back. If you use **Auto Load State**, sync states as well, so the newest one is what travels.

## RetroArch Cloud Sync or Hoard?

To be fair to both:

- **RetroArch Cloud Sync** is built in and syncs saves and states with a WebDAV server you run or rent. It works on Android and iOS too, which Hoard doesn't today. If you already run Nextcloud, which keeps file versions of its own, it's a good fit, and it's the better choice if your phone is part of the setup.
- **Hoard** needs no WebDAV server. It backs up and syncs automatically, keeps a version history you can roll back to, and covers your standalone emulators and PC games as well. It tracks the whole `saves` folder as one item, so rolling back restores every game in it as it was at that moment. Before you confirm, it shows what will change, and your current files are captured first.

Pick one per folder. Two tools writing the same saves is how you create conflicts.

## Set it up with Hoard

1. Install Hoard on each device and sign in with the same account.
2. In the **Library**, add RetroArch. Saves and states appear as two entries.
3. Match the settings above on every device.
4. Play, quit RetroArch, and pick up on the other device.

Prefer to keep it at home? Run `hoard-server` on your own PC or NAS: no account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

<!-- faq -->

## Frequently asked questions

### Does RetroArch have cloud saves?

Yes, a built-in Cloud Sync that needs a WebDAV server. Hoard is the alternative if you don't want to run one, want a version history, or also use standalone emulators.

### Why didn't my RetroArch save sync?

Usually one of three things: RetroArch hadn't written the `.srm` yet (it was still open, with no autosave interval), the two devices sort saves into different subfolders, or the ROM files have different names.

### Can I sync save states too?

Yes. Hoard tracks `states` as its own entry. Whether a state loads on the other device depends on both running the same core version.

### Does it work with EmuDeck and RetroDECK?

Yes. Hoard reads RetroArch's configuration to follow EmuDeck's links to the real folders. For RetroDECK, add `~/retrodeck/saves` and `~/retrodeck/states` if they aren't picked up.

### Does Hoard sync RetroArch on Android?

Not today. Hoard runs on Windows, macOS, Linux and Steam Deck.
