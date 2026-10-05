<script>
  import { api } from '../api.js'
  import OrderCard from './OrderCard.svelte'

  let code = ''
  let detail = null
  let error = ''
  let busy = false
  let duelResults = null

  async function lookup() {
    error = ''
    detail = null
    duelResults = null
    if (!code.trim()) return
    try {
      detail = await api.lookup(code.trim())
    } catch (e) {
      error = e.message
    }
  }

  async function deliver() {
    busy = true
    error = ''
    try {
      detail = await api.deliver(code.trim())
    } catch (e) {
      error = e.message
      try { detail = await api.lookup(code.trim()) } catch (_) {}
    } finally {
      busy = false
    }
  }

  // 模拟两个窗口同时交付同一取货码：一个成功，另一个收到"已处理"
  async function deliverDuel() {
    duelResults = null
    error = ''
    const settled = await Promise.allSettled([api.deliver(code.trim()), api.deliver(code.trim())])
    duelResults = settled.map((r, i) => ({
      window: `窗口 ${i + 1}`,
      ok: r.status === 'fulfilled',
      msg: r.status === 'fulfilled' ? '交付成功' : r.reason.message
    }))
    try { detail = await api.lookup(code.trim()) } catch (_) {}
  }
</script>

<div class="card">
  <h3>取货柜台</h3>
  <div style="display:flex; gap:8px; max-width:480px">
    <input
      class="mono"
      bind:value={code}
      placeholder="扫描或输入取货码"
      on:keydown={(e) => e.key === 'Enter' && lookup()}
      style="text-transform:uppercase"
    />
    <button class="btn" on:click={lookup}>查询</button>
  </div>
  {#if error}<div class="error">{error}</div>{/if}
</div>

{#if detail}
  <OrderCard {detail} />

  {#if detail.pickup_code?.status === 'active'}
    <div class="card">
      <h4>核对无误后交付（交付后取货码立即失效）</h4>
      <div class="actions">
        <button class="btn" on:click={deliver} disabled={busy}>
          {busy ? '交付中…' : '✔ 确认交付'}
        </button>
        <button class="btn secondary" on:click={deliverDuel}>⚡ 双窗口并发交付演示</button>
      </div>
      {#if duelResults}
        <div class="hint">两个窗口同时点击交付的结果：</div>
        {#each duelResults as r}
          <div class={r.ok ? 'success' : 'error'}>{r.window}：{r.msg}</div>
        {/each}
      {/if}
    </div>
  {:else}
    <div class="card">
      <div class="error" style="margin:0">该取货码已失效（订单已交付），不能重复交付。</div>
    </div>
  {/if}
{/if}
