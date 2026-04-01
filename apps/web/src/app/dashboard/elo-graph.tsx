"use client";

import { useState, useEffect, useRef } from "react";

interface DataPoint {
  date: string;
  rating: number;
  label?: string;
}

// Placeholder progression data — will be replaced with real session data
const SAMPLE_DATA: DataPoint[] = [
  { date: "Jan 1", rating: 1000, label: "Started" },
  { date: "Jan 8", rating: 1020 },
  { date: "Jan 15", rating: 1005 },
  { date: "Jan 22", rating: 1060 },
  { date: "Jan 29", rating: 1045 },
  { date: "Feb 5", rating: 1110 },
  { date: "Feb 12", rating: 1090 },
  { date: "Feb 19", rating: 1150 },
  { date: "Feb 26", rating: 1135 },
  { date: "Mar 5", rating: 1180 },
  { date: "Mar 12", rating: 1200, label: "Current" },
];

const CHART_HEIGHT = 180;
const CHART_WIDTH = 600;
const PADDING = { top: 20, right: 20, bottom: 30, left: 45 };

function useAnimatedProgress(duration = 1400) {
  const [progress, setProgress] = useState(0);
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;

    const start = performance.now();
    let raf: number;

    function tick(now: number) {
      const elapsed = now - start;
      const p = Math.min(elapsed / duration, 1);
      // ease-out cubic
      const eased = 1 - Math.pow(1 - p, 3);
      setProgress(eased);
      if (p < 1) {
        raf = requestAnimationFrame(tick);
      }
    }

    // Small delay so the component fades in first
    const timeout = setTimeout(() => {
      raf = requestAnimationFrame(tick);
    }, 300);

    return () => {
      clearTimeout(timeout);
      cancelAnimationFrame(raf);
    };
  }, [duration]);

  return progress;
}

