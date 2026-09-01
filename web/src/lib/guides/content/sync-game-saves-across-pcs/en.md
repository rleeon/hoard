---
title: "How to sync game saves across multiple PCs"
description: "Play the same game on your desktop and laptop without losing progress. Sync your game saves across PCs automatically with Hoard — managed cloud sync without wiring up Ludusavi and Rclone by hand."
order: 2
updated: 2026-09-01
---

If you play on more than one computer — a desktop at home and a laptop on the go — Hoard keeps your saves in sync so you always pick up where you left off.

## How sync works

Hoard backs up each save to your cloud and pulls the latest version down on your other machines. When you finish playing on one PC, the newest save is waiting on the next one.

## Set up sync

1. Install **Hoard** on every PC you play on (Windows, macOS or Linux).
2. Sign in with the **same account** on each machine, or connect them to the same self-hosted server.
3. Add the same games to your **Library** on each PC. Hoard matches them by game, so a save backed up on one shows up on the others.
4. Keep **automatic mode** on. Hoard uploads after you play and downloads the latest before you start.

## Coming from Ludusavi?

Ludusavi is a great open-source tool for backing up and restoring saves locally, and it can push those backups to a cloud you configure yourself with Rclone. But syncing across devices is something you wire up manually: schedule the backup, set up the remote, then restore on the other PC before you play.

Hoard turns that into managed sync. It uses the same community save-location data as Ludusavi to find your saves, then uploads after each session and downloads the latest before the next one — across every PC on your account, with versioned history in the cloud. No Rclone remotes, no scripts. And like Ludusavi, Hoard is open source and can be self-hosted. See the full [Ludusavi alternative comparison](/guides/ludusavi-alternative).

## Avoiding conflicts

Hoard is conflict-aware: it compares modification times and keeps a local copy of any replaced save, so a sync never silently destroys progress. If a game is still running or a save was touched in the last few minutes, Hoard waits.

## Steam Deck and desktop

The most common two-machine setup is also the one that breaks most often when it's wired by hand, and nearly always for the same reason.

On Windows, a game's save might sit in `Documents\My Games\…` or inside Steam's `userdata`. On a Steam Deck, that same Windows game runs through Proton, so its save lives inside a compatibility prefix: `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…`. Two very different paths, one game, one run of progress. Hoard reads the Proton prefixes as well as the native locations and matches what it finds by game, so the Deck save and the desktop save become two versions of one history instead of two unrelated folders.

The detail that decides whether any of this works: for Steam games Hoard tracks `<AppID>/remote/` inside `userdata`, **not** the folder above it. The parent also holds `remotecache.vdf` and per-machine achievement and playtime files, which are supposed to differ between your Deck and your desktop. Sync the parent and every launch looks like a conflict even though no save actually moved. That single mistake is what makes most hand-rolled Deck ↔ PC setups feel broken.

## Games Steam Cloud doesn't cover

If every game you played supported Steam Cloud, you wouldn't need any of this. In practice:

- **Games from anywhere but Steam.** GOG, Epic, itch, Battle.net, the Xbox app, and anything you installed by hand.
- **Steam games where the developer never turned it on**, or turned it on for one platform only.
- **Emulators.** RetroArch, Dolphin, PCSX2, RPCS3 and the rest save where they like, and Steam knows nothing about it.
- **Games that write outside the folder Steam watches**, which is more of them than you'd expect.

Hoard doesn't care who published a game or where it came from. It tracks the folder that changes when you play.

## When two PCs edit the same save

Play on the laptop without letting the desktop finish syncing and you get the classic problem: two saves, both newer than the last common version.

Hoard never overwrites blind. It compares modification times, keeps a local copy of whatever it replaces, and holds off while a game is running or the save was touched in the last few minutes — a save file being written is not a save you want to upload halfway. Every earlier version stays in the cloud history, so picking the wrong one costs you two clicks, not a weekend.

The honest limit: **Hoard does not merge two divergent saves.** No tool can — a save file is opaque, and there is no correct way to blend two different afternoons of play. What you get instead is every version, on every machine, and the ability to choose.

## Syncing without our servers

Worth being explicit, because it's the part most comparisons get wrong. There are two ways to run this:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **Self-hosting is entirely yours.** You run `hoard-server` on your own PC or NAS and your machines sync through it. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same version history. The only thing that changes is who owns the storage.

## Tip

Give each machine a moment to finish syncing before you launch a game — the dashboard shows live status, so you know the latest save is in place.

<!-- faq -->

## Frequently asked questions

### How many PCs can I sync?

Three on the free tier, unlimited on Pro, and unlimited when you self-host — your server, your rules.

### Do both machines have to be online at the same time?

No. Your save goes up to the server when you finish playing and comes down when the other machine asks for it, so the second PC can be switched off for a week and still get the latest version when it wakes up.

### What if I play offline?

Fine. The snapshot is taken locally when you stop playing, and it uploads on its own once the machine has a connection again.

### Does it sync my mods and settings too?

Saves, yes. Files that belong to one machine — configuration, logs, and similar — are uploaded so they're in the backup, but are not written back over another PC's copy, because a graphics setting that suits your desktop is rarely the one your laptop wants.

### Does self-hosting send anything to Hoard?

No. In self-hosted mode there is no account with us and no telemetry to us: your saves, your users and your logs live on your own server and never touch ours.
