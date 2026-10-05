<script>
  import WeighIn from './lib/WeighIn.svelte'
  import Pickup from './lib/Pickup.svelte'
  import Transfer from './lib/Transfer.svelte'
  import Reweigh from './lib/Reweigh.svelte'
  import Orders from './lib/Orders.svelte'

  const tabs = [
    { id: 'weigh', label: '称重入库', comp: WeighIn },
    { id: 'pickup', label: '取货柜台', comp: Pickup },
    { id: 'transfer', label: '转箱转位', comp: Transfer },
    { id: 'reweigh', label: '复称争议', comp: Reweigh },
    { id: 'orders', label: '订单列表', comp: Orders },
  ]
  let active = 'weigh'
  $: current = tabs.find((t) => t.id === active)
</script>

<header>
  <h1>🐟 海鲜市场 · 活鱼称重暂存与取货系统</h1>
  <nav>
    {#each tabs as t}
      <button class:active={active === t.id} on:click={() => (active = t.id)}>
        {t.label}
      </button>
    {/each}
  </nav>
</header>

<main>
  <svelte:component this={current.comp} />
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: 'PingFang SC', 'Microsoft YaHei', system-ui, sans-serif;
    background: #f0f6fa;
    color: #1c2b33;
  }
  header {
    background: linear-gradient(135deg, #0b5e8f, #12876f);
    color: #fff;
    padding: 14px 24px 0;
  }
  h1 { font-size: 20px; margin: 0 0 12px; }
  nav { display: flex; gap: 4px; flex-wrap: wrap; }
  nav button {
    border: none;
    background: rgba(255, 255, 255, 0.15);
    color: #fff;
    padding: 9px 18px;
    border-radius: 8px 8px 0 0;
    cursor: pointer;
    font-size: 15px;
  }
  nav button.active { background: #f0f6fa; color: #0b5e8f; font-weight: 600; }
  main { padding: 20px 24px; max-width: 960px; margin: 0 auto; }

  :global(.card) {
    background: #fff;
    border-radius: 10px;
    padding: 18px 20px;
    margin-bottom: 16px;
    box-shadow: 0 1px 4px rgba(11, 94, 143, 0.12);
  }
  :global(label) { display: block; font-size: 13px; color: #5a6b75; margin: 10px 0 4px; }
  :global(input), :global(select) {
    width: 100%;
    box-sizing: border-box;
    padding: 8px 10px;
    border: 1px solid #c8d6de;
    border-radius: 6px;
    font-size: 14px;
  }
  :global(button.primary) {
    background: #0b5e8f;
    color: #fff;
    border: none;
    padding: 10px 22px;
    border-radius: 6px;
    font-size: 15px;
    cursor: pointer;
    margin-top: 14px;
  }
  :global(button.primary:disabled) { background: #9db8c6; cursor: not-allowed; }
  :global(button.ghost) {
    background: #fff;
    color: #0b5e8f;
    border: 1px solid #0b5e8f;
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    margin-top: 14px;
    margin-left: 8px;
  }
  :global(.error) {
    background: #fdecea;
    color: #b3261e;
    border: 1px solid #f5c6c0;
    padding: 10px 14px;
    border-radius: 6px;
    margin-top: 12px;
  }
  :global(.success) {
    background: #e6f4ea;
    color: #137333;
    border: 1px solid #b7e0c3;
    padding: 10px 14px;
    border-radius: 6px;
    margin-top: 12px;
  }
  :global(table) { width: 100%; border-collapse: collapse; font-size: 14px; }
  :global(th), :global(td) { text-align: left; padding: 8px 10px; border-bottom: 1px solid #e3ecf1; }
  :global(th) { color: #5a6b75; font-weight: 600; font-size: 13px; }
  :global(.tag) {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 999px;
    font-size: 12px;
  }
  :global(.tag.stored) { background: #e3f2fd; color: #0b5e8f; }
  :global(.tag.delivered) { background: #e6f4ea; color: #137333; }
  :global(.tag.used) { background: #eceff1; color: #607d8b; }
  :global(.big-code) {
    font-size: 40px;
    font-weight: 700;
    letter-spacing: 8px;
    color: #0b5e8f;
    text-align: center;
    margin: 10px 0;
  }
  :global(.row) { display: flex; gap: 16px; }
  :global(.row > div) { flex: 1; }
</style>
