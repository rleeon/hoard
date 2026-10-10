---
title: "Corrupted game save? How to recover it"
description: "A save that won't load isn't always gone. Where to find a working copy (game backups, Steam Cloud, Windows, OneDrive) and how never to be stuck again."
order: 12
updated: 2026-10-09
related: restore-a-game-save, back-up-game-saves, where-are-pc-game-saves-stored, onedrive-game-saves
---

A save that won't load is rarely beyond saving. Most of the time a working copy exists somewhere: a backup the game made itself, the copy in Steam Cloud, a previous version kept by Windows or OneDrive, or the save on another PC. The order you try things in matters, though, because the wrong move can overwrite the good copy with the broken one. Start here.

## First: stop, and copy the folder

1. **Close the game**, and don't start a new game in that slot. Every save from now on can push an older copy out.
2. **Copy the whole save folder** to your desktop or a USB stick. Whatever you try next can then be undone. If you don't know where the folder is, see [where PC games keep their saves](/guides/where-are-pc-game-saves-stored).
3. **Pause anything that syncs that folder.** Steam Cloud (per game, under **Properties → General**), OneDrive, Syncthing. Otherwise the broken file can travel to the one place that still has a good copy.

## Make sure it's actually corrupted

A few things look like corruption and aren't:

- **The game was updated** and old saves don't load, or need a patch. Check the game's news or forum.
- **Mods are missing.** Bethesda games in particular warn about missing plugins and may refuse a save that used them. Reinstall the mods first.
- **You're on a different account.** Some games file saves under your Steam or Ubisoft account ID, so a different account sees an empty slot.
- **The file is online-only.** With OneDrive, a save with a cloud icon has been moved off the disk to free space. Right-click it and choose **Always keep on this device**.

A save that is **0 KB**, or much smaller than its neighbours, really is broken: the write was cut off halfway.

## Where a working copy may be

Go through these in order. The first ones are quicker and more likely to work.

### 1. The game's own backups

Many games keep a spare copy without telling you. Look in the save folder for files ending in `.bak`, `_old` or `.backup`, and for extra autosave slots. Some well-known cases:

- **Elden Ring** writes `ER0000.sl2.bak` next to the save.
- **Stardew Valley** keeps a `_old` copy of each farm, which is the previous in-game day.
- **Terraria** keeps `.bak` files for players and worlds.
- **Minecraft Java** keeps `level.dat_old` inside each world.

To use one, keep the broken file aside (you've already copied the folder), then rename the backup to the original file name.

### 2. Steam Cloud

Steam Cloud keeps the **latest** copy, not a history. It only helps if the save broke after the last upload, for example because the game crashed and the bad file never got synced. You can see and download what Steam holds for each game at [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage), then put the file back by hand.

### 3. Windows Previous Versions

Right-click the save folder, open **Properties → Previous Versions**. If File History or System Protection was turned on for that drive, older copies of the folder show up here, and you can open them to take the files you need. If the list is empty, neither was on.

### 4. OneDrive version history

If your `Documents` folder is backed up by OneDrive, many saves are in it without you knowing. On onedrive.com, right-click the save file and choose **Version history** to download an earlier version. OneDrive keeps these for a limited time, and deleted files go to its recycle bin for a while too.

### 5. Your other machines

Played on a laptop or a Steam Deck recently? Its copy may predate the problem. Copy it over before that machine syncs the broken one.

### 6. If the file was deleted, not broken

Check the Recycle Bin first. After that, a file-recovery tool may find it, as long as you stop writing to that drive. Every install and download lowers the odds.

## When nothing turns up

For a few popular games, the community has save editors or repair tools that can rebuild a damaged file: search the game's name with "save repair". Otherwise, the honest answer is that the only copy that counts is one made before the problem.

## Why saves break

- **A crash or power cut mid-write.** The game was halfway through saving when it died.
- **A full disk.** The game couldn't finish writing and left a truncated file.
- **A sync tool caught it mid-write**, or two PCs edited the same save and one copy won.
- **A mod** wrote something the game can't read back.
- **A failing drive**, which usually shows up in other files too.

## Never be stuck again

Every recovery above depends on luck: the game happened to keep a backup, or Steam hadn't synced yet. A versioned backup takes the luck out. That's what Hoard does: it backs up each save automatically after you stop playing, once the folder has gone quiet, so a backup is never a half-written file. Every version is kept. When something breaks, you open the game's **History** and restore the last good one in one click; your current save is captured first, so even that is reversible. And because Hoard also keeps your saves in sync, the copy on your laptop or Steam Deck is never an older, forgotten one: every machine works from the same history.

A tip for spotting the moment it broke: a sudden drop in size between two versions usually means a truncated save. More in [how to restore an old game save](/guides/restore-a-game-save).

If you'd rather keep backups at home, run `hoard-server` on your own PC or NAS. No account with us, no telemetry to us, nothing passing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

<!-- faq -->

## Frequently asked questions

### Can a corrupted save be repaired?

Rarely in place. A few games have community repair tools, but in most cases recovering means finding an older copy: the game's own backup, Steam Cloud, Windows or OneDrive, or another PC.

### Does Steam Cloud keep old versions of my saves?

No. It keeps the current file only. If a broken save has already been uploaded, Steam Cloud has the broken one too.

### Will verifying game files fix a corrupted save?

No. Verifying checks the game's own files against Steam's, not your saves. It can help if the game itself is damaged, but it won't bring back progress.

### Why is my save file 0 KB?

The game started writing the save and never finished: a crash, a power cut or a full disk. Look for a `.bak` or `_old` file next to it, or an earlier version elsewhere.

### How do I stop this from happening again?

Keep versioned backups taken when the game isn't running. Hoard does that automatically after every session and keeps each version, so you can go back to any one of them.
