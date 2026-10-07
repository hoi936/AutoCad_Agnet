import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { AgentRun, Conversation, ProviderConfig, RunSnapshot } from "./contracts";
import { ChatPanel } from "./ChatPanel";
import { ProviderSettings } from "./ProviderSettings";

const fake = vi.hoisted(() => ({
  conversations: vi.fn(), createConversation: vi.fn(), conversation: vi.fn(),
  provider: vi.fn(), startRun: vi.fn(), run: vi.fn(), stopRun: vi.fn(),
  saveProvider: vi.fn(), setSecret: vi.fn(), deleteSecret: vi.fn(),
}));
vi.mock("./ipc", () => ({
  native: true, api: fake,
  errorMessage: (error: unknown) => error instanceof Error ? error.message : "Lỗi backend",
}));
const conversation: Conversation = { id: "conversation-1", project_id: "project-1", title: "Thiết kế tấm đế", created_at: 1 };
const config: ProviderConfig = { project_id: "project-1", provider: "mock", model: "mock-plate-v1",
  secret_ref: null, cloud_consent: false, configured: false, storage: "session" };
const run: AgentRun = { id: "run-1", conversation_id: conversation.id, project_id: "project-1",
  mode: "execute", provider: "mock", model: "mock-plate-v1", status: "running",
  error_code: null, started_at: 2, finished_at: null };
const snapshot: RunSnapshot = { run, events: [], change_sets: [] };
const clients: QueryClient[] = [];
function wrap(child: React.ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  clients.push(client);
  return render(<QueryClientProvider client={client}>{child}</QueryClientProvider>);
}
function chat(onBusyChange = vi.fn()) {
  return wrap(<ChatPanel projectId="project-1" mode="execute" locked={false}
    onModeChange={vi.fn()} onPreview={vi.fn()} onBusyChange={onBusyChange} />);
}
beforeEach(() => {
  vi.resetAllMocks();
  fake.conversations.mockResolvedValue([conversation]);
  fake.conversation.mockResolvedValue({ conversation, messages: [], runs: [] });
  fake.provider.mockResolvedValue(config);
  fake.startRun.mockResolvedValue(run);
  fake.run.mockResolvedValue(snapshot);
  fake.stopRun.mockResolvedValue(undefined);
});
afterEach(() => {
  cleanup();
  clients.splice(0).forEach((client) => client.clear());
  vi.restoreAllMocks();
});

