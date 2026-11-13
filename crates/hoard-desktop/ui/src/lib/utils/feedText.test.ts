import { describe, expect, it } from "vitest";

import type { FeedEntry } from "../stores/live";
import { feedSummary, type Translate } from "./feedText";

/** Runs `feedSummary` against a stand-in for `$_` and returns the key it picked
 *  with the values it passed, which is the part this module decides. */
function summarise(entry: Partial<FeedEntry> & Pick<FeedEntry, "kind">) {
  let seen: { key: string; values?: Record<string, unknown> } | undefined;
  const t = ((key: string, opts?: { values?: Record<string, unknown> }) => {
    seen = { key, values: opts?.values };
    return key;
  }) as unknown as Translate;
  feedSummary({ id: 1, at: 0, ...entry }, t);
  return seen;
}

describe("feedSummary", () => {
  it("names a row without a slug after a short save id, then a dash", () => {
    expect(summarise({ kind: "game_started", save_id: "0123456789abcdef" })?.values).toEqual({
      name: "01234567",
    });
    expect(summarise({ kind: "game_started" })?.values).toEqual({ name: "—" });
  });

  describe("backup_too_large", () => {
    const row = { kind: "backup_too_large", game_slug: "elden-ring" } as const;

    it("blames the user's own server when it set the limit", () => {
      expect(
        summarise({ ...row, too_large_kind: "server_limit", bytes: 0, limit_bytes: 512 * 1024 ** 2 }),
      ).toEqual({
        key: "library.backup_too_large_server_toast",
        values: { name: "elden-ring", limit: "512.0 MB" },
      });
    });

    it("blames the proxy, with no sizes it cannot know", () => {
      expect(summarise({ ...row, too_large_kind: "proxy" })).toEqual({
        key: "library.backup_too_large_proxy_toast",
        values: { name: "elden-ring" },
      });
    });

    it("reads a row without a kind as the plan cap, with both sizes", () => {
      expect(summarise({ ...row, bytes: 3 * 1024 ** 3, limit_bytes: 1024 ** 3 })).toEqual({
        key: "activity.backup_too_large",
        values: { name: "elden-ring", size: "3.0 GB", limit: "1.0 GB" },
      });
    });

    it("drops to the generic sentence rather than print a size of 0 B", () => {
      expect(summarise({ ...row, too_large_kind: "plan_cap", limit_bytes: 1024 ** 3 })?.key).toBe(
        "library.backup_too_large_generic_toast",
      );
    });
  });
});
