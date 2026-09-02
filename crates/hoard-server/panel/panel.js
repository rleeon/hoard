/* Hoard panel.
 *
 * No framework and no build step: the whole thing is three files compiled into
 * the server binary, and adding a toolchain would mean a self-hoster's `cargo
 * install` needs node. The trade is that state management is manual, so it is
 * kept trivial, every tab re-fetches and re-renders, there is no client-side
 * cache to invalidate, and the only shared state is the session and the
 * locale.
 *
 * Nodes are built with h() rather than innerHTML. Half the strings on these
 * pages are user-controlled (game slugs, save labels, device names, log lines
 * shipped from a client), so string-splicing HTML here would be an injection
 * with a straight path from "name a save `<img onerror>`" to the admin's
 * browser.
 */

const LOCALES = [
  ["en", "English"],
  ["es", "Español"],
  ["fr", "Français"],
  ["de", "Deutsch"],
  ["pt", "Português"],
  ["it", "Italiano"],
  ["ja", "日本語"],
  ["zh", "简体中文"],
];

const LOCALE_KEY = "hoard.panel.locale";

let S = {};            // active locale strings
let locale = "en";
let me = null;         // { username, is_admin } once past the gate
let overview = null;   // last /v1/me/overview payload
let adminData = null;  // last /v1/admin/overview payload

// ---------------------------------------------------------------------------
// tiny DOM builder
// ---------------------------------------------------------------------------

function h(tag, props, ...kids) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props || {})) {
    if (v === null || v === undefined || v === false) continue;
    if (k === "class") node.className = v;
    else if (k === "text") node.textContent = v;
    else if (k === "css") for (const [p, val] of Object.entries(v)) node.style.setProperty(p, val);
    else if (k.startsWith("on")) node.addEventListener(k.slice(2), v);
    else node.setAttribute(k, v === true ? "" : v);
  }
  // flat(Infinity): the renderers below build nested arrays (a list of games,
  // each holding a list of saves), and a single flat() would leave the inner
  // array to be stringified into "[object HTMLDetailsElement]".
  for (const kid of kids.flat(Infinity)) {
    if (kid === null || kid === undefined || kid === false) continue;
    node.append(kid.nodeType ? kid : document.createTextNode(String(kid)));
  }
  return node;
}

const $ = (id) => document.getElementById(id);

function clear(node, ...kids) {
  node.replaceChildren(...kids.flat(Infinity).filter(Boolean));
  return node;
}

// ---------------------------------------------------------------------------
// i18n
// ---------------------------------------------------------------------------

function t(key, vars) {
  let s = S[key];
  if (s === undefined) return key;
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.replaceAll("{" + k + "}", v);
  return s;
}

function pickLocale() {
  const saved = localStorage.getItem(LOCALE_KEY);
  if (saved && LOCALES.some(([c]) => c === saved)) return saved;
  for (const want of navigator.languages || [navigator.language || "en"]) {
    const primary = String(want).toLowerCase().split(/[-_]/)[0];
    const hit = LOCALES.find(([c]) => c === primary);
    if (hit) return hit[0];
  }
  return "en";
}

async function setLocale(code) {
  const res = await fetch("/panel/i18n/" + encodeURIComponent(code));
  if (!res.ok) throw new Error("locale " + code);
  S = await res.json();
  locale = code;
  localStorage.setItem(LOCALE_KEY, code);
  document.documentElement.lang = code;
  document.documentElement.dataset.locale = code;
  applyStatic();
}

/** Fill every element the HTML tagged with a key. Attributes get their own
 *  tags because a placeholder or an aria-label is as translatable as text. */
function applyStatic() {
  for (const el of document.querySelectorAll("[data-i18n]")) {
    el.textContent = t(el.dataset.i18n);
  }
  for (const el of document.querySelectorAll("[data-i18n-placeholder]")) {
    el.placeholder = t(el.dataset.i18nPlaceholder);
  }
  for (const el of document.querySelectorAll("[data-i18n-label]")) {
    el.setAttribute("aria-label", t(el.dataset.i18nLabel));
  }
  document.title = me ? t("app.title_user", { user: me.username }) : "Hoard";
}

// ---------------------------------------------------------------------------
// formatting
// ---------------------------------------------------------------------------

const UNITS = ["B", "KiB", "MiB", "GiB", "TiB"];

function bytes(n) {
  if (n === null || n === undefined) return "—";
  let v = Number(n);
  let i = 0;
  while (Math.abs(v) >= 1024 && i < UNITS.length - 1) { v /= 1024; i++; }
  const digits = i === 0 ? 0 : v < 10 ? 1 : 0;
  return new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(v) + " " + UNITS[i];
}

/** Split so the caller can render the unit smaller than the figure. */
function bytesParts(n) {
  const s = bytes(n);
  const cut = s.lastIndexOf(" ");
  return cut < 0 ? [s, ""] : [s.slice(0, cut), s.slice(cut + 1)];
}

function num(n) {
  return new Intl.NumberFormat(locale).format(Number(n || 0));
}

function hours(secs) {
  const h = Math.floor(secs / 3600);
  const m = Math.round((secs % 3600) / 60);
  if (h === 0 && m === 0) return t("time.none");
  if (h === 0) return t("time.minutes", { n: m });
  return t("time.hours", { n: num(h), m: String(m).padStart(2, "0") });
}

function when(iso) {
  if (!iso) return "—";
  const d = new Date(iso.endsWith("Z") || iso.includes("+") ? iso : iso + "Z");
  if (isNaN(d)) return iso;
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(d);
}

function ago(iso) {
  if (!iso) return "—";
  const d = new Date(iso.endsWith("Z") || iso.includes("+") ? iso : iso + "Z");
  if (isNaN(d)) return iso;
  const secs = (d.getTime() - Date.now()) / 1000;
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  const steps = [[60, "second", 1], [3600, "minute", 60], [86400, "hour", 3600],
                 [2592000, "day", 86400], [31536000, "month", 2592000]];
  for (const [limit, unit, div] of steps) {
    if (Math.abs(secs) < limit) return rtf.format(Math.round(secs / div), unit);
  }
  return rtf.format(Math.round(secs / 31536000), "year");
}

function pct(part, whole) {
  if (!whole) return 0;
  return Math.min(100, Math.max(0, (part / whole) * 100));
}

// ---------------------------------------------------------------------------
// api
// ---------------------------------------------------------------------------

