# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **Accounts are managed from the web panel.** Creating a user, renaming one,
  setting a password, deleting an account and issuing a device token were all
  `hoard-admin` subcommands, so on a NAS the second person to use the server
  meant opening the container's console. The Users tab does all five now. The
  irreversible one asks you to type the name, and shows how much it is about to
  delete before you do: an account's saves go with it.
- **A device token can be issued from the panel.** The container prints one for
  the first PC on first boot; every PC after that needed a shell. The token is
  shown once — only its SHA-256 is stored — with a copy button that falls back
  to selecting the text, because `navigator.clipboard` does not exist on the
  plain-HTTP origin a NAS panel is reached over.

### Fixed
- **Self-hosted clients never auto-pulled a save that was ahead on the server.**
  After ADR 0021 the engine only restores when `cloud_ahead` is true (or the
  folder is empty). That flag is fed from a head cache the cloud poller fills
  via `/v1/cloud/sync`. Self-hosted skipped that observation entirely, and the
  SSE `force-restore` nudge ignored the event's `version_num`, so a non-empty
  folder stayed stuck on the local version. The engine now lists `/v1/saves`
  on the same cadence as cloud, and a `save` SSE frame merges that version
  into the cache before reconcile.
- **Deleting a user left every byte of their saves on disk.** `hoard-admin user
  delete` removed `data_dir/<user_id>`, a path nothing has written to since the
  content-addressed store landed, and reported success. The blobs and chunks are
  now deleted through the storage backend, driven off the index and before the
  row cascades away — which also means it works on an S3-compatible bucket,
  where there was no directory to remove and the objects stayed for good.
- **`hoard-admin user delete` would delete the only admin.** The panel refuses
  to demote the last one, because the admin flag guards its own route and a
  server with zero admins needs a shell to come back. The command that runs on
  that shell had no such check, and would happily leave the server without one.

## [1.1.5] - 2026-08-24

### Added
- **The server installs itself on a NAS.** An Unraid template ships in
  `templates/`, so Hoard can be installed from the Apps tab: the ports and
  folders come filled in, and the only two boxes to type into are an admin
  username and a password. The container does the rest of what used to be a
  shell session — it writes a working `config.toml` if the config folder is
  empty, creates that first admin when the database is new, and prints a device
  token in the log, once, for the desktop app to paste. `HOARD_ADMIN_USERNAME`
  and `HOARD_ADMIN_PASSWORD` do the same for anyone running the container by
  hand; they are ignored from the second start on.
- **An emulator is tracked one game at a time.** An emulator's save root is a
  shelf with one folder per title on it, not a save, and tracking it whole broke
  twice over: the name came off the emulator's own plumbing — rpcs3's
  `dev_hdd0/home/<profile>/savedata` became a game called `dev-hdd0` — and the
  backup could never run, because a shelf has no save files of its own. One
  rpcs3 root logged "nothing to back up" 224 times and was still logging it this
  month; RetroArch, Ryujinx, Dolphin and Yuzu have the same shape. Each title
  inside is now offered as its own row, named the way the "add emulator" dialog
  names them, and the root itself is offered only when it holds saves directly —
  RetroArch's flat `.srm` files in `saves/` are the save. The roots are
  recognised by the tail of their path rather than the whole of it, so an
  emulator installed somewhere unexpected, or a second profile whose id isn't
  the first one, is still identified. Pointing at a title folder and pointing at
  the shelf above it now give the same answer.
- **Appearance gets a gem, a backdrop and a size.** The accent used to be a bare
  0–359 hue slider, which is not how anyone thinks about colour; it is now seven
  named gems, each swatch drawn with the exact maths the app will apply, with
  the wheel one click away behind Custom for anyone who wants it (and opening by
  itself if your saved hue matches no gem). The pure-black canvas was a
  deliberate call for WOLED panels — no glow, so black stays genuinely off — and
  a good default is a bad decree: there are now four backdrops, with today's
  look still selected for everyone who never opens the setting. And the whole
  interface scales, from the slider or with Ctrl + wheel / Ctrl +/- / Ctrl+0.
  All three are per-machine, like the theme.
- **A game's history page shows its cover.** The one page dedicated to a single
  game was the only place that never showed which game it was, drawing a tile
  with the first letter of its name while the Library and the dashboard both
  showed the art. The initial stays as the fallback for a game with no cover.

### Changed
- **The mode is called Self-Host, everywhere.** Onboarding said "Self-hosted",
  the guides said "Autohost" and the Unraid page said something else again —
  three names for the one card you have to click. One name now, in the app and
  in all eight translations of the guides.
- **Self-hosting no longer reads as "clone the repo and build it".** Every
  release tag publishes a multi-arch server image, so the short way goes first
  in the guide: one `curl` for the compose file, one command to start it, and
  the container writes its own config. Cloning stays as the alternative. How to
  update is written down too, which is the half that silently doesn't happen —
  `pull` without `up -d` leaves the old container running and the health
  endpoint still reporting the old version, and `git pull` updates nothing at
  all, since what runs is the published image and not the checkout.

### Fixed
- **One save folder could be backed up twice, every time it changed.** The same
  directory can end up tracked under two save ids — the one this machine minted
  and the one the server considers canonical for that game — and nothing ever
  collapsed them: both got a watcher, both hashed the folder, both uploaded the
  same bytes. Nothing looked wrong on the server, because the upload path
  already redirects both to the canonical id; the whole cost landed on the
  machine doing the work and the connection carrying it. A folder is now watched
  once no matter how many rows name it. Deliberately narrow, so it cannot take
  anything real with it: the same game tracked in two different folders is a
  slot and keeps both, and two different games sharing one folder keep both too.
- **A game you had frozen went on being backed up, hourly, saying nothing.**
  Parking a game in the archive frees its space on the server and the server
  then refuses to accept new versions of it — which is the point. The client
  never learned that, so every reconcile decided to back the game up, hashed the
  whole save folder, sent it and got turned away: 30 times in two days in the
  report that found this, with "Backing up…" appearing in the activity feed each
  time and nothing ever following it, because that particular refusal was the
  one outcome that told the app nothing. Frozen games are now left alone, the
  same way a paused one is — no hashing, no attempt, no line in the feed.
  Freezing or restoring a game takes effect immediately instead of at the next
  restart. If the server can't be asked which games are frozen, everything stays
  watched exactly as before.
- **Accent colours left a green square in the middle of the ramp.** The chosen
  gem repointed most of the emerald scale but not all of it, so anything using
  one of the shades it missed stayed green while everything around it turned
  purple, amber or blue. Most visible in the playtime heatmap, whose lowest
  level is one of those shades: four squares followed the gem and the fifth did
  not. The whole scale follows now, which also fixes the selected chips in the
  recap and the overlay, the toasts, and the badge counters.
- **A game with many save files could never finish uploading to a self-hosted
  server.** A save folder travels as one request per file, so a game keeping
  dozens of slots — 46 in the report that found this — sends more requests
  back-to-back than the server's per-IP limit allows. That limit answers "you
  are going too fast", which is a request to slow down, but the client read it
  as "this upload does not fit" and threw away the whole attempt, including the
  files that had already arrived. The next attempt started from zero and hit the
  same wall, forever: one user's game retried 105 times over two days without a
  single version ever being saved. Uploads now slow down instead of failing —
  each file waits its turn and the upload finishes, taking seconds rather than
  never. Size was never the problem: the same server had accepted a 3.8 GB game
  the day before, because a few big files arrive slowly enough to stay under the
  limit.
- **An account that was out of space kept trying forever, and stopped saying
  so.** After a handful of refusals the server asks that account to stand down
  for an hour, which is the polite version of the same "you are out of space"
  message. The client answered that pause with a five-minute one of its own,
  came back twelve times before it was welcome, and — because the pause arrives
  in a different shape from the first refusals — stopped showing the "free up
  space or go Pro" prompt at exactly the point the problem started repeating. So
  the wall went up and the sign explaining it came down. One account spent four
  days bouncing off it at around 170 refusals an hour, day and night, without
  ever completing a backup. The wait the server asks for is now the wait that is
  taken, the prompt stays on screen with the plan and the numbers behind it, and
  twenty games hitting one wall still read as one message. The same cap comes
  off restores, which can ask for anything from fifteen minutes to a day.
