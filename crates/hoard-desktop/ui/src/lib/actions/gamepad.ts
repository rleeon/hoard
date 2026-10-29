/**
 * A controller, turned into what the arrow navigation already understands
 * (`spatialNav.ts`): the d-pad and the left stick are the arrows, the bottom
 * face button is Enter, the right one Escape, and the bumpers change page. A
 * direction held down repeats, as a key does.
 *
 * On Linux the controller comes from `src/gamepad.rs`: Ubuntu's WebKitGTK has
 * no Gamepad API. WebView2 and macOS's engine have it, and there it is read
 * here.
 *
 * Steam may already be turning the same controller into keys (the Deck, or a
 * pad under Steam's desktop layout), and then every press would land twice. A
 * press here waits a moment and is dropped when the same key came in around
 * it. That is also how the hint knows what is in the user's hands (`navInput`).
 */

import { listen } from "@tauri-apps/api/event";
import { writable } from "svelte/store";

import { osTag } from "../os";
import { navSwitchPage } from "./spatialNav";

export type PadFamily = "xbox" | "playstation" | "nintendo" | "steam";
type PadButton = "up" | "down" | "left" | "right" | "a" | "b" | "lb" | "rb";

/** What is driving the app: `pad` a controller read here, `steam` a
 *  controller whose presses arrive as keys, `keys` the keyboard (a controller
 *  is around, but this was not it), `both` no way to tell. */
export type NavInput = { kind: "both" | "keys" | "pad" | "steam"; family: PadFamily };
export const navInput = writable<NavInput>({ kind: "both", family: "steam" });

const KEY: Record<PadButton, string> = {
  up: "ArrowUp",
  down: "ArrowDown",
  left: "ArrowLeft",
  right: "ArrowRight",
  a: "Enter",
  b: "Escape",
  lb: "q",
  rb: "e",
};
const KEYS = new Set(Object.values(KEY));
const DIRECTIONS = new Set<PadButton>(["up", "down", "left", "right"]);

/** How long a press waits for Steam's key, and how far apart the two can be. */
const WAIT_MS = 60;
const NEAR_MS = 150;
const FIRST_REPEAT_MS = 400;
const REPEAT_MS = 110;

let padSeen = false;
let family: PadFamily = "steam";
/** Real keys and controller presses of the last second. */
const keysIn: { key: string; at: number }[] = [];
const padsIn: { key: string; at: number; family: PadFamily }[] = [];
const down = new Set<string>();
const repeats = new Map<string, number>();
const lastPress = new Map<PadButton, { pad: number; at: number }>();

function recent<T extends { at: number }>(list: T[], now: number): T[] {
  while (list.length > 0 && now - list[0].at > 1000) list.shift();
  return list;
}

function onKey(e: KeyboardEvent): void {
  if (!e.isTrusted || e.repeat) return;
  const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;
  if (!KEYS.has(key)) return;
  const now = performance.now();
  recent(keysIn, now).push({ key, at: now });
  const pad = padsIn.find((p) => p.key === key && now - p.at < NEAR_MS);
  if (pad) navInput.set({ kind: "steam", family: pad.family });
  else navInput.set({ kind: padSeen ? "keys" : "both", family });
}

function fire(button: PadButton): void {
  if (!document.hasFocus()) return;
  if (button === "lb" || button === "rb") {
    navSwitchPage(button === "rb" ? 1 : -1);
    return;
  }
  const target = document.activeElement instanceof HTMLElement ? document.activeElement : document.body;
  target.dispatchEvent(new KeyboardEvent("keydown", { key: KEY[button], bubbles: true, cancelable: true }));
}

function stop(id: string): void {
  clearTimeout(repeats.get(id));
  repeats.delete(id);
}

