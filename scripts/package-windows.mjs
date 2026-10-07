import { spawnSync } from "node:child_process";
if (process.platform !== "win32") {
  console.error(
    "NOT BUILT ON WINDOWS: chạy script này trong PowerShell Windows với Rust MSVC, Build Tools và WebView2.",
  );
  process.exit(1);
}
console.log(
  "Đóng gói bản nền MOCK chưa hoàn chỉnh. Có MCP sidecar mô phỏng; chưa có AutoCAD plugin. Installer development chưa ký.",
);
const result = spawnSync(
  "pnpm.cmd",
  ["--filter", "@cad/desktop", "tauri", "build", "--bundles", "nsis"],
  { stdio: "inherit", shell: true },
);
process.exit(result.status ?? 1);
