<script>
  import Intake from './lib/Intake.svelte'
  import Pickup from './lib/Pickup.svelte'
  import Transfer from './lib/Transfer.svelte'
  import Reweigh from './lib/Reweigh.svelte'
  import Dashboard from './lib/Dashboard.svelte'

  const tabs = [
    { id: 'intake', label: '称重入库', comp: Intake },
    { id: 'pickup', label: '取货柜台', comp: Pickup },
    { id: 'transfer', label: '转箱', comp: Transfer },
    { id: 'reweigh', label: '复称', comp: Reweigh },
    { id: 'dashboard', label: '订单总览', comp: Dashboard }
  ]
  let active = 'intake'
  $: current = tabs.find((t) => t.id === active)
</script>

<header>
  <div class="wrap">
    <h1>🐟 海鲜市场 · 活鱼暂存取货系统</h1>
    <p>称重入库 → 凭码取货 → 转箱 / 复称全程留痕</p>
  </div>
</header>

<nav>
  <div class="wrap">
    {#each tabs as t}
      <button class:active={active === t.id} on:click={() => (active = t.id)}>{t.label}</button>
    {/each}
  </div>
</nav>

<main class="wrap">
  <svelte:component this={current.comp} />
</main>

<style>
  header {
    background: linear-gradient(135deg, #0f6fde, #0a4fa8);
    color: #fff;
    padding: 22px 0 18px;
  }
  header h1 { font-size: 22px; margin: 0 0 4px; }
  header p { margin: 0; opacity: 0.85; font-size: 13px; }
  .wrap { max-width: 1024px; margin: 0 auto; padding: 0 16px; }
  nav { background: #fff; border-bottom: 1px solid var(--border); }
  nav .wrap { display: flex; gap: 4px; overflow-x: auto; }
  nav button {
    background: none; border: none; padding: 13px 18px; font-size: 15px;
    color: var(--muted); border-bottom: 3px solid transparent; white-space: nowrap;
  }
  nav button.active { color: var(--blue); border-bottom-color: var(--blue); font-weight: 600; }
  main { padding-top: 20px; padding-bottom: 48px; }
</style>
