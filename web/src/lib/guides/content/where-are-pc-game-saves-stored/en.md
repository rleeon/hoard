---
title: "Where are PC game saves stored? Every common location"
description: "Where PC games keep their save files on Windows, Steam Deck, Linux and Mac, which launchers add their own folders, and how to find any game's saves fast."
order: 11
updated: 2026-10-09
related: back-up-game-saves, transfer-game-saves-to-new-pc, onedrive-game-saves, sync-saves-steam-deck-pc
---

There's no single folder. On Windows, almost every game saves in one of six places: `Documents`, `Saved Games`, one of the three `AppData` folders, Steam's `userdata`, or its own install folder. The engine and the developer decide which, not the store you bought it from. This page lists every common location, the launchers that add a layer of their own, and a quick way to find any game's saves, even one nobody has documented.

## Windows: the six usual places

| Folder | Typical path | Who uses it |
|---|---|---|
| Documents | `%USERPROFILE%\Documents\My Games\<Game>` | Bethesda games, Rockstar (`Documents\Rockstar Games`), many older big releases |
| Saved Games | `%USERPROFILE%\Saved Games\<Publisher>\<Game>` | Cyberpunk 2077 and a stubborn minority |
| AppData\Roaming | `%APPDATA%\<Game>` | Elden Ring, Stardew Valley, Minecraft (`.minecraft`), lots of indies |
| AppData\Local | `%LOCALAPPDATA%\<Game>\Saved\SaveGames` | Unreal Engine games (Palworld is `Pal\Saved\SaveGames`) |
| AppData\LocalLow | `%USERPROFILE%\AppData\LocalLow\<Company>\<Game>` | Unity games (Hollow Knight and many more) |
| Steam userdata | `C:\Program Files (x86)\Steam\userdata\<UserID>\<AppID>\remote` | Games that use Steam's own save storage |

And a seventh that refuses to die: **the game's install folder**, which is where plenty of older games and some indies still write.

Two practical notes. `AppData` is hidden, so type `%APPDATA%` or `%LOCALAPPDATA%` into the Explorer address bar instead of clicking your way there. And if your `Documents` folder is backed up by OneDrive, its real path is `C:\Users\<you>\OneDrive\Documents`, which surprises a lot of people. See [OneDrive and game saves](/guides/onedrive-game-saves).

## Launchers that add their own layer

Most launchers don't decide where saves go; the game does. A few exceptions:

- **Steam** keeps a save area per game in `userdata`. `<UserID>` is a number tied to your Steam account (there's one folder per account that has signed in on that PC) and `<AppID>` is the number in the game's store URL.
- **Ubisoft Connect** puts saves in `C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\savegames\<user ID>\<game ID>`, with numbers instead of names on both levels.
- **The Xbox app and PC Game Pass** use `%LOCALAPPDATA%\Packages\<package>\SystemAppData\wgs`, where files have random names and only make sense to the Xbox app. Leave these to Xbox cloud saves; copying them by hand rarely works.
- **Epic, GOG and the EA app** generally leave it to the game, so their titles land in the usual places above. Their cloud saves, where they exist, copy from there.

## The registry, rarely

A few games, mostly small Unity titles, keep progress in the Windows registry under `HKEY_CURRENT_USER\Software\<Company>\<Game>` instead of a file. There's nothing to copy in a folder, and folder-based backup tools, Hoard included, don't see it. If you need it, export that key with `regedit`.

## Steam Deck and Linux

- **Windows games through Proton** save inside a per-game prefix: `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, then the Windows path from the table (`Documents`, `AppData/Roaming`, and so on). Games on a microSD card have the same tree under the card's own `steamapps/compatdata`.
- **Native Linux games** use `~/.local/share/<game>` or `~/.config/<game>`. Unity games go to `~/.config/unity3d/<Company>/<Game>`.
- **Steam userdata** is at `~/.local/share/Steam/userdata/<UserID>/<AppID>/remote`.
- **Heroic, Lutris and Bottles** each keep a Wine prefix per game. Inside, the Windows tree is under `drive_c/users/<your user>/`, not `steamuser`.

The Deck has its own guide: [syncing saves between Steam Deck and PC](/guides/sync-saves-steam-deck-pc).

## Mac

- **Most games:** `~/Library/Application Support/<Game>`. Unity games use `~/Library/Application Support/<Company>/<Game>`.
- **Mac App Store games** are sandboxed: `~/Library/Containers/<bundle id>/Data/Library/Application Support/`.

`~/Library` is hidden too. In Finder, open the **Go** menu while holding Option and it appears.

## How to find any game's saves

When a game isn't on any list, three tricks find it in a couple of minutes:

1. **Look it up on PCGamingWiki.** Almost every game page has a "Save game data location" section. It's the same source the save databases behind Hoard and Ludusavi are built from.
2. **Watch what changes.** Save in the game, quit, and search your user folder for files changed in the last few minutes. On Windows, search for `datemodified:today` inside `C:\Users\<you>` and sort by date. On Linux or a Deck: `find ~ -type f -mmin -5 -not -path '*/.cache/*'`.
3. **Ask Steam.** For a Steam Cloud game, [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) lists the files Steam keeps for each game, with names and sizes. Once you know what the file is called, finding the folder is easy.

## Or let something find them for you

Hoard reads that same community save database, covering thousands of games, and checks every candidate path on your machine: Proton, Heroic and Lutris prefixes, OneDrive's `Documents`, emulators, portable installs. Whatever it finds is backed up automatically each time you stop playing, with every version kept, and kept in sync across your PCs and Steam Deck. Anything it misses you can add by pointing at the folder once. See [how to back up your game saves automatically](/guides/back-up-game-saves).

There are also pages with exact paths for a few popular games: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location), [Palworld](/guides/palworld-save-location), [Crimson Desert](/guides/crimson-desert-save-location) and [Marvel's Spider-Man 2](/guides/spider-man-2-save-location).

<!-- faq -->

## Frequently asked questions

### Where does Steam keep game saves?

It depends on the game. Some use Steam's own area, `Steam\userdata\<UserID>\<AppID>\remote`; most write to `Documents`, `AppData` or `Saved Games` like any other game, and Steam Cloud copies them from there.

### Why can't I find the AppData folder?

It's hidden. Type `%APPDATA%` (Roaming) or `%LOCALAPPDATA%` (Local) in the Explorer address bar or the Run box (Win + R). `LocalLow` sits next to `Local`.

### Are saves in the same place for the Steam, GOG and Epic versions?

Usually, because the game decides, not the store. There are exceptions: some games add a folder named after your account ID, and a few store versions use a different folder name. Check before copying saves from one version to another.

### Where are Xbox app and Game Pass saves?

In `%LOCALAPPDATA%\Packages\<package>\SystemAppData\wgs`, as files with random names that only the Xbox app understands. They're synced by Xbox cloud saves; copying them by hand rarely works.

### My Documents folder is inside OneDrive. Is that a problem?

It can be. Games follow the folder into OneDrive, and OneDrive then syncs saves while games write them. See [OneDrive and game saves](/guides/onedrive-game-saves).
