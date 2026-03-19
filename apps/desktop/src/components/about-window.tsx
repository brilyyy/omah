import { useEffect, useRef } from "react";
import {
  QueryClient,
  QueryClientProvider,
  useQuery,
} from "@tanstack/react-query";
import { ipc } from "@/lib/ipc";
import { ThemeProvider, useTheme } from "@/components/theme-provider";

const qc = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, staleTime: Number.POSITIVE_INFINITY },
  },
});

export default function AboutWindow() {
  return (
    <QueryClientProvider client={qc}>
      <ThemeProvider storageKey="omah-theme">
        <AboutContent />
      </ThemeProvider>
    </QueryClientProvider>
  );
}

// ── Batik kawung animation ────────────────────────────────────────────────────
//
// Kawung is a sacred geometric batik pattern from the Javanese court tradition.
// It consists of four elliptical "petals" arranged around each grid point,
// creating an interlocking pattern that tiles infinitely.

function drawKawungCell(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  S: number,
  alpha: number,
  isDark: boolean,
) {
  const offset = S * 0.27;
  const rNarrow = S * 0.21;
  const rWide = S * 0.38;

  const lineColor = isDark
    ? `rgba(200, 150, 52, ${alpha})`
    : `rgba(95, 52, 12, ${alpha})`;
  const dotColor = isDark
    ? `rgba(225, 175, 65, ${alpha * 1.3})`
    : `rgba(115, 65, 18, ${alpha * 1.3})`;

  ctx.strokeStyle = lineColor;
  ctx.lineWidth = Math.max(0.5, S * 0.028);

  // Top petal (tall & narrow)
  ctx.beginPath();
  ctx.ellipse(cx, cy - offset, rNarrow, rWide, 0, 0, Math.PI * 2);
  ctx.stroke();

  // Bottom petal
  ctx.beginPath();
  ctx.ellipse(cx, cy + offset, rNarrow, rWide, 0, 0, Math.PI * 2);
  ctx.stroke();

  // Left petal (wide & short)
  ctx.beginPath();
  ctx.ellipse(cx - offset, cy, rWide, rNarrow, 0, 0, Math.PI * 2);
  ctx.stroke();

  // Right petal
  ctx.beginPath();
  ctx.ellipse(cx + offset, cy, rWide, rNarrow, 0, 0, Math.PI * 2);
  ctx.stroke();

  // Center accent dot
  ctx.fillStyle = dotColor;
  ctx.beginPath();
  ctx.arc(cx, cy, S * 0.048, 0, Math.PI * 2);
  ctx.fill();
}

