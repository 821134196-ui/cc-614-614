<script>
  import { onMount } from 'svelte'
  import { api, kg } from '../api.js'

  let orders = []
  let boxes = []
  let sms = []
  let error = ''
  let newBox = { box_no: '', location: '空箱架' }
  let boxMsg = ''

  async function load() {
    error = ''
    try {
      ;[orders, boxes, sms] = await Promise.all([api.listOrders(), api.listBoxes(), api.sms()])
    } catch (e) {
      error = e.message
    }
  }
  onMount(load)

  async function addBox() {
    boxMsg = ''
    error = ''
    try {
      await api.createBox(newBox)
      boxMsg = `鱼箱 ${newBox.box_no} 已添加`
      newBox = { box_no: '', location: '空箱架' }
      await load()
    } catch (e) {
      error = e.message
    }
  }
</script>

{#if error}<div class="error">{error}</div>{/if}

<div class="card">
  <div style="display:flex; justify-content:space-between; align-items:center">
    <h3>订单（{orders.length}）</h3>
    <button class="btn secondary small" on:click={load}>刷新</button>
  </div>
  <div style="overflow-x:auto">
    <table>
      <thead>
        <tr><th>订单号</th><th>顾客</th><th>鱼种</th><th>销售重量</th><th>状态</th><th>取货码</th><th>鱼箱</th><th>箱位</th></tr>
      </thead>
      <tbody>
        {#each orders as o}
          <tr>
            <td class="mono">{o.order_no}</td>
            <td>{o.customer_name}</td>
            <td>{o.fish_species}</td>
            <td>{kg(o.sale_weight_grams)}</td>
            <td><span class="badge {o.status === 'stored' ? 'blue' : 'gray'}">{o.status === 'stored' ? '暂存中' : '已交付'}</span></td>
            <td class="mono">
              {o.pickup_code ?? '-'}
              {#if o.code_status}<span class="badge {o.code_status === 'active' ? 'green' : 'gray'}">{o.code_status === 'active' ? '有效' : '已失效'}</span>{/if}
            </td>
            <td class="mono">{o.box_no ?? '-'}</td>
            <td>{o.location ?? '-'}</td>
          </tr>
        {/each}
        {#if !orders.length}<tr><td colspan="8" style="color:var(--muted)">暂无订单</td></tr>{/if}
      </tbody>
    </table>
  </div>
</div>

<div class="card">
  <h3>鱼箱（{boxes.filter((b) => b.status === 'in_use').length} 在用 / {boxes.length} 总计）</h3>
  <div style="overflow-x:auto">
    <table>
      <thead>
        <tr><th>箱号</th><th>状态</th><th>当前位置</th><th>绑定订单</th></tr>
      </thead>
      <tbody>
        {#each boxes as b}
          <tr>
            <td class="mono">{b.box_no}</td>
            <td><span class="badge {b.status === 'idle' ? 'green' : 'orange'}">{b.status === 'idle' ? '空闲' : '使用中'}</span></td>
            <td>{b.location}</td>
            <td class="mono">{b.order_no ?? '-'}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <h4>添加鱼箱</h4>
  <div style="display:flex; gap:8px; max-width:520px">
    <input bind:value={newBox.box_no} placeholder="箱号，如 D-01" />
    <input bind:value={newBox.location} placeholder="存放位置" />
    <button class="btn secondary" on:click={addBox} disabled={!newBox.box_no}>添加</button>
  </div>
  {#if boxMsg}<div class="success">{boxMsg}</div>{/if}
</div>

<div class="card">
  <h3>模拟短信记录（最近 {sms.length} 条）</h3>
  <table>
    <thead>
      <tr><th>时间</th><th>手机号</th><th>内容</th></tr>
    </thead>
    <tbody>
      {#each sms as s}
        <tr><td style="white-space:nowrap">{s.created_at}</td><td class="mono">{s.phone}</td><td>{s.content}</td></tr>
      {/each}
      {#if !sms.length}<tr><td colspan="3" style="color:var(--muted)">暂无短信</td></tr>{/if}
    </tbody>
  </table>
</div>
