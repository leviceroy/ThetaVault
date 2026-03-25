<script lang="ts">
  import Terminal from "./lib/Terminal.svelte";
  import PayoffChart from "./lib/PayoffChart.svelte";
  import PerformanceCharts from "./lib/PerformanceCharts.svelte";
  import StrategyGuidePanel from "./lib/StrategyGuidePanel.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  let chartTrade: any = null;
  let showChart = false;

  let perfData: any = null;
  let showPerf = false;

  let guideData: any = null;
  let showGuide = false;

  onMount(() => {
    const unlisten1 = listen<string>("chart-show", (event) => {
      console.log("[TV] chart-show received, payload length:", (event.payload as string)?.length);
      try {
        chartTrade = JSON.parse(event.payload);
        showChart = true;
        showPerf = false;
        showGuide = false;
        console.log("[TV] chart trade set:", chartTrade?.ticker, "showChart:", showChart);
      } catch (e) {
        console.error("[TV] Failed to parse chart trade data", e, event.payload?.toString().slice(0, 200));
      }
    });

    const unlisten2 = listen("chart-hide", () => {
      showChart = false;
      chartTrade = null;
    });

    const unlisten3 = listen<string>("perf-show", (event) => {
      console.log("[TV] perf-show received, payload length:", (event.payload as string)?.length);
      try {
        perfData = JSON.parse(event.payload);
        showPerf = true;
        showChart = false;
        showGuide = false;
      } catch (e) {
        console.error("[TV] Failed to parse perf data", e, event.payload?.toString().slice(0, 200));
      }
    });

    const unlisten4 = listen("perf-hide", () => {
      showPerf = false;
      perfData = null;
    });

    const unlisten5 = listen<string>("guide-show", (event) => {
      const raw = event.payload as string;
      try {
        // New format: JSON payload with { strategy, name, description, criteria }
        guideData = JSON.parse(raw);
      } catch {
        // Legacy: plain strategy string
        guideData = { strategy: raw, name: "", description: "", criteria: null };
      }
      showGuide = true;
      showChart = false;
      showPerf = false;
    });

    const unlisten6 = listen("guide-hide", () => {
      showGuide = false;
      guideData = null;
    });

    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
      unlisten3.then((f) => f());
      unlisten4.then((f) => f());
      unlisten5.then((f) => f());
      unlisten6.then((f) => f());
    };
  });

  function closeChart() {
    showChart = false;
    chartTrade = null;
  }

  function closePerf() {
    showPerf = false;
    perfData = null;
  }

  function closeGuide() {
    showGuide = false;
    guideData = null;
  }
</script>

<main class:split={showChart && !!chartTrade || showPerf && !!perfData || showGuide && !!guideData}>
  <div class="terminal-pane">
    <Terminal />
  </div>
  {#if showChart && chartTrade}
    <div class="chart-pane">
      {#key chartTrade.id}
        <PayoffChart trade={chartTrade} on:close={closeChart} />
      {/key}
    </div>
  {:else if showPerf && perfData}
    <div class="chart-pane">
      <PerformanceCharts data={perfData} on:close={closePerf} />
    </div>
  {:else if showGuide && guideData}
    <div class="chart-pane">
      {#key guideData.strategy + (guideData.name ?? "")}
        <StrategyGuidePanel strategy={guideData.strategy} playbook={guideData} on:close={closeGuide} />
      {/key}
    </div>
  {/if}
</main>

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    background: #0d1117;
    overflow: hidden;
  }

  main {
    width: 100vw;
    height: 100vh;
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: 1fr;
  }

  main.split {
    grid-template-columns: 1fr 1fr;
  }

  .terminal-pane {
    min-width: 0;
    height: 100%;
    overflow: hidden;
  }

  .chart-pane {
    height: 100%;
    border-left: 1px solid #30363d;
    background: #0d1117;
    overflow-y: auto;
  }
</style>
