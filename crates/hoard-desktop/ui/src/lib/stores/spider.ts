/**
 * The Halloween spider's knobs. The debug panel moves them live, so the feel
 * can be tuned on the real window instead of by editing and reloading;
 * everyone else only ever sees the defaults.
 */
import { writable } from "svelte/store";

export type SpiderTune = {
  /** Width in px; the height follows the drawing's proportions. */
  size: number;
  /** px per second along a thread. */
  speed: number;
  /** Seconds sitting still between two walks. */
  restMin: number;
  restMax: number;
  /** Crossings walked in one go, at most. */
  maxSteps: number;
  /** The web's radius in px, at most; a short window gets less. */
  webMax: number;
  /** Degrees each leg swings either way while walking. */
  legSwing: number;
  /** ms per leg swing, so the gait's tempo. */
  beat: number;
  /** Degrees it leans towards where it is going. */
  lean: number;
  /** How far the pupils travel when following the pointer. */
  lookReach: number;
  /** Seconds between two idle gestures (blink, foot, hat, looking round). */
  idleMin: number;
  idleMax: number;
};

export const SPIDER_DEFAULTS: SpiderTune = {
  size: 44,
  speed: 26,
  restMin: 2.5,
  restMax: 8,
  maxSteps: 4,
  webMax: 220,
  legSwing: 5,
  beat: 140,
  lean: 6,
  lookReach: 1.1,
  idleMin: 2.2,
  idleMax: 5,
};

export const spiderTune = writable<SpiderTune>({ ...SPIDER_DEFAULTS });

/** Bumped by the debug panel's "walk now", so a change of speed can be seen
 *  without waiting out the rest. */
export const spiderWalkNow = writable(0);

/** Bumped by the debug panel's "new garland": the title bar mounts the garland
 *  again, which rolls a fresh one, as starting the app does. */
export const garlandRoll = writable(0);
