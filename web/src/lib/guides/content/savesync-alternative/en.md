---
title: "SaveSync alternative: free, open-source save sync for every game"
description: "Want a free SaveSync alternative? Hoard syncs and backs up saves for every game, not only co-op ones, on Windows, Linux, macOS and Steam Deck."
order: 29
updated: 2026-11-05
related: game-save-sync-comparison, sync-game-saves-across-pcs, steam-cloud-alternative, sync-saves-steam-deck-pc, self-host-hoard
---

SaveSync is a paid app on Steam, in Early Access, built around co-op: it keeps a shared world available to the group through private Steam Workshop entries, even when the host is offline. If what you want is free sync of your own saves, for every game you play and across every machine you own, [Hoard](/download) does that, and it's open source.

## What SaveSync does well

- **Co-op worlds.** Handing a Valheim or Factorio world to whoever hosts tonight is the job it was built for.
- **Steam as the transport.** Saves travel as private, unlisted, encrypted Workshop entries, so there's no separate account to make.
- **Gets along with Steam Cloud.** It works next to Steam's own sync instead of fighting it.

## Where Hoard differs

| | SaveSync | Hoard |
|---|---|---|
| Price | Paid, on Steam | Free and open source (AGPL-3.0) |
| Games | A set of supported co-op games | Every game in the community save database, 19 emulators and any folder you add |
| Storage | 500 MB included, more sold in the app | Hoard One free with 2 GB and three devices, or your own server with no quota from us |
| History | Per the app | Every version, deduplicated |
| Platforms | Windows | Windows, Linux, macOS and SteamOS |
| Sharing a world with friends | Yes | No |

The last row is the honest one. If the point is passing a world between friends, SaveSync is built for that and Hoard isn't. For everything else, your own saves on your desktop, laptop and Steam Deck, Hoard covers more games and costs nothing.

## Set it up

1. Install Hoard and sign in to Hoard One, recommended for everyone, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. Your games are detected from your launchers and the community save database.
3. Play. Every save adds a version, and with Hoard on your other machines under the same account, the newest one is waiting there. See [how to sync saves between Steam Deck and PC](/guides/sync-saves-steam-deck-pc).

## Hoard One, or your own server

Hoard is free either way, and the app is the same on both:

- **Hoard One**, recommended for everyone. Nothing to set up and nothing to keep running: a managed server in the EU, free with 2 GB and three devices, no card needed. [Pro](/pricing) only adds room, 100 GB and unlimited devices, for when your saves outgrow that.
- **Your own server.** Run `hoard-server` on a PC, a NAS or a Raspberry Pi and every machine syncs through it. There's no account with us, no telemetry to us and no quota from us: nothing passes through our servers, because there's nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same history. Hoard One keeps every version for good; on your own server you decide how much history to keep.

<!-- faq -->

## Frequently asked questions

### Is there a free alternative to SaveSync?

Hoard. The app, the command line and the server are free and open source, and Hoard One, the managed server, is free with 2 GB and three devices. It doesn't share worlds between friends, but it syncs your own saves for every game.

### Can I send a save to a friend with Hoard?

Not as a feature: Hoard keeps your own saves in sync across your own machines. You can export your saves as the game wrote them and send the files yourself.

### Is Hoard free?

Yes. The app, the command line and the server are free and open source (AGPL-3.0), and running your own server costs nothing but the machine. Hoard One, the managed server we recommend for everyone, is free too with 2 GB and three devices; Pro adds room for €1.99 a month.

### Does self-hosting send anything to Hoard?

No. On your own server there's no account with us and no telemetry to us: your saves, your users and your logs stay on your machine and never touch ours.
