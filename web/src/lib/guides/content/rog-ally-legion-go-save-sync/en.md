---
title: "Sync saves between ROG Ally, Legion Go, MSI Claw and your PC"
description: "Windows handhelds like the ROG Ally, Legion Go and MSI Claw are just PCs. Sync their game saves with your desktop automatically, with version history."
order: 19
updated: 2026-10-09
related: sync-game-saves-across-pcs, sync-saves-steam-deck-pc, epic-gog-cloud-saves, steam-cloud-alternative
---

The ROG Ally, the Legion Go and the MSI Claw run Windows, so to a game they're just another PC. That's exactly the problem: your desktop and your handheld each keep their own saves. Steam Cloud covers part of your library and Xbox cloud saves cover Game Pass, but everything else stays on the machine you played it on. Hoard keeps saves in sync between your handheld and your desktop automatically: stop playing on one and your game is waiting on the other, with every earlier version kept.

## What already follows you

- **Steam games with Steam Cloud** sync on their own.
- **Game Pass and Xbox app games** use Xbox cloud saves, as long as you play the Xbox version on both machines.
- **Epic, GOG, Ubisoft and EA** have cloud saves for some of their games, inside their own launchers. See [Epic and GOG cloud saves](/guides/epic-gog-cloud-saves).

What's left over: games whose developer never turned cloud saves on, emulators, games you installed by hand, and any game where your desktop and your handheld don't use the same launcher.

## Set it up

1. **On the handheld**, switch to the Windows desktop, open [the download page](/download) and install Hoard for Windows.
2. **Sign in** with the account you use on your desktop, or point the app at your own server.
3. Open the **Library** and check what Hoard found. Add anything missing by pointing at its folder, such as an emulator.
4. **On the desktop**, install Hoard with the same account. The same games match up on their own.

The sync engine runs as a background service that starts with Windows, so it keeps working while you're in Armoury Crate, Legion Space, MSI Center M or Steam's Big Picture mode. You don't need to open Hoard's window to play.

## Handheld traps

### Sleep is not quitting

Handhelds make it easy to press the power button and put the game away still running. Hoard only backs up once the game has closed, because a running game may be halfway through writing its save, and it never swaps the save of a running game. If you sleep the handheld and then play on the desktop, the handheld's progress hasn't been uploaded yet. **Quit the game before you switch.**

### Games on the microSD card

Installing games on the card is normal on a handheld, and it rarely matters for saves: most games save in your user folder on the internal drive wherever they're installed. Games that save next to their own install folder are the exception; if one of those isn't detected, add its folder by hand.

### Screen and settings

Your handheld runs at a lower resolution, on a smaller GPU, than your desktop. Hoard backs up settings files like `graphics.ini` along with the save but doesn't write them over the other machine's copy, so each keeps settings that suit it. If you want them copied anyway, there's an option for that when you restore.

### Same game, different store

A game bought on Steam for the desktop and played through Game Pass on the handheld is two different installs, and the Xbox version keeps its saves in a format only the Xbox app understands. To share one save, play the same store's version on both.

### SteamOS or Bazzite instead of Windows?

Then your handheld is a Linux machine and saves live inside Proton prefixes, exactly as on a Steam Deck. See [syncing saves between Steam Deck and PC](/guides/sync-saves-steam-deck-pc).

## Without our servers

If you'd rather keep saves at home, run `hoard-server` on your PC or a NAS and point both machines at it. No account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

<!-- faq -->

## Frequently asked questions

### Does the ROG Ally have cloud saves?

It has whatever each launcher has: Steam Cloud, Xbox cloud saves, and the clouds of Epic, GOG, Ubisoft or EA for the games that support them. There's no system-wide save sync. Hoard adds one for the games those leave out.

### Does Hoard work in Armoury Crate or Legion Space?

Yes. Hoard's sync engine is a Windows background service, independent of whichever launcher you use to start games.

### Does the handheld count as a device?

Yes. The free plan covers three devices, so a desktop, a laptop and a handheld fit. Pro and self-hosted servers have no device limit.

### What about Game Pass saves?

Leave those to Xbox cloud saves, which sync them between Xbox app installs on both machines. Hoard covers the games that don't have a cloud of their own.

### Can I sync my handheld with a Steam Deck too?

Yes. Hoard runs on both, and matches each game across Windows and the Deck's Proton prefixes.
