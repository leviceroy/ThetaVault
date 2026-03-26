<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let data: any;

  const dispatch = createEventDispatcher();

  // ── Types ──────────────────────────────────────────────────────────────────

  type MonthlyPnl = {
    year: number;
    month: number;
    pnl: number;
    trade_count: number;
    win_count: number;
  };

  type SectorTrend = {
    sector: string;
    monthly_counts: number[];
    total_trades: number;
  };

  // ── Data extraction ─────────────────────────────────────────────────────────

  const accountSize: number    = data.account_size ?? 0;
  const balHist: number[]      = data.balance_history ?? [];
  const unrHist: number[]      = data.unrealized_history ?? [];
  const peakHist: number[]     = data.peak_history ?? [];
  const monthly: MonthlyPnl[]  = data.monthly_pnl ?? [];
  const rollingWR: number[]    = data.rolling_win_rate ?? [];
  const rollingTC: number[]    = data.rolling_theta_capture ?? [];
  const scatter: [number, number, string][] = data.dte_roc_scatter ?? [];
  const bprHist: number[]      = data.bpr_history ?? [];
  const sectorTrends: SectorTrend[] = data.sector_trends ?? [];

  // ── SVG helpers ─────────────────────────────────────────────────────────────

  const W  = 640;
  const PH = 180;  // panel height
  const PL = 60;   // left padding
  const PR = 20;   // right padding
  const PT = 36;   // top padding
  const PB = 36;   // bottom padding
  const CW = W - PL - PR;
  const CH = PH - PT - PB;

  function toX(i: number, n: number): number {
    return PL + (n <= 1 ? 0 : (i / (n - 1)) * CW);
  }

  function toY(v: number, minV: number, maxV: number): number {
    if (maxV === minV) return PT + CH / 2;
    return PT + CH - ((v - minV) / (maxV - minV)) * CH;
  }

  function linePath(xs: number[], ys: number[]): string {
    return xs.map((x, i) => `${i === 0 ? "M" : "L"}${x.toFixed(1)},${ys[i].toFixed(1)}`).join(" ");
  }

  function fmtK(v: number): string {
    if (Math.abs(v) >= 1000) return `$${(v / 1000).toFixed(0)}k`;
    return `$${v.toFixed(0)}`;
  }

  function monthLabel(m: MonthlyPnl): string {
    const names = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
    return names[(m.month - 1) % 12];
  }

  // ── Panel 1: Account Growth ─────────────────────────────────────────────────

  const balMin  = Math.min(...balHist,  ...unrHist,  accountSize * 0.95);
  const balMax  = Math.max(...balHist,  ...unrHist,  accountSize * 1.05);
  const balXs   = balHist.map((_, i) => toX(i, balHist.length));
  const balYs   = balHist.map((v) => toY(v, balMin, balMax));
  const unrYs   = unrHist.map((v) => toY(v, balMin, balMax));
  const zeroBalY = toY(accountSize, balMin, balMax);
  const balPath  = linePath(balXs, balYs);
  const unrPath  = linePath(balXs.slice(0, unrYs.length), unrYs);

  // ── Panel 2: Monthly P&L ────────────────────────────────────────────────────

  const mPnls   = monthly.map((m) => m.pnl);
  const mMax    = Math.max(...mPnls, 1);
  const mMin    = Math.min(...mPnls, -1);
  const mAbsMax = Math.max(Math.abs(mMax), Math.abs(mMin));
  const mBarW   = monthly.length > 0 ? Math.floor(CW / monthly.length) - 2 : 10;
  const mZeroY  = toY(0, -mAbsMax, mAbsMax);

  // ── Panel 3: Drawdown ───────────────────────────────────────────────────────

  const ddSeries: number[] = peakHist.map((pk, i) => {
    const bal = balHist[i] ?? pk;
    return accountSize > 0 ? ((bal - pk) / accountSize * 100) : 0;
  });
  const ddMin   = Math.min(...ddSeries, -0.01);
  const ddXs    = ddSeries.map((_, i) => toX(i, ddSeries.length));
  const ddYs    = ddSeries.map((v) => toY(v, ddMin * 1.1, 0));
  const ddPath  = linePath(ddXs, ddYs);
  const ddZeroY = toY(0, ddMin * 1.1, 0);
  // Fill area
  const ddFill  = ddXs.length > 0
    ? `M${ddXs[0].toFixed(1)},${ddZeroY.toFixed(1)} `
      + ddXs.map((x, i) => `L${x.toFixed(1)},${ddYs[i].toFixed(1)}`).join(" ")
      + ` L${ddXs[ddXs.length-1].toFixed(1)},${ddZeroY.toFixed(1)} Z`
    : "";
  const currentDd = ddSeries[ddSeries.length - 1] ?? 0;
  const worstDd   = ddMin;

  // ── Panel 4: Rolling Win Rate ────────────────────────────────────────────────

  const wrXs    = rollingWR.map((_, i) => toX(i, rollingWR.length));
  const wrYs    = rollingWR.map((v) => toY(v, 0, 100));
  const wrPath  = linePath(wrXs, wrYs);
  const wr50Y   = toY(50, 0, 100);
  const wr66Y   = toY(66, 0, 100);
  const latestWR = rollingWR[rollingWR.length - 1] ?? 0;

  // ── Panel 5: DTE@Entry vs ROC% scatter ─────────────────────────────────────

  const scDTEs  = scatter.map(([d]) => d);
  const scROCs  = scatter.map(([, r]) => r);
  const scMaxDTE = Math.max(...scDTEs, 60);
  const scMinROC = Math.min(...scROCs, -5);
  const scMaxROC = Math.max(...scROCs, 5);
  const scZeroY  = toY(0, scMinROC, scMaxROC);

  function stratColor(label: string): string {
    if (label.includes("Iron Condor"))    return "#22d3ee";
    if (label.includes("Iron Butterfly")) return "#22d3ee";
    if (label.includes("Short Put V"))    return "#4ade80";
    if (label.includes("Short Call V"))   return "#f87171";
    if (label.includes("Cash Secured"))   return "#86efac";
    if (label.includes("Covered"))        return "#fbbf24";
    if (label.includes("Strangle"))       return "#818cf8";
    if (label.includes("Straddle"))       return "#818cf8";
    return "#94a3b8";
  }

  // ── Panel 5b: Rolling Theta Capture ────────────────────────────────────────

  const tcXs      = rollingTC.map((_, i) => toX(i, rollingTC.length));
  const tcYMax    = 150;
  const tcYs      = rollingTC.map((v) => toY(v, 0, tcYMax));
  const tcPath    = linePath(tcXs, tcYs);
  const tc80Y     = toY(80, 0, tcYMax);
  const tc50Y     = toY(50, 0, tcYMax);
  const tc100Y    = toY(100, 0, tcYMax);
  const latestTC  = rollingTC[rollingTC.length - 1] ?? 0;

  // ── Panel 6: BPR History ────────────────────────────────────────────────────

  const bprMin  = Math.min(...bprHist) * 0.9;
  const bprMax  = Math.max(...bprHist) * 1.1;
  const bprAvg  = bprHist.length > 0 ? bprHist.reduce((a, b) => a + b, 0) / bprHist.length : 0;
  const bprXs   = bprHist.map((_, i) => toX(i, bprHist.length));
  const bprYs   = bprHist.map((v) => toY(v, bprMin, bprMax));
  const bprPath = linePath(bprXs, bprYs);
  const bprAvgY = toY(bprAvg, bprMin, bprMax);

  // ── Sector palette ──────────────────────────────────────────────────────────

  const sectorPalette = [
    "#63b3ed", "#9acd96", "#fbb005", "#f07167", "#c084fc", "#fb923c", "#94a3b8"
  ];

  const MONTH_NAMES = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
