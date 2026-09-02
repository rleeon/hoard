---
title: "Game save sync compared: Hoard vs Ludusavi, Syncthing, OpenSave and the rest"
description: "An honest comparison of the tools that back up and sync PC game saves — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync and Hoard — with a table, and a section on where Hoard loses."
order: 4
updated: 2026-09-01
---

Steam Cloud only covers games you bought on Steam, and only when the developer bothered to switch it on. Emulators, GOG, Epic, itch.io, non-Steam games, anything modded — none of that is covered. If you play on more than one machine, a desktop and a Steam Deck say, you end up copying folders by hand and hoping you grabbed the newest one.

Several tools fix this, and they don't all do the same thing. Some make local backups, some mirror folders between devices, some upload to a cloud. This page goes through them and says what each one is genuinely best at. Hoard is my project, so the honest part comes at the end: a section on where Hoard loses, and a table you can read without trusting a word of the prose.

## Ludusavi

The best-known one, and deservedly so. Ludusavi (by mtkennerly) is a free, open-source backup tool with a GUI and a CLI, and it's built on the community save-location manifest that covers tens of thousands of games — the same manifest most of the tools here use, Hoard included. It keeps versioned local backups and can push them to your own cloud through Rclone.

**Best if:** you want local backups, full control, and no server anywhere. It's the safest default on this list and costs nothing.

**Where it stops:** cross-machine sync is a thing you assemble. Schedule a backup, configure an Rclone remote, remember to restore on the other PC *before* you play. It works, but nothing stops you forgetting the last step.

## Syncthing

Not a game tool at all — a general-purpose, peer-to-peer folder mirror, and a very good one. Point it at a save folder and it appears on your other devices.

**Best if:** you already run it and you want files in two places with no cloud in between.

**Where it stops:** it mirrors, it doesn't snapshot. A corrupted save reaches every device in seconds, exactly as fast as a good one. Its file versioning is per-file, with no idea what a play session is, so "roll back to how it was on Tuesday night" is something you reconstruct by hand. Two machines that both played offline give you conflict files, not a merge.

## OpenSave

Peer-to-peer sync built specifically for saves, in Go, MIT licensed, for Windows, Linux and Steam Deck. No account, no server: devices pair with each other and sync over the LAN or through a relay room code. It snapshots every change, has branches for parallel playthroughs, resolves conflicts by sync lineage rather than clock timestamps, and transfers only changed blocks. It can optionally mirror to Drive, Dropbox, OneDrive or WebDAV.

**Best if:** you refuse to have an account, and your devices are on together often enough to actually meet.

**Where it stops:** peer-to-peer means the save lives only on your devices. If the Deck holding the only recent copy dies and the mirror was never configured, that's it. Both devices have to be running for a sync to happen, and there's no macOS build.

## OpenCloudSaves

A cross-platform GUI that syncs your save folders into a cloud you already pay for — OneDrive, Google Drive, Dropbox, Nextcloud — using Rclone underneath.

**Best if:** you want your saves in a storage account you already have, with a UI instead of Rclone config files.

**Where it stops:** there's no content-level deduplication. Ten copies of a 2 GB save is 20 GB of your Drive quota, and cloud drives sync files, not play sessions, so what you get back is whatever the folder looked like at the time.

## Game Backup Monitor

Windows-first, and the original of this whole genre. GBM watches for a game process, and when you quit, it compresses the save with 7-Zip and keeps a numbered history.

**Best if:** you're on one Windows PC and want a compressed local archive with zero thinking.

**Where it stops:** it's a backup tool, not a sync tool. Getting the archive onto a second machine is your problem, and Steam Deck / SteamOS is not its home turf.

## Aletheia

The newest of the bunch, AGPL, and it goes after the part everyone else half-covers: launchers. Heroic, itch.io, Lutris, Steam, GOG Galaxy and Xbox, across Windows, Linux and macOS.

**Best if:** your library is spread across launchers that other tools detect badly — especially Xbox/Game Pass and Heroic.

**Where it stops:** it's a young project with a deliberately narrow scope. Backup and restore is the feature set; there's no versioned cloud behind it.

## SaveSync

The commercial one, sold on Steam as a one-time purchase, Windows-focused. Its trick is that it isn't really aimed at you-on-two-PCs — it's aimed at co-op. Saves go into private, unlisted Steam Workshop entries so a friend can pull your Valheim or Factorio world, and there's LAN sync too.

**Best if:** the problem you're solving is "my friend hosts and I need their save", not "my saves follow me".

**Where it stops:** closed source, Windows, tied to Steam as the transport, and a set of supported co-op games rather than everything you own.

## A note on EmuDeck

EmuDeck comes up in these conversations, and it isn't a competitor in the normal sense — it's an emulator installer and configurator for Steam Deck, and the sync it offers is a convenience bolted onto that job (Rclone against a cloud drive, for emulator saves only). It overlaps with the tools above without being the same kind of thing: EmuDeck sets your emulators up, the tools here look after saves for the whole library. People do run EmuDeck alongside one of these, and that's a sensible setup, not a redundant one.

## Hoard

Hoard treats a play session as the unit. The engine runs as a background service — `hoardd`, no window, so it works in SteamOS game mode — notices you stopped playing, and takes a snapshot then, instead of reacting to every file write mid-game.

