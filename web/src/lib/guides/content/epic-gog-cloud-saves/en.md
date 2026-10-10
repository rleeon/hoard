---
title: "Epic and GOG cloud saves: what they cover and how to sync the rest"
description: "Epic and GOG cloud saves work only for some games, only in their launchers, and keep no history. What they cover, and how to sync the rest automatically."
order: 19.5
updated: 2026-10-09
related: steam-cloud-alternative, sync-game-saves-across-pcs, rog-ally-legion-go-save-sync, where-are-pc-game-saves-stored
---

Both Epic and GOG have cloud saves, with the same catches as Steam: the developer has to support them game by game, they only work through the store's own launcher, and they keep the latest copy instead of a history. Here's what each one covers, where the gaps are, and how to keep every game in sync between your PCs and a Steam Deck regardless of where you bought it.

## Epic Games Store

The Epic launcher has a cloud saves switch in its settings, and games that support cloud saves sync through it when you play on another PC. Support is per game: the developer has to implement it, and plenty of games in the store never did.

On Linux and the Steam Deck there's no official Epic launcher. Heroic can sync Epic cloud saves for games that support them, but you have to turn it on for each game.

## GOG

GOG Galaxy syncs cloud saves for the games that list "Cloud saves" among their features on the store page. Two catches are specific to GOG:

- **Only through Galaxy.** GOG's offline installers, the DRM-free part of the appeal, have no cloud at all. Play the installer version and your saves stay on that PC.
- **Per game and per platform.** A game only syncs between platforms where the developer set it up.

As with Epic, Heroic can sync GOG cloud saves on Linux and the Steam Deck when you enable it per game.

## Ubisoft, EA and the rest

Ubisoft Connect and the EA app have cloud saves for many of their own games, and each runs only in its own launcher. Amazon Games and the smaller stores vary game by game.

## What none of them do

- **History.** Every launcher keeps the current save. If a save breaks and syncs, the good one is gone everywhere.
- **Cross-store.** The same game bought on Steam for one PC and on GOG for another has two separate clouds that never talk.
- **Everything outside the launcher.** Emulators, DRM-free installers, games you installed by hand.
- **Games without support.** If the developer didn't implement it, the launcher can't help.

## Syncing the rest

Hoard works by game, not by store. It finds each game's save folder from a community database covering thousands of titles, wherever the game came from, backs it up automatically when you stop playing and syncs it to your other PCs and your Steam Deck, with every version kept.

That covers the gaps above:

- **Any launcher, or none.** Epic, GOG, Galaxy or the offline installer, Heroic on Linux, a game you unzipped into a folder.
- **Across stores.** Most games save in the same place whatever store sold them, usually under `AppData` or `Documents`, so a GOG install on one PC and a Steam install on another can share a save. A few add an account-ID folder or use a different name per store; check before relying on it.
- **A history.** Every session is a version you can roll back to.

Where a launcher's own cloud already syncs a game, let it keep doing that. Hoard adds the history, and does the syncing for everything else.

If you'd rather not use anyone's cloud, run `hoard-server` on your own PC or NAS. No account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

<!-- faq -->

## Frequently asked questions

### Does Epic have cloud saves?

Yes, for games whose developer implemented them, through the Epic launcher. Many games in the store don't support them, and the launcher keeps no history of earlier saves.

### Does GOG have cloud saves?

Yes, through GOG Galaxy, for games that list cloud saves on their store page. The offline installers don't sync at all.

### Do Epic or GOG keep old versions of my saves?

No. Both keep the latest copy only. To go back to an earlier save you need a backup that keeps versions.

### Can I move a save from the GOG version to the Steam version?

Often, yes: most games save in the same folder whichever store sold them. Some add an account-ID folder or use a different folder name, so check the paths first.

### Do GOG offline installers sync saves?

Not through GOG, since the cloud only works in Galaxy. Hoard syncs them like any other game, because it tracks the save folder rather than the launcher.
