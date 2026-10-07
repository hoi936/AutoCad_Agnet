using System.Buffers.Binary;
using System.IO.Pipes;
using CadAgent.Bridge.Protocol;

namespace CadAgent.Bridge.Transport;

public static class BridgeFrames
{
    public static async Task<byte[]> ReadAsync(Stream stream, CancellationToken cancel)
    {
        using var time = CancellationTokenSource.CreateLinkedTokenSource(cancel);
        time.CancelAfter(BridgeProtocol.IoTimeout);
        var lengthBytes = new byte[4];
        await stream.ReadExactlyAsync(lengthBytes, time.Token).ConfigureAwait(false);
        var length = BinaryPrimitives.ReadUInt32LittleEndian(lengthBytes);
        if (length is < 2 or > BridgeProtocol.MaxFrameBytes)
            throw new BridgeProtocolException("FRAME_SIZE", "Frame exceeds limit.");
        var bytes = new byte[length];
        await stream.ReadExactlyAsync(bytes, time.Token).ConfigureAwait(false);
        return bytes;
    }
    public static async Task WriteAsync(Stream stream, byte[] payload, CancellationToken cancel)
    {
        if (payload.Length is < 2 or > BridgeProtocol.MaxFrameBytes)
            throw new BridgeProtocolException("FRAME_SIZE", "Frame exceeds limit.");
        using var time = CancellationTokenSource.CreateLinkedTokenSource(cancel);
        time.CancelAfter(BridgeProtocol.IoTimeout);
        var length = new byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(length, checked((uint)payload.Length));
        await stream.WriteAsync(length, time.Token).ConfigureAwait(false);
        await stream.WriteAsync(payload, time.Token).ConfigureAwait(false);
        await stream.FlushAsync(time.Token).ConfigureAwait(false);
    }
    public static NamedPipeServerStream CreateLocalServer(string name) =>
        NamedPipeServerStreamAcl.Create(name, PipeDirection.InOut, 1, PipeTransmissionMode.Byte,
            PipeOptions.Asynchronous | PipeOptions.CurrentUserOnly, 65536, 65536, null);
}
