---
title: "Steam Cloud alternative: back up the saves Steam doesn't"
description: "Steam Cloud only covers Steam games whose developer switched it on, and it keeps no version history. Hoard backs up every game you play, from any launcher, with a versioned history you can roll back — in the cloud or on your own server."
order: 7
updated: 2026-09-01
---

Steam Cloud is genuinely good at the narrow job it does, and most people only find its edges the day they lose something. This guide explains exactly where those edges are, and what to do about the games that fall outside them.

## What Steam Cloud actually covers

Steam Cloud syncs a folder for a game when **the developer set it up** — either by declaring which files to sync, or by calling the Steam API from inside the game. That's the whole model, and three things follow from it:

- It only works for games bought and launched through Steam.
- Whether it works at all is the developer's decision, per game, and sometimes per platform.
- Each game has its own storage allowance, set by that developer.

When it works, it's invisible and excellent: you close the game on one PC, open it on another, and your progress is there.

## Where it leaves you exposed

- **Everything that isn't a Steam game.** GOG, Epic, itch, Battle.net, the Xbox app, emulators, anything installed by hand. Steam doesn't know they exist.
- **Steam games where it was never switched on.** Plenty of titles, especially older or smaller ones, simply don't have it. The store page tells you, but nobody checks before starting a 60-hour run.
- **There is no going back.** This is the big one. Steam holds the current state of your save, not a history of it. Corrupt the file, let a mod eat your world, or overwrite a good save with a bad one, and the cloud copy is already the bad one. You can browse the files Steam is holding for a game, but there's no earlier version to restore.
- **The conflict dialog.** When Steam thinks the local and remote saves disagree, it asks you to choose, with little more than two timestamps to go on. Choose wrong and the other copy is gone.

## What Hoard adds

Hoard watches the folder each game actually writes to and captures a **new version every time you finish playing**:

- **It doesn't care where a game came from.** Steam, GOG, Epic, itch, emulators, a folder you pointed it at by hand.
- **Every version is kept**, so rolling back a corrupted save or a bad decision is two clicks rather than a lost run.
- **It syncs between your machines** the same way, including a Steam Deck and a desktop.
- **Nothing is destroyed silently.** The save being replaced is captured first, so even a wrong restore is reversible.

Snapshots are stored by content hash, so ten versions of a 2 GB save cost about 2 GB, not 20 — which is what makes keeping the whole history practical.

## Using both at once

They don't fight, and you don't have to pick. For a Steam game with cloud support, let Steam do the syncing it's already doing; Hoard's contribution there is the history — the thing Steam doesn't keep. For everything else, Hoard is doing the syncing too.

One detail that matters if you're on a Steam Deck as well as a desktop: Hoard tracks `<AppID>/remote/` inside `userdata`, not the folder above it, because the parent holds `remotecache.vdf` and per-machine achievement and playtime files. That's the distinction a hand-rolled sync usually gets wrong, and it's why those setups seem to conflict on every launch.

## When Steam Cloud is enough

Worth saying plainly: if every game you play is a Steam game with cloud support, you play on one PC, and you've never needed to undo a save, Steam Cloud already does the job and you don't need anything else. The case for adding Hoard is version history, games from outside Steam, and machines Steam Cloud doesn't reach.

## Without anyone's cloud

If the appeal is not depending on a platform at all, Hoard can be run entirely on your own hardware: `hoard-server` on a PC or a NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us and no relay** — nothing passes through our servers, because there is nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same version history. The only thing that changes is who owns the storage.

<!-- faq -->

## Frequently asked questions

### Does Hoard replace Steam Cloud?

It doesn't have to. Steam Cloud keeps your current save in sync for the games that support it; Hoard adds a version history and covers the games it doesn't. Running both is normal.

### Can Steam Cloud roll back to an older save?

No. Steam holds the current state of the files, not a history of them. Once a bad save has synced, that's what's in the cloud. A versioned tool is the only way to go back.

### Why don't all my Steam games sync?

Because it's the developer who enables it, per game and sometimes per platform. A game's store page lists Steam Cloud among its features when it's supported — and plenty of titles simply don't.

### Does Hoard work with non-Steam games?

Yes, that's most of the point. It locates saves through a community database covering 20,000+ titles, from any launcher, and you can point it at a folder by hand for anything unusual.

### Will running both cause conflicts?

No. Hoard captures a version after you stop playing, once the folder goes quiet, and never overwrites without capturing what it replaces first.

### Can I keep my saves off both clouds?

Yes. Self-host the server and your saves never leave hardware you own, with no account and no telemetry going anywhere.
