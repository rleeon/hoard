import { beforeAll, describe, expect, it } from "vitest";
import { get } from "svelte/store";
import { addMessages, init } from "svelte-i18n";

import { fmtRelativeTime, formatBytes, intlLocale, prettifySlug, titleFromSlug } from "./format";

describe("formatBytes", () => {
  it("collapses empty and negative input to 0 B", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(-1)).toBe("0 B");
  });

  it("prints whole bytes without decimals and switches unit at 1024", () => {
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1024)).toBe("1.0 kB");
  });

  it("moves up a unit when rounding reaches it", () => {
    expect(formatBytes(1048575)).toBe("1.0 MB");
    expect(formatBytes(1023.6)).toBe("1.0 kB");
    expect(formatBytes(1024 ** 3 - 1)).toBe("1.0 GB");
    expect(formatBytes(1048575, 0)).toBe("1 MB");
    expect(formatBytes(1048575, 2)).toBe("1.00 MB");
  });

  it("leaves values that do not round up alone", () => {
    expect(formatBytes(1024 * 1023.94)).toBe("1023.9 kB");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1024 ** 2)).toBe("1.0 MB");
    expect(formatBytes(1024 ** 6 - 1)).toBe("1024.0 PB");
  });


  it("honors the decimals argument", () => {
    expect(formatBytes(1.5 * 1024 ** 3, 2)).toBe("1.50 GB");
  });

  it("stays in B below one byte", () => {
    expect(formatBytes(0.5)).toBe("1 B");
    expect(formatBytes(0.4)).toBe("0 B");
    expect(formatBytes(0.001)).toBe("0 B");
  });


  it("stays on PB past the last unit", () => {
    expect(formatBytes(1024 ** 6)).toBe("1024.0 PB");
  });
});

describe("intlLocale", () => {
  it("maps any English, and no language at all, to en-GB", () => {
    expect(intlLocale("en-US")).toBe("en-GB");
    expect(intlLocale(null)).toBe("en-GB");
  });

  it("keeps only the primary subtag, lowercased, for everything else", () => {
    expect(intlLocale("PT-br")).toBe("pt");
  });
});

describe("slug titles", () => {
  it("title-case words split on runs of dashes and underscores", () => {
    expect(titleFromSlug("hollow_knight--silksong")).toBe("Hollow Knight Silksong");
  });

  it("differ only on a slug with no words in it", () => {
    // prettifySlug feeds a visible game name, so it falls back to the slug
    // rather than leave the card blank.
    expect(prettifySlug("--")).toBe("--");
    expect(titleFromSlug("--")).toBe("");
  });
});

describe("fmtRelativeTime", () => {
  // Echo the count instead of loading en.json, so these pin the thresholds and
  // the numbers handed to the translator, not the English copy.
  beforeAll(async () => {
    addMessages("en", {
      "history.relative_just_now": "now",
      "history.relative_minutes": "{count}m",
      "history.relative_hours": "{count}h",
      "dashboard.time_yesterday": "yesterday",
      "history.relative_days": "{count}d",
    });
    await init({ fallbackLocale: "en", initialLocale: "en" });
  });

  const now = Date.UTC(2026, 6, 21, 17, 47);
  const ago = (secs: number) =>
    get(fmtRelativeTime)(new Date(now - secs * 1000).toISOString(), now);

  it("floors into each step at its boundary", () => {
    expect(ago(59)).toBe("now");
    expect(ago(60)).toBe("1m");
    expect(ago(86399)).toBe("23h");
    expect(ago(86400)).toBe("yesterday");
    expect(ago(172800)).toBe("2d");
  });

  it("treats a timestamp in the future (clock skew) as just now", () => {
    expect(ago(-300)).toBe("now");
  });
});
