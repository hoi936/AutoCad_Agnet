# Tương thích

Kiểm tra ngày 05/10/2026 trong WSL Linux x86_64.

| Thành phần | Kết quả |
|---|---|
| Rust 1.99.0 | Core/contracts build và test |
| SQLx 0.9.0 / SQLite 3.46.0 | Mock tests; DELETE journal, chưa đáp ứng patched WAL |
| Node 26.7.0 / pnpm 10.18.3 | Frontend tooling; dependency cụ thể trong pnpm-lock.yaml |
| Tauri 2.12.1 / tauri-build 2.7.1 | Source/config; native build bị chặn bởi thiếu GLib/GDK development libs |
| rmcp 3.5.0 | Client/sidecar stdio process integration trên Linux |
| reqwest 0.13.5 / keyring 3.6.3 | HTTP fixture Linux; Windows vault chưa chạy |
| Windows desktop | NOT RUN — WINDOWS REQUIRED |
| Windows installer | NOT BUILT ON WINDOWS |
| AutoCAD / SDK / .NET bridge | NOT RUN — AUTOCAD REQUIRED; chưa có source bridge |

Không khẳng định hỗ trợ bất kỳ release/build AutoCAD nào. Trước khi xây bridge phải
đọc release/update và managed assemblies của máy đích, chọn TFM đúng runtime theo
https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-A6C680F2-DE2E-418A-A182-E4884073338A.htm.
Liên kết này là bước kiểm tra tiếp theo, chưa phải chứng nhận compatibility.
Không copy Autodesk assemblies vào repository hoặc dùng DLL giả.
