# AutoCAD bridge — khảo sát T0

**NOT_IMPLEMENTED:** chưa có C# plugin, named-pipe transport, pairing hoặc live gateway.
Prompt continuation 02 mở phạm vi triển khai; không còn là hạng mục bị loại khỏi đợt này.

Máy đích đã quan sát AutoCAD 2026.1.2, registered update 25.1.179.0,
acad.exe R25.1.179.0.0 và ba assembly AcMgd/AcDbMgd/AcCoreMgd version 25.1.0.0.
Không commit Autodesk DLL. Doctor chỉ đọc metadata, không load assembly để chạy API.
AutoCAD Mechanical 2024 cũng có trên máy nhưng không nằm trong profile được chọn.

Profile ứng viên: AutoCAD đầy đủ 2026.1.2 Windows x64, .NET 10 theo
[tài liệu tương thích Autodesk](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-A6C680F2-DE2E-418A-A182-E4884073338A.htm).
Còn phải xác minh SDK/build tool, runtime thực tế, reference path và load test;
chưa tạo profile có nhãn verified chỉ từ bản cài. Không tự nâng/hạ AutoCAD.

Bước T4: tách Protocol/Transport khỏi AutoCAD API, test framing/authorization độc lập,
main-thread dispatcher, CADAGENTCONNECT/STATUS/DISCONNECT và read-only document/layer/selection.
Sau handshake/readback thật mới mở gate T5 mutation có permit, transaction, marker và recovery.
Process acad tồn tại không có nghĩa Connected. Chưa cho phép gửi bất kỳ mutation tới DWG.
