// Product events from the interface. The list of names lives in
// `hoard_core::product`; the backend drops anything not on it, and the shipper
// sends nothing when diagnostics are switched off or the server is self-hosted.
import { invoke } from "@tauri-apps/api/core";
import { push } from "svelte-spa-router";

export type AppEvent =
  | "app.opened"
  | "pricing.opened"
  | "pricing.continue"
  | "checkout.opened"
  | "checkout.failed"
  | "upsell.shown"
  | "restore.manual";

type Props = Record<string, string | number | boolean>;

/** Fire-and-forget. A lost event is a gap in a chart, never an error. */
export function track(name: AppEvent, props: Props = {}): void {
  invoke("track_event", { name, props }).catch(() => {});
}

/** Every way into the pricing page. The 30 checkouts of 03-13 oct that nobody
 *  paid could not say where the click came from; this is where that is said. */
export type PricingFrom = "sidebar" | "account" | "storage" | "devices";

export function openPricing(from: PricingFrom): void {
  track("pricing.opened", { from });
  push("/pricing");
}