- **One unreadable file lost the entire backup.** A file the system refuses to
  hand over — most often a cloud-storage placeholder that hasn't been downloaded
  — aborted the whole snapshot instead of being skipped, so a single stray file
  inside a GTA San Andreas Definitive save meant 3,934 attempts across 13 days
  and not one version uploaded. The rest of the save is worth more than nothing
  at all, so the file is skipped and the backup goes through — out loud: the
  game's card carries an amber warning naming how many files were left out, and
  keeps it until a complete backup lands. A save where nothing at all is
  readable still uploads nothing, because an empty version would bury the last
  good copy.
- **A save could give itself up and go on retrying for weeks, with nothing on
  screen.** When the server says "you're behind" but there is nothing newer to
  pull — the version it was pointing at was deleted, or two machines raced — the
  answer to the question never changes, and asking again on a flat ten-minute
  timer is just noise. Production had 1,701 of those, one save stuck at roughly
  four and a half attempts an hour for 14 days, across three app versions, and
  never a word about it. The wait now backs off (10 → 20 → 40 → 80 minutes), and
  after five of them the save stops and says so: a red state on its card with
  what to try next. Anything that could actually change the answer clears it —
  the cloud moving on, a successful backup, or pressing "back up now". Nothing
  is lost while it waits; the pending changes stay pending, so a restore can't
  overwrite them.
- **Settings and tracked games could vanish after a hard shutdown.** Both files
  were written by emptying them first and filling them afterwards, so a process
  that died in that window — a closing laptop lid, an update replacing the
  binary — left a zero-byte file that reads exactly like a file that was never
  valid. One user's telemetry carries 917 "settings were corrupt, resetting to
  defaults": 917 times losing every preference they had set. On the state files
  the same accident costs the manual paths and exclusions, or the entire list of
  tracked saves — the app opens to an empty library and every game has to be
  adopted again. Those files are now replaced whole or not at all: the new
  contents are written beside the old ones, flushed to the disk, and only then
  swapped in.
- **What was being backed up was the game's own backup copy.** Some games mirror
  their save into a sibling folder every few minutes (`SaveGamesBackup`,
  `SavesOld`, `…-bak`), and detection was happy to pick the mirror — so the only
  thing syncing was the game's private archive, and because every copy counts as
  new content, none of it deduplicated and the quota drained. A folder whose
  name ends in a backup suffix is now graded down, but only when it also looks
  like a rotating mirror, so a real save folder with an unlucky name doesn't get
  punished for it — and the suffix has to be at the end: `BackupSaves` is a save
  folder with an odd name. Games already tracked that way get a warning naming
  the sibling that looks like the real save and what the mirror is costing in
  the cloud, with an offer to repoint or archive; moving a save stays your call.
  The reason a folder was picked now reaches the screen, so "why THIS folder?"
  has an answer.
- **63 games came back with no save folder at all.** The catalog writes some
  save paths relative to "wherever the store put it", and the only store Hoard
  knew how to resolve was Steam — so every Ubisoft launcher title looked inside
  Steam's folder, found nothing, and reported no saves: the Assassin's Creed,
  Far Cry and Watch Dogs lines among them, leaving the folder to be found by
  hand. Non-Steam storefronts are resolved now, natively and inside a Proton
  prefix, and adding another one is a single line.
- **Saves named `user`, `steam`, `cd` or a bare number.** When a save's own
  folder is a plumbing directory, the name was taken from whatever sat above it,
  and the climb could land on a path segment that names nothing: production
  carries saves called `user` on 13 accounts, `steam` on 11, plus loose
  `settings`, `local`, `logs`, `game` and raw Steam ids. The single biggest
  source was a Windows machine whose account is literally named `user`. Those
  names are now refused at the moment of naming rather than quarantined later,
  and the check reads your own home path as well as a fixed list, so it also
  catches the segment that only means something on your machine.
- **A phantom game called "storage" on a Linux handheld, always running and
  impossible to close.** A front-end keeps its emulator trees under a folder by
  that name, so a save was minted from it — and on an image-based distro, every
  containerised process also runs out of a path containing `storage`, so that
  name then matched half the process table and the game never stopped playing.
  Names like that can no longer be minted, are quarantined if already on disk,
  and never count as evidence that a game is running.
- **Blacklisting a game left it being backed up.** The button filtered the game
  out of future scans and did nothing about the save already tracked under that
  name, which went on being watched, synced and counted as playing — so the
  user who blacklisted a phantom game saw nothing change and had no way to tell
  why. It now stops tracking it too, in one step, and the confirmation says how
  many tracked saves that will be before the click rather than afterwards.
  Nothing is deleted from the server: the versions stay and the game can be
  tracked again from the Library.
- **A game tracked for months kept being offered as a new find, every ten
  minutes.** Two unrelated causes. Certain repacks leave a bookkeeping file in
  the folder that would hold cloud saves, and a folder with a file in it isn't
  empty, so it passed the only test there was — the folder now has to hold
  something that isn't known bookkeeping. And the warning that fires when a busy
  program looks like a game asked "is this executable listed on a tracked save?"
  of saves that were tracked by folder and so list no programs at all; it
  corroborates by the game's identity now, before deciding anyone needs
  bothering.
- **Start-at-login asked for an administrator, or was refused outright.** 142
  reports across 14 users where the sync service never got registered to start
  with the machine. On Windows, registering the scheduled task can be refused
  without an elevated console, and all Hoard had to offer was "re-run this from
  an administrator PowerShell" — which is asking someone to open an admin shell
  so their game can save itself. The task is still tried first, since it is the
  better mechanism, but when it is refused a normal per-user startup entry is
  written instead, which never needs elevation. On an AppImage there was no way
  in at all, because the program runs from a mount that is gone by the next
  login; it now points at an installed copy of the engine when there is one, and
  otherwise stages its own. And when there is genuinely no way in, the switch
  says so under itself instead of reading "on" while the sync only ever ran with
  the window open.
- **The sync service could die on every single launch at login.** Priming its
  clock did arithmetic that doesn't exist on a computer that has been on for
  less than a minute, so the loop crashed, was restarted, and crashed again on
  the same sum — which is exactly the machine that starts Hoard at login.
  Telemetry caught four of those with the loop having run for zero seconds.
- **A Linux keyring that misbehaved could take the session with it.** Seven
  users signed out of Cloud with nothing to go on: no keyring daemon at all, a
  locked one, a damaged entry, and Hoard's own five-second timeout all produced
  the same blank "the sync service is offline". Each of those now says which one
  it was and what to do about it. More seriously, saving the session trusted the
  keyring's word: a keyring that accepts a write and then can't decrypt what it
  holds left the machine with its only copy in a store that would never give it
  back, and nothing on disk to fall back to. Every write is now read back before
  the on-disk copy is dropped, which is what makes "sign in again" real advice
  instead of another lap of the same loop.
- **The dashboard undercounted what a game was costing.** It showed the size of
  the newest version and called it the total, while the storage bar beside it
  showed real usage — so a game with history disagreed with itself on screen,
  35 MB of saves reporting 79 MB of quota, with nothing explaining the gap. Both
  numbers now come from the game's whole deduplicated footprint, and sorting by
  size ranks on that, since "biggest" has to mean "what is eating the quota".
  The newest version's size is still worth knowing — it is what a fresh restore
  would pull — so it stays as a subtitle when history makes the two differ.
- **"You've hit the bandwidth limit" when nothing had hit any limit.** The
  client treated every kind of "too many requests" as the account running out
  of bandwidth, so a server merely asking it to slow down produced a message
  that sent people looking at their plan for a problem that was not there, and
  a wait of a fabricated 60 seconds — the real wait was a fraction of a second.
  The two are now told apart and each says what it actually is.
- **Storage that couldn't be reached read as though Hoard was down.** On Cloud
  the files go straight to storage and never through Hoard's server, so when
  that address stops answering the failure has nothing to do with Hoard — but
  the message was the raw network error wrapped around a 400-character signed
  URL, with Hoard's own name nowhere in it. It now names the host that wouldn't
  answer and says the server itself replied fine. It also stops retrying: a
  connection that cannot be opened will not open on the next attempt either, and
  six timeouts a round, four minutes at a time, only delayed the moment anyone
  found out. Found on a machine whose internet provider had stopped routing to
  the storage endpoint while every other address at the same provider answered
  in 20 ms.
- **An unplugged Steam library filled the log with the same line.** One user
  produced 553 entries in 48 hours, every one of them the same "no such device"
  about a different game on the same absent external drive. The drive is asked
  once now, before anything inside it is opened, and the first device-level
  failure abandons that library instead of asking it about the next thirty
  games — one line per library per sweep, saying how many were skipped.