class ApiError extends Error {
  constructor(status, code, body) {
    super(code || String(status));
    this.status = status;
    this.code = code;
    // Some failures carry a number worth showing, how long the login throttle
    // is holding the door, for one. Keeping the parsed body means the caller
    // doesn't have to re-read a consumed response.
    this.body = body || {};
  }
}

async function api(path, opts = {}) {
  const res = await fetch(path, {
    ...opts,
    headers: { ...(opts.body ? { "content-type": "application/json" } : {}), ...(opts.headers || {}) },
  });
  if (res.ok) return res.status === 204 ? null : res.json();

  let body = {};
  try { body = await res.json(); } catch { /* not every failure is JSON */ }
  const code = body.error || null;

  // A 401 normally means the session died under us, expired, or revoked from
  // the users tab, and the honest response is to show the gate. But the
  // password-change endpoint answers 401 for a wrong *current* password, and
  // treating that as a dead session threw the user out to the login screen
  // over a typo. The server distinguishes them by code; trust it.
  if (res.status === 401 && code !== "invalid_credentials" && me) {
    me = null;
    showGate(t("error.session_expired"));
  }
  throw new ApiError(res.status, code, body);
}

function errorText(e) {
  if (e instanceof ApiError && e.code && S["error." + e.code]) return t("error." + e.code);
  if (e instanceof ApiError && e.status === 403) return t("error.admin_only");
  return t("error.generic");
}

// ---------------------------------------------------------------------------
// toast + dialogs
// ---------------------------------------------------------------------------

let toastTimer = null;

function toast(msg, bad) {
  const el = $("toast");
  el.textContent = msg;
  el.classList.toggle("bad", !!bad);
  el.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { el.hidden = true; }, 4000);
}

function confirmDialog(title, body, okLabel) {
  const dlg = $("dlg-confirm");
  $("confirm-title").textContent = title;
  $("confirm-body").textContent = body;
  $("confirm-ok").textContent = okLabel || t("common.confirm");
  return new Promise((resolve) => {
    dlg.addEventListener("close", () => resolve(dlg.returnValue === "ok"), { once: true });
    dlg.showModal();
  });
}

// ---------------------------------------------------------------------------
// gate
// ---------------------------------------------------------------------------

function showGate(message) {
  $("app").hidden = true;
  $("gate").hidden = false;
  const err = $("gate-error");
  err.textContent = message || "";
  err.hidden = !message;
}

async function enter(payload) {
  me = payload;
  $("gate").hidden = true;
  $("app").hidden = false;
  $("who-name").textContent = me.username;
  $("who-admin").hidden = !me.is_admin;
  for (const el of document.querySelectorAll(".admin-only")) el.hidden = !me.is_admin;
  applyStatic();
  await refresh();
  listen();
}

async function submitLogin(ev) {
  ev.preventDefault();
  const btn = $("f-submit");
  btn.disabled = true;
  try {
    const payload = await api("/v1/auth/login", {
      method: "POST",
      body: JSON.stringify({
        username: $("f-username").value,
        password: $("f-password").value,
      }),
    });
    $("f-password").value = "";
    $("gate-error").hidden = true;
    await enter(payload);
  } catch (e) {
    const msg = e instanceof ApiError && e.code === "too_many_attempts"
      ? t("error.too_many_attempts", { n: e.body.retry_after_secs ?? 10 })
      : errorText(e);
    showGate(msg);
  } finally {
    btn.disabled = false;
  }
}

/** The token never touches storage: it goes out as one Bearer header and what
 *  comes back is an httpOnly cookie. */
async function submitToken(ev) {
  ev.preventDefault();
  const btn = $("t-submit");
  btn.disabled = true;
  try {
    const payload = await api("/v1/auth/session", {
      method: "POST",
      headers: { authorization: "Bearer " + $("f-token").value.trim() },
    });
    $("f-token").value = "";
    $("gate-error").hidden = true;
    await enter(payload);
  } catch (e) {
    showGate(e instanceof ApiError && e.status === 401 ? t("error.bad_token") : errorText(e));
  } finally {
    btn.disabled = false;
  }
}

function toggleMode() {
  const tokenMode = $("token-form").hidden;
  $("token-form").hidden = !tokenMode;
  $("login-form").hidden = tokenMode;
  $("toggle-mode").textContent = tokenMode ? t("gate.use_password") : t("gate.use_token");
  $("gate-error").hidden = true;
}

async function logout() {
  try { await api("/v1/auth/logout", { method: "POST" }); } catch { /* leaving anyway */ }
  me = null;
  location.reload();
}

// ---------------------------------------------------------------------------
// summary
// ---------------------------------------------------------------------------

function stat(label, value, unit, sub, meter) {
  return h("div", { class: "stat" },
    h("span", { class: "stat-label", text: label }),
    h("span", { class: "stat-value" }, value, unit ? h("span", { class: "unit", text: unit }) : null),
    sub ? h("span", { class: "stat-sub", text: sub }) : null,
    meter !== undefined
      ? h("div", { class: "meter" + (meter > 90 ? " over" : "") },
          h("i", { css: { "--pct": meter.toFixed(1) + "%" } }))
      : null);
}

function renderSummary() {
  const o = overview;
  const [storedV, storedU] = bytesParts(o.storage.stored_bytes);
  const quotaUsed = pct(o.storage.used_bytes, o.storage.quota_bytes);
  const saved = o.storage.logical_bytes - o.storage.stored_bytes;

  clear($("summary-stats"),
    stat(t("stat.stored"), storedV, storedU,
      t("stat.of_quota", { quota: bytes(o.storage.quota_bytes), pct: Math.round(quotaUsed) }),
      quotaUsed),
    stat(t("stat.versions"), num(o.counts.versions), "",
      o.counts.trashed_versions > 0
        ? t("stat.across_saves", { saves: num(o.counts.saves), games: num(o.counts.games) })
          + " · " + t("stat.versions_trashed", { n: num(o.counts.trashed_versions) })
        : t("stat.across_saves", { saves: num(o.counts.saves), games: num(o.counts.games) })),
    stat(t("stat.deduped"), ...bytesParts(saved > 0 ? saved : 0),
      t("stat.deduped_sub", { logical: bytes(o.storage.logical_bytes) })),
    stat(t("stat.devices"), num(o.counts.devices), "",
      o.counts.devices_online > 0
        ? t("stat.devices_online", { n: num(o.counts.devices_online) })
        : t("stat.devices_idle")));

  const days = o.playtime.days;
  const peak = Math.max(1, ...days.map((d) => d.secs));
  clear($("playtime-strip"), days.map((d) => h("i", {
    class: d.secs === 0 ? "zero" : "",
    title: d.day + " · " + hours(d.secs),
    css: { "--h": Math.max(2, Math.round((d.secs / peak) * 54)) + "px" },
  })));
  $("playtime-caption").textContent = o.playtime.total_secs > 0
    ? t("summary.playtime_caption", { total: hours(o.playtime.total_secs), days: o.playtime.window_days })
    : t("summary.playtime_empty");

  clear($("summary-games"), o.games.length ? gamesTable(o.games) : empty(t("summary.no_games")));
  renderActivity($("summary-activity"), o.recent || [], 8);
}

