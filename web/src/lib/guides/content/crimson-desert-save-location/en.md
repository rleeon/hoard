---
title: "Crimson Desert save location (PC & Steam Deck)"
description: "Where Crimson Desert keeps its saves on Windows, Steam Deck and Mac, which folder actually holds them, and how to back them up or move them between PCs."
order: 21
updated: 2026-10-02
related: sync-game-saves-across-pcs, restore-a-game-save, steam-cloud-alternative
---

On Windows, Crimson Desert keeps its saves in `%LOCALAPPDATA%\Pearl Abyss\CD\save`. That's the folder Pearl Abyss points to in its own FAQ. Below are the Steam Deck and Mac paths, what's inside, and how to keep it backed up.

## Where Crimson Desert keeps its saves

- **Windows:** `%LOCALAPPDATA%\Pearl Abyss\CD\save` (that is, `C:\Users\<you>\AppData\Local\Pearl Abyss\CD\save`)
- **Steam Deck and Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`
- **Mac, Steam version:** `~/Library/Application Support/Pearl Abyss/CD/save`
- **Mac, App Store version:** `~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save`

There's no native Linux build, so on a Steam Deck the game runs through Proton and its saves sit inside the Proton prefix Steam keeps for it; `3321460` is the game's Steam app ID. If the game is on the microSD card, the `compatdata` folder is on the card.

`AppData` is a hidden folder on Windows. The quickest way in is to paste `%LOCALAPPDATA%\Pearl Abyss\CD\save` into the File Explorer address bar.

## What's in the folder

Inside `save` there are two subfolders. According to Pearl Abyss, **the one with a numeric name holds the saves you create in the game**. When you back up, take the whole `save` folder rather than picking files: it's small, and you won't leave anything the game needs behind.

The Steam and App Store versions on Mac use different paths. If you switch between them, copy the saves across by hand once.

## Does Crimson Desert have cloud saves?

Yes, the Steam version has Steam Cloud, which keeps the latest saves in step between machines on the same Steam account.

What it doesn't do:

- **Keep older versions.** Steam Cloud holds the current state. If a save gets corrupted, the corrupted one is what syncs.
- **Cover other stores.** A Mac App Store copy and a Steam copy don't share a cloud.

## Back it up by hand

1. Close the game completely.
2. Copy the whole `save` folder from the path above somewhere safe: another drive, a USB stick, a cloud folder.
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

Inside the game's Proton prefix: `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`. If the game is on the microSD card, the `compatdata` folder is on the card.

### Which subfolder has my saves?

The one with the numeric name inside `save`. Back up the whole `save` folder anyway, so nothing is left out.

### I can't find the AppData folder. Where is it?

It's hidden by default. Paste `%LOCALAPPDATA%\Pearl Abyss\CD\save` into the File Explorer address bar and press Enter, or turn on hidden items in the View menu.

### Can I play on my desktop and my Steam Deck with the same save?

Yes. Steam Cloud does it for the latest save on the same Steam account. Hoard does it too, and keeps a version per session, so you can go back if something breaks.
