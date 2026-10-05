<script>
  import { onMount } from 'svelte'
  import { api } from '../api.js'

  let boxes = []
  let slots = []
  const devices = ['SCALE-01', 'SCALE-02', 'SCALE-03']
  const fishTypes = ['鲈鱼', '桂花鱼', '石斑鱼', '多宝鱼', '草鱼', '鲫鱼']

  let customer_name = ''
  let customer_phone = ''
  let fish_type = '鲈鱼'
  let device_id = 'SCALE-01'
  let box_code = ''
  let slot_code = ''
  let weight = null
  let reading = false

  let error = ''
  let result = null

  onMount(loadMeta)

  async function loadMeta() {
    const [b, s] = await Promise.all([api.get('/boxes'), api.get('/slots')])
    boxes = b.boxes
    slots = s.slots
    const free = boxes.find((x) => !x.occupied)
    box_code = free ? free.code : ''
    slot_code = slots[0]?.code ?? ''
  }

  async function readScale() {
    reading = true
    error = ''
    try {
      const r = await api.get(`/mock/scale/${device_id}/read`)
      weight = r.weight
    } catch (e) {
      error = e.message
    } finally {
      reading = false
    }
  }

  async function submit() {
    error = ''
    result = null
    try {
      result = await api.post('/orders', {
        customer_name, customer_phone, fish_type, device_id,
        box_code, slot_code,
        weight: weight ?? undefined,
      })
      await loadMeta()
    } catch (e) {
      error = e.message
    }
  }
</script>

<div class="card">
  <h3>称重入库</h3>
  <div class="row">
    <div>
      <label>顾客姓名</label>
      <input bind:value={customer_name} placeholder="张三" />
    </div>
    <div>
      <label>手机号（接收取货码短信）</label>
      <input bind:value={customer_phone} placeholder="13800000000" />
    </div>
  </div>
  <div class="row">
    <div>
      <label>鱼类品种</label>
      <select bind:value={fish_type}>
        {#each fishTypes as f}<option value={f}>{f}</option>{/each}
      </select>
    </div>
    <div>
      <label>电子秤设备号</label>
      <select bind:value={device_id} on:change={() => (weight = null)}>
        {#each devices as d}<option value={d}>{d}</option>{/each}
      </select>
    </div>
  </div>
  <div class="row">
    <div>
      <label>鱼箱编号（已被占用的鱼箱不可选）</label>
      <select bind:value={box_code}>
        {#each boxes as b}
          <option value={b.code} disabled={b.occupied}>
            {b.code}{b.occupied ? '（占用中）' : ''}
          </option>
        {/each}
      </select>
    </div>
    <div>
      <label>暂存箱位</label>
      <select bind:value={slot_code}>
        {#each slots as s}<option value={s.code}>{s.code}</option>{/each}
      </select>
    </div>
  </div>

  <label>重量（kg）— 从电子秤读取，或手工录入</label>
  <div class="row" style="align-items: center;">
    <div>
      <input type="number" step="0.01" min="0.01" bind:value={weight} placeholder="点击右侧按钮读秤" />
    </div>
    <div style="flex: 0 0 auto;">
      <button class="ghost" style="margin: 0;" on:click={readScale} disabled={reading}>
        {reading ? '读数中…' : '⚖️ 读取电子秤'}
      </button>
    </div>
  </div>

  <button class="primary" on:click={submit} disabled={!customer_name || !box_code || !slot_code}>
    保存并生成取货码
  </button>
  {#if error}<div class="error">{error}</div>{/if}
</div>

{#if result}
  <div class="card">
    <div class="success">✅ 已入库，取货码已通过短信（模拟）发送给顾客</div>
    <div class="big-code">{result.pickup_code}</div>
    <table>
      <tbody>
        <tr><th>订单号</th><td>{result.order.order_no}</td></tr>
        <tr><th>鱼箱 / 箱位</th><td>{result.order.box_code} / {result.order.slot_code}</td></tr>
        <tr><th>原始重量</th><td>{result.order.sale_weight.toFixed(2)} kg（{result.order.device_id}）</td></tr>
        <tr><th>短信内容</th><td>{result.sms}</td></tr>
      </tbody>
    </table>
  </div>
{/if}
