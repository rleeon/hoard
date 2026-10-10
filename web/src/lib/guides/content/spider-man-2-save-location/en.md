---
title: "Marvel's Spider-Man 2 save location (PC & Steam Deck)"
description: "Where Marvel's Spider-Man 2 keeps its PC saves, what the long-number folder is, the OneDrive trap, the Steam Deck path, and how to back up and sync saves."
order: 22
updated: 2026-10-09
related: sync-game-saves-across-pcs, restore-a-game-save, steam-cloud-alternative
---

On PC, Marvel's Spider-Man 2 keeps its saves in `Documents\Marvel's Spider-Man 2\`, inside a subfolder named with a long number. On Steam, that number is your Steam ID. Below is what that means in practice, the OneDrive trap, the Steam Deck path, and how to keep the saves backed up and in sync between your PC and Steam Deck.

## Where Marvel's Spider-Man 2 keeps its saves

- **Windows:** `%USERPROFILE%\Documents\Marvel's Spider-Man 2\<long number>`
- **Steam Deck and Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<long number>`

The PC version is Windows only, so on a Steam Deck it runs through Proton, and the saves sit inside the Proton prefix Steam keeps for the game; `2651280` is its Steam app ID. If the game is installed on the microSD card, the `compatdata` folder is on the card.

Nixxes, the studio behind the PC port, describes the save folder as "a subfolder with a long number or a combination of letters and numbers" under `Documents\Marvel's Spider-Man 2\`.

## The long-number folder

The subfolder is named after your account: on Steam it's your **64-bit Steam ID**; the Epic version uses a mix of letters and numbers instead. Either way it's different for every account. Two consequences:

- If two people play on the same PC with different Steam accounts, each has their own save folder.
- If you copy saves to another PC by hand, put them in the folder for **that** machine's Steam account. Dropped into a folder with a different ID, the game won't see them.

The parent `Marvel's Spider-Man 2` folder also holds the game's log and crash dumps (`.log`, `.mdmp`). Those aren't saves and don't need backing up.

## The OneDrive trap

Many Windows PCs redirect `Documents` into OneDrive. If yours does, the real path is `%USERPROFILE%\OneDrive\Documents\Marvel's Spider-Man 2\`, and OneDrive syncs the folder on its own while you play. That causes two problems: OneDrive may upload a save halfway through being written, and "Free up space" can turn the save into an online-only placeholder. If you rely on OneDrive here, mark the folder **Always keep on this device**.

## Does Spider-Man 2 have cloud saves?

Yes, Steam Cloud, which keeps the latest saves in step between machines on the same Steam account. It doesn't keep older versions: if a save breaks, the broken one is what syncs.

## Back it up by hand

1. Close the game completely.
2. Copy the `Marvel's Spider-Man 2` folder from `Documents` somewhere safe.
3. To restore, close the game and copy the long-number folder back into the same place, under the same Steam account.

## Keep it backed up and in sync with Hoard

[Hoard](/download) backs up the save folder every time you stop playing and keeps every version. It also syncs it between your PCs and a Steam Deck, so the game picks up where you left off on either one.

1. Install Hoard and sign in, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library** and check the folder it shows for Spider-Man 2 is the one under `Documents` (or `OneDrive\Documents`). If it points somewhere else, change it to that folder.
3. Play. When you quit, the first version appears in the history.

If a save goes wrong later, [restoring an older version](/guides/restore-a-game-save) puts it back.

<!-- faq -->

## Frequently asked questions

### What is the long number in the save folder?

On Steam, your 64-bit Steam ID; on Epic, your account's ID. Every account gets its own folder, and the game only reads the one for the account that's signed in.

### Where are Spider-Man 2 saves on Steam Deck?

Inside the Proton prefix: `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/`, in the long-number folder.

### I can't find the folder in Documents. Where is it?

Check `OneDrive\Documents\Marvel's Spider-Man 2`. On most new Windows installs, Documents lives inside OneDrive.

### Can I copy my saves to a friend's PC?

The files will copy, but they go in the folder named after the Steam ID of the account on that PC. Whether the game accepts saves made on a different account is up to the game, so keep a copy of the original before you try.
