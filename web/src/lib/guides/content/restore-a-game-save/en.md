---
title: "How to restore an old game save"
description: "Made a wrong move, corrupted a file or want a fresh start? Roll back to any previous version of your game save with Hoard's cloud history — including saves backed up by tools like Ludusavi."
order: 3
updated: 2026-09-01
---

A bad decision in-game, a corrupted file, or a botched mod — sometimes you just need to go back. Because Hoard keeps a full version history of every save, restoring an earlier one takes seconds.

## Restore a previous version

1. Open **Hoard** and go to the game in your **Library**.
2. Open its **History** tab. You'll see every backup with its date and size.
3. Pick the version you want and choose **Restore**.
4. Hoard writes that snapshot back into the game's save folder. Your current save is backed up first, so the restore itself is reversible.

## Restore on a new or reinstalled PC

1. Install Hoard and sign in with your account.
2. Add the game to your Library — Hoard finds the matching cloud backup.
3. Restore the latest version, or any older one, and keep playing.

Because Hoard locates save folders using the same community database as Ludusavi, it knows where to put a restored save even on a fresh install — no manual path hunting.

## When a save is corrupted or a mod broke it

A game that crashes on load, a mod that rewrote something it shouldn't, an autosave that landed halfway through a write: the fix is the same. Open the game's **History**, pick the last version from before the problem started, and restore it. Dates and sizes are usually enough to spot the moment things went wrong — a sudden drop in size is a good sign that a save got truncated.

If you're not sure which version is the good one, restore the most likely candidate and check in-game. Trying again costs nothing, because the version you just replaced was kept too.

## What a restore actually does

Three things worth knowing, because they are what make a restore safe to try:

1. **Your current save is captured first.** The restore is reversible: whatever you replaced becomes a version in the history like any other.
2. **Only what's missing is downloaded.** Files already on disk with the right content are used as they are, so restoring a large save after a small change moves a few megabytes instead of the whole folder.
3. **Files that belong to this machine are left alone.** Configuration and logs sitting next to the save are backed up, but not written over your local copies — your key bindings and graphics settings survive a restore that came from another PC.

## Restoring without our servers

If you run your own `hoard-server`, restores work exactly the same way, except the versions come from your machine instead of ours. There is no account with us, no telemetry to us and nothing passing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

## Tip

Restores are never destructive: the save you replace is captured as a new version first, so you can always undo a restore by restoring the previous entry. If you've only ever kept local backups (for example with Ludusavi), moving to Hoard adds an off-machine, versioned history you can restore from even after a disk failure.

<!-- faq -->

## Frequently asked questions

### Will restoring overwrite my current progress?

Only after your current save has been captured as a new version. If you restore the wrong one, restore the previous entry and you're back where you started.

### How far back does the history go?

As far as the version limit on your plan allows, and a version you pin is never pruned to make room. On a self-hosted server the only limit is your disk.

### Can I restore to a PC where the game isn't installed yet?

Install the game first so its save folder exists, then restore. Hoard knows where each game expects its saves, so it writes the snapshot to the right place without you hunting for the path.

### Does restoring work between Windows and a Steam Deck?

Yes. The same game keeps its save in different places on each — on the Deck, inside the Proton prefix — and Hoard writes the restored version wherever that machine expects it.

### Is a restore any different on a self-hosted server?

No. Same app, same history, same one-click restore. Only the storage is yours.
