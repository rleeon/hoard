---
title: "Baldur's Gate 3 save location (PC & Steam Deck)"
description: "Where Baldur's Gate 3 keeps its saves on Windows, Steam Deck and Mac, what's a save and what's mods or settings, Honour Mode, and how to back saves up."
order: 23
updated: 2026-10-02
related: sync-game-saves-across-pcs, restore-a-game-save, steam-cloud-alternative
---

On Windows, Baldur's Gate 3 keeps its saves in `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`, one folder per save. That's the path Larian gives in its own support FAQ. Below are the Steam Deck and Mac paths, what sits next to the saves, Honour Mode, and how to keep it all backed up.

## Where Baldur's Gate 3 keeps its saves

- **Windows:** `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`
- **Steam Deck and Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`
- **Mac:** `~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`

Larian retired the native Linux build, so on a Steam Deck the game runs through Proton and the saves sit inside the Proton prefix Steam keeps for it; `1086940` is the game's Steam app ID. If it's installed on the microSD card, the `compatdata` folder is on the card.

## What's a save and what isn't

Each save is **a folder** inside `Story`, holding an `.lsv` file and a thumbnail. Everything else around it is something else:

- **`Mods`** (under `Baldur's Gate 3`) holds mod files.
- **`modsettings.lsx`** (under `PlayerProfiles\Public`) is the list of mods that are switched on, and their load order.
- **Settings** such as graphics and controls are config files next to the profile, not part of a save.

The one that catches people is mods. A save made with mods expects the same mods to be active when it loads. If you move a modded save to another PC, bring the mod list with it, or the game warns you about missing mods and the save may not load as expected.

## Honour Mode

Honour Mode keeps a single save that the game overwrites as you play, and if your party falls, the Honour run is over (you can carry on in Custom Mode, without the Honour). Backing up that save is your call: a copy taken before a hard fight is technically a way back, and some players want exactly that after a crash or a bug, while others consider it cheating the mode. A backup tool keeps the versions either way; whether you ever restore one is between you and the dice.

## Does Baldur's Gate 3 have cloud saves?

Yes. On Steam it uses Steam Cloud, which keeps the latest saves in step between machines on the same account. It holds the current state only: if a save gets corrupted or a mod update breaks it, that's the version that syncs.

## Back it up by hand

1. Close the game completely.
2. Copy the whole `PlayerProfiles` folder from the path above (it contains `Savegames` and `modsettings.lsx`).
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

Inside the game's Proton prefix: `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`.

### Can I move a modded save to another PC?

Yes, as long as the other PC has the same mods installed and switched on in the same order. Copy `modsettings.lsx` along with the save, and install the same mod files.

### Can I back up an Honour Mode save?

The save is an ordinary folder, so yes, any backup tool can copy it. Whether restoring it fits the spirit of the mode is up to you.

### Why does my save say mods are missing?

It was made with mods that aren't active now. Turn the same mods back on, in the same order, and it loads normally.
