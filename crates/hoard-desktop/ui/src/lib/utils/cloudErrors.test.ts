import { beforeAll, describe, expect, it } from "vitest";
import { addMessages, init } from "svelte-i18n";

import { cloudErrorCode, formatCloudError } from "./cloudErrors";

const GB = 1024 ** 3;

describe("cloudErrorCode", () => {
  it("finds the code behind the HTTP status Tauri prefixes", () => {
    expect(cloudErrorCode('413 Payload Too Large: {"code":"save_too_large"}')).toBe(
      "save_too_large",
    );
  });

  it("returns null for JSON without a code and for a body cut short", () => {
    expect(cloudErrorCode('{"error":"nope"}')).toBeNull();
    expect(cloudErrorCode('429 {"code":"bandwidth_limit_exc')).toBeNull();
  });
});

describe("formatCloudError", () => {
  // Templates that only echo their values, so these pin what the helper hands
  // the translator rather than the English copy.
  beforeAll(async () => {
    addMessages("en", {
      "cloud_error.save_too_large": "{requested} over {limit}",
      "cloud_error.bandwidth_exceeded": "{limit} per {window}, retry in {retry}",
      "cloud_error.retry_seconds": "{count}s",
      "cloud_error.retry_minutes": "{count}min",
      "cloud_error.generic": "generic",
      "help.err_empty": "empty",
    });
    await init({ fallbackLocale: "en", initialLocale: "en" });
  });

  it("drops the decimal from sizes of 10 units and up", () => {
    const body = JSON.stringify({
      code: "save_too_large",
      requested_bytes: 25 * GB,
      limit_bytes: 1 * GB,
    });
    expect(formatCloudError(body)).toBe("25 GB over 1.0 GB");
  });

  it("rounds the retry up to whole minutes from one minute on", () => {
    const bandwidth = (extra: object) =>
      formatCloudError(JSON.stringify({ code: "bandwidth_limit_exceeded", ...extra }));
    expect(
      bandwidth({ retry_after_seconds: 30, limit_bytes: 5 * GB, window_secs: 3600 }),
    ).toBe("5.0 GB per 60min, retry in 30s");
    expect(bandwidth({ retry_after_seconds: 61 })).toContain("retry in 2min");
    // What the server leaves out falls back to a 15-minute window and a
    // one-minute retry, not to "NaN".
    expect(bandwidth({})).toBe("0 B per 15min, retry in 1min");
  });

  it("translates the i18n: keys the Rust side sends back", () => {
    expect(formatCloudError("i18n:help.err_empty")).toBe("empty");
  });

  it("passes plain messages through and falls back to the generic text", () => {
    expect(formatCloudError(new Error("connection refused"))).toBe("connection refused");
    expect(formatCloudError(undefined)).toBe("generic");
  });
});