function empty(text) {
  return h("p", { class: "empty", text });
}

function gamesTable(games) {
  return h("div", { class: "scroll" },
    h("table", { class: "grid" },
      h("thead", {}, h("tr", {},
        h("th", { class: "grow", text: t("col.game") }),
        h("th", { class: "num", text: t("col.saves") }),
        h("th", { class: "num", text: t("col.versions") }),
        h("th", { class: "num", text: t("col.size") }),
        h("th", { class: "num", text: t("col.playtime") }),
        h("th", { class: "num", text: t("col.last_backup") }))),
      h("tbody", {}, games.map((g) => h("tr", {},
        h("td", { class: "grow" },
          h("div", { class: "name", text: g.display_name }),
          g.display_name !== g.slug ? h("div", { class: "sub mono", text: g.slug }) : null),
        h("td", { class: "num", text: num(g.saves) }),
        h("td", { class: "num", text: num(g.versions) }),
        h("td", { class: "num", text: bytes(g.bytes) }),
        h("td", { class: "num", text: g.playtime_secs ? hours(g.playtime_secs) : "—" }),
        h("td", { class: "num sub", text: g.last_backup_at ? ago(g.last_backup_at) : "—" }))))));
}

// ---------------------------------------------------------------------------
// activity
// ---------------------------------------------------------------------------

const EVENT_KEYS = {
  "snapshot.created": "event.created",
  "snapshot.deleted": "event.deleted",
  "snapshot.restored": "event.restored",
  "snapshot.pruned": "event.pruned",
};

function renderActivity(host, rows, limit) {
  if (!rows.length) return clear(host, empty(t("activity.empty")));
  const shown = limit ? rows.slice(0, limit) : rows;
  clear(host, h("div", { class: "scroll" },
    h("table", { class: "grid" },
      h("tbody", {}, shown.map((r) => h("tr", {},
        h("td", { class: "sub", text: when(r.at) }),
        h("td", {}, t(EVENT_KEYS[r.event] || "event.other", { event: r.event })),
        h("td", { class: "grow" },
          r.game_slug ? h("span", { class: "name", text: r.game_slug }) : h("span", { class: "sub", text: t("activity.gone") }),
          r.label && r.label !== "default" ? h("span", { class: "sub", text: " · " + r.label }) : null),
        h("td", { class: "num sub", text: r.version_num !== null && r.version_num !== undefined ? "v" + r.version_num : "" }),
        h("td", { class: "num sub", text: r.device_name || "" }),
        h("td", { class: "num", text: r.bytes ? bytes(r.bytes) : "" }),
        h("td", {
          class: "num sub",
          title: r.new_bytes !== null && r.new_bytes !== undefined ? t("activity.new_bytes_title") : null,
          text: r.new_bytes !== null && r.new_bytes !== undefined ? "+" + bytes(r.new_bytes) : "",
        })))))));
}

// ---------------------------------------------------------------------------
// saves
// ---------------------------------------------------------------------------

async function renderSaves() {
  const host = $("saves-list");
  clear(host, empty(t("common.loading")));
  const saves = await api("/v1/saves");
  if (!saves.length) return clear(host, empty(t("saves.empty")));

  const byGame = new Map();
  for (const s of saves) {
    if (!byGame.has(s.game_slug)) byGame.set(s.game_slug, []);
    byGame.get(s.game_slug).push(s);
  }

  clear(host, [...byGame.entries()].map(([slug, rows]) => {
    const display = (overview.games.find((g) => g.slug === slug) || {}).display_name || slug;
    return rows.map((s) => {
      const body = h("div", { class: "save-body" }, empty(t("common.loading")));
      const meta = h("span", { class: "save-meta", text: t("saves.meta", {
        versions: num(s.snapshot_count ?? 0),
        size: bytes(s.total_size_bytes ?? 0),
      }) });
      const node = h("details", { class: "save" },
        h("summary", {},
          h("span", { class: "save-name", text: display }),
          s.label && s.label !== "default" ? h("span", { class: "sub", text: s.label }) : null,
          meta),
        body);
      let loaded = false;
      node.addEventListener("toggle", async () => {
        if (!node.open || loaded) return;
        loaded = true;
        if (!(await loadVersions(s, body, meta))) loaded = false;
      });
      return node;
    });
  }));
}

/// Fetch (or re-fetch) one save's versions and repaint its body. Everything
/// that mutates a version calls this instead of poking at the DOM row it just
/// changed: recovering v3 puts a row back in the middle of the table, and the
/// summary line has to agree with what the table now shows.
async function loadVersions(save, body, meta) {
  try {
    const snaps = await api("/v1/saves/" + save.id + "/snapshots?include_deleted=true&limit=200");
    clear(body, versionsTable(save, snaps, () => loadVersions(save, body, meta)));
    const live = snaps.filter((s) => !s.deleted_at);
    meta.textContent = t("saves.meta", {
      versions: num(live.length),
      size: bytes(live.reduce((sum, s) => sum + (s.total_size_bytes || 0), 0)),
    });
    return true;
  } catch (e) {
    clear(body, empty(errorText(e)));
    return false;
  }
}

