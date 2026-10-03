---
title: "Palworld save location (PC & Steam Deck)"
description: "Where Palworld keeps its worlds on PC and Steam Deck, what each file is, how co-op worlds work, and how to back up your saves or move them between PCs."
order: 24
updated: 2026-10-02
related: sync-game-saves-across-pcs, restore-a-game-save, steam-cloud-alternative
---

On PC (Steam), Palworld keeps its saves in `%LOCALAPPDATA%\Pal\Saved\SaveGames\<your Steam ID>`, with one folder per world inside. That's the path Pocketpair gives in its official FAQ. Below is the Steam Deck path, what each file does, how co-op changes things, and how to keep your worlds backed up.

## Where Palworld keeps its saves

- **Windows, Steam version:** `%LOCALAPPDATA%\Pal\Saved\SaveGames\<your Steam ID>\<world ID>`
- **Steam Deck and Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`

The Steam ID folder is a long number tied to your Steam account. Inside it, each world you've created has its own folder with a long hexadecimal name. On a Steam Deck the game runs through Proton, so the saves sit inside the Proton prefix Steam keeps for it; `1623730` is the game's Steam app ID.

The Xbox app / Game Pass version keeps its saves in a different, packaged location, and they aren't the same files you'd copy between Steam installs.

## What's in a world folder

- **`Level.sav`** is the world itself: your base, the map, the Pals placed in it.
- **`LevelMeta.sav`** holds the world's name and summary for the menu.
- **`Players\`** holds one `.sav` per player who has been in that world.
- **`LocalData.sav`** and **`WorldOption.sav`** hold local data and the world's settings.
- **`backup\`** is the game's own automatic backups of the world.

Settings are elsewhere: `Pal\Saved\Config\Windows\GameUserSettings.ini` holds graphics and controls, and `Pal\Saved\Logs` holds logs. Neither is part of your progress.

When you back up, take the **whole world folder**, not just `Level.sav`. The world and the player files belong together, and restoring one without the other is how characters end up out of step with the world they're in.

## Co-op and dedicated servers

In co-op, **the world lives on the host's PC**. Your character in that world is a file in the host's `Players` folder, not on your machine. If the host loses their save, everyone's progress in that world goes with it. On a dedicated server, the world lives on the server.

So for a shared world, it's the host's folder that needs the backup.

## Does Palworld have cloud saves?

The Steam version uses Steam Cloud, which keeps the latest state of your worlds in step between machines on the same account. It doesn't keep older versions, and the game's own `backup\` folder lives on the same disk as the save, so a dead drive takes both.

## Back it up by hand

1. Close the game completely.
2. Copy your Steam ID folder from `SaveGames` (it contains all your worlds) somewhere safe.
3. To restore, close the game and copy it back to the same place.

## Keep it backed up and in sync with Hoard

[Hoard](/download) backs up your worlds every time you stop playing and keeps every version off the machine, so a corrupted world or a lost drive isn't the end of a base you spent weeks on. It also syncs the worlds between your PCs and a Steam Deck.

1. Install Hoard and sign in, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. Palworld is detected from your Steam library and the community save database.
3. Play. When you quit, the first version appears in the history.

If you host co-op, this is the machine that matters: back up the host, and the shared world is covered. To roll a world back, [restore an earlier version](/guides/restore-a-game-save).

<!-- faq -->

## Frequently asked questions

### Where are Palworld saves on Steam Deck?

Inside the Proton prefix: `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`, in the folder named after your Steam ID.

### Where is my character in a friend's world saved?

On the host's PC, in that world's `Players` folder. Your own PC doesn't keep a copy of worlds hosted by someone else.

### Which file is my world?

`Level.sav`, but back up the whole world folder: the player files and the world are meant to stay together.

### Does Palworld back up my world on its own?

It keeps automatic backups in the world's `backup` folder. They're on the same disk as the save, so they protect against a bad save, not against losing the drive.
