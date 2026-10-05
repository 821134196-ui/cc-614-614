<script>
  import { api, kg } from '../api.js'
  import OrderCard from './OrderCard.svelte'

  const DEVICES = ['SCALE-01', 'SCALE-02', 'SCALE-03']

  let code = ''
  let detail = null
  let error = ''
  let okMsg = ''
  let busy = false
  let scaleBusy = false
  let form = { scale_device_id: 'SCALE-02', weight_grams: null, note: '顾客异议复称' }

  async function lookup() {
    error = ''
    okMsg = ''
    detail = null
    if (!code.trim()) return
    try {
      detail = await api.lookup(code.trim())
    } catch (e) {
      error = e.message
    }
  }

  async function readScale() {
    scaleBusy = true
    error = ''
    try {
      const r = await api.readScale(form.scale_device_id)
      form.weight_grams = r.weight_grams
    } catch (e) {
      error = e.message
    } finally {
      scaleBusy = false
    }
  }

  async function submit() {
    error = ''
    okMsg = ''
    busy = true
    try {
      detail = await api.reweigh(detail.order.id, {
        scale_device_id: form.scale_device_id,
        weight_grams: form.weight_grams,
        note: form.note
      })
      okMsg = '复称已记录：原始值与复核值均已保留，销售重量不变。'
      form.weight_grams = null
    } catch (e) {
      error = e.message
    } finally {
      busy = false
    }
  }

  $: original = detail?.weighings?.find((w) => w.kind === 'original')
</script>

<div class="card">
  <h3>复称（重量异议处理）</h3>
  <p class="hint">顾客对取货重量有异议时先复称。复核读数会追加到称重流水，原始值与复核值都保留，不直接修改销售重量。</p>
  <div style="display:flex; gap:8px; max-width:480px">
    <input class="mono" bind:value={code} placeholder="取货码" on:keydown={(e) => e.key === 'Enter' && lookup()} style="text-transform:uppercase" />
    <button class="btn" on:click={lookup}>查询</button>
  </div>
  {#if error}<div class="error">{error}</div>{/if}
</div>

{#if detail}
  {#if original}
    <div class="card">
      <div class="grid2">
        <div><label>原始称重（{original.scale_device_id}）</label><b>{kg(original.weight_grams)}</b></div>
        <div><label>销售重量（不变）</label><b>{kg(detail.order.sale_weight_grams)}</b></div>
      </div>
    </div>
  {/if}

  {#if detail.order.status === 'stored'}
    <div class="card">
      <h4>复核称重</h4>
      <div class="row">
        <div class="field">
          <label>电子秤设备号</label>
          <select bind:value={form.scale_device_id}>
            {#each DEVICES as d}<option value={d}>{d}</option>{/each}
          </select>
        </div>
        <div class="field">
          <label>复核重量（克）</label>
          <div style="display:flex; gap:8px">
            <input type="number" min="1" bind:value={form.weight_grams} placeholder="点击右侧按钮读数" />
            <button class="btn secondary" on:click={readScale} disabled={scaleBusy}>
              {scaleBusy ? '读数中…' : '⚖ 读取电子秤'}
            </button>
          </div>
        </div>
        <div class="field"><label>备注</label><input bind:value={form.note} /></div>
      </div>
      <div class="actions">
        <button class="btn" on:click={submit} disabled={busy || !form.weight_grams}>
          {busy ? '提交中…' : '记录复核结果'}
        </button>
      </div>
      {#if okMsg}<div class="success">{okMsg}</div>{/if}
    </div>
  {:else}
    <div class="card"><div class="error" style="margin:0">订单已交付，不能复称。</div></div>
  {/if}

  <OrderCard {detail} />
{/if}
