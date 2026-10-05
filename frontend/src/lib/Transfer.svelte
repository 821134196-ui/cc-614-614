<script>
  import { api } from '../api.js'
  import OrderCard from './OrderCard.svelte'

  let code = ''
  let detail = null
  let error = ''
  let okMsg = ''
  let busy = false
  let idleBoxes = []
  let form = { to_box_no: '', to_location: '', operator: '' }

  async function lookup() {
    error = ''
    okMsg = ''
    detail = null
    if (!code.trim()) return
    try {
      detail = await api.lookup(code.trim())
      if (detail.order.status === 'stored') {
        idleBoxes = (await api.listBoxes('idle')).filter((b) => b.box_no !== detail.current_box?.box_no)
      }
    } catch (e) {
      error = e.message
    }
  }

  async function submit() {
    error = ''
    okMsg = ''
    busy = true
    try {
      detail = await api.transfer(detail.order.id, {
        to_box_no: form.to_box_no,
        to_location: form.to_location,
        operator: form.operator
      })
      okMsg = `转箱成功：已移至 ${detail.current_box.box_no}（${detail.current_box.location}），取货码不变。`
      form = { to_box_no: '', to_location: '', operator: '' }
      idleBoxes = (await api.listBoxes('idle')).filter((b) => b.box_no !== detail.current_box?.box_no)
    } catch (e) {
      error = e.message
    } finally {
      busy = false
    }
  }
</script>

<div class="card">
  <h3>转箱</h3>
  <p class="hint">输入取货码定位订单，把活鱼换到另一个编号鱼箱。系统记录新旧鱼箱、箱位与经手人，取货码不变。</p>
  <div style="display:flex; gap:8px; max-width:480px">
    <input class="mono" bind:value={code} placeholder="取货码" on:keydown={(e) => e.key === 'Enter' && lookup()} style="text-transform:uppercase" />
    <button class="btn" on:click={lookup}>查询</button>
  </div>
  {#if error}<div class="error">{error}</div>{/if}
</div>

{#if detail}
  <OrderCard {detail} />

  {#if detail.order.status === 'stored'}
    <div class="card">
      <h4>转入新箱</h4>
      <div class="row">
        <div class="field">
          <label>新鱼箱 *（仅显示空闲箱）</label>
          <select bind:value={form.to_box_no}>
            <option value="">请选择</option>
            {#each idleBoxes as b}<option value={b.box_no}>{b.box_no}</option>{/each}
          </select>
        </div>
        <div class="field"><label>新暂存箱位 *</label><input bind:value={form.to_location} placeholder="如 B区-7" /></div>
        <div class="field"><label>经手人 *</label><input bind:value={form.operator} placeholder="老王" /></div>
      </div>
      <div class="actions">
        <button class="btn" on:click={submit} disabled={busy || !form.to_box_no || !form.to_location || !form.operator}>
          {busy ? '提交中…' : '确认转箱'}
        </button>
      </div>
      {#if okMsg}<div class="success">{okMsg}</div>{/if}
    </div>
  {:else}
    <div class="card"><div class="error" style="margin:0">订单已交付，不能转箱。</div></div>
  {/if}
{/if}
