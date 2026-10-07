using System.IO.Pipes;
using CadAgent.Bridge.Protocol;

namespace CadAgent.Bridge.Transport;

public sealed class PipeServer(string name, Func<BridgeRequest, Task<BridgeResponse>> dispatch) : IAsyncDisposable
{
    private readonly CancellationTokenSource _stop = new();
    private readonly string _name = name;
    private readonly Func<BridgeRequest, Task<BridgeResponse>> _dispatch = dispatch;
    private Task? _loop;
    private NamedPipeServerStream? _listener;
    public void Start()
    {
        if (_loop is not null) throw new InvalidOperationException("Pipe server already started.");
        _loop = Task.Run(RunAsync);
    }
    private async Task RunAsync()
    {
        while (!_stop.IsCancellationRequested)
        {
            try
            {
                using var pipe = BridgeFrames.CreateLocalServer(_name);
                _listener = pipe;
                await pipe.WaitForConnectionAsync(_stop.Token).ConfigureAwait(false);
                _listener = null;
                while (pipe.IsConnected && !_stop.IsCancellationRequested)
                {
                    BridgeResponse response;
                    try
                    {
                        var payload = await BridgeFrames.ReadAsync(pipe, _stop.Token).ConfigureAwait(false);
                        var request = BridgeProtocol.ParseRequest(payload);
                        using var deadline = CancellationTokenSource.CreateLinkedTokenSource(_stop.Token);
                        deadline.CancelAfter(BridgeProtocol.CadTimeout);
                        response = await _dispatch(request).WaitAsync(deadline.Token).ConfigureAwait(false);
                    }
                    catch (EndOfStreamException) { break; }
                    catch (OperationCanceledException) when (_stop.IsCancellationRequested) { break; }
                    catch (OperationCanceledException) { response = BridgeResponse.Failure("", null, "CAD_BUSY", "AutoCAD read timed out."); }
                    catch (BridgeProtocolException e) { response = BridgeResponse.Failure("", null, e.Code, e.Message); }
                    catch (Exception) { response = BridgeResponse.Failure("", null, "BRIDGE_ERROR", "Read request failed."); }
                    try { await BridgeFrames.WriteAsync(pipe, BridgeProtocol.Encode(response), _stop.Token).ConfigureAwait(false); }
                    catch (IOException) { break; }
                    catch (OperationCanceledException) { break; }
                }
            }
            catch (OperationCanceledException) when (_stop.IsCancellationRequested) { break; }
            catch (ObjectDisposedException) when (_stop.IsCancellationRequested) { break; }
            catch (IOException) { await Task.Delay(200, _stop.Token).ConfigureAwait(false); }
        }
    }
    public async ValueTask DisposeAsync()
    {
        _stop.Cancel();
        _listener?.Dispose();
        if (_loop is not null) { try { await _loop.ConfigureAwait(false); } catch (OperationCanceledException) { } }
        _stop.Dispose();
    }
}
