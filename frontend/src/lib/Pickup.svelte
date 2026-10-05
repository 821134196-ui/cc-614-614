<script>
  import { api } from '../api.js'

  let code = ''
  let data = null
  let error = ''
  let message = ''
  let busy = false

  async function lookup() {
    error = ''
    message = ''
    data = null
    try {
      data = await api.get(`/pickup/${code.trim()}`)
    } catch (e) {
      error = e.message
    }
  }

  async function deliver() {
    busy = true
    error = ''
    message = ''
    try {
      const r = await api.post(`/pickup/${code.trim()}/deliver`)
      message = r.message
      await lookup()
    } catch (e) {
      error = e.message
      await lookup()
    } finally {
      busy = false
    }
  }
</script>

<div class="card">
  <h3>取货柜台</h3>
  <label>取货码（扫码或手工输入）</label>
  <div class="row" style="align-items: center;">
    <div><input bind:value={code} placeholder="6 位取货码" on:keydown={(e) => e.key === 'Enter' && lookup()} /></div>
    <div style="flex: 0 0 auto;">
      <button class="ghost" style="margin: 0;" on:click={lookup}>查询</button>
    </div>
  </div>
  {#if error}<div class="error">⚠️ {error}</div>{/if}
  {#if message}<div class="success">✅ {message}</div>{/if}
</div>

{#if data}
  {@const o = data.order}
  <div class="card">
    <h3>
      订单 {o.order_no}
      <span class="tag {o.status}">{o.status === 'stored' ? '暂存中' : '已交付'}</span>
      <span class="tag {data.code_status === 'active' ? 'stored' : 'used'}">
        取货码{data.code_status === 'active' ? '有效' : '已失效'}
      </span>
    </h3>
    <table>
      <tbody>
        <tr><th>顾客</th><td>{o.customer_name} {o.customer_phone}</td></tr>
        <tr><th>品种</th><td>{o.fish_type}</td></tr>
        <tr>
          <th>当前位置</th>
          <td><strong style="font-size: 18px; color: #0b5e8f;">鱼箱 {o.box_code} ｜ 箱位 {o.slot_code}</strong></td>
        </tr>
        <tr><th>销售重量</th><td>{o.sale_weight.toFixed(2)} kg（{o.device_id} 原始读数）</td></tr>
        <tr>
          <th>称重记录</th>
          <td>
            {#each o.weigh_records as r}
              <div>{r.kind === 'original' ? '原始' : '复核'}：{r.weight.toFixed(2)} kg（{r.device_id}，{r.created_at}）</div>
            {/each}
          </td>
        </tr>
      </tbody>
    </table>
    {#if data.code_status === 'active' && o.status === 'stored'}
      <button class="primary" on:click={deliver} disabled={busy}>
        {busy ? '交付中…' : '核对无误，确认交付'}
      </button>
    {/if}
  </div>
{/if}
