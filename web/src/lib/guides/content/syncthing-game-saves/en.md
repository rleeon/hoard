---
title: "Syncthing for game saves: what works and what breaks"
description: "Syncthing is an excellent general-purpose file syncer, but game saves break three of its assumptions. What goes wrong, how people work around it, and when a save-aware tool is the better answer."
order: 9
updated: 2026-09-01
---

Syncthing is the answer a lot of people reach for first, and for good reason: it's free, open source, peer-to-peer, and it works. But game saves break three of the assumptions a general-purpose file syncer is built on, and the failures are quiet ones. This guide is about what actually goes wrong, and when it's worth using something that knows what a save is.

## Why people reach for it

It's genuinely good software. No account, no subscription, your files never sit on a company's disk, and it syncs anything: documents, photos, a folder of saves. If you already run it for other things, pointing it at a save folder costs you thirty seconds. That's a real argument, and for some setups it's the right one.

## The three things that break

**It syncs while the game is running.** Syncthing reacts to a file changing, because that's the correct behaviour for a document. A game writes its save in the middle of a session, sometimes in several passes, and a file caught mid-write is a file that propagates half-finished. The other machine now holds a save the game may refuse to load.

**Conflicts become files, not decisions.** When both machines change the same save, Syncthing does the safe thing and keeps both, renaming one to `something.sync-conflict-20260901-143022-ABCDEFG.sav`. Nothing is lost — but the game doesn't know what that file is, and you're left comparing timestamps in a file manager to work out which afternoon of play to keep. Do this a few times and the folder fills with conflict files nobody dares delete.

**Versioning is per file, not per session.** Syncthing can keep old copies in `.stversions`, and that's better than nothing. But a save is often several files that only make sense together, and restoring means finding the right timestamp for each one by hand. There's no "put this game back the way it was on Tuesday".

And a fourth, specific to Steam: point it at `userdata/<UserID>/<AppID>/` instead of the `remote/` folder inside, and you're also syncing `remotecache.vdf` plus achievement and playtime files that are *supposed* to differ between machines. Every launch then looks like a conflict even though no save actually moved. This is the single most common reason a hand-rolled Steam Deck and desktop setup feels broken.

## What you end up building

None of the above is unfixable. People handle it with ignore patterns per game, a versioning policy, and the habit of closing the game and waiting before touching the other PC. That works, and it's a maintenance job you own forever: a new game means new paths, and the day you forget to wait is the day you find out.

## What a save-aware tool does instead

Hoard captures **after you stop playing**, once the folder goes quiet, so a snapshot is never a half-written file. Each capture is a version of the whole save, not of individual files, so restoring is one click and puts everything back together. It knows which folder belongs to which game — reading the same community save-location manifest the open-source ecosystem shares, covering 20,000+ titles — so there are no paths to maintain, and it tracks `<AppID>/remote/` rather than the folder above it.

## When Syncthing is the better answer

Being fair about it:

- **You already run it**, and adding a folder is free.
- **You want peer-to-peer with no server at all**, not even your own.
- **You're syncing much more than saves** and would rather have one tool for everything.
- **You never roll back.** If the latest save is all you've ever needed, a version history is machinery you won't use.

## Using both

They coexist without a fight, and it's a reasonable setup: let the general syncer handle your documents and whatever else, and let a save-aware tool handle the save folders. The only rule is not to point both at the same folder — two tools writing the same files is how you manufacture the conflicts you were trying to avoid.

## Without our servers either

If part of the appeal is that nothing touches a company's disk, Hoard can be run the same way: `hoard-server` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us and no relay** — nothing passes through our servers, because there is nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same binary, same detection, same history. The only thing that changes is who owns the storage. There's also a full [comparison of every save sync tool](/guides/game-save-sync-comparison).

<!-- faq -->

## Frequently asked questions

### Can Syncthing sync game saves at all?

Yes, and for simple cases it does it fine. The trouble starts with games that write while you play, saves made of several files, and any setup where both machines get edited between syncs.

### What are the .sync-conflict files in my save folder?

That's the syncer keeping both versions after a conflict instead of choosing one. Nothing is lost, but the game can't read them, and deciding which to keep is manual work every time.

### Why does my Steam save conflict on every launch?

Almost always because the synced folder is the one above `remote/`. It contains `remotecache.vdf` and achievement and playtime files that legitimately differ per machine, so the two ends never agree.

### Do I need to close the game before syncing?

With a general-purpose syncer, yes — that's the habit that prevents half-written saves. A save-aware tool waits for the folder to go quiet on its own.

### Can I keep using both together?

Yes. Just don't point both at the same folder, or the two of them will fight over the same files.
