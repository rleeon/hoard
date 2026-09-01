---
title: "OpenSave alternative: peer-to-peer or a server you own"
description: "OpenSave syncs game saves directly between your PCs, with no server in the middle. Hoard syncs through a server — ours or one you host — and keeps a versioned history. An honest look at when each design wins."
order: 8
updated: 2026-09-01
---

Both tools solve the same problem and disagree about the architecture, which is the only thing worth comparing. This page lays the two designs side by side, including the cases where the other one is the better answer.

## The actual difference: peer-to-peer or a server

**OpenSave** is peer-to-peer. Your machines talk to each other directly, and nothing sits in between. There's no account and no storage to pay for, and it can optionally mirror a copy to a cloud drive you already have.

**Hoard** syncs through a server. That server is either Hoard Cloud, managed by us, or `hoard-server` running on your own PC or NAS. Your save goes up when you stop playing and comes down when another machine asks for it.

Everything else follows from that one choice.

## What a server buys you

- **The other machine doesn't have to be on.** You finish on the desktop, the laptop stays shut for a week, and the latest save is waiting when you open it. Peer-to-peer needs both ends awake at the same time, which is fine at a desk and awkward with a handheld you pick up twice a month.
- **A version history, not just the latest state.** Every session becomes a version you can roll back to. This is the part that matters the day a mod eats your world or a save is written half-corrupt: direct sync faithfully copies the broken file to your other PC.
- **A copy that survives the hardware.** Both your PCs dying in the same flat is not an exotic scenario. A save that only ever existed on those two machines dies with them.
- **Nothing to arrange on the network.** No NAT to traverse, no port to open, no both-devices-on-the-same-LAN caveat.

## What peer-to-peer buys you

Being fair about the other side:

- **No storage to pay for, ever.** There's no quota to hit, because there's no bucket. Hoard's free tier is 2 GB, and above that you either pay or self-host.
- **Nothing in the middle by design.** If the goal is that a file never touches a third party's disk, direct transfer is the shortest possible answer.
- **Nothing to run.** No server to keep up, not even your own.

If you play on two desktops that are both switched on, you never want to roll back, and you'd rather not think about storage at all, that design is a clean fit and Hoard is more machinery than you need.

## The privacy question, answered precisely

This is where comparisons of Hoard usually go wrong, so it's worth being exact. There are two ways to run Hoard, and they are genuinely different:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **Self-hosting is entirely yours.** You run `hoard-server` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, because none of it ever reaches us. If Hoard Cloud shut down tomorrow, a self-hosted setup would carry on unchanged.

So "server" doesn't mean "someone else's computer" unless you choose that. A self-hosted Hoard keeps your saves on hardware you own, exactly like a direct transfer does, and still gives you the history and the offline-machine case.

## Detection and coverage

Both tools find saves for a large catalogue automatically. Hoard reads the same community save-location manifest that the open-source ecosystem shares, covering 20,000+ titles, and adds Steam library scanning, running processes and a filesystem sweep on top. For Steam games it tracks `<AppID>/remote/` inside `userdata` rather than the folder above, because the parent holds `remotecache.vdf` and per-machine achievement and playtime files — sync those and every launch looks like a conflict. Anything unusual you can point it at by hand.

## Which one should you use?

- **Peer-to-peer** if your machines are on at the same time, you don't want storage in the picture at all, and the latest save is all you've ever needed.
- **Hoard** if you want a version history you can roll back, a machine that can be off for a week, and a copy that outlives both PCs — with the choice of our cloud or your own server.

There's a wider [comparison of every save sync tool](/guides/game-save-sync-comparison) if you want the whole field, and a [Ludusavi comparison](/guides/ludusavi-alternative) for the local-backup end of it.

<!-- faq -->

## Frequently asked questions

### Does Hoard need an account?

For Hoard Cloud, yes — that's what the sync is tied to. Self-hosted, there's no account with us at all; your server has its own users and a token per device, and they never leave your machine.

### Can Hoard work without any cloud?

Yes. Run `hoard-server` on a PC or a NAS and your saves go from your machine to your disk, with nothing passing through our servers.

### Do both PCs need to be online at the same time?

No, and that's the practical advantage of syncing through a server. Your save is uploaded when you stop playing and downloaded whenever the other machine next asks for it.

### Does a direct transfer keep a version history?

Not inherently — copying a file to another machine gives you the current state on both. Hoard captures every session as a version, which is what makes rolling back a corrupted save possible.

### Is Hoard open source too?

Yes, AGPL-3.0, server included. The self-hosted server is the same binary we run, not a cut-down edition.