export function EloGraph({ data = SAMPLE_DATA }: { data?: DataPoint[] }) {
  const progress = useAnimatedProgress();
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);

  const innerWidth = CHART_WIDTH - PADDING.left - PADDING.right;
  const innerHeight = CHART_HEIGHT - PADDING.top - PADDING.bottom;

  const ratings = data.map((d) => d.rating);
  const minRating = Math.floor((Math.min(...ratings) - 30) / 50) * 50;
  const maxRating = Math.ceil((Math.max(...ratings) + 30) / 50) * 50;

  // Scale functions
  const xScale = (i: number) =>
    PADDING.left + (i / (data.length - 1)) * innerWidth;
  const yScale = (rating: number) =>
    PADDING.top +
    innerHeight -
    ((rating - minRating) / (maxRating - minRating)) * innerHeight;

  // Build the line path
  const visibleCount = Math.floor(progress * (data.length - 1)) + 1;
  const partialProgress =
    progress * (data.length - 1) - Math.floor(progress * (data.length - 1));

  let pathD = "";
  for (let i = 0; i < Math.min(visibleCount, data.length); i++) {
    const x = xScale(i);
    const y = yScale(data[i].rating);
    if (i === 0) {
      pathD += `M ${x} ${y}`;
    } else {
      pathD += ` L ${x} ${y}`;
    }
  }

  // Add partial segment for smooth animation
  if (
    visibleCount < data.length &&
    partialProgress > 0 &&
    visibleCount > 0
  ) {
    const prevX = xScale(visibleCount - 1);
    const prevY = yScale(data[visibleCount - 1].rating);
    const nextX = xScale(visibleCount);
    const nextY = yScale(data[visibleCount].rating);
    const interpX = prevX + (nextX - prevX) * partialProgress;
    const interpY = prevY + (nextY - prevY) * partialProgress;
    pathD += ` L ${interpX} ${interpY}`;
  }

  // Area path (for gradient fill under the line)
  const areaD =
    pathD +
    ` L ${xScale(Math.min(visibleCount - 1, data.length - 1))} ${PADDING.top + innerHeight} L ${xScale(0)} ${PADDING.top + innerHeight} Z`;

  // Y-axis tick values
  const yTicks: number[] = [];
  for (let v = minRating; v <= maxRating; v += 50) {
    yTicks.push(v);
  }

  // Current rating change
  const firstRating = data[0].rating;
  const lastRating = data[data.length - 1].rating;
  const ratingChange = lastRating - firstRating;

  const handleMouseMove = (e: React.MouseEvent<SVGSVGElement>) => {
    if (!svgRef.current) return;
    const rect = svgRef.current.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const scaleFactor = CHART_WIDTH / rect.width;
    const scaledX = mouseX * scaleFactor;

    let closest = 0;
    let closestDist = Infinity;
    for (let i = 0; i < data.length; i++) {
      const dist = Math.abs(xScale(i) - scaledX);
      if (dist < closestDist) {
        closestDist = dist;
        closest = i;
      }
    }
    setHoveredIndex(closestDist < 30 ? closest : null);
  };

  return (
    <div>
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <div className="w-3 h-[2px] bg-[#00e5a0] rounded-full" />
            <span className="font-mono text-[10px] text-[#64748b]">
              Elo Rating
            </span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <span className="font-mono text-[10px] text-[#64748b]">
            Change:
          </span>
          <span
            className={`font-mono text-xs font-bold ${ratingChange >= 0 ? "text-[#00e5a0]" : "text-[#ef4444]"}`}
          >
            {ratingChange >= 0 ? "+" : ""}
            {ratingChange}
          </span>
        </div>
      </div>

      {/* Chart */}
      <div className="bg-[#060b14] border border-[#1e293b] rounded-sm p-3 overflow-hidden">
        <svg
          ref={svgRef}
          viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
          className="w-full h-auto"
          onMouseMove={handleMouseMove}
          onMouseLeave={() => setHoveredIndex(null)}
        >
          <defs>
            {/* Gradient fill under the line */}
            <linearGradient
              id="areaGradient"
              x1="0"
              y1="0"
              x2="0"
              y2="1"
            >
              <stop offset="0%" stopColor="#00e5a0" stopOpacity="0.15" />
              <stop offset="100%" stopColor="#00e5a0" stopOpacity="0" />
            </linearGradient>

            {/* Glow filter for the line */}
            <filter id="lineGlow" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="3" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>

            {/* Dot glow */}
            <filter id="dotGlow" x="-100%" y="-100%" width="300%" height="300%">
              <feGaussianBlur stdDeviation="4" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {/* Grid lines */}
          {yTicks.map((tick) => (
            <g key={tick}>
              <line
                x1={PADDING.left}
                y1={yScale(tick)}
                x2={PADDING.left + innerWidth}
                y2={yScale(tick)}
                stroke="#1e293b"
                strokeWidth="0.5"
                strokeDasharray="3 3"
              />
              <text
                x={PADDING.left - 8}
                y={yScale(tick) + 3}
                textAnchor="end"
                fill="#334155"
                fontSize="9"
                fontFamily="IBM Plex Mono, monospace"
              >
                {tick}
              </text>
            </g>
          ))}

          {/* X-axis labels */}
          {data.map((d, i) => {
            // Only show every other label to avoid crowding
            if (i % 2 !== 0 && i !== data.length - 1) return null;
            return (
              <text
                key={i}
                x={xScale(i)}
                y={PADDING.top + innerHeight + 18}
                textAnchor="middle"
                fill="#334155"
                fontSize="8"
                fontFamily="IBM Plex Mono, monospace"
              >
                {d.date}
              </text>
            );
          })}

          {/* Area fill */}
          {pathD && (
            <path d={areaD} fill="url(#areaGradient)" />
          )}

          {/* Main line */}
          {pathD && (
            <path
              d={pathD}
              fill="none"
              stroke="#00e5a0"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              filter="url(#lineGlow)"
            />
          )}

          {/* Data points */}
          {data.map((d, i) => {
            if (i >= visibleCount) return null;
            const x = xScale(i);
            const y = yScale(d.rating);
            const isHovered = hoveredIndex === i;
            const isLast = i === data.length - 1 && progress >= 0.99;

            return (
              <g key={i}>
                {/* Hover/last point highlight */}
                {(isHovered || isLast) && (
                  <>
                    {/* Vertical guide line */}
                    <line
                      x1={x}
                      y1={PADDING.top}
                      x2={x}
                      y2={PADDING.top + innerHeight}
                      stroke={isLast ? "#00e5a0" : "#ffffff"}
                      strokeWidth="0.5"
                      strokeDasharray="2 2"
                      opacity={0.3}
                    />
                    {/* Outer glow ring */}
                    <circle
                      cx={x}
                      cy={y}
                      r={isLast ? 6 : 5}
                      fill="#00e5a0"
                      opacity={0.15}
                      filter="url(#dotGlow)"
                    />
                  </>
                )}

                {/* Dot */}
                <circle
                  cx={x}
                  cy={y}
                  r={isHovered ? 4 : isLast ? 3.5 : 2}
                  fill={isLast ? "#00e5a0" : "#0a0f1a"}
                  stroke="#00e5a0"
                  strokeWidth={isHovered || isLast ? 2 : 1.5}
                  style={{
                    transition: "r 0.15s ease-out",
                  }}
                />

                {/* Tooltip */}
                {isHovered && (
                  <g>
                    <rect
                      x={x - 36}
                      y={y - 32}
                      width={72}
                      height={22}
                      rx={2}
                      fill="#111827"
                      stroke="#1e293b"
                      strokeWidth="0.5"
                    />
                    <text
                      x={x}
                      y={y - 18}
                      textAnchor="middle"
                      fill="#e2e8f0"
                      fontSize="10"
                      fontFamily="IBM Plex Mono, monospace"
                      fontWeight="bold"
                    >
                      {d.rating}
                    </text>
                    <text
                      x={x}
                      y={y - 38}
                      textAnchor="middle"
                      fill="#64748b"
                      fontSize="8"
                      fontFamily="IBM Plex Mono, monospace"
                    >
                      {d.date}
                    </text>
                  </g>
                )}
              </g>
            );
          })}
        </svg>
      </div>

      {/* Bottom stats row */}
      <div className="flex items-center justify-between mt-3">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-[#00e5a0]" />
            <span className="font-mono text-[10px] text-[#64748b]">
              Peak: {Math.max(...ratings)}
            </span>
          </div>
          <div className="flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-[#334155]" />
            <span className="font-mono text-[10px] text-[#64748b]">
              Low: {Math.min(...ratings)}
            </span>
          </div>
        </div>
        <span className="font-mono text-[10px] text-[#334155]">
          {data.length} sessions
        </span>
      </div>
    </div>
  );
}
