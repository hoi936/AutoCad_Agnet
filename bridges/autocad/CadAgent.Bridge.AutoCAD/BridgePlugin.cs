using System.Collections.Concurrent;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Windows.Forms;
using Autodesk.AutoCAD.Runtime;
using CadAgent.Bridge.Protocol;
using CadAgent.Bridge.Transport;
using AcApplication = Autodesk.AutoCAD.ApplicationServices.Application;

[assembly: CommandClass(typeof(CadAgent.Bridge.AutoCAD.BridgePlugin))]
namespace CadAgent.Bridge.AutoCAD;

public sealed class BridgePlugin : IExtensionApplication
{
    public const string PluginVersion = "0.2.0-r2";
    private static readonly ConcurrentQueue<(BridgeRequest Request, TaskCompletionSource<BridgeResponse> Completion)> Pending = new();
    private static readonly object Gate = new();
    private static PipeServer? _server;
    private static string? _pairCode;
    private static string? _session;
    private static int _pairFailures;
    private static readonly HashSet<string> SeenRequests = [];
    private static string _lastError = "none";
    private static readonly CadReads Reads = new();

    public void Initialize() => AcApplication.Idle += OnIdle;
    public void Terminate()
    {
        AcApplication.Idle -= OnIdle;
        _session = null;
        _pairCode = null;
        if (_server is not null) _ = _server.DisposeAsync();
    }

    [CommandMethod("CADAGENTCONNECT")]
    public void Connect()
    {
        // This UI is not AutoCAD command history or a log. Code is never echoed there.
        lock (Gate)
        {
            _pairCode = Convert.ToHexString(RandomNumberGenerator.GetBytes(16));
            _session = null;
            _pairFailures = 0;
            SeenRequests.Clear();
            if (_server is null)
            {
                _server = new PipeServer($"cadagent-r2-{Environment.ProcessId}", DispatchAsync);
                _server.Start();
            }
        }
        var doc = AcApplication.DocumentManager.MdiActiveDocument;
        doc?.Editor.WriteMessage("\nCAD Agent read-only bridge waiting for pairing. Open CAD Agent Studio > Kết nối.\n");
        using var form = new Form { Text = "CAD Agent Studio — Ghép cặp AutoCAD (chỉ đọc)", Width = 610, Height = 175, StartPosition = FormStartPosition.CenterScreen };
        var box = new TextBox { Text = _pairCode, ReadOnly = true, Width = 550, Left = 20, Top = 26 };
        var label = new Label { Text = "Nhập mã này vào CAD Agent Studio. Mã chỉ dùng một lần; không gửi cho LLM.", AutoSize = true, Left = 20, Top = 70 };
        form.Controls.Add(box); form.Controls.Add(label);
        form.ShowDialog();
    }
    [CommandMethod("CADAGENTSTATUS")]
    public void Status()
    {
        var doc = AcApplication.DocumentManager.MdiActiveDocument;
        var release = FileVersionInfo.GetVersionInfo(typeof(AcApplication).Assembly.Location).ProductVersion ?? "unknown";
        doc?.Editor.WriteMessage($"\nCAD Agent plugin {PluginVersion}, protocol {BridgeProtocol.Version}, AutoCAD {release}, PID {Environment.ProcessId}, bridge {(_server is null ? "stopped" : "listening")}, paired {(_session is not null)}, document {Path.GetFileName(doc.Name)}, read_document/read_selection/read_layers/query_entities/validate_entities=true, write=false, last_error={_lastError}.\n");
    }
    [CommandMethod("CADAGENTDISCONNECT")]
    public void Disconnect()
    {
        lock (Gate) { _session = null; _pairCode = null; SeenRequests.Clear(); }
        var server = _server;
        _server = null;
        if (server is not null) _ = server.DisposeAsync();
        AcApplication.DocumentManager.MdiActiveDocument?.Editor.WriteMessage("\nCAD Agent read-only bridge disconnected.\n");
    }
    private static Task<BridgeResponse> DispatchAsync(BridgeRequest request)
    {
        lock (Gate)
        {
            if (request.Operation == "pair")
            {
                if (_pairCode is null || _session is not null ||
                    !request.Args.TryGetProperty("code", out var supplied) || supplied.ValueKind != JsonValueKind.String ||
                    !request.Args.TryGetProperty("expected_pid", out var pid) || !pid.TryGetInt32(out var expected) || expected != Environment.ProcessId)
                    return Task.FromResult(BridgeResponse.Failure(request.RequestId, null, "PAIR_DENIED", "Pairing unavailable."));
                var received = supplied.GetString() ?? "";
                var same = received.Length == _pairCode.Length && CryptographicOperations.FixedTimeEquals(Encoding.ASCII.GetBytes(received), Encoding.ASCII.GetBytes(_pairCode));
                if (!same)
                {
                    if (++_pairFailures >= 3) _pairCode = null;
                    return Task.FromResult(BridgeResponse.Failure(request.RequestId, null, "PAIR_DENIED", "Pairing code invalid."));
                }
                _pairCode = null;
                _session = Guid.NewGuid().ToString("N");
                SeenRequests.Clear();
                return Task.FromResult(BridgeResponse.Success(request, _session, new { process_id = Environment.ProcessId, plugin_version = PluginVersion, protocol_version = BridgeProtocol.Version, read_only = true, write = false }));
            }
            if (_session is null || request.SessionId != _session)
                return Task.FromResult(BridgeResponse.Failure(request.RequestId, null, "SESSION_CHANGED", "Pair again in AutoCAD."));
            if (!SeenRequests.Add(request.RequestId))
                return Task.FromResult(BridgeResponse.Failure(request.RequestId, _session, "DUPLICATE_REQUEST", "Duplicate request ID."));
            if (SeenRequests.Count > 1024) SeenRequests.Clear();
            if (request.Operation == "disconnect")
            {
                var old = _session;
                _session = null;
                return Task.FromResult(BridgeResponse.Success(request, old, new { disconnected = true }));
            }
        }
        var completion = new TaskCompletionSource<BridgeResponse>(TaskCreationOptions.RunContinuationsAsynchronously);
        Pending.Enqueue((request, completion));
        return completion.Task;
    }
    private static void OnIdle(object? sender, EventArgs args)
    {
        if (!Pending.TryDequeue(out var work)) return;
        try
        {
            var session = _session;
            if (session is null || work.Request.SessionId != session)
                work.Completion.TrySetResult(BridgeResponse.Failure(work.Request.RequestId, null, "SESSION_CHANGED", "Pairing ended."));
            else
                work.Completion.TrySetResult(BridgeResponse.Success(work.Request, session, Reads.Execute(work.Request, session)));
        }
        catch (BridgeProtocolException e)
        {
            _lastError = e.Code;
            work.Completion.TrySetResult(BridgeResponse.Failure(work.Request.RequestId, _session, e.Code, e.Message));
        }
        catch (System.Exception)
        {
            _lastError = "CAD_READ_FAILED";
            work.Completion.TrySetResult(BridgeResponse.Failure(work.Request.RequestId, _session, "CAD_READ_FAILED", "AutoCAD read unavailable."));
        }
    }
}
