<script>
  import { onMount } from 'svelte'
  import { api } from '../api.js'

  let code = ''
  let data = null
  let error = ''
  let message = ''
  let boxes = []
  let slots = []
  let to_box_code = ''
  let to_slot_code = ''
  let operator = ''

  onMount(async () => {
    const [b, s] = await Promise.all([api.get('/boxes'), api.get('/slots')])
    boxes = b.boxes
    slots = s.slots
  })

  async function lookup() {
    error = ''
    message = ''
    data = null
    try {
      data = await api.get(`/pickup/${code.trim()}`)
      const curBox = boxes.find((b) => b.code === data.order.box_code)
      const free = boxes.find((b) => !b.occupied)
      to_box_code = (free ?? curBox)?.code ?? ''
      to_slot_code = data.order.slot_code
    } catch (e) {
      error = e.message
    }
  }

  async function submit() {
    error = ''
    message = ''
    try {
      const r = await api.post(`/orders/${data.order.id}/transfer`, {
        to_box_code, to_slot_code, operator,
      })
      message = `${r.message}：${r.order.transfers.at(-1).from_box}/${r.order.transfers.at(-1).from_slot} → ${r.order.box_code}/${r.order.slot_code}`
      await lookup()
      const b = await api.get('/boxes')
      boxes = b.boxes
    } catch (e) {
      error = e.message
    }
  }
</script>

<div class="card">
  <h3>转箱转位</h3>
  <label>取货码</label>
  <div class="row" style="align-items: center;">
    <div><input bind:value={code} placeholder="6 位取货码" on:keydown={(e) => e.key === 'Enter' && lookup()} /></div>
    <div style="flex: 0 0 auto;"><button class="ghost" style="margin: 0;" on:click={lookup}>查询</button></div>
  </div>
  {#if error}<div class="error">⚠️ {error}</div>{/if}
  {#if message}<div class="success">✅ {message}</div>{/if}
</div>

{#if data}
  {@const o = data.order}
  <div class="card">
    <p>
      订单 <strong>{o.order_no}</strong>（{o.fish_type}，{o.sale_weight.toFixed(2)} kg）
      当前：<strong>鱼箱 {o.box_code} ｜ 箱位 {o.slot_code}</strong>
      <span class="tag {o.status}">{o.status === 'stored' ? '暂存中' : '已交付'}</span>
    </p>
    {#if o.status === 'stored'}
      <div class="row">
        <div>
          <label>新鱼箱（已被其他订单占用的不可选）</label>
          <select bind:value={to_box_code}>
            {#each boxes as b}
              <option value={b.code} disabled={b.occupied && b.code !== o.box_code}>
                {b.code}{b.occupied && b.code !== o.box_code ? '（占用中）' : ''}
              </option>
            {/each}
          </select>
        </div>
        <div>
          <label>新箱位</label>
          <select bind:value={to_slot_code}>
            {#each slots as s}<option value={s.code}>{s.code}</option>{/each}
          </select>
        </div>
        <div>
          <label>经手人</label>
          <input bind:value={operator} placeholder="老王" />
        </div>
      </div>
      <button class="primary" on:click={submit} disabled={!operator}>确认转箱</button>
    {:else}
      <p class="error">订单已交付，不能转箱。</p>
    {/if}

    {#if o.transfers.length}
      <h4>转箱历史</h4>
      <table>
        <thead><tr><th>时间</th><th>从</th><th>到</th><th>经手人</th></tr></thead>
        <tbody>
          {#each o.transfers as t}
            <tr>
              <td>{t.created_at}</td>
              <td>{t.from_box} / {t.from_slot}</td>
              <td>{t.to_box} / {t.to_slot}</td>
              <td>{t.operator}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
{/if}
