---
title: "How to sync saves between Steam Deck and PC"
description: "Keep Steam Deck and PC saves in sync automatically, including non-Steam games, emulators and games without Steam Cloud. Setup, paths and traps."
order: 10
updated: 2026-10-09
related: sync-game-saves-across-pcs, retroarch-save-sync, steam-cloud-alternative, where-are-pc-game-saves-stored, rog-ally-legion-go-save-sync, dual-boot-game-saves
---

For Steam games with Steam Cloud, your Deck and your PC already share saves. Everything else needs help: games where the developer never turned Steam Cloud on, Epic and GOG games running through Heroic, emulators, and anything you added as a non-Steam game. Hoard covers all of them automatically. When you quit a game on one machine it backs up the save, and the other machine pulls it down, with every earlier version kept in case something goes wrong.

## What Steam Cloud already does on the Deck

If a game supports Steam Cloud, Steam uploads the save when you quit and downloads it when you launch on another machine. You can see whether a game has it on its store page, and switch it off per game under **Properties → General**.

The gaps are the usual ones:

- **Games without it.** It's the developer's choice, game by game, and plenty of PC games never opted in.
- **Everything outside Steam.** Heroic, Lutris, emulators, a game you installed by hand.
- **No way back.** Steam keeps the current save, not a history. If a bad save syncs, the good one is gone on both machines.

There's more on this in the [Steam Cloud alternative](/guides/steam-cloud-alternative) guide.

## Where the Deck keeps your saves

The Deck runs Windows games through Proton, so the same game saves in a different place than it does on your PC:

- **Windows games (Proton):** `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, followed by the usual Windows path: `Documents`, `AppData/Roaming`, `AppData/Local`, `AppData/LocalLow` or `Saved Games`. The AppID is the number in the game's store URL.
- **Games on the microSD card:** the card has its own `steamapps/compatdata/<AppID>`, with the same tree inside.
- **Native Linux games:** usually `~/.local/share/<game>` or `~/.config/<game>`. Unity games use `~/.config/unity3d/<Company>/<Game>`.
- **Heroic, Lutris and Bottles:** each keeps its own Wine prefix per game, with the Windows tree under `drive_c/users/<your user>/` instead of `steamuser`.
- **Emulators:** EmuDeck gathers them under `~/Emulation/saves/`. See [emulator saves](/guides/back-up-emulator-saves) and [RetroArch](/guides/retroarch-save-sync).

On your PC, the same game writes to `C:\Users\<you>\...`. Two different paths for one save is exactly why copying folders by hand goes wrong. Hoard looks in all of these places and matches what it finds by game, so the Deck's save and the PC's save become two versions of one history.

## Set it up

1. On the Deck, switch to Desktop Mode: **Steam button → Power → Switch to Desktop**.
2. Open a browser, go to [the download page](/download) and get **Hoard Setup** for Linux. In the file manager, open the file's properties and allow it to run as a program, then open it.
3. Sign in with the account you use on your PC, or point the app at your own server.
4. Open the **Library** and check what Hoard found. Add anything missing by pointing at its folder: a Heroic prefix, an emulator, a game you installed yourself.
5. Install Hoard on your PC with the same account. The same games match up on their own.
6. Go back to Game Mode. You don't need to come back to Desktop Mode again.

Hoard Setup puts the app in your home folder and the sync engine in a background service that starts with the Deck. Nothing is written to SteamOS's read-only system, so system updates leave it alone.

## What a normal day looks like

You play on the PC in the evening and quit. Hoard waits until the game has closed and the save has stopped changing, then uploads it. The next morning you pick up the Deck. Once it's online, Hoard sees the newer version and writes it into the Proton prefix. You launch the game and carry on. When you quit on the Deck, the same thing happens in reverse.

Neither machine has to be on at the same time as the other. The save waits on the server until the other machine asks for it.

## The traps worth knowing

### Suspending is not quitting

The Deck makes it very easy to press the power button and walk away with the game still open. Hoard only backs up a save once the game has closed, because a game that's still running may be halfway through writing it. It also never swaps a save under a running game. So if you suspend on the Deck and then play on the PC, the Deck's progress isn't uploaded yet, and the PC's new save waits until you close the game on the Deck.

The habit that avoids all of it: **quit the game before you switch machines.** If Proton leaves a dead process behind after you quit, which happens often, Hoard notices that the game is gone and carries on.

### Give it a few seconds after waking

After the Deck wakes up, Wi-Fi takes a moment to come back, and only then can Hoard check for a newer save. Launch a game in those first seconds and the download waits until you close it again. Give it a moment online before you start playing.

### The microSD card

If a game lives on the card and the card isn't inserted, Hoard doesn't download a save into a folder that isn't there. It waits until the card is back.

### Settings stay on each machine

The Deck runs at 1280×800 on a handheld GPU. Your desktop probably doesn't. Hoard backs up settings files like `graphics.ini` along with the save, but doesn't write them over the other machine's copy, so the Deck keeps its own settings. If you want them copied anyway, there's an option for that when you restore. More in [syncing saves across PCs](/guides/sync-game-saves-across-pcs).

### Steam's `remote` folder

For Steam games, the save sits in `userdata/<UserID>/<AppID>/remote/`. The folder above it also holds `remotecache.vdf` plus playtime and achievement files that are supposed to differ between the Deck and your PC. Sync the parent folder by hand and every launch looks like a conflict. Hoard tracks `remote/` only.

## Steam Cloud and Hoard together

They don't get in each other's way. For a game with Steam Cloud, let Steam keep syncing it. What Hoard adds there is the version history, so a broken save on one machine doesn't take your progress with it. For every other game, Hoard does the syncing as well.

## Without our servers

If you'd rather keep saves at home, run `hoard-server` on your PC or a NAS and point both the Deck and the PC at it. No account with us, no telemetry to us, nothing passing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

<!-- faq -->

## Frequently asked questions

### Does Hoard work in Game Mode?

Yes. The sync engine runs as a background service that starts with the Deck, so it backs up and restores with no window open. You only need Desktop Mode to install it and to add folders by hand.

### Will a SteamOS update remove it?

No. Everything Hoard installs lives in your home folder, which SteamOS updates don't touch.

### Does it sync games from Heroic, Lutris or EmuDeck?

Yes. Hoard looks inside Heroic, Lutris and Bottles prefixes and in EmuDeck's folders. If a game isn't detected, point Hoard at its save folder once and it's tracked like any other.

### What if I played on both without syncing?

Hoard never overwrites blind. It compares versions, keeps a copy of whatever it replaces, and every earlier version stays in the history. It can't merge two different play sessions into one save (nothing can), but you can pick which one to keep.

### Does the Deck count as a device?

Yes. The free plan covers three devices, so a PC, a laptop and a Deck fit. Pro and self-hosted servers have no device limit.

### Can I use the command-line version on the Deck instead?

Yes. The `hoard` command runs the same engine without a window, which some people prefer on a handheld. See [the CLI page](/cli).