- **Uploads that started and never finished were charged for, forever.** An
  upload that gives up halfway is invisible and harmless to look at, and nothing
  ever cleaned up behind one: production was carrying 22 of them across 14
  accounts, 45,506 database rows describing versions that will never exist, plus
  whatever files each had managed to send — bytes on the meter that belonged to
  nobody and no cleanup could see. Both are swept now, and only for an account
  with nothing in flight, so a healthy upload in progress can't be mistaken for
  an abandoned one.
- **A container on a NAS could never write its own data.** Bind-mounted folders
  arrive owned by root and the server does not run as root, so the very first
  write failed and the container restarted forever. It now takes ownership of
  the two mounted folders at startup and drops root before the server runs —
  `PUID`/`PGID` say who to become (`10001:10001` as before, `99:100` on Unraid,
  where it matches the rest of appdata). Starting the container with an explicit
  `--user` skips all of that, exactly as it used to behave.
- **Mounting an empty config folder stopped the container instead of filling
  it.** The example config lived at the one path a config mount hides, so the
  container had nothing to copy from and exited with instructions. It now keeps
  its copy out of reach of the mount and bootstraps.

## [1.1.4] - 2026-08-20

### Added
- **A web panel for your own server.** Point a browser at your server and it
  answers: every game, save and version with its real size, what deduplication
  saved you, which machine each version came from, your machines and what they
  are playing, and the log of everything the server did. Any version can be
  downloaded or trashed from there. Admin accounts also get server-wide storage,
  users and quotas, and the diagnostic logs the clients upload. It ships inside
  the binary — nothing to deploy, no build step — translated into the same eight
  languages as the app, and it can be turned off with `[panel] enabled = false`.
  Five wrong passwords shut that account's door, twenty from one origin shut it
  for every account, and both counters key on the address the request actually
  came from: `X-Forwarded-For` is believed only from a peer listed in the new
  `server.trusted_proxies` (default `loopback`, which covers a reverse proxy on
  the same machine — name your proxy's address there if it reaches the server
  from a container or another box, and the server prints what it trusts at
  startup). Without that the counters would be decoration, since anyone can
  write that header and a fresh value means a fresh counter.
  It also shows the trash: a deleted version stays listed, struck through, with
  the way back one click away — the server has always kept those bytes for
  thirty days and the CLI could already undelete, but nothing said so where you
  were doing the deleting. And it updates itself while you watch: the push the
  server has published since 1.1.2 finally has a reader, so a version landing
  from another machine repaints the page.
- **The password you set when creating a user finally does something.** It has
  been stored, hashed, since the first release and read by nothing: the API
  authenticates with tokens. It is now what you type into the panel, so an
  account made two years ago can sign in today. Pasting a `hoard_v1_…` token
  works too, and it is traded for a session instead of being kept in the
  browser. New: `hoard-admin user passwd`, and `user promote` / `user demote`
  for the admin flag, which until now needed an UPDATE by hand in SQLite.
- **The account page finally shows your own server.** Running your own box
  meant a permanent "Sign in" in the sidebar — an invitation to join the
  service you had deliberately not joined — while your backups were reaching
  your own disk perfectly well, and the page behind it pitched Pro instead of
  showing the server you were actually signed in to. It now renders that
  session: the address it points at, its uptime, the disk in use, and the
  three limits the operator sets. Those limits had no way of reaching the
  client at all — the only way to learn `storage.max_snapshot_size_mb` existed
  was for a backup to bounce off it with a 413. They travel on `whoami` and
  not on `/v1/health`, because an operator's ceiling is nobody's business
  until they authenticate, and a server too old to report one shows a dash
  rather than a zero. Raising a limit and restarting shows up on the next
  poll. Cloud still wins when both sessions exist, and the self-hosted card
  stays in Settings for that case.
- **A game that keeps one folder per save can be tracked by the folder that
  holds them.** Cyberpunk 2077 writes `AutoSave-0/sav.dat`,
  `ManualSave-3/sav.dat` — no save files of its own, one subfolder per slot —
  and that shape slipped through all three detection paths at once. The
  catalog pointed straight at the game's folder and it was dropped whole,
  because none of the subfolders inside is spelled like a save directory;
  what was left were the loose folders rescued one by one, a separate "game"
  per slot, and only the ones the game had written to lately, so the manual
  saves could not be backed up at all. The nest is recognised now and kept
  whole, on all three paths. It is deliberately conservative, because the
  expensive mistake is the opposite one — swallowing an install directory, or a
  container of several games, as if it were one save. So: at least two
  subfolders holding data, every one of them named like a save slot, none of
  them spelled exactly like a save directory (a child called `saves` means the
  folder is a container of saves and the answer is to go into it), no files of
  its own alongside them, and never a profile root, a system folder or a whole
  Proton prefix — checked structurally, so the Windows rules that live under
  `drive_c` are seen on Linux too. Run over 129,383 real directories on a
  development machine before shipping, which is how the install-directory case
  turned up. And when slots from before are still tracked one by one, adding
  the parent names them instead of refusing flatly.

### Fixed
- **"Back up now" with nothing changed did nothing at all.** The button went
  through the same two gates the watcher does — cheap signature, then content
  hash — and both said the bytes match the last autosave, so the engine
  skipped it: no version, no error, nothing on screen, just an INFO line in a
  log nobody opens. Those gates exist to stop a watcher re-cutting identical
  snapshots on a timer, and that is not what a person pressing a button is
  doing: a deliberate copy is a marker placed on purpose — right here, before
  the boss — and whether it happens to be byte-identical to the last automatic
  one is beside the point. It costs no transfer, because storage is
  content-addressed and the blobs are already up there. The safety copy taken
  before a restore counts as deliberate too; skipping that one is worse, since
  it is what lets you undo a restore you didn't want. The notification answers
  a button press even with "notify on success" off, which was meant to silence
  the engine narrating every autosave, never to ignore you when you press a
  button.
- **A game that rewrites its autosave every few seconds no longer eats its own
  history.** Without a preset there was no minimum interval at all, so every
  rewrite became a cloud version: one save reached 2,233 versions in a day,
  1,027 uploads in four and a half hours, and a history meant to let you go
  back a week held about four. A fixed floor for everyone was tried before and
  had to be reverted — it was invisible, and it read as "Hoard isn't picking
  up my changes" — so this one only appears once the save itself proves it
  needs it: three commits inside ten minutes, one single step of 60 s, and the
  count reopens from zero as soon as the burst stops, so the floor lasts
  exactly as long as the burst does. An explicit interval always wins:
  `short_session`'s 30 s belongs to a game that wipes its folder between
  rounds, where losing one copy is losing the run, and `data_saver`'s 600 s is
  someone paying for bandwidth. And it says so while it waits: the deadline
  goes into the "next copy in" the overlay and the diagnostics already show,
  and the activity feed gets one "queued, waiting" row per wait rather than one
  per tick. Invisible waiting is exactly what earned the first floor its
  reversal.
- **The save folder is looked for by name before the install directory gets
  walked.** The order was backwards. A game whose catalogue path didn't
  resolve went straight to the aggressive walk of its installation, and that
  walk is good at finding *a* folder that looks like saves — so it confidently
  offered a directory inside the installation (3.6 GB of game data, in the
  case that exposed this) while the real save folder sat one `read_dir` away
  in LocalLow. The standard save roots are checked by name first now, and the
  walk is only the fallback. It also runs for games with neither an install
  directory nor a Wine prefix, which used to be dropped before anything was
  looked at — that is every game that doesn't come from Steam.
- **A refused upload now names who refused it, and quotes a number that
  exists.** Every client surface read a 413 as "upgrade to Pro", so a
  self-hoster who hit their own `storage.max_snapshot_size_mb` was told to buy
  a plan for a service they had chosen not to use, and the sentence quoted a
  limit of 0 B — the plan cap it was reading does not exist on their server.
  There are three possible refusers and three different places to go and fix
  it: the plan, the operator's `config.toml`, or a reverse proxy in front of
  the server that Hoard has no say over at all. The size was wrong too, in a
  way that read as precision: a rejection at `cas_init` happens before a
  single byte has moved, and it was announcing "3.6 GB sent before it
  stopped". Bytes received and bytes declared are separate fields now, and a
  413 that arrives before any transmission mentions no bytes at all.
