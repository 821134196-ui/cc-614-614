<script>
  import { onMount } from 'svelte'
  import { api, kg } from '../api.js'

  const DEVICES = ['SCALE-01', 'SCALE-02', 'SCALE-03']
  const SPECIES = ['鲈鱼', '石斑鱼', '多宝鱼', '草鱼', '鲫鱼', '鳜鱼', '基围虾', '帝王蟹', '波士顿龙虾']

  let form = emptyForm()
  let idleBoxes = []
  let error = ''
  let busy = false
  let scaleBusy = false
  let result = null

  function emptyForm() {
    return {
      customer_name: '', customer_phone: '', fish_species: '', unit_price: '',
      scale_device_id: 'SCALE-01', box_no: '', location: '', weight_grams: null
    }
  }

  async function loadBoxes() {
    idleBoxes = await api.listBoxes('idle')
  }
  onMount(loadBoxes)

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
    busy = true
    try {
      result = await api.createOrder({
        customer_name: form.customer_name,
        customer_phone: form.customer_phone,
        fish_species: form.fish_species,
        unit_price_cents: Math.round(parseFloat(form.unit_price || '0') * 100),
        scale_device_id: form.scale_device_id,
        box_no: form.box_no,
        location: form.location,
        weight_grams: form.weight_grams
      })
      form = emptyForm()
      await loadBoxes()
    } catch (e) {
      error = e.message
    } finally {
      busy = false
    }
  }

  function next() {
    result = null
  }

  $: amount =
    form.weight_grams && form.unit_price
      ? ((form.weight_grams / 1000) * parseFloat(form.unit_price)).toFixed(2)
      : null
</script>

{#if result}
  <div class="card done">
    <h3>✅ 入库成功，短信已发送（模拟）</h3>
    <p class="hint">请将取货码告知顾客，取货时凭码核对。</p>
    <div class="big-code">{result.pickup_code.code}</div>
    <div class="grid2">
      <div><label>订单号</label><span class="mono">{result.order.order_no}</span></div>
      <div><label>鱼种 / 重量</label>{result.order.fish_species} / {kg(result.order.sale_weight_grams)}</div>
      <div><label>鱼箱</label><b class="mono">{result.current_box.box_no}</b></div>
      <div><label>暂存箱位</label><b>{result.current_box.location}</b></div>
      <div><label>电子秤</label><span class="mono">{result.weighings[0].scale_device_id}</span></div>
    </div>
    <div class="actions">
      <button class="btn" on:click={next}>再开一单</button>
    </div>
  </div>
{:else}
  <div class="card">
    <h3>称重入库</h3>
    <div class="row">
      <div class="field"><label>顾客姓名 *</label><input bind:value={form.customer_name} placeholder="张三" /></div>
      <div class="field"><label>手机号 *</label><input bind:value={form.customer_phone} placeholder="13800000000" /></div>
      <div class="field">
        <label>鱼种 *</label>
        <input bind:value={form.fish_species} list="species" placeholder="选择或输入" />
        <datalist id="species">
          {#each SPECIES as s}<option value={s} />{/each}
        </datalist>
      </div>
      <div class="field"><label>单价（元/公斤）</label><input type="number" min="0" step="0.1" bind:value={form.unit_price} placeholder="56" /></div>
    </div>

    <div class="row">
      <div class="field">
        <label>电子秤设备号</label>
        <select bind:value={form.scale_device_id}>
          {#each DEVICES as d}<option value={d}>{d}</option>{/each}
        </select>
      </div>
      <div class="field">
        <label>重量（克）— 由模拟电子秤读入</label>
        <div style="display:flex; gap:8px">
          <input type="number" min="1" bind:value={form.weight_grams} placeholder="点击右侧按钮读数" />
          <button class="btn secondary" on:click={readScale} disabled={scaleBusy}>
            {scaleBusy ? '读数中…' : '⚖ 读取电子秤'}
          </button>
        </div>
      </div>
      <div class="field">
        <label>金额</label>
        <input value={amount ? `¥${amount}` : '—'} disabled />
      </div>
    </div>

    <div class="row">
      <div class="field">
        <label>鱼箱编号 *（仅显示空闲箱）</label>
        <select bind:value={form.box_no}>
          <option value="">请选择</option>
          {#each idleBoxes as b}<option value={b.box_no}>{b.box_no}</option>{/each}
        </select>
      </div>
      <div class="field"><label>暂存箱位 *</label><input bind:value={form.location} placeholder="如 A区-3" /></div>
    </div>

    {#if error}<div class="error">{error}</div>{/if}

    <div class="actions">
      <button
        class="btn"
        on:click={submit}
        disabled={busy || !form.customer_name || !form.customer_phone || !form.fish_species || !form.box_no || !form.location || !form.weight_grams}
      >
        {busy ? '提交中…' : '保存并生成取货码'}
      </button>
    </div>
  </div>
{/if}

<style>
  .done { text-align: center; }
  .done .grid2 { text-align: left; margin-top: 16px; }
  .done .actions { justify-content: center; }
</style>
