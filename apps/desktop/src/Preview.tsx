import type { Geometry } from "./contracts";
export function Preview({ geometry }: { geometry: Geometry | null }) {
  if (!geometry)
    return (
      <div className="empty">
        <span className="crosshair">＋</span>
        <p>Chưa có hình học</p>
        <small>Lập phương án để xem trước tấm đế.</small>
      </div>
    );
  const p = geometry.parameters;
  const [x, y] = p.origin;
  const pad = Math.max(p.width, p.height) * 0.18;
  return (
    <svg
      role="img"
      aria-label="Preview hình học tấm đế theo mm"
      viewBox={`${x - pad} ${-y - p.height - pad} ${p.width + pad * 2} ${p.height + pad * 2}`}
    >
      <g transform="scale(1,-1)">
        {geometry.entities.map((e) =>
          e.kind === "polyline" ? (
            <polygon
              key={e.id}
              points={e.points.map((v) => v.join(",")).join(" ")}
              fill="var(--plate-fill)"
              stroke="var(--accent)"
              strokeWidth="1"
            />
          ) : e.kind === "circle" ? (
            <circle
              key={e.id}
              cx={e.center[0]}
              cy={e.center[1]}
              r={e.radius}
              fill="var(--panel)"
              stroke="var(--accent)"
              strokeWidth="0.8"
            />
          ) : null,
        )}
        <path
          d={`M ${x - 5} ${y} h 10 M ${x} ${y - 5} v 10`}
          stroke="var(--muted)"
          strokeWidth="0.5"
        />
      </g>
      <g
        fill="var(--muted)"
        fontSize={Math.max(p.width, p.height) / 28}
        textAnchor="middle"
      >
        <text x={x + p.width / 2} y={-y - p.height - pad / 2}>
          {p.width} mm
        </text>
        <text x={x + p.width + pad / 2} y={-y - p.height / 2}>
          {p.height}
        </text>
        <text x={x + p.width / 2} y={-y + pad / 1.5}>
          4 × Ø{p.hole_diameter} · mép {p.edge_offset} mm
        </text>
      </g>
    </svg>
  );
}
