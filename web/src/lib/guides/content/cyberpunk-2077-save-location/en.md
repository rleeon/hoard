---
title: "Cyberpunk 2077 save location (PC & Steam Deck)"
description: "Where Cyberpunk 2077 keeps its saves on Windows, Steam Deck and Mac, what each folder holds, and how to back them up or move them between PCs."
order: 20
updated: 2026-10-02
related: sync-game-saves-across-pcs, restore-a-game-save, steam-cloud-alternative
---

On Windows, Cyberpunk 2077 keeps its saves in `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`, one folder per save. That's the short answer. The rest of this page covers the Steam Deck and Mac paths, what's actually in the folder, and how to keep it backed up.

## Where Cyberpunk 2077 keeps its saves

- **Windows** (Steam, GOG or Epic): `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`
- **Steam Deck and Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`
- **Mac:** `~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves`

The Windows and Mac paths are the ones CD Projekt Red gives in its own support pages. On a Steam Deck, the game runs inside a Proton prefix, which is a small Windows folder tree that Steam keeps per game; `1091500` is Cyberpunk's Steam app ID. If the game is installed on the microSD card, look for `steamapps/compatdata/1091500` on the card instead.

## What's in the folder

Cyberpunk doesn't write one save file. It writes **one folder per save**: `AutoSave-0`, `AutoSave-1` and so on, `ManualSave-0`, `ManualSave-1`, and `QuickSave-0`. Each holds the save itself (`sav.dat`) plus the screenshot and metadata the load menu shows.

Two things follow from that:

- **Back up the parent folder, not a single save.** Copying only the newest `ManualSave` leaves out the autosaves, which are often the most recent progress.
- **Autosaves rotate.** The game reuses a small set of `AutoSave` folders and overwrites the oldest. An autosave from three hours ago is usually already gone, which is why an external history is worth having.

Settings are not here. Graphics and controls live in `UserSettings.json` under `%LOCALAPPDATA%\CD Projekt Red\Cyberpunk 2077`, next to caches and logs. That's the folder a lot of people (and some tools) back up by mistake: it holds nothing you'd lose progress over.

## Does Cyberpunk 2077 have cloud saves?

Yes. The Steam version uses Steam Cloud, and the GOG version uses GOG Galaxy's cloud. Both keep the latest state of your saves in step between machines on the same store.

What neither does:

- **Keep older versions.** If a save gets corrupted, or a mod breaks it, the cloud holds the broken copy too.
- **Cross stores.** Steam Cloud and GOG's cloud don't talk to each other, even though PC saves from Steam, GOG and Epic load fine in any of them if you copy the folder over.

## Back it up by hand

1. Close the game completely.
2. Copy the whole `Cyberpunk 2077` folder from the path above to a USB stick, another drive or a cloud folder.
3. To restore, close the game and copy the folder back, replacing what's there.

It works, but only as often as you remember to do it, and only the copy you made last time.

## Keep it backed up and in sync with Hoard

[Hoard](/download) backs up the save folder every time you stop playing and keeps every version, so a corrupted save or an autosave that rotated away is one click back. It also syncs the folder between your PCs and a Steam Deck.

1. Install Hoard and sign in, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. Cyberpunk is detected from your Steam library and the community save database.
3. Check that the folder shown is the `Saved Games\CD Projekt Red\Cyberpunk 2077` one. If it shows the `AppData\Local` folder instead, change it: that one only has settings.
4. Play. When you quit, the first version appears in the history.

Hoard tracks the whole folder, so every `AutoSave`, `ManualSave` and `QuickSave` goes into the same version. On a Deck and a desktop, the newest version is waiting on whichever one you pick up next — see [how syncing between PCs works](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Frequently asked questions

### Where are Cyberpunk 2077 saves on Steam Deck?

Inside the game's Proton prefix: `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`. If the game is on the microSD card, the `compatdata` folder is on the card.

### Can I move my Cyberpunk 2077 saves from GOG to Steam?

Yes. PC saves are the same across Steam, GOG and Epic. Copy the save folders into the same path on the other install with the game closed, and they show up in the load menu.

### Why are there so many AutoSave folders?

The game keeps a handful of autosave slots and overwrites the oldest one each time. They're normal saves; it's just that they get replaced on their own.

### Why did my older autosave disappear?

Because the slot it lived in was reused. The game only keeps a few. A backup tool that keeps versions is the only way to get one back after it rotates out.

### Does Hoard keep my settings in sync too?

No. Settings live in a separate folder that isn't part of the save, so each machine keeps its own, which is usually what you want: a Deck and a desktop need different graphics settings. More on that in [syncing saves across PCs](/guides/sync-game-saves-across-pcs).
