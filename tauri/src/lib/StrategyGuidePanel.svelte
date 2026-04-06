<script lang="ts">
  import { createEventDispatcher } from "svelte";
  export let strategy: string;
  export let playbook: any = null;
  const dispatch = createEventDispatcher();

  function fmt(val: any, suffix = "") {
    return val != null ? `${val}${suffix}` : "—";
  }
</script>

<div class="guide-root">
  <button class="close-btn" on:click={() => dispatch("close")}>✕</button>

  {#if strategy === "covered_call"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">COVERED CALL</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Bullish stock position where we are selling an ATM/OTM call against 100 long shares of stock
          to reduce the cost basis of the shares. The short call risk is "covered" by the 100 shares
          of long stock we own.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↗</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bullish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">50% to 70%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">S</div>
          <div class="step-label">Buy 100 shares of stock</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell an ATM/OTM call for every 100 shares</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Distance Between Stock Purchase &amp; Short Call + Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Stock Purchase Price − Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Stock Purchase Price − Credit Received</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may purchase 100 shares and sell the $105 call against the
            shares to reduce our breakeven price.
          </p>
          <div class="chain-diagram">
            <div class="chain-row chain-call">
              <span class="chain-price">105</span>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row chain-stock">
              <div class="chain-badge green-badge">S</div>
              <span class="chain-price">100</span>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — XYZ $100 stock, sell $105 call @ $2 credit</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Price range $75–$125, cost basis $100, call strike $105, credit $2, breakeven $98 -->
        <!-- coordinate mapping: x=(price-75)/(125-75)*600, y=(pnl+1500)/(1500+800)*160 inverted -->
        <!-- pnl(price): price<98 → (price-100)*100 (loss), 98–105 → (price-100)*100+200 (gain), >105 → 700 flat -->

        <!-- Red loss zone (below breakeven at x=276) -->
        <polygon
          points="0,160 276,160 276,78.9 0,0"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone (above breakeven up to call strike x=360, then flat) -->
        <polygon
          points="276,78.9 360,43 600,43 600,160 276,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Dashed vertical: stock cost $100 (x=300) -->
        <line x1="300" y1="0" x2="300" y2="160" stroke="#64748b" stroke-width="1" stroke-dasharray="4,3" />
        <!-- Dashed vertical: call strike $105 (x=360) -->
        <line x1="360" y1="0" x2="360" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: slope down from x=0, breakeven at x=276, peaks at x=360, flat beyond -->
        <polyline
          points="0,0 276,78.9 360,43 600,43"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="78.9" x2="600" y2="78.9" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Labels -->
        <text x="370" y="38" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $700</text>
        <text x="240" y="74" fill="#f59e0b" font-size="9" font-family="monospace">BE $98</text>
        <text x="5" y="12" fill="#dc2626" font-size="9" font-family="monospace">Loss ↑</text>
        <text x="302" y="155" fill="#94a3b8" font-size="8" font-family="monospace">$100</text>
        <text x="362" y="155" fill="#dc2626" font-size="8" font-family="monospace">$105C</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock moves up to the short call strike by the expiration of the contract.
            This results in max extrinsic value collected from the short call, as well as max value
            gained on the long shares up to the call strike.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock goes down. This results in losses in the shares you own, although the short call
            will lose value and hedge the loss on those shares.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the short call loses value, we can roll it out in time to add extrinsic value to the trade,
            reducing the cost basis on the shares further. We can also move the call strike down in the same
            cycle to achieve the same cost basis reduction result, or a combination of rolling out in time
            and down a few strikes. Avoid rolling the call below your breakeven on the trade overall to
            ensure potential profit if the stock rallies back.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            We may hold — this may result in an extrinsic value loss in the short call, but extrinsic
            value goes to zero by expiry. If this is paired with a stock price selloff, we can adjust
            the short call if desired.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            The short call may decrease in value and add profit to our position overall, especially if
            this is paired with a small bullish move.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber">EXPIRATION</div>
      <div class="exp-grid">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            The short call will expire worthless — we can deploy another one in a further expiration,
            or lean long with the shares.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The short call will be exercised and "call away" the 100 shares of stock we own.
            The position will go away and we will realize max profit.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Covered calls are cost basis reduction strategies where we are limiting our upside profit
            potential to guarantee a credit and cost basis reduction on the shares. With this said,
            we should place our short call at a level we're comfortable capping our profit potential at.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            If we want to preserve our shares and avoid assignment, we can roll the short call out in
            time and up a few strikes for a small credit before the short call moves ITM. This moves
            the existing extrinsic value to the next cycle, and moving our short strike up gives us
            more potential profit on the shares.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "short_naked_put"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">SHORT NAKED PUT</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Neutral-Bullish undefined risk credit trade where we are betting against the stock moving below
          our strike price by the expiration of our contract. No cash set-aside required — margin-backed.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↗</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Neutral-Bullish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">60% to 80%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell an OTM put (~30 delta)</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Put Strike × 100 − Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Put Strike − Credit Received</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we might sell the 95 put for $1.00 credit.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div style="width:20px;height:20px;border-radius:50%;border:2px solid #64748b;display:flex;align-items:center;justify-content:center;font-size:10px;color:#94a3b8;">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">95</span>
              <div class="chain-badge red-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Short</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      X range $80–$105, width 600 → scale 24px/$
      Y range -$9400 to +$100, height 160
      Sell 95P for $1.00 credit → max profit $100, max loss $9400 (theoretical $0)
      Breakeven = $95 - $1.00 = $94 → x=336
      We clip max loss display at $500 for readability: below $92, we show flat line at bottom
      y scale: 160/(100+500)=0.2667 → profit $100 → y=26.7, zero → y=26.7+26.7=53.4 (wrong)
      Simple display: profit cap $100 at top, loss shown to $500:
        pnl range displayed: +100 to -500 (600 total)
        y(pnl) = (100 - pnl) / 600 * 160
        pnl=+100 → y=0, pnl=0 → y=26.7, pnl=-500 → y=160
      Stock > $95: flat $100 profit
      Stock = $94 (BE): pnl=0 → y=26.7 → x=(94-80)*24=336
      Stock = $80: pnl=-1500 → clamped to -500 → y=160
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — XYZ $100 stock, sell 95P @ $1.00 credit</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red loss zone: left of breakeven -->
        <polygon
          points="0,160 0,160 336,26.7 336,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone: right of breakeven -->
        <polygon
          points="336,26.7 360,0 600,0 600,160 336,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="26.7" x2="600" y2="26.7" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed vertical: short put $95 (x=360) -->
        <line x1="360" y1="0" x2="360" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <!-- Dashed vertical: stock $100 (x=480) -->
        <line x1="480" y1="0" x2="480" y2="160" stroke="#64748b" stroke-width="1" stroke-dasharray="4,3" />
        <!-- Dashed vertical: BE $94 (x=336) -->
        <line x1="336" y1="0" x2="336" y2="160" stroke="#f59e0b" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: steep downslope left → flat at max profit right of $95 -->
        <polyline
          points="0,160 336,26.7 360,0 600,0"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="370" y="12" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $100</text>
        <text x="339" y="22" fill="#f59e0b" font-size="9" font-family="monospace">BE $94</text>
        <text x="5" y="155" fill="#dc2626" font-size="9" font-family="monospace">Loss (unlimited↓)</text>
        <text x="363" y="155" fill="#dc2626" font-size="8" font-family="monospace">$95P</text>
        <text x="483" y="155" fill="#94a3b8" font-size="8" font-family="monospace">$100</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock stays above the short put strike or rises. The put loses value through theta decay
            and we close the trade at 50% of max profit, or it expires OTM and we keep the full credit.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock drops below the short put strike. The put moves ITM and increases in value,
            resulting in an unrealized loss. If the stock continues lower, losses are theoretically
            unlimited (down to zero).
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the put is tested, roll it out in time for a net credit to buy more time and reduce
            the effective cost basis. Many traders manage short puts by rolling perpetually rather than
            taking assignment.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            We may hold — extrinsic value increases our unrealized loss but decays to zero by
            expiration. If IV expansion means the stock is moving, we may roll early.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            Extrinsic value collapses quickly. This typically results in a profitable trade, and
            we should consider closing if we've captured 50% of max profit.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            Put expires worthless. Keep the full credit. Redeploy in the next cycle if desired.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            You are assigned 100 shares of stock at the short put strike. Your effective cost basis
            is the breakeven (strike − credit). You can then sell a covered call to begin reducing
            cost basis further — the "Wheel."
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Short puts can profit if the stock goes up, stays flat, or moves down slightly. The position
            wins if the stock stays above the strike at expiration — three of four market directions
            work in our favor.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            If the stock moves ITM, the short put replicates a covered call risk profile. Many traders
            roll the put perpetually for a credit rather than taking assignment, effectively managing
            the position as an ongoing income strategy.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "short_put_vertical"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">SHORT PUT<br>VERTICAL SPREAD</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Neutral-Bullish defined risk credit trade where we are betting against the stock moving below
          our short strike price by the expiration of our contract. Spread width depends on account size,
          risk tolerance, etc.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↗</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Neutral-Bullish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">60% to 80%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell an OTM/ATM put</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge green-badge">P</div>
          <div class="step-label">Buy a further OTM put</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Distance Between Strikes − Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Short Put Strike − Credit Received</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we might sell a 95/90 put spread and look to collect $1.65.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div style="width:20px;height:20px;border-radius:50%;border:2px solid #64748b;display:flex;align-items:center;justify-content:center;font-size:10px;color:#94a3b8;">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">95</span>
              <div class="chain-badge red-badge">P</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">90</span>
              <div class="chain-badge green-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Flat</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      X range $80–$105, width 600 → scale 24px/$
      Y range -$350 to +$250, height 160 → scale 160/600
      Short put $95 → x=360, Long put $90 → x=240, Stock $100 → x=480
      Credit $1.65 → max profit $165, max loss $335
      Breakeven = $95 - $1.65 = $93.35 → x=320.4
      y(pnl) = (250 - pnl) / 600 * 160
        pnl=+165 → y=22.7
        pnl=0    → y=66.7
        pnl=-335 → y=156.3
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — XYZ $100 stock, sell 95/90 put spread @ $1.65 credit</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red loss zone: from $80 to breakeven $93.35 -->
        <polygon
          points="0,156.3 0,156.3 240,156.3 320.4,66.7 320.4,160 0,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone: from breakeven to right edge -->
        <polygon
          points="320.4,66.7 360,22.7 600,22.7 600,160 320.4,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="66.7" x2="600" y2="66.7" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed vertical: long put $90 (x=240) -->
        <line x1="240" y1="0" x2="240" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <!-- Dashed vertical: short put $95 (x=360) -->
        <line x1="360" y1="0" x2="360" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <!-- Dashed vertical: stock $100 (x=480) -->
        <line x1="480" y1="0" x2="480" y2="160" stroke="#64748b" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: flat max loss → slope up to breakeven → slope up to short put → flat max profit -->
        <polyline
          points="0,156.3 240,156.3 320.4,66.7 360,22.7 600,22.7"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="370" y="18" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $165</text>
        <text x="323" y="62" fill="#f59e0b" font-size="9" font-family="monospace">BE $93.35</text>
        <text x="5" y="153" fill="#dc2626" font-size="9" font-family="monospace">Max Loss $335</text>
        <text x="243" y="155" fill="#22c55e" font-size="8" font-family="monospace">$90P</text>
        <text x="363" y="155" fill="#dc2626" font-size="8" font-family="monospace">$95P</text>
        <text x="483" y="155" fill="#94a3b8" font-size="8" font-family="monospace">$100</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock increases in value. A short put spread is a directionally bullish position so
            ideally the stock rises, time passes, volatility contracts, or a combination of the three
            so the spread loses value over time.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock decreases in value. The value of the short put spread can increase, which means
            the spread will be more expensive compared to the original opening sale price, which would
            result in a loss.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the spread is OTM/ATM, rolling out to a farther expiration can be done for a credit,
            which adds time to the trade, reduces max loss, and increases max profit if the new spread
            expires OTM.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            We may hold — this may result in an extrinsic value loss, but extrinsic value will always
            go to zero by expiration.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            This could result in a profitable trade, as extrinsic value goes down across the board,
            so we may consider closing if the trade is profitable.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            The trade will realize max profit — we can close it to remove risk, or let it expire
            worthless if we believe it will remain OTM.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            We close the trade — holding a put spread through expiration will result in both options
            being exercised resulting in no position, but we close the trade for max loss to avoid
            assignment.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            We either roll out in time to extend the trade or close it. We avoid letting these trades
            go through expiration because if the short put is ITM and the long put is OTM, we can come
            back to the market the next trading session with 100 shares of stock.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Vertical spreads have a less volatile P/L because of the long option that defines our risk.
            If we see profit on the short option, we will see losses on the long option and vice versa.
            For this reason, we should expect to be in spread trades longer than naked options to reach
            profit targets.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            With spreads, it's important to realize that options will be exercised if they are ITM and
            held through expiration. If one strike is ITM and the other moves OTM, close the trade prior
            to expiration to avoid unwanted shares.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "iron_condor"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">IRON CONDOR</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Neutral, defined risk strategy consisting of an OTM put credit spread and OTM call credit spread,
          where we want the stock to stay between our short strikes through expiration. We aim to collect
          1/3rd the width of the strikes.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↔</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Neutral</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">60% to 80%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell an OTM put spread</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell an OTM call spread</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Widest Spread − Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Short Put Strike − Credit &amp; Short Call Strike + Credit</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may sell the 95/92 put spread and the 105/108 call spread
            and look to collect $1.00.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">108</span>
              <div class="chain-badge green-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">105</span>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div style="width:20px;height:20px;border-radius:50%;border:2px solid #64748b;display:flex;align-items:center;justify-content:center;font-size:9px;color:#94a3b8;">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">95</span>
              <div class="chain-badge red-badge">P</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">92</span>
              <div class="chain-badge green-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Flat</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      X range $82–$118 (36 pts), width 600 → scale 16.667px/$
      x(price) = (price-82)*16.667
      x(92)=167, x(94)=200, x(95)=217, x(100)=300, x(105)=383, x(106)=400, x(108)=433
      Credit=$1.00 → max profit $100, max loss $200
      Put BE=$94, Call BE=$106
      Y: range -250 to +200 (450 total), height 160
      y(pnl) = (200-pnl)/450*160
      y(+100)=35.6, y(0)=71.1, y(-200)=142.2
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — XYZ $100, sell 95/92 put spread + 105/108 call spread @ $1.00 credit</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Left red loss zone (below zero, left of put BE x=200) -->
        <polygon
          points="0,160 200,160 200,71.1 167,142.2 0,142.2"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Right red loss zone (below zero, right of call BE x=400) -->
        <polygon
          points="400,160 600,160 600,142.2 433,142.2 400,71.1"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone (tent shape between the two BEs) -->
        <polygon
          points="200,160 200,71.1 217,35.6 383,35.6 400,71.1 400,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="71.1" x2="600" y2="71.1" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: long put $92, short put $95, stock $100, short call $105, long call $108 -->
        <line x1="167" y1="0" x2="167" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="217" y1="0" x2="217" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#64748b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="383" y1="0" x2="383" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="433" y1="0" x2="433" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: tent shape -->
        <polyline
          points="0,142.2 167,142.2 200,71.1 217,35.6 383,35.6 400,71.1 433,142.2 600,142.2"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="250" y="30" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $100</text>
        <text x="2" y="139" fill="#dc2626" font-size="9" font-family="monospace">Loss $200</text>
        <text x="169" y="155" fill="#22c55e" font-size="8" font-family="monospace">$92P</text>
        <text x="219" y="155" fill="#dc2626" font-size="8" font-family="monospace">$95P</text>
        <text x="285" y="155" fill="#94a3b8" font-size="8" font-family="monospace">$100</text>
        <text x="363" y="155" fill="#dc2626" font-size="8" font-family="monospace">$105C</text>
        <text x="435" y="155" fill="#22c55e" font-size="8" font-family="monospace">$108C</text>
        <text x="168" y="68" fill="#f59e0b" font-size="8" font-family="monospace">BE$94</text>
        <text x="370" y="68" fill="#f59e0b" font-size="8" font-family="monospace">BE$106</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock stays between our strikes as time passes. This results in extrinsic value decay
            on both sides and the trade can be bought back for a profit over time.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock moves outside of one of our credit spreads. This will go from a neutral to a
            directional trade if this happens, and we can start to see losses as the "tested" side
            increases in value as the stock price moves closer to that side.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            Defensive management is limited with defined risk credit trades, but we can roll a spread
            out in time for a credit if it is not ITM yet. If one of our spreads begins to be tested,
            we can roll it out in time. We can also close or roll the untested side if we want to
            extend duration and add credit to the trade to reduce max loss.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            We may hold — this may result in an extrinsic value loss, but extrinsic value will always
            go to zero by expiration.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            Close for a winner if we reach our desired profit target and the stock is still between
            our strikes.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            Close for profit to remove risk and secure the profit in the position.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            Close for a loss to avoid assignment fees and the stock moving between our spread after
            hours, which could result in unwanted shares on Monday.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Be mindful of liquidity. With 4 legs to this trade, the most liquid products are best
            to reduce slippage.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            Select strikes that create a wide range of profitability while still collecting around
            1/3rd the width of the spread. Defensive management is limited, so the best defense
            is a wide profit range up front.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "short_call_vertical"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">SHORT CALL<br>VERTICAL SPREAD</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Neutral-Bearish defined risk credit trade where we are betting against the stock moving above
          our short strike price by the expiration of our contract. Spread width depends on account size,
          risk tolerance, etc.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↘</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Neutral-Bearish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">60% to 80%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell an ATM/OTM call</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge green-badge">C</div>
          <div class="step-label">Buy a further OTM call</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Distance Between Strikes − Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Call Strike + Credit Received</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we might sell a 105/110 call spread and look to collect $1.65.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">110</span>
              <div class="chain-badge green-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">105</span>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div style="width:20px;height:20px;border-radius:50%;border:2px solid #64748b;display:flex;align-items:center;justify-content:center;font-size:9px;color:#94a3b8;">○</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Flat</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      X range $83–$118 (35 pts), width 600 → scale 17.14px/$
      x(price) = (price-83)*17.14
      x(100)=291, x(105)=377, x(106.65)=405, x(110)=463
      Credit=$1.65 → max profit $165, max loss $335
      Breakeven = $105 + $1.65 = $106.65
      Y: range -400 to +250 (650 total), height 160
      y(pnl) = (250-pnl)/650*160
      y(+165)=20.9, y(0)=61.5, y(-335)=144
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — XYZ $100 stock, sell 105/110 call spread @ $1.65 credit</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Green profit zone: left of breakeven -->
        <polygon
          points="0,160 0,20.9 377,20.9 405,61.5 405,160"
          fill="rgba(34,197,94,0.15)"
        />
        <!-- Red loss zone: right of breakeven -->
        <polygon
          points="405,160 405,61.5 463,144 600,144 600,160"
          fill="rgba(220,38,38,0.18)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="61.5" x2="600" y2="61.5" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: stock $100, short call $105, breakeven $106.65, long call $110 -->
        <line x1="291" y1="0" x2="291" y2="160" stroke="#64748b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="377" y1="0" x2="377" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="463" y1="0" x2="463" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: flat max profit → slope down at short call → flat max loss beyond long call -->
        <polyline
          points="0,20.9 377,20.9 405,61.5 463,144 600,144"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="5" y="17" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $165</text>
        <text x="466" y="141" fill="#dc2626" font-size="9" font-family="monospace">Max Loss $335</text>
        <text x="293" y="155" fill="#94a3b8" font-size="8" font-family="monospace">$100</text>
        <text x="379" y="155" fill="#dc2626" font-size="8" font-family="monospace">$105C</text>
        <text x="465" y="155" fill="#22c55e" font-size="8" font-family="monospace">$110C</text>
        <text x="368" y="58" fill="#f59e0b" font-size="8" font-family="monospace">BE $106.65</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock decreases in value. A short call spread is a directionally bearish position —
            so ideally the stock price falls, time passes, volatility contracts, or a combination of
            the three so that the spread loses value over time.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock increases in value. The value of the short call spread can increase, which means
            the spread will be more expensive compared to the original opening sale price, which would
            result in a loss.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the spread is OTM/ATM, rolling out to a farther expiration can be done for a credit,
            which adds time to the trade, reduces max loss, and increases max profit if the new spread
            expires OTM.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            The trade may increase in extrinsic value, but if the increase in IV is paired with a
            selloff, the trade could be profitable and we can close for a winner.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            The trade may lose value, unless this is paired with a bullish move that could offset
            the extrinsic value contraction.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            Both options will expire worthless, and the trade will be at max profit.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The trade will be at max loss, and we can close the trade to avoid assignment.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            We either close the trade or roll out in time to extend it. We avoid letting these trades
            go through expiration because if the short call is ITM and the long call is OTM, we can
            come back to the market the next trading session with 100 shares of short stock.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Vertical spreads have a less volatile P/L because of the long option that defines our risk.
            If we see profit on the short option, we will see losses on the long option and vice versa.
            For this reason, we should expect to be in spread trades longer than naked options to reach
            profit targets.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            With spreads, it's important to realize that options will be exercised if they are ITM and
            held through expiration. If one strike is ITM and the other moves OTM, close the trade prior
            to expiration to avoid unwanted shares.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "call_calendar_spread"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">CALL CALENDAR<br>SPREAD</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          A neutral, defined risk trade where we are betting on an increase in IV while the stock stays
          near our strikes, or for the stock to stay stagnant and our short premium to decay faster than
          our long premium.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↗</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bullish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Low</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">N/A</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">C</div>
          <div class="step-label">Buy call in a long-term expiration cycle</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell call in a near-term expiration cycle</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Variable</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">10–25% of Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Variable</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 105 call in a long-term expiration and sell the 105
            call in a near-term expiration.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">105</span>
              <div class="chain-badge red-badge" style="font-size:9px;">C</div>
              <div class="chain-badge green-badge" style="font-size:9px;">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div style="width:20px;height:20px;border-radius:50%;border:2px solid #64748b;display:flex;align-items:center;justify-content:center;font-size:9px;color:#94a3b8;">○</div>
            </div>
          </div>
          <div class="cal-time-labels">
            <span class="cal-time-label red-text">Short — Near-term</span>
            <span class="cal-time-label" style="color:#22c55e;">Long — Far-term</span>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Calendar payoff is a "hill" centered at the short call strike ($105).
      X range $85–$125, 40 pts → scale 15px/$; x(price) = (price-85)*15
      x(85)=0, x(97)=180, x(100)=225, x(105)=300, x(113)=420, x(125)=600
      Approx pnl: far wings = -debit (-200), peak at strike = +300 variable
      BE approx $97 (left) and $113 (right)
      Y: range -$250 to +$350 (600 total), height 160
      y(pnl)=(350-pnl)/600*160
      y(+300)=13.3, y(0)=93.3, y(-200)=146.7
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Approximate Payoff at Front Expiration — XYZ $100, buy/sell 105 call (variable, IV-dependent)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red left loss zone -->
        <polygon
          points="0,160 0,146.7 75,141.3 150,114.7 180,93.3 180,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone -->
        <polygon
          points="180,160 180,93.3 225,66.7 300,13.3 375,66.7 420,93.3 420,160"
          fill="rgba(34,197,94,0.15)"
        />
        <!-- Red right loss zone -->
        <polygon
          points="420,160 420,93.3 450,114.7 525,141.3 600,146.7 600,160"
          fill="rgba(220,38,38,0.18)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="93.3" x2="600" y2="93.3" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: stock $100, short/long call strike $105 -->
        <line x1="225" y1="0" x2="225" y2="160" stroke="#64748b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Smooth hill payoff line -->
        <polyline
          points="0,146.7 75,141.3 150,114.7 180,93.3 225,66.7 300,13.3 375,66.7 420,93.3 450,114.7 525,141.3 600,146.7"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="252" y="10" fill="#22c55e" font-size="10" font-family="monospace">Max Profit (Variable)</text>
        <text x="2" y="144" fill="#dc2626" font-size="9" font-family="monospace">Loss ↓</text>
        <text x="490" y="144" fill="#dc2626" font-size="9" font-family="monospace">Loss ↓</text>
        <text x="228" y="155" fill="#94a3b8" font-size="8" font-family="monospace">$100</text>
        <text x="303" y="155" fill="#dc2626" font-size="8" font-family="monospace">$105</text>
        <text x="140" y="90" fill="#f59e0b" font-size="8" font-family="monospace">BE≈$97</text>
        <text x="423" y="90" fill="#f59e0b" font-size="8" font-family="monospace">BE≈$113</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock trickles up to the call calendar strikes over time. This will result in an
            expansion in the value of the long call option with a contraction in value of the
            near-term short call option, resulting in a net profit.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock goes well beyond the call calendar strikes in either direction. This will result
            in the options losing their extrinsic value, which is what you paid for the trade. Intrinsic
            value is completely offset, resulting in a loss. This is an extrinsic value trade.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the short call option loses a lot of value, we can roll it out in time closer to the long
            call option's expiration. This will result in a reduction in our net debit and max loss, but
            it will desensitize the trade's ability to appreciate in value with the short option now
            closer to the long option's expiration.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            The call calendar could see a profit, as long as this expansion is not paired with a bearish
            move. A bearish move may have a greater effect than the increase in implied volatility.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            This could result in a profit if the contraction is paired with a bullish move, as the long
            option could still increase in value to a greater degree than the short option.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            The short call will expire worthless, and you can hold the long call or roll the short call
            into a new expiration to reduce cost basis further.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ATM AT EXPIRATION</div>
          <p>
            As time passes, the short option will decay faster than the long option, and ultimately expire
            worthless. This is the ideal spot — the value of the spread is the decay in the short call
            option plus the remaining extrinsic value in the long call.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The short call will convert to 100 short shares of stock. Your long call still protects your
            risk on the short shares, but buying power will increase dramatically. Close or roll the short
            call to avoid this.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            This strategy is typically not one we will hold to expiration, and we temper our profit target
            because the spread cannot go too far ITM or OTM.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            This is a short-term, vol expansion trade where we are purely trading the extrinsic value and
            IV spread between the short front-month option and the long back-month option. For this reason,
            we look for a quick exit if we see profitability and a move towards our spread.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "put_calendar_spread"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">PUT CALENDAR<br>SPREAD</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          A neutral, defined risk trade where we are betting on an increase in IV while the stock stays
          near our strikes, or for the stock to stay stagnant and our short premium to decay faster than
          our long premium.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↙</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bearish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Low</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">N/A</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">P</div>
          <div class="step-label">Buy put in a long-term expiration cycle</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell put in a near-term expiration cycle</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Variable</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">10–25% of Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Variable</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 95 put in a long-term expiration and sell the 95
            put in a near-term expiration.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div style="width:20px;height:20px;border-radius:50%;border:2px solid #64748b;display:flex;align-items:center;justify-content:center;font-size:9px;color:#94a3b8;">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">95</span>
              <div class="chain-badge red-badge" style="font-size:9px;">P</div>
              <div class="chain-badge green-badge" style="font-size:9px;">P</div>
            </div>
          </div>
          <div class="cal-time-labels">
            <span class="cal-time-label red-text">Short — Near-term</span>
            <span class="cal-time-label" style="color:#22c55e;">Long — Far-term</span>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Put calendar: tent/hill centered at put strike ($95) — mirror of call calendar
      X range $75–$115, 40 pts → scale 15px/$; x(price) = (price-75)*15
      x(75)=0, x(83)=120, x(87)=180, x(95)=300, x(103)=420, x(107)=480, x(115)=600
      Y: range -$250 to +$350 (600 total), height 160
      y(pnl)=(350-pnl)/600*160
      y(+300)=13.3, y(0)=93.3, y(-200)=146.7
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Approximate Payoff at Front Expiration — XYZ $100, buy/sell 95 put (variable, IV-dependent)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red left loss zone -->
        <polygon
          points="0,160 0,146.7 75,141.3 150,114.7 180,93.3 180,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone -->
        <polygon
          points="180,160 180,93.3 225,66.7 300,13.3 375,66.7 420,93.3 420,160"
          fill="rgba(34,197,94,0.15)"
        />
        <!-- Red right loss zone -->
        <polygon
          points="420,160 420,93.3 450,114.7 525,141.3 600,146.7 600,160"
          fill="rgba(220,38,38,0.18)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="93.3" x2="600" y2="93.3" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: stock $100, put strike $95 -->
        <line x1="375" y1="0" x2="375" y2="160" stroke="#64748b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Smooth hill payoff line -->
        <polyline
          points="0,146.7 75,141.3 150,114.7 180,93.3 225,66.7 300,13.3 375,66.7 420,93.3 450,114.7 525,141.3 600,146.7"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="252" y="10" fill="#22c55e" font-size="10" font-family="monospace">Max Profit (Variable)</text>
        <text x="2" y="144" fill="#dc2626" font-size="9" font-family="monospace">Loss ↓</text>
        <text x="490" y="144" fill="#dc2626" font-size="9" font-family="monospace">Loss ↓</text>
        <text x="303" y="155" fill="#dc2626" font-size="8" font-family="monospace">$95P</text>
        <text x="378" y="155" fill="#94a3b8" font-size="8" font-family="monospace">$100</text>
        <text x="140" y="90" fill="#f59e0b" font-size="8" font-family="monospace">BE≈$87</text>
        <text x="423" y="90" fill="#f59e0b" font-size="8" font-family="monospace">BE≈$103</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock trickles down to the put calendar strikes over time. This will result in an
            expansion in the value of the long put option with a contraction in value of the near-term
            short put option, resulting in a net profit.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock goes well beyond the put calendar strikes in either direction. This will result
            in the options losing their extrinsic value, which is what you paid for the trade. Intrinsic
            value is completely offset, resulting in a loss. This is an extrinsic value trade.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the short put option loses a lot of value, we can roll it out in time closer to the long
            put option's expiration. This will result in a reduction in our net debit and max loss, but
            it will desensitize the trade's ability to appreciate in value with the short option now
            closer to the long option's expiration.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            The put calendar will likely see a profit, especially if this is paired with a bearish
            move in the stock price.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            The put calendar will likely see losses, especially if this is paired with a bullish move
            moving the strikes further OTM.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            The short put will expire worthless, and you can hold the long put or roll the short put
            into a new expiration to reduce cost basis further.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ATM AT EXPIRATION</div>
          <p>
            As time passes, the short option will decay faster than the long option, and ultimately
            expire worthless. This is the ideal spot — the value of the spread is the decay in the
            short put option plus the remaining extrinsic value in the long put.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The short put will convert to 100 shares of stock. Your long put still protects your risk
            on the shares, but buying power will increase dramatically. Close or roll the short put
            to avoid this.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            This strategy is typically not one we will hold to expiration, and we temper our profit
            target because the spread cannot go too far ITM or OTM.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            This is a short-term, vol expansion trade where we are purely trading the extrinsic value
            and IV spread between the short front-month put and the long back-month put. For this
            reason, we look for a quick exit if we see profitability and a move towards our spread.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "strangle"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">SHORT STRANGLE</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Neutral, undefined risk strategy consisting of an OTM short put and an OTM short call.
          We want the stock to stay between our strikes through expiration so the options expire
          worthless and we keep the credit received up front as profit.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↔</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Neutral</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">60% to 80%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell an OTM put</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell an OTM call</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Unlimited</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Put Strike − Credit Received &amp; Call Strike + Credit Received</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we might sell the 90 put and 110 call. We do not aim for a specific
            target credit, but trust the premium will be sufficient if the market is liquid.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">110</span>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div style="width:20px;height:20px;border-radius:50%;border:2px solid #64748b;display:flex;align-items:center;justify-content:center;font-size:9px;color:#94a3b8;">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">90</span>
              <div class="chain-badge red-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Short</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      X range $72–$128 (56 pts), width 600 → scale 10.71px/$
      x(price) = (price-72)*10.71
      x(72)=0, x(80)=86, x(87)=161, x(90)=193, x(100)=300, x(110)=407, x(113)=439, x(120)=514, x(128)=600
      Credit=$3.00 → max profit $300; put BE=$87; call BE=$113
      Y: range -$400 to +$400 (800 total), height 160
      y(pnl)=(400-pnl)/800*160
      y(+300)=20, y(0)=80, y(-400)=160 (clamp — unlimited loss)
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — XYZ $100, sell 90 put + 110 call @ $3.00 credit (undefined risk)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Left red loss zone (unlimited downside) -->
        <polygon
          points="0,160 0,80 161,80 161,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone (between BEs) -->
        <polygon
          points="161,160 161,80 193,20 407,20 439,80 439,160"
          fill="rgba(34,197,94,0.15)"
        />
        <!-- Right red loss zone (unlimited upside) -->
        <polygon
          points="439,160 439,80 600,160"
          fill="rgba(220,38,38,0.18)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="80" x2="600" y2="80" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: short put $90, stock $100, short call $110 -->
        <line x1="193" y1="0" x2="193" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#64748b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="407" y1="0" x2="407" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: unlimited loss → slope up to put → flat max profit → slope down → unlimited loss -->
        <polyline
          points="0,160 161,80 193,20 407,20 439,80 600,160"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Unlimited loss arrows -->
        <text x="2" y="155" fill="#dc2626" font-size="9" font-family="monospace">∞ Loss</text>
        <text x="504" y="155" fill="#dc2626" font-size="9" font-family="monospace">∞ Loss</text>

        <!-- Labels -->
        <text x="220" y="16" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $300</text>
        <text x="130" y="77" fill="#f59e0b" font-size="8" font-family="monospace">BE $87</text>
        <text x="442" y="77" fill="#f59e0b" font-size="8" font-family="monospace">BE $113</text>
        <text x="196" y="155" fill="#dc2626" font-size="8" font-family="monospace">$90P</text>
        <text x="285" y="155" fill="#94a3b8" font-size="8" font-family="monospace">$100</text>
        <text x="410" y="155" fill="#dc2626" font-size="8" font-family="monospace">$110C</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock stays between our strikes as time passes. This results in extrinsic value decay
            on both sides and the trade can be bought back for a profit over time.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock moves outside of one of our strikes. The trade now moves from neutral to
            directional. We can start to see losses as the "tested" side increases in value because
            the stock price has moved closer to that side.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            Strangles are undefined risk trades and can be adjusted very easily. If the stock moves
            towards or past one of our strikes, we can roll the other "untested" side closer to the
            "tested" side to pick up additional credit and reduce the delta of the position. We can
            also roll both strikes out in time to add more credit, or a combination of both. If the
            trade is small enough upon entry, we can adjust perpetually, picking up a large credit
            that offsets our breakevens well beyond our strikes.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            Extrinsic value may have increased — we could see a marked extrinsic value loss, but our
            strikes could still be OTM. We hold in this case as these options will expire worthless
            if the stock stays between our strikes.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            Extrinsic value will likely contract — in this case, we close the trade if it's net
            profitable at a percentage we're happy with.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            Both strikes will expire worthless and we will realize max profit.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            We can roll the strikes out in time to add credit and duration to the trade, or close
            the trade if our assumption has changed.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            As the market moves, we may see profitability on one side of a strangle and losses on
            the other. Realize that this is a neutral strategy because the short put hedges the
            short call and vice versa.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            Strangles take risk on both sides of the market, so being tested is pretty normal. We
            need to trade small so that we can roll losers out in time and adjust the strikes if we
            want to, as time &amp; extrinsic value credit are our biggest assets in this type of trade.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "put_broken_wing_butterfly"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">PUT BROKEN WING<br>BUTTERFLY</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Omnidirectional, defined risk trade consisting of a long put spread with a wider OTM short put
          spread to finance the trade and receive a net credit overall. No risk to the upside, but max
          profit at the short strikes.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">✦</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Omnidirectional</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">15 to 45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">60% to 80%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">P</div>
          <div class="step-label">Buy ATM/OTM put</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell two further OTM puts</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 3 -->
        <div class="step-row">
          <div class="step-num">3</div>
          <div class="step-badge green-badge">P</div>
          <div class="step-label">Buy much further OTM put</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Width of Long Spread + Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Short Spread − Long Spread − Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Credit Received or 25% of Long Spread Width</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Short Put Strike − (Long Spread Width + Credit Received)</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 100 put, sell two of the 97 puts, and buy one 91 put
            for a small credit.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div class="chain-badge green-badge">P</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">97</span>
              <div class="chain-badge red-badge">P</div>
              <div class="chain-badge red-badge">P</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">91</span>
              <div class="chain-badge green-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long/Dynamic</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Example: buy 100P, sell 2×97P, buy 91P, credit $1.00
      Max profit = (100-97) + $1 = $4/share = $400
      Max loss = (97-91) - (100-97) - $1 = 6-3-1 = $2/share = $200
      Breakeven = 97 - (3+1) = $93
      Pnl:
        price>100: +100 (credit flat)
        97≤p≤100: (100-p)*100+100 → at 100=100, at 97=400
        91≤p≤97:  100*(p-94)+100 → at 97=400, at 93=100*(93-94)+100=0 ✓, at 91=-200
        p<91: -200 (max loss, flat)

      X range $82–$112 (30 pts), width 600 → scale 20px/$
      x(p)=(p-82)*20; x(82)=0, x(91)=180, x(93)=220, x(97)=300, x(100)=360, x(104)=440, x(112)=600
      Y: range -300 to +500 (800 total), height 160
      y(pnl)=(500-pnl)/800*160
      y(+400)=12.5, y(+100)=50, y(0)=100, y(-200)=140
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 100P, sell 2×97P, buy 91P @ $1.00 credit (defined risk)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red loss zone: price < breakeven $93 -->
        <polygon
          points="0,160 0,140 180,140 220,100 220,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone: price > $93 (including upside flat credit) -->
        <polygon
          points="220,160 220,100 300,12.5 360,50 600,50 600,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="100" x2="600" y2="100" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: long put $91, short puts $97, long put $100 -->
        <line x1="180" y1="0" x2="180" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="360" y1="0" x2="360" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line -->
        <polyline
          points="0,140 180,140 220,100 300,12.5 360,50 600,50"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="247" y="9" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $400</text>
        <text x="2" y="137" fill="#dc2626" font-size="9" font-family="monospace">Loss $200</text>
        <text x="370" y="46" fill="#22c55e" font-size="9" font-family="monospace">No upside risk →</text>
        <text x="183" y="155" fill="#22c55e" font-size="8" font-family="monospace">$91P</text>
        <text x="220" y="97" fill="#f59e0b" font-size="8" font-family="monospace">BE $93</text>
        <text x="303" y="155" fill="#dc2626" font-size="8" font-family="monospace">$97P×2</text>
        <text x="363" y="155" fill="#22c55e" font-size="8" font-family="monospace">$100P</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            Put broken wing butterflies are similar to put ratio spreads, but are defined risk. Max
            profit occurs at the short strikes, where the long put spread would realize max value and
            the short put spread would be worthless at expiration. The spread has no risk to the upside
            if entered for a credit, so the position can also be profitable with an upside move in the
            stock.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            If the spread moves fully ITM at expiration, you will realize max loss on the trade.
            Additionally, if the stock price moves too quickly towards the spread, you can see an
            extrinsic value loss on the trade since the bulk of the potential profit on a trade like
            this requires extrinsic value to be low. We look to remove risk if the spread moves further
            OTM by rolling into a symmetrical butterfly for a debit that's less than the credit received
            upon entry.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the long spread is ITM and near max value, we sell out of it to retain that value and
            either hold the credit spread, or adjust the trade into something else like an iron condor.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            This trade doesn't have a ton of exposure to vega since it's defined risk, but this could
            result in an extrinsic value marked loss.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            This trade doesn't have a ton of exposure to vega since it's defined risk, but this may
            make it easier to "fly off" the risk by rolling into a symmetrical butterfly for a debit
            that's less than the credit received upon entry.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The trade would be at max loss if it is completely ITM. Close the trade to avoid assignment
            and move on.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            We can sell out of the spread for a profit if we're in our profit zone, since our long spread
            will increase in value and the short spread will decrease in value if it is OTM.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            The strikes would expire worthless and we can keep our credit collected up front as profit.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Put Broken Wing Butterflies are frequently used in products that have put skew, because we
            can make them wider or collect a larger credit up front.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            Broken Wing Butterflies don't appreciate in value too much until closer to expiration when
            extrinsic value gets closer to zero, so our initial goal with BWBs is to remove risk by
            rolling into a symmetrical butterfly if the spread moves further OTM. If we can do this for
            a debit less than the initial credit received, we lock in a small profit and remove initial
            risk from the trade.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "call_broken_wing_butterfly"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">CALL BROKEN WING<br>BUTTERFLY</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Omnidirectional, defined risk trade consisting of a long call spread with a wider OTM short call
          spread to finance the trade and receive a net credit overall. No risk to the downside, but max
          profit at the short strikes.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">✦</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Omnidirectional</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">15 to 45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">60% to 80%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">C</div>
          <div class="step-label">Buy ATM/OTM call</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell two further OTM calls</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 3 -->
        <div class="step-row">
          <div class="step-num">3</div>
          <div class="step-badge green-badge">C</div>
          <div class="step-label">Buy much further OTM call</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Width of Long Spread + Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Short Spread − Long Spread − Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Credit Received or 25% of Long Spread Width</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Short Call Strike + (Long Spread Width + Credit Received)</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 100 call, sell two of the 103 calls, and buy one 109
            call for a small credit.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">109</span>
              <div class="chain-badge green-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">103</span>
              <div class="chain-badge red-badge">C</div>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div class="chain-badge green-badge">C</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Short/Dynamic</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Example: buy 100C, sell 2×103C, buy 109C, credit $1.00
      Max profit = (103-100) + $1 = $4/share = $400 at $103
      Max loss = (109-103) - (103-100) - $1 = 6-3-1 = $2/share = $200 (upside only)
      Breakeven = 103 + (3+1) = $107
      No downside risk: flat +$100 below $100

      X range $82–$118 (36 pts), width 600 → scale 16.667px/$
      x(82)=0, x(100)=300, x(103)=350, x(107)=417, x(109)=450, x(118)=600
      Y: range -300 to +500 (800 total), height 160
      y(pnl)=(500-pnl)/800*160
      y(+400)=20, y(+100)=80, y(0)=100, y(-200)=140
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 100C, sell 2×103C, buy 109C @ $1.00 credit (defined risk)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Green zone: flat credit + profit tent (left of BE $107) -->
        <polygon
          points="0,160 0,80 300,80 350,20 417,100 417,160"
          fill="rgba(34,197,94,0.15)"
        />
        <!-- Red loss zone: price > BE $107 -->
        <polygon
          points="417,160 417,100 450,140 600,140 600,160"
          fill="rgba(220,38,38,0.18)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="100" x2="600" y2="100" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: long call $100, short calls $103, outer wing $109 -->
        <line x1="300" y1="0" x2="300" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="350" y1="0" x2="350" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="450" y1="0" x2="450" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line -->
        <polyline
          points="0,80 300,80 350,20 417,100 450,140 600,140"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="260" y="17" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $400</text>
        <text x="2" y="76" fill="#22c55e" font-size="9" font-family="monospace">← No downside risk</text>
        <text x="455" y="137" fill="#dc2626" font-size="9" font-family="monospace">Loss $200</text>
        <text x="303" y="155" fill="#22c55e" font-size="8" font-family="monospace">$100C</text>
        <text x="353" y="155" fill="#dc2626" font-size="8" font-family="monospace">$103C×2</text>
        <text x="417" y="97" fill="#f59e0b" font-size="8" font-family="monospace">BE $107</text>
        <text x="453" y="155" fill="#22c55e" font-size="8" font-family="monospace">$109C</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            Call broken wing butterflies are similar to call ratio spreads, but are defined risk. Max
            profit occurs at the short strikes, where the long call spread would realize max value and
            the short call spread would be worthless at expiration. The spread has no risk to the downside
            if entered for a credit, so the position can also be profitable with a downside move in the
            stock.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            If the spread moves fully ITM at expiration, you will realize max loss on the trade.
            Additionally, if the stock price moves too quickly towards the spread, you can see an
            extrinsic value loss on the trade since the bulk of the potential profit on a trade like
            this requires extrinsic value to be low. We look to remove risk if the spread moves further
            OTM by rolling into a symmetrical butterfly for a debit that's less than the credit received
            upon entry.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the long spread is ITM and near max value, we sell out of it to retain that value and
            either hold the credit spread, or manipulate the trade into something else like an iron condor.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            This trade doesn't have a ton of exposure to vega since it's defined risk, but this could
            result in an extrinsic value marked loss.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            This trade doesn't have a ton of exposure to vega since it's defined risk, but this may
            make it easier to "fly off" the risk by rolling into a symmetrical butterfly for a debit
            that's less than the credit received upon entry.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The trade would be at max loss if it is completely ITM. Close the trade to avoid assignment
            and move on.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            We can sell out of the spread for a profit if we're in our profit zone, since our long spread
            will increase in value and the short spread will decrease in value if it is OTM.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            The strikes would expire worthless and we can keep our credit collected up front as profit.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Call Broken Wing Butterflies are frequently used in products that have call skew, because we
            can make them wider or collect a larger credit up front.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            Broken Wing Butterflies don't appreciate in value too much until closer to expiration when
            extrinsic value gets closer to zero, so our initial goal with BWBs is to remove risk by
            rolling into a symmetrical butterfly if the spread moves further OTM. If we can do this for
            a debit less than the initial credit received, we lock in a small profit and remove initial
            risk from the trade.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "jade_lizard"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">JADE LIZARD</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          A bullish position that is constructed by selling an OTM short put combined with an OTM short
          call spread, where the total credit received is greater than the width of the call spread to
          remove upside risk entirely.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">✦</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Neutral-Bullish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">60% to 80%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell an OTM put</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell an OTM vertical call spread</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Short Put Strike × 100 − Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Short Put Strike − Credit Received</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may sell the 95 put and sell the 105/110 call spread for a credit
            over $5.00.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">110</span>
              <div class="chain-badge green-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">105</span>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <span class="chain-price" style="color:#94a3b8;">○ stock</span>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">95</span>
              <div class="chain-badge red-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Short</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Example: sell 95P, sell 105/110 call spread, credit $5.50
      BE = 95 - 5.50 = $89.50
      Max profit = $550 (stock stays between 95 and 105)
      No upside risk (credit $5.50 > spread width $5)
      Above 110: flat +$50

      X range $84–$118 (34 pts), 600px → scale 17.65px/$
      x(84)=0, x(89.5)=97, x(95)=194, x(105)=371, x(110)=459, x(118)=600
      Y range -700 to +700 (1400 total), height 160
      y(pnl)=(700-pnl)/1400*160
      y(+550)=17, y(+50)=74, y(0)=80, y(-550)=143
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — sell 95P, sell 105/110C spread @ $5.50 credit (no upside risk)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red loss zone: price below BE $89.50 -->
        <polygon
          points="0,160 0,143 97,80 97,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone: price above BE -->
        <polygon
          points="97,160 97,80 194,17 371,17 459,74 600,74 600,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="80" x2="600" y2="80" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: short put $95, short call $105, long call $110 -->
        <line x1="194" y1="0" x2="194" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="371" y1="0" x2="371" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="459" y1="0" x2="459" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line -->
        <polyline
          points="0,143 97,80 194,17 371,17 459,74 600,74"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="220" y="13" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $550</text>
        <text x="2" y="140" fill="#dc2626" font-size="9" font-family="monospace">Downside risk</text>
        <text x="462" y="71" fill="#22c55e" font-size="9" font-family="monospace">No upside risk →</text>
        <text x="97" y="77" fill="#f59e0b" font-size="8" font-family="monospace">BE $89.50</text>
        <text x="197" y="155" fill="#dc2626" font-size="8" font-family="monospace">$95P</text>
        <text x="374" y="155" fill="#dc2626" font-size="8" font-family="monospace">$105C</text>
        <text x="462" y="155" fill="#22c55e" font-size="8" font-family="monospace">$110C</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock stays between our short strikes. There is no risk to the upside if our net credit
            is greater than the width of the call credit spread, but max profit is realized if all
            options expire OTM.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock goes down. We have a naked short put, so if the stock drops below our short put
            strike, we take on intrinsic value losses equivalent to 100 shares of stock, less the credit
            received from selling the jade lizard up front.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the short put is ITM, we can either roll that out in time, or roll the call spread down
            to defend the short put. We just ensure that there is no risk to the upside by keeping the
            net credit higher than the width of the call spread we roll to.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            We may hold — this may result in an extrinsic value loss, but extrinsic value will always go
            to zero by expiration.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            We look to close the position for a profit if our strikes are still OTM.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            All options will expire worthless and we'll keep the credit received up front as profit.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            We typically close the trade for a profit to ensure we do not end up with short shares in
            the next trading session.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The short put can be rolled out in time if we don't mind being bullish on the product for
            another cycle, and we can roll the call spread out in time as well if we want to keep that
            portion of the trade on.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            The most important aspect of the jade lizard is to ensure the net credit received is greater
            than the width of the call spread — this ensures we have no risk to the upside, and increases
            our probability of profit substantially.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            Because we are taking risk on the call side in this trade, ensure that the premium we are
            collecting on the call spread is around 1/3rd the width. There is no reason to take risk on
            that side or reduce our potential max profit if we're not collecting a fair amount to do so.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "put_butterfly"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">PUT BUTTERFLY</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Symmetrical long put spread and short put spread that share the same short strikes.
          This is a low probability trade because we pay for it up front and need the stock
          to be within our strikes at expiration.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">✦</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bearish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Any</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">15 to 45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">20% to 40%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">P</div>
          <div class="step-label">Buy an ATM/OTM put</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell 2 further OTM puts</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 3 -->
        <div class="step-row">
          <div class="step-num">3</div>
          <div class="step-badge green-badge">P</div>
          <div class="step-label">Buy further OTM put for equidistant spreads</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Width of Long Spread − Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">25% of Long Spread Width</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Long Put Strike ± Debit Paid (two BEs)</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 100 put, sell two of the 95 puts,
            and buy one 90 put for a small debit (~$0.60).
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div class="chain-badge green-badge">P</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">95</span>
              <div class="chain-badge red-badge">P</div>
              <div class="chain-badge red-badge">P</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">90</span>
              <div class="chain-badge green-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Short / Dynamic</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Example: buy 100P, sell 2×95P, buy 90P, debit $0.60
      Max profit = (100-95) - 0.60 = $4.40/share = $440 at $95
      Max loss = $0.60/share = $60 above $100 or below $90
      Upper BE = 100 - 0.60 = 99.40
      Lower BE = 90 + 0.60 = 90.60

      X range $83–$107 = 24 pts, 600px → 25px/pt
      x(p) = (p - 83) * 25
      x(83)=0  x(90)=175  x(90.6)=190  x(95)=300  x(99.4)=410  x(100)=425  x(107)=600

      Y: range -100 to 540 = 640 total, height 160
      y(pnl) = (540 - pnl) / 640 * 160
      y(+440) = 100/640*160 = 25
      y(0)    = 540/640*160 = 135
      y(-60)  = 600/640*160 = 150
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 100P, sell 2×95P, buy 90P @ $0.60 debit (defined risk)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Left red loss zone: price ≤ lower BE $90.60 -->
        <polygon
          points="0,160 0,150 175,150 190,135 190,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Right red loss zone: price ≥ upper BE $99.40 -->
        <polygon
          points="410,160 410,135 425,150 600,150 600,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone: between the two BEs -->
        <polygon
          points="190,160 190,135 300,25 410,135 410,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="135" x2="600" y2="135" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals at key strikes -->
        <line x1="175" y1="0" x2="175" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="425" y1="0" x2="425" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: flat loss → lower BE → peak → upper BE → flat loss -->
        <polyline
          points="0,150 175,150 190,135 300,25 410,135 425,150 600,150"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="247" y="20" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $440</text>
        <text x="2" y="147" fill="#dc2626" font-size="9" font-family="monospace">Loss $60</text>
        <text x="488" y="147" fill="#dc2626" font-size="9" font-family="monospace">Loss $60</text>
        <text x="303" y="155" fill="#dc2626" font-size="8" font-family="monospace">$95P×2</text>
        <text x="178" y="155" fill="#22c55e" font-size="8" font-family="monospace">$90P</text>
        <text x="428" y="155" fill="#22c55e" font-size="8" font-family="monospace">$100P</text>
        <text x="148" y="132" fill="#f59e0b" font-size="8" font-family="monospace">BE $90.60</text>
        <text x="412" y="132" fill="#f59e0b" font-size="8" font-family="monospace">BE $99.40</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="section-title">TASTYLIVE APPROACH</div>

    <div class="approach-section">
      <div class="approach-title">HOW THE TRADE WORKS</div>
      <div class="approach-grid">
        <div class="approach-card green-border">
          <div class="approach-card-title">IDEAL</div>
          <p>The stock is between our strikes at expiration and we sell out of the butterfly for a higher amount than we bought it for.</p>
        </div>
        <div class="approach-card red-border">
          <div class="approach-card-title">NOT IDEAL</div>
          <p>The spread is completely ITM or OTM and we realize max loss of the debit paid up front.</p>
        </div>
      </div>
    </div>

    <div class="approach-section">
      <div class="approach-title">EXPIRATION</div>
      <div class="approach-grid">
        <div class="approach-card red-border">
          <div class="approach-card-title">IF ITM AT EXPIRATION</div>
          <p>We close the trade for max loss to avoid assignment.</p>
        </div>
        <div class="approach-card gray-border">
          <div class="approach-card-title">IF OTM AT EXPIRATION</div>
          <p>We close the trade for max loss to avoid assignment.</p>
        </div>
        <div class="approach-card green-border">
          <div class="approach-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>We look to sell the butterfly for a profit. Avoid holding through expiration as this can result in unwanted shares.</p>
        </div>
      </div>
    </div>

    <div class="approach-section">
      <div class="approach-title">TAKEAWAYS</div>
      <div class="takeaway-list">
        <div class="takeaway-item">
          <div class="takeaway-num">1</div>
          <p>These trades are low probability because the range of success is so small relative to normal stock price movement for the cycle. We like to roll into equidistant butterflies from broken wing butterflies for this reason, as opposed to starting with them.</p>
        </div>
        <div class="takeaway-item">
          <div class="takeaway-num">2</div>
          <p>The less time we have to expiration, the more we can expect to get out of a butterfly if the stock price moves through it. Too much extrinsic value will prevent the trade from moving much at all.</p>
        </div>
      </div>
    </div>

  {:else if strategy === "call_butterfly"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">CALL BUTTERFLY</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Symmetrical long call spread and short call spread that share the same short strikes.
          This is a low probability trade because we pay for it up front and need the stock
          to be within our strikes at expiration.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">✦</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bullish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Any</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">15 to 45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">20% to 40%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">C</div>
          <div class="step-label">Buy an ATM/OTM call</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell 2 further OTM calls</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 3 -->
        <div class="step-row">
          <div class="step-num">3</div>
          <div class="step-badge green-badge">C</div>
          <div class="step-label">Buy further OTM call for equidistant spreads</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Width of Long Spread − Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">25% of Long Spread Width</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Long Call Strike + Debit Paid (two BEs)</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 100 call, sell two of the 105 calls,
            and buy one 110 call for a small debit (~$0.60).
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">110</span>
              <div class="chain-badge green-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">105</span>
              <div class="chain-badge red-badge">C</div>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div class="chain-badge green-badge">C</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long / Dynamic</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Example: buy 100C, sell 2×105C, buy 110C, debit $0.60
      Max profit = (105-100) - 0.60 = $4.40/share = $440 at $105
      Max loss = $0.60/share = $60 below $100 or above $110
      Lower BE = 100 + 0.60 = 100.60
      Upper BE = 110 - 0.60 = 109.40

      X range $93–$117 = 24 pts, 600px → 25px/pt
      x(p) = (p - 93) * 25
      x(93)=0  x(100)=175  x(100.6)=190  x(105)=300  x(109.4)=410  x(110)=425  x(117)=600

      Y: range -100 to 540 = 640 total, height 160
      y(pnl) = (540 - pnl) / 640 * 160
      y(+440) = 25    y(0) = 135    y(-60) = 150
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 100C, sell 2×105C, buy 110C @ $0.60 debit (defined risk)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Left red loss zone: below lower BE $100.60 -->
        <polygon
          points="0,160 0,150 175,150 190,135 190,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Right red loss zone: above upper BE $109.40 -->
        <polygon
          points="410,160 410,135 425,150 600,150 600,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone: between the two BEs -->
        <polygon
          points="190,160 190,135 300,25 410,135 410,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="135" x2="600" y2="135" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals at key strikes -->
        <line x1="175" y1="0" x2="175" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="425" y1="0" x2="425" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: flat loss → lower BE → peak → upper BE → flat loss -->
        <polyline
          points="0,150 175,150 190,135 300,25 410,135 425,150 600,150"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="247" y="20" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $440</text>
        <text x="2" y="147" fill="#dc2626" font-size="9" font-family="monospace">Loss $60</text>
        <text x="488" y="147" fill="#dc2626" font-size="9" font-family="monospace">Loss $60</text>
        <text x="303" y="155" fill="#dc2626" font-size="8" font-family="monospace">$105C×2</text>
        <text x="178" y="155" fill="#22c55e" font-size="8" font-family="monospace">$100C</text>
        <text x="428" y="155" fill="#22c55e" font-size="8" font-family="monospace">$110C</text>
        <text x="140" y="132" fill="#f59e0b" font-size="8" font-family="monospace">BE $100.60</text>
        <text x="412" y="132" fill="#f59e0b" font-size="8" font-family="monospace">BE $109.40</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="section-title">TASTYLIVE APPROACH</div>

    <div class="approach-section">
      <div class="approach-title">HOW THE TRADE WORKS</div>
      <div class="approach-grid">
        <div class="approach-card green-border">
          <div class="approach-card-title">IDEAL</div>
          <p>The stock is between our strikes at expiration and we sell out of the butterfly for a higher amount than we bought it for.</p>
        </div>
        <div class="approach-card red-border">
          <div class="approach-card-title">NOT IDEAL</div>
          <p>The spread is completely ITM or OTM and we realize max loss of the debit paid up front.</p>
        </div>
      </div>
    </div>

    <div class="approach-section">
      <div class="approach-title">EXPIRATION</div>
      <div class="approach-grid">
        <div class="approach-card red-border">
          <div class="approach-card-title">IF OTM AT EXPIRATION</div>
          <p>We close the trade for max loss to avoid assignment.</p>
        </div>
        <div class="approach-card gray-border">
          <div class="approach-card-title">IF ITM AT EXPIRATION</div>
          <p>We close the trade for max loss to avoid assignment.</p>
        </div>
        <div class="approach-card green-border">
          <div class="approach-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>We look to sell the butterfly for a profit. Avoid holding through expiration as this can result in unwanted shares.</p>
        </div>
      </div>
    </div>

    <div class="approach-section">
      <div class="approach-title">TAKEAWAYS</div>
      <div class="takeaway-list">
        <div class="takeaway-item">
          <div class="takeaway-num">1</div>
          <p>These trades are low probability because the range of success is so small relative to normal stock price movement for the cycle. We like to roll into equidistant butterflies from broken wing butterflies for this reason, as opposed to starting with them.</p>
        </div>
        <div class="takeaway-item">
          <div class="takeaway-num">2</div>
          <p>The less time we have to expiration, the more we can expect to get out of a butterfly if the stock price moves through it. Too much extrinsic value will prevent the trade from moving much at all.</p>
        </div>
      </div>
    </div>

  {:else if strategy === "iron_butterfly"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">IRON FLY</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Neutral, defined risk strategy consisting of an ATM put credit spread and ATM call credit
          spread. We want the stock to stay between our breakeven prices through expiration.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↔</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Neutral</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">60% to 80%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div style="display:flex;gap:3px;">
            <div class="step-badge red-badge">P</div>
            <div class="step-badge red-badge">C</div>
          </div>
          <div class="step-label">Sell a straddle (ATM put + ATM call)</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge green-badge">P</div>
          <div class="step-label">Buy an OTM put</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 3 -->
        <div class="step-row">
          <div class="step-num">3</div>
          <div class="step-badge green-badge">C</div>
          <div class="step-label">Buy an OTM call</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Widest Spread − Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">25% of Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Short Put Strike − Credit &amp; Short Call Strike + Credit</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may sell the 100/90 put spread and the 100/110 call spread
            and look to collect $5.00.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">110</span>
              <div class="chain-badge green-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">100</span>
              <div class="chain-badge red-badge">P</div>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">90</span>
              <div class="chain-badge green-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Flat</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Sell 100P + 100C (straddle), buy 90P + 110C, credit $5.00
      Max profit = $500 at $100; Max loss = $500 (1:1 risk/reward)
      Put BE = $95; Call BE = $105
      X range $82–$118, 36 pts, 600px → scale 16.67px/$
      x(p)=(p-82)*16.67; x(90)=133, x(95)=217, x(100)=300, x(105)=383, x(110)=467
      Y: range -600 to +600 (1200 total), height 160
      y(pnl)=(600-pnl)/1200*160
      y(+500)=13.3, y(0)=80, y(-500)=146.7
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — sell 100P+100C, buy 90P+110C @ $5.00 credit (1:1 risk/reward)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Left red loss zone -->
        <polygon
          points="0,160 0,146.7 133,146.7 217,80 217,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone (sharp triangle) -->
        <polygon
          points="217,160 217,80 300,13.3 383,80 383,160"
          fill="rgba(34,197,94,0.15)"
        />
        <!-- Right red loss zone -->
        <polygon
          points="383,160 383,80 467,146.7 600,146.7 600,160"
          fill="rgba(220,38,38,0.18)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="80" x2="600" y2="80" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: long put $90, short straddle $100, long call $110 -->
        <line x1="133" y1="0" x2="133" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,3" />
        <line x1="467" y1="0" x2="467" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Sharp tent payoff line -->
        <polyline
          points="0,146.7 133,146.7 217,80 300,13.3 383,80 467,146.7 600,146.7"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="240" y="10" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $500</text>
        <text x="2" y="144" fill="#dc2626" font-size="9" font-family="monospace">Loss $500</text>
        <text x="470" y="144" fill="#dc2626" font-size="9" font-family="monospace">Loss $500</text>
        <text x="136" y="155" fill="#22c55e" font-size="8" font-family="monospace">$90P</text>
        <text x="168" y="77" fill="#f59e0b" font-size="8" font-family="monospace">BE$95</text>
        <text x="285" y="155" fill="#dc2626" font-size="8" font-family="monospace">$100</text>
        <text x="350" y="77" fill="#f59e0b" font-size="8" font-family="monospace">BE$105</text>
        <text x="470" y="155" fill="#22c55e" font-size="8" font-family="monospace">$110C</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock stays between our breakeven points as time passes. This results in extrinsic value
            decay on both sides and the trade can be bought back for a profit over time.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock moves outside of one of our breakeven points. The trade goes from neutral to
            directional if this happens. We can start to see losses as the "tested" side increases in
            value as the stock price moves closer to that side.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            Iron flies are like a defined-risk straddle. Because of this, management is limited compared
            to an undefined risk trade. If one of the sides is completely ITM, it cannot be rolled for a
            credit, so there is not much we can do with this trade if the stock moves outside of our long
            strikes without manipulating risk.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            We may hold — this may result in an extrinsic value loss. However, if the stock is still
            within our breakeven points, we know that the trade can be profitable at expiration.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            The extrinsic value may collapse to a point where we can buy the trade back for 25% of
            max profit, which is our target.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            This strategy will always have one strike that is ITM, so we have to either close or roll
            the trade prior to expiration to avoid assignment of shares.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            We can still see profitability if we are within our breakeven ranges. We close the trade
            for a profit if this is the case to avoid assignment.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            If the spread is completely ITM at expiration, we can close the trade to avoid assignment
            and move on.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            With spreads, it's important to realize that options will be exercised if they are ITM and
            held through expiration. If one strike is ITM and the other moves OTM, close the trade prior
            to expiration to avoid unwanted shares.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            Target risk:reward is 1:1 so that we do not have an impractical spread width. Defensive
            management is limited, so we want to have a nice wide breakeven range.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "long_call_vertical"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">LONG CALL<br>VERTICAL SPREAD</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Bullish defined risk debit trade where we are betting on the stock moving above our short
          call strike price by the expiration of our contract. Spread width depends on account size,
          risk tolerance, etc.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↗</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bullish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Any</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">40% to 60%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">C</div>
          <div class="step-label">Buy an ITM call</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell an ATM/OTM call</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Distance Between Strikes − Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Long Call Strike + Debit Paid</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 95/105 call spread and look to pay ~$5.00 debit.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">105</span>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div class="chain-circle">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">95</span>
              <div class="chain-badge green-badge">C</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Flat</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Buy 95C, sell 105C, debit $5.00
      Max profit = (105−95)−5 = $500 (above $105); Max loss = $500 (below $95)
      Breakeven = 95 + 5 = $100
      X range $82–$118, 36 pts, 600px → scale 16.67px/$
      x(p)=(p-82)*16.67; x(95)=217, x(100)=300, x(105)=383
      Y: range -600 to +600 (1200 total), height 160
      y(pnl)=(600-pnl)/1200*160
      y(+500)=13.3, y(0)=80, y(-500)=146.7
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 95C / sell 105C @ $5.00 debit</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red loss zone (left, below breakeven) -->
        <polygon
          points="0,160 0,146.7 217,146.7 300,80 300,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone (right, above breakeven) -->
        <polygon
          points="300,160 300,80 383,13.3 600,13.3 600,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="80" x2="600" y2="80" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: long 95C (green), BE $100 (amber), short 105C (red) -->
        <line x1="217" y1="0" x2="217" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#f59e0b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="383" y1="0" x2="383" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: flat loss → slope up → flat profit -->
        <polyline
          points="0,146.7 217,146.7 300,80 383,13.3 600,13.3"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="400" y="10" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $500</text>
        <text x="2" y="144" fill="#dc2626" font-size="9" font-family="monospace">Loss $500</text>
        <text x="220" y="155" fill="#22c55e" font-size="8" font-family="monospace">95C long</text>
        <text x="267" y="77" fill="#f59e0b" font-size="8" font-family="monospace">BE $100</text>
        <text x="386" y="155" fill="#dc2626" font-size="8" font-family="monospace">105C short</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            Stock increases in value. A long call spread is directionally bullish — ideally the stock
            price rises so that the long call strike increases in value to a greater degree than the
            short call, resulting in a profit.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            Stock decreases in value. The long call spread would decrease in value, which means the
            spread will be less valuable to sell to close compared to the original purchase price,
            resulting in a loss.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            Long call spreads trade for a debit, which means extending duration actually increases
            risk since we'd pay another debit to roll the trade out in time. We can roll the short
            call down closer to the long call to reduce the net debit, but we don't roll below our
            breakeven price.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            We may hold the position — this may be paired with a sell-off in the stock price, but
            our risk is capped at the debit paid so we typically let the trade play out. However, we
            can close if our assumption has changed.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            We may hold the position — if this is paired with a bullish move in the stock price, we
            may see profit in the spread and can close if we're happy with the trade.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            Trade is at max loss; both options lost all value. Let the trade expire worthless.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            Both options trading for intrinsic value, trade is at max profit. To avoid assignment
            fees and the possibility of one option moving OTM, close the trade prior to expiration.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            Either close the trade or roll out in time to extend it. Avoid letting these trades go
            through expiration — if the long call is ITM and the short call is OTM, you can come
            back to the market with 100 shares of stock.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Vertical spreads have a less volatile P/L because of the long option that defines our
            risk. If we see profit on the short option, we will see losses on the long option and
            vice versa. For this reason, expect to be in spread trades longer than naked options to
            reach profit targets.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            With spreads, it's important to realize that options will be exercised if they are ITM
            and held through expiration. If one strike is ITM and the other moves OTM, close the
            trade prior to expiration to avoid unwanted shares.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "long_put_vertical"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">LONG PUT<br>VERTICAL SPREAD</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Bearish, defined risk debit trade where we are betting on the stock moving below our short
          put strike price by the expiration of our contract. Spread width depends on account size,
          risk tolerance, etc.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↘</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bearish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Any</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">50% to 60%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">P</div>
          <div class="step-label">Buy an ITM put</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell an ATM/OTM put</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Distance Between Strikes − Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Long Put Strike − Debit Paid</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we might look to buy the 105/95 put spread and pay around $5.00.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">105</span>
              <div class="chain-badge green-badge">P</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div class="chain-circle">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">95</span>
              <div class="chain-badge red-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Flat</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Buy 105P, sell 95P, debit $5.00
      Max profit = (105−95)−5 = $500 (below $95); Max loss = $500 (above $105)
      Breakeven = 105 − 5 = $100
      X range $82–$118, 36 pts, 600px → scale 16.67px/$
      x(p)=(p-82)*16.67; x(95)=217, x(100)=300, x(105)=383
      Y: range -600 to +600 (1200 total), height 160
      y(pnl)=(600-pnl)/1200*160
      y(+500)=13.3, y(0)=80, y(-500)=146.7
      Payoff: flat profit left → slope up → flat loss right (mirror of LCV)
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 105P / sell 95P @ $5.00 debit</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Green profit zone (left, below breakeven) -->
        <polygon
          points="0,160 0,13.3 217,13.3 300,80 300,160"
          fill="rgba(34,197,94,0.15)"
        />
        <!-- Red loss zone (right, above breakeven) -->
        <polygon
          points="300,160 300,80 383,146.7 600,146.7 600,160"
          fill="rgba(220,38,38,0.18)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="80" x2="600" y2="80" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: short 95P (red), BE $100 (amber), long 105P (green) -->
        <line x1="217" y1="0" x2="217" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#f59e0b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="383" y1="0" x2="383" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: flat profit → slope down → flat loss -->
        <polyline
          points="0,13.3 217,13.3 300,80 383,146.7 600,146.7"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="2" y="10" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $500</text>
        <text x="450" y="144" fill="#dc2626" font-size="9" font-family="monospace">Loss $500</text>
        <text x="220" y="155" fill="#dc2626" font-size="8" font-family="monospace">95P short</text>
        <text x="267" y="77" fill="#f59e0b" font-size="8" font-family="monospace">BE $100</text>
        <text x="386" y="155" fill="#22c55e" font-size="8" font-family="monospace">105P long</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock decreases in value. A long put spread is a directionally bearish position —
            so ideally the stock price decreases so that the long put strike increases in value to
            a greater degree than the short put.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock increases in value. The value of the long put spread would decrease, which
            means the spread will be less valuable to sell to close compared to the original
            purchase price, resulting in a loss.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            Long put spreads are debit trades which means extending duration actually increases risk
            since we'd pay another debit to roll the trade out in time. We can roll the short put up
            closer to the long put to bring in more credit and reduce our net debit paid and reduce
            max profit potential, but we do not roll above our breakeven price.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            Extrinsic value may have increased — but this is primarily a bearish trade and if the
            increase in IV is paired with a bearish move, we may see profitability and can close if
            we are happy with the exit price.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            Extrinsic value may have decreased, but this could be paired with a rally in the
            product. We may consider holding the position, or rolling the short option up closer to
            the long option, but not above the breakeven price.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            The trade will be at max loss, as both options will have lost all of their value.
            We let the trade expire worthless.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The trade will be at max profit. We close the trade.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            We either roll out in time to extend the trade or close it. We avoid letting these
            trades go through expiration — if the long put is ITM and the short put is OTM we can
            come back to the market the next trading session with 100 shares of short stock.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Vertical spreads have a less volatile P/L because of the long option that defines our
            risk. If we see profit on the short option, we will see losses on the long option and
            vice versa. For this reason, we should expect to be in spread trades longer than naked
            options to reach profit targets.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            With spreads, it's important to realize that options will be exercised if they are ITM
            and held through expiration. If one strike is ITM and the other moves OTM, close the
            trade prior to expiration to avoid unwanted shares.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "pmcc"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">POOR MAN'S<br>COVERED CALL</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          A bullish synthetic covered call strategy that consists of an ITM long-term call to
          replicate 100 shares of stock, with an ATM/OTM short call in a near-term cycle to reduce
          cost basis. Net debit should not exceed width between strikes.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↗</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bullish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Low</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45 to 60</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">50% to 60%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">C</div>
          <div class="step-label">Buy an ITM call in a long-term expiration cycle</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell an OTM call in a near-term expiration cycle</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Distance Between Strikes − Debit Paid + Estimated Extrinsic Value in Long Option</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Estimated Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Long Call Strike + Debit Paid</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 90 call in a long-term expiration and sell the
            105 call in a near-term expiration.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">105</span>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div class="chain-circle">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">90</span>
              <div class="chain-badge green-badge">C</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Buy 90C (long-term), sell 105C (near-term), net debit $10
      Max profit = (105-90)-10 = $500 (above $105); Max loss = $1000 (below $90)
      Breakeven = 90 + 10 = $100
      X range $82–$118, 36 pts, 600px → scale 16.67px/$
      x(p)=(p-82)*16.67; x(90)=133, x(100)=300, x(105)=383
      Y range -1100 to +600 (1700 total), height 160
      y(pnl)=(600-pnl)/1700*160
      y(+500)=9.4, y(0)=56.5, y(-1000)=150.6
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 90C long-term / sell 105C near-term @ $10 debit</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red loss zone (left, below breakeven) -->
        <polygon
          points="0,160 0,150.6 133,150.6 300,56.5 300,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone (right, above breakeven) -->
        <polygon
          points="300,160 300,56.5 383,9.4 600,9.4 600,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="56.5" x2="600" y2="56.5" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: long 90C (green), BE $100 (amber), short 105C (red) -->
        <line x1="133" y1="0" x2="133" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#f59e0b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="383" y1="0" x2="383" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: flat max loss → slope up → flat max profit -->
        <polyline
          points="0,150.6 133,150.6 300,56.5 383,9.4 600,9.4"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="390" y="7" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $500</text>
        <text x="2" y="148" fill="#dc2626" font-size="9" font-family="monospace">Loss $1000</text>
        <text x="136" y="155" fill="#22c55e" font-size="8" font-family="monospace">90C long</text>
        <text x="258" y="53" fill="#f59e0b" font-size="8" font-family="monospace">BE $100</text>
        <text x="386" y="155" fill="#dc2626" font-size="8" font-family="monospace">105C short</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock moves up to the short call strike by the expiration of the contract. This
            results in max extrinsic value collected from the short call, as well as max value
            gained on the long call option that acts as a synthetic long stock position, plus any
            remaining extrinsic value in the long option as well.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock goes down. This results in losses on the long call you own, although the
            short call will lose value and hedge the loss on the long call, and you would not lose
            as much as owning 100 shares of stock if there is a big selloff.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the short call loses value, we can roll it out in time to add extrinsic value to
            the trade, further reducing the cost basis on the position. We can also move the call
            strike down in the same cycle to achieve the same cost basis reduction result, or a
            combination of rolling out in time and down a few strikes. Avoid rolling the call
            below your breakeven on the trade overall to ensure potential profit if the stock
            rallies back.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            We typically hold onto the trade — extrinsic value moves against us are temporary, but
            if this is paired with a stock selloff, we can adjust the short call if we want to.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            We can consider adjusting the short call if it has lost value by rolling it out in time
            or moving it closer to the long strike, but not below our breakeven point.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="vol-grid">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            We will realize max loss on the trade, which is the debit paid up front.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The trade will be at max profit — we close the trade.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Make sure the debit paid is no more than 75% the width of the strikes. If you pay a
            debit higher than the width of the strikes and there is a huge move where the spread
            moves ITM, you can lose money as the strikes lose extrinsic value and start to trade
            with pure intrinsic value.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            The idea here is to buy a long-term low volatility contract and take advantage of
            heightened IV in the front-month by placing our short contract there. This setup can be
            very efficient for products with pending news, or big realized movements that pump up
            the near-term IV.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "long_diagonal_spread"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">LONG DIAGONAL<br>SPREAD</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          A directional debit strategy that uses two different expiration cycles — buy an ITM
          long-term option to replicate a stock position, and sell an OTM near-term option to
          reduce cost basis. Net debit should not exceed the width between strikes.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↗</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Directional</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Low</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45 to 60</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">50% to 60%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">C</div>
          <div class="step-label">Buy an ITM option in a long-term expiration cycle</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell an OTM option in a near-term expiration cycle</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Distance Between Strikes − Debit Paid + Estimated Extrinsic Value in Long Option</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Estimated Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Long Strike + Debit Paid</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 90 call in a long-term expiration and sell the
            105 call in a near-term expiration.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">105</span>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div class="chain-circle">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">90</span>
              <div class="chain-badge green-badge">C</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Buy 90C (long-term), sell 105C (near-term), net debit $10
      Max profit = (105-90)-10 = $500 (above $105); Max loss = $1000 (below $90)
      Breakeven = 90 + 10 = $100
      X range $82–$118, 36 pts, 600px → scale 16.67px/$
      x(90)=133, x(100)=300, x(105)=383
      Y range -1100 to +600 (1700 total), height 160
      y(pnl)=(600-pnl)/1700*160
      y(+500)=9.4, y(0)=56.5, y(-1000)=150.6
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 90 long-term / sell 105 near-term @ $10 debit</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red loss zone (left, below breakeven) -->
        <polygon
          points="0,160 0,150.6 133,150.6 300,56.5 300,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone (right, above breakeven) -->
        <polygon
          points="300,160 300,56.5 383,9.4 600,9.4 600,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="56.5" x2="600" y2="56.5" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: long 90 (green), BE $100 (amber), short 105 (red) -->
        <line x1="133" y1="0" x2="133" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#f59e0b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="383" y1="0" x2="383" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: flat max loss → slope up → flat max profit -->
        <polyline
          points="0,150.6 133,150.6 300,56.5 383,9.4 600,9.4"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="390" y="7" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $500</text>
        <text x="2" y="148" fill="#dc2626" font-size="9" font-family="monospace">Loss $1000</text>
        <text x="136" y="155" fill="#22c55e" font-size="8" font-family="monospace">90 long</text>
        <text x="258" y="53" fill="#f59e0b" font-size="8" font-family="monospace">BE $100</text>
        <text x="386" y="155" fill="#dc2626" font-size="8" font-family="monospace">105 short</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock moves toward the short option strike by the expiration of the near-term
            contract. This results in max extrinsic value collected from the short option, as well
            as max value gained on the long option, plus any remaining extrinsic value in the long
            option at expiration.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock moves against the position. This results in losses on the long option you
            own, although the short option will lose value and partially hedge the long option's
            loss — you would not lose as much as an outright long or short position in a big move.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the short option loses value, roll it out in time to add extrinsic value and further
            reduce cost basis. You can also move the short strike closer to the long strike in the
            same cycle, or combine rolling out in time with adjusting a few strikes. Avoid rolling
            below your breakeven on the overall trade to ensure potential profit if the stock
            reverses.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            We typically hold onto the trade — extrinsic value moves against us are temporary. If
            this is paired with a move against the position, we can adjust the short option if we
            want to.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            We can consider adjusting the short option if it has lost value by rolling it out in
            time or moving it closer to the long strike, but not below our breakeven point.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="vol-grid">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            We will realize max loss on the trade, which is the debit paid up front.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The trade will be at max profit — we close the trade.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Make sure the debit paid is no more than 75% the width of the strikes. If you pay a
            debit higher than the width of the strikes and there is a large move where the spread
            moves ITM, you can lose money as the strikes lose extrinsic value and trade with pure
            intrinsic value.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            The goal is to buy a long-term, low-volatility contract and take advantage of
            heightened IV in the near-term cycle by placing the short option there. This can be
            very efficient for products with pending news or big realized movements that pump up
            near-term IV.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "short_diagonal_spread"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">SHORT DIAGONAL<br>SPREAD</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          A bearish directional debit strategy — buy an ITM long-term put to replicate a short
          stock position, and sell an OTM near-term put to reduce cost basis. Net debit should
          not exceed the width between strikes.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↘</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bearish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Low</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">45 to 60</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">50% to 60%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">P</div>
          <div class="step-label">Buy an ITM put in a long-term expiration cycle</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell an OTM put in a near-term expiration cycle</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Distance Between Strikes − Debit Paid + Estimated Extrinsic Value in Long Option</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Estimated Max Profit</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Long Put Strike − Debit Paid</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 110 put in a long-term expiration and sell the
            95 put in a near-term expiration.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">110</span>
              <div class="chain-badge green-badge">P</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div class="chain-circle">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">95</span>
              <div class="chain-badge red-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Buy 110P (long-term), sell 95P (near-term), net debit $10
      Max profit = (110-95)-10 = $500 (below $95); Max loss = $1000 (above $110)
      Breakeven = 110 - 10 = $100
      X range $82–$118, 36 pts, 600px → scale 16.67px/$
      x(95)=217, x(100)=300, x(110)=467
      Y range -1100 to +600 (1700 total), height 160
      y(pnl)=(600-pnl)/1700*160
      y(+500)=9.4, y(0)=56.5, y(-1000)=150.6
      Payoff slopes down left→right (profit left, loss right — bearish)
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 110P long-term / sell 95P near-term @ $10 debit</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Green profit zone (left, below breakeven) -->
        <polygon
          points="0,160 0,9.4 217,9.4 300,56.5 300,160"
          fill="rgba(34,197,94,0.15)"
        />
        <!-- Red loss zone (right, above breakeven) -->
        <polygon
          points="300,160 300,56.5 467,150.6 600,150.6 600,160"
          fill="rgba(220,38,38,0.18)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="56.5" x2="600" y2="56.5" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: short 95P (red), BE $100 (amber), long 110P (green) -->
        <line x1="217" y1="0" x2="217" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#f59e0b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="467" y1="0" x2="467" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: flat profit → slope down → flat max loss -->
        <polyline
          points="0,9.4 217,9.4 300,56.5 467,150.6 600,150.6"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="2" y="7" fill="#22c55e" font-size="10" font-family="monospace">Max Profit $500</text>
        <text x="450" y="148" fill="#dc2626" font-size="9" font-family="monospace">Loss $1000</text>
        <text x="220" y="155" fill="#dc2626" font-size="8" font-family="monospace">95P short</text>
        <text x="258" y="53" fill="#f59e0b" font-size="8" font-family="monospace">BE $100</text>
        <text x="470" y="155" fill="#22c55e" font-size="8" font-family="monospace">110P long</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock moves down to the short put strike by the expiration of the near-term
            contract. This results in max extrinsic value collected from the short put, as well as
            max value gained on the long put option that acts as a synthetic short stock position,
            plus any remaining extrinsic value in the long option as well.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock goes up. This results in losses on the long put you own, although the short
            put will lose value and hedge the loss on the long put, and you would not lose as much
            as being short 100 shares of stock if there is a big rally.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            If the short put loses value, roll it out in time to add extrinsic value and further
            reduce cost basis. You can also move the put strike up in the same cycle to achieve the
            same cost basis reduction, or combine rolling out in time with adjusting a few strikes.
            Avoid rolling the put above your breakeven on the overall trade to ensure potential
            profit if the stock sells off.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            The long put may appreciate in value to a greater degree than the short put and the
            trade may become profitable, especially if this is paired with a bearish move.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            The long put may decrease in value to a greater degree than the short put, especially
            if this is paired with a bullish move. We typically hold if this is the case, or close
            if our assumption has changed.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="vol-grid">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            We will realize max loss on the trade, which is the debit paid up front.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            The trade will be at max profit — we close the trade.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            Make sure the debit paid is no more than 75% the width of the strikes. If you pay a
            debit higher than the width of the strikes and there is a huge move where the spread
            moves ITM, you can lose money as the strikes lose extrinsic value and start to trade
            with pure intrinsic value.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            The goal is to buy a long-term low volatility contract and take advantage of heightened
            IV in the near-term cycle by placing the short put there. This setup can be very
            efficient for products with pending news or big realized movements that pump up
            near-term IV.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "pzbr"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">PUT<br>ZEBRA</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          A bearish back-ratio spread where we buy two ITM puts and sell one ATM put to remove all
          extrinsic value and achieve 100 negative deltas. Acts like a married call — the most we
          can lose is the debit paid, and we have 100 shares of short stock profit potential.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↘</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bearish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Any</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">Any</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">50%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div style="display:flex;gap:3px;">
            <div class="step-badge green-badge">P</div>
            <div class="step-badge green-badge">P</div>
          </div>
          <div class="step-label">Buy 2 ITM puts</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">P</div>
          <div class="step-label">Sell 1 ATM put (to remove all extrinsic value)</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Unlimited</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">25% of Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Short Put Strike − Any Extrinsic Value Paid</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy two of the 105 puts and sell one 100 strike put for
            a debit with no extrinsic value.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">105</span>
              <div class="chain-badge green-badge">P</div>
              <div class="chain-badge green-badge">P</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">100</span>
              <div class="chain-circle">○</div>
              <div class="chain-badge red-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Short/Dyn</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Buy 2×105P, sell 1×100P, net debit $5 (= no extrinsic)
      Max loss = $500 (above $105); Unlimited profit below $100
      Breakeven = 102.5 (between strikes where 2×(105-p)-5 = 0)
      X range $82–$118, 36 pts, 600px → scale 16.67px/$
      x(100)=300, x(102.5)=342, x(105)=383
      Y range -600 to +1100 (1700 total), height 160
      y(pnl)=(1100-pnl)/1700*160
      y(+1000)=9.4, y(+500)=56.5, y(0)=103.5, y(-500)=150.6
      Payoff: line enters from top-left (~x=200,y=0 for price $94)
      then slopes to breakeven $102.5, then flat loss above $105
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 2×105P / sell 1×100P @ $5 debit (zero extrinsic)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Green profit zone (left, below breakeven) -->
        <polygon
          points="0,160 0,0 200,0 342,103.5 342,160"
          fill="rgba(34,197,94,0.15)"
        />
        <!-- Red loss zone (right, above breakeven) -->
        <polygon
          points="342,160 342,103.5 383,150.6 600,150.6 600,160"
          fill="rgba(220,38,38,0.18)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="103.5" x2="600" y2="103.5" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: short 100P (red), BE $102.50 (amber), long 105P (green) -->
        <line x1="300" y1="0" x2="300" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="342" y1="0" x2="342" y2="160" stroke="#f59e0b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="383" y1="0" x2="383" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: unlimited slope from top-left → breakeven → flat max loss -->
        <polyline
          points="200,0 300,56.5 342,103.5 383,150.6 600,150.6"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Arrow indicating unlimited profit to the left -->
        <text x="4" y="12" fill="#22c55e" font-size="10" font-family="monospace">↑ Unlimited</text>

        <!-- Labels -->
        <text x="450" y="148" fill="#dc2626" font-size="9" font-family="monospace">Max Loss $500</text>
        <text x="303" y="155" fill="#dc2626" font-size="8" font-family="monospace">100P short</text>
        <text x="310" y="101" fill="#f59e0b" font-size="8" font-family="monospace">BE$102.50</text>
        <text x="386" y="155" fill="#22c55e" font-size="8" font-family="monospace">105P×2</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock price moves down. This is a synthetic short stock position that acts like a
            married call — you have 100 short shares of downside profit potential with limited risk
            above your long put strikes. Unlike a married call, the put ZEBRA gives you defined risk
            without having to pay extrinsic value for the protection (if set up with zero extrinsic
            value on entry).
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock price moves up. The synthetic short stock position will lose value at almost
            the same rate as being short 100 shares of stock, but losses will taper off above the
            long put strikes, since the most you can lose is the debit paid for the trade.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            In an effort to reduce the cost of the trade, roll the short put up a few strikes if
            the stock rallies. This will decrease the short delta amount on a selloff, but if that
            never happens, we reduce the overall debit paid on the trade by rolling the put up.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            The trade could be profitable if this is paired with a bearish move in the stock price.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            This trade will likely be unaffected as we start with zero extrinsic value, unless this
            is paired with a bullish move in the stock price, which could result in losses.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            We close the trade for a profit.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            We close the trade and restructure in a later cycle if we want to stay in. We want to
            avoid assignment by closing the trade prior to expiration.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            We will realize max loss on the trade, which is the debit paid up front.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            This can be a great short stock replacement strategy with limited risk if you're
            directionally correct. The risk profile for a put ZEBRA is similar to a married call,
            since our risk is capped at the debit paid for the spread.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            Long-term cycles and high IV products will be more expensive trades on entry. Short-term
            cycles and low IV products will be cheaper on entry. We still need the directional move
            to be profitable.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "czbr"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">CALL<br>ZEBRA</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          A bullish back-ratio spread where we buy two ITM calls and sell one ATM call to remove all
          extrinsic value and achieve 100 positive deltas. Acts like a married put — the most we
          can lose is the debit paid, and we have 100 shares of long stock profit potential.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↗</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Bullish</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">Any</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">Any</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">50%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div style="display:flex;gap:3px;">
            <div class="step-badge green-badge">C</div>
            <div class="step-badge green-badge">C</div>
          </div>
          <div class="step-label">Buy 2 ITM calls</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div class="step-badge red-badge">C</div>
          <div class="step-label">Sell 1 ATM call (to remove all extrinsic value)</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Unlimited</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">25% of Debit Paid</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Short Call Strike + Any Extrinsic Value Paid</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy two of the 95 calls and sell one 100 strike call for
            a debit with no extrinsic value.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">100</span>
              <div class="chain-circle">○</div>
              <div class="chain-badge red-badge">C</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">95</span>
              <div class="chain-badge green-badge">C</div>
              <div class="chain-badge green-badge">C</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long/Dyn</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Flat</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Buy 2×95C, sell 1×100C, net debit $5 (= no extrinsic)
      Max loss = $500 (below $95); Unlimited profit above $100
      Breakeven = 97.5 (where 2*(p-95)-5 = 0)
      X range $82–$118, 36 pts, 600px → scale 16.67px/$
      x(95)=217, x(97.5)=258, x(100)=300
      Y range -600 to +1100 (1700 total), height 160
      y(pnl)=(1100-pnl)/1700*160
      y(+1000)=9.4, y(+500)=56.5, y(0)=103.5, y(-500)=150.6
      At price $106: P&L=(106-95)*100=$1100 → y=0 (exits top of chart at x=400)
      Payoff: flat max loss left → slope up → exits chart top-right (unlimited)
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 2×95C / sell 1×100C @ $5 debit (zero extrinsic)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red loss zone (left, below breakeven) -->
        <polygon
          points="0,160 0,150.6 217,150.6 258,103.5 258,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone (right, above breakeven) -->
        <polygon
          points="258,160 258,103.5 300,56.5 400,0 600,0 600,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="103.5" x2="600" y2="103.5" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: long 95C (green), BE $97.50 (amber), short 100C (red) -->
        <line x1="217" y1="0" x2="217" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="258" y1="0" x2="258" y2="160" stroke="#f59e0b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />

        <!-- Payoff line: flat max loss → slope up → exits chart top-right -->
        <polyline
          points="0,150.6 217,150.6 258,103.5 300,56.5 400,0"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Arrow indicating unlimited profit to the right -->
        <text x="410" y="12" fill="#22c55e" font-size="10" font-family="monospace">↑ Unlimited</text>

        <!-- Labels -->
        <text x="2" y="148" fill="#dc2626" font-size="9" font-family="monospace">Max Loss $500</text>
        <text x="220" y="155" fill="#22c55e" font-size="8" font-family="monospace">95C×2</text>
        <text x="218" y="101" fill="#f59e0b" font-size="8" font-family="monospace">BE$97.50</text>
        <text x="303" y="155" fill="#dc2626" font-size="8" font-family="monospace">100C short</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock price moves up. This is a synthetic long stock position that acts like a
            married put — you have 100 shares of upside profit potential with limited risk below
            your long call strikes. Unlike a married put, the call ZEBRA gives you defined risk
            without having to pay extrinsic value for the protection (if set up with zero extrinsic
            value on entry).
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock price moves down. The synthetic stock position will lose value at almost the
            same rate as owning 100 shares of stock, but losses will taper off below the long call
            strikes, since the most you can lose is the debit paid for the trade.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            In an effort to reduce the cost of the trade, roll the short call down a few strikes
            if the stock sells off. This will reduce the long delta amount on a rally, but if that
            never happens, we reduce the overall debit paid on the trade by rolling the call down.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            This trade will likely be unaffected as we start with zero extrinsic value, unless this
            is paired with a bearish move in the stock price, which could result in losses.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            The trade could be profitable if this is paired with a bullish move in the stock price.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            We will realize max loss on the trade, which is the debit paid up front.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            We close the trade and restructure in a later cycle if we want to stay in. We want to
            avoid assignment by closing the trade prior to expiration.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            We close the trade for a profit.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            This can be a great long stock replacement strategy with limited risk if you're
            directionally correct. The risk profile for a call ZEBRA is similar to a married put,
            since our risk is capped at the debit paid for the spread.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            Long-term cycles and high IV products will be more expensive trades on entry. Short-term
            cycles and low IV products will be cheaper on entry. We still need the directional move
            to be profitable.
          </p>
        </div>
      </div>
    </div>

  {:else if strategy === "ratio_spread" || strategy === "custom"}
    <!-- ─── HEADER ─────────────────────────────────────────────────────────── -->
    <div class="header-block">
      <div class="header-left">
        <div class="red-bar"></div>
        <div class="header-title">RATIO<br>SPREAD</div>
      </div>
      <div class="header-right">
        <p class="header-desc">
          Omnidirectional, undefined risk trade consisting of a long put spread with an extra short
          put to finance the trade and receive a net credit overall. No risk to the upside, but max
          profit at the short strike. Also known as a put ratio spread.
        </p>
        <div class="metrics-row">
          <div class="metric-cell">
            <div class="metric-icon">↔</div>
            <div class="metric-label">DIRECTIONAL ASSUMPTION</div>
            <div class="metric-value">Omnidirectional</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">◈</div>
            <div class="metric-label">IV ENVIRONMENT</div>
            <div class="metric-value">High</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▦</div>
            <div class="metric-label">DAYS TO EXPIRATION</div>
            <div class="metric-value">15 to 45</div>
          </div>
          <div class="metric-cell">
            <div class="metric-icon">▲</div>
            <div class="metric-label">PROBABILITY OF PROFIT</div>
            <div class="metric-value">60% to 80%</div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SETUP ─────────────────────────────────────────────────────────── -->
    <div class="section-title-white">SETUP</div>
    <div class="setup-row">
      <div class="setup-left">
        <!-- Step 1 -->
        <div class="step-row">
          <div class="step-num">1</div>
          <div class="step-badge green-badge">P</div>
          <div class="step-label">Buy an ATM or OTM put</div>
        </div>
        <div class="step-connector"></div>
        <!-- Step 2 -->
        <div class="step-row">
          <div class="step-num">2</div>
          <div style="display:flex;gap:3px;">
            <div class="step-badge red-badge">P</div>
            <div class="step-badge red-badge">P</div>
          </div>
          <div class="step-label">Sell two further OTM puts for a net credit</div>
        </div>

        <!-- Stats box -->
        <div class="stats-box">
          <div class="stats-row">
            <span class="stats-icon red-text">↑</span>
            <div>
              <div class="stats-key red-text">MAX PROFIT</div>
              <div class="stats-val">Width of Long Spread + Credit Received</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">↓</span>
            <div>
              <div class="stats-key red-text">MAX LOSS</div>
              <div class="stats-val">Breakeven Price × 100 (undefined to downside)</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">◎</span>
            <div>
              <div class="stats-key red-text">PROFIT TARGET</div>
              <div class="stats-val">50% of Credit Received or 25% of Long Spread Width</div>
            </div>
          </div>
          <div class="stats-row">
            <span class="stats-icon red-text">⚖</span>
            <div>
              <div class="stats-key red-text">BREAKEVEN</div>
              <div class="stats-val">Short Put Strike − (Long Spread Width + Credit Received)</div>
            </div>
          </div>
        </div>
      </div>

      <div class="setup-right">
        <!-- Example box -->
        <div class="example-box">
          <div class="example-header">EXAMPLE</div>
          <p class="example-text">
            With XYZ stock at $100, we may buy the 95 put and sell two of the 90 puts for a small
            credit.
          </p>
          <div class="chain-diagram">
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">100</span>
              <div class="chain-circle">○</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end;">
              <span class="chain-price">95</span>
              <div class="chain-badge green-badge">P</div>
            </div>
            <div class="chain-spacer"></div>
            <div class="chain-row" style="justify-content:flex-end; gap:4px;">
              <span class="chain-price">90</span>
              <div class="chain-badge red-badge">P</div>
              <div class="chain-badge red-badge">P</div>
            </div>
          </div>
          <div class="greeks-grid">
            <div class="greek-cell">
              <span class="greek-sym amber-text">Δ</span>
              <span class="greek-label">DELTA</span>
              <span class="greek-val">Long/Dyn</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">V</span>
              <span class="greek-label">VEGA</span>
              <span class="greek-val">Short</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Θ</span>
              <span class="greek-label">THETA</span>
              <span class="greek-val">Long</span>
            </div>
            <div class="greek-cell">
              <span class="greek-sym amber-text">Γ</span>
              <span class="greek-label">GAMMA</span>
              <span class="greek-val">Dynamic</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ─── SVG PAYOFF DIAGRAM ────────────────────────────────────────────── -->
    <!--
      Buy 95P, sell 2×90P, net credit ~$0.50
      Max profit = $500 + $50 = $550 at $90
      Small profit = $50 (credit) above $95
      Breakeven = 90 − (5 + 0.50) = $84.50; undefined loss below that
      X range $72–$108, 36 pts, 600px → scale 16.67px/$
      x(p)=(p-72)*16.67; x(84.5)=208, x(90)=300, x(95)=383, x(100)=467
      Y range -700 to +600 (1300 total), height 160
      y(pnl)=(600-pnl)/1300*160
      y(+550)=6.15, y(+50)=67.7, y(0)=73.8, y(-600)=147.7
    -->
    <div class="payoff-wrap">
      <div class="payoff-label gray-text">Payoff at Expiration — buy 95P / sell 2×90P @ $0.50 credit (1×2 ratio)</div>
      <svg viewBox="0 0 600 160" class="payoff-svg" preserveAspectRatio="none">
        <!-- Red loss zone (left of breakeven, undefined) -->
        <polygon
          points="0,160 0,148 208,74 208,160"
          fill="rgba(220,38,38,0.18)"
        />
        <!-- Green profit zone (right of breakeven) -->
        <polygon
          points="208,160 208,74 300,6 383,68 600,68 600,160"
          fill="rgba(34,197,94,0.15)"
        />

        <!-- Zero P&L line -->
        <line x1="0" y1="74" x2="600" y2="74" stroke="#475569" stroke-width="0.8" stroke-dasharray="2,4" />

        <!-- Dashed verticals: BE $84.50 (amber), short 90P (red), long 95P (green), stock $100 (gray) -->
        <line x1="208" y1="0" x2="208" y2="160" stroke="#f59e0b" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="300" y1="0" x2="300" y2="160" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="383" y1="0" x2="383" y2="160" stroke="#22c55e" stroke-width="1" stroke-dasharray="4,3" />
        <line x1="467" y1="0" x2="467" y2="160" stroke="#475569" stroke-width="0.8" stroke-dasharray="3,4" />

        <!-- Payoff line: loss slope → peak → drop to flat credit -->
        <polyline
          points="0,148 208,74 300,6 383,68 600,68"
          fill="none" stroke="#f8fafc" stroke-width="2.5" stroke-linejoin="round"
        />

        <!-- Labels -->
        <text x="270" y="4" fill="#22c55e" font-size="10" font-family="monospace">Max $550</text>
        <text x="2" y="146" fill="#dc2626" font-size="9" font-family="monospace">↓ Unlimited</text>
        <text x="390" y="65" fill="#22c55e" font-size="8" font-family="monospace">Credit $50</text>
        <text x="168" y="71" fill="#f59e0b" font-size="8" font-family="monospace">BE$84.50</text>
        <text x="303" y="155" fill="#dc2626" font-size="8" font-family="monospace">90P×2</text>
        <text x="386" y="155" fill="#22c55e" font-size="8" font-family="monospace">95P long</text>
        <text x="470" y="155" fill="#94a3b8" font-size="8" font-family="monospace">$100</text>
      </svg>
    </div>

    <!-- ─── TASTYLIVE APPROACH ────────────────────────────────────────────── -->
    <div class="dark-section">
      <div class="section-title-amber">TASTYLIVE APPROACH</div>
      <div class="subsection-title-amber">HOW THE TRADE WORKS</div>

      <div class="approach-grid">
        <!-- Ideal -->
        <div class="approach-card">
          <div class="approach-card-title">IDEAL</div>
          <p>
            The stock moves towards the short strikes at expiration. Put ratio spreads are
            omnidirectional — they can be profitable from a rise in stock price or a move down
            towards the short strikes. Max profit occurs when the long put spread is fully ITM at
            the short strike, so ideally the stock moves towards the short put strikes near
            expiration.
          </p>
        </div>

        <!-- Not Ideal -->
        <div class="approach-card not-ideal">
          <div class="approach-card-title red-text">NOT IDEAL</div>
          <p>
            The stock moves well below the short strikes and below our breakeven. This results in
            the stock passing through our profit zone and into our loss zone, because we have a
            short put with undefined risk.
          </p>
        </div>

        <!-- Defensive Tactics -->
        <div class="approach-card defensive">
          <div class="approach-card-title">DEFENSIVE TACTICS</div>
          <p>
            The naked put portion is where the risk lies. Rolling it out in time for a credit adds
            extrinsic value and more time without adding risk. The long put spread would be near max
            value if we're seeing losses overall — it can be closed for an additional credit against
            the remaining short put.
          </p>
        </div>
      </div>

      <!-- ─── VOLATILITY ───────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">VOLATILITY</div>
      <div class="vol-grid">
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY EXPANDS</div>
          <p>
            We may hold — this may result in an extrinsic value loss if paired with a bearish move
            in the stock price. However, extrinsic value will always go to zero by expiration.
          </p>
        </div>
        <div class="vol-card">
          <div class="vol-card-title">IF VOLATILITY CONTRACTS</div>
          <p>
            Our spread could lose value if contraction is paired with a bullish move. We can close
            for a profit, or purchase an OTM put to create a symmetrical butterfly. If we can buy
            the put for less than the credit received, we lock in a small profit and remove buying
            power and initial risk from the trade.
          </p>
        </div>
      </div>

      <!-- ─── EXPIRATION ────────────────────────────────────────────────── -->
      <div class="subsection-title-amber vol-title">EXPIRATION</div>
      <div class="exp-grid-spv">
        <div class="exp-card">
          <div class="exp-card-title">IF ITM AT EXPIRATION</div>
          <p>
            Close the long put spread to secure value, and either roll the remaining short put out
            in time or consider closing the trade overall.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF PARTIALLY ITM AT EXPIRATION</div>
          <p>
            We can likely sell out of the long put spread for some sort of profit. We consider
            closing the entire trade if this is the case.
          </p>
        </div>
        <div class="exp-card">
          <div class="exp-card-title">IF OTM AT EXPIRATION</div>
          <p>
            All strikes expire worthless and we keep the credit received on entry as profit.
          </p>
        </div>
      </div>
    </div>

    <!-- ─── TAKEAWAYS ────────────────────────────────────────────────────── -->
    <div class="takeaways-section">
      <div class="section-title-amber">TAKEAWAYS</div>
      <div class="takeaways-grid">
        <div class="takeaway-card">
          <div class="takeaway-num">1</div>
          <p>
            For us to see profits on the long spread if it goes ITM, we need extrinsic value to be
            close to zero so we can realize the pure intrinsic value of the spread. If we see a
            move ITM too soon prior to expiration, we can see extrinsic value losses even if we're
            in our max profit zone. For this reason, hold ratio spreads closer to expiration.
          </p>
        </div>
        <div class="takeaway-card">
          <div class="takeaway-num">2</div>
          <p>
            For earnings trade ratio spreads, we typically go into the weekly cycle — this is
            because we want the stock to move towards our spread, and we need extrinsic value close
            to zero to see profits on the long spread if we get the desired stock price move in our
            favor.
          </p>
        </div>
      </div>
    </div>

  {:else}
    <!-- ─── FALLBACK: Entry Checklist + Thesis ─────────────────────────────── -->
    <div class="fallback-root">
      <div class="fallback-name">{playbook?.name ?? strategy}</div>

      {#if playbook?.criteria}
        {@const c = playbook.criteria}
        <div class="section-title-white" style="margin-top:12px;">ENTRY CHECKLIST</div>
        <div class="checklist-grid">
          {#if c.minIVR != null || c.maxIVR != null}
            <div class="check-card">
              <div class="check-label">IVR</div>
              <div class="check-val">{fmt(c.minIVR)}–{fmt(c.maxIVR)}</div>
            </div>
          {/if}
          {#if c.minDTE != null || c.maxDTE != null}
            <div class="check-card">
              <div class="check-label">DTE</div>
              <div class="check-val">{fmt(c.minDTE)}–{fmt(c.maxDTE)}</div>
            </div>
          {/if}
          {#if c.minDelta != null || c.maxDelta != null}
            <div class="check-card">
              <div class="check-label">Delta</div>
              <div class="check-val">{fmt(c.minDelta)}–{fmt(c.maxDelta)}</div>
            </div>
          {/if}
          {#if c.minPOP != null}
            <div class="check-card">
              <div class="check-label">Min POP</div>
              <div class="check-val">≥{fmt(c.minPOP, "%")}</div>
            </div>
          {/if}
          {#if c.maxAllocationPct != null}
            <div class="check-card">
              <div class="check-label">Max Alloc</div>
              <div class="check-val">≤{fmt(c.maxAllocationPct, "%")}</div>
            </div>
          {/if}
          {#if c.maxBprPct != null}
            <div class="check-card">
              <div class="check-label">Max BPR</div>
              <div class="check-val">≤{fmt(c.maxBprPct, "%")}</div>
            </div>
          {/if}
          {#if c.targetProfitPct != null}
            <div class="check-card">
              <div class="check-label">Target</div>
              <div class="check-val">{fmt(c.targetProfitPct, "%")}</div>
            </div>
          {/if}
          {#if c.vixMin != null || c.vixMax != null}
            <div class="check-card">
              <div class="check-label">VIX</div>
              <div class="check-val">{fmt(c.vixMin)}–{fmt(c.vixMax)}</div>
            </div>
          {/if}
          {#if c.stopLossPct != null}
            <div class="check-card">
              <div class="check-label">Stop Loss</div>
              <div class="check-val">{fmt(c.stopLossPct, "%")}</div>
            </div>
          {/if}
          {#if c.managementRule}
            <div class="check-card check-card-wide">
              <div class="check-label">Management</div>
              <div class="check-val">{c.managementRule}</div>
            </div>
          {/if}
        </div>
        {#if c.notes}
          <div class="check-notes">{c.notes}</div>
        {/if}
      {/if}

      {#if playbook?.description}
        <div class="section-title-white" style="margin-top:16px;">THESIS</div>
        <div class="fallback-thesis">{playbook.description}</div>
      {:else}
        <div class="unsupported" style="margin-top:16px;">
          <p>No guide or thesis available for <strong>{playbook?.name ?? strategy}</strong>.</p>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  /* ── Root ─────────────────────────────────────────────────────────────── */
  .guide-root {
    background: #0d1117;
    color: #f8fafc;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    font-size: 13px;
    padding: 16px;
    padding-top: 12px;
    min-height: 100%;
    position: relative;
    line-height: 1.5;
  }

  /* ── Close button ─────────────────────────────────────────────────────── */
  .close-btn {
    position: fixed;
    top: 10px;
    right: 14px;
    background: rgba(30,41,59,0.85);
    border: 1px solid #334155;
    color: #f8fafc;
    font-size: 14px;
    width: 28px;
    height: 28px;
    border-radius: 4px;
    cursor: pointer;
    z-index: 99;
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }
  .close-btn:hover { background: #dc2626; }

  /* ── Header ───────────────────────────────────────────────────────────── */
  .header-block {
    display: flex;
    gap: 16px;
    margin-bottom: 16px;
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    overflow: hidden;
  }
  .header-left {
    display: flex;
    align-items: center;
    gap: 0;
    min-width: 0;
  }
  .red-bar {
    width: 6px;
    background: #dc2626;
    align-self: stretch;
    flex-shrink: 0;
  }
  .header-title {
    font-size: 22px;
    font-weight: 900;
    color: #f8fafc;
    padding: 16px 12px;
    white-space: nowrap;
    letter-spacing: 0.05em;
  }
  .header-right {
    flex: 1;
    padding: 10px 12px;
    border-left: 1px solid #30363d;
  }
  .header-desc {
    color: #94a3b8;
    margin-bottom: 10px;
    font-size: 12px;
  }
  .metrics-row {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .metric-cell {
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 6px 8px;
  }
  .metric-icon { color: #dc2626; font-size: 11px; }
  .metric-label { color: #dc2626; font-size: 9px; font-weight: 700; letter-spacing: 0.05em; margin: 2px 0 1px; }
  .metric-value { color: #f8fafc; font-size: 12px; font-weight: 600; }

  /* ── Section titles ───────────────────────────────────────────────────── */
  .section-title-white {
    font-size: 16px;
    font-weight: 800;
    color: #f8fafc;
    margin: 12px 0 8px;
    letter-spacing: 0.05em;
  }
  .section-title-amber {
    font-size: 15px;
    font-weight: 800;
    color: #f59e0b;
    margin: 12px 0 6px;
    letter-spacing: 0.05em;
  }
  .subsection-title-amber {
    font-size: 13px;
    font-weight: 700;
    color: #f59e0b;
    margin: 10px 0 6px;
    letter-spacing: 0.04em;
  }
  .vol-title { margin-top: 16px; }

  /* ── Setup ────────────────────────────────────────────────────────────── */
  .setup-row {
    display: flex;
    gap: 16px;
    margin-bottom: 12px;
  }
  .setup-left { flex: 1.2; }
  .setup-right { flex: 1; }

  .step-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 6px 0;
  }
  .step-num {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: #1e293b;
    border: 2px solid #64748b;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 12px;
    flex-shrink: 0;
  }
  .step-connector {
    width: 2px;
    height: 16px;
    background: #334155;
    margin-left: 11px;
  }
  .step-label { color: #f8fafc; font-size: 12px; }

  .green-badge {
    background: #15803d;
    color: #f0fdf4;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 900;
    font-size: 11px;
    flex-shrink: 0;
  }
  .red-badge {
    background: #dc2626;
    color: #fff;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 900;
    font-size: 11px;
    flex-shrink: 0;
  }

  /* ── Stats box ────────────────────────────────────────────────────────── */
  .stats-box {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 10px 12px;
    margin-top: 10px;
  }
  .stats-row {
    display: flex;
    gap: 8px;
    margin: 5px 0;
    align-items: flex-start;
  }
  .stats-icon { font-size: 13px; flex-shrink: 0; margin-top: 1px; }
  .stats-key { font-size: 9px; font-weight: 700; letter-spacing: 0.05em; }
  .stats-val { font-size: 11px; color: #94a3b8; margin-top: 1px; }

  /* ── Example box ──────────────────────────────────────────────────────── */
  .example-box {
    border: 1px solid #dc2626;
    border-radius: 6px;
    background: #161b22;
    padding: 10px 12px;
    height: 100%;
  }
  .example-header {
    font-size: 10px;
    font-weight: 700;
    color: #dc2626;
    letter-spacing: 0.08em;
    margin-bottom: 6px;
  }
  .example-text { color: #94a3b8; font-size: 11px; margin-bottom: 10px; }

  .chain-diagram {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
    margin: 8px 0;
    padding: 6px;
    background: #0d1117;
    border-radius: 4px;
  }
  .chain-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .chain-price { font-family: monospace; font-size: 13px; font-weight: 700; color: #f8fafc; }
  .chain-spacer { height: 12px; width: 2px; background: #334155; margin: 0 auto; }

  .greeks-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 6px;
    margin-top: 10px;
  }
  .greek-cell {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
  }
  .greek-sym { font-size: 14px; font-weight: 700; }
  .greek-label { font-size: 9px; color: #dc2626; font-weight: 700; letter-spacing: 0.05em; }
  .greek-val { font-size: 11px; color: #f8fafc; }

  /* ── Payoff SVG ───────────────────────────────────────────────────────── */
  .payoff-wrap {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 8px 12px;
    margin-bottom: 14px;
  }
  .payoff-label { font-size: 10px; margin-bottom: 6px; }
  .payoff-svg {
    width: 100%;
    height: 160px;
    display: block;
  }

  /* ── Dark section (approach, vol, exp) ───────────────────────────────── */
  .dark-section {
    background: #0a0f16;
    border: 1px solid #1e293b;
    border-radius: 8px;
    padding: 14px 16px;
    margin-bottom: 14px;
  }

  /* ── Approach grid ────────────────────────────────────────────────────── */
  .approach-grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 10px;
    margin-bottom: 14px;
  }
  .approach-card {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 10px 12px;
  }
  .approach-card.not-ideal { border-color: #7f1d1d; }
  .approach-card.defensive { border-color: #1e3a5f; }
  .approach-card-title {
    font-size: 11px;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: 0.06em;
    margin-bottom: 6px;
  }
  .approach-card p { color: #94a3b8; font-size: 11px; }

  /* ── Volatility grid ──────────────────────────────────────────────────── */
  .vol-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin-bottom: 14px;
  }
  .vol-card {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 10px 12px;
  }
  .vol-card-title {
    font-size: 11px;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: 0.04em;
    margin-bottom: 6px;
  }
  .vol-card p { color: #94a3b8; font-size: 11px; }

  /* ── Expiration grid ──────────────────────────────────────────────────── */
  .exp-grid-spv {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 10px;
  }
  .exp-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .exp-card {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 10px 12px;
  }
  .exp-card-title {
    font-size: 11px;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: 0.04em;
    margin-bottom: 6px;
  }
  .exp-card p { color: #94a3b8; font-size: 11px; }

  /* ── Takeaways ────────────────────────────────────────────────────────── */
  .takeaways-section {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 8px;
    padding: 14px 16px;
    margin-bottom: 8px;
  }
  .takeaways-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-top: 8px;
  }
  .takeaway-card {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .takeaway-num {
    font-size: 36px;
    font-weight: 900;
    color: #dc2626;
    line-height: 1;
    flex-shrink: 0;
    margin-top: -4px;
  }
  .takeaway-card p { color: #94a3b8; font-size: 12px; }

  /* ── Calendar time labels ─────────────────────────────────────────────── */
  .cal-time-labels {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 4px 0 8px;
  }
  .cal-time-label { font-size: 10px; }

  /* ── Utilities ────────────────────────────────────────────────────────── */
  .red-text   { color: #dc2626; }
  .amber-text { color: #f59e0b; }
  .gray-text  { color: #94a3b8; }

  .unsupported {
    padding: 40px;
    color: #94a3b8;
    text-align: center;
  }

  /* ── Fallback: checklist + thesis ────────────────────────────────────── */
  .fallback-root {
    padding: 8px 0;
  }
  .fallback-name {
    font-size: 22px;
    font-weight: 800;
    color: #f8fafc;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    line-height: 1.2;
    padding-bottom: 4px;
    border-bottom: 2px solid #dc2626;
    margin-bottom: 2px;
  }
  .checklist-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  .check-card {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 6px 10px;
    min-width: 80px;
  }
  .check-card-wide {
    flex: 1 1 100%;
  }
  .check-label {
    font-size: 9px;
    font-weight: 700;
    color: #dc2626;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin-bottom: 2px;
  }
  .check-val {
    font-size: 13px;
    font-weight: 600;
    color: #f8fafc;
  }
  .check-notes {
    margin-top: 8px;
    font-size: 11px;
    color: #94a3b8;
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 8px 10px;
    white-space: pre-wrap;
  }
  .fallback-thesis {
    margin-top: 8px;
    font-size: 12px;
    color: #cbd5e1;
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 12px;
    white-space: pre-wrap;
    line-height: 1.6;
    max-height: calc(100vh - 280px);
    overflow-y: auto;
  }
</style>
