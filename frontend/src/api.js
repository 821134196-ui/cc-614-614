async function req(method, url, body) {
  const res = await fetch(url, {
    method,
    headers: body ? { 'content-type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined
  })
  const data = await res.json().catch(() => ({}))
  if (!res.ok) {
    const err = new Error(data?.error?.message || `请求失败 (${res.status})`)
    err.code = data?.error?.code
    err.status = res.status
    throw err
  }
  return data
}

export const api = {
  listBoxes: (status) => req('GET', '/api/boxes' + (status ? `?status=${status}` : '')),
  createBox: (b) => req('POST', '/api/boxes', b),
  readScale: (deviceId, weight) =>
    req('GET', `/api/mock/scale/read?device_id=${encodeURIComponent(deviceId)}${weight ? `&weight=${weight}` : ''}`),
  createOrder: (o) => req('POST', '/api/orders', o),
  listOrders: () => req('GET', '/api/orders'),
  getOrder: (id) => req('GET', `/api/orders/${id}`),
  lookup: (code) => req('GET', `/api/pickup/${encodeURIComponent(code)}`),
  deliver: (code) => req('POST', `/api/pickup/${encodeURIComponent(code)}/deliver`, {}),
  transfer: (id, t) => req('POST', `/api/orders/${id}/transfer`, t),
  reweigh: (id, r) => req('POST', `/api/orders/${id}/reweigh`, r),
  sms: () => req('GET', '/api/mock/sms')
}

export const kg = (g) => (g / 1000).toFixed(2) + ' kg'
export const yuan = (cents) => '¥' + (cents / 100).toFixed(2)
