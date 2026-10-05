<script>
  import { onMount } from 'svelte'
  import { api } from '../api.js'

  let orders = []
  let detail = null
  let error = ''

  onMount(load)

  async function load() {
    error = ''
    try {
      const r = await api.get('/orders')
      orders = r.orders
    } catch (e) {
      error = e.message
    }
  }

  async function open(id) {
    const r = await api.get(`/orders/${id}`)
    detail = r.order
  }
</script>

<div class="card">
  <h3>订单列表 <button class="ghost" style="margin: 0 0 0 10px;" on:click={load}>刷新</button></h3>
  {#if error}<div class="error">{error}</div>{/if}
  <table>
    <thead>
      <tr>
        <th>订单号</th><th>顾客</th><th>品种</th><th>重量</th>
        <th>鱼箱</th><th>箱位</th><th>取货码</th><th>状态</th><th></th>
      </tr>
    </thead>
    <tbody>
      {#each orders as o}
        <tr>
          <td>{o.order_no}</td>
          <td>{o.customer_name}</td>
          <td>{o.fish_type}</td>
          <td>{o.sale_weight.toFixed(2)} kg</td>
          <td>{o.box_code}</td>
          <td>{o.slot_code}</td>
          <td><strong>{o.pickup_code ?? '-'}</strong></td>
          <td><span class="tag {o.status}">{o.status === 'stored' ? '暂存中' : '已交付'}</span></td>
          <td><button class="ghost" style="margin: 0; padding: 4px 10px;" on:click={() => open(o.id)}>详情</button></td>
        </tr>
      {:else}
        <tr><td colspan="9" style="color: #8aa;">暂无订单，请先在「称重入库」开单</td></tr>
      {/each}
    </tbody>
  </table>
</div>

{#if detail}
  <div class="card">
    <h3>订单详情 {detail.order_no}</h3>
    <div class="row">
      <div>
        <h4>称重记录</h4>
        <table>
          <thead><tr><th>类型</th><th>重量</th><th>设备</th><th>时间</th></tr></thead>
          <tbody>
            {#each detail.weigh_records as r}
              <tr>
                <td>{r.kind === 'original' ? '原始' : '复核'}</td>
                <td>{r.weight.toFixed(2)} kg</td>
                <td>{r.device_id}</td>
                <td>{r.created_at}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <div>
        <h4>转箱记录</h4>
        <table>
          <thead><tr><th>从</th><th>到</th><th>经手人</th><th>时间</th></tr></thead>
          <tbody>
            {#each detail.transfers as t}
              <tr>
                <td>{t.from_box}/{t.from_slot}</td>
                <td>{t.to_box}/{t.to_slot}</td>
                <td>{t.operator}</td>
                <td>{t.created_at}</td>
              </tr>
            {:else}
              <tr><td colspan="4" style="color: #8aa;">无</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
    <button class="ghost" on:click={() => (detail = null)}>关闭</button>
  </div>
{/if}
