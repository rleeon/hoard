---
title: "Ludusavi alternative: automatic cloud sync for your game saves"
description: "A fair comparison of Ludusavi and Hoard. Ludusavi is a great open-source local backup tool; Hoard adds managed cloud sync and versioned history across all your PCs — using the same save-location data."
order: 5
updated: 2026-09-01
---

If you're looking for a way to back up and sync your game saves, you've probably found **Ludusavi** — and it's excellent. This guide is an honest comparison so you can pick the right tool, and it explains where Hoard fits if you want automatic cloud sync across machines.

## What Ludusavi does well

Ludusavi is a free, open-source tool (made by mtkennerly) for backing up and restoring PC game saves on Windows, macOS and Linux. It has a clean GUI and a CLI, finds saves for thousands of games automatically, keeps versioned local backups, and can push those backups to a cloud you own by configuring **Rclone** (Google Drive, Dropbox, and many others). If you want full control and a do-it-yourself setup, Ludusavi is a fantastic choice — and it's completely free.

Hoard isn't here to replace that. In fact, **Hoard uses the same community save-location database that Ludusavi relies on** to locate where each game stores its saves, so detection quality is on par.

## Where Hoard is different

The gap most people hit with any local-first tool is **syncing across devices**. With Ludusavi you do it yourself: schedule a backup, configure an Rclone remote, then restore on the other PC before you play. That works, but it's manual.

Hoard turns that into **managed cloud sync**:

- **Sign in and go.** No Rclone remotes, no scripts. Hoard uploads your save after you finish playing and downloads the latest before you start, on every PC on your account.
- **Versioned history in the cloud.** Every backup is kept, so you can roll back to any earlier save — even after a disk failure or a fresh install.
- **Conflict-aware.** Hoard compares timestamps and keeps a local copy of anything it replaces, so a sync never silently destroys progress.
- **Still open source and self-hostable.** Like Ludusavi, you're not locked in — run Hoard Cloud or host the server yourself.

## Side by side

| | Ludusavi | Hoard |
|---|---|---|
| Local backups | Yes | Yes |
| Save detection | Community manifest | The same manifest, plus Steam libraries, running processes and a filesystem scan |
| Cloud storage | Bring your own, through Rclone | Included, or your own server |
| Sync between PCs | Manual: back up here, restore there | Automatic, after you stop playing and before you start |
| Version history | Local backups you prune yourself | Every version kept in the cloud, deduplicated by content hash |
| Emulators | Yes | Yes |
| Interfaces | Desktop app and CLI | Desktop app, CLI, and an in-game overlay |
| Price | Free | Free tier of 2 GB and 3 devices, Pro above that, no quota at all if you self-host |
| Licence | MIT | AGPL-3.0 |

## When Ludusavi is the better choice

This is the part most comparison pages skip. Ludusavi is the better tool when:

- **You only play on one PC.** Cloud sync solves a problem you don't have. A local backup is enough, and Ludusavi does local backups very well.
- **You already have an Rclone remote you trust.** If your storage is wired up and working, Hoard's main advantage is a setup step you've already paid for.
- **You want to run it from Game Mode on a Steam Deck.** Ludusavi has a Decky plugin, so you can trigger a backup without leaving the console interface.
- **You want a permissive licence.** Ludusavi is MIT, Hoard is AGPL-3.0. If you intend to build something on top and not publish the result, that difference matters.
- **You don't want anything running.** Self-hosting Hoard means keeping a small server up somewhere, even if it's the same PC. Ludusavi is an app you open when you want it.

## Moving from Ludusavi to Hoard

There's no importer, and that's on purpose. The steps:

1. **Leave your Ludusavi backups exactly where they are.** Nothing is migrated or deleted. Keep them as a safety net for the first few weeks.
2. **Install Hoard and sign in**, or point it at your own server.
3. **Let it scan.** It reads the same manifest, so the list of detected games should look familiar.
4. **Don't point Hoard at your Ludusavi backup folder.** Track the folder the game itself writes to. A backup folder is a copy that changes on a schedule rather than when you play, and syncing a copy of a copy is how you end up restoring yesterday's progress. Hoard tries to catch this on its own — `hoard doctor` flags a tracked folder that looks like a backup mirror — but it's easier never to track it.
5. **Play once.** When you quit, the first version appears in the history.
6. **Repeat on the second PC.** Sign in there and the versions are already waiting.

## Two details worth knowing

**Steam saves live one folder deeper than you think.** For Steam games, Hoard tracks `<AppID>/remote/` inside `userdata`, not the folder above it. The parent also holds `remotecache.vdf` and achievement and playtime files, and those legitimately differ from machine to machine. Sync the parent and every launch looks like a conflict even though no save actually moved. It's the most common reason a hand-rolled Steam Deck ↔ desktop setup ends up fighting itself.

**Versions are cheap.** Snapshots are stored by content hash, so unchanged files are stored once. Ten versions of a 2 GB save cost about 2 GB, not 20 — which is what makes keeping the full history practical instead of pruning it.

## What self-hosting actually means

This is the point most comparisons get wrong about Hoard, so it's worth being exact. There are two ways to run it, and they are genuinely different:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **Self-hosting is entirely yours.** You run `hoard-server` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, for the simple reason that none of it ever reaches us. If Hoard Cloud disappeared tomorrow, a self-hosted setup would carry on unchanged.

Same program, same detection, same version history. The only thing that changes is who owns the storage.

## Which should you choose?

- Choose **Ludusavi** if you want a free, local-first backup tool and you're happy to wire up your own cloud with Rclone.
- Choose **Hoard** if you want backups *and* automatic sync across PCs to just work, with a versioned cloud history, while keeping the option to self-host.

Many people start with Ludusavi for local backups and move to Hoard once they're playing the same games on more than one machine. If that's you, see [how to sync game saves across PCs](/guides/sync-game-saves-across-pcs) or just [download Hoard](/download) and sign in. For the wider field, there's a [comparison of every save sync tool](/guides/game-save-sync-comparison).

<!-- faq -->

## Frequently asked questions

### Can I use Ludusavi and Hoard at the same time?

Yes. They read the same save locations and neither one holds the files open. Plenty of people keep Ludusavi for local archive backups and let Hoard handle sync between machines. The only rule is not to point either tool at the other's backup folder.

### Does Hoard import my Ludusavi backups?

No, and that's deliberate. A backup folder is a copy that changes on its own schedule, so tracking it would sync a stale mirror instead of your live save. Hoard tracks the folder the game writes to and starts its own history from your next session. Keep the Ludusavi archive as a safety net.

### Is Hoard free?

Hoard Cloud has a free tier with 2 GB of storage and 3 devices, which covers most save collections; Pro raises both. Self-hosting the server is free and has no quota at all. Everything is open source under AGPL-3.0.

### Does Hoard work on Steam Deck?

Yes, on Steam Deck and any Linux desktop, as well as Windows and macOS. The Deck is exactly the case that needs the `remote/` detail above, because a Deck and a desktop write different achievement and playtime files next to the same save.

### Do I need Rclone or a cloud account of my own?

No. That's the main practical difference: with Hoard Cloud, storage is already set up when you sign in. If you'd rather own the storage, run the server yourself against an S3-compatible bucket or a plain folder on your own machine.

### Does self-hosting send anything to Hoard?

No. In self-hosted mode there is no account with us and no telemetry to us: your saves, your users and your logs live on your own server and never touch ours. That's the whole point of the mode, and it's why the server is the same open-source binary we run ourselves rather than a cut-down version.