it("keeps the composer busy until cancellation is confirmed by backend snapshot", async () => {
  const onBusy = vi.fn();
  chat(onBusy);
  fireEvent.change(screen.getByLabelText("Yêu cầu thiết kế"), { target: { value: "Vẽ tấm đế" } });
  await waitFor(() => expect(screen.getByRole("button", { name: "Gửi yêu cầu" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Gửi yêu cầu" }));
  await waitFor(() => expect(fake.startRun).toHaveBeenCalledWith(conversation.id, "Vẽ tấm đế", "execute"));
  expect(screen.getByLabelText("Yêu cầu thiết kế")).toBeDisabled();
  await waitFor(() => expect(screen.getByRole("button", { name: "Dừng" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Dừng" }));
  await waitFor(() => expect(fake.stopRun).toHaveBeenCalledWith(run.id));
  expect(screen.getByText(/Đã yêu cầu dừng; đang chờ backend xác nhận/)).toBeInTheDocument();
  expect(screen.getByLabelText("Yêu cầu thiết kế")).toBeDisabled();
  fake.run.mockResolvedValue({ ...snapshot, run: { ...run, status: "cancelled", finished_at: 3 } });
  await waitFor(() => expect(screen.getByText("Đã dừng")).toBeInTheDocument(), { timeout: 2000 });
  expect(screen.getByLabelText("Yêu cầu thiết kế")).toBeEnabled();
  expect(onBusy).toHaveBeenLastCalledWith(false);
});

it("shows a provider failure and retains the unsent prompt without switching provider", async () => {
  fake.provider.mockResolvedValue({ ...config, provider: "openai", model: "account-model", configured: true, cloud_consent: true });
  fake.startRun.mockRejectedValue(new Error("OpenAI giới hạn tốc độ (429)."));
  chat();
  const input = screen.getByLabelText("Yêu cầu thiết kế");
  fireEvent.change(input, { target: { value: "Yêu cầu cần giữ lại" } });
  await waitFor(() => expect(screen.getByRole("button", { name: "Gửi yêu cầu" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Gửi yêu cầu" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("OpenAI giới hạn tốc độ (429).");
  expect(input).toHaveValue("Yêu cầu cần giữ lại");
  expect(input).toBeEnabled();
  expect(fake.saveProvider).not.toHaveBeenCalled();
  expect(screen.getByText(/LLM live · OpenAI/)).toBeInTheDocument();
});

it("reloads persisted conversation messages and recovers a run from its snapshot", async () => {
  fake.conversation.mockResolvedValue({ conversation, messages: [
    { id: "message-1", conversation_id: conversation.id, run_id: run.id, role: "user", content: "Yêu cầu từ lần mở trước", created_at: 1 },
  ], runs: [run] });
  fake.run.mockResolvedValue({ ...snapshot, run: { ...run, status: "completed" }, events: [
    { run_id: run.id, sequence: 2, kind: "text.delta", text: "xong", tool_name: null, change_set_id: null, created_at: 2 },
    { run_id: run.id, sequence: 1, kind: "text.delta", text: "Đã ", tool_name: null, change_set_id: null, created_at: 2 },
    { run_id: run.id, sequence: 2, kind: "text.delta", text: "xong", tool_name: null, change_set_id: null, created_at: 2 },
  ] });
  chat();
  expect(await screen.findByText("Yêu cầu từ lần mở trước")).toBeInTheDocument();
  expect(await screen.findByText("Đã xong")).toBeInTheDocument();
  expect(fake.run).toHaveBeenCalledWith(run.id);
  expect(screen.getAllByText("Đã xong")).toHaveLength(1);
});

it.each(["success", "failure"])("clears one-shot credential after vault %s without browser storage", async (outcome) => {
  const secretValue = "fixture-secret-never-persist";
  const storageSpy = vi.spyOn(Storage.prototype, "setItem");
  let complete: (() => void) | undefined;
  fake.setSecret.mockImplementation(() => new Promise<void>((resolve, reject) => {
    complete = () => outcome === "success" ? resolve() : reject(new Error("Vault không khả dụng."));
  }));
  wrap(<ProviderSettings config={{ ...config, provider: "openai", model: "account-model", storage: "windows", cloud_consent: true }}
    disabled={false} onBusyChange={vi.fn()} />);
  fireEvent.click(screen.getByText("Cài đặt provider và quyền dữ liệu"));
  const input = screen.getByLabelText("API key (gửi một lần)");
  expect(input).toHaveAttribute("type", "password");
  fireEvent.change(input, { target: { value: secretValue } });
  fireEvent.click(screen.getByRole("button", { name: "Lưu khóa" }));
  expect(fake.setSecret).toHaveBeenCalledExactlyOnceWith("project-1", secretValue);
  expect(input).toHaveValue("");
  await act(async () => complete?.());
  if (outcome === "failure") expect(await screen.findByRole("alert")).toHaveTextContent("Vault không khả dụng.");
  else expect(await screen.findByRole("status")).toHaveTextContent("Đã lưu khóa trong Windows Credential Manager.");
  expect(input).toHaveValue("");
  expect(document.body.textContent).not.toContain(secretValue);
  expect(storageSpy).not.toHaveBeenCalled();
});

it("clears a form or old-run preview before sending an unsupported new request", async () => {
  const preview = vi.fn();
  fake.startRun.mockRejectedValue(new Error("Yêu cầu không được hỗ trợ"));
  wrap(<ChatPanel projectId="project-1" mode="execute" locked={false}
    onModeChange={vi.fn()} onPreview={preview} onBusyChange={vi.fn()} />);
  fireEvent.change(screen.getByLabelText("Yêu cầu thiết kế"), { target: { value: "Vẽ 260 mm" } });
  await waitFor(() => expect(screen.getByRole("button", { name: "Gửi yêu cầu" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Gửi yêu cầu" }));
  await screen.findByRole("alert");
  expect(preview).toHaveBeenCalledExactlyOnceWith(null);
});

it("does not expose a late snapshot from an older run as the new request preview", async () => {
  const preview = vi.fn();
  const oldRun = { ...run, id: "old-run", status: "awaiting_approval" };
  fake.run.mockResolvedValue({ run: oldRun, events: [], change_sets: [{ id: "old-204", status: "pending_approval" }] });
  wrap(<ChatPanel projectId="project-1" mode="execute" locked={false}
    onModeChange={vi.fn()} onPreview={preview} onBusyChange={vi.fn()} />);
  fireEvent.change(screen.getByLabelText("Yêu cầu thiết kế"), { target: { value: "Vẽ 300 mm" } });
  await waitFor(() => expect(screen.getByRole("button", { name: "Gửi yêu cầu" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Gửi yêu cầu" }));
  await waitFor(() => expect(fake.run).toHaveBeenCalledWith(run.id));
  expect(preview).toHaveBeenCalledExactlyOnceWith(null);
});
