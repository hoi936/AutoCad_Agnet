import { spawnSync } from "node:child_process";
import { mkdirSync, copyFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";
const root = fileURLToPath(new URL("../", import.meta.url));
const mode = process.argv[2];
if (!["dev", "build"].includes(mode)) throw new Error("Expected dev or build");
function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit", shell: process.platform === "win32" });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
const info = spawnSync("rustc", ["-vV"], { encoding: "utf8" });
const target = info.stdout?.match(/^host: (.+)$/m)?.[1];
if (!target) throw new Error("Không tìm thấy Rust host toolchain");
// Keep a sibling binary for tauri build --debug --no-bundle as well.
run("cargo", ["build", "--locked", "-p", "cad-tool-host"]);
if (mode === "build") run("cargo", ["build", "--locked", "-p", "cad-tool-host", "--release"]);
const extension = process.platform === "win32" ? ".exe" : "";
const destination = resolve(root, "apps/desktop/src-tauri/binaries");
mkdirSync(destination, { recursive: true });
copyFileSync(resolve(root, process.env.CARGO_TARGET_DIR ?? "target", mode === "build" ? "release" : "debug", `cad-tool-host${extension}`), resolve(destination, `cad-tool-host-${target}${extension}`));
run(process.platform === "win32" ? "pnpm.cmd" : "pnpm", [mode === "dev" ? "dev:web" : "build"]);
