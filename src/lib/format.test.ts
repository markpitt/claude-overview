import { describe, expect, it } from "vitest";
import { componentStatusLabel, duration, incidentStatusLabel, levelLabel, relativeTime } from "./format";

describe("relativeTime", () => {
  const now = Date.parse("2026-11-23T12:00:00Z");
  it("handles never, seconds, minutes, hours and days", () => {
    expect(relativeTime(0, now)).toBe("never");
    expect(relativeTime(now - 10_000, now)).toBe("just now");
    expect(relativeTime(now - 4 * 60_000, now)).toBe("4 min ago");
    expect(relativeTime(now - 3 * 3_600_000, now)).toBe("3 h ago");
    expect(relativeTime(now - 48 * 3_600_000, now)).toBe("2 days ago");
  });
});

describe("labels", () => {
  it("names component states plainly", () => {
    expect(componentStatusLabel("partial_outage")).toBe("Partial outage");
    expect(componentStatusLabel("something_new")).toBe("something new");
  });
  it("capitalises incident states", () => {
    expect(incidentStatusLabel("investigating")).toBe("Investigating");
  });
  it("names levels", () => {
    expect(levelLabel("bad")).toBe("Outage");
    expect(levelLabel("unknown")).toBe("Unknown");
  });
});

describe("duration", () => {
  it("formats minutes and hours", () => {
    expect(duration("2026-11-23T09:49:00Z", "2026-11-23T11:07:00Z")).toBe("1 h 18 min");
    expect(duration("2026-11-23T09:00:00Z", "2026-11-23T09:20:00Z")).toBe("20 min");
    expect(duration("2026-11-23T09:00:00Z", "2026-11-23T11:00:00Z")).toBe("2 h");
  });
});
