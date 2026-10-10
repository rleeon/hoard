---
title: "OneDrive and game saves: what breaks and how to fix it"
description: "OneDrive moved your Documents and now saves fail, vanish or show up twice. Why it happens, how to fix it, and a better way to sync game saves."
order: 15
updated: 2026-10-09
related: where-are-pc-game-saves-stored, recover-corrupted-game-save, syncthing-game-saves, sync-game-saves-across-pcs
---

On many Windows PCs, OneDrive backs up the `Documents` folder, often switched on during setup without anyone noticing. Games that save in `Documents`, which is most of `My Games`, follow it into OneDrive. Then the trouble starts: saves that fail to write, saves that need a download before they load, and copies with your PC's name stuck on them that the game never reads. Here's why it happens and how to fix it.

## How your saves ended up in OneDrive

OneDrive's folder backup moves `Documents`, `Desktop` and `Pictures` to `C:\Users\<you>\OneDrive\...`. Games ask Windows where `Documents` is, so they quietly follow. To check, right-click `Documents` and open **Properties → Location**: if the path contains `OneDrive`, your saves are in it.

`AppData` and `Saved Games` aren't part of that backup, so games that save there aren't affected.

## What goes wrong

- **Writes collide.** OneDrive uploads files the moment they change. A game writing its save at the same moment can find the file in use.
- **Online-only saves.** OneDrive can free up space by keeping files only in the cloud (the cloud icon). The game then needs a download before it can load the save, and offline there's nothing to load.
- **Conflict copies.** Use OneDrive on two PCs with the same account and both sync the same `My Games`. Play on both before one has caught up and OneDrive keeps both versions by renaming one with the PC's name. The game ignores that file.
- **Space.** The free plan is 5 GB, and some games keep mods, caches or recordings in `Documents` too.
- **No sense of a play session.** OneDrive syncs file by file, mid-game, and versions each file on its own rather than the save as a whole.

## The fixes

### Quick: keep saves on the device

Right-click `Documents\My Games` (or the game's own folder) and choose **Always keep on this device**. That ends the online-only problem. It doesn't stop OneDrive syncing while you play.

### Clean: stop backing up Documents

In OneDrive, open **Settings → Sync and backup → Manage back up** and turn off `Documents`. Windows points `Documents` back to the local folder, but the files already backed up stay in the OneDrive folder. Before switching off, mark them **Always keep on this device** so they're really on disk. Then, with your games closed, move the game folders back into the local `Documents`, or the games will start fresh.

## A better split

OneDrive is good at documents. Saves need something different: a backup taken once the game has closed, versions of the whole save rather than single files, and sync to your other PCs and a Steam Deck.

That's what Hoard does. It finds your saves whether `Documents` is in OneDrive or not, because it asks Windows where the folder really is. It backs them up automatically after each session, keeps every version, and syncs them to your other machines.

One rule: let one tool own the syncing between PCs. If OneDrive backs up `Documents` on several gaming PCs with the same account, it's already syncing those saves between them. Turn off the `Documents` backup on those PCs and let Hoard handle the saves.

Prefer no cloud at all? Run `hoard-server` on your own PC or NAS: no account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

<!-- faq -->

## Frequently asked questions

### Should I let OneDrive back up my game saves?

On a single PC, as a backup, it's better than nothing. As a way to sync saves between PCs it causes conflicts, because it doesn't know when a game is running.

### I turned off the backup and my saves are gone. Where are they?

Still in the OneDrive folder: `C:\Users\<you>\OneDrive\Documents\My Games`. Close your games and move them back into the local `Documents`.

### What are the save files with my PC's name in them?

OneDrive conflict copies. Two PCs changed the same file before syncing, and OneDrive kept both. Work out which one is newer, give it the original name, and keep the other aside.

### Can OneDrive bring back an older save?

Sometimes. On onedrive.com, right-click the file and choose **Version history**. It works file by file and only for a limited time. See [how to recover a corrupted save](/guides/recover-corrupted-game-save).

### Does Hoard work if my Documents folder is in OneDrive?

Yes. Hoard reads where Windows says `Documents` is, so it finds saves in either place.