function useBatikAnimation(
  ref: React.RefObject<HTMLCanvasElement | null>,
  isDark: boolean,
) {
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;

    const dpr = window.devicePixelRatio || 1;
    const W = canvas.offsetWidth;
    const H = canvas.offsetHeight;
    canvas.width = W * dpr;
    canvas.height = H * dpr;
    const ctx = canvas.getContext("2d")!;
    ctx.scale(dpr, dpr);

    const CELL = 48; // background tile size
    const MED_CELL = 19; // medallion tile size (denser)
    const MED_R = Math.min(W, H) * 0.305; // medallion radius

    // Gold-dust particles (rising)
    const particles = Array.from({ length: 28 }, () => ({
      x: Math.random() * W,
      y: H * Math.random(),
      vy: 0.28 + Math.random() * 0.42,
      vx: (Math.random() - 0.5) * 0.22,
      r: 0.9 + Math.random() * 2.2,
      alpha: 0.25 + Math.random() * 0.5,
    }));

    let t = 0;
    let raf: number;

    function frame() {
      ctx.clearRect(0, 0, W, H);
      t += 0.0052;

      // ── 1. Background ───────────────────────────────────────────────────
      const bg = ctx.createLinearGradient(0, 0, W, H);
      if (isDark) {
        bg.addColorStop(0, "#140a03");
        bg.addColorStop(0.45, "#1d0f05");
        bg.addColorStop(1, "#110804");
      } else {
        bg.addColorStop(0, "#f2e3c6");
        bg.addColorStop(0.5, "#eddbba");
        bg.addColorStop(1, "#e6d0a8");
      }
      ctx.fillStyle = bg;
      ctx.fillRect(0, 0, W, H);

      // ── 2. Background kawung tiling (slowly scrolling diagonally) ───────
      const sx = (t * 7) % CELL;
      const sy = (t * 5) % CELL;

      const cols = Math.ceil(W / CELL) + 3;
      const rows = Math.ceil(H / CELL) + 3;

      for (let row = -1; row < rows; row++) {
        for (let col = -1; col < cols; col++) {
          // Offset every other row by half a cell (classic kawung grid)
          const xOff = row % 2 === 0 ? 0 : CELL / 2;
          const cx = col * CELL + xOff - sx;
          const cy = row * CELL - sy;

          // Travelling diagonal shimmer wave
          const wave = 0.4 + 0.6 * Math.sin(t * 0.65 + col * 0.55 + row * 0.48);
          const baseAlpha = isDark ? 0.11 : 0.08;
          const alpha = baseAlpha + wave * (isDark ? 0.14 : 0.1);

          drawKawungCell(ctx, cx, cy, CELL, alpha, isDark);
        }
      }

      // ── 3. Warm ambient center glow ─────────────────────────────────────
      const ambGlow = ctx.createRadialGradient(
        W / 2,
        H / 2,
        0,
        W / 2,
        H / 2,
        MED_R * 2.2,
      );
      if (isDark) {
        ambGlow.addColorStop(0, "rgba(170, 105, 28, 0.28)");
        ambGlow.addColorStop(0.5, "rgba(130, 78, 18, 0.14)");
        ambGlow.addColorStop(1, "transparent");
      } else {
        ambGlow.addColorStop(0, "rgba(160, 100, 25, 0.20)");
        ambGlow.addColorStop(0.5, "rgba(130, 78, 18, 0.08)");
        ambGlow.addColorStop(1, "transparent");
      }
      ctx.fillStyle = ambGlow;
      ctx.fillRect(0, 0, W, H);

      // ── 4. 3-D rotating batik medallion ─────────────────────────────────
      // The y-scale gives it the illusion of a disk tilted ~50° toward viewer.
      ctx.save();
      ctx.translate(W / 2, H / 2);
      ctx.scale(1, 0.58); // perspective tilt — fixed depth angle
      ctx.rotate(t * 0.18); // slow steady spin

      // Clip to circle
      ctx.beginPath();
      ctx.arc(0, 0, MED_R, 0, Math.PI * 2);
      ctx.clip();

      // Tile kawung inside the medallion
      const medCols = Math.ceil((MED_R * 2) / MED_CELL) + 2;
      const medRows = Math.ceil((MED_R * 2) / MED_CELL) + 2;
      for (let row = -medRows / 2; row <= medRows / 2; row++) {
        for (let col = -medCols / 2; col <= medCols / 2; col++) {
          const xOff = row % 2 === 0 ? 0 : MED_CELL / 2;
          const cx = col * MED_CELL + xOff;
          const cy = row * MED_CELL;
          const d = Math.sqrt(cx * cx + cy * cy);
          const fade = Math.max(0, 1 - d / MED_R);
          const alpha = isDark ? 0.15 + fade * 0.55 : 0.12 + fade * 0.45;
          if (alpha > 0.05) {
            drawKawungCell(ctx, cx, cy, MED_CELL, alpha, isDark);
          }
        }
      }

      ctx.restore();

      // ── 5. Medallion outer glow ring (in same perspective space) ─────────
      ctx.save();
      ctx.translate(W / 2, H / 2);
      ctx.scale(1, 0.58);

      // Glowing halo just outside the medallion edge
      const halo = ctx.createRadialGradient(
        0,
        0,
        MED_R * 0.72,
        0,
        0,
        MED_R * 1.18,
      );
      halo.addColorStop(0, "transparent");
      halo.addColorStop(
        0.55,
        isDark ? "rgba(200, 148, 45, 0.32)" : "rgba(130, 78, 18, 0.22)",
      );
      halo.addColorStop(1, "transparent");
      ctx.fillStyle = halo;
      ctx.beginPath();
      ctx.arc(0, 0, MED_R * 1.2, 0, Math.PI * 2);
      ctx.fill();

      // Crisp outer ring stroke
      ctx.strokeStyle = isDark
        ? "rgba(200, 150, 50, 0.42)"
        : "rgba(110, 65, 18, 0.35)";
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.arc(0, 0, MED_R, 0, Math.PI * 2);
      ctx.stroke();

      // Second inner ring decoration
      ctx.strokeStyle = isDark
        ? "rgba(180, 130, 42, 0.22)"
        : "rgba(110, 65, 18, 0.18)";
      ctx.lineWidth = 0.8;
      ctx.beginPath();
      ctx.arc(0, 0, MED_R * 0.88, 0, Math.PI * 2);
      ctx.stroke();

      ctx.restore();

      // ── 6. Rising gold-dust particles ───────────────────────────────────
      for (const p of particles) {
        p.y -= p.vy;
        p.x += p.vx;
        if (p.y < -8) {
          p.y = H + 8;
          p.x = Math.random() * W;
        }
        // Fade in from bottom, fade out at top
        const lifeFrac = 1 - p.y / H;
        const alpha =
          p.alpha * Math.min(lifeFrac * 3, 1) * Math.min((1 - lifeFrac) * 5, 1);
        ctx.fillStyle = isDark
          ? `rgba(215, 165, 52, ${alpha})`
          : `rgba(125, 75, 18, ${alpha * 0.7})`;
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        ctx.fill();
      }

      raf = requestAnimationFrame(frame);
    }

    frame();
    return () => cancelAnimationFrame(raf);
  }, [isDark]);
}

