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

describe("en-GB formats", () => {
  it("separates thousands", async () => {
    const { formatCount } = await import("./format");
    expect(formatCount(1360)).toBe("1,360");
  });
  it("writes day and month as 17/12", async () => {
    const { dayMonth } = await import("./format");
    expect(dayMonth(new Date(2026, 11, 17).getTime())).toBe("17/12");
  });
  it("uses a 24-hour clock", async () => {
    const { clockTime } = await import("./usage");
    expect(clockTime(new Date(2026, 11, 8, 14, 0).getTime())).toBe("14:00");
  });
});

describe("short dates", () => {
  it("has no comma after the weekday", async () => {
    const { shortDate } = await import("./format");
    expect(shortDate(Date.UTC(2026, 11, 8, 12), "UTC")).toBe("Tue 8 Dec");
  });
});

describe("display helpers", () => {
  it("shortens a home folder to ~", async () => {
    const { tildePath } = await import("./format");
    expect(tildePath("/home/mark/.claude/projects")).toBe("~/.claude/projects");
    expect(tildePath("C:\\Users\\Mark\\.claude\\projects")).toBe("~\\.claude\\projects");
    expect(tildePath("/srv/claude/projects")).toBe("/srv/claude/projects");
  });
  it("puts Statuspage headlines in sentence case", async () => {
    const { sentenceCase } = await import("./format");
    expect(sentenceCase("All Systems Operational")).toBe("All systems operational");
    expect(sentenceCase("Partial System Outage")).toBe("Partial system outage");
    expect(sentenceCase("Major System Outage")).toBe("Major system outage");
    expect(sentenceCase("Elevated errors on API")).toBe("Elevated errors on API");
  });
});
