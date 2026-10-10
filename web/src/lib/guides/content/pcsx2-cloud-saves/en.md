---
title: "PCSX2 cloud saves: sync PS2 memory cards between PC and Steam Deck"
description: "PCSX2 has no cloud saves. Sync your PS2 memory cards automatically between PCs and Steam Deck, with version history: paths, folder cards and traps."
order: 16
updated: 2026-10-09
related: back-up-emulator-saves, dolphin-cloud-saves, retroarch-save-sync, sync-saves-steam-deck-pc
---

PCSX2 doesn't sync saves on its own: your PS2 progress lives in memory card files on one machine, and the other machine never hears about it. Hoard syncs them automatically. When you close PCSX2 it backs up your memory cards, brings them down on your other PCs and your Steam Deck, and keeps every version so a bad save never costs you a run.

## Where PCSX2 keeps your saves

PCSX2 saves the way a real PS2 does: on memory cards. By default there are two, `Mcd001.ps2` and `Mcd002.ps2`, 8 MB each, in a `memcards` folder. One card holds the saves of every game you've played on it.

- **Windows:** `Documents\PCSX2\memcards`. In portable mode, the folder sits next to the program instead.
- **Linux:** `~/.config/PCSX2/memcards`.
- **Steam Deck** (the Flatpak from Discover): `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`. With EmuDeck, the link under `~/Emulation/saves/pcsx2` points to one of these.
- **Mac:** `~/Library/Application Support/PCSX2/memcards`.

**Settings → Memory Cards** in PCSX2 shows the folder it's really using and which card sits in each slot. Hoard finds the Windows, Linux and Steam Deck folders on its own; on a Mac or a portable install, point it at the `memcards` folder once.

## File cards and folder cards

PCSX2 can make two kinds of memory card, and the choice matters more than it looks when you sync.

- **A file card** (`.ps2`) is one 8 MB file with every game's saves inside. Save in any game and the whole file changes, so each new version is the full 8 MB.
- **A folder card** is a folder instead of a file, with each save in its own subfolder. Save in one game and only that game's files change, so versions stay small and the history shows which game's save moved.

You can create either kind from **Settings → Memory Cards**. Whichever you pick, use the **same type, the same card names and the same slots** on every machine. A file card on the desktop and a folder card on the Deck are two different cards, and each machine will think the other's save doesn't exist.

## How syncing works day to day

You play on the desktop and close PCSX2. Hoard waits until PCSX2 has exited and the cards have stopped changing, then uploads the new version. Later you pick up the Steam Deck. Once it's online, Hoard brings the newer cards down, and when you start PCSX2 your save is there. Close it on the Deck and the same thing happens the other way.

Neither machine has to be on at the same time. The cards wait on the server until the other machine asks for them.

## Traps worth knowing

- **Close PCSX2, not just the game.** Hoard backs up once the emulator has exited, so it never copies a card in the middle of a write. On a Deck, suspending doesn't count as closing.
- **Same disc, same region.** The PAL and NTSC versions of a game have different serials (SLES and SLUS, for example), so they don't see each other's saves. Use the same disc image everywhere.
- **Save states are a different thing.** States (`.p2s` files in `sstates`) are snapshots of the emulator and often won't load in another PCSX2 version. Hoard syncs the memory cards; if you want states to travel too, add the `sstates` folder as its own item and keep PCSX2 on the same version on every machine.
- **A version is the whole card.** Restoring an earlier version puts the whole card back as it was, every game on it. Hoard shows what will change before you confirm, and your current card is captured first, so a restore can always be undone.

## Set it up

1. Install Hoard on each machine and sign in with the same account.
2. In the **Library**, add PCSX2 from the emulator list.
3. Check that every machine uses the same card type, names and slots.
4. Play, close PCSX2, and carry on on the other machine.

Rather keep it at home? Run `hoard-server` on your own PC or NAS and point every machine at it. No account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard). For the other emulators, see [emulator saves](/guides/back-up-emulator-saves).

<!-- faq -->

## Frequently asked questions

### Does PCSX2 have cloud saves?

No. PCSX2 writes memory cards to a local folder and leaves syncing to you. Hoard is one way to do it automatically, with a version history on top.

### Can I sync PCSX2 saves between a PC and a Steam Deck?

Yes. Install Hoard on both with the same account. Hoard knows where PCSX2 keeps its cards on Windows and in the Steam Deck's Flatpak, and matches them across machines.

### Should I use a file card or a folder card?

For syncing, a folder card is the better fit: only the saves that changed get uploaded, and the history shows which game moved. Either works, as long as every machine uses the same one.

### Does Hoard sync PCSX2 save states?

Not by default, because states break between PCSX2 versions. Add the `sstates` folder by hand if you want them, and keep PCSX2 on the same version everywhere.

### Will restoring an old version roll back every game on the card?

Yes. A version is the whole card. Hoard shows what will change first, and keeps the card you're replacing as a version too.

### Does it work with PS2 emulators on Android?

Not today. Hoard runs on Windows, macOS, Linux and Steam Deck.
