import { describe, expect, it } from "vitest";
import type { RunEvent } from "./contracts";
import { mergeRunEvents } from "./run-events";

function event(sequence: number, text: string, runId = "run-a"): RunEvent {
  return { run_id: runId, sequence, kind: "text.delta", text, tool_name: null,
    change_set_id: null, created_at: 1 };
}
describe("run event reconnect", () => {
  it("orders snapshots, ignores other runs and does not render repeated chunks twice", () => {
    const previous = [event(2, "B"), event(1, "A")];
    const reconnected = mergeRunEvents(previous,
      [event(3, "C"), event(2, "B"), event(1, "A"), event(1, "foreign", "run-b")], "run-a");
    expect(reconnected.map((item) => item.sequence)).toEqual([1, 2, 3]);
    expect(reconnected.map((item) => item.text).join("")).toBe("ABC");
    expect(mergeRunEvents(reconnected, reconnected, "run-a")).toEqual(reconnected);
  });
});
