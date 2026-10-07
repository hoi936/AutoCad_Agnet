import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { ProviderConfig } from "./contracts";
import { api, errorMessage, native } from "./ipc";

export function ProviderSettings({
  config,
  disabled,
  onBusyChange,
}: {
  config: ProviderConfig;
  disabled: boolean;
  onBusyChange: (busy: boolean) => void;
}) {
  const client = useQueryClient();
  const [provider, setProvider] = useState(config.provider);
  const [model, setModel] = useState(config.model);
  const [storage, setStorage] = useState(config.storage);
  const [consent, setConsent] = useState(config.cloud_consent);
  const [secret, setSecret] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const dirty = provider !== config.provider || model !== config.model ||
    storage !== config.storage || consent !== config.cloud_consent;

  async function update(action: () => Promise<unknown>, success: string) {
    setBusy(true);
    onBusyChange(true);
    setError("");
    setNotice("");
    try {
      await action();
      await client.invalidateQueries({ queryKey: ["provider", config.project_id] });
      setNotice(success);
    } catch (failure) {
      setError(errorMessage(failure));
    } finally {
      setSecret("");
      setBusy(false);
      onBusyChange(false);
    }
  }

  return (
    <details className="provider-settings">
      <summary>Cài đặt provider và quyền dữ liệu</summary>
      <form onSubmit={(event) => {
        event.preventDefault();
        void update(
          () => api.saveProvider(config.project_id, provider, model, consent, storage),
          "Đã lưu cấu hình cho dự án này.",
        );
      }}>
        <fieldset disabled={!native || disabled || busy}>
          <label>
            Provider
            <select value={provider} onChange={(event) => {
              setProvider(event.target.value);
              setModel(event.target.value === "mock" ? "mock-plate-v1" : "");
              setStorage(event.target.value === "openai" ? "windows" : "session");
              setConsent(false);
              setSecret("");
            }}>
              <option value="mock">MockProvider — fixture, không Internet</option>
              <option value="openai">OpenAI — gọi API cloud</option>
            </select>
          </label>
          <label>
            Model ID
            <input value={model} onChange={(event) => setModel(event.target.value)}
              disabled={provider === "mock"} required maxLength={200}
              placeholder="Nhập model ID được cấp quyền trong tài khoản API" />
          </label>
          {provider === "openai" && <>
            <label>
              Nơi giữ credential
              <select value={storage} onChange={(event) => setStorage(event.target.value)}>
                <option value="windows">Windows Credential Manager</option>
                <option value="session">Chỉ phiên chạy này — mất khi đóng ứng dụng</option>
              </select>
            </label>
            <p className="muted">OpenAI nhận nội dung hội thoại và ngữ cảnh CAD tối thiểu của dự án. Dữ liệu lưu phía provider tuân theo chính sách tài khoản/API; không cam kết zero retention.</p>
            <label className="checkbox">
              <input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} />
              Cho phép dự án này gửi nội dung trên tới OpenAI
            </label>
          </>}
          <button disabled={!dirty || !model.trim()}>Lưu cấu hình</button>
        </fieldset>
      </form>
      {config.provider === "openai" && <form onSubmit={(event) => {
        event.preventDefault();
        const submittedSecret = secret;
        setSecret("");
        void update(() => api.setSecret(config.project_id, submittedSecret),
          config.storage === "session" ? "Đã giữ khóa trong phiên hiện tại." : "Đã lưu khóa trong Windows Credential Manager.");
      }}>
        <fieldset disabled={!native || disabled || busy || dirty}>
          <p>Credential: {config.configured ? "Đã cấu hình" : "Chưa có khóa"} · {config.storage === "session" ? "chỉ trong phiên" : "Windows Credential Manager"}</p>
          <label>
            API key (gửi một lần)
            <input type="password" value={secret} onChange={(event) => setSecret(event.target.value)}
              autoComplete="new-password" spellCheck={false} maxLength={4096} />
          </label>
          <div className="actions">
            <button disabled={!secret.trim()}>{config.configured ? "Thay khóa" : "Lưu khóa"}</button>
            <button type="button" disabled={!config.configured} onClick={() => {
              setSecret("");
              void update(() => api.deleteSecret(config.project_id), "Đã xóa credential của cấu hình này.");
            }}>Xóa khóa</button>
          </div>
        </fieldset>
        {dirty && <p className="muted">Lưu cấu hình trước khi thay credential.</p>}
      </form>}
      {notice && <p role="status" className="notice">{notice}</p>}
      {error && <p role="alert" className="error">{error}</p>}
    </details>
  );
}