function versionsTable(save, snaps, reload) {
  if (!snaps.length) return empty(t("saves.no_versions"));
  const rows = snaps.slice().sort((a, b) => b.version_num - a.version_num);
  return h("div", { class: "scroll" },
    h("table", { class: "grid" },
      h("thead", {}, h("tr", {},
        h("th", { class: "grow", text: t("col.version") }),
        h("th", { text: t("col.when") }),
        h("th", { text: t("col.device") }),
        h("th", { class: "num", text: t("col.files") }),
        h("th", { class: "num", text: t("col.size") }),
        h("th", {}))),
      h("tbody", {}, rows.map((s) => h("tr", { class: s.deleted_at ? "gone" : null },
        h("td", { class: "mono grow" }, "v" + s.version_num,
          s.is_pinned ? h("span", { class: "badge", text: t("saves.pinned") }) : null,
          s.deleted_at ? h("span", { class: "badge muted", text: t("saves.trashed") }) : null),
        h("td", { class: "sub", title: when(s.created_at), text: ago(s.created_at) }),
        h("td", { class: "sub", text: s.device_name || "—" }),
        h("td", { class: "num", text: num(s.file_count) }),
        h("td", { class: "num", text: bytes(s.total_size_bytes) }),
        h("td", {}, h("div", { class: "row-actions" },
          h("a", {
            class: "link",
            href: "/v1/saves/" + save.id + "/snapshots/" + s.version_num + "/download",
            download: "",
            title: t("saves.download_title"),
            text: t("common.download"),
          }),
          s.deleted_at
            ? h("button", {
                class: "link",
                type: "button",
                text: t("common.recover"),
                onclick: () => recoverVersion(save, s, reload),
              })
            : h("button", {
                class: "link warn",
                type: "button",
                text: t("common.delete"),
                onclick: () => deleteVersion(save, s, reload),
              }))))))));
}

async function deleteVersion(save, snap, reload) {
  const ok = await confirmDialog(
    t("saves.delete_title", { version: snap.version_num }),
    t("saves.delete_body", { days: overview.server.trash_retention_days }),
    t("common.delete"));
  if (!ok) return;
  try {
    await api("/v1/saves/" + save.id + "/snapshots/" + snap.version_num, { method: "DELETE" });
    toast(t("saves.deleted", { version: snap.version_num }));
    await reload();
    overview = await api("/v1/me/overview");
  } catch (e) {
    toast(errorText(e), true);
  }
}

/// The other half of delete. Nothing confirms here: putting a version back is
/// additive, and the trash is only useful if getting out of it is one click.
async function recoverVersion(save, snap, reload) {
  try {
    await api("/v1/saves/" + save.id + "/snapshots/" + snap.version_num + "/restore",
      { method: "POST" });
    toast(t("saves.recovered", { version: snap.version_num }));
    await reload();
    overview = await api("/v1/me/overview");
  } catch (e) {
    toast(errorText(e), true);
  }
}

/** Two calls on purpose: the dry run is what turns "set a cap of 5" into "set a
 *  cap of 5 and throw 112 versions in the trash", which is a different
 *  question and deserves to be asked before the fact. */
async function applyLimits() {
  const parse = (el) => {
    const raw = el.value.trim();
    if (raw === "") return null;
    const n = parseInt(raw, 10);
    return Number.isFinite(n) && n > 0 ? n : null;
  };
  const jobs = [
    { manual: false, value: parse($("f-max-versions")), current: overview.server.max_versions },
    { manual: true, value: parse($("f-max-manual")), current: overview.server.max_manual_versions },
  ].filter((j) => (j.value ?? null) !== (j.current ?? null));

  if (!jobs.length) return toast(t("saves.limits_unchanged"));

  try {
    let toPrune = 0;
    for (const j of jobs) {
      const dry = await api("/v1/me/max-versions", {
        method: "PUT",
        body: JSON.stringify({ max_versions: j.value, manual: j.manual, dry_run: true }),
      });
      toPrune += dry.pruned || 0;
    }
    if (toPrune > 0) {
      const ok = await confirmDialog(t("saves.limits_title"),
        t("saves.limits_body", { n: num(toPrune), days: overview.server.trash_retention_days }),
        t("common.apply"));
      if (!ok) return;
    }
    for (const j of jobs) {
      await api("/v1/me/max-versions", {
        method: "PUT",
        body: JSON.stringify({ max_versions: j.value, manual: j.manual }),
      });
    }
    toast(toPrune > 0 ? t("saves.limits_done_pruned", { n: num(toPrune) }) : t("saves.limits_done"));
    await refresh();
  } catch (e) {
    toast(errorText(e), true);
  }
}

// ---------------------------------------------------------------------------
// devices
// ---------------------------------------------------------------------------

async function renderDevices() {
  const host = $("devices-list");
  clear(host, empty(t("common.loading")));
  const { devices } = await api("/v1/devices");
  if (!devices.length) return clear(host, empty(t("devices.empty")));

  clear(host, h("div", { class: "scroll" },
    h("table", { class: "grid" },
      h("thead", {}, h("tr", {},
        h("th", { class: "grow", text: t("col.device") }),
        h("th", { text: t("col.system") }),
        h("th", { text: t("col.playing") }),
        h("th", { class: "num", text: t("col.last_seen") }),
        h("th", {}))),
      h("tbody", {}, devices.map((d) => {
        const tr = h("tr", {},
          h("td", { class: "grow" },
            h("span", { class: "dot" + (d.online ? " on" : "") }),
            h("span", { class: "name", text: d.device_name }),
            d.this_device ? h("span", { class: "badge", text: t("devices.this_one") }) : null),
          h("td", { class: "sub", text: [d.os, d.device_kind].filter(Boolean).join(" · ") || "—" }),
          h("td", { class: "sub", text: d.playing && d.playing.length ? d.playing.map((p) => p.slug).join(", ") : "—" }),
          h("td", { class: "num sub", title: when(d.last_seen_at), text: d.online ? t("devices.now") : ago(d.last_seen_at) }),
          h("td", {}, h("div", { class: "row-actions" },
            h("button", {
              class: "link warn",
              type: "button",
              text: t("common.remove"),
              onclick: () => removeDevice(d, tr),
            }))));
        return tr;
      })))));
}

async function removeDevice(device, tr) {
  const ok = await confirmDialog(t("devices.remove_title"),
    t("devices.remove_body", { name: device.device_name }), t("common.remove"));
  if (!ok) return;
  try {
    await api("/v1/devices/" + device.id, { method: "DELETE" });
    tr.remove();
    toast(t("devices.removed", { name: device.device_name }));
  } catch (e) {
    toast(errorText(e), true);
  }
}

// ---------------------------------------------------------------------------
// admin: server
// ---------------------------------------------------------------------------

async function loadAdmin() {
  adminData = await api("/v1/admin/overview");
  return adminData;
}