- **A server URL written with credentials made login impossible.**
  `http://insider@ubserver:12421` is how people write an address they also
  reach over ssh. Nothing in this API uses HTTP Basic — the access key is a
  bearer token — but the HTTP client turns URL credentials into a `basic_auth`
  call on every request it builds, and headers append rather than replace, so
  each request went out with two `Authorization` headers: Basic first, then
  ours. The server read the first, found no bearer token, and answered 401;
  the client then blamed the one thing that was fine, "token rejected by
  server (401)". The credentials are stripped rather than rejected — an
  ssh-shaped address is a habit, not a mistake worth an error — and it happens
  at every door, so a URL already sitting in a config from before this gets
  cleaned on the way out too.
- **The two commands printed after you create a token now run.** `hoard config
  set server <url>` errors out — the key is `server.url` — and joining the two
  halves with `&&` is a parse error in Windows PowerShell, so the login half
  never ran either. Two lines, one command each, and the real key name.
  `hoard-admin token revoke` also says which prefix it wants now: the one in
  the `Prefix` column of `token list`, which is the start of the token's hash,
  not of the token itself, because the plaintext is never stored.

## [1.1.3] - 2026-08-16

### Added
- **Name your folders, not just number them.** A game with three tracked
  folders was a list of integers. Each one can carry a name next to its number
  now — "2 · Mods" — and the number keeps doing the pairing with the same folder
  on your other machines. Renaming and renumbering are separate on purpose, so
  naming a folder here can never repoint one over there. And every folder now
  restores like the first: backing one up and then refusing to put it back was
  half a backup, not a safety feature.
- **A game can have more than one folder now.** Factorio keeps saves in one
  folder and settings in another; a Paradox game splits saves and mods; an
  emulator separates memory cards from BIOS files. Hoard tracked one folder per
  game, so you had to choose — and pointing at the second one left the real save
  folder out of sight. Each game now holds a numbered list instead. Folder 1 is
  always the saved games: it's what a new machine restores on its own and what
  "this game is synced" means. From 2 up it's everything else, backed up just
  the same but never written over unless you press restore yourself, because one
  machine's config folder has no business landing on another's. The number is
  what pairs a folder with the same folder on your other computers, so you pick
  it once and both machines agree. Rows added before this keep their old labels
  and keep working.
- **Emulators are detected, per game.** An emulator has no store page, no
  install folder Hoard can look up and no catalogue entry, so its saves were
  yours to find by hand. There's now a curated list of the common emulators and
  where they save, including the copy you unzipped onto another drive that keeps
  its saves next to the executable. Where a console's save folder holds one
  subfolder per game, Hoard can split it and track the games separately instead
  of the whole tree — which also fixes the playtime: ten titles from one
  emulator share an executable, and starting it used to mark all ten as being
  played at once.
- **Copies you made on purpose have their own budget.** The cap on stored
  versions counted every version together, so a game that autosaves every minute
  filled it in a single session and pushed out the copy you took by hand before
  a boss fight. Manual copies — and the safety copy Hoard takes before a
  restore — now count against a separate cap that has no limit by default, so an
  autosave burst can only displace other autosaves.
- **Cover art for games that aren't on Steam.** Covers were looked up by Steam
  app id, which meant a tenth of the catalogue — Minecraft Java, every emulated
  title, anything from another launcher — had no cover and no way to be given
  one. Any game Hoard tracks can have one now, either from a small index we
  publish or one you pick yourself.
- **A word when your plan changes.** Paying for Pro and cancelling it both
  happen in the browser, and the app never acknowledged either. There's now a
  one-time thank-you after an upgrade, and after a cancellation a screen that
  says plainly what you keep — the devices you already paired stay paired — and
  what changes.
- **Legal notice, sub-processor list and a security policy.** Who runs the
  service, which providers touch your data and since when, and where to send a
  vulnerability report. The Terms and the Privacy Policy have been rewritten
  around what the service actually does, and both the app and the website now
  record which version of them you accepted.
- **Hoard updates itself now.** Until now an update was a button: the app
  checked GitHub, painted an amber badge, and waited. Anyone who didn't press
  it stayed on their version indefinitely — and because `hoard`, `hoardd` and
  the app are replaced together or not at all, "indefinitely" meant a bug fixed
  three releases ago was still live on machines that had been running for
  months. The sync service now owns the update, for the same reason it owns the
  sync engine: it's the only piece that's always there. It checks hourly,
  downloads and signature-verifies the new version in the background, and
  installs it when the machine is idle — no game running, no backup in flight.
  Most people will never see any of it; they'll just notice the version number
  changed.

  Where it can't be silent, it says so instead of pretending. Whether an update
  can install unattended isn't a preference — it's decided by how the app got
  onto the machine. An AppImage or a per-user Windows installer writes inside
  your home directory and nobody needs to be asked; a `.deb`, an `.rpm` or a
  `.dmg` needs a privilege prompt or a hand. Those get one native notification
  when the update is downloaded and ready, and install the moment you open Hoard
  and approve — the window is the only place where a permission dialog has
  somebody to ask.

  And there's a deadline. Two days after a release first appears, an update
  stops being optional: `hoard upgrade` and the app both push it through, and if
  it still needs approval the app asks for it on a screen you can't dismiss.
  Two things the deadline never overrides: a backup or restore in flight always
  finishes first, and nothing is installed behind a running game unless you ask
  for it yourself. `hoard` and `hoard sync` show what's happening — downloading,
  waiting for you to close the game, waiting for approval — instead of a badge
  that says "update available" about something already underway.
- **Hoard Screen now reports whether anyone actually uses it.** The overlay
  shipped with no way of telling a feature nobody wants from a feature nobody
  finds — the two need opposite fixes, and polishing the wrong one is the most
  expensive work there is. The desktop now times each overlay session (how long
  it was up, how much of that was spent in edit mode, how it ended) and records
  what gets built inside it: how many panels, and whether they're mirrored
  windows, crosshairs or scopes. It rides the existing opt-in telemetry channel
  and it is deliberately blind to content — no window titles, no application
  names, no captures, only the *kind* of panel and how many. A new Screen tab in
  the admin dashboard turns that into the one number worth having: of everyone
  who has had Pro, how many opened the overlay, and how many came back a second
  day.
- **Self-hosted backups only upload what changed.** Your server has always
  stored each file once and let versions share the bytes — but every backup still
  sent the whole folder and the server threw away the part it already had. A 3 GB
  save that changed 10 MB cost 3 GB of upload, every time. The client now tells
  the server what the version contains, the server answers which files it's
  missing, and only those travel. A second backup of the same game moves
  megabytes. It also stops a big save from arriving as one enormous request,
  which is what used to collide with `max_snapshot_size_mb` and with the body
  limit of any reverse proxy in front — nginx, a Synology's built-in one, a
  Cloudflare hostname. Nothing to configure: the server announces it and older
  clients keep working against the same server. (Hoard Cloud has worked this way
  since launch.)

- **Your own server now knows your machines.** The Eye panel used to show only
  the computer you were sitting at — the list of other devices was never wired
  up, on either deployment. Now it shows every machine on the account: which are
  on right now, what each is playing and for how long. Self-hosted included, and
  there it stays entirely between your machines and your server: the census
  lives in your own database and nothing about it is sent anywhere. Machines
  identify themselves by a stable fingerprint, so reinstalling doesn't duplicate
  them, and one that goes months without appearing is forgotten.

### Fixed
- **Windows: Hoard looked for your home directory in the wrong place.** If Git
  was installed, `HOME` was set by its shell to a path that only exists inside
  that shell, and Hoard preferred it over the account's real profile — so the
  folders it refuses to treat as a single save (your whole user folder,
  Documents, Saved Games) were being checked against a directory that isn't
  there. It asks Windows first now.
- **Hoard noticed your saves in two seconds and then sat on them.** A slider
  that stopped being shown in June kept setting the minimum wait between two
  uploads of the same save, at whatever value you had last left it — on one
  machine ten minutes, with nothing in the app able to show or change it. Worse,
  restores skipped that wait, so changes arriving from your other computer
  synced instantly while your own waited, which made the two look like unrelated
  problems. The wait is gone; the `data_saver` preset still paces a game you
  choose it for.
