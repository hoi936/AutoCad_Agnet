import { useCallback, useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { create } from "zustand";
import { api, errorMessage, native } from "./ipc";
import {
  defaultPlate,
  type ChangeSet,
  type RunMode,
  type PlateParameters,
} from "./contracts";
import { Preview } from "./Preview";
import { ChatPanel } from "./ChatPanel";
const useUI = create<{
  project: string;
  theme: "dark" | "light";
  select: (id: string) => void;
  toggle: () => void;
}>((set) => ({
  project: "",
  theme: "dark",
  select: (project) => set({ project }),
  toggle: () => set((s) => ({ theme: s.theme === "dark" ? "light" : "dark" })),
}));
export function DemoWorkspace() {
  const { project, select, theme, toggle } = useUI();
  const client = useQueryClient();
  const [name, setName] = useState("");
  const [parameters, setParameters] = useState<PlateParameters>(defaultPlate);
  const [mode, setMode] = useState<RunMode>("execute");
  const [change, setChange] = useState<ChangeSet | null>(null);
  const [historical, setHistorical] = useState(false);
  const [actionBusy, setBusy] = useState(false);
  const [chatBusy, setChatBusy] = useState(false);
  const busy = actionBusy || chatBusy;
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [now, setNow] = useState(Date.now());
  const health = useQuery({
    queryKey: ["health"],
    queryFn: api.health,
    enabled: native,
    retry: false,
  });
  const projects = useQuery({
    queryKey: ["projects"],
    queryFn: api.projects,
    enabled: native,
    retry: false,
  });
  const context = useQuery({
    queryKey: ["context", project],
    queryFn: () => api.context(project),
    enabled: native && !!project,
    refetchInterval: 5000,
    retry: false,
  });
  const history = useQuery({
    queryKey: ["changes", project],
    queryFn: () => api.changes(project),
    enabled: native && !!project,
    retry: false,
  });
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  const data = context.data;
  const p = change?.payload;
  const stale =
    !!p &&
    (p.base_revision !== data?.revision ||
      p.session_id !== data?.session_id ||
      p.expires_at * 1000 <= now);
  const canApply =
    !!change &&
    change.status === "pending_approval" &&
    !stale &&
    !context.isError &&
    !!data &&
    !busy && !historical;
  async function act(action: () => Promise<void>) {
    setBusy(true);
    setError("");
    try {
      await action();
      await client.invalidateQueries();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      await client.invalidateQueries();
      setBusy(false);
    }
  }
  const receivePreview = useCallback((preview: ChangeSet | null) => {
    setChange(preview);
    setHistorical(false);
  }, []);
  function switchProject(id: string) {
    select(id);
    setChange(null);
    setNotice("");
    setParameters(defaultPlate);
  }
  return (
    <div className={`studio ${theme}`}>
      <header>
        <div className="brand">
          <b className="logo">CA</b>
          <strong>
            CAD Agent <span>Studio</span>
          </strong>
          <small>FOUNDATION / 0.1</small>
        </div>
        <button onClick={toggle} aria-label="Đổi giao diện sáng tối">
          {theme === "dark" ? "☀ Sáng" : "☾ Tối"}
        </button>
      </header>
      <div className="demo-banner">
        MÔ PHỎNG — KHÔNG PHẢI AUTOCAD{" "}
        <span>CAD mô phỏng 2D · Quyền cloud theo dự án/provider</span>
      </div>
      {!native && (
        <div role="status" className="browser-banner">
          Bản xem giao diện trên trình duyệt. Các thao tác cần Rust đã khóa;
          chạy <code>pnpm dev</code> để dùng desktop.
        </div>
      )}
      <main>
        <aside className="sidebar">
          <h2>KHÔNG GIAN LÀM VIỆC</h2>
          <p className="muted">Dự án cục bộ</p>
          {projects.isLoading && <p>Đang đọc dự án…</p>}
          {projects.error && <p role="alert">{errorMessage(projects.error)}</p>}
          <nav aria-label="Dự án">
            {projects.data?.map((item) => (
              <button
                className={project === item.id ? "project active" : "project"}
                key={item.id}
                disabled={busy}
                onClick={() => switchProject(item.id)}
              >
                ▧ <span>{item.name}</span>
              </button>
            ))}
          </nav>
          {!projects.data?.length && (
            <p className="muted">Chưa có dự án. Tạo một dự án để bắt đầu.</p>
          )}
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void act(async () => {
                const created = await api.create(name);
                switchProject(created.id);
                setName("");
              });
            }}
          >
            <label htmlFor="project-name">Tên dự án</label>
            <input
              id="project-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={120}
              placeholder="Ví dụ: Tấm đế động cơ"
            />
            <button disabled={!native || busy || !name.trim()}>
              ＋ Tạo dự án
            </button>
          </form>
          <div className="sidebar-section">
            <h2>BẢN VẼ</h2>
            <p>▧ {project ? "Tấm đế mô phỏng" : "Chưa chọn dự án"}</p>
            <small className="muted">WCS XY · Model space · mm</small>
          </div>
          <div className="sidebar-section">
            <h2>QUY TRÌNH</h2>
            <p>◇ Tấm đế 2D / v1</p>
            <small className="muted">4 lỗ · 3 layer · Preview trước khi duyệt</small>
          </div>
          <div className="sidebar-section">
            <h2>KẾT NỐI</h2>
            <button disabled title="Chưa triển khai bridge AutoCAD">
              AutoCAD — chưa tích hợp
            </button>
            <p className="muted">LLM mock/live được chọn trong cài đặt hội thoại.</p>
          </div>
        </aside>
        <section className="work">
          <div className="section-heading">
            <div>
              <span className="eyebrow">DESIGN WORKSPACE</span>
              <h1>Từ yêu cầu đến phương án</h1>
            </div>
            <span className="chip">CAD Mock</span>
          </div>
          <div className="intro">
            <span className="agent-icon">◇</span>
            <div>
              <h3>Tấm đế bốn lỗ</h3>
              <p>
                Trao đổi với agent, kiểm tra hình học dự kiến rồi duyệt để ghi
                vào CAD mô phỏng. Có thể nhập tham số trực tiếp bên dưới.
              </p>
            </div>
          </div>
          <div className="workflow">
            <span>01 Yêu cầu</span>
            <span>02 Preview</span>
            <span>03 Duyệt & xác minh</span>
          </div>
          <ChatPanel key={project} projectId={project} mode={mode} locked={actionBusy}
            onModeChange={setMode} onPreview={receivePreview} onBusyChange={setChatBusy} />
          <details><summary>Examples / Developer Demo — Thông số tấm đế</summary>
          <form
            className="parameter-form"
            onSubmit={(e) => {
              e.preventDefault();
              void act(async () => {
                setHistorical(false);
                setChange(await api.propose(project, parameters, mode));
                setNotice(
                  "Đã kiểm tra tham số và tạo preview. Hình học mô phỏng chưa thay đổi.",
                );
              });
            }}
          >
            <div className="form-heading">
              <h2>Thông số tấm đế</h2>
              <span className="muted">Đơn vị: mm</span>
            </div>
            <div className="fields">
              {(
                [
                  ["width", "Chiều dài"],
                  ["height", "Chiều rộng"],
                  ["hole_diameter", "Đường kính lỗ"],
                  ["edge_offset", "Tâm lỗ cách mép"],
                ] as const
              ).map(([key, label]) => (
                <label key={key}>
                  {label}
                  <input
                    type="number"
                    required
                    step="any"
                    min="0.001"
                    max="1000000"
                    value={parameters[key]}
                    onChange={(e) =>
                      setParameters({
                        ...parameters,
                        [key]: Number(e.target.value),
                      })
                    }
                  />
                </label>
              ))}
            </div>
            <p className="muted">
              Gốc (0, 0) · Không xoay · AI_OUTLINE / AI_HOLES / AI_DIM
            </p>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={parameters.add_dimensions}
                onChange={(e) =>
                  setParameters({
                    ...parameters,
                    add_dimensions: e.target.checked,
                  })
                }
              />{" "}
              Ghi kích thước chiều dài, chiều rộng, đường kính
            </label>
            <div className="actions">
              <select
                aria-label="Chế độ lập phương án"
                value={mode}
                disabled={busy}
                onChange={(e) => setMode(e.target.value as RunMode)}
              >
                <option value="execute">Execute — cần duyệt</option>
                <option value="plan">Plan — chỉ phương án</option>
                <option value="ask">Ask — chỉ đọc</option>
              </select>
              <button
                type="button"
                disabled={!data?.geometry || busy}
                onClick={() => {
                  if (data?.geometry)
                    setParameters({ ...data.geometry.parameters, width: 240 });
                }}
              >
                Resize → 240
              </button>
              <button
                className="primary"
                disabled={!native || !project || busy || mode === "ask"}
              >
                {busy ? "Đang xử lý…" : "Lập phương án →"}
              </button>
            </div>
            {mode === "ask" && (
              <p className="muted">
                Ask không được tạo hoặc áp dụng thay đổi. Ngữ cảnh hiện tại hiển
                thị bên phải.
              </p>
            )}
          </form>
          </details>
          {notice && (
            <p role="status" className="notice">
              {notice}
            </p>
          )}
          {(error || context.error) && (
            <p role="alert" className="error">
              {error || errorMessage(context.error)}
            </p>
          )}
          <div className="history">
            <h2>LỊCH SỬ PHƯƠNG ÁN</h2>
            {history.data?.length ? (
              history.data.map((c) => (
                <div className="history-row" key={c.id}>
                  <button disabled={busy} onClick={() => { setHistorical(true); setChange(c); }}>
                    {c.payload.reverts
                      ? "Hoàn tác"
                      : c.payload.before
                        ? "Sửa tấm đế"
                        : "Tạo tấm đế"}{" "}
                    · {c.payload.after?.parameters.width ?? 0} mm
                  </button>
                  <span>{c.status}</span>
                  {["outcome_unknown", "recovery_required", "sending"].includes(c.status) && (
                    <button disabled={busy} onClick={() => void act(async () => {
                      await api.recover(c.id);
                      setChange(null);
                      setNotice("Đã đối chiếu receipt và hình học; không gửi lại thao tác ghi.");
                    })}>Đối chiếu kết quả</button>
                  )}
                  {c.status === "verified" && (
                    <button
                      disabled={busy}
                      onClick={() =>
                        void act(async () => {
                          setHistorical(false);
                          setChange(await api.revert(c.id));
                          setNotice(
                            "Đã tạo phương án hoàn tác. Cần duyệt trước khi áp dụng.",
                          );
                        })
                      }
                    >
                      Đề xuất hoàn tác
                    </button>
                  )}
                </div>
              ))
            ) : (
              <p className="muted">
                Các phương án và thao tác đã duyệt sẽ xuất hiện tại đây.
              </p>
            )}
          </div>
        </section>
        <aside className="preview-panel">
          <div className="form-heading">
            <h2>HÌNH HỌC 2D</h2>
            <span className="chip">{change ? "Dự kiến" : "Hiện tại"}</span>
          </div>
          <div className="drawing">
            <Preview
              geometry={
                change ? change.payload.after : (data?.geometry ?? null)
              }
            />
          </div>
          <div className="details">
            <h3>{change ? "Kiểm tra trước khi duyệt" : "Ngữ cảnh document"}</h3>
            <dl>
              <dt>Document</dt>
              <dd>{project ? "Tấm đế mô phỏng" : "—"}</dd>
              <dt>Revision hiện tại</dt>
              <dd>{data?.revision ?? "—"}</dd>
              <dt>Đơn vị / gốc</dt>
              <dd>mm / (0, 0)</dd>
              <dt>Entity hiện tại</dt>
              <dd>{data?.geometry?.entities.length ?? 0}</dd>
            </dl>
            {change && (
              <>
                <dl>
                  <dt>Trạng thái</dt>
                  <dd>{change.status}</dd>
                  <dt>Base revision</dt>
                  <dd>{p?.base_revision}</dd>
                  <dt>Entity trước → sau</dt>
                  <dd>
                    {p?.before?.entities.length ?? 0} →{" "}
                    {p?.after?.entities.length ?? 0}
                  </dd>
                  <dt>Chiều dài trước → sau</dt>
                  <dd>
                    {p?.before?.parameters.width ?? "—"} →{" "}
                    {p?.after?.parameters.width ?? "—"}
                  </dd>
                  <dt>Validation</dt>
                  <dd>Hợp lệ lúc lập phương án</dd>
                  <dt>Hiệu lực</dt>
                  <dd>
                    {stale
                      ? "Cần lập lại"
                      : `${Math.max(0, Math.ceil(((p?.expires_at ?? 0) * 1000 - now) / 1000))} giây`}
                  </dd>
                </dl>
                <details>
                  <summary>Hash & document ID</summary>
                  <code className="hash">
                    {change.payload_hash}
                    <br />
                    {p?.document_id}
                  </code>
                </details>
                {historical && <p role="status">Phương án lịch sử chỉ để xem. Gửi lại yêu cầu để kiểm tra ngữ cảnh và lập phương án mới.</p>}
                <div className="approval">
                  <button
                    className="primary"
                    disabled={!canApply}
                    onClick={() =>
                      void act(async () => {
                        await api.approve(change);
                        setChange(null);
                        setNotice(
                          "Đã áp dụng vào mô phỏng và xác minh bằng mã. Không tạo hoặc lưu file DWG.",
                        );
                      })
                    }
                  >
                    Duyệt & áp dụng
                  </button>
                  <button
                    disabled={
                      busy ||
                      !["pending_approval", "planned"].includes(change.status)
                    }
                    onClick={() =>
                      void act(async () => {
                        await api.reject(change.id);
                        setChange(null);
                        setNotice("Đã từ chối; không thay đổi hình học.");
                      })
                    }
                  >
                    Từ chối
                  </button>
                </div>
              </>
            )}
            <p className="footnote">
              Preview chỉ thể hiện chi tiết đang xử lý. Hình học mô phỏng lưu
              cục bộ; không phải bản vẽ DWG.
            </p>
          </div>
        </aside>
      </main>
      <footer>
        <span className={health.data ? "connected" : ""}>
          ● {health.data ?? "Chưa kết nối Rust"}
        </span>
        <span>
          CAD mock · SQLite/chat chưa mã hóa · {busy ? "Đang xử lý" : "Sẵn sàng"}
        </span>
      </footer>
    </div>
  );
}