async function renderServer() {
  const a = adminData || (await loadAdmin());
  const s = overview.server;
  const tot = a.totals;

  clear($("server-stats"),
    stat(t("stat.stored"), ...bytesParts(tot.stored_bytes),
      t("server.objects", { n: num(tot.objects) })),
    stat(t("stat.versions"), num(tot.versions), "",
      t("server.across_users", { users: num(tot.users), saves: num(tot.saves) })),
    stat(t("server.trash"), ...bytesParts(tot.trash_bytes),
      t("server.trash_sub", { n: num(tot.trashed_versions), days: s.trash_retention_days })),
    stat(t("server.uptime"), uptime(s.uptime_secs), "", "v" + s.version));

  clear($("server-storage"), h("div", { class: "scroll" },
    h("table", { class: "grid" }, h("tbody", {},
      infoRow(t("server.backend"), s.storage_backend === "s3" ? t("server.backend_s3") : t("server.backend_local")),
      infoRow(t("server.logical"), bytes(tot.logical_bytes), t("server.logical_sub")),
      infoRow(t("server.physical"), bytes(tot.stored_bytes),
        t("server.physical_sub", { pct: Math.round(100 - pct(tot.stored_bytes, tot.logical_bytes || 1)) })),
      infoRow(t("server.orphans"), num(tot.orphan_objects) + " · " + bytes(tot.orphan_bytes), t("server.orphans_sub")),
      infoRow(t("server.database"), tot.db_bytes === null ? "—" : bytes(tot.db_bytes),
        t("server.database_sub", { logs: num(tot.client_logs) })),
      infoRow(t("server.oldest"), tot.oldest_snapshot_at ? when(tot.oldest_snapshot_at) : "—")))));

  clear($("server-policy"), h("div", { class: "scroll" },
    h("table", { class: "grid" }, h("tbody", {},
      infoRow(t("server.pruning"), s.snapshot_pruning ? t("common.on") : t("common.off"),
        s.snapshot_pruning ? t("server.pruning_sub", { k: s.data_saving }) : t("server.pruning_off_sub")),
      infoRow(t("server.trash_days"), t("server.days", { n: s.trash_retention_days })),
      infoRow(t("server.max_upload"), bytes(s.max_snapshot_size_mb * 1024 * 1024), t("server.max_upload_sub"))))));
}

function infoRow(label, value, sub) {
  return h("tr", {},
    h("td", { class: "grow wrap" },
      h("div", { text: label }),
      sub ? h("div", { class: "sub", text: sub }) : null),
    h("td", { class: "num", text: value }));
}

function uptime(secs) {
  const d = Math.floor(secs / 86400);
  const hrs = Math.floor((secs % 86400) / 3600);
  if (d > 0) return t("time.days_hours", { d, h: hrs });
  const m = Math.floor((secs % 3600) / 60);
  return hrs > 0 ? t("time.hours_min", { h: hrs, m }) : t("time.minutes", { n: m });
}

// ---------------------------------------------------------------------------
// admin: users and tokens
// ---------------------------------------------------------------------------

async function renderUsers() {
  const a = await loadAdmin();
  const head = ["col.user", "col.used", "col.quota", "col.saves", "col.versions",
                "col.devices", "col.last_seen"];
  clear($("users-list"), h("div", { class: "scroll" },
    h("table", { class: "grid" },
      h("thead", {}, h("tr", {},
        head.map((k, i) => h("th", { class: i === 0 ? "grow" : "num", text: t(k) })),
        h("th", {}))),
      h("tbody", {}, a.users.map(userRow)))));

  await renderTokens();
}

function userRow(u) {
  const used = pct(u.used_bytes, u.quota_bytes);
  return h("tr", {},
    h("td", { class: "grow" },
      h("span", { class: "name", text: u.username }),
      u.is_admin ? h("span", { class: "badge", text: t("common.admin") }) : null,
      h("div", { class: "sub", text: t("users.since", { date: when(u.created_at) }) })),
    h("td", { class: "num" },
      bytes(u.used_bytes),
      h("div", { class: "meter" + (used > 90 ? " over" : "") },
        h("i", { css: { "--pct": used.toFixed(1) + "%" } }))),
    h("td", { class: "num", text: bytes(u.quota_bytes) }),
    h("td", { class: "num", text: num(u.saves) }),
    h("td", { class: "num", text: num(u.versions) }),
    h("td", { class: "num", text: num(u.devices) }),
    h("td", { class: "num sub", text: u.last_seen_at ? ago(u.last_seen_at) : "—" }),
    h("td", {}, h("div", { class: "row-actions" },
      h("button", {
        class: "link",
        type: "button",
        text: t("users.quota"),
        onclick: () => editQuota(u),
      }),
      h("button", {
        class: "link",
        type: "button",
        text: u.is_admin ? t("users.demote") : t("users.promote"),
        onclick: () => toggleAdmin(u),
      }),
      h("button", {
        class: "link",
        type: "button",
        text: t("users.rename"),
        onclick: () => renameUser(u),
      }),
      h("button", {
        class: "link",
        type: "button",
        text: t("users.password"),
        onclick: () => setPassword(u),
      }),
      // Deleting yourself is the one the server refuses outright, so the row
      // does not offer it rather than explaining the refusal afterwards.
      u.username === me.username ? null : h("button", {
        class: "link warn",
        type: "button",
        text: t("common.delete"),
        onclick: () => deleteUser(u),
      }))));
}

// Collect values from a dialog built out of `fields`, or null if cancelled.
// The same modal `editQuota` fills in by hand, generalized: every admin action
// added here would otherwise repeat the showModal/close/returnValue dance.
async function formDialog(title, fields, okLabel) {
  const dlg = $("dlg-confirm");
  const inputs = fields.map((f) => h("input", {
    type: f.type || "text",
    value: f.value || "",
    placeholder: f.placeholder || "",
    autocomplete: f.type === "password" ? "new-password" : "off",
    onkeydown: (e) => {
      if (e.key === "Enter") { e.preventDefault(); $("confirm-ok").click(); }
    },
  }));
  $("confirm-title").textContent = title;
  clear($("confirm-body"),
    fields.map((f, i) => h("label", { class: "field" },
      h("span", { text: f.label }), inputs[i])),
    fields.some((f) => f.hint)
      ? h("p", { class: "sub", text: fields.find((f) => f.hint).hint })
      : null);
  $("confirm-ok").textContent = okLabel || t("common.save");

  const ok = await new Promise((resolve) => {
    dlg.addEventListener("close", () => resolve(dlg.returnValue === "ok"), { once: true });
    dlg.showModal();
    inputs[0]?.focus();
  });
  if (!ok) return null;
  return inputs.map((i) => i.value);
}

