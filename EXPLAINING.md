# How Hoard works

The [README](README.md) says what Hoard does. This file says how, and what that
means for your machine and your data. It was checked against the code, and each
section ends with where to read that code yourself.

Where this file says **never**, that is a promise, not a description of today's
code.


## Finding your games

Most of detection is not searching. Hoard starts from a catalogue of more than
20,000 games, the [Ludusavi manifest](https://github.com/mtkennerly/ludusavi-manifest),
which says where each game keeps its saves, and checks whether those places
exist on your machine. It also reads your launchers' own library files, your
Wine and Proton prefixes and, on Windows, the registry values the catalogue
names.

The two signals people ask about most, running processes and the filesystem
scan, cover what that misses.


## Running processes

Hoard has to know when a game starts and stops: it backs up when you stop
playing, and it holds back a restore while the game is open.

**What it reads for each process:** the name, the path of the executable, CPU
usage, start time, and whether it is a process, a thread or already dead.
Nothing else.

**How often:** every 2 seconds while a game is running, every 8 seconds otherwise.

**On Linux and SteamOS**, for processes that look like games, Hoard also checks
which paths they have open, through `/proc/<pid>/fd` and `/proc/<pid>/cwd`, and
compares them with your tracked save folders. It looks at where those links
point. It does not open the files.

**On Windows**, while a game is running, Hoard tries to open up to 64 of the
save's files read-only. If Windows says another process holds one, the game is
still writing, so it is not the moment to copy it. This touches your save files,
not the game.

**When a watched folder changes**, Hoard notes which game-like process was alive
at the time. That is how it learns that a folder belongs to a game when the name
gives nothing away. These notes stay on your machine.

**Hoard never:**

- reads a process's memory
- reads a process's command line or environment variables
- injects code, attaches a debugger or loads anything into a game
- pauses, stops or kills a game (the only processes it stops are its own, to
  update or uninstall itself)
- needs root or administrator rights to detect or sync on Windows, Linux or macOS
  (the service runs as your user; installing or updating can ask for your
  password, a `.deb` or `.rpm` needs it)

### Anti-cheat

Hoard is safe to use with anti-cheat. It never touches a game's process, for the
reasons above.

The in-game HUD (`Alt+H`) is an ordinary transparent window on top of the game,
not something inside it. It is on by default, and you can turn it off in Settings,
under In-game overlay. The shortcut is registered with the system: Hoard does not
read your keyboard.

On Linux the desktop app reads game controllers from `/dev/input`, so you can
move around its own interface with them. It reads controllers, never keyboards.

*Read the code:* `crates/hoard-agent/src/agent.rs` (`proc_refresh_kind`,
`process_poll`, `open_paths_matching`), `crates/hoard-agent/src/locks.rs`,
`crates/hoard-agent/src/correlation.rs`, `crates/hoard-desktop/src/gamepad.rs`,
`crates/hoard-desktop/src/commands/overlay.rs`.


## Filesystem scan

The scan looks for save folders the catalogue could not place. It does not walk
your whole disk.

| When | What happens | Does it track anything? |
|------|--------------|-------------------------|
| Fresh install | Nothing, until you ask for the first scan | No |
| You scan | A normal scan | Only what you then add |
| Once a day, after your first scan | The Library's list is refreshed in the background | No |
| You ask for a deep scan | A wider, slower scan | Only what you then add |
| Automatic mode is on | See [Automatic mode](#automatic-mode) | Yes |

**Where it looks:**

| System | Folders |
|--------|---------|
| Windows | `AppData` (Roaming, Local, LocalLow), `Saved Games`, `Documents`, `Documents/My Games` |
| Linux | `~/.local/share`, `~/.config`, `~/.local/state`, `~/Documents`, `~/Saved Games` |
| macOS | `Application Support`, `Preferences`, `Documents` |

It also looks inside game install folders and Wine and Proton prefixes. A deep
scan on Linux adds Flatpak and Snap app data, EmuDeck and RetroDECK folders on any
drive, and Wine prefixes in unusual places.

**Limits:** 4 levels deep (6 in a deep scan), and between 1.5 and 5 seconds per
folder. It skips folders like `node_modules`, `.git`, caches, audio, video and
fonts.

**What it reads:** folder names, file names, sizes and dates. It opens launcher
and emulator config files, and it reads the file list of a `.zip` without
extracting it. It does not read your saves. Once you track a folder, Hoard reads
every file in it, because it has to hash and upload them.

**What keeps the wrong folder out:** Hoard refuses to treat a whole system or
profile folder as one game's save, whether detection found it or you added it by
hand. That covers the filesystem root, system folders, your home, `~/.config`,
`~/.local`, `Documents`, `Downloads`, `Program Files` and a whole Wine prefix.

There is no list of sensitive folders by name. `~/.ssh` and `~/.gnupg` sit outside
the folders above. A browser profile under `~/.config` or `AppData` sits inside
them, and what keeps it out is detection's score, not a rule.

*Read the code:* `crates/hoard-agent/src/roots.rs`,
`crates/hoard-agent/src/detection.rs`, `crates/hoard-agent/src/junkdirs.rs`,
`crates/hoardd/src/detect.rs`.


## Automatic mode

Automatic mode is off by default. It is the one case where Hoard acts without
asking.

When you turn it on, Hoard:

- scans every 10 minutes, and also when a new process that looks like a game
  starts using real CPU
- tracks and uploads whatever it detects with high confidence, **without asking**
- watches folders with lower confidence on your machine, so a later scan can
  promote them
- checks every tracked save once an hour, to catch changes the file watcher missed
- turns on automatic restore as well, which writes to your save folders

*Read the code:* `crates/hoardd/src/detect.rs`, `crates/hoard-agent/src/prefs.rs`,
`plan_auto_track` in `crates/hoard-agent/src/library.rs`.


## What leaves your machine

"Your server" means Hoard One or your own, whichever you use.

| What | Goes to | Off switch |
|------|---------|------------|
| Files in the folders you track | Your server | Stop tracking the folder |
| Per version: game name, size, hash, date, device name | Your server | None, the history needs it |
| Presence: the game you have open and for how long, every 30 seconds | Your server | None |
| Playtime: seconds per game, per day, per device, including Steam games you do not track | Your server | Settings, "Share my playtime with Wrapple" (on by default, turning it off disables the recap) |
| Warnings and errors from the app, and detection corrections | Your server | Settings, "Send diagnostics" (on by default) |
| App usage: days you open it, manual restores, Pro screens | Hoard One only, never your own server | Settings, "Send diagnostics" |
| Hoard-help reports | Hoard One, even if you self-host, only when you press send | Do not send one |

**Diagnostics are tied to your account.** They carry your device name (your
machine's hostname) and an identifier hashed from it, plus `/etc/machine-id` on
Linux. A self-hosted server receives more detail, down to debug level, because
it is yours.

**Paths in diagnostics and reports** have your username replaced with `<user>`
before they leave. Only that part: the rest of the path stays, folder names
included.

**Some downloads have nothing to do with your server.** The game catalogue comes
from GitHub, the update check goes to GitHub, the cover index comes from
`hoard.services`, and cover art comes from Steam's public APIs, so those requests
carry the game's name or ID.

**If you self-host**, nothing above reaches me unless you send a Hoard-help report.

Hoard One's retention times and sub-processors are in the
[privacy policy](https://hoard.services/legal/privacy).

*Read the code:* `crates/hoard-agent/src/logship.rs`,
`crates/hoard-agent/src/telemetry.rs`, `crates/hoard-agent/src/presence.rs`,
`crates/hoard-agent/src/feedback.rs`.


## Where your saves live

Saves are **not end-to-end encrypted**. Whoever runs the server can technically
read them. On your own server that is you. On Hoard One that is me.

- **In transit:** HTTPS to Hoard One. On your own server it depends on the address
  you set: `http://` means no encryption.
- **At rest:** Hoard One's storage provider encrypts files at rest. On your own
  server that is up to your disk or bucket.
- **Integrity:** every file is SHA-256 hashed on upload and verified on restore.


## Credentials and the local service

Your session is stored in the system keychain. If there is no keychain, or Hoard
has to start before you log in, it uses a file only your user can read.

`hoardd`, the background service, talks to the app and the terminal over a local
socket: `0600` inside a `0700` folder on Linux and macOS, and a named pipe limited
to your user and SYSTEM on Windows. Nothing else on the network can reach it.
Anything already running as your user can, the same way it can read your files.

*Read the code:* `crates/hoard-agent/src/credentials.rs`,
`crates/hoardd/src/transport.rs`, `crates/hoardd/src/winsec.rs`.


## Found a problem?

Do not open a public issue for a security bug. [SECURITY.md](SECURITY.md) says how
to report it.
