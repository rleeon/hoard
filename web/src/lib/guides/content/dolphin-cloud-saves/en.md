---
title: "Dolphin cloud saves: sync GameCube and Wii saves between PC and Steam Deck"
description: "Dolphin has no cloud saves. Sync GameCube and Wii saves automatically between PCs and Steam Deck, with version history: paths, card types and traps."
order: 17
updated: 2026-10-09
related: back-up-emulator-saves, pcsx2-cloud-saves, retroarch-save-sync, sync-saves-steam-deck-pc
---

Dolphin doesn't sync saves between machines: your GameCube memory cards and your emulated Wii live in a folder on one PC. Hoard syncs them automatically. When you close Dolphin it backs up your GameCube and Wii saves, brings them down on your other PCs and your Steam Deck, and keeps every version so you can always go back.

## Where Dolphin keeps your saves

Everything lives in Dolphin's user folder. The quickest way to find it is **File → Open User Folder** inside Dolphin. In it:

- `GC` holds the GameCube memory cards.
- `Wii` is the emulated Wii's internal memory, saves included.
- `StateSaves` holds save states.

Where that folder is:

- **Windows:** `Documents\Dolphin Emulator`. Newer installs may use `%APPDATA%\Dolphin Emulator` instead, and a portable install keeps a `User` folder next to `Dolphin.exe`.
- **Linux:** `~/.local/share/dolphin-emu`.
- **Steam Deck** (the Flatpak from Discover): `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu`.
- **Mac:** `~/Library/Application Support/Dolphin`.

Hoard finds the `Documents`, Linux and Steam Deck folders on its own. For `%APPDATA%`, a portable install or a Mac, point it at the `GC` and `Wii` folders once.

## GameCube: card files or GCI folders

Under **Options → Configuration → GameCube**, each memory card slot can be one of two things:

- **A memory card file**, a raw image such as `MemoryCardA.USA.raw` with every game's saves inside. Any save rewrites the whole file.
- **A GCI folder**, where each save is its own `.gci` file, in a folder like `GC/USA/Card A`. Only the save that changed is new, so versions stay small and easy to read.

For syncing, GCI folders are the better fit. Either way, **use the same setting on every machine**: a card file on one PC and a GCI folder on the other means each sees an empty card. If you need to move saves from one kind to the other, Dolphin's **Tools → Memory Card Manager** imports and exports `.gci` files.

Cards are also kept **per region** (USA, EUR, JAP). A PAL and an NTSC copy of the same game don't see each other's saves, so use the same disc image everywhere.

## Wii: the emulated console's memory

Wii saves live inside the `Wii` folder, under `Wii/title/00010000/<game ID>/data` for disc games. That folder is the whole emulated console memory: saves, Miis, system settings and any channels you've installed. Hoard backs it up as one item, so restoring a version puts the console memory back as it was at that moment. Before you confirm, Hoard shows what will change, and your current files are captured first.

If you only want to move one Wii save by hand, Dolphin can export it: right-click the game in the list and choose **Export Wii Save**.

## How syncing works day to day

You play on the desktop and close Dolphin. Hoard waits until Dolphin has exited and the folders have gone quiet, then uploads the new version. Later you pick up the Steam Deck; once it's online, Hoard brings the newer saves down. Close it on the Deck and the same thing happens the other way. Neither machine has to be on at the same time as the other.

## Traps worth knowing

- **Close Dolphin, not just the game.** Hoard backs up once the emulator has exited. On a Deck, suspending doesn't count as closing.
- **Save states are fragile.** Dolphin's states often break between Dolphin versions. Hoard syncs the real saves; if you want `StateSaves` too, add it as its own item and keep Dolphin on the same version everywhere.
- **Custom paths.** If you changed the Wii NAND root or the GCI folder path in **Options → Configuration → Paths**, point Hoard at those folders instead.

## Set it up

1. Install Hoard on each machine and sign in with the same account.
2. In the **Library**, add Dolphin from the emulator list.
3. Use the same memory card setting and region on every machine.
4. Play, close Dolphin, and carry on on the other machine.

Rather keep it at home? Run `hoard-server` on your own PC or NAS and point every machine at it. No account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard). For the other emulators, see [emulator saves](/guides/back-up-emulator-saves).

<!-- faq -->

## Frequently asked questions

### Does Dolphin have cloud saves?

No. Dolphin keeps saves in a local folder and leaves syncing to you. Hoard is one way to sync them automatically, with a version history on top.

### Can I sync Dolphin saves between a PC and a Steam Deck?

Yes. Install Hoard on both with the same account. Hoard knows where Dolphin keeps its saves on Windows, Linux and in the Steam Deck's Flatpak, and matches them across machines.

### Should I use a memory card file or a GCI folder?

For syncing, a GCI folder: each save is its own file, so versions are small and show which game changed. Whichever you choose, use the same on every machine.

### Does it sync Wii saves too?

Yes. The `Wii` folder holds the emulated console's memory, saves included, and Hoard backs it up and syncs it like the GameCube cards.

### Does Hoard sync Dolphin save states?

Not by default, because states break between Dolphin versions. Add the `StateSaves` folder by hand if you want them.

### Does it work with Dolphin on Android?

Not today. Hoard runs on Windows, macOS, Linux and Steam Deck.