// ── Content ───────────────────────────────────────────────────────────────────

function AboutContent() {
  const { data: version } = useQuery({
    queryKey: ["version"],
    queryFn: () => ipc.getVersion(),
  });

  const { theme } = useTheme();
  const isDark =
    theme === "dark" ||
    (theme === "system" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  useBatikAnimation(canvasRef, isDark);

  // Earth-tone stack badges
  const STACK = [
    {
      label: "Tauri 2",
      style: isDark
        ? {
            background: "rgba(120,60,15,0.55)",
            color: "#e8c070",
            border: "rgba(175,110,35,0.35)",
          }
        : {
            background: "rgba(160,85,20,0.18)",
            color: "#7a3c08",
            border: "rgba(140,80,20,0.3)",
          },
    },
    {
      label: "Rust",
      style: isDark
        ? {
            background: "rgba(90,35,12,0.55)",
            color: "#e09060",
            border: "rgba(160,90,35,0.35)",
          }
        : {
            background: "rgba(140,65,15,0.18)",
            color: "#6b2e08",
            border: "rgba(120,60,15,0.3)",
          },
    },
    {
      label: "React",
      style: isDark
        ? {
            background: "rgba(20,45,60,0.55)",
            color: "#7ac8e0",
            border: "rgba(40,110,145,0.35)",
          }
        : {
            background: "rgba(20,80,110,0.12)",
            color: "#0e5272",
            border: "rgba(20,80,110,0.28)",
          },
    },
    {
      label: "TypeScript",
      style: isDark
        ? {
            background: "rgba(18,42,70,0.55)",
            color: "#7aabe0",
            border: "rgba(35,90,160,0.35)",
          }
        : {
            background: "rgba(18,60,120,0.12)",
            color: "#0d3a72",
            border: "rgba(18,60,120,0.28)",
          },
    },
  ];

  const cardBg = isDark ? "rgba(18, 9, 2, 0.82)" : "rgba(242, 226, 196, 0.85)";
  const cardBorder = isDark
    ? "rgba(185, 138, 45, 0.38)"
    : "rgba(130, 78, 18, 0.32)";
  const cardShadow = isDark
    ? "0 8px 40px rgba(10,5,1,0.7), 0 0 0 1px rgba(185,138,45,0.18), inset 0 1px 0 rgba(220,175,70,0.10)"
    : "0 8px 32px rgba(80,40,8,0.18), 0 0 0 1px rgba(130,78,18,0.14), inset 0 1px 0 rgba(255,240,200,0.8)";

  const textPrimary = isDark ? "#f0ddb8" : "#2e1505";
  const textSecondary = isDark
    ? "rgba(210,175,95,0.75)"
    : "rgba(100,55,12,0.75)";
  const textFaint = isDark ? "rgba(175,140,65,0.42)" : "rgba(100,55,12,0.45)";
  const divColor = isDark ? "rgba(180,138,45,0.22)" : "rgba(130,78,18,0.2)";

  return (
    <div className="relative h-screen w-screen overflow-hidden select-none">
      {/* Batik canvas */}
      <canvas ref={canvasRef} className="absolute inset-0 h-full w-full" />

      {/* Content */}
      <div className="absolute inset-0 flex flex-col items-center justify-center gap-0 px-8">
        <div
          className="relative flex w-full max-w-[320px] flex-col items-center gap-4 rounded-2xl px-7 py-6 text-center"
          style={{
            background: cardBg,
            border: `1px solid ${cardBorder}`,
            backdropFilter: "blur(22px)",
            WebkitBackdropFilter: "blur(22px)",
            boxShadow: cardShadow,
          }}
        >
          {/* Top ornament line */}
          <div
            className="absolute left-6 right-6 top-0 h-px"
            style={{
              background: isDark
                ? "linear-gradient(90deg, transparent, rgba(210,168,58,0.55), transparent)"
                : "linear-gradient(90deg, transparent, rgba(160,95,25,0.4), transparent)",
            }}
          />

          {/* Logo mark */}
          <div className="relative flex flex-col items-center gap-1">
            {/* Pulsing outer glow ring */}
            <div
              className="absolute rounded-2xl opacity-50"
              style={{
                width: 76,
                height: 76,
                background: isDark
                  ? "radial-gradient(circle, rgba(180,120,35,0.4), transparent 70%)"
                  : "radial-gradient(circle, rgba(160,90,20,0.25), transparent 70%)",
                filter: "blur(8px)",
                animation: "pulse 3s ease-in-out infinite",
              }}
            />
            <div
              className="relative flex size-14 items-center justify-center rounded-xl"
              style={{
                background: isDark
                  ? "linear-gradient(145deg, #3d1a05 0%, #6b3510 45%, #9c5c1a 100%)"
                  : "linear-gradient(145deg, #8b4a12 0%, #a85e1c 50%, #c07828 100%)",
                boxShadow: isDark
                  ? "0 4px 20px rgba(130,70,15,0.6), inset 0 1px 0 rgba(220,170,70,0.25)"
                  : "0 4px 16px rgba(130,70,15,0.35), inset 0 1px 0 rgba(255,220,140,0.5)",
              }}
            >
              {/* Corner batik accent dots */}
              {[
                [-1, -1],
                [1, -1],
                [-1, 1],
                [1, 1],
              ].map(([dx, dy], i) => (
                <span
                  key={i}
                  className="absolute size-1 rounded-full"
                  style={{
                    top: dy === -1 ? 6 : "auto",
                    bottom: dy === 1 ? 6 : "auto",
                    left: dx === -1 ? 6 : "auto",
                    right: dx === 1 ? 6 : "auto",
                    background: isDark
                      ? "rgba(220,175,70,0.5)"
                      : "rgba(255,220,140,0.6)",
                  }}
                />
              ))}
              <span
                className="font-bold tracking-tighter text-white/90"
                style={{ fontSize: "1.15rem", letterSpacing: "-0.06em" }}
              >
                om
              </span>
            </div>
          </div>

          {/* Title */}
          <div className="space-y-1">
            <h1
              className="font-bold tracking-tighter"
              style={{
                fontSize: "2rem",
                color: textPrimary,
                letterSpacing: "-0.06em",
                textShadow: isDark
                  ? "0 2px 12px rgba(210,165,55,0.25)"
                  : "none",
              }}
            >
              ꦲꦺꦴꦩꦃ
            </h1>

            {/* Javanese script + meaning */}
            <div className="flex items-center justify-center gap-2 mt-4">
              <div style={{ width: 24, height: "1px", background: divColor }} />
              <p
                className="text-[11px] tracking-widest uppercase"
                style={{ color: textSecondary, letterSpacing: "0.18em" }}
              >
                omah (home in Javanese)
              </p>
              <div style={{ width: 24, height: "1px", background: divColor }} />
            </div>

            {version && (
              <div className="flex justify-center pt-0.5">
                <span
                  className="rounded-full px-2.5 py-0.5 font-mono text-[11px] font-medium"
                  style={{
                    background: isDark
                      ? "rgba(155,100,25,0.30)"
                      : "rgba(140,80,20,0.15)",
                    color: isDark ? "#ddb84a" : "#7a3c08",
                    border: `1px solid ${isDark ? "rgba(185,138,45,0.38)" : "rgba(140,80,20,0.28)"}`,
                  }}
                >
                  v{version}
                </span>
              </div>
            )}
          </div>

          {/* Thin divider */}
          <div
            className="w-full"
            style={{
              height: "1px",
              background: isDark
                ? "linear-gradient(90deg, transparent, rgba(185,138,45,0.3), transparent)"
                : "linear-gradient(90deg, transparent, rgba(130,78,18,0.25), transparent)",
            }}
          />

          {/* Tagline */}
          <p className="text-sm leading-snug" style={{ color: textSecondary }}>
            A dotfile manager built for the{" "}
            <span
              style={{
                color: isDark ? "#d4a83c" : "#8b4a12",
                fontStyle: "italic",
              }}
            >
              wandering developer
            </span>
            .
          </p>

          {/* Tech stack */}
          <div className="flex flex-wrap justify-center gap-1.5">
            {STACK.map(({ label, style }) => (
              <span
                key={label}
                className="rounded-full px-2 py-0.5 text-[11px] font-medium"
                style={{
                  background: style.background,
                  color: style.color,
                  border: `1px solid ${style.border}`,
                }}
              >
                {label}
              </span>
            ))}
          </div>

          {/* Thin divider */}
          <div
            className="w-full"
            style={{
              height: "1px",
              background: isDark
                ? "linear-gradient(90deg, transparent, rgba(185,138,45,0.2), transparent)"
                : "linear-gradient(90deg, transparent, rgba(130,78,18,0.18), transparent)",
            }}
          />

          {/* Copyright */}
          <p className="text-[11px]" style={{ color: textFaint }}>
            © {new Date().getFullYear()} brilyyy · MIT License
          </p>

          {/* Bottom ornament line */}
          <div
            className="absolute bottom-0 left-6 right-6 h-px"
            style={{
              background: isDark
                ? "linear-gradient(90deg, transparent, rgba(210,168,58,0.35), transparent)"
                : "linear-gradient(90deg, transparent, rgba(160,95,25,0.28), transparent)",
            }}
          />
        </div>
      </div>
    </div>
  );
}