- **Version history per session.** Every session is a version you can roll back to, including after a disk failure or a fresh install.
- **Content-hash deduplication.** Ten versions of a 2 GB save cost about 2 GB, not 20 GB. Transfers are zstd-compressed.
- **SHA-256 on the way up and on the way down.** Corruption is caught before it can overwrite a good save. Nothing is ever silently overwritten — that's the whole design.
- **Cloud or self-hosted, same binary.** Hoard Cloud has a free tier (2 GB, 3 devices, full history). Or run `hoard-server` yourself with Docker Compose against any S3-compatible storage — MinIO, Garage, Backblaze B2 — with no account and no quota. AGPL-3.0.
- **Windows, Linux, macOS**, plus a headless CLI for a Steam Deck or a server.
- **Emulators in beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP and others as presets.

## The detail that decides Steam Deck ↔ PC sync

Worth knowing whichever tool you pick. A Steam game's cloud save lives in `<AppID>/remote/`, and the folder *above* it holds `remotecache.vdf`, achievement state, stats and playtime counters — all of which legitimately differ between your Deck and your desktop.

Sync the parent folder and you get a permanent conflict between two machines that never disagreed about a single save. Hoard tracks `remote/`, not the parent. Any tool pointed at a folder by hand can be told to do the same, and it's the first thing to check when a sync setup keeps flagging conflicts for no visible reason.

## Where Hoard loses

- **It wants a server.** Cloud account or your own box — either way it's infrastructure, and OpenSave or Ludusavi need none.
- **Emulator support is beta.** Portable installs and per-emulator quirks still catch it out; Aletheia and OpenSave cover some launcher/emulator edge cases better today.
- **macOS is barely tested on real hardware.** It builds and it runs, but nobody has lived on it for months.
- **It's young.** Ludusavi and Game Backup Monitor have years of bug reports behind them. Hoard doesn't, and that matters for something guarding a 200-hour save.
- **It doesn't do co-op sharing.** If you want to hand a world to a friend, SaveSync is built for that and Hoard isn't.

## The Hoard Cloud / self-host distinction

Comparisons of Hoard almost always collapse these two into one, and the result is wrong, so it's worth stating plainly:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **A self-hosted Hoard is entirely yours.** You run `hoard-server` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, because none of it ever reaches us. If Hoard Cloud shut down tomorrow, a self-hosted setup would carry on unchanged.

Same binary, same detection, same version history. The only thing that changes is who owns the storage. Being exact about one detail: your own server does have logins of its own — a user and a token per device — but they live in your database, not ours.

## The table

| Tool | Automatic sync between devices | Where saves live | History | Platforms | Licence |
|---|---|---|---|---|---|
| **Hoard** | Yes, per play session | Hoard Cloud or your own server (S3-compatible) | Versioned per session, deduplicated | Win · Linux · macOS · Deck | AGPL-3.0, free tier |
| **Ludusavi** | Manual, or Rclone that you wire up | Local, plus your Rclone remote | Versioned local backups | Win · Linux · macOS | Free, open source |
| **Syncthing** | Yes, continuous mirror | Your devices only | Per-file versioning | Everything | Free, open source |
| **OpenSave** | Yes, peer-to-peer | Your devices, optional cloud mirror | Snapshots and branches | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Yes, via your cloud drive | OneDrive / Drive / Dropbox / Nextcloud | Whatever the drive keeps | Win · Linux · macOS | Free, open source |
| **Game Backup Monitor** | No | Local 7-Zip archives | Numbered backups | Windows | Free, open source |
| **Aletheia** | Backup and restore per launcher | Your storage | Backups | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Yes, and with friends | Private Steam Workshop entries | Per the app | Windows | Paid, closed source |

## So which one

If you want one machine backed up and nothing else, take Ludusavi or Game Backup Monitor. If you want no account under any circumstances and your devices are usually on together, OpenSave. If your saves should be in a Drive folder you already pay for, OpenCloudSaves. If you're sharing a co-op world with friends, SaveSync.

If you want backups *and* automatic sync across PCs and a Steam Deck to just happen, with a version per session you can roll back to and the option to self-host the whole thing, that's what Hoard is for. [Download it](/download), or read [how to self-host it with Docker](/guides/self-host-hoard) first. There's also a longer [Ludusavi comparison](/guides/ludusavi-alternative) if that's the one you're weighing it against.

<!-- faq -->

## Frequently asked questions

### Which of these tools keeps a version history?

Hoard keeps every session as a version you can roll back to. Ludusavi keeps versioned local backups. Most of the rest sync or copy the current state, which means a corrupted save is faithfully propagated to your other machine.

### Which one works without any server or account?

Ludusavi with local backups, and any peer-to-peer tool. Hoard also qualifies if you self-host: no account with us, and nothing passing through our servers.

### Which one covers games that aren't on Steam?

All the save-manager tools here do, because they locate saves through the same community database rather than through a store. Steam Cloud is the one that doesn't: it only covers Steam games whose developer enabled it.

### Do I have to pick just one?

No, and plenty of people don't. A local backup tool and a sync tool solve different halves of the problem. The only rule is never to point one tool at another's backup folder, or you end up syncing a stale mirror instead of your live save.

### What's the single detail that breaks most DIY setups?

Syncing the folder above `<AppID>/remote/` in Steam's `userdata`. The parent holds `remotecache.vdf` plus achievement and playtime files that are supposed to differ per machine, so every launch looks like a conflict even though no save moved.
