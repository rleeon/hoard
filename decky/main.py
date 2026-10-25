"""Backend of the Hoard Decky plugin.

Game Mode has no desktop notifications: gamescope does not show them, so a
backup that failed at 3 a.m. on a handheld simply never reaches anyone. This
backend talks to the user's `hoardd` over its local socket, the same way the
desktop app and `hoard sync run` do, and forwards what matters to the frontend,
which turns it into Steam toasts and a panel in the Quick Access Menu.

Protocol (hoard-core `ipc`): every frame is a big-endian u32 length plus JSON.
The client says `hello`, the daemon answers `welcome` with its journal cursor,
and a `subscribe` from that cursor streams every new event. Requests that are
not the subscription go over a short connection of their own.
"""

from __future__ import annotations

import asyncio
import json
import os
import pwd
import struct

import decky

PROTOCOL = 1
CLIENT = "hoard-decky 0.1.0"
RECENT = 30
RETRY_SECS = 5
STATUS_POLL_SECS = 10
MAX_FRAME = 8 * 1024 * 1024


FLATPAK_ID = "services.hoard.saves"


def socket_path() -> str:
    # Decky starts backends without the user's session environment, so
    # XDG_RUNTIME_DIR is not to be trusted here: hoardd binds under
    # /run/user/<uid>/hoard. A Flatpak hoardd sees the same path from inside,
    # but on the host it lives in the app's private runtime dir.
    run = f"/run/user/{pwd.getpwnam(decky.DECKY_USER).pw_uid}"
    native = f"{run}/hoard/hoardd.sock"
    flatpak = f"{run}/.flatpak/{FLATPAK_ID}/xdg-run/hoard/hoardd.sock"
    if not os.path.exists(native) and os.path.exists(flatpak):
        return flatpak
    return native


async def write_frame(writer: asyncio.StreamWriter, msg: dict) -> None:
    body = json.dumps(msg).encode()
    writer.write(struct.pack(">I", len(body)) + body)
    await writer.drain()


async def read_frame(reader: asyncio.StreamReader) -> dict:
    (size,) = struct.unpack(">I", await reader.readexactly(4))
    if size > MAX_FRAME:
        raise ValueError(f"frame of {size} bytes")
    return json.loads(await reader.readexactly(size))


async def handshake(reader, writer) -> dict:
    await write_frame(
        writer, {"frame": "hello", "protocol": PROTOCOL, "client": CLIENT, "caps": []}
    )
    frame = await read_frame(reader)
    if frame.get("frame") == "rejected":
        raise RuntimeError(
            f"hoardd {frame.get('daemon_version')} refused protocol {PROTOCOL}: {frame.get('reason')}"
        )
    if frame.get("frame") != "welcome":
        raise RuntimeError(f"unexpected first frame {frame.get('frame')!r}")
    return frame


async def request(op: dict, timeout: float = 10.0) -> dict:
    """One request over its own connection; returns the reply's payload. The
    whole exchange, handshake included, sits under `timeout`: a daemon that
    accepts and never answers must not leave a button spinning forever."""
    return await asyncio.wait_for(_request(op), timeout)


async def _request(op: dict) -> dict:
    reader, writer = await asyncio.open_unix_connection(socket_path())
    try:
        await handshake(reader, writer)
        await write_frame(writer, {"frame": "request", "id": 1, "request": op})
        while True:
            frame = await read_frame(reader)
            if frame.get("frame") == "reply" and frame.get("id") == 1:
                reply = frame["reply"]
                if reply.get("result") != "ok":
                    # IpcError travels as {"result": "error", "error": <code>, ...}
                    # plus a human field that depends on the code.
                    detail = reply.get("message") or reply.get("op") or reply.get("error")
                    raise RuntimeError(str(detail or "the service refused the request"))
                return reply
    finally:
        writer.close()


def settings_path() -> str:
    return os.path.join(decky.DECKY_PLUGIN_SETTINGS_DIR, "settings.json")


def plain_error(error: str) -> str:
    """The engine's errors are written for logs. A toast gets the gist; the
    full text stays in hoardd's log."""
    e = (error or "").lower()
    if "error sending request" in e or "connection refused" in e or "dns error" in e:
        return "can't reach the server"
    if "timed out" in e or "timeout" in e:
        return "the server took too long to answer"
    if " 401" in e or "unauthorized" in e:
        return "signed out, open Hoard to sign in again"
    if "no space left" in e:
        return "the disk is full"
    if "permission denied" in e:
        return "a file could not be read (permission denied)"
    return error if len(error) <= 90 else error[:87] + "..."