async function newUser() {
  const vals = await formDialog(t("users.new_title"), [
    { label: t("users.username_label"), placeholder: "player-two" },
    { label: t("users.password_label"), type: "password", hint: t("users.password_hint") },
  ], t("users.create"));
  if (!vals) return;
  const [username, password] = vals;
  try {
    const created = await api("/v1/admin/users", {
      method: "POST",
      body: JSON.stringify({ username: username.trim(), password, is_admin: false }),
    });
    toast(t("users.created", { user: created.username }));
    await renderUsers();
    // The account is useless until a PC can reach it, and the operator is
    // already here, offer the token instead of making them find the button.
    await newToken(created.id);
  } catch (e) {
    toast(errorText(e), true);
  }
}

async function renameUser(user) {
  const vals = await formDialog(
    t("users.rename_title", { user: user.username }),
    [{ label: t("users.username_label"), value: user.username, hint: t("users.rename_hint") }],
    t("users.rename"));
  if (!vals) return;
  const username = vals[0].trim();
  if (!username || username === user.username) return;
  try {
    await api("/v1/admin/users/" + user.id, {
      method: "PATCH",
      body: JSON.stringify({ username }),
    });
    toast(t("users.renamed", { from: user.username, to: username }));
    await renderUsers();
    // The header still greets you by the old name otherwise.
    if (user.username === me.username) location.reload();
  } catch (e) {
    toast(errorText(e), true);
  }
}

async function setPassword(user) {
  const vals = await formDialog(
    t("users.password_title", { user: user.username }),
    [{ label: t("users.password_label"), type: "password", hint: t("users.password_sessions") }],
    t("common.save"));
  if (!vals) return;
  const password = vals[0];
  if (!password) return;
  try {
    await api("/v1/admin/users/" + user.id, {
      method: "PATCH",
      body: JSON.stringify({ password }),
    });
    toast(t("users.password_done", { user: user.username }));
    // Setting your own password revokes your own session along with it.
    if (user.username === me.username) return location.reload();
    await renderUsers();
  } catch (e) {
    toast(errorText(e), true);
  }
}

async function deleteUser(user) {
  // Typing the name, not clicking OK: this is the only button in the panel
  // that destroys saves, and the row above it holds how much is about to go.
  const vals = await formDialog(
    t("users.delete_title", { user: user.username }),
    [{
      label: t("users.delete_label", { user: user.username }),
      placeholder: user.username,
      hint: t("users.delete_hint", {
        user: user.username,
        size: bytes(user.used_bytes),
        versions: num(user.versions),
      }),
    }],
    t("common.delete"));
  if (!vals) return;
  if (vals[0].trim() !== user.username) return toast(t("users.delete_mismatch"), true);
  try {
    const gone = await api("/v1/admin/users/" + user.id, { method: "DELETE" });
    toast(t("users.deleted", { user: gone.username, size: bytes(gone.bytes_removed) }));
    await renderUsers();
  } catch (e) {
    toast(errorText(e), true);
  }
}

async function editQuota(user) {
  const dlg = $("dlg-confirm");
  const current = Math.round((user.quota_bytes / (1024 ** 3)) * 100) / 100;
  const input = h("input", {
    type: "number", min: "0", step: "0.5", value: String(current),
    onkeydown: (e) => {
      if (e.key === "Enter") { e.preventDefault(); $("confirm-ok").click(); }
    },
  });
  $("confirm-title").textContent = t("users.quota_title", { user: user.username });
  clear($("confirm-body"),
    h("label", { class: "field" }, h("span", { text: t("users.quota_label") }), input));
  $("confirm-ok").textContent = t("common.save");
  const ok = await new Promise((resolve) => {
    dlg.addEventListener("close", () => resolve(dlg.returnValue === "ok"), { once: true });
    dlg.showModal();
  });
  if (!ok) return;
  const gib = parseFloat(input.value);
  if (!Number.isFinite(gib) || gib < 0) return toast(t("error.bad_quota"), true);
  try {
    await api("/v1/admin/users/" + user.id, {
      method: "PATCH",
      body: JSON.stringify({ storage_quota_bytes: Math.round(gib * 1024 ** 3) }),
    });
    toast(t("users.quota_done", { user: user.username, quota: bytes(gib * 1024 ** 3) }));
    await renderUsers();
  } catch (e) {
    toast(errorText(e), true);
  }
}

async function toggleAdmin(user) {
  const promoting = !user.is_admin;
  const ok = await confirmDialog(
    promoting ? t("users.promote_title") : t("users.demote_title"),
    promoting ? t("users.promote_body", { user: user.username }) : t("users.demote_body", { user: user.username }),
    promoting ? t("users.promote") : t("users.demote"));
  if (!ok) return;
  try {
    await api("/v1/admin/users/" + user.id, {
      method: "PATCH",
      body: JSON.stringify({ is_admin: promoting }),
    });
    toast(promoting ? t("users.promoted", { user: user.username }) : t("users.demoted", { user: user.username }));
    await renderUsers();
    if (user.username === me.username && !promoting) location.reload();
  } catch (e) {
    toast(errorText(e), true);
  }
}

// Mint a device token. `presetUserId` skips the picker, for the hand-off
// straight after creating an account.
async function newToken(presetUserId) {
  const users = (adminData || (await loadAdmin())).users;
  const dlg = $("dlg-confirm");
  const picker = h("select", {},
    users.map((u) => h("option", { value: u.id, selected: u.id === presetUserId }, u.username)));
  const device = h("input", {
    type: "text",
    placeholder: t("users.device_placeholder"),
    onkeydown: (e) => {
      if (e.key === "Enter") { e.preventDefault(); $("confirm-ok").click(); }
    },
  });

  $("confirm-title").textContent = t("users.token_title");
  clear($("confirm-body"),
    presetUserId ? null : h("label", { class: "field" },
      h("span", { text: t("col.user") }), picker),
    h("label", { class: "field" }, h("span", { text: t("users.device_label") }), device),
    h("p", { class: "sub", text: t("users.token_hint") }));
  $("confirm-ok").textContent = t("users.token_create");

  const ok = await new Promise((resolve) => {
    dlg.addEventListener("close", () => resolve(dlg.returnValue === "ok"), { once: true });
    dlg.showModal();
    device.focus();
  });
  if (!ok) return;

  try {
    const minted = await api("/v1/admin/tokens", {
      method: "POST",
      body: JSON.stringify({
        user_id: presetUserId || picker.value,
        device_name: device.value.trim() || null,
      }),
    });
    await renderTokens();
    await showToken(minted);
  } catch (e) {
    toast(errorText(e), true);
  }
}

