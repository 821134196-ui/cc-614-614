<script>
  import { api } from '../api.js'

  let code = ''
  let data = null
  let error = ''
  let message = ''
  const devices = ['SCALE-01', 'SCALE-02', 'SCALE-03']
  let device_id = 'SCALE-02'
  let weight = null

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

  async function readScale() {
    error = ''
    try {
      const r = await api.get(`/mock/scale/${device_id}/read`)
      weight = r.weight
    } catch (e) {
      error = e.message
    }
  }

  async function submit() {
    error = ''
    message = ''
    try {
      const r = await api.post(`/orders/${data.order.id}/reweigh`, {
        device_id,
        weight: weight ?? undefined,
      })
      message = r.message
      weight = null
      await lookup()
    } catch (e) {
      error = e.message
    }
  }
</script>

<div class="card">
  <h3>复称（重量争议处理）</h3>
  <p style="color: #5a6b75; font-size: 13px;">
    顾客对取货重量有异议时先复称。原始读数与复核读数都会保留，销售重量不会被修改。
  </p>
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
      订单 <strong>{o.order_no}</strong>（{o.fish_type}）
      销售重量：<strong>{o.sale_weight.toFixed(2)} kg</strong>
      <span class="tag {o.status}">{o.status === 'stored' ? '暂存中' : '已交付'}</span>
    </p>

    <h4>称重记录（全部保留）</h4>
    <table>
      <thead><tr><th>类型</th><th>重量</th><th>设备</th><th>时间</th></tr></thead>
      <tbody>
        {#each o.weigh_records as r}
          <tr>
            <td>{r.kind === 'original' ? '🔹 原始读数' : '🔁 复核读数'}</td>
            <td><strong>{r.weight.toFixed(2)} kg</strong></td>
            <td>{r.device_id}</td>
            <td>{r.created_at}</td>
          </tr>
        {/each}
      </tbody>
    </table>

    {#if o.status === 'stored'}
      <div class="row" style="align-items: flex-end;">
        <div>
          <label>复称设备</label>
          <select bind:value={device_id}>
            {#each devices as d}<option value={d}>{d}</option>{/each}
          </select>
        </div>
        <div>
          <label>复核重量（kg）</label>
          <input type="number" step="0.01" min="0.01" bind:value={weight} placeholder="可点右侧读秤" />
        </div>
        <div style="flex: 0 0 auto;">
          <button class="ghost" style="margin: 0;" on:click={readScale}>⚖️ 读取电子秤</button>
        </div>
      </div>
      <button class="primary" on:click={submit}>记录复称结果</button>
    {:else}
      <p class="error">订单已交付，不能复称。</p>
    {/if}
  </div>
{/if}
