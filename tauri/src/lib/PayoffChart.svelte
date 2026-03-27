<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let trade: any;

  const dispatch = createEventDispatcher();

  // ── Types ──────────────────────────────────────────────────────────────────

  type Leg = {
    type: "short_put" | "long_put" | "short_call" | "long_call";
    strike: number;
    premium: number;
    close_premium?: number;
  };

  // ── Calculations (mirror of src/calculations.rs) ──────────────────────────

  function payoffAt(legs: Leg[], credit: number, price: number): number {
    const intrinsic = legs.reduce((sum, leg) => {
      switch (leg.type) {
        case "short_put":  return sum - Math.max(leg.strike - price, 0);
        case "long_put":   return sum + Math.max(leg.strike - price, 0);
        case "short_call": return sum - Math.max(price - leg.strike, 0);
        case "long_call":  return sum + Math.max(price - leg.strike, 0);
        default:           return sum;
      }
    }, 0);
    return (credit + intrinsic) * 100;
  }

  function breakevens(legs: Leg[], credit: number, spreadType: string): number[] {
    const puts  = legs.filter((l) => l.type === "short_put"  || l.type === "long_put");
    const calls = legs.filter((l) => l.type === "short_call" || l.type === "long_call");
    const shortPut  = puts.find((l)  => l.type === "short_put");
    const shortCall = calls.find((l) => l.type === "short_call");
    const bes: number[] = [];
    if (shortPut)  bes.push(shortPut.strike  - Math.abs(credit));
    if (shortCall) bes.push(shortCall.strike + Math.abs(credit));
    return bes;
  }

  function maxProfit(credit: number, qty: number): number {
    return credit > 0 ? credit * 100 * qty : 0;
  }

  function maxLoss(legs: Leg[], credit: number, qty: number, spreadType: string): number {
    const strikes = legs.map((l) => l.strike).filter((s) => s > 0);
    if (strikes.length < 2) return Math.abs(credit) * 100 * qty;
    const minS = Math.min(...strikes);
    const maxS = Math.max(...strikes);
    const width = maxS - minS;
    if (spreadType.includes("strangle") || spreadType.includes("straddle")) return Infinity;
    return (width - Math.abs(credit)) * 100 * qty;
  }

  // ── Derived data ───────────────────────────────────────────────────────────

  const legs: Leg[] = trade.legs || [];
  const credit: number = trade.credit_received ?? 0;
  const qty: number = trade.quantity ?? 1;
  const ticker: string = trade.ticker ?? "";
  const strategy: string = (trade.strategy ?? "").replace(/_/g, " ").toUpperCase();
  const spot: number | null = trade.underlying_price ?? null;
  const pop: number | null = trade.pop ?? null;
  const spreadType: string = trade.strategy ?? "";
  const delta: number | null = trade.delta ?? null;
  const theta: number | null = trade.theta ?? null;
  const vega: number | null = trade.vega ?? null;
  const gamma: number | null = trade.gamma ?? null;
  const iv: number | null = trade.implied_volatility ?? null;
  const expirationDate: string | null = trade.expiration_date ?? null;

  const strikes = legs.map((l) => l.strike).filter((s) => s > 0);
  const minS = strikes.length ? Math.min(...strikes) : (spot ?? 100);
  const maxS = strikes.length ? Math.max(...strikes) : (spot ?? 100);
  const range = maxS - minS < 0.01 ? minS * 0.2 : 0;
  const rawPriceMin = (minS - (range || (maxS - minS) * 0.5)) * 0.94;
  const rawPriceMax = (maxS + (range || (maxS - minS) * 0.5)) * 1.06;
  // Always include spot in the visible range (with a small buffer)
  const priceMin = spot !== null ? Math.min(rawPriceMin, spot * 0.97) : rawPriceMin;
  const priceMax = spot !== null ? Math.max(rawPriceMax, spot * 1.03) : rawPriceMax;

  const N_POINTS = 300;
  const prices = Array.from({ length: N_POINTS }, (_, i) =>
    priceMin + ((priceMax - priceMin) * i) / (N_POINTS - 1)
  );
  const pnls = prices.map((p) => payoffAt(legs, credit, p));

  const maxPnl = Math.max(...pnls);
  const minPnl = Math.min(...pnls);
  const pnlRange = Math.max(Math.abs(maxPnl), Math.abs(minPnl)) * 1.15 || 100;

  const maxPft = maxProfit(credit, qty);
  const maxLss = maxLoss(legs, credit, qty, spreadType);
  const bes = breakevens(legs, credit, spreadType);

  // ── Expected move (±1σ) ────────────────────────────────────────────────────
  const ivDec: number | null = iv !== null ? (iv > 2 ? iv / 100 : iv) : null;
  const dteDays: number | null = (() => {
    if (!expirationDate) return null;
    const msLeft = new Date(expirationDate).getTime() - Date.now();
    if (msLeft <= 0) {
      // Trade has expired — use entry_dte so SD lines still render for closed trades
      const dte = trade.entry_dte ?? trade.dte ?? null;
      return typeof dte === "number" && dte > 0 ? dte : null;
    }
    return msLeft / 86_400_000;
  })();
  const expectedMove: number | null =
    spot !== null && ivDec !== null && dteDays !== null && dteDays > 0
      ? spot * ivDec * Math.sqrt(dteDays / 365)
      : null;
  const sd1Lo: number | null = expectedMove !== null && spot !== null ? spot - expectedMove : null;
  const sd1Hi: number | null = expectedMove !== null && spot !== null ? spot + expectedMove : null;

  // ── SVG dimensions ─────────────────────────────────────────────────────────

  const W = 700;
  const H = 420;
  const PAD_L = 70;
  const PAD_R = 20;
  const PAD_T = 60;
  const PAD_B = 50;
  const CHART_W = W - PAD_L - PAD_R;
  const CHART_H = H - PAD_T - PAD_B;

  function xScale(price: number): number {
    return PAD_L + ((price - priceMin) / (priceMax - priceMin)) * CHART_W;
  }

  function yScale(pnl: number): number {
    return PAD_T + CHART_H / 2 - (pnl / pnlRange) * (CHART_H / 2);
  }

  const zeroY = yScale(0);

  // Build SVG path
  const pathD = pnls
    .map((pnl, i) => `${i === 0 ? "M" : "L"}${xScale(prices[i]).toFixed(1)},${yScale(pnl).toFixed(1)}`)
    .join(" ");

  // Profit fill (above zero line)
  const profitFill =
    `M${xScale(priceMin).toFixed(1)},${zeroY.toFixed(1)} ` +
    pnls.map((pnl, i) => `L${xScale(prices[i]).toFixed(1)},${yScale(Math.max(pnl, 0)).toFixed(1)}`).join(" ") +
    ` L${xScale(priceMax).toFixed(1)},${zeroY.toFixed(1)} Z`;

  // Loss fill (below zero line)
  const lossFill =
    `M${xScale(priceMin).toFixed(1)},${zeroY.toFixed(1)} ` +
    pnls.map((pnl, i) => `L${xScale(prices[i]).toFixed(1)},${yScale(Math.min(pnl, 0)).toFixed(1)}`).join(" ") +
    ` L${xScale(priceMax).toFixed(1)},${zeroY.toFixed(1)} Z`;

  // Greeks direction labels
  function greekDir(val: number | null, isTheta = false): string {
    if (val === null) return "N/A";
    if (isTheta) return val < 0 ? "Short" : "Long";
    if (Math.abs(val) < 0.05) return "Flat";
    return val > 0 ? "Long" : "Short";
  }

  function fmtMoney(v: number): string {
    if (!isFinite(v)) return "Unlimited";
    return `$${Math.abs(v).toLocaleString("en-US", { maximumFractionDigits: 0 })}`;
  }

  function legBadge(leg: Leg): string {
    switch (leg.type) {
      case "short_put":  return "SP";
      case "long_put":   return "LP";
      case "short_call": return "SC";
      case "long_call":  return "LC";
    }
  }

  function legColor(leg: Leg): string {
    if (leg.type.includes("put"))  return "#ef4444";
    if (leg.type.includes("call")) return "#3b82f6";
    return "#9ca3af";
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="chart-card"
  on:keydown={(e) => e.key === "Escape" && dispatch("close")}
  tabindex="-1"
>
  <!-- Header -->
  <div class="header">
    <div class="strategy-name">{strategy}</div>
    <div class="header-meta">
      <span class="ticker">{ticker}</span>
      {#if pop !== null}
        <span class="meta-item">POP: <b>{pop.toFixed(0)}%</b></span>
      {/if}
      {#if spot !== null}
        <span class="meta-item">Spot: <b>${spot.toFixed(2)}</b></span>
      {/if}
      <button class="close-btn" on:click={() => dispatch("close")} aria-label="Close">✕</button>
    </div>
  </div>

  <!-- Chart + Sidebar -->
  <div class="body">
    <!-- SVG Chart -->
    <div class="chart-area">
      <svg viewBox="0 0 {W} {H}" width="100%" height="auto">
        <!-- Defs: hatched loss pattern -->
        <defs>
          <pattern id="hatch-loss" patternUnits="userSpaceOnUse" width="8" height="8" patternTransform="rotate(45)">
            <line x1="0" y1="0" x2="0" y2="8" stroke="#ef4444" stroke-width="1.5" stroke-opacity="0.35" />
          </pattern>
          <pattern id="hatch-profit" patternUnits="userSpaceOnUse" width="8" height="8">
            <rect width="8" height="8" fill="rgba(34,197,94,0.12)" />
          </pattern>
          <clipPath id="chart-clip">
            <rect x={PAD_L} y={PAD_T} width={CHART_W} height={CHART_H} />
          </clipPath>
        </defs>

        <!-- Filled zones -->
        <g clip-path="url(#chart-clip)">
          <path d={profitFill} fill="url(#hatch-profit)" />
          <path d={lossFill}   fill="url(#hatch-loss)" />
        </g>

        <!-- Zero line -->
        <line
          x1={PAD_L} y1={zeroY}
          x2={PAD_L + CHART_W} y2={zeroY}
          stroke="#4b5563" stroke-width="1.5" stroke-dasharray="4 3"
        />

        <!-- P&L curve -->
        <g clip-path="url(#chart-clip)">
          <path d={pathD} fill="none" stroke="#22c55e" stroke-width="2.5" />
        </g>

        <!-- Y-axis labels -->
        {#each [-1, -0.5, 0, 0.5, 1] as frac}
          {@const pnlVal = frac * pnlRange}
          {@const y = yScale(pnlVal)}
          <text x={PAD_L - 8} y={y + 4} text-anchor="end" font-size="10" fill="#6b7280" font-family="monospace">
            {pnlVal >= 0 ? "+" : ""}{(pnlVal / 1).toFixed(0)}
          </text>
          <line x1={PAD_L - 4} y1={y} x2={PAD_L} y2={y} stroke="#374151" stroke-width="1" />
        {/each}

        <!-- X-axis line -->
        <line
          x1={PAD_L} y1={PAD_T + CHART_H}
          x2={PAD_L + CHART_W} y2={PAD_T + CHART_H}
          stroke="#374151" stroke-width="1"
        />

        <!-- Strike markers -->
        {#each legs as leg}
          {#if leg.strike > 0}
            {@const sx = xScale(leg.strike)}
            {@const color = legColor(leg)}
            <!-- Vertical line -->
            <line x1={sx} y1={PAD_T} x2={sx} y2={PAD_T + CHART_H} stroke={color} stroke-width="1" stroke-dasharray="3 3" stroke-opacity="0.6" />
            <!-- Badge -->
            <rect x={sx - 14} y={PAD_T + CHART_H + 6} width="28" height="18" rx="4" fill={color} />
            <text x={sx} y={PAD_T + CHART_H + 19} text-anchor="middle" font-size="9" fill="white" font-weight="bold" font-family="monospace">
              {legBadge(leg)}
            </text>
            <!-- Strike price label -->
            <text x={sx} y={PAD_T - 8} text-anchor="middle" font-size="9" fill={color} font-family="monospace">
              {leg.strike.toFixed(0)}
            </text>
          {/if}
        {/each}

        <!-- Breakeven markers -->
        {#each bes as be}
          {@const bx = xScale(be)}
          <circle cx={bx} cy={zeroY} r="5" fill="none" stroke="#facc15" stroke-width="2" />
          <text x={bx} y={zeroY - 10} text-anchor="middle" font-size="9" fill="#facc15" font-family="monospace">BE</text>
          <text x={bx} y={zeroY + 22} text-anchor="middle" font-size="9" fill="#facc15" font-family="monospace">{be.toFixed(1)}</text>
        {/each}

        <!-- Spot price marker -->
        {#if spot !== null && spot >= priceMin && spot <= priceMax}
          {@const sx = xScale(spot)}
          <line x1={sx} y1={PAD_T} x2={sx} y2={PAD_T + CHART_H} stroke="#ffffff" stroke-width="1.5" stroke-opacity="0.5" />
          <circle cx={sx} cy={zeroY} r="6" fill="#0d1117" stroke="#ffffff" stroke-width="2" />
          <text x={sx} y={zeroY - 22} text-anchor="middle" font-size="9" fill="#e5e7eb" font-family="monospace">SPOT</text>
          <text x={sx} y={zeroY - 10} text-anchor="middle" font-size="10" fill="#ffffff" font-family="monospace" font-weight="bold">${spot.toFixed(2)}</text>
        {/if}

        <!-- ±1σ expected move lines -->
        {#if sd1Lo !== null && sd1Lo >= priceMin && sd1Lo <= priceMax}
          {@const lx = xScale(sd1Lo)}
          <line x1={lx} y1={PAD_T} x2={lx} y2={PAD_T + CHART_H}
                stroke="#a855f7" stroke-width="1.5" stroke-dasharray="5 4" stroke-opacity="0.85" />
          <text x={lx} y={PAD_T - 6} text-anchor="middle" font-size="9" fill="#a855f7" font-family="monospace">{sd1Lo.toFixed(0)}</text>
          <text x={lx} y={PAD_T + CHART_H + 32} text-anchor="middle" font-size="9" fill="#a855f7" font-family="monospace">-1σ</text>
        {/if}
        {#if sd1Hi !== null && sd1Hi >= priceMin && sd1Hi <= priceMax}
          {@const hx = xScale(sd1Hi)}
          <line x1={hx} y1={PAD_T} x2={hx} y2={PAD_T + CHART_H}
                stroke="#a855f7" stroke-width="1.5" stroke-dasharray="5 4" stroke-opacity="0.85" />
          <text x={hx} y={PAD_T - 6} text-anchor="middle" font-size="9" fill="#a855f7" font-family="monospace">{sd1Hi.toFixed(0)}</text>
          <text x={hx} y={PAD_T + CHART_H + 32} text-anchor="middle" font-size="9" fill="#a855f7" font-family="monospace">+1σ</text>
        {/if}

        <!-- Y-axis title -->
        <text
          x={18} y={PAD_T + CHART_H / 2}
          text-anchor="middle" font-size="10" fill="#6b7280"
          font-family="monospace"
          transform="rotate(-90, 18, {PAD_T + CHART_H / 2})"
        >P&amp;L ($)</text>
      </svg>
    </div>

    <!-- Sidebar -->
    <div class="sidebar">
      <div class="info-box">
        <div class="info-row">
          <span class="info-label">MAX PROFIT</span>
          <span class="info-value profit">{fmtMoney(maxPft)}</span>
        </div>
        <div class="info-row">
          <span class="info-label">MAX LOSS</span>
          <span class="info-value loss">{fmtMoney(maxLss)}</span>
        </div>
        <div class="info-row">
          <span class="info-label">CREDIT</span>
          <span class="info-value">${Math.abs(credit).toFixed(2)}</span>
        </div>
        {#if bes.length > 0}
          <div class="info-row">
            <span class="info-label">BREAKEVENS</span>
            <span class="info-value be">{bes.map((b) => b.toFixed(1)).join(" / ")}</span>
          </div>
        {/if}
        {#if pop !== null}
          <div class="info-row">
            <span class="info-label">POP</span>
            <span class="info-value">{pop.toFixed(0)}%</span>
          </div>
        {/if}
        {#if expectedMove !== null}
          <div class="info-row">
            <span class="info-label">±1σ EM</span>
            <span class="info-value em">±${expectedMove.toFixed(2)}</span>
          </div>
        {/if}
      </div>

      <div class="greeks-box">
        <div class="greeks-title">GREEKS</div>
        <div class="greek-row">
          <span class="greek-sym">Δ</span>
          <span class="greek-name">DELTA</span>
          <span class="greek-val" class:long={delta !== null && delta > 0.05} class:short={delta !== null && delta < -0.05}>{greekDir(delta)}</span>
        </div>
        <div class="greek-row">
          <span class="greek-sym">V</span>
          <span class="greek-name">VEGA</span>
          <span class="greek-val" class:long={vega !== null && vega > 0} class:short={vega !== null && vega < 0}>{greekDir(vega)}</span>
        </div>
        <div class="greek-row">
          <span class="greek-sym">Θ</span>
          <span class="greek-name">THETA</span>
          <span class="greek-val" class:long={theta !== null && theta > 0} class:short={theta !== null && theta < 0}>{greekDir(theta, true)}</span>
        </div>
        <div class="greek-row">
          <span class="greek-sym">γ</span>
          <span class="greek-name">GAMMA</span>
          <span class="greek-val" class:long={gamma !== null && gamma > 0.05} class:short={gamma !== null && gamma < -0.05}>{greekDir(gamma)}</span>
        </div>
      </div>
    </div>
  </div>

  <!-- Footer -->
  <div class="footer">
    Press <kbd>Esc</kbd> in the terminal to close · Breakevens in yellow · Purple dashed = ±1σ EM · White line = spot
  </div>
</div>

<style>
  .chart-card {
    width: 100%;
    height: 100%;
    background: #0d1117;
    border: none;
    border-radius: 0;
    box-shadow: none;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    color: #e6edf3;
    outline: none;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px 10px;
    border-bottom: 1px solid #21262d;
    background: #161b22;
    border-radius: 0;
  }

  .strategy-name {
    font-size: 18px;
    font-weight: 700;
    color: #f0f6fc;
    letter-spacing: 0.5px;
  }

  .header-meta {
    display: flex;
    align-items: center;
    gap: 16px;
    font-size: 12px;
    color: #8b949e;
  }

  .ticker {
    font-size: 14px;
    font-weight: 700;
    color: #58a6ff;
  }

  .meta-item b {
    color: #e6edf3;
  }

  .close-btn {
    background: none;
    border: 1px solid #30363d;
    color: #6e7681;
    cursor: pointer;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 12px;
    transition: all 0.15s;
  }

  .close-btn:hover {
    background: #21262d;
    color: #e6edf3;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .chart-area {
    width: 100%;
    padding: 8px 4px 4px 4px;
    overflow: hidden;
  }

  .chart-area svg {
    display: block;
    max-width: 100%;
    height: auto;
  }

  .sidebar {
    width: 100%;
    flex-shrink: 0;
    padding: 12px 16px;
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: 12px;
    border-top: 1px solid #21262d;
  }

  .info-box {
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: 12px 20px;
  }

  .info-row {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .info-label {
    font-size: 9px;
    font-weight: 700;
    color: #ef4444;
    letter-spacing: 0.8px;
  }

  .info-value {
    font-size: 13px;
    font-weight: 600;
    color: #e6edf3;
  }

  .info-value.profit { color: #3fb950; }
  .info-value.loss   { color: #f85149; }
  .info-value.be     { color: #e3b341; font-size: 11px; }
  .info-value.em     { color: #a855f7; }

  .greeks-box {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .greeks-title {
    font-size: 9px;
    font-weight: 700;
    color: #6e7681;
    letter-spacing: 1px;
    padding-right: 4px;
    border-right: 1px solid #21262d;
  }

  .greek-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .greek-sym {
    font-size: 14px;
    width: 16px;
    color: #8b949e;
  }

  .greek-name {
    font-size: 9px;
    color: #ef4444;
    font-weight: 700;
    letter-spacing: 0.5px;
    width: 40px;
  }

  .greek-val {
    font-size: 11px;
    color: #6e7681;
  }

  .greek-val.long  { color: #3fb950; }
  .greek-val.short { color: #f85149; }

  .footer {
    padding: 8px 20px;
    font-size: 10px;
    color: #484f58;
    border-top: 1px solid #21262d;
    text-align: center;
  }

  kbd {
    background: #21262d;
    border: 1px solid #30363d;
    border-radius: 3px;
    padding: 0 4px;
    font-size: 9px;
    color: #8b949e;
  }
</style>
