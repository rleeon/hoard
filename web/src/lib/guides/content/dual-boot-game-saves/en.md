---
title: "Sync game saves between Windows and Linux on a dual-boot PC"
description: "Same PC, two systems, two save folders. Keep game saves in sync between Windows and Linux automatically, why a shared NTFS folder breaks, and the traps."
order: 18
updated: 2026-10-09
related: sync-saves-steam-deck-pc, sync-game-saves-across-pcs, where-are-pc-game-saves-stored, steam-cloud-alternative
---

On a dual-boot PC, the same game keeps two separate saves: one in your Windows user folder, and one inside a Proton prefix on Linux. Steam Cloud bridges the two for games that support it; everything else drifts apart the first time you switch systems. Hoard keeps them in sync automatically. Install it on both systems with the same account, and each game's save follows you whichever one you boot.

## Why one game has two saves

The disk is shared, but the save folders aren't:

- **On Windows**, a game writes to `Documents`, `Saved Games` or one of the `AppData` folders under `C:\Users\<you>`.
- **On Linux**, the same Windows game runs through Proton and writes inside its own prefix: `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, followed by that same Windows path.

Two copies of one save, in two systems that never run at the same time. Whichever one you played last, the other system doesn't know about it. Hoard looks in both places and matches them by game, so the Windows save and the Linux save become two versions of one history.

## What Steam Cloud already covers

For Steam games with Steam Cloud, Steam syncs the save between your Windows install and your Proton install on its own. Hoard's part there is the history: Steam keeps the current save only, so a broken one replaces the good one on both systems. For games without Steam Cloud, and for everything outside Steam, Hoard does the syncing too.

## Why not just share a folder on the Windows drive?

It's the first idea most people have: point Linux at the saves on the Windows partition and be done. It tends to break in three ways:

- **Fast Startup and hibernation.** When Windows shuts down with Fast Startup on, it leaves its partition half-hibernated, and Linux mounts it read-only or refuses. Your game can't write its save.
- **NTFS under Proton.** Running Proton prefixes or Steam libraries from an NTFS drive is a well-known source of permission and file-name problems. Linux games are happier on a Linux file system.
- **Links get replaced.** Symlinking one system's save folder into the other works until a game, an update or a reinstall replaces the link with a real folder, quietly.

Letting each system keep its saves where the game expects them, and syncing between them, avoids all three.

## Set it up

1. **On Windows**, install Hoard and sign in.
2. **On Linux**, install Hoard from [the download page](/download) and sign in with the same account.
3. **Launch each Proton game once on Linux**, so its prefix exists. Before that, there's no folder to put the save in.
4. Check the **Library** on both systems: the same games should show up on each, and Hoard matches them by game.

## The trap that only dual boot has

On two separate PCs, the save waits on the server until the other machine asks for it. On a dual-boot PC, the "other machine" is the same computer after a reboot, and that changes one habit.

Hoard uploads a save once the game has closed and the folder has gone quiet. **If you quit the game and reboot straight away, the upload may not have happened yet**, and the other system boots without your latest progress. It will catch up the next time you boot back, but by then you may have played on the old save.

So: quit the game, give Hoard a moment, check in the app that the save is up to date, and then reboot.

## Native Linux versions

Some games have a native Linux build as well as the Windows one. The two don't always use the same save format, and a few keep their saves in completely different places. If you want the same save on both systems, the safest route is to run the Windows version through Proton on Linux too: in Steam, **Properties → Compatibility**, force a Proton version. Then both systems run the same game and write the same files.

## Settings and devices

Graphics settings may differ between the two systems, so Hoard backs up settings files like `graphics.ini` but doesn't write them over the other system's copy. When you want them copied anyway, there's an option for it when you restore.

Each operating system counts as its own device, so a dual-boot PC uses two of the free plan's three. Pro and self-hosted servers have no device limit.

Prefer to keep saves at home? Run `hoard-server` on a NAS or another machine and point both systems at it. No account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

<!-- faq -->

## Frequently asked questions

### Does Steam Cloud sync between Windows and Linux?

Yes, for games that support it: Steam keeps one cloud copy per account, whichever system you play on. It keeps no history, and it doesn't cover games without Steam Cloud or anything outside Steam.

### Can I keep my saves on the shared NTFS partition?

It's not recommended. Fast Startup can leave the partition read-only in Linux, and Proton is known to struggle on NTFS. Keeping each system's saves in its own place and syncing them is more reliable.

### Why didn't my save show up after rebooting?

Most likely the upload hadn't finished when you rebooted. Boot back into the first system, let Hoard upload, and check the app before switching again.

### Does a dual-boot PC count as one device?

No, two: each operating system registers as its own device. On the free plan that's two of three; Pro and self-hosted servers have no limit.

### What if a game has a native Linux version?

Its saves may not match the Windows version's. To share one save, run the Windows version through Proton on Linux as well.
