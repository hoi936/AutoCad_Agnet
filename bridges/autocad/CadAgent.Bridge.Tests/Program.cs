using System.Text;
using System.Text.Json;
using CadAgent.Bridge.Protocol;
using CadAgent.Bridge.Transport;

static void Check(bool condition, string name)
{
    if (!condition) throw new Exception(name);
    Console.WriteLine($"PASS {name}");
}
static void Reject(string json, string code)
{
    try { BridgeProtocol.ParseRequest(Encoding.UTF8.GetBytes(json)); }
    catch (BridgeProtocolException e) { Check(e.Code == code, code); return; }
    throw new Exception($"Expected {code}");
}
var rid = Guid.NewGuid();
var good = $"{{\"version\":1,\"request_id\":\"{rid}\",\"operation\":\"list_documents\",\"args\":{{}}}}";
Check(BridgeProtocol.ParseRequest(Encoding.UTF8.GetBytes(good)).Operation == "list_documents", "parse_read");
Reject(good.Replace("list_documents", "apply_change_set"), "READ_ONLY_MODE");
Reject(good.Replace("\"version\":1", "\"version\":2"), "PROTOCOL_MISMATCH");
Reject(good.Replace("\"args\":{}", "\"args\":{},\"args\":{}"), "INVALID_ARGUMENTS");
Reject(good.Replace("\"args\":{}", "\"args\":{},\"unexpected\":1"), "INVALID_ARGUMENTS");
Reject(good.Replace("list_documents", "query_entities").Replace("\"args\":{}", "\"args\":{\"max_results\":101}"), "QUERY_LIMIT");
var unicode = "Bản vẽ thử nghiệm 漢字";
var response = BridgeResponse.Success(BridgeProtocol.ParseRequest(Encoding.UTF8.GetBytes(good)), "session", new { file_name = unicode });
using var stream = new MemoryStream();
await BridgeFrames.WriteAsync(stream, BridgeProtocol.Encode(response), CancellationToken.None);
stream.Position = 0;
var read = await BridgeFrames.ReadAsync(stream, CancellationToken.None);
using (var decoded = JsonDocument.Parse(read))
    Check(decoded.RootElement.GetProperty("result").GetProperty("file_name").GetString() == unicode, "unicode_frame_roundtrip");
stream.SetLength(0);
stream.Write([0xff, 0xff, 0xff, 0x7f]);
stream.Position = 0;
try { await BridgeFrames.ReadAsync(stream, CancellationToken.None); throw new Exception("Expected FRAME_SIZE"); }
catch (BridgeProtocolException e) { Check(e.Code == "FRAME_SIZE", "oversized_frame"); }