// The token in the clear, once. Only its SHA-256 is stored, so closing this
// without copying means minting another one, which is why the dialog says so
// and the only button is "done".
async function showToken(minted) {
  const dlg = $("dlg-confirm");
  const field = h("input", { type: "text", readonly: true, value: minted.token, class: "token" });
  const copy = h("button", {
    class: "ghost",
    type: "button",
    text: t("common.copy"),
    onclick: async () => {
      // navigator.clipboard is undefined on a plain-HTTP origin, which is
      // exactly how a NAS panel is reached. Select the text so Ctrl+C works.
      try {
        await navigator.clipboard.writeText(minted.token);
        toast(t("users.token_copied"));
      } catch {
        field.select();
        toast(t("users.token_select"), true);
      }
    },
  });

  $("confirm-title").textContent = t("users.token_ready");
  clear($("confirm-body"),
    h("p", { class: "sub", text: t("users.token_once", { user: minted.username }) }),
    h("div", { class: "token-row" }, field, copy));
  $("confirm-ok").textContent = t("common.done");
  await new Promise((resolve) => {
    dlg.addEventListener("close", resolve, { once: true });
    dlg.showModal();
    field.select();
  });
}

async function renderTokens() {
  const tokens = await api("/v1/admin/tokens");
  const host = $("tokens-list");
  if (!tokens.length) return clear(host, empty(t("users.tokens_empty")));
  clear(host, h("div", { class: "scroll" },
    h("table", { class: "grid" },
      h("thead", {}, h("tr", {},
        h("th", { text: t("col.user") }),
        h("th", { class: "grow", text: t("col.device") }),
        h("th", { class: "num", text: t("col.created") }),
        h("th", { class: "num", text: t("col.last_used") }),
        h("th", { class: "num", text: t("col.expires") }),
        h("th", {}))),
      h("tbody", {}, tokens.map((tok) => {
        const tr = h("tr", {},
          h("td", { class: "name", text: tok.username }),
          h("td", { class: "grow" },
            tok.is_session ? t("users.browser_session") : (tok.device_name || t("users.unnamed_token")),
            tok.is_session ? h("span", { class: "badge", text: t("users.session") }) : null),
          h("td", { class: "num sub", text: ago(tok.created_at) }),
          h("td", { class: "num sub", text: tok.last_used_at ? ago(tok.last_used_at) : t("users.never") }),
          h("td", { class: "num sub", text: tok.expires_at ? ago(tok.expires_at) : t("users.no_expiry") }),
          h("td", {}, h("div", { class: "row-actions" },
            h("button", {
              class: "link warn",
              type: "button",
              text: t("users.revoke"),
              onclick: () => revokeToken(tok, tr),
            }))));
        return tr;
      })))));
}

async function revokeToken(tok, tr) {
  const ok = await confirmDialog(t("users.revoke_title"),
    tok.is_session
      ? t("users.revoke_body_session", { user: tok.username })
      : t("users.revoke_body_device", { user: tok.username, device: tok.device_name || "" }),
    t("users.revoke"));
  if (!ok) return;
  try {
    await api("/v1/admin/tokens/" + tok.id + "/revoke", { method: "POST" });
    tr.remove();
    toast(t("users.revoked"));
  } catch (e) {
    toast(errorText(e), true);
  }
}

// ---------------------------------------------------------------------------
// admin: client logs
// ---------------------------------------------------------------------------

const LEVELS = ["", "error", "warn", "info", "debug", "trace"];

function fillLevelFilter() {
  const sel = $("f-log-level");
  clear(sel, LEVELS.map((lv) => h("option", { value: lv, text: lv === "" ? t("logs.all_levels") : lv })));
}

async function renderLogs() {
  const host = $("logs-list");
  clear(host, empty(t("common.loading")));
  const params = new URLSearchParams();
  const level = $("f-log-level").value;
  const q = $("f-log-search").value.trim();
  if (level) params.set("level", level);
  if (q) params.set("q", q);
  const rows = await api("/v1/admin/logs" + (params.toString() ? "?" + params : ""));
  if (!rows.length) return clear(host, empty(t("logs.empty")));

  clear(host, h("div", { class: "scroll" },
    h("table", { class: "grid" },
      h("thead", {}, h("tr", {},
        h("th", { text: t("col.when") }),
        h("th", { text: t("col.level") }),
        h("th", { text: t("col.user") }),
        h("th", { text: t("col.device") }),
        h("th", { class: "grow", text: t("col.message") }))),
      h("tbody", {}, rows.map((r) => h("tr", {},
        h("td", { class: "sub", text: when(r.at) }),
        h("td", {}, h("span", { class: "lvl " + (r.level === "error" ? "error" : r.level === "warn" ? "warn" : ""), text: r.level })),
        h("td", { class: "sub", text: r.username }),
        h("td", { class: "sub" },
          r.device_name || "—",
          r.app_version ? h("div", { class: "sub mono", text: "v" + r.app_version }) : null),
        h("td", { class: "grow wrap" },
          h("div", { text: r.message }),
          r.target ? h("div", { class: "sub mono", text: r.target }) : null)))))));
}

// ---------------------------------------------------------------------------
// password change
// ---------------------------------------------------------------------------

/// Always open empty. Leaving whatever was there, a failed attempt, or the
/// browser's autofill, is how you end up typing into a field that already had
/// content and getting "wrong password" with no idea why.
function openPasswordDialog() {
  for (const id of ["f-current", "f-new"]) {
    const input = $(id);
    input.value = "";
    input.type = "password";
  }
  for (const btn of document.querySelectorAll("#dlg-password .reveal")) {
    btn.querySelector("use").setAttribute("href", "#eye");
  }
  $("password-error").hidden = true;
  $("dlg-password").showModal();
  $("f-current").focus();
}

async function changePassword() {
  const err = $("password-error");
  err.hidden = true;
  try {
    await api("/v1/auth/password", {
      method: "POST",
      body: JSON.stringify({
        current_password: $("f-current").value,
        new_password: $("f-new").value,
      }),
    });
    $("dlg-password").close();
    $("f-current").value = "";
    $("f-new").value = "";
    toast(t("account.password_done"));
  } catch (e) {
    // "Wrong user or password" is the login's wording and it is wrong here:
    // this dialog has no username, and the only thing that can be wrong is the
    // current password.
    err.textContent = e instanceof ApiError && e.code === "invalid_credentials"
      ? t("account.wrong_current")
      : errorText(e);
    err.hidden = false;
    const current = $("f-current");
    current.focus();
    current.select();
  }
}

