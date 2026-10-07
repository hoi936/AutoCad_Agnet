import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api, errorMessage, native } from "./ipc";
import { LiveWorkspace } from "./LiveWorkspace";
import { DemoWorkspace } from "./DemoWorkspace";
const pages = ["Chat", "Kết nối", "Skills", "Lịch sử", "Chẩn đoán"] as const;
export default function App() {
  const [page, setPage] = useState<typeof pages[number]>("Chat");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [pairCode, setPairCode] = useState("");
  const [selectedPid, setSelectedPid] = useState(0);
  const client = useQueryClient();
  const runtime = useQuery({ queryKey: ["runtime"], queryFn: api.runtime, enabled: native, retry: false });
  const demo = runtime.data?.mode === "demo";
  const cadConnected = runtime.data?.mode === "live_read_only";
  const candidates = useQuery({ queryKey:["cad-candidates"], queryFn:api.cadCandidates, enabled:native && page === "Kết nối", retry:false });
  const cadStatus = useQuery({ queryKey:["cad-live-status"], queryFn:api.cadLiveStatus, enabled:native && page === "Kết nối", retry:false, refetchInterval:3000 });
  const pid = candidates.data?.includes(selectedPid) ? selectedPid : candidates.data?.[0] ?? 0;
  async function pairCad() {
    setBusy(true); setError("");
    try { await api.cadPair(pid, pairCode.trim()); setPairCode(""); await client.invalidateQueries(); }
    catch (failure) { setPairCode(""); setError(errorMessage(failure)); }
    finally { setBusy(false); }
  }
  async function disconnectCad() {
    setBusy(true); setError("");
    try { await api.cadDisconnect(); await client.invalidateQueries(); }
    catch (failure) { setError(errorMessage(failure)); }
    finally { setBusy(false); }
  }
  async function enterDemo() {
    setBusy(true); setError("");
    try { await api.enterDemo(); await client.invalidateQueries(); setPage("Chat"); }
    catch (failure) { setError(errorMessage(failure)); }
    finally { setBusy(false); }
  }
  return <div className="studio dark chat-first">
    <header><div className="brand"><b className="logo">CA</b><strong>CAD Agent Studio</strong></div>
      <span>{native ? "Desktop native" : "Web preview"} · {demo ? "Demo đã chọn" : cadConnected ? "AutoCAD · chỉ đọc" : "Live · CAD chưa kết nối"}</span></header>
    <nav className="top-navigation" aria-label="Điều hướng chính">{pages.map((item) =>
      <button key={item} aria-current={page === item ? "page" : undefined} onClick={() => setPage(item)}>{item}</button>)}</nav>
    {!native && <p role="status" className="browser-banner">Bản xem giao diện trên trình duyệt. Các thao tác cần Rust đã khóa; mở desktop bằng pnpm dev. Chưa kết nối Rust.</p>}
    {(error || runtime.error) && <p role="alert">{error || errorMessage(runtime.error)}</p>}
    <LiveWorkspace view={page} demo={demo} />
    {page === "Chat" && demo && <DemoWorkspace />}
    {page === "Kết nối" && <section className="connection-page"><article><h2>AutoCAD · chỉ đọc</h2><p>{cadStatus.data?.connected ? "Đã ghép cặp và đọc document thật. Chat có thể dùng công cụ đọc AutoCAD." : "Mở DWG thử nghiệm trong AutoCAD. Chạy CADAGENTCONNECT, sao chép mã trong cửa sổ plugin rồi đóng cửa sổ đó trước khi ghép cặp."}</p>
      {!cadStatus.data?.connected && <><label>Tiến trình AutoCAD <select value={pid} onChange={event=>setSelectedPid(Number(event.target.value))}>{(candidates.data ?? []).map(value=><option key={value} value={value}>acad.exe · PID {value}</option>)}</select></label>
      <label>Mã ghép cặp <input type="password" autoComplete="off" value={pairCode} onChange={event=>setPairCode(event.target.value)} maxLength={32} /></label>
      <button disabled={!native||busy||!pid||pairCode.trim().length!==32||demo} onClick={()=>void pairCad()}>{busy?"Đang xác minh…":"Ghép cặp và đọc document"}</button></>}
      {cadStatus.data?.connected && <button disabled={busy} onClick={()=>void disconnectCad()}>Ngắt kết nối</button>}
      {!candidates.data?.length && !cadStatus.data?.connected && <p>Chưa tìm thấy acad.exe đang chạy. Mở AutoCAD rồi tải lại trang Kết nối.</p>}</article>
      <article><h2>Developer Demo</h2><p>MockProvider fixture tấm đế; CAD mô phỏng, không ghi DWG.</p><button disabled={!native||busy||demo} onClick={()=>void enterDemo()}>{busy?"Đang mở Demo…":demo?"Demo đang mở":"Chọn Demo mô phỏng"}</button></article></section>}
    {page === "Skills" && <section className="connection-page"><h1>Skills</h1><p>Chưa có runtime nạp/bật/tắt skill. Tấm đế hiện là fixture regression trong Demo.</p><p>Primitives, sửa selection và phác thảo mặt bằng chưa triển khai; chưa có quyền chạy trên AutoCAD.</p></section>}
    {page === "Lịch sử" && <section className="connection-page"><h1>Lịch sử</h1><p>{demo ? "Mở Chat, chọn dự án để xem hội thoại và phương án Demo đã lưu." : "Chưa mở phiên dữ liệu. Lịch sử Demo cũ được giữ; chọn Demo để xem."}</p></section>}
    {page === "Chẩn đoán" && <section className="connection-page"><h1>Chẩn đoán / About</h1><dl>
      <dt>Build ID</dt><dd>{runtime.data?.build_id ?? "Chưa nhận từ Rust"}</dd><dt>Runtime</dt><dd>{native ? "Desktop native" : "Web preview"}</dd>
      <dt>MCP</dt><dd>{runtime.data?.mcp ?? "Chưa xác nhận"}</dd><dt>CAD</dt><dd>{runtime.data?.cad ?? "Chưa xác nhận"}</dd>
      <dt>OAuth</dt><dd>{runtime.data?.oauth ?? "Chưa xác nhận"}</dd></dl><p>Startup cửa sổ không thay thế kiểm thử IPC, inference hoặc readback AutoCAD.</p></section>}
    <footer>Build: {runtime.data?.build_id ?? "web / chưa có backend"} · {demo ? "CAD mock" : cadConnected ? "AutoCAD chỉ đọc" : "Chưa kết nối CAD"}</footer>
  </div>;
}
