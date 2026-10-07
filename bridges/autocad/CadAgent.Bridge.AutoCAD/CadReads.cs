using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Autodesk.AutoCAD.ApplicationServices;
using Autodesk.AutoCAD.DatabaseServices;
using Autodesk.AutoCAD.EditorInput;
using Autodesk.AutoCAD.Geometry;
using CadAgent.Bridge.Protocol;
using AcApplication = Autodesk.AutoCAD.ApplicationServices.Application;

namespace CadAgent.Bridge.AutoCAD;

// Created and called only on AutoCAD's main/UI thread from Application.Idle.
internal sealed class CadReads
{
    private sealed class Stamp(Document document)
    {
        public string Id { get; } = Guid.NewGuid().ToString("N");
        public long Revision;
        public Document Document { get; } = document;
    }
    private readonly Dictionary<Document, Stamp> _stamps = [];
    private Stamp Track(Document document)
    {
        if (_stamps.TryGetValue(document, out var stamp)) return stamp;
        stamp = new Stamp(document);
        _stamps.Add(document, stamp);
        document.Database.ObjectAppended += (_, _) => stamp.Revision++;
        document.Database.ObjectModified += (_, _) => stamp.Revision++;
        document.Database.ObjectErased += (_, _) => stamp.Revision++;
        return stamp;
    }
    private static IEnumerable<Document> Documents()
    {
        foreach (Document document in AcApplication.DocumentManager) yield return document;
    }
    private (Document Doc, Stamp Stamp) Require(BridgeRequest request)
    {
        var doc = Documents().FirstOrDefault(d => Track(d).Id == request.DocumentId);
        if (doc is null) throw new BridgeProtocolException("DOCUMENT_CHANGED", "Select the current drawing again.");
        return (doc, Track(doc));
    }
    private static string Short(string? value, int max = 160) => value is null ? "" : value.Length <= max ? value : value[..max];
    private static object Describe(Document doc, Stamp stamp, string session)
    {
        var db = doc.Database;
        return new
        {
            document_id = stamp.Id,
            session_id = session,
            process_id = Environment.ProcessId,
            file_name = Short(Path.GetFileName(doc.Name)),
            saved = !string.IsNullOrWhiteSpace(doc.Name) && File.Exists(doc.Name),
            active = ReferenceEquals(AcApplication.DocumentManager.MdiActiveDocument, doc),
            space = db.TileMode ? "model" : "paper",
            units = db.Insunits.ToString(),
            ucs = "not_observed",
            dirty = "unknown",
            revision = stamp.Revision.ToString(System.Globalization.CultureInfo.InvariantCulture),
            lifecycle = "open"
        };
    }
    public object Execute(BridgeRequest request, string session)
    {
        if (request.Operation == "get_status")
        {
            var active = AcApplication.DocumentManager.MdiActiveDocument;
            return new { connected = true, mode = "autocad", process_id = Environment.ProcessId, session_id = session,
                active_document_id = active is null ? null : Track(active).Id, write = false };
        }
        if (request.Operation == "list_documents")
            return new { documents = Documents().Take(64).Select(d => Describe(d, Track(d), session)).ToArray(), truncated = Documents().Skip(64).Any() };
        if (request.Operation == "get_operation_status")
            return new { status = "READ_ONLY_MODE", write = false };
        var (doc, stamp) = Require(request);
        using var documentLock = doc.LockDocument();
        using var transaction = doc.Database.TransactionManager.StartTransaction();
        object result = request.Operation switch
        {
            "get_document_context" => Describe(doc, stamp, session),
            "list_layers" => Layers(doc.Database, transaction, stamp),
            "get_selection" => Selection(doc, transaction, stamp),
            "query_entities" => Query(doc, transaction, stamp, request.Args),
            "validate_entities" => Validate(doc, transaction, stamp, request.Args),
            _ => throw new BridgeProtocolException("READ_ONLY_MODE", "Operation unavailable in read-only bridge.")
        };
        return result; // Dispose transaction and document lock before pipe worker resumes.
    }
    private static object Layers(Database db, Transaction tr, Stamp stamp)
    {
        var table = (LayerTable)tr.GetObject(db.LayerTableId, OpenMode.ForRead);
        var layers = new List<object>();
        var truncated = false;
        foreach (ObjectId id in table)
        {
            if (layers.Count >= 5000) { truncated = true; break; }
            var layer = (LayerTableRecord)tr.GetObject(id, OpenMode.ForRead);
            layers.Add(new { name = Short(layer.Name), off = layer.IsOff, frozen = layer.IsFrozen, locked = layer.IsLocked });
        }
        return new { document_id = stamp.Id, revision = stamp.Revision.ToString(), layers, count = layers.Count, truncated };
    }
    private static object Selection(Document doc, Transaction tr, Stamp stamp)
    {
        var selected = doc.Editor.SelectImplied();
        if (selected.Status != PromptStatus.OK || selected.Value is null)
            return new { document_id = stamp.Id, revision = stamp.Revision.ToString(), count = 0, entities = Array.Empty<object>(), truncated = false };
        var count = selected.Value.Count;
        var entities = new List<object>();
        foreach (var id in selected.Value.GetObjectIds().Take(100))
        {
            if (tr.GetObject(id, OpenMode.ForRead, false) is Entity entity) entities.Add(Summary(entity));
        }
        return new { document_id = stamp.Id, revision = stamp.Revision.ToString(), count, entities, truncated = count > 100 };
    }
    private static object Query(Document doc, Transaction tr, Stamp stamp, JsonElement args)
    {
        var limit = args.TryGetProperty("max_results", out var m) && m.TryGetInt32(out var n) ? n : 50;
        if (limit is < 1 or > BridgeProtocol.MaxQueryEntities) throw new BridgeProtocolException("QUERY_LIMIT", "Maximum 100 entities.");
        var layer = args.TryGetProperty("layer", out var l) && l.ValueKind == JsonValueKind.String ? l.GetString() : null;
        var type = args.TryGetProperty("entity_type", out var t) && t.ValueKind == JsonValueKind.String ? t.GetString() : null;
        var space = (BlockTableRecord)tr.GetObject(doc.Database.CurrentSpaceId, OpenMode.ForRead);
        var results = new List<object>();
        var scanned = 0;
        var truncated = false;
        foreach (ObjectId id in space)
        {
            if (++scanned > 10000) { truncated = true; break; }
            if (tr.GetObject(id, OpenMode.ForRead, false) is not Entity entity) continue;
            if (layer is not null && !string.Equals(entity.Layer, layer, StringComparison.OrdinalIgnoreCase)) continue;
            if (type is not null && !string.Equals(entity.GetType().Name, type, StringComparison.OrdinalIgnoreCase)) continue;
            if (results.Count >= limit) { truncated = true; break; }
            results.Add(Summary(entity));
        }
        return new { document_id = stamp.Id, revision = stamp.Revision.ToString(), entities = results, count = results.Count, scanned, truncated };
    }
    private static object Validate(Document doc, Transaction tr, Stamp stamp, JsonElement args)
    {
        if (!args.TryGetProperty("handles", out var handles) || handles.ValueKind != JsonValueKind.Array || handles.GetArrayLength() > 100)
            throw new BridgeProtocolException("INVALID_ARGUMENTS", "Up to 100 handles required.");
        var results = new List<object>();
        foreach (var handle in handles.EnumerateArray())
        {
            var text = handle.GetString();
            if (text is null || text.Length is < 1 or > 16 || !long.TryParse(text, System.Globalization.NumberStyles.HexNumber, null, out var raw))
                throw new BridgeProtocolException("INVALID_ARGUMENTS", "Handle format invalid.");
            try
            {
                var id = doc.Database.GetObjectId(false, new Handle(raw), 0);
                results.Add(new { handle = text, exists = tr.GetObject(id, OpenMode.ForRead, false) is Entity });
            }
            catch (Autodesk.AutoCAD.Runtime.Exception) { results.Add(new { handle = text, exists = false }); }
        }
        return new { document_id = stamp.Id, revision = stamp.Revision.ToString(), entities = results };
    }
    private static object Summary(Entity entity)
    {
        var type = entity.GetType().Name;
        object? geometry = entity switch
        {
            Line line => new { start = Point(line.StartPoint), end = Point(line.EndPoint) },
            Polyline poly => new { vertices = Enumerable.Range(0, Math.Min(poly.NumberOfVertices, 100)).Select(i => Point(poly.GetPoint3dAt(i))).ToArray(), closed = poly.Closed, truncated = poly.NumberOfVertices > 100 },
            Circle circle => new { center = Point(circle.Center), radius = circle.Radius },
            Arc arc => new { center = Point(arc.Center), radius = arc.Radius, start_angle = arc.StartAngle, end_angle = arc.EndAngle },
            DBText text => new { text = Short(text.TextString, 256), position = Point(text.Position) },
            MText text => new { text = Short(text.Text, 256), position = Point(text.Location) },
            BlockReference block => new { name = Short(block.Name), position = Point(block.Position) },
            Dimension dimension => new { text = Short(dimension.DimensionText, 256) },
            _ => null
        };
        return new { handle = entity.Handle.ToString(), type, layer = Short(entity.Layer), unsupported = geometry is null, geometry };
    }
    private static double[] Point(Point3d point) => [point.X, point.Y, point.Z];
}
