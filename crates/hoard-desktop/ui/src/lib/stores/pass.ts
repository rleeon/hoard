// The one-year pass as the app sees it: where in its life it is and which
// reminder is due. The server sends the dates (`subscription_interval`,
// `renews_at`, `pro_until`); the days are the ones the emails go out on
// (`PASS_REMINDERS` in hoard-server's notify.rs), so the app and the inbox
// speak up at the same moments and a dismissed card comes back with the next
// email, not the next day.
import { derived, writable } from "svelte/store";
import { cloud } from "./cloud";

const DAY = 24 * 60 * 60 * 1000;
const GRACE_DAYS = 30;
const REMINDERS = [-25, -14, -7, -1, 3, 10, 17, 24];
const DISMISS_KEY = "hoard:pass-dismissed";

export type PassState = {
  /** `ending`: the year runs out within 25 days. `grace`: the year is over and
   *  the account keeps Pro until `proUntil`. */
  phase: "ending" | "grace";
  yearEnds: string;
  proUntil: string;
  /** To `yearEnds` while ending, to `proUntil` in grace. Never below 1. */
  daysLeft: number;
  /** Which reminder is due, and with the end date, what a dismissal hides. */
  key: string;
};

export function passState(
  account: {
    subscription_interval?: string | null;
    renews_at: string | null;
    pro_until?: string | null;
  } | null,
  now: number,
): PassState | null {
  if (account?.subscription_interval !== "once" || !account.renews_at) return null;
  const end = new Date(account.renews_at).getTime();
  if (Number.isNaN(end)) return null;
  const until = account.pro_until
    ? new Date(account.pro_until).getTime()
    : end + GRACE_DAYS * DAY;
  if (now >= until) return null;
  const due = REMINDERS.filter((d) => end + d * DAY <= now);
  if (due.length === 0) return null;
  const grace = now >= end;
  return {
    phase: grace ? "grace" : "ending",
    yearEnds: account.renews_at,
    proUntil: new Date(until).toISOString(),
    daysLeft: Math.max(1, Math.ceil(((grace ? until : end) - now) / DAY)),
    key: `${account.renews_at}:${due[due.length - 1]}`,
  };
}

function readDismissed(): string | null {
  try {
    return localStorage.getItem(DISMISS_KEY);
  } catch {
    return null;
  }
}

const dismissed = writable<string | null>(readDismissed());

export const pass = derived(cloud, ($c) => passState($c.account, Date.now()));

/** The card, unless this reminder was already put away. */
export const passReminder = derived([pass, dismissed], ([$p, $d]) =>
  $p && $p.key !== $d ? $p : null,
);

export function dismissPassReminder(key: string): void {
  dismissed.set(key);
  try {
    localStorage.setItem(DISMISS_KEY, key);
  } catch {
    // A webview without storage just shows the card again next start.
  }
}
