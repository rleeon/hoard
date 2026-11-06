// Only the pure helpers. `invoke` is imported by the module but never called
// here, so nothing of Tauri's needs to exist.
import { describe, expect, it } from "vitest";

import { FULL, coverFits, coverKey, coverSize } from "./covers";

describe("coverKey", () => {
  it("prefers the slug, so a custom cover follows the game to every screen", () => {
    expect(coverKey(1245620, "elden-ring")).toBe("slug-elden-ring");
    expect(coverKey(1245620, null)).toBe("1245620");
    expect(coverKey(null, "")).toBeNull();
  });
});

describe("coverSize", () => {
  it("rounds up to the next step Rust keeps, and past the last one asks for full", () => {
    expect(coverSize(96)).toBe(96);
    expect(coverSize(97)).toBe(192);
    expect(coverSize(769)).toBe(FULL);
  });
});

describe("coverFits", () => {
  it("accepts the full art for any frame, never a smaller step", () => {
    expect(coverFits(192, 384)).toBe(false);
    expect(coverFits(FULL, 768)).toBe(true);
    expect(coverFits(768, FULL)).toBe(false);
  });
});
