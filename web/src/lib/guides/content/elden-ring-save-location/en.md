---
title: "Elden Ring save location (PC & Steam Deck)"
description: "Where Elden Ring keeps ER0000.sl2 on PC and Steam Deck, what's in it, Seamless Co-op and Nightreign saves, and how to back up every character."
order: 26
updated: 2026-10-18
related: back-up-game-saves, restore-a-game-save, recover-corrupted-game-save, sync-saves-steam-deck-pc, steam-cloud-alternative
---

On PC, Elden Ring keeps its saves in `%APPDATA%\EldenRing\<your Steam ID>`, in a single file called `ER0000.sl2` that holds every character you've made, up to ten. Below is the Steam Deck path, what else is in the folder, where Seamless Co-op and Nightreign keep their saves, and how to keep all of it backed up and in sync between your PC and Steam Deck.

## Where Elden Ring keeps its saves

- **Windows (Steam):** `%APPDATA%\EldenRing\<your Steam ID>\ER0000.sl2`
- **Steam Deck and Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1245620/pfx/drive_c/users/steamuser/AppData/Roaming/EldenRing/<your Steam ID>/`

`%APPDATA%` is `C:\Users\<you>\AppData\Roaming`: paste `%APPDATA%\EldenRing` into the File Explorer address bar to go straight there. The Steam ID folder is a long number tied to your Steam account. On a Steam Deck the game runs through Proton, so the save sits inside the Proton prefix Steam keeps for it; `1245620` is the game's Steam app ID.

## What's in the save folder

- **`ER0000.sl2`** is your progress: every character, their gear and where they are in the world, Shadow of the Erdtree included.
- **`ER0000.sl2.bak`** is a backup the game keeps next to it.
- **`ER0000.co2`**, if you see one, belongs to the Seamless Co-op mod (more below).

Graphics settings aren't in this folder: they live one level up, in `%APPDATA%\EldenRing\GraphicsConfig.xml`.

Two things follow from how the save file works:

- **Characters can't be backed up or restored one at a time.** Restoring an older `ER0000.sl2` takes every character back to that moment, not just the one you meant to fix.
- **The save is tied to your Steam account.** Your Steam ID is written inside the file, and the game won't load a save made on another account. Moving your own save between your own PCs and a Steam Deck works fine.

## Seamless Co-op and Nightreign

### Seamless Co-op

The Seamless Co-op mod leaves `ER0000.sl2` alone and keeps its own save, `ER0000.co2`, in the same folder, so your modded and unmodded characters are separate files. The mod's community advises against copying a `.co2` back over the normal save to play online.

### Elden Ring Nightreign

Nightreign is a separate game with its own folder: `%APPDATA%\Nightreign\<your Steam ID>\NR0000.sl2` on Windows, and the same path inside its own Proton prefix on a Steam Deck (`compatdata/2622380`). It never touches your Elden Ring characters.

## Does Elden Ring have cloud saves?

Yes, through Steam Cloud, which keeps `ER0000.sl2` in step between machines on the same account. It holds the latest state only, with no older versions to go back to, and the game's `.bak` lives on the same disk as the save, so one dead drive takes both.

## Back it up by hand

1. Close the game completely.
2. Copy your Steam ID folder from `%APPDATA%\EldenRing` somewhere safe. It holds `ER0000.sl2`, its `.bak` and any Seamless Co-op save.
3. To restore, close the game and copy the folder back to the same place.

## Keep it backed up and in sync with Hoard

[Hoard](/download) is free and open source. It backs up your Elden Ring saves every time the game writes one and again when you quit, keeps a history you can go back to, and syncs the folder between your PCs and a Steam Deck. Only the files that changed travel, compressed, and every file is checked against its SHA-256 hash when it comes back.

1. Install Hoard and sign in to Hoard One, recommended for everyone, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. Elden Ring is detected from your Steam library and the community save database.
3. Play. When you quit, the first version appears in the history.

Hoard takes the whole Steam ID folder, so a Seamless Co-op save is in every version too. Each version is all your characters at that moment: to roll back, [restore an earlier version](/guides/restore-a-game-save), and the newer versions stay in the history if you change your mind.

## Hoard One, or your own server

Hoard is free either way, and the app is the same on both:

- **Hoard One**, recommended for everyone. Nothing to set up and nothing to keep running: a managed server in the EU, free with 2 GB and three devices, no card needed. [Pro](/pricing) only adds room, 100 GB and unlimited devices, for when your saves outgrow that.
- **Your own server.** Run `hoard-server` on a PC, a NAS or a Raspberry Pi and every machine syncs through it. There's no account with us, no telemetry to us and no quota from us: nothing passes through our servers, because there's nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same history. Hoard One keeps every version for good; on your own server you decide how much history to keep.

<!-- faq -->

## Frequently asked questions

### Can I use my Elden Ring save on another Steam account?

Not as it is. Your Steam ID is written inside `ER0000.sl2`, and the game won't load a save made on a different account. Your own save on your own account moves freely between your PCs and a Steam Deck.

### Will restoring an old save roll back all my characters?

Yes. All your characters share `ER0000.sl2`, so any restore takes every one of them back to that moment. With Hoard the newer versions stay in the history, so a rollback is never a one-way trip.

### Does Hoard back up Seamless Co-op saves?

Yes. The mod keeps `ER0000.co2` in the same Steam ID folder as the normal save, and Hoard backs up the whole folder, so both are in every version.

### Is Hoard free?

Yes. The app, the command line and the server are free and open source (AGPL-3.0), and running your own server costs nothing but the machine. Hoard One, the managed server we recommend for everyone, is free too with 2 GB and three devices; Pro adds room for €1.99 a month.

### Does self-hosting send anything to Hoard?

No. On your own server there's no account with us and no telemetry to us: your saves, your users and your logs stay on your machine and never touch ours.