function press(pad: number, button: PadButton, from: PadFamily): void {
  padSeen = true;
  family = from;
  const now = performance.now();
  // The same press from a second device: Steam's virtual controller copies
  // the real one.
  const before = lastPress.get(button);
  if (before && before.pad !== pad && now - before.at < 40) return;
  lastPress.set(button, { pad, at: now });
  const key = KEY[button];
  recent(padsIn, now).push({ key, at: now, family: from });
  const id = `${pad}:${button}`;
  down.add(id);
  setTimeout(() => {
    if (recent(keysIn, performance.now()).some((k) => k.key === key && Math.abs(k.at - now) < NEAR_MS)) {
      // Steam sent it as a key already, and the key has done the work.
      navInput.set({ kind: "steam", family: from });
      return;
    }
    navInput.set({ kind: "pad", family: from });
    fire(button);
    if (!DIRECTIONS.has(button) || !down.has(id)) return;
    stop(id);
    const again = () => {
      fire(button);
      repeats.set(id, window.setTimeout(again, REPEAT_MS));
    };
    repeats.set(id, window.setTimeout(again, FIRST_REPEAT_MS));
  }, WAIT_MS);
}

function release(pad: number, button: PadButton): void {
  const id = `${pad}:${button}`;
  down.delete(id);
  stop(id);
}

function familyOf(id: string): PadFamily {
  if (/054c|playstation|dualsense|dualshock/i.test(id)) return "playstation";
  if (/057e|nintendo|pro controller/i.test(id)) return "nintendo";
  if (/28de|steam/i.test(id)) return "steam";
  return "xbox";
}

/** The engine's own Gamepad API, polled once a frame while a controller is
 *  connected. Standard layout: 0 and 1 the bottom and right face buttons, 4
 *  and 5 the bumpers, 12 to 15 the d-pad. */
function readEngine(): void {
  const STANDARD: [PadButton, number][] = [
    ["a", 0],
    ["b", 1],
    ["lb", 4],
    ["rb", 5],
    ["up", 12],
    ["down", 13],
    ["left", 14],
    ["right", 15],
  ];
  const held = new Map<number, Set<PadButton>>();
  let running = false;
  const frame = () => {
    let any = false;
    const pads = navigator.getGamepads().filter((gp) => gp?.connected);
    // Unplugged, or out of battery, with a direction held: let go of it, or
    // it went on repeating with nothing left to release it.
    for (const [index, was] of held) {
      if (pads.some((gp) => gp?.index === index)) continue;
      for (const b of was) release(index, b);
      held.delete(index);
    }
    for (const gp of pads) {
      if (!gp) continue;
      any = true;
      const was = held.get(gp.index) ?? new Set<PadButton>();
      const now = new Set<PadButton>();
      for (const [button, i] of STANDARD) {
        if (gp.mapping === "standard" || i < 2) {
          if (gp.buttons[i]?.pressed) now.add(button);
        }
      }
      // The stick leans: pressed past 0.6, let go back under 0.35.
      const [x = 0, y = 0] = gp.axes;
      const lean = (button: PadButton, v: number) => {
        if (v > (was.has(button) ? 0.35 : 0.6)) now.add(button);
      };
      lean("left", -x);
      lean("right", x);
      lean("up", -y);
      lean("down", y);
      for (const b of now) if (!was.has(b)) press(gp.index, b, familyOf(gp.id));
      for (const b of was) if (!now.has(b)) release(gp.index, b);
      held.set(gp.index, now);
    }
    if (any) requestAnimationFrame(frame);
    else running = false;
  };
  window.addEventListener("gamepadconnected", () => {
    if (running) return;
    running = true;
    requestAnimationFrame(frame);
  });
}

export function installGamepad(): void {
  window.addEventListener("keydown", onKey, true);
  window.addEventListener("blur", () => {
    for (const id of [...repeats.keys()]) stop(id);
    down.clear();
  });
  if (osTag() === "linux") {
    void listen<{ button: PadButton; pressed: boolean; family: PadFamily; pad: number }>(
      "gamepad",
      ({ payload: p }) => {
        if (p.pressed) press(p.pad, p.button, p.family);
        else release(p.pad, p.button);
      },
    ).catch(() => {});
  } else if (typeof navigator.getGamepads === "function") {
    readEngine();
  }
}