- **Updating on Windows could fail forever on the machine that needed it most.**
  The installer stopped the sync service to replace it, the app noticed the
  service was missing two seconds later and started the old copy again, and the
  installer then failed on a file back in use — then retried an hour later and
  lost the same race. Nothing launches the service while its binaries are being
  replaced now.
- **An upload could overwrite a newer version from another machine.** A computer
  that had never synced a save — or whose local state had been rebuilt — sent no
  starting point, and that was the one case the server let through without
  checking. The older folder became the current one. Nothing was lost from the
  history, but the save read as "it stopped syncing". That upload is now checked
  like every other.
- **A save renamed on one machine broke uploads from the other**, with a plain
  server error and no way to tell what was wrong. Saves are matched by identity
  first now, so a rename elsewhere, a reinstall or a rebuilt server no longer
  strands the copy you are uploading.
- **Settings files: one answer per game, not one per restore.** Whether a
  restore should write a game's `.ini` and `.cfg` files back is a question with
  no answer that's right twice — in one game the settings and the save live in
  the same file, in another they carry the resolution of the machine that
  uploaded them. The switch existed but only for one restore at a time, so a
  game that needs its settings written asked again every single time, and never
  got asked at all on automatic restores. You can now settle it for a game and
  Hoard remembers, automatic restores included.
- **The account-full warning stays on screen.** It was a row in the activity
  feed — a scrolling log you can hide — so the one fact that stops Hoard from
  backing anything up scrolled away, or never appeared if you'd closed the
  panel. It's now a card that stays for as long as the account is over its
  limit and clears itself when there's room.
- **A machine stuck in a loop no longer burns your bandwidth.** A client bug
  could have one machine download the same version over and over — one account
  pulled 10,6 GB of the same 2,83 MB save in a week — and nothing on the server
  side counted it. Stopping it meant noticing by hand and switching that save to
  backup-only, which hid the user's own cloud copy from their own machine. The
  server now recognises the repetition and asks the client to slow down, which
  every released version already knows how to obey.
- **Two games at once look like two games.** The Eye panel and `hoard devices`
  showed only the first game a machine was playing, and showed its slug rather
  than the name you gave it.
- **Save folders are no longer synced whole, junk and all.** A game's save
  folder rarely holds only saves: sitting next to your world files there are
  engine logs, crash telemetry, the analytics queue with the GUID that
  identifies *that* installation, shader information about *that* GPU, and
  settings files carrying *that* monitor's resolution. Hoard swept all of it
  into the snapshot and wrote all of it back on restore, which is how a save
  restored from one machine can crash the game on another. Now every file in
  the folder is sorted before it moves. Logs, crash dumps, temporary files, OS
  clutter and engine telemetry stay out of the backup entirely. Settings files
  are backed up as before — losing them is not an option — but a restore no
  longer writes them over the machine you're restoring onto unless you ask:
  there's a checkbox in the restore dialog, off by default, and `--allow-ini`
  on the CLI. Your save files themselves are unaffected either way. Two things
  fall out of it: a Unity game whose `Player.log` is rewritten at every launch
  used to cut a fresh cloud version on every single launch even when you hadn't
  played, and that stops; and the save catalog's own file patterns (`*.sav`,
  `*.plr` — 20,499 of them) are now read and used to protect anything the
  catalog says is real save data, so a game that genuinely saves into `.ini` or
  `.log` files is left alone.
- **Dropping from Pro to Free deleted your history without warning.** The
  grace window that was supposed to give you a month before a smaller plan takes
  effect never ran on the one downgrade that matters: the code worked out "how
  much room do you have today" using the plan you were moving *to*, so a Pro→Free
  drop looked like it changed nothing, the limit collapsed the same second, and
  the auto-purge started deleting old versions immediately. The window is now
  real — your old limit is frozen in place until the date, nothing is purged
  meanwhile, and the app counts down to it.
- **A full account failed one upload at a time, forever.** Hitting the storage
  limit surfaced as a raw server error per game (the JSON body, verbatim, in the
  activity panel) and every save kept retrying against a wall only you can move.
  It's now a state of its own: uploads park for an hour, the panel says what's
  happening in one line instead of once per game, and the row carries the button
  that opens "free up space" — which used to be buried in Account, three screens
  from wherever you were when it happened.
- **"Free up space" couldn't see space shared between two games.** When the same
  folder ends up tracked twice (it happens: the slug can change under you), both
  copies point at the same stored bytes, which belong exclusively to neither — so
  both reported "0 bytes to free" and archiving either one freed nothing. Those
  bytes are now counted and the pair is flagged as the duplicate it is. On the
  account that turned this up it was 1.25 GB: 60% of a Free plan, invisible.
- **"Free up space" picked your games for you.** It archived the heaviest ones
  until the numbers worked. Now it proposes that as a starting point and lets you
  tick what actually goes, with a live meter showing where your account lands —
  and it says so plainly when archiving everything still wouldn't be enough.

## [1.1.2] — 2026-08-07

### Added
- **Every version now tells you which machine it came from.** The history of a
  save listed a date and a size and left you to guess whether that snapshot was
  the desktop's or the laptop's — which is the one thing you actually want to
  know before restoring one. Each version now carries the name of the machine
  that made it, and the history shows it.
- **A heads-up display over the game.** Alt+H brings up a panel on top of
  whatever you are playing, in the shape of the Steam overlay, showing what the
  sync engine is doing right now: what it backed up, when, and whether anything
  is waiting. It reads the engine and nothing more — there is no button on it
  that can touch your saves — and Alt+H puts it away again.
- **The Hoard Screen scope can be bound to a mouse or keyboard button.** The
  magnifier used to live in the overlay's own controls; you can now put it on a
  button and choose how it behaves — press to toggle, hold while you aim, or
  show it for a fixed moment. Extra mouse buttons work as bindings too.
- **Scanning a folder you point at yourself.** Telling Hoard "the saves are in
  here" now means exactly that: it looks inside the folder you chose and offers
  what it finds, without applying the size and name rules it uses when guessing
  on its own — those rules exist for scanning your whole disk unattended, and
  they were throwing away folders you had explicitly pointed at. The three
  slightly different ways of adding a save by hand are now one dialog.
- **One install, whatever your machine is.** Hoard now installs and updates as
  a set of components rather than as "the app" or "the CLI": the installer works
  out which pieces your machine wants and puts them all in at the same version,
  in one pass. A NAS or a server stops at the engine and the terminal; a desktop
  or a Steam Deck gets the app as well. Upgrades move everything together, so
  the pieces can't drift apart, and the app now ships the terminal command with
  it instead of leaving it as a separate download.
- **Saves sync in game mode on SteamOS, Bazzite and CachyOS.** The sync engine
  is now installed in its own right instead of riding inside the app bundle, so
  it can start with your session on systems where the app has to be an AppImage
  — which is every immutable/atomic image, the Steam Deck included. Nothing to
  keep open and nothing to launch: install once from the desktop and game mode
  syncs on its own.

### Changed
- **A pass over the app's surfaces.** Behaviour on hover and focus, the
  contrast of the greys, the depth of panels and cards, and how versions are
  laid out in history all got a revision, with a control for how pronounced the
  relief is. Covers and the Library frames stayed as they were.

### Fixed
- **A save that the game rotated mid-upload could be stored corrupt.** Many
  games write a new save by renaming the old one out of the way, and if that
  happened between Hoard reading a file and finishing sending it, what reached
  the server was half of one file and half of another — a version that looked
  fine in the list and could not be restored. Hoard now checks what it actually
  sent, byte for byte, and aborts the whole snapshot if a file moved underneath
  it, so a bad version is never committed. The next backup picks up the new
  contents normally. **If you have used Hoard on a game that rotates saves,
  this is the fix to update for.**
- **"Hoard already tracks this folder" on a folder it did not track.** A save's
  identity was tied to a name derived from the game's title, and that name is
  not stable — the same game could be `vrising` on one machine and `v-rising`
  on another, or gain a year in the catalogue. Two different folders could
  collide on it and Hoard would refuse to add the second, with the only way out
  being to untrack and re-add. Identity is now the folder itself, which is what
  it always meant.
- **Restoring into an empty folder could loop.** If the folder a save lives in
  was empty — a fresh machine, a game reinstalled — the restore bypassed the
  check that decides whether there is anything to do, and a restore that wrote
  nothing still reported success, so it started again immediately. One account
  moved 10.6 GB this way. Both halves are fixed: the empty folder no longer
  skips the check, and a restore that writes nothing is a failure.
