using System.Text.Json;
using System.Text.Json.Serialization;

namespace CadAgent.Bridge.Protocol;

public sealed record BridgeRequest(
    [property: JsonPropertyName("version")] int Version,
    [property: JsonPropertyName("request_id")] string RequestId,
    [property: JsonPropertyName("session_id")] string? SessionId,
    [property: JsonPropertyName("operation")] string Operation,
    [property: JsonPropertyName("document_id")] string? DocumentId,
    [property: JsonPropertyName("args")] JsonElement Args);

public sealed record BridgeError(
    [property: JsonPropertyName("code")] string Code,
    [property: JsonPropertyName("message")] string Message);

public sealed record BridgeResponse(
    [property: JsonPropertyName("version")] int Version,
    [property: JsonPropertyName("request_id")] string RequestId,
    [property: JsonPropertyName("session_id")] string? SessionId,
    [property: JsonPropertyName("ok")] bool Ok,
    [property: JsonPropertyName("result")] object? Result,
    [property: JsonPropertyName("error")] BridgeError? Error)
{
    public static BridgeResponse Success(BridgeRequest request, string? session, object result) =>
        new(BridgeProtocol.Version, request.RequestId, session, true, result, null);
    public static BridgeResponse Failure(string requestId, string? session, string code, string message) =>
        new(BridgeProtocol.Version, requestId, session, false, null, new(code, message));
}

public sealed class BridgeProtocolException(string code, string message) : Exception(message)
{
    public string Code { get; } = code;
}

public static class BridgeProtocol
{
    public const int Version = 1;
    public const int MaxFrameBytes = 65536;
    public const int MaxQueryEntities = 100;
    public static readonly TimeSpan IoTimeout = TimeSpan.FromSeconds(5);
    public static readonly TimeSpan CadTimeout = TimeSpan.FromSeconds(10);
    public static readonly JsonSerializerOptions Json = new(JsonSerializerDefaults.Web)
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        PropertyNameCaseInsensitive = false,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        MaxDepth = 24
    };
    private static readonly HashSet<string> RequestFields =
        ["version", "request_id", "session_id", "operation", "document_id", "args"];
    private static readonly HashSet<string> ReadOperations =
        ["pair", "disconnect", "get_status", "list_documents", "get_document_context",
         "get_selection", "list_layers", "query_entities", "validate_entities", "get_operation_status"];
    public static BridgeRequest ParseRequest(ReadOnlySpan<byte> bytes)
    {
        if (bytes.Length is < 2 or > MaxFrameBytes) throw new BridgeProtocolException("FRAME_SIZE", "Frame size invalid.");
        using var document = JsonDocument.Parse(bytes.ToArray(), new JsonDocumentOptions { MaxDepth = 24, CommentHandling = JsonCommentHandling.Disallow });
        var root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Object || root.EnumerateObject().Any(p => !RequestFields.Contains(p.Name)) ||
            root.EnumerateObject().GroupBy(p => p.Name).Any(g => g.Count() != 1))
            throw new BridgeProtocolException("INVALID_ARGUMENTS", "Request fields invalid.");
        foreach (var required in new[] { "version", "request_id", "operation", "args" })
            if (!root.TryGetProperty(required, out _)) throw new BridgeProtocolException("INVALID_ARGUMENTS", "Required field missing.");
        var request = JsonSerializer.Deserialize<BridgeRequest>(root.GetRawText(), Json)
            ?? throw new BridgeProtocolException("INVALID_ARGUMENTS", "Request invalid.");
        if (request.Version != Version) throw new BridgeProtocolException("PROTOCOL_MISMATCH", "Protocol version mismatch.");
        if (!Guid.TryParse(request.RequestId, out _)) throw new BridgeProtocolException("INVALID_ARGUMENTS", "Request ID invalid.");
        if (!ReadOperations.Contains(request.Operation))
            throw new BridgeProtocolException(request.Operation.StartsWith("apply", StringComparison.Ordinal) ? "READ_ONLY_MODE" : "UNKNOWN_OPERATION", "Operation unavailable in R2.");
        if (request.Args.ValueKind != JsonValueKind.Object) throw new BridgeProtocolException("INVALID_ARGUMENTS", "Arguments must be an object.");
        if (request.DocumentId is { Length: > 100 } || request.SessionId is { Length: > 100 })
            throw new BridgeProtocolException("INVALID_ARGUMENTS", "Identifier too long.");
        if (request.Operation == "query_entities" && request.Args.TryGetProperty("max_results", out var max) &&
            (max.ValueKind != JsonValueKind.Number || !max.TryGetInt32(out var count) || count is < 1 or > MaxQueryEntities))
            throw new BridgeProtocolException("QUERY_LIMIT", "Entity query exceeds limit.");
        return request;
    }
    public static byte[] Encode(BridgeResponse response)
    {
        var bytes = JsonSerializer.SerializeToUtf8Bytes(response, Json);
        if (bytes.Length is < 2 or > MaxFrameBytes) throw new BridgeProtocolException("FRAME_SIZE", "Response exceeds limit.");
        return bytes;
    }
}