// ---------------------------------------------------------------------------
// tabs and boot
// ---------------------------------------------------------------------------

const TABS = {
  summary: renderSummary,
  saves: renderSaves,
  devices: renderDevices,
  activity: async () => renderActivity($("activity-list"), await api("/v1/me/activity?limit=200")),
  server: renderServer,
  users: renderUsers,
  logs: renderLogs,
};

let current = "summary";

async function show(tab) {
  if (!TABS[tab]) tab = "summary";
  if ((tab === "server" || tab === "users" || tab === "logs") && !me.is_admin) tab = "summary";
  current = tab;
  location.hash = "#" + tab;
  for (const btn of document.querySelectorAll(".tab")) {
    const on = btn.dataset.tab === tab;
    btn.toggleAttribute("aria-current", on);
    if (on) btn.setAttribute("aria-current", "page");
  }
  for (const sec of document.querySelectorAll(".panel")) {
    sec.hidden = sec.id !== "panel-" + tab;
  }
  try {
    await TABS[tab]();
  } catch (e) {
    if (!(e instanceof ApiError && e.status === 401)) toast(errorText(e), true);
  }
}

async function refresh() {
  overview = await api("/v1/me/overview");
  // The account view and the recent-activity block come from two endpoints but
  // read as one screen, so they are fetched together and the summary never
  // renders half-populated.
  overview.recent = await api("/v1/me/activity?limit=8");
  $("server-version").textContent = "v" + overview.server.version;
  $("f-max-versions").value = overview.server.max_versions ?? "";
  $("f-max-manual").value = overview.server.max_manual_versions ?? "";
  $("foot-note").textContent = t("foot.note", {
    user: me.username,
    backend: overview.server.storage_backend,
  });
  adminData = null;
  await show(current);
}

/// Listen to the push the server has had since 1.1.2 (`GET /v1/events`).
///
/// The note that used to sit here said a browser could not consume it, because
/// `EventSource` cannot send an Authorization header. That stopped being true
/// the moment the panel started authenticating with a cookie: same-origin
/// EventSource sends it, reconnects on its own, and costs one idle connection.
///
/// A "save" event means some machine just pushed a version. The panel does not
/// try to patch the row in place, it re-runs whatever tab you are on, coalesced
/// so a device uploading five saves in a row repaints once.
function listen() {
  let timer = null;
  const source = new EventSource("/v1/events");
  const repaint = () => {
    clearTimeout(timer);
    timer = setTimeout(async () => {
      if (!me) return;
      try {
        overview = await api("/v1/me/overview");
        overview.recent = await api("/v1/me/activity?limit=8");
        await TABS[current]();
      } catch { /* a dead session already sent us to the gate */ }
    }, 1500);
  };
  source.addEventListener("save", repaint);
  // "lagged" means the server dropped events because we fell behind; the
  // answer is the same full repaint, which is what this panel does anyway.
  source.addEventListener("lagged", repaint);
  return source;
}

/// Wire every show/hide button to its field.
///
/// It is not a nicety: the browser autofills `current-password` fields, and a
/// field that already held something you could not see is the likeliest reason
/// a password change kept coming back wrong.
function wireReveals() {
  for (const btn of document.querySelectorAll(".reveal")) {
    btn.addEventListener("click", () => {
      const input = $(btn.dataset.reveal);
      const showing = input.type === "text";
      input.type = showing ? "password" : "text";
      btn.querySelector("use").setAttribute("href", showing ? "#eye" : "#eye-off");
      btn.setAttribute("aria-label", t(showing ? "a11y.show_password" : "a11y.hide_password"));
      input.focus();
    });
  }
}

function wire() {
  $("login-form").addEventListener("submit", submitLogin);
  $("token-form").addEventListener("submit", submitToken);
  $("toggle-mode").addEventListener("click", toggleMode);
  $("btn-logout").addEventListener("click", logout);
  $("btn-refresh").addEventListener("click", refresh);
  $("btn-limits").addEventListener("click", applyLimits);
  $("btn-logs").addEventListener("click", () => renderLogs().catch((e) => toast(errorText(e), true)));
  $("btn-new-user").addEventListener("click", () => newUser());
  $("btn-new-token").addEventListener("click", () => newToken());
  $("f-log-search").addEventListener("keydown", (e) => {
    if (e.key === "Enter") { e.preventDefault(); renderLogs().catch(() => {}); }
  });
  $("f-log-level").addEventListener("change", () => renderLogs().catch(() => {}));
  $("btn-password").addEventListener("click", openPasswordDialog);
  $("password-cancel").addEventListener("click", () => $("dlg-password").close());
  // The form owns the submit, so Enter in either field saves. It used to be a
  // `method="dialog"` form, where Enter closed the dialog and changed nothing.
  $("password-form").addEventListener("submit", (e) => {
    e.preventDefault();
    changePassword();
  });
  for (const btn of document.querySelectorAll(".tab")) {
    btn.addEventListener("click", () => show(btn.dataset.tab));
  }
  window.addEventListener("hashchange", () => {
    const tab = location.hash.replace(/^#/, "");
    if (me && tab && tab !== current) show(tab);
  });
}

function fillLocalePickers() {
  for (const id of ["gate-locale", "app-locale"]) {
    const sel = $(id);
    clear(sel, LOCALES.map(([code, label]) => h("option", { value: code, text: label })));
    sel.value = locale;
    sel.addEventListener("change", async () => {
      await setLocale(sel.value);
      fillLevelFilter();
      $("toggle-mode").textContent = $("token-form").hidden ? t("gate.use_token") : t("gate.use_password");
      for (const other of ["gate-locale", "app-locale"]) $(other).value = locale;
      if (me) await refresh();
    });
  }
}

async function boot() {
  await setLocale(pickLocale());
  fillLocalePickers();
  fillLevelFilter();
  wire();
  wireReveals();
  current = location.hash.replace(/^#/, "") || "summary";

  // A live cookie means no login screen at all. Anything else, expired,
  // revoked, never had one, lands on the gate without an error message: a
  // first visit is not a failure.
  try {
    const who = await api("/v1/auth/whoami");
    await enter({ username: who.username, is_admin: who.is_admin });
  } catch {
    showGate(null);
  }
}

boot();