- **Hoard could be pointed at a folder no backup should ever cover.** Nothing
  stopped you from tracking a Wine prefix root, a `Documents`, or a home
  directory — a mistake that turns the next backup into an attempt to upload
  everything you own. Those roots are now refused, the Windows rules apply
  inside Proton prefixes as well as outside, and the check sits on the path the
  backup actually takes rather than only in the dialog.
- **Self-hosted: rebuilding your server left invisible duplicate rows.** A
  rebuilt server hands out new identifiers, and re-adding a save created a
  second row while the old one stayed behind — not shown anywhere, still
  holding a claim on the folder, and answering 404 for every sync. The stale
  rows are now dropped when the library is listed.
- **Self-hosted: an update could leave the server unable to find its own
  database.** The Docker stack shipped a `config.toml` in the repository, so a
  `git pull` overwrote yours — including `data_dir`, which is where your saves
  and your database are. A server that starts against an empty database where
  a populated one is expected now refuses to run and says so, and the config
  file is no longer versioned. Copy `deploy/config.toml.example` once, as the
  self-host guide says, and updates stop touching it.
- **404s from guessing what kind of server was on the other end.** When the
  check that asks a server what it is could not reach it, the client assumed
  self-hosted and spoke the wrong dialect to a Hoard Cloud server, which
  answered 404 to everything. An unreachable server and a self-hosted one are
  now two different answers, and the client waits for a real one.
- **Windows: a black console window at every sign-in.** The sync service was
  built as a console program, so Windows opened a terminal for it when the
  scheduled task started it with your session. It is a windowless program now.
- **Detection got a broad overhaul.** Eighteen changes to how Hoard works out
  where a game keeps its saves — following a game's launch command through
  wrappers to the process that actually runs, resolving base-folder references,
  handling saves that are a single loose file rather than a folder, and more.
  The bundled catalogue also went from 7.3 MB to 1.7 MB.
- **Server: a stuck compression job retried every five minutes, forever.** Six
  stored objects had been failing to compress since July with no terminal
  state, so the sweep picked them up again on every pass. Attempts are now
  counted and capped.
- **Server: rate-limit responses are readable from a browser again.** The
  limiter sat outside the layer that adds the cross-origin headers, so its 429
  never carried them and the web only ever saw "network error" — precisely when
  knowing the real status matters. The order is now the other way round, and
  the preflight no longer counts against your quota.
- **Diagnostic reports were never actually being sent.** Hoard has had a
  diagnostics channel since 1.0, on by default, and it has never delivered a
  single line: it looked for your session in the wrong place, so on a Hoard
  Cloud machine it found nothing and gave up, every time. It works now, and the
  reports carry what makes a bug findable — including where detection got a
  game's save folder wrong and how you fixed it, which until now only reached us
  when somebody wrote in on Discord. Two things changed alongside it: paths are
  stripped of your username before they leave your machine (`C:\Users\<user>\…`
  is what arrives), and the Settings toggle now says what is actually sent
  instead of promising "anonymous pings" that never leave out paths or game
  names. It is still one switch, still on by default, and turning it off still
  stops the stream within seconds.
- **Self-hosting on OneDrive, Mega, Google Drive or Dropbox actually works
  now.** The guide has pointed at `rclone serve s3` as the way to keep saves on
  a cloud drive you already pay for, without ever explaining how to set it up —
  and worse, going that route quietly stored every save wrong. The uploads were
  being framed in a way that AWS, R2 and MinIO unwrap and the rclone bridge does
  not, so what landed in your drive was not what was sent, and you'd only find
  out the day you needed a restore. The server now speaks the plainest version
  of the protocol, checks at startup that the storage it was given returns
  exactly the bytes it wrote (and refuses to start if not), and the self-host
  guide has a step-by-step section for each provider, including what the
  trade-offs are.
- **A backup to remote storage no longer holds up everyone else's.** While one
  save was uploading to an S3 bucket or a cloud drive, the rest of the server's
  writes queued behind it and started failing outright if it took more than a
  few seconds — which, on a consumer drive, it does. Uploads now happen outside
  the database lock.
- **Restoring from remote storage stopped needing a copy of the whole save on
  the server's disk.** A restore staged every file of the snapshot locally
  before sending any of it, so a 10 GB library needed 10 GB free on the server.
  It now streams a piece at a time (a few MB), and a download that gets cut
  short is reported as an error instead of quietly producing a short file.
- **The terminal install could not sync at all.** Since 1.1.0 `hoard` has been a
  thin client of the sync service, but the published tarball only ever contained
  `hoard` — never the `hoardd` engine it talks to. Installing from the terminal
  produced a command that could not start or reach a service, which is exactly
  what the headless install is for. The tarball now carries both halves, and CI
  refuses to publish one without the other.
- **Which engine ran no longer depends on who started it.** With the app and the
  terminal install both present, the running engine could be either copy
  depending on `PATH` order and which client woke it. The installed service is
  now the single authority on that, and clients follow it.
- **In-app updates on SteamOS, Bazzite and other atomic systems.** The updater
  offered an `.rpm` on any machine with `rpm` present, including images whose
  `/usr` is read-only — so the download succeeded and the install could not
  possibly apply. It now picks the format the machine can actually install, the
  same way the terminal installer does.

## [1.1.1] — 2026-08-02

### Added
- **A shareable card at the bottom of Hoard-Wrapped.** The recap now ends in a
  wide camera button that opens your card: photo, name, a random line that
  riffs on your most-played game (22 games, eight languages — play a Fallout
  and it says "war never changes"), your stats for the last week, month or
  year, and a row of cubes for that range (a week is seven big ones). Photo,
  name, line and range are editable and stay **on this device only** — nothing
  is uploaded or synced. A separate camera button takes the shot and drops the
  PNG in your gallery (`Pictures/Hoard/`), branded with hoard.services both on
  the image and in its PNG metadata.
- **Link a cloud save by picking the game, not the folder.** "Link to this
  machine" now lists the games detection already found here, best name match
  first, so a save synced from another device can be bound in one click.
  Games whose folder another save already tracks are left out, and the folder
  picker stays as the fallback for what detection genuinely missed.

### Fixed
- **Self-hosted sync was dead in 1.1.0 if you only ever signed in through the
  app.** Moving the engine into the background service also moved where it
  looks for your session: it read only `config.toml`, which just the CLI
  (`hoard login --token`) writes, while the app keeps its own. So the service
  started with no session, no save was ever backed up, and all the window could
  say was "the sync service is offline". The service now uses the app's session
  first and keeps `config.toml` as the headless fallback — nothing to redo, it
  picks up the session you already have on the next start.
- **"The sync service is offline" now says why, and offers the fix.** The
  reason existed inside the service and was dropped on the way to the window.
  It travels now: no session, a keyring that won't hand it over, an expired
  session — each with its own sentence, the raw error underneath for a bug
  report, and a "Sign in again" button on the cases that actually fixes.
- **The service takes ownership of the saved session.** When it starts from a
  session that was left in the file (a client that had no service to hand it
  to, or a keyring that was locked at the time), it now stores it in the
  keyring itself. On macOS that's what stops the password prompt on every
  engine start, since a keychain item only authorises the binary that created
  it.
- **The app re-hands its session to a service that has none.** If the service
  reports "no session" while the app has one, it hands it over instead of
  waiting out a backoff that can't fix anything on its own.
- **"Link to this machine" opened the file manager again.** The 1.0.4 UI
  rewrite dropped the wiring for the link dialog added in 1.0.3, so the button
  went straight to the OS folder picker — making you hand-find a save folder
  Hoard had already detected. The dialog is back.

## [1.1.0] — 2026-07-28

### Added
- **Hoard keeps syncing with the app closed.** The sync engine moved out of
  the window and into a local service (`hoardd`) that starts with your session
  and stays resident: the desktop app and the `hoard` CLI are now thin clients
  that talk to it over a local socket. Close the app mid-game and your saves
  still get backed up; open it again and it just attaches to the service
  that was already running. On Linux the service also sends the native
  notifications, so a finished backup tells you even with no window open
  (Windows and macOS still notify from the app).
