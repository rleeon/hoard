---
title: "Where does Steam store game saves? Every location on PC and Steam Deck"
description: "Steam has a userdata folder for saves, but most games save in AppData, Documents or their own folder. Where to look on Windows, Steam Deck and Mac."
order: 30
updated: 2026-11-05
related: where-are-pc-game-saves-stored, sync-saves-steam-deck-pc, steam-cloud-alternative, transfer-game-saves-to-new-pc, back-up-game-saves
---

Steam has one folder of its own for saves, `userdata\<account-id>\<AppID>\remote` inside the Steam folder, but most games never use it. In the community save database, which knows the Windows save path of 10,937 Steam games, only about **1 in 10** keeps its saves there. The rest write to AppData, Documents or their own install folder, and Steam Cloud, when the game has it, picks the files up from wherever they are.

## Steam's own folder: userdata

- **Windows:** `C:\Program Files (x86)\Steam\userdata\<account-id>\<AppID>\remote`
- **Steam Deck and Linux:** `~/.local/share/Steam/userdata/<account-id>/<AppID>/remote`
- **macOS:** `~/Library/Application Support/Steam/userdata/<account-id>/<AppID>/remote`

`<account-id>` is the number Steam shows as your **Friend Code** (Friends, then Add a Friend). There is one folder for each account that has signed in on that machine. `<AppID>` is the number in the game's store address: `store.steampowered.com/app/413150` is Stardew Valley.

The folder above `remote`, the bare `<AppID>` one, also holds `remotecache.vdf`. That's Steam's own bookkeeping, not your save, and it is supposed to differ between your PC and your Deck.

## Where most games actually save

Counted on that same database on 5 November 2026, the share of Steam games that use each place (some use more than one):

- **`AppData\LocalLow`**, 23%, mostly games made with Unity: `%USERPROFILE%\AppData\LocalLow\<developer>\<game>`
- **The game's install folder**, 22%: `C:\Program Files (x86)\Steam\steamapps\common\<game>`
- **`AppData\Local`**, 21%: `%LOCALAPPDATA%\<developer or game>`
- **`Documents`**, 15%, often under `My Games`: `%USERPROFILE%\Documents\My Games\<game>`
- **`AppData\Roaming`**, 12%: `%APPDATA%\<game>`
- **Steam's `userdata`**, 10%: see above
- **`Saved Games`**, 3%: `%USERPROFILE%\Saved Games\<developer>\<game>`

## On Steam Deck: the Proton prefix

Windows games on the Deck run through Proton, and each one gets its own little Windows inside `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`. AppData, Documents and Saved Games all live in there, so `%APPDATA%\Game` on a PC becomes `compatdata/<AppID>/pfx/drive_c/users/steamuser/AppData/Roaming/Game` on the Deck. The prefix appears the first time the game runs.

Games with a native Linux version skip the prefix and usually save under `~/.local/share` or `~/.config`. The install folder is `~/.local/share/Steam/steamapps/common/<game>`, or the same `steamapps` tree on the microSD card if the game is installed there.

## Find one game's saves fast

1. Look the game up in the [save locations list](/saves), or on its PCGamingWiki page under "Save game data location".
2. If it uses Steam's folder, open `userdata\<account-id>\<AppID>\remote` with the AppID from its store page.
3. If not, save in the game, then sort `%LOCALAPPDATA%`, `%USERPROFILE%\AppData\LocalLow`, `%APPDATA%` and `Documents` by date modified. The folder that just changed is the one.

## What Steam Cloud keeps

Steam Cloud keeps the latest save on your Steam account, for the games whose developer switched it on. Steam's [remote storage page](https://store.steampowered.com/account/remotestorage) lists every game with files in your Steam Cloud and lets you download them.

It keeps the latest state, not a history, so a corrupted save syncs as faithfully as a good one, and it only covers Steam games: nothing from GOG, Epic, an emulator or a non-Steam shortcut. More on that in the [Steam Cloud alternative guide](/guides/steam-cloud-alternative).

## Keep it backed up and in sync with Hoard

[Hoard](/download) is free and open source. It finds each game's saves wherever the game put them, userdata, AppData, Documents, the install folder or a Proton prefix on the Deck, backs them up every time the game writes one and again when you quit, keeps a history you can go back to, and syncs them between your PCs and a Steam Deck. In Steam's folder it tracks `<AppID>/remote/` and leaves `remotecache.vdf` alone, so your PC and your Deck never look like they disagree.

1. Install Hoard and sign in to Hoard One, recommended for everyone, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. Your Steam games are detected from your Steam library and the community save database.
3. Play. When you quit, the first version appears in the history.

## Hoard One, or your own server

Hoard is free either way, and the app is the same on both:

- **Hoard One**, recommended for everyone. Nothing to set up and nothing to keep running: a managed server in the EU, free with 2 GB and three devices, no card needed. [Pro](/pricing) only adds room, 100 GB and unlimited devices, for when your saves outgrow that.
- **Your own server.** Run `hoard-server` on a PC, a NAS or a Raspberry Pi and every machine syncs through it. There's no account with us, no telemetry to us and no quota from us: nothing passes through our servers, because there's nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same history. Hoard One keeps every version for good; on your own server you decide how much history to keep.

<!-- faq -->

## Frequently asked questions

### Where is the Steam userdata folder?

Inside the Steam folder: `C:\Program Files (x86)\Steam\userdata` on Windows if Steam is in its default place, `~/.local/share/Steam/userdata` on a Steam Deck or Linux, and `~/Library/Application Support/Steam/userdata` on a Mac.

### Does uninstalling a game delete its saves?

Saves in the install folder go with it. Saves in AppData, Documents or userdata usually stay. On the Deck, uninstalling can also remove the game's Proton prefix, and with it every save inside, so back it up first.

### Can I copy Steam saves to another PC by hand?

Yes. Close the game, copy the save folder to the same place on the other PC and start the game there. If it uses Steam Cloud, Steam may ask which copy to keep on the next launch; pick the one with the newer date.

### Is Hoard free?

Yes. The app, the command line and the server are free and open source (AGPL-3.0), and running your own server costs nothing but the machine. Hoard One, the managed server we recommend for everyone, is free too with 2 GB and three devices; Pro adds room for €1.99 a month.

### Does self-hosting send anything to Hoard?

No. On your own server there's no account with us and no telemetry to us: your saves, your users and your logs stay on your machine and never touch ours.
