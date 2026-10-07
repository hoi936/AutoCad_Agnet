import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { AgentRun, ChangeSet, RunMode, RunSnapshot } from "./contracts";
import { api, errorMessage, native } from "./ipc";
import { ProviderSettings } from "./ProviderSettings";
import { mergeRunEvents, runStatus } from "./run-events";

export function ChatPanel({ projectId, mode, locked, onModeChange, onPreview, onBusyChange }: {
  projectId: string;
  mode: RunMode;
  locked: boolean;
  onModeChange: (mode: RunMode) => void;
  onPreview: (change: ChangeSet | null) => void;
  onBusyChange: (busy: boolean) => void;
}) {
  const client = useQueryClient();
  const [selectedConversation, selectConversation] = useState("");
  const [newRun, setNewRun] = useState<AgentRun | null>(null);
  const [content, setContent] = useState("");
  const [starting, setStarting] = useState(false);
  const [settingsBusy, setSettingsBusy] = useState(false);
  const [stopRequested, setStopRequested] = useState(false);
  const [error, setError] = useState("");
  const observedPreview = useRef("");
  const observedStatus = useRef("");
  const conversations = useQuery({
    queryKey: ["conversations", projectId],
    queryFn: () => api.conversations(projectId),
    enabled: native && !!projectId, retry: false,
  });
  const conversationId = selectedConversation || conversations.data?.[0]?.id || "";
  const conversation = useQuery({
    queryKey: ["conversation", conversationId],
    queryFn: () => api.conversation(conversationId),
    enabled: native && !!conversationId, retry: false,
  });
  const provider = useQuery({
    queryKey: ["provider", projectId],
    queryFn: () => api.provider(projectId),
    enabled: native && !!projectId, retry: false,
  });
  const previousRun = conversation.data?.runs.reduce<AgentRun | undefined>(
    (latest, run) => !latest || run.started_at >= latest.started_at ? run : latest, undefined,
  );
  const currentRun = newRun ?? previousRun;
  const runId = currentRun?.id ?? "";
  const snapshot = useQuery({
    queryKey: ["run", runId],
    queryFn: () => api.run(runId),
    enabled: native && !!runId, retry: false,
    refetchOnReconnect: "always", refetchOnWindowFocus: "always",
    refetchInterval: (query) => query.state.data?.run.status === "running" ||
      (!query.state.data && currentRun?.status === "running") ? 500 : false,
    structuralSharing: (oldData, newData) => {
      const next = newData as RunSnapshot;
      const previous = oldData as RunSnapshot | undefined;
      return { ...next, events: mergeRunEvents(previous?.events ?? [], next.events, next.run.id) };
    },
  });
  const run = snapshot.data?.run ?? currentRun;
  const running = run?.status === "running";
  const busy = starting || running || settingsBusy;
  const config = provider.data;
  const providerReady = !!config && (config.provider === "mock" ||
    (config.configured && config.cloud_consent && !!config.model.trim()));
  const events = snapshot.data?.events ?? [];
  const stream = events.filter((event) => event.kind === "text.delta").map((event) => event.text ?? "").join("");
  const persistedResponse = conversation.data?.messages.some((message) =>
    message.run_id === runId && message.role === "assistant");

  useEffect(() => {
    onBusyChange(busy);
    return () => onBusyChange(false);
  }, [busy, onBusyChange]);
  useEffect(() => {
    if (starting || !newRun || snapshot.data?.run.id !== newRun.id || snapshot.data.run.conversation_id !== conversationId) return;
    const preview = snapshot.data?.change_sets.slice().reverse().find((change) =>
      ["pending_approval", "planned"].includes(change.status));
    if (preview && observedPreview.current !== preview.id) {
      observedPreview.current = preview.id;
      onPreview(preview);
      void client.invalidateQueries({ queryKey: ["changes", projectId] });
    }
  }, [snapshot.data, onPreview, client, projectId, starting, newRun, conversationId]);
  useEffect(() => {
    if (settingsBusy) onPreview(null);
  }, [settingsBusy, onPreview]);
  useEffect(() => {
    const statusKey = `${run?.id}:${run?.status}`;
    if (run && !running && observedStatus.current !== statusKey) {
      observedStatus.current = statusKey;
      void client.invalidateQueries({ queryKey: ["conversation", conversationId] });
      void client.invalidateQueries({ queryKey: ["context", projectId] });
    }
  }, [run, running, conversationId, projectId, client]);

  async function newConversation() {
    onPreview(null);
    setStarting(true);
    setError("");
    try {
      const created = await api.createConversation(projectId, "Hội thoại mới");
      selectConversation(created.id);
      setNewRun(null);
      setStopRequested(false);
      await client.invalidateQueries({ queryKey: ["conversations", projectId] });
    } catch (failure) { setError(errorMessage(failure)); }
    finally { setStarting(false); }
  }
  async function send() {
    if (busy || locked || !content.trim() || !providerReady) return;
    onPreview(null);
    setStarting(true);
    setError("");
    setStopRequested(false);
    try {
      let target = conversationId;
      if (!target) {
        const created = await api.createConversation(projectId, content.trim().slice(0, 80));
        target = created.id;
        selectConversation(target);
        await client.invalidateQueries({ queryKey: ["conversations", projectId] });
      }
      const started = await api.startRun(target, content.trim(), mode);
      setNewRun(started);
      setContent("");
      await client.invalidateQueries({ queryKey: ["conversation", target] });
    } catch (failure) { setError(errorMessage(failure)); }
    finally { setStarting(false); }
  }
  async function stop() {
    setStopRequested(true);
    setError("");
    try {
      await api.stopRun(runId);
      await snapshot.refetch();
    } catch (failure) {
      setStopRequested(false);
      setError(errorMessage(failure));
    }
  }

  return <section className="chat-panel" aria-label="Hội thoại agent">
    <div className="form-heading">
      <h2>HỘI THOẠI AGENT</h2>
      <span className="chip">{config?.provider === "openai" ? "LLM live · OpenAI" : "LLM mock"} · CAD mock</span>
    </div>
    <div className="actions conversation-actions">
      <select aria-label="Hội thoại" value={conversationId} disabled={busy || locked || !conversations.data?.length}
        onChange={(event) => {
          onPreview(null);
          selectConversation(event.target.value);
          setNewRun(null);
          setError("");
          setStopRequested(false);
        }}>
        {!conversations.data?.length && <option value="">Chưa có hội thoại</option>}
        {conversations.data?.map((item) => <option key={item.id} value={item.id}>{item.title}</option>)}
      </select>
      <button disabled={!native || !projectId || busy || locked} onClick={() => void newConversation()}>＋ Hội thoại mới</button>
    </div>
    {config?.provider !== "openai" ? <p className="mock-hint">
      MockProvider chỉ chạy fixture tạo tấm đế 200 × 120 mm, Ø10, mép 20 mm và resize lên 240 mm; không phải LLM tổng quát, không dùng Internet.
    </p> : <p className="mock-hint">
      OpenAI · {config.model || "chưa chọn model"} · {config.cloud_consent ? "Đã cho phép gửi dữ liệu cloud của dự án" : "Chưa cho phép gửi dữ liệu cloud"}
    </p>}
    {run && <p className="muted">Nguồn chat · Hội thoại {conversationId} · Run {run.id}</p>}
    {config && <ProviderSettings key={`${projectId}:${config.provider}:${config.model}:${config.storage}:${config.cloud_consent}`}
      config={config} disabled={starting || running || locked} onBusyChange={setSettingsBusy} />}
    <div className="messages" aria-label="Lịch sử tin nhắn" aria-live="polite" aria-busy={running}>
      {!conversation.data?.messages.length && <p className="muted">{projectId ? "Nhập yêu cầu để bắt đầu. Hội thoại được lưu cục bộ theo dự án." : "Chọn hoặc tạo dự án để mở hội thoại."}</p>}
      {conversation.data?.messages.map((message) => <article key={message.id} className={`message ${message.role === "user" ? "user" : "assistant"}`}>
        <strong>{message.role === "user" ? "Bạn" : message.role === "assistant" ? "Agent" : "Hệ thống"}</strong>
        <p>{message.content}</p>
      </article>)}
      {stream && !persistedResponse && <article className="message assistant"><strong>Agent</strong><p>{stream}</p></article>}
    </div>
    {events.some((event) => event.kind.startsWith("tool.")) && <details className="tool-activity" open={running}>
      <summary>Hoạt động công cụ</summary>
      <ol>{events.filter((event) => event.kind.startsWith("tool.")).map((event) => <li key={`${event.run_id}:${event.sequence}`}>
        {event.tool_name ?? "Công cụ"} · {event.kind === "tool.started" ? "đang xử lý" : event.kind === "tool.failed" ? "thất bại" : "hoàn tất"}{event.text ? ` — ${event.text}` : ""}
      </li>)}</ol>
    </details>}
    <p role="status" className="run-status">{starting ? "Đang gửi yêu cầu…" : runStatus(run?.status)}{running && stopRequested ? " · Đã yêu cầu dừng; đang chờ backend xác nhận." : ""}</p>
    {(error || conversations.error || conversation.error || provider.error || snapshot.error) && <div role="alert" className="error">
      <p>{error || errorMessage(conversations.error || conversation.error || provider.error || snapshot.error)}</p>
      {snapshot.error && <><p>Mất cập nhật trạng thái. Không tự gửi lại yêu cầu hoặc áp dụng thay đổi.</p><button onClick={() => void snapshot.refetch()}>Lấy lại trạng thái</button></>}
    </div>}
    {run?.status === "error" && <p role="alert" className="error">{events.slice().reverse().find((event) => event.kind === "run.error")?.text ?? "Provider xử lý thất bại."}{run.error_code ? ` (${run.error_code})` : ""}</p>}
    <form onSubmit={(event) => { event.preventDefault(); void send(); }}>
      <label htmlFor="chat-message">Yêu cầu thiết kế</label>
      <textarea id="chat-message" rows={3} value={content} maxLength={16000}
        disabled={!native || !projectId || busy || locked} onChange={(event) => setContent(event.target.value)}
        placeholder="Vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm cách mép 20 mm."
        onKeyDown={(event) => {
          if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) { event.preventDefault(); void send(); }
        }} />
      <div className="actions">
        <select aria-label="Chế độ hội thoại" value={mode} disabled={busy || locked} onChange={(event) => onModeChange(event.target.value as RunMode)}>
          <option value="ask">Ask — chỉ đọc</option>
          <option value="plan">Plan — chỉ phương án</option>
          <option value="execute">Execute — cần duyệt</option>
        </select>
        <button type="button" disabled={!running || stopRequested} onClick={() => void stop()}>Dừng</button>
        <button className="primary" disabled={!native || !projectId || busy || locked || !content.trim() || !providerReady}>Gửi yêu cầu</button>
      </div>
      <p className="muted">Ctrl/⌘ + Enter để gửi · Preview phải được duyệt ở panel hình học. Dừng không hoàn tác thay đổi đã gửi.</p>
      {config?.provider === "openai" && !providerReady && <p className="muted">Cần model ID, credential và quyền gửi dữ liệu theo dự án trước khi dùng OpenAI.</p>}
    </form>
  </section>;
}
