<script>
  import { kg } from '../api.js'
  export let detail

  const kindLabel = (k) => (k === 'original' ? '原始称重' : '复核称重')
  const statusLabel = (s) => (s === 'stored' ? '暂存中' : '已交付')
</script>

{#if detail}
  <div class="card">
    <div class="head">
      <h3>订单 <span class="mono">{detail.order.order_no}</span></h3>
      <span class="badge {detail.order.status === 'stored' ? 'blue' : 'gray'}">
        {statusLabel(detail.order.status)}
      </span>
    </div>

    <div class="grid2">
      <div><label>顾客</label>{detail.order.customer_name}（{detail.order.customer_phone}）</div>
      <div><label>鱼种</label>{detail.order.fish_species}</div>
      <div><label>销售重量</label><b>{kg(detail.order.sale_weight_grams)}</b></div>
      <div>
        <label>取货码</label>
        {#if detail.pickup_code}
          <span class="mono">{detail.pickup_code.code}</span>
          <span class="badge {detail.pickup_code.status === 'active' ? 'green' : 'gray'}">
            {detail.pickup_code.status === 'active' ? '有效' : '已失效'}
          </span>
        {:else}-{/if}
      </div>
      <div><label>当前鱼箱</label><b class="mono">{detail.current_box?.box_no ?? '-'}</b></div>
      <div><label>暂存箱位</label><b>{detail.current_box?.location ?? '-'}</b></div>
      <div><label>下单时间</label>{detail.order.created_at}</div>
      {#if detail.order.delivered_at}
        <div><label>交付时间</label>{detail.order.delivered_at}</div>
      {/if}
    </div>

    <h4>称重记录（原始值与复核值均保留）</h4>
    <table>
      <thead>
        <tr><th>类型</th><th>电子秤设备</th><th>重量</th><th>备注</th><th>时间</th></tr>
      </thead>
      <tbody>
        {#each detail.weighings as w}
          <tr>
            <td><span class="badge {w.kind === 'original' ? 'blue' : 'orange'}">{kindLabel(w.kind)}</span></td>
            <td class="mono">{w.scale_device_id}</td>
            <td><b>{kg(w.weight_grams)}</b></td>
            <td>{w.note ?? ''}</td>
            <td>{w.created_at}</td>
          </tr>
        {/each}
      </tbody>
    </table>

    {#if detail.transfers.length}
      <h4>转箱记录</h4>
      <table>
        <thead>
          <tr><th>原鱼箱</th><th>新鱼箱</th><th>原箱位</th><th>新箱位</th><th>经手人</th><th>时间</th></tr>
        </thead>
        <tbody>
          {#each detail.transfers as t}
            <tr>
              <td class="mono">{t.from_box_no}</td>
              <td class="mono">{t.to_box_no}</td>
              <td>{t.from_location}</td>
              <td>{t.to_location}</td>
              <td>{t.operator}</td>
              <td>{t.created_at}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
{/if}

<style>
  .head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
  .head h3 { margin: 0; }
</style>