- **Real game covers, in the shape covers are.** The panel now asks Steam for
  each game's vertical 2:3 art instead of the 460×215 store banner, so a card
  shows the actual cover instead of a center-cropped strip of one. Games with
  no vertical art keep their banner, letterboxed over a blurred blow-up of
  itself rather than cropped to a third of the image. You can frame the whole
  grid as 2:3 posters or as squares (toolbar, top right — the square is there
  for custom art that isn't a poster), and your own image still beats both:
  hover a cover and click the pencil in its corner.
- **Redesigned dashboard.** The list of rows is now a grid of cover cards, each
  one carrying what the row had no room for: last save, total size across
  versions, stored-version count, a per-game menu (rename, pause, history) and
  a status pill that always speaks for *this* device — the cloud's version
  rides in a separate chip over the cover. A summary bar at the bottom totals
  games, versions, size and last backup.
- **Sniper scope (magnifier) in Hoard Screen.** A lens — circle or square —
  that shows whatever is under it magnified (×1–×4), sniper-style. Drag and
  resize it anywhere; clicks pass through to the game, and a crosshair draws
  on top of it unmagnified. Windows-only capture for now; while a scope is
  active the overlay is excluded from recordings/OBS (it has to be, or the
  lens would magnify itself).
- **Layers panel in Hoard Screen.** An ordered list of everything on the
  overlay: click to select, arrows to decide what draws over what. New
  crosshairs start above everything; widgets always float over placed apps.
- **Crosshair widget in Hoard Screen.** The overlay grows its first
  non-capture widget: a procedural crosshair (cross, ×, dot or circle) with
  color, opacity, size, thickness, center gap, center dot and outline — all
  editable live from the Screen panel, per monitor or mirrored. It renders
  through the same compositing path on every OS, is always click-through,
  and stays pixel-crisp at any size.

### Changed
- **Restores skip what your disk already has.** Before downloading a version,
  the client indexes the live folder by content: any file whose contents
  already sit there is copied locally instead of fetched. Restoring a 400 MB
  Factorio save after a small change now moves single-digit megabytes over the
  network.

### Fixed
- **Dismissing a message from the bell now sticks.** Dismissing only removed
  it from that window: the next time the app checked in — a restart, or a
  minute later — the server sent it back and it reappeared, forever. The
  dismissal is now recorded on the server, so a message you close stays closed
  on every machine you sign in from and after a reinstall. (Operator
  broadcasts also reach the bell again at all, which they hadn't since 1.0.4.)
- **Updating on Windows no longer trips over the sync service.** With the
  service now outliving the window, the installer had to overwrite a file the
  daemon was holding open, and the update failed — leaving the app running
  without its service. The installer stops the service (and the overlay)
  before replacing anything, and the in-app updater downloads the installer
  that does so.
- **Saves no longer get tracked under an app's name.** A background app that
  happened to be busy while a save folder changed could be credited with it,
  so the panel grew entries called "ChatGPT", "opencode" or "Codex … Setup"
  pointing at another game's folder — and since each wrong name made a new
  entry, they piled up. AI/desktop apps, capture tools (OBS, Streamlabs) and
  file-sync clients (Dropbox, Nextcloud, Syncthing, …) are no longer taken for
  games, the same folder can't be tracked twice under different names, and
  entries already poisoned are dropped when a real game covers that folder.
- **The cloud panel no longer goes stale in silence.** A background task that
  died could leave the app showing versions that no longer matched the cloud,
  with nothing on screen saying so; the engine now watches the cloud itself,
  restarts the task that died, and says out loud when its view is stale.
- **A locked keyring can't freeze the app any more.** If the system keyring
  never answered (locked wallet, no unlock prompt), the engine hung and the
  service refused to stop. Keyring reads now give up after 5 seconds with a
  reason you can read.
- **Hoard Screen editor can no longer lose track of the overlay.** The editor
  now re-syncs with the overlay process every few seconds (and on open), so a
  panel that is really on screen — e.g. a TikTok capture while gaming — can
  always be moved or removed even if the app's own copy of the layout went
  stale (reload, missed event). The overlay also shuts itself down if the app
  dies instead of lingering as an unremovable ghost.

## [1.0.4] — 2026-07-18

### Added
- **Sort the panel.** Order the dashboard's games by last backup (new
  default) or by cloud size. Cloud saves now carry their real "last backup"
  time, so the recency sort works on Hoard Cloud too.
- **Cloud size at a glance.** Every game row in the panel shows the space it
  occupies in the cloud (and only in the cloud — local footprints live in
  the Library, clearly labelled as such).
- **Bulk-delete versions.** History grew a checkbox per version plus
  select-all: tick as many as you want and delete them in one confirmed go
  instead of one dialog per version.
- **Max versions per game.** A per-account cap on stored versions, set right
  in the panel (empty = unlimited, like before). The server enforces it after
  every backup and prunes immediately when you lower it — oldest versions go
  first; pinned versions and the newest one are never touched. If the new cap
  would delete anything, a confirmation dialog first tells you exactly how
  many versions are about to go (server-side dry-run, so the number is real).
  Works on Cloud and self-hosted (`hoard snapshots max-versions` in the CLI,
  same preview + `[y/N]` prompt, `--yes` to skip).

### Fixed
- **Leaner startup sync.** When several games need restoring at once (e.g.
  first launch of the day), the app now fetches the cloud save list once for
  the whole batch instead of once per game — faster startup and fewer
  requests.

### Changed
- **Faster cloud sync.** Backups now hash and upload several files at a time,
  and restores download several at a time, instead of strictly one by one.
  Saves made of many small files — the common case — sync noticeably faster
  in both directions.
- **Local vs. server sizes, labelled.** The Library's tracked-games header
  (local, this machine) and each card's size pill (server-side) now carry
  icons and tooltips saying which is which, so the two totals can no longer
  be confused.
- **Cloud poll cadence is now fixed (60 s).** The `/v1/cloud/sync` fallback
  poll is no longer a preference — Realtime push already delivers changes
  instantly, so a faster poll bought nothing and a hand-edited `prefs.json`
  could hammer the server. Existing prefs files keep loading; the old key is
  simply ignored.
- **Server: internal storage maintenance.** Background housekeeping of how
  the cloud tier stores snapshot data internally. No user-facing changes:
  quotas, sizes shown in the app and download behavior are identical.
- **Server: per-device rate limit on polling endpoints.** `/v1/cloud/sync`,
  `/v1/devices`, `/v1/notifications` and `/v1/presence/heartbeat` are now
  capped per (user, device, endpoint) — 10 requests/minute by default
  (`[server.rate_limit] poll_per_minute`, cloud mode). The official client
  polls each at most twice a minute, so only modified or misconfigured
  clients ever see the 429 (which carries `Retry-After`). The client now
  sends its device fingerprint on sync/notifications so the cap is truly
  per machine, and the devices-feed refresh floor went from 2 s to 10 s so
  many-device accounts stay well under the cap.

## [1.0.3] — 2026-07-15

Sync you can trust, and an app that feels alive. Three deep fixes end the
"reload Steam on both devices" dance and the download-timeout loop; on top of
that, see every machine on your account live, hear from the dev through an
in-app bell, and pick a theme.

### Added
- **The Eye: your devices, live.** (Cloud) A header panel listing every
  machine on the account — online dot, which games each one is running right
  now and for how long. Agents heartbeat every 30 s and beat instantly when a
  game starts or stops, so launching a game on the Deck shows on the desktop
  in a second or two; a crashed machine simply ages out of the window instead
  of staying green. Desktop and CLI daemon both report.
- **The bell: announcements from the dev.** (Cloud) Operator broadcasts land
  in seconds over Realtime push (cursor-based polling as fallback, so nothing
  is ever re-delivered), render a mini-markdown subset, can carry an action
  button and expire on their own. Only the operator can send one — rows are
  inserted via direct service-role SQL, there is no HTTP write path.
  Dismissals sync server-side: dismissed on one device, gone on all of them.
- **Themes.** Obsidian (the classic dark), Quartz (light) or Auto to follow
  the OS scheme, plus an accent-colour picker — all in Settings. A pure
  CSS-variable re-skin that persists locally.
- **Link a cloud save without hunting for the folder.** When a save lives in
  the cloud but isn't linked on this machine, the link dialog now leads with
  the folders detection already found here — one click and done. The folder
  picker stays as the fallback, and a never-scanned machine is offered the
  scan instead of a false "nothing found".
- **Rename works on Hoard Cloud saves.** The cloud grew the rename endpoint
  the self-hosted server already had; duplicate labels are rejected cleanly.
- **Wrapped: browse any year.** The playtime recap grew a year picker —
  every year with playtime, latest first.
- **Operator tools** in `tools/`: the broadcast sender
  (`send-notification.sh`) and a single-file metrics dashboard.

### Fixed
- **Saves from another device now arrive without reloading Steam.** On the
  Steam Deck, Proton often leaves zombie processes behind after a game
  closes, so the engine kept believing the game was still running and held
  the cross-device restore forever — the hold itself is deliberate (never
  swap saves under a live game), but it had no way out. Zombie processes no
  longer count as running, a held restore is delivered the moment the game
  actually stops, and while it waits the app says so ("update ready — waiting
  for the game to close") instead of staying silent. Failed backups also
  retry on a 10-minute backoff instead of wedging restores until the next
  file event.
- **A Cloud session can no longer die permanently.** Two internal refresh
  paths could race over the same refresh token, and losing that race revoked
  the whole token family — sync stopped for good until re-login plus a
  restart. Every refresh now goes through one serialized path that re-reads
  the token from disk and collapses bursts into a single request. If a
  session does expire, the daemon announces it once, re-checks quietly, and
  everything — refresher and realtime push — reconnects on its own after
  `hoard login`, no restart needed. Daemon boot also survives starting before
  the network is up instead of exiting.
- **Big saves no longer die with "operation timed out".** Snapshot transfers
  ran on an HTTP client whose 60-second total timeout covered the response
  body too, so any download longer than a minute (Paradox-sized saves) was
  killed mid-stream and retried in a loop — and slow uploads could hang the
  "Uploading…" pill the same way. Transfers now use dedicated streaming
  clients: no total cap, a stall detector on downloads, TCP keepalive on
  uploads.

## [1.0.2] — 2026-07-12

The open-source release. The whole app — including the Pro layer — now lives in
one AGPL repo, the CLI grows into a first-class frontend, and Hoard Wrapped is
free for everyone. Plus an official Docker image, packaging for more distros,
and a round of detection and reliability fixes.

### Added
- **The Pro layer is now open source, in this repo.** Hoard Screen (the in-game
  overlay) and Hoard Wrapped (the year-in-games recap) ship as regular AGPL
  crates. The paywall isn't the code — the Hoard Screen entitlement is signed
  server-side, so anyone can build it but only Cloud unlocks it. There's nothing
  to patch out locally.
- **Hoard Wrapped is free for everyone.** The playtime recap renders for Cloud
  and self-hosted alike, with no gate — a two-mode engine that generates the
  recap server-side on Cloud and locally when self-hosted.
- **The CLI is now a full frontend of the shared engine.** `hoard` and the
  desktop app run the exact same `hoard-agent` core, so every feature lands in
  both. New: an interactive `hoard login` flow that no longer needs a
  hand-pasted token.
- **Sign in the CLI by pairing a device.** Cloud login on a headless box can now
  be approved from an already-signed-in device instead of copying credentials
  around, with a `/link` page to complete the pairing.
- **More install options.** An official multi-arch Docker image on GHCR
  (`ghcr.io/rleeon/hoard`, amd64 + arm64) — `docker compose pull && docker
  compose up -d` to update instead of building on your box — plus `.rpm` and
  Snap packages for the desktop app.
- **Reclaim archived games from the app.** Games you archived to free quota now
  show up in Library and History with a **Reactivar** action, so bringing one
  back no longer means digging through the CLI.

### Fixed
- **AppImage on SteamOS / Bazzite and other newer distros.** The bundle no
  longer ships its own `libwayland-client`/`libEGL`/`libGL`/`libgbm` — those
  now resolve from the host, fixing the solid-white window and
  `could not create default EGL display: EGL_BAD_PARAMETER` that forced users
  to launch with `LD_PRELOAD`.
- **Sign-in did nothing under the AppImage.** Outward links (OAuth sign-in,
  upgrade/billing, terms) now open through a Rust `open_external` command that
  strips the AppImage-injected loader env, so the browser starts against the
  host's libraries instead of Hoard's bundled (mismatched) ones and actually
  appears.
- **Detection sweep.** Several fixes to game/save detection and the backup
  queue, so more games are found automatically and fewer get stuck.
- **No more phantom "game started" flaps.** A brief CPU dip on a correlation
  match is now debounced instead of flapping the running-game state.
- **One agent per machine.** A single-instance lock stops two daemons from
  rotating the same token and 401-ing each other's syncs.
- **Safer self-hosted upgrades.** `hoard-server upgrade` refuses to run inside a
  container and points you at rebuilding the image instead of swapping a binary
  that a `docker compose pull` will overwrite.

### Changed
- **Failed syncs are now visible.** Bandwidth-window rejections are recorded in
  `sync_log` alongside quota rejections, so the sync failure rate is no longer
  invisible.
- **Storage downgrade grace widened to 30 days** (was 14) — more room before a
  plan change trims your ceiling.
- **Community docs in the repo.** Added CONTRIBUTING, a self-hosting guide, a
  funding breakdown, and a GitHub Sponsor button.
- CI now runs only on version tags, pull requests, and manual dispatch —
  routine branch pushes (including docs-only edits) no longer spend Actions
  minutes. Validate locally with `cargo check` + `pnpm check` before pushing.

## [1.0.1] — 2026-07-09

The reliability release. A single-PC data-loss window in Global Sync is closed
for good, cloud limits get roomier across the board, and running out of quota
is no longer a dead end — you can now buy your way *down* by archiving the
whales instead of deleting anything.

### Added
- **Reclaim quota without deleting a single byte.** When your live saves push
  past the plan ceiling, a new dialog ranks your games by footprint and lets
  you archive the heaviest ones. Archiving frees the quota **instantly**
  (refcount drops, `/v1/me` reflects it on the next poll) while the cloud copy
  is frozen and stays downloadable for a 7-day grace window before a cron
  purges it. Your local save is never touched, and the whole thing is
  reversible the moment you upgrade — it's an escape hatch, not a guillotine.
- **Wrapped credits playtime for *any* Steam game you actually run** — even
  ones with no local save to capture and no catalog entry (online-only titles,
  private servers, War Selection, and friends). When the agent sees a process
  launch from its Steam install dir, it attributes the time. Nothing gets
  enrolled and the "Played, not backed up" list stays clean; Proton, runtimes
  and SteamVR are filtered out so they never book phantom hours.

### Changed
- **Cloud limits, meaningfully bigger.** Storage: Free **1 → 2 GB**, Pro
  **25 → 100 GB**. Per-save ceiling: Free **200 MB → 1 GB**, Pro **2 → 10 GB**.
  Rolling 15-minute bandwidth window: Free **→ 3 GB**, Pro **→ 15 GB** (kept
  above the max single-save size so a first upload can never wedge itself
  behind its own window). The Pro base tier no longer pins a per-user storage
  override, so raising the plan default now actually reaches existing
  subscribers on renewal instead of being shadowed by a stale `storage_gb`.
- **Account screen: dropped the redundant "Compare plans" button** and its
  modal — one fewer detour between you and the upgrade CTA.

### Fixed
- **Global Sync can no longer clobber an in-progress save (real data loss).**
  With Sync on, three independent code paths — the SSE/poller instant pull, the
  reconciliation sweep, and the pre-launch barrier — bypassed the live-session
  guards. On a *single* PC that meant an automatic pull could re-apply the last
  uploaded version on top of progress the autosave hadn't captured yet, and
  those intermediate saves were never versioned at all (reproducible loss with
  R.E.P.O.). Every automatic pull now waits for the game to close and the save
  to settle. The legitimate multi-device path is untouched: an idle machine
  still pulls the new version immediately, and genuine divergence is resolved by
  upload-conflict reconciliation rather than a silent overwrite.
- **In-progress work is versioned in seconds, not left in a queue.** When a
  pull is deferred for a live session and there are un-uploaded local changes,
  the agent now pushes them immediately — skipping the data-saving interval —
  instead of parking them in the backup queue. What you played exists as a cloud
  version within seconds even if it isn't the version you ultimately keep; if
  the cloud was ahead, upload-conflict reconciliation versions both sides.
- **"Export all data" can't hang forever anymore.** An export job that died
  mid-build (worker restart) left a phantom `running` row that blocked every
  subsequent attempt. A reaper now marks jobs stale after 1h so you can retry,
  and the button stays responsive even when the delivery email never lands.
- **The reclaim-storage dialog shows real game names** instead of a wall of
  "main", and a failed load surfaces a clear message with a retry button
  instead of a raw error string.
