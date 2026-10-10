---
title: "How to transfer game saves to a new PC"
description: "Moving to a new PC or reinstalling Windows? Bring every game save with you: what Steam Cloud covers, the manual way, the automatic way and the traps."
order: 13
updated: 2026-10-09
related: where-are-pc-game-saves-stored, sync-game-saves-across-pcs, back-up-game-saves, sync-saves-steam-deck-pc
---

Steam Cloud games come back on their own when you sign in on the new PC. Everything else is a folder you have to bring across yourself: games without cloud saves, emulators, anything outside Steam. You can copy those folders by hand, or let Hoard back them up on the old PC and put each one back in the right place on the new one. Both ways are below, along with the traps that cost people their saves.

## Before you wipe anything

- **Make a list of what you play**, including games you haven't touched in months. Those are the ones people forget.
- **Check which games have cloud saves.** On Steam, the store page says so, and **Properties → General** shows whether it's on. Epic and GOG show it per game too.
- **Back up the rest, and ideally everything.** Cloud saves keep one copy, the latest. If that copy is broken, it's broken everywhere.

## The manual way

1. **Find each game's folder.** Most are in `Documents\My Games`, `Saved Games`, or the `AppData` folders (`Roaming`, `Local`, `LocalLow`). The full list is in [where PC games keep their saves](/guides/where-are-pc-game-saves-stored).
2. **Copy them to an external drive**, keeping the folder structure. Also take Steam's whole `userdata` folder: it's small, and it covers games that store saves through Steam without having Steam Cloud switched on.
3. **On the new PC, install the game first.** If the game needs to create its folders, launch it once and quit at the main menu. Don't start a new game.
4. **Copy the saves into place** and launch. Check that your progress is there before deleting anything on the old drive.

It works. The downside is that it's a one-off copy: you have to remember every folder, and if you keep playing on the old PC, the two drift apart from that day on.

## The traps

- **Saves tied to an account.** Some games put your account ID in the folder name or inside the save: Elden Ring files saves under your SteamID, and Ubisoft games under your Ubisoft ID. Same account on both PCs: fine. A different account: the game sees an empty slot.
- **OneDrive moved Documents.** If one PC backs up `Documents` with OneDrive and the other doesn't, the "same" folder is in two different places. Right-click `Documents`, open **Properties → Location** to see where it really is. More in [OneDrive and game saves](/guides/onedrive-game-saves).
- **Game versions.** A save from a newer version of a game may not load in an older one. Update the game on the new PC before you copy.
- **Mods.** A modded save (Bethesda games especially) may refuse to load without the same mods. Reinstall them first.
- **Windows to Steam Deck or Linux.** The save goes inside the game's Proton prefix, which only exists after the game has been launched once. See [syncing saves between Steam Deck and PC](/guides/sync-saves-steam-deck-pc).

## The automatic way

Hoard turns the move into the same thing it does every day: back up on one machine, restore on another.

1. **On the old PC**, install Hoard and sign in. Open the **Library**: Hoard lists the saves it found for your games, using the same community save database as Ludusavi. Add anything missing by pointing at its folder.
2. **Check that each game has a version** in its **History**. That's your safety net before you wipe the old drive.
3. **On the new PC**, install Hoard and sign in with the same account, then install your games. Hoard matches them to their backups by game and restores the latest version into the folder this machine expects, even when the path is different (another drive, another user name, a Proton prefix on a Deck).
4. **Before starting a new game**, let Hoard finish putting your saves back. The app shows each game's status.

Two details make this safer than a copy. Settings files such as `graphics.ini` are backed up but not written over the new PC's own, so your new hardware starts with settings that suit it (you can bring them along when you restore, if both machines are alike). And nothing is final: every version stays in the history, so a wrong restore is undone by restoring the one before.

If the old PC stays in use, it simply keeps syncing with the new one. If it's gone for good, remove it from your devices. The free plan covers three.

## Without our servers

You can do all of this against your own server: run `hoard-server` on a PC or NAS, point both machines at it, and the saves never leave your home. No account with us, no telemetry to us. See [how to self-host Hoard](/guides/self-host-hoard).

<!-- faq -->

## Frequently asked questions

### Do Steam saves transfer automatically?

Only for games with Steam Cloud. Sign in on the new PC, install the game, and the save downloads. Games without it need their folder copied, or a tool that does it for you.

### Can I just copy my whole user folder?

It works for most saves, but it also drags along gigabytes of caches, settings meant for the old hardware and app data that can cause problems on a new install. Copying the save folders themselves is cleaner.

### Will my saves work if my Windows user name is different?

Yes, almost always. Saves are stored relative to your user folder, so the name in the path doesn't matter. Hoard handles this on its own.

### Can I move saves from Windows to a Steam Deck?

Yes. Launch the game once on the Deck so its Proton prefix exists, then put the save inside it, or let Hoard do it. See [the Steam Deck guide](/guides/sync-saves-steam-deck-pc).

### Do I need to keep the old PC until the new one is ready?

With a manual copy, keep the external drive until you've checked every game. With Hoard, the saves are already on the server, so the old PC can go as soon as each game shows a version in its history.