</script>

<div class="perf-charts">
  <div class="header">
    <span class="title">Performance Charts</span>
    <button class="close-btn" on:click={() => dispatch("close")}>✕</button>
  </div>

  <div class="charts-grid">
  <!-- Panel 1: Account Growth -->
  {#if balHist.length >= 2}
  <div class="panel">
    <div class="panel-title">Account Growth</div>
    <svg width="100%" height={Math.round(PH * 1.3)} viewBox="0 0 {W} {PH}" preserveAspectRatio="xMinYMin meet">
      <!-- Zero / baseline -->
      <line x1={PL} y1={zeroBalY} x2={W - PR} y2={zeroBalY}
            stroke="#30363d" stroke-width="1" stroke-dasharray="4,3"/>
      <!-- Unrealized (theta-est) -->
      {#if unrHist.length >= 2}
        <path d={unrPath} fill="none" stroke="#6366f1" stroke-width="1.5" stroke-dasharray="5,3" opacity="0.7"/>
      {/if}
      <!-- Realized balance -->
      <path d={balPath} fill="none" stroke="#22d3ee" stroke-width="2"/>
      <!-- Axis labels -->
      <text x={PL} y={PT - 6} fill="#94a3b8" font-size="10">{fmtK(balMax)}</text>
      <text x={PL} y={PH - PB + 14} fill="#94a3b8" font-size="10">{fmtK(balMin)}</text>
      <text x={W - PR} y={PH - PB + 14} fill="#94a3b8" font-size="10" text-anchor="end">
        {balHist.length} trades
      </text>
      <!-- Legend -->
      <line x1={PL + 4} y1={PT + 10} x2={PL + 20} y2={PT + 10} stroke="#22d3ee" stroke-width="2"/>
      <text x={PL + 24} y={PT + 14} fill="#94a3b8" font-size="10">Realized</text>
      {#if unrHist.length >= 2}
        <line x1={PL + 80} y1={PT + 10} x2={PL + 96} y2={PT + 10}
              stroke="#6366f1" stroke-width="1.5" stroke-dasharray="5,3"/>
        <text x={PL + 100} y={PT + 14} fill="#94a3b8" font-size="10">+Θ est.</text>
      {/if}
    </svg>
  </div>
  {/if}

  <!-- Panel 2: Monthly P&L -->
  {#if monthly.length >= 2}
  <div class="panel">
    <div class="panel-title">Monthly P&amp;L</div>
    <svg width="100%" height={Math.round(PH * 1.3)} viewBox="0 0 {W} {PH}" preserveAspectRatio="xMinYMin meet">
      <line x1={PL} y1={mZeroY} x2={W - PR} y2={mZeroY}
            stroke="#30363d" stroke-width="1"/>
      {#each monthly as m, i}
        {@const x = PL + (i / monthly.length) * CW + 1}
        {@const barH = Math.abs(toY(m.pnl, -mAbsMax, mAbsMax) - mZeroY)}
        {@const y = m.pnl >= 0 ? mZeroY - barH : mZeroY}
        {@const color = m.pnl >= 0 ? "#4ade80" : "#f87171"}
        <rect {x} {y} width={Math.max(mBarW, 2)} height={Math.max(barH, 1)}
              fill={color} opacity="0.85"/>
        {#if i % Math.ceil(monthly.length / 8) === 0}
          <text x={x + mBarW / 2} y={PH - PB + 14} fill="#64748b" font-size="9" text-anchor="middle">
            {monthLabel(m)}{String(m.year).slice(2)}
          </text>
        {/if}
      {/each}
      <text x={PL} y={PT - 6} fill="#94a3b8" font-size="10">{fmtK(mAbsMax)}</text>
      <text x={PL} y={PH - PB + 14} fill="#94a3b8" font-size="10">-{fmtK(mAbsMax)}</text>
    </svg>
  </div>
  {/if}

  <!-- Panel 3: Drawdown -->
  {#if ddSeries.length >= 2 && ddMin < -0.001}
  <div class="panel">
    <div class="panel-title">
      Drawdown &nbsp;
      <span style="color: {currentDd < -5 ? '#f87171' : currentDd < -2 ? '#fbbf24' : '#4ade80'}">
        current: {currentDd.toFixed(1)}%
      </span>
      &nbsp; worst: <span style="color:#f87171">{worstDd.toFixed(1)}%</span>
    </div>
    <svg width="100%" height={Math.round(PH * 1.3)} viewBox="0 0 {W} {PH}" preserveAspectRatio="xMinYMin meet">
      <line x1={PL} y1={ddZeroY} x2={W - PR} y2={ddZeroY}
            stroke="#30363d" stroke-width="1"/>
      <path d={ddFill} fill="#ef4444" opacity="0.15"/>
      <path d={ddPath} fill="none" stroke="#ef4444" stroke-width="1.5"/>
      <text x={PL} y={PH - PB + 14} fill="#94a3b8" font-size="10">{(ddMin * 1.1).toFixed(1)}%</text>
      <text x={PL} y={PT - 6} fill="#94a3b8" font-size="10">0%</text>
    </svg>
  </div>
  {/if}

  <!-- Panel 4: Rolling Win Rate -->
  {#if rollingWR.length >= 5}
  <div class="panel">
    <div class="panel-title">
      Rolling Win Rate &nbsp;
      <span style="color: {latestWR >= 60 ? '#4ade80' : latestWR >= 45 ? '#fbbf24' : '#f87171'}">
        latest: {latestWR.toFixed(0)}%
      </span>
    </div>
    <svg width="100%" height={Math.round(PH * 1.3)} viewBox="0 0 {W} {PH}" preserveAspectRatio="xMinYMin meet">
      <!-- 66% target -->
      <line x1={PL} y1={wr66Y} x2={W - PR} y2={wr66Y}
            stroke="#4ade80" stroke-width="1" stroke-dasharray="4,4" opacity="0.5"/>
      <text x={W - PR - 2} y={wr66Y - 3} fill="#4ade80" font-size="9" text-anchor="end">66%</text>
      <!-- 50% line -->
      <line x1={PL} y1={wr50Y} x2={W - PR} y2={wr50Y}
            stroke="#64748b" stroke-width="1" stroke-dasharray="3,3"/>
      <text x={W - PR - 2} y={wr50Y - 3} fill="#64748b" font-size="9" text-anchor="end">50%</text>
      <path d={wrPath} fill="none"
            stroke={latestWR >= 60 ? "#4ade80" : latestWR >= 45 ? "#fbbf24" : "#f87171"}
            stroke-width="2"/>
      <text x={PL} y={PT - 6} fill="#94a3b8" font-size="10">100%</text>
      <text x={PL} y={PH - PB + 14} fill="#94a3b8" font-size="10">0%</text>
      <text x={W - PR} y={PH - PB + 14} fill="#94a3b8" font-size="10" text-anchor="end">
        {rollingWR.length} trades
      </text>
    </svg>
  </div>
  {/if}

  <!-- Panel 4b: Rolling Theta Capture -->
  {#if rollingTC.length >= 5}
  <div class="panel">
    <div class="panel-title">
      Rolling Theta Capture &nbsp;
      <span style="color: {latestTC >= 80 && latestTC <= 120 ? '#4ade80' : latestTC >= 50 ? '#fbbf24' : '#f87171'}">
        latest: {latestTC.toFixed(0)}%
      </span>
    </div>
    <svg width="100%" height={Math.round(PH * 1.3)} viewBox="0 0 {W} {PH}" preserveAspectRatio="xMinYMin meet">
      <!-- 100% reference (break-even theta) -->
      <line x1={PL} y1={tc100Y} x2={W - PR} y2={tc100Y}
            stroke="#30363d" stroke-width="1" stroke-dasharray="4,3"/>
      <text x={W - PR - 2} y={tc100Y - 3} fill="#475569" font-size="9" text-anchor="end">100%</text>
      <!-- 80% target -->
      <line x1={PL} y1={tc80Y} x2={W - PR} y2={tc80Y}
            stroke="#4ade80" stroke-width="1" stroke-dasharray="4,4" opacity="0.5"/>
      <text x={W - PR - 2} y={tc80Y - 3} fill="#4ade80" font-size="9" text-anchor="end">80%</text>
      <!-- 50% caution -->
      <line x1={PL} y1={tc50Y} x2={W - PR} y2={tc50Y}
            stroke="#64748b" stroke-width="1" stroke-dasharray="3,3"/>
      <text x={W - PR - 2} y={tc50Y - 3} fill="#64748b" font-size="9" text-anchor="end">50%</text>
      <path d={tcPath} fill="none"
            stroke={latestTC >= 80 && latestTC <= 120 ? "#4ade80" : latestTC >= 50 ? "#fbbf24" : "#f87171"}
            stroke-width="2"/>
      <text x={PL} y={PT - 6} fill="#94a3b8" font-size="10">{tcYMax}%</text>
      <text x={PL} y={PH - PB + 14} fill="#94a3b8" font-size="10">0%</text>
      <text x={W - PR} y={PH - PB + 14} fill="#94a3b8" font-size="10" text-anchor="end">
        {rollingTC.length} trades
      </text>
    </svg>
  </div>
  {/if}

  <!-- Panel 5: DTE@Entry vs ROC% scatter -->
  {#if scatter.length >= 3}
  <div class="panel">
    <div class="panel-title">DTE@Entry vs ROC%</div>
    <svg width="100%" height={Math.round(PH * 1.3)} viewBox="0 0 {W} {PH}" preserveAspectRatio="xMinYMin meet">
      <!-- Zero ROC line -->
      <line x1={PL} y1={scZeroY} x2={W - PR} y2={scZeroY}
            stroke="#30363d" stroke-width="1" stroke-dasharray="4,3"/>
      {#each scatter as [dte, roc, strat]}
        {@const cx = PL + (dte / scMaxDTE) * CW}
        {@const cy = toY(roc, scMinROC, scMaxROC)}
        <circle {cx} {cy} r="3" fill={stratColor(strat)} opacity="0.75"/>
      {/each}
      <text x={PL} y={PH - PB + 14} fill="#94a3b8" font-size="10">0d</text>
      <text x={W - PR} y={PH - PB + 14} fill="#94a3b8" font-size="10" text-anchor="end">
        {scMaxDTE}d
      </text>
      <text x={PL} y={PT - 6} fill="#94a3b8" font-size="10">{scMaxROC.toFixed(0)}%</text>
      <text x={PL} y={PH - PB + 14} fill="#94a3b8" font-size="10">{scMinROC.toFixed(0)}%</text>
      <!-- X axis label -->
      <text x={PL + CW / 2} y={PH - 6} fill="#64748b" font-size="9" text-anchor="middle">DTE at Entry</text>
    </svg>
  </div>
  {/if}

  <!-- Panel 6: BPR History -->
  {#if bprHist.length >= 3}
  <div class="panel">
    <div class="panel-title">
      Position Sizing (BPR) &nbsp;
      <span style="color:#94a3b8">avg {fmtK(bprAvg)}</span>
    </div>
    <svg width="100%" height={Math.round(PH * 1.3)} viewBox="0 0 {W} {PH}" preserveAspectRatio="xMinYMin meet">
      <!-- avg reference line -->
      <line x1={PL} y1={bprAvgY} x2={W - PR} y2={bprAvgY}
            stroke="#64748b" stroke-width="1" stroke-dasharray="4,3" opacity="0.7"/>
      <path d={bprPath} fill="none" stroke="#22d3ee" stroke-width="1.5"/>
      <text x={PL} y={PT - 6} fill="#94a3b8" font-size="10">{fmtK(bprMax)}</text>
      <text x={PL} y={PH - PB + 14} fill="#94a3b8" font-size="10">{fmtK(bprMin)}</text>
      <text x={W - PR} y={PH - PB + 14} fill="#94a3b8" font-size="10" text-anchor="end">
        {bprHist.length} trades
      </text>
    </svg>
  </div>
  {/if}

  <!-- Sector Exposure heatmap -->
  {#if sectorTrends.length >= 1 && monthly.length >= 2}
  <div class="panel">
    <div class="panel-title">Sector Exposure Over Time</div>
    <div class="sector-grid">
      <!-- Month headers -->
      <div class="sector-header-row">
        <span class="sector-name-cell"></span>
        {#each monthly.slice(-16) as m}
          <span class="sector-month-cell">{MONTH_NAMES[(m.month - 1) % 12]}</span>
        {/each}
      </div>
      {#each sectorTrends as trend, ti}
        {@const col = sectorPalette[ti % sectorPalette.length]}
        {@const globalMax = Math.max(...sectorTrends.flatMap(t => t.monthly_counts), 1)}
        <div class="sector-row">
          <span class="sector-name-cell" style="color:{col}">{trend.sector.slice(0, 14)}</span>
          {#each monthly.slice(-16) as _m, mi}
            {@const realIdx = monthly.length - Math.min(16, monthly.length) + mi}
            {@const count = trend.monthly_counts[realIdx] ?? 0}
            {@const intensity = count === 0 ? 0 : Math.min(1, 0.2 + (count / globalMax) * 0.8)}
            <span class="sector-month-cell">
              <span class="heat-block"
                    style="background: {col}; opacity: {intensity}; display:block; width:100%; height:12px; border-radius:2px;">
              </span>
            </span>
          {/each}
          <span class="sector-total" style="color:#64748b">{trend.total_trades}tr</span>
        </div>
      {/each}
    </div>
  </div>
  {/if}
  </div><!-- end charts-grid -->
</div>

<style>
  .perf-charts {
    background: #0d1117;
    color: #e2e8f0;
    font-family: "JetBrains Mono", "Fira Code", "Consolas", monospace;
    font-size: 12px;
    padding: 8px;
    min-height: 100%;
    overflow-y: auto;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px 10px;
    border-bottom: 1px solid #21262d;
    margin-bottom: 8px;
  }

  .title {
    color: #22d3ee;
    font-size: 14px;
    font-weight: bold;
    letter-spacing: 0.05em;
  }

  .close-btn {
    background: none;
    border: 1px solid #30363d;
    color: #94a3b8;
    cursor: pointer;
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 12px;
    transition: background 0.15s;
  }

  .close-btn:hover {
    background: #21262d;
    color: #e2e8f0;
  }

  .charts-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-bottom: 8px;
  }

  .panel {
    background: #0d1117;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 0 0 4px;
    overflow: hidden;
  }

  .panel-title {
    color: #94a3b8;
    font-size: 11px;
    padding: 6px 10px 2px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  svg {
    display: block;
    max-width: 100%;
  }

  /* Sector heatmap */
  .sector-grid {
    padding: 4px 8px 6px;
    overflow-x: auto;
  }

  .sector-header-row,
  .sector-row {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-bottom: 2px;
  }

  .sector-name-cell {
    width: 100px;
    min-width: 100px;
    font-size: 10px;
    color: #94a3b8;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sector-month-cell {
    width: 22px;
    min-width: 22px;
    text-align: center;
    font-size: 8px;
    color: #475569;
  }

  .sector-total {
    width: 28px;
    min-width: 28px;
    font-size: 9px;
    text-align: right;
    color: #475569;
  }
</style>
