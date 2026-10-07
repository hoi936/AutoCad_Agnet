import type { RunEvent } from "./contracts";

// Snapshot polling may overlap a reconnect. Keep each persisted event exactly once.
export function mergeRunEvents(
  previous: RunEvent[],
  incoming: RunEvent[],
  runId: string,
): RunEvent[] {
  const bySequence = new Map<number, RunEvent>();
  for (const event of [...previous, ...incoming]) {
    if (event.run_id === runId && Number.isSafeInteger(event.sequence)) {
      bySequence.set(event.sequence, event);
    }
  }
  return [...bySequence.values()].sort((a, b) => a.sequence - b.sequence);
}

export function runStatus(status?: string): string {
  const labels: Record<string, string> = {
    running: "Đang xử lý",
    completed: "Đã hoàn tất",
    awaiting_approval: "Đang chờ duyệt preview",
    planned: "Đã lập phương án — không được áp dụng",
    cancelled: "Đã dừng",
    error: "Xử lý thất bại",
    interrupted: "Phiên xử lý bị gián đoạn; kiểm tra trạng thái trước khi tiếp tục",
    rejected: "Đã từ chối phương án",
    outcome_unknown: "Chưa xác định kết quả; đối chiếu trong lịch sử phương án",
    verified: "Đã áp dụng và xác minh",
  };
  return status ? labels[status] ?? status : "Sẵn sàng";
}
