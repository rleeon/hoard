---
title: "GTA 6 save location: PS5, Xbox and PC (what we know)"
description: "GTA 6 launches on PS5 and Xbox on November 19, 2026, with no PC version yet. Where console saves live, how to back them up, and what to expect on PC."
order: 25
updated: 2026-10-17
related: where-are-pc-game-saves-stored, back-up-game-saves, sync-game-saves-across-pcs
---

GTA 6 comes out on **November 19, 2026, on PS5 and Xbox Series X|S only**. There's no PC version yet: Rockstar hasn't announced a date, and the only window going around is an unconfirmed October 2026 report that points to early 2027. Until then, GTA 6 saves live on the console and in the console's cloud, not in a folder you can open. Below is how those saves are kept, how to protect them, and where the PC saves will most likely go, judging by Rockstar's last PC releases. This page gets the real path as soon as the PC version is out.

## Where GTA 6 keeps its saves on console

- **PS5:** on the console. With PlayStation Plus, the PS5 also uploads saves to its cloud storage automatically. Without it, the only copy is the one on the console.
- **Xbox Series X|S:** on the console, synced to the Xbox cloud under your Microsoft account. That's automatic and needs no subscription.

Neither is a folder you can browse from a computer. The PS5 can back up the whole console, saves included, to a USB drive, but that copy can only be restored on a PS5.

## Can I back up GTA 6 saves with Hoard?

Not on console. Hoard works with saves that are files on a computer: Windows, macOS, Linux and the Steam Deck. Console saves never leave the console's own system, so no PC app can reach them.

Once the PC version is out, GTA 6 saves will be files like those of any other PC game, and Hoard can back them up and keep them in sync between your PCs.

## Where the PC saves will probably be

Rockstar's recent PC releases all save to the same place, a folder per game under `Documents\Rockstar Games`:

- **GTA V Legacy:** `%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<profile ID>`
- **GTA V Enhanced:** `%USERPROFILE%\Documents\Rockstar Games\GTAV Enhanced\Profiles\<profile ID>`
- **Red Dead Redemption 2:** `%USERPROFILE%\Documents\Rockstar Games\Red Dead Redemption 2\Profiles\<profile ID>`

The last folder is named after your Rockstar account, so every account gets its own. If GTA 6 follows the same pattern, its saves will end up in `Documents\Rockstar Games\<game name>\Profiles\<profile ID>`. **That's an educated guess from precedent, not a confirmed path.**

If your PC redirects `Documents` into OneDrive, the whole `Rockstar Games` folder moves with it, to `%USERPROFILE%\OneDrive\Documents\Rockstar Games`. OneDrive then syncs the saves on its own while you play, which brings its own problems: see [game saves and OneDrive](/guides/onedrive-game-saves).

## Rockstar's own cloud saves on PC

On PC, GTA V and Red Dead Redemption 2 sync saves through the Rockstar Games Launcher, not Steam Cloud, even if you bought them on Steam. Cloud saves are turned on per game in the launcher's settings. Like Steam Cloud, it keeps the current saves in step between machines; it doesn't give you older versions to go back to.

Rockstar hasn't said how GTA 6 will handle saves on PC. Using the same launcher cloud would be the obvious move, but until there's an announcement it's an assumption.

## Get ready for the PC version with Hoard

[Hoard](/download) backs up a save folder each time you stop playing and keeps every version, so a corrupted save or a mission you'd rather replay is one restore away. It also keeps the folder in sync between your PCs and a Steam Deck, and it already works with GTA V and Red Dead Redemption 2.

When GTA 6 reaches PC:

1. Install Hoard and sign in, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. A brand-new game can take a while to reach the community save database, so if GTA 6 isn't listed yet, add its folder under `Documents\Rockstar Games` by hand.
3. Play. When you quit, the first version appears in the history.

<!-- faq -->

## Frequently asked questions

### Is GTA 6 on PC?

Not at launch. On November 19, 2026, it comes out on PS5 and Xbox Series X|S. Rockstar hasn't announced a PC date; an October 2026 report points to early 2027, without confirmation.

### Where are GTA 6 saves on PS5?

On the console, and in PlayStation Plus cloud storage if you're a member and automatic uploads are on. They aren't files you can open on a computer.

### Can I move my GTA 6 save from console to PC?

Rockstar hasn't said. For GTA V it let players carry Story Mode saves over to the new consoles through its own cloud, but nothing similar has been announced for GTA 6.

### Does the save folder include online progress?

Not in Rockstar's current games: GTA Online and Red Dead Online progress is stored on Rockstar's servers, not in the save folder. There's nothing on disk to back up for it.

### Will Hoard support GTA 6?

On PC, yes, as long as it saves to a folder the way Rockstar's other PC games do. Console saves are out of reach for Hoard and for any other PC app.
