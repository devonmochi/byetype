import React, { useState, useEffect, useMemo, useRef, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { onEvent } from '../../../lib/tauri-api'
import type { UsageRecord, UsageScene } from '../../../core/types'

const SCENES: { id: UsageScene; label: string; color: string }[] = [
  { id: 'transcribe', label: '转写', color: '#007aff' },
  { id: 'extract', label: '识别', color: '#ff9500' },
  { id: 'optimize', label: '优化', color: '#34c759' },
  { id: 'learn', label: '学习', color: '#af52de' },
]

type RangeId = 'today' | '7d' | '30d' | 'all'

const RANGES: { id: RangeId; label: string; trendBase: string }[] = [
  { id: 'today', label: '今天', trendBase: '较昨天' },
  { id: '7d', label: '近7天', trendBase: '较上一周' },
  { id: '30d', label: '近30天', trendBase: '较上一月' },
  { id: 'all', label: '全部', trendBase: '较前半周期' },
]

const DAY_MS = 86400000
const HOUR_MS = 3600000

function fmtTok(n: number): string {
  if (n >= 1e8) return (n / 1e8).toFixed(2) + '亿'
  if (n >= 1e4) return (n / 1e4).toFixed(1) + '万'
  return n.toLocaleString('zh-CN')
}

function fmtNum(n: number): string {
  return n.toLocaleString('zh-CN')
}

function pad(n: number): string {
  return (n < 10 ? '0' : '') + n
}

function labelMD(ts: number): string {
  const d = new Date(ts)
  return pad(d.getMonth() + 1) + '/' + pad(d.getDate())
}

function startOfDay(ts: number): number {
  const d = new Date(ts)
  d.setHours(0, 0, 0, 0)
  return d.getTime()
}

interface Bucket {
  label: string
  tipLabel: string
  scenes: Partial<Record<UsageScene, number>>
  total: number
}

interface ModelAgg {
  model: string
  provider: string
  calls: number
  input: number
  output: number
  total: number
}

interface TipState {
  x: number
  y: number
  h: number
  w: number
  index: number
}

export function UsageTab() {
  const [records, setRecords] = useState<UsageRecord[]>([])
  const [range, setRange] = useState<RangeId>('today')
  const [scene, setScene] = useState<'all' | UsageScene>('all')
  const [tip, setTip] = useState<TipState | null>(null)
  const [confirming, setConfirming] = useState(false)
  const confirmTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const chartWrapRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    let cancelled = false
    const load = () => {
      invoke<UsageRecord[]>('get_usage_records')
        .then(r => { if (!cancelled) setRecords(r) })
        .catch(() => {})
    }
    load()

    let unlisten: (() => void) | null = null
    onEvent<null>('usage-updated', () => load())
      .then(fn => { if (cancelled) fn(); else unlisten = fn })

    return () => {
      cancelled = true
      unlisten?.()
      if (confirmTimer.current) clearTimeout(confirmTimer.current)
    }
  }, [])

  const handleClear = useCallback(async () => {
    if (!confirming) {
      setConfirming(true)
      if (confirmTimer.current) clearTimeout(confirmTimer.current)
      confirmTimer.current = setTimeout(() => setConfirming(false), 3000)
      return
    }
    if (confirmTimer.current) clearTimeout(confirmTimer.current)
    setConfirming(false)
    try {
      await invoke('clear_usage_records')
      const r = await invoke<UsageRecord[]>('get_usage_records')
      setRecords(r)
    } catch (e) {
      console.error('clear usage failed:', e)
    }
  }, [confirming])

  const handleBarEnter = useCallback((e: React.MouseEvent, index: number) => {
    const wrap = chartWrapRef.current
    const col = e.currentTarget as HTMLElement
    if (!wrap) return
    const wrapRect = wrap.getBoundingClientRect()
    const colRect = col.getBoundingClientRect()
    setTip({
      x: colRect.left - wrapRect.left + colRect.width / 2,
      y: colRect.top - wrapRect.top,
      h: colRect.height,
      w: wrapRect.width,
      index,
    })
  }, [])

  const view = useMemo(() => {
    const now = Date.now()
    const todayStart = startOfDay(now)
    const sorted = [...records].sort((a, b) => a.ts - b.ts)
    const firstTs = sorted.length > 0 ? sorted[0].ts : now
    const allStart = startOfDay(firstTs)

    let curStart = 0
    let prevStart = 0
    let prevEnd = 0
    let days = 1

    switch (range) {
      case 'today':
        curStart = todayStart
        prevStart = todayStart - DAY_MS
        prevEnd = todayStart
        days = 1
        break
      case '7d':
        curStart = todayStart - 6 * DAY_MS
        prevStart = todayStart - 13 * DAY_MS
        prevEnd = todayStart - 6 * DAY_MS
        days = 7
        break
      case '30d':
        curStart = todayStart - 29 * DAY_MS
        prevStart = todayStart - 59 * DAY_MS
        prevEnd = todayStart - 29 * DAY_MS
        days = 30
        break
      case 'all':
      default:
        curStart = allStart
        days = Math.max(1, Math.floor((todayStart - allStart) / DAY_MS) + 1)
        break
    }

    const inScene = (r: UsageRecord) => scene === 'all' || r.scene === scene
    const current = sorted.filter(r => r.ts >= curStart && inScene(r))

    const input = current.reduce((s, r) => s + r.inputTokens, 0)
    const output = current.reduce((s, r) => s + r.outputTokens, 0)
    const total = input + output
    const calls = current.length

    // 场景筛选按钮上的调用次数（当前时间范围、不限场景）
    const sceneCounts: Record<string, number> = { all: 0 }
    for (const r of sorted) {
      if (r.ts >= curStart) {
        sceneCounts.all++
        sceneCounts[r.scene] = (sceneCounts[r.scene] || 0) + 1
      }
    }

    // 按模型汇总
    const byModel = new Map<string, ModelAgg>()
    for (const r of current) {
      const key = r.model + '|' + r.provider
      let agg = byModel.get(key)
      if (!agg) {
        agg = { model: r.model, provider: r.provider, calls: 0, input: 0, output: 0, total: 0 }
        byModel.set(key, agg)
      }
      agg.calls++
      agg.input += r.inputTokens
      agg.output += r.outputTokens
      agg.total += r.inputTokens + r.outputTokens
    }
    const models = [...byModel.values()].sort((a, b) => b.total - a.total || b.calls - a.calls)
    const topModel = models[0]
    const shareBase = total > 0 ? total : calls
    const topShare = topModel && shareBase > 0
      ? Math.round(((total > 0 ? topModel.total : topModel.calls) / shareBase) * 100)
      : 0

    // 消耗趋势：与上一周期对比（「全部」与前半段对比）
    let trendCur = total
    let trendPrev = 0
    if (range === 'all') {
      const midpoint = (allStart + now) / 2
      trendCur = 0
      for (const r of current) {
        const t = r.inputTokens + r.outputTokens
        if (r.ts < midpoint) trendPrev += t
        else trendCur += t
      }
    } else {
      trendPrev = sorted
        .filter(r => r.ts >= prevStart && r.ts < prevEnd && inScene(r))
        .reduce((s, r) => s + r.inputTokens + r.outputTokens, 0)
    }
    const trend = trendPrev > 0 ? Math.round(((trendCur - trendPrev) / trendPrev) * 100) : null

    // 图表分桶
    const buckets: Bucket[] = []
    const addToBucket = (r: UsageRecord, b: Bucket) => {
      const t = r.inputTokens + r.outputTokens
      b.scenes[r.scene] = (b.scenes[r.scene] || 0) + t
      b.total += t
    }

    if (range === 'today') {
      const curHour = new Date(now).getHours()
      for (let h = 0; h <= curHour; h++) {
        buckets.push({ label: h + '时', tipLabel: h + '时', scenes: {}, total: 0 })
      }
      for (const r of current) {
        const idx = Math.floor((r.ts - todayStart) / HOUR_MS)
        if (idx >= 0 && idx < buckets.length) addToBucket(r, buckets[idx])
      }
    } else if (range === '7d' || range === '30d') {
      const n = range === '7d' ? 7 : 30
      for (let i = n - 1; i >= 0; i--) {
        buckets.push({ label: labelMD(todayStart - i * DAY_MS), tipLabel: labelMD(todayStart - i * DAY_MS), scenes: {}, total: 0 })
      }
      for (const r of current) {
        const idx = Math.floor((r.ts - curStart) / DAY_MS)
        if (idx >= 0 && idx < buckets.length) addToBucket(r, buckets[idx])
      }
    } else {
      const bucketCount = Math.min(days, 14)
      const bucketDays = Math.max(1, Math.ceil(days / bucketCount))
      const bucketMs = bucketDays * DAY_MS
      const count = Math.ceil((now - allStart) / bucketMs)
      for (let i = 0; i < count; i++) {
        const s = allStart + i * bucketMs
        const e = s + bucketMs
        const l = labelMD(s)
        buckets.push({
          label: l,
          tipLabel: bucketDays > 1 ? l + '–' + labelMD(e - DAY_MS) : l,
          scenes: {},
          total: 0,
        })
      }
      for (const r of current) {
        const idx = Math.floor((r.ts - allStart) / bucketMs)
        if (idx >= 0 && idx < buckets.length) addToBucket(r, buckets[idx])
      }
    }
    const maxBucket = Math.max(...buckets.map(b => b.total), 0)
    const labelStep = buckets.length > 20 ? Math.ceil(buckets.length / 8) : 1

    const rangeLabel = RANGES.find(r => r.id === range)?.label ?? ''
    const trendBase = RANGES.find(r => r.id === range)?.trendBase ?? ''

    return {
      hasAny: sorted.length > 0,
      input, output, total, calls,
      sceneCounts, models, topModel, topShare,
      trend, trendBase,
      buckets, maxBucket, labelStep,
      days, rangeLabel,
    }
  }, [records, range, scene])

  const handleRangeChange = useCallback((id: RangeId) => {
    setRange(id)
    setTip(null)
  }, [])

  const rangeTabs = (
    <div className="range-tabs">
      {RANGES.map(r => (
        <button
          key={r.id}
          className={`range-tab${range === r.id ? ' active' : ''}`}
          onClick={() => handleRangeChange(r.id)}
        >
          {r.label}
        </button>
      ))}
    </div>
  )

  if (!view.hasAny) {
    return (
      <div>
        <h2 className="content-title">用量统计</h2>
        {rangeTabs}
        <div className="usage-empty">暂无用量记录，使用语音转写、图像识别或自动学习后自动累积</div>
      </div>
    )
  }

  const trendClass = view.trend === null ? '' : view.trend >= 0 ? 'usage-trend-up' : 'usage-trend-down'

  return (
    <div
      onMouseLeave={() => setTip(null)}
    >
      <h2 className="content-title">用量统计</h2>

      <div className="usage-toolbar">
        {rangeTabs}
        <div className="scene-chips">
          <button
            className={`scene-chip${scene === 'all' ? ' active' : ''}`}
            onClick={() => { setScene('all'); setTip(null) }}
          >
            全部 <span className="cnt">{fmtNum(view.sceneCounts.all || 0)}次</span>
          </button>
          {SCENES.map(s => (
            <button
              key={s.id}
              className={`scene-chip${scene === s.id ? ' active' : ''}`}
              onClick={() => { setScene(s.id); setTip(null) }}
            >
              <span className="chip-dot" style={{ background: s.color }} />
              {s.label}
              <span className="cnt">{fmtNum(view.sceneCounts[s.id] || 0)}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="usage-cards">
        <div className="usage-stat-card">
          <div className="usage-stat-label">总消耗Token · {view.rangeLabel}</div>
          <div className="usage-stat-value num">{fmtTok(view.total)}</div>
          <div className="usage-stat-sub num">输入 {fmtTok(view.input)} · 输出 {fmtTok(view.output)}</div>
        </div>
        <div className="usage-stat-card">
          <div className="usage-stat-label">API调用</div>
          <div className="usage-stat-value num">{fmtNum(view.calls)}<span style={{ fontSize: 13, fontWeight: 400 }}> 次</span></div>
          <div className="usage-stat-sub num">日均 {fmtNum(Math.round(view.calls / view.days))} 次</div>
        </div>
        <div className="usage-stat-card">
          <div className="usage-stat-label">最常用模型</div>
          <div className="usage-stat-value small" title={view.topModel?.model}>{view.topModel ? view.topModel.model : '—'}</div>
          <div className="usage-stat-sub num">{view.topModel ? `占${view.topShare}% · ${fmtTok(view.topModel.total)}` : '暂无调用'}</div>
        </div>
        <div className="usage-stat-card">
          <div className="usage-stat-label">消耗趋势</div>
          <div className={`usage-stat-value num ${trendClass}`}>
            {view.trend === null ? '—' : `${view.trend >= 0 ? '+' : ''}${view.trend}%`}
          </div>
          <div className="usage-stat-sub">
            {view.trend === null ? '上期无记录' : `${view.trendBase}${view.trend >= 0 ? '增加' : '减少'}`}
          </div>
        </div>
      </div>

      <div className="usage-panel">
        <div className="usage-panel-head">
          <span className="usage-panel-title">
            Token用量{range === 'today' ? '（按小时）' : range === 'all' ? '（按天及以上汇总）' : '（按天）'}
          </span>
          <div className="usage-legend">
            {SCENES.map(s => (
              <span key={s.id} className="usage-legend-item">
                <span className="chip-dot" style={{ background: s.color }} />
                {s.label}
              </span>
            ))}
          </div>
        </div>
        <div className="usage-chart-wrap" ref={chartWrapRef}>
          <div className="usage-chart">
            {view.buckets.map((b, i) => (
              <div
                key={i}
                className={`usage-bar-col${b.total === 0 ? ' empty' : ''}`}
                onMouseEnter={e => handleBarEnter(e, i)}
              >
                {b.total > 0 && view.maxBucket > 0 && SCENES.map(s => {
                  const v = b.scenes[s.id]
                  if (!v) return null
                  return (
                    <div
                      key={s.id}
                      className="usage-bar-seg"
                      style={{ height: `${(v / view.maxBucket) * 100}%`, background: s.color }}
                    />
                  )
                })}
              </div>
            ))}
          </div>
          <div className="usage-x-axis">
            {view.buckets.map((b, i) => (
              <div key={i} className="usage-x-label">
                {i % view.labelStep === 0 || i === view.buckets.length - 1 ? b.label : ''}
              </div>
            ))}
          </div>
          {tip && view.buckets[tip.index] && (() => {
            const flipBelow = tip.y < 90
            return (
              <div
                className={`usage-tooltip${flipBelow ? ' below' : ''}`}
                style={{
                  display: 'block',
                  left: Math.max(85, Math.min(tip.x, tip.w - 85)),
                  top: flipBelow ? tip.y + tip.h + 8 : tip.y - 8,
                }}
              >
                <div className="tt-date">{view.buckets[tip.index].tipLabel}</div>
                {SCENES.filter(s => view.buckets[tip.index].scenes[s.id]).map(s => (
                  <div key={s.id} className="tt-row">
                    <span className="tt-scene">
                      <span className="chip-dot" style={{ background: s.color }} />
                      {s.label}
                    </span>
                    <b>{fmtNum(view.buckets[tip.index].scenes[s.id] || 0)}</b>
                  </div>
                ))}
                <div className="tt-row tt-total">
                  <span>合计</span>
                  <b>{fmtNum(view.buckets[tip.index].total)}</b>
                </div>
              </div>
            )
          })()}
        </div>
      </div>

      <div className="usage-panel">
        <div className="usage-panel-head"><span className="usage-panel-title">按模型汇总</span></div>
        <div style={{ overflowX: 'auto' }}>
          <table className="usage-table">
            <thead>
              <tr>
                <th>模型</th>
                <th>调用次数</th>
                <th>输入Token</th>
                <th>输出Token</th>
                <th>总Token</th>
                <th>占比</th>
              </tr>
            </thead>
            <tbody>
              {view.models.length === 0 ? (
                <tr><td colSpan={6} className="usage-table-empty">当前筛选条件下暂无调用</td></tr>
              ) : view.models.map(m => {
                const share = view.total > 0
                  ? Math.round((m.total / view.total) * 100)
                  : Math.round((m.calls / Math.max(view.calls, 1)) * 100)
                const barWidth = view.models[0].total > 0
                  ? (m.total / view.models[0].total) * 100
                  : (m.calls / Math.max(view.models[0].calls, 1)) * 100
                return (
                  <tr key={m.model + '|' + m.provider}>
                    <td>
                      <div className="usage-model-name" title={m.model}>{m.model}</div>
                      <div className="usage-model-provider">{m.provider}</div>
                    </td>
                    <td className="num">{fmtNum(m.calls)}</td>
                    <td className="num">{fmtTok(m.input)}</td>
                    <td className="num">{fmtTok(m.output)}</td>
                    <td className="num"><b>{fmtTok(m.total)}</b></td>
                    <td className="usage-share-cell">
                      <span className="usage-share-pct num">{share}%</span>
                      <div className="usage-share-bar">
                        <div className="usage-share-fill" style={{ width: `${barWidth}%` }} />
                      </div>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      </div>

      <div className="usage-panel">
        <div className="usage-actions">
          <div className="usage-storage-note">
            用量记录保存在本机，从功能上线后开始累积，此前的调用无法追溯；个别服务商不返回Token数时，该次调用记为0。
          </div>
          <button
            className={`usage-clear-btn${confirming ? ' confirming' : ''}`}
            onClick={handleClear}
          >
            {confirming ? '确认清空？' : '清空记录'}
          </button>
        </div>
      </div>
    </div>
  )
}