def engine_problem(engine: dict) -> str:
    """Why the engine is down, in a line for the panel. The service answering
    says nothing about the engine: between boot and the first login a locked
    keyring keeps it down while the socket is up and well."""
    if engine.get("running"):
        return ""
    reason = engine.get("reason") or "unknown"
    if reason == "no_session":
        return "Signed out, open Hoard to sign in"
    if reason == "session_expired":
        return "The session expired, open Hoard to sign in again"
    if reason == "keyring_unreadable":
        if engine.get("keyring") in ("missing", "locked"):
            return "Waiting for the system keyring. If it stays like this, sign in again from Hoard in Desktop Mode"
        return "The system keyring won't hand over the session, open Hoard to sign in again"
    if engine.get("last_error"):
        return plain_error(engine["last_error"])
    return "Starting"


def size(n: int) -> str:
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024 or unit == "GB":
            return f"{n:.0f} {unit}" if unit == "B" else f"{n:.1f} {unit}"
        n /= 1024
    return f"{n} B"


class Plugin:
    def __init__(self):
        self.connected = False
        self.error = ""
        self.running = False
        self.problem = ""
        self.daemon_version = ""
        self.server = ""
        self.watched = 0
        self.names: dict[str, str] = {}
        self.slugs: dict[str, str] = {}
        self.slots: list[dict] = []
        self.recent: list[dict] = []
        self.unnamed: set[str] = set()
        self.epoch = ""
        self.debug_toasts = False

    # ---- naming

    def name_of(self, ev: dict) -> str:
        # `label` is the partida's name inside a game ("main" unless the user
        # renamed it), so the game comes first and the label only when it says
        # something.
        sid = ev.get("save_id", "")
        game = self.names.get(sid) or self.slugs.get(sid) or ev.get("game_slug") or "a save"
        if game == game.lower() and " " not in game:
            # hoardd 1.2.2 names a slot by its slug ("hollow-knight"); the
            # catalogue's real name is not on the socket yet.
            game = game.replace("-", " ").title()
        label = ev.get("label") or ""
        return f"{game} ({label})" if label and label != "main" and label not in game else game

    def describe(self, ev: dict) -> tuple[str, str, str, bool] | None:
        """(level, title, body, toast) for an AgentEvent, or None to drop it.

        Toasts are for what someone in a game wants to know without asking:
        a save landed, failed, came back. The rest only goes to the panel,
        where it helps answer "did Hoard even see my game?".
        """
        kind = ev.get("type")
        name = self.name_of(ev)
        if kind == "backup_started":
            # The result follows within seconds; listing both doubles the panel.
            return None
        if kind == "backup_success":
            if ev.get("already_landed"):
                return ("info", "Already up to date", name, False)
            return ("ok", "Save backed up", name, True)
        if kind == "backup_failed":
            retry = " Hoard will retry." if ev.get("will_retry") else ""
            return ("error", "Backup failed", f"{name}: {plain_error(ev.get('error', ''))}.{retry}", True)
        if kind == "backup_throttled":
            return ("warn", "Backup delayed", f"{name}, retrying in {ev.get('retry_after_secs')} s", False)
        if kind == "backup_too_large":
            body = f"{name} is {size(ev.get('actual_bytes', 0))}, the limit is {size(ev.get('limit_bytes', 0))}"
            return ("error", "Save too large", body, True)
        if kind == "backup_quota_full":
            return ("error", "Storage full", f"{name} was not uploaded", True)
        if kind == "backup_trimmed":
            return ("warn", "Partial backup", f"{name}: {ev.get('omitted_files')} files left out", True)
        if kind == "backup_files_unreadable":
            return ("warn", "Some files could not be read", f"{name}: {ev.get('count')} files", True)
        if kind == "backup_needs_attention":
            return ("error", "Backup needs attention", f"{name}: {plain_error(ev.get('error', ''))}", True)
        if kind == "backup_attention_cleared":
            return ("ok", "Backup back to normal", name, False)
        if kind == "backup_skipped_empty":
            if ev.get("likely_wrong_path"):
                return ("warn", "Nothing to back up", f"{name}: the save folder may be the wrong one", True)
            return ("info", "Nothing to back up", name, False)
        if kind == "save_auto_restored":
            return ("ok", "Save restored", name, True)
        if kind in ("save_auto_restore_failed", "save_auto_restore_stuck"):
            return ("error", "Restore failed", f"{name}: {plain_error(ev.get('error', ''))}", True)
        if kind == "save_auto_restore_recovered":
            return ("ok", "Restore back to normal", name, False)
        if kind == "restore_deferred":
            return ("info", "Restore waiting", f"{name}: {ev.get('reason')}", False)
        if kind == "save_conflicts_backed_up":
            return ("warn", "Conflicting files kept aside", f"{name}: {ev.get('count')} files", True)
        if kind == "game_started":
            return ("info", "Game detected", name, self.debug_toasts)
        if kind == "game_stopped":
            return ("info", "Game closed", name, self.debug_toasts)
        if kind == "heavy_process_detected":
            # hoardd's hint to run a detection scan now. Internal, and its
            # "looks like a game" guess also catches system tools, so it only
            # shows up when the user asked to see detection.
            if not self.debug_toasts:
                return None
            return ("info", "Possible new game spotted", ev.get("name", ""), False)
        if kind == "backup_scheduled":
            return None
        return ("info", kind or "event", name, False)

    # ---- event stream

    async def emit_state(self):
        await decky.emit("hoard_state", await self.get_state())

    async def handle(self, entry: dict, quiet: bool = False):
        ev = entry.get("event") or {}
        sid = ev.get("save_id")
        if sid and ev.get("game_slug"):
            self.slugs[sid] = ev["game_slug"]
        if sid and sid not in self.names and sid not in self.unnamed:
            # A save tracked after we connected: its display name is in the
            # service's status, not in the event. Asked once: a backlog of saves
            # nobody watches any more would otherwise cost a connection each.
            try:
                await self.refresh_status()
            except Exception as e:
                decky.logger.warning(f"status: {e}")
            if sid not in self.names:
                self.unnamed.add(sid)
        described = self.describe(ev)
        if described is None:
            return
        level, title, body, toast = described
        # The journal's seq restarts with every hoardd run, so it only identifies
        # an event together with the run's epoch. Keyed on seq alone, a restarted
        # daemon's first events matched old ones and were dropped in silence.
        key = f"{self.epoch}:{entry.get('seq')}"
        if any(r["key"] == key for r in self.recent):
            return
        item = {"key": key, "at": entry.get("at"), "level": level,
                "title": title, "body": body, "toast": toast and not quiet}
        self.recent.insert(0, item)
        del self.recent[RECENT:]
        decky.logger.info(f"event {ev.get('type')}: {title} / {body}")
        await decky.emit("hoard_event", item)

    async def refresh_status(self):
        reply = await request({"op": "status"})
        engine = reply.get("engine") or {}
        self.daemon_version = reply.get("daemon_version", "")
        self.running = bool(engine.get("running"))
        self.problem = engine_problem(engine)
        self.server = engine.get("server") or ""
        self.watched = engine.get("watched", 0)
        self.slots = [
            {"save_id": s.get("save_id"),
             "name": s.get("display_name") or s.get("save_id"),
             "path": s.get("path", ""),
             "running": bool(s.get("process_running")),
             "armed": bool(s.get("watcher_armed"))}
            for s in reply.get("slots") or []
        ]
        for s in reply.get("slots") or []:
            if s.get("display_name"):
                self.names[s["save_id"]] = s["display_name"]

    async def follow(self):
        epoch, cursor, replay_until = None, None, 0
        while True:
            try:
                reader, writer = await asyncio.open_unix_connection(socket_path())
                try:
                    welcome = await handshake(reader, writer)
                    # Same daemon run: pick up where we left. A new run (or the
                    # first connection) backs up a little to fill the panel, and
                    # that backlog is shown but never toasted: a reconnect must
                    # not replay an hour of notifications.
                    fresh = welcome.get("epoch") != epoch
                    since = max(0, welcome.get("cursor", 0) - RECENT) if fresh else cursor
                    replay_until = welcome.get("cursor", 0) if fresh else 0
                    epoch = welcome.get("epoch")
                    self.epoch = epoch or ""
                    await write_frame(writer, {"frame": "request", "id": 1,
                                               "request": {"op": "subscribe", "since": since}})
                    self.connected, self.error = True, ""
                    try:
                        await self.refresh_status()
                    except Exception as e:  # status is a nicety, events are the point
                        decky.logger.warning(f"status: {e}")
                    await self.emit_state()
                    while True:
                        frame = await read_frame(reader)
                        kind = frame.get("frame")
                        if kind == "event":
                            cursor = frame.get("seq", cursor)
                            await self.handle(frame, quiet=frame.get("seq", 0) <= replay_until)
                        elif kind == "reply" and frame.get("reply", {}).get("payload") == "backlog":
                            reply = frame["reply"]
                            for entry in reply.get("entries") or []:
                                await self.handle(entry, quiet=entry.get("seq", 0) <= replay_until)
                            cursor = reply.get("cursor", cursor)
                        elif kind == "resync":
                            # The push dropped rows. Asked again from our own cursor,
                            # the journal hands them over; from the daemon's (the
                            # frame's) they would be skipped. Shown, not toasted: it
                            # takes hundreds of rows in a burst to get here.
                            decky.logger.warning(f"hoardd: {frame.get('dropped')} events behind, re-reading the journal")
                            replay_until = frame.get("cursor") or 0
                            await write_frame(writer, {"frame": "request", "id": 2,
                                                       "request": {"op": "subscribe", "since": cursor}})
                        elif kind == "goodbye":
                            raise ConnectionError(frame.get("reason") or "goodbye")
                finally:
                    writer.close()
            except asyncio.CancelledError:
                raise
            except Exception as e:
                message = "Hoard's sync service is not running" if isinstance(
                    e, (FileNotFoundError, ConnectionRefusedError)) else str(e)
                if self.connected or self.error != message:
                    decky.logger.warning(f"hoardd: {message}")
                    self.connected, self.error = False, message
                    await self.emit_state()
            await asyncio.sleep(RETRY_SECS)

    async def poll_status(self):
        # The status read on (re)connect can come before hoardd has loaded its
        # saves ("0 saves", no server), and nothing else would correct it while
        # the panel stays open. A local socket round trip, so cheap.
        while True:
            await asyncio.sleep(STATUS_POLL_SECS)
            if not self.connected:
                continue
            before = (self.watched, self.server, self.slots, self.running, self.problem)
            try:
                await self.refresh_status()
            except Exception as e:
                decky.logger.warning(f"status: {e}")
                continue
            if (self.watched, self.server, self.slots, self.running, self.problem) != before:
                await self.emit_state()

    # ---- callables

    async def get_state(self) -> dict:
        return {
            "connected": self.connected,
            "error": self.error,
            "running": self.running,
            "problem": self.problem,
            "daemon_version": self.daemon_version,
            "server": self.server,
            "watched": self.watched,
            "slots": self.slots,
            "recent": self.recent,
            "debug_toasts": self.debug_toasts,
        }

    async def refresh(self) -> dict:
        if self.connected:
            try:
                await self.refresh_status()
            except Exception as e:
                decky.logger.warning(f"status: {e}")
        return await self.get_state()

    async def sync_now(self) -> str:
        # A "back up now" per save, the request that may lift a failure backoff
        # (hoardd keeps its own budget for that). The results arrive as events.
        try:
            await self.refresh_status()
            if not self.slots:
                return "no saves are being watched yet"
            for slot in self.slots:
                await request({"op": "backup_now", "save_id": slot["save_id"]})
            return ""
        except Exception as e:
            return str(e)

    async def set_debug_toasts(self, enabled: bool) -> None:
        self.debug_toasts = bool(enabled)
        try:
            os.makedirs(decky.DECKY_PLUGIN_SETTINGS_DIR, exist_ok=True)
            with open(settings_path(), "w") as f:
                json.dump({"debug_toasts": self.debug_toasts}, f)
        except Exception as e:
            decky.logger.warning(f"settings: {e}")

    # ---- lifecycle

    async def _main(self):
        try:
            with open(settings_path()) as f:
                self.debug_toasts = bool(json.load(f).get("debug_toasts"))
        except Exception as e:
            # A missing or unreadable file just means the default; nothing here
            # may keep the plugin from following hoardd.
            if not isinstance(e, FileNotFoundError):
                decky.logger.warning(f"settings: {e}")
        loop = asyncio.get_event_loop()
        self.task = loop.create_task(self.follow())
        self.poller = loop.create_task(self.poll_status())
        decky.logger.info(f"following {socket_path()}")

    async def _unload(self):
        for name in ("task", "poller"):
            task = getattr(self, name, None)
            if task:
                task.cancel()
