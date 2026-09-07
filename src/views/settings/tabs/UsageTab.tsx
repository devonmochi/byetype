import React, { useState, useEffect, useMemo, useRef, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { onEvent } from '../../../lib/tauri-api'
import type { UsageRecord, UsageScene, TimingRecord } from '../../../core/types'

const SCENES: { id: UsageScene; label: string; color: string }[] = [
  { id: 'transcribe', label: '转写', color: '#007aff' },
  { id: 'extract', label: '识别', color: '#ff9500' },
  { id: 'optimize', label: '优化', color: '#34c759' },
  { id: 'learn', label: '学习', color: '#af52de' },
]

const TIMING_COLORS = { transcribe: '#007aff', optimize: '#34c759' }

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

function fmtMs(ms: number): string {
  if (!ms || ms <= 0) return '—'
  const s = ms / 1000
  if (s < 10) return s.toFixed(1) + '秒'
  if (s < 60) return Math.round(s) + '秒'
  const m = Math.floor(s / 60)
  return m + '分' + Math.round(s - m * 60) + '秒'
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

interface BucketLabel {
  label: string
  tipLabel: string
}

interface UsageBucket extends BucketLabel {
  scenes: Partial<Record<UsageScene, number>>
  total: number
}

interface TimingBucket extends BucketLabel {
  count: number
  avgTrans: number
  avgOpt: number
  avgOther: number
  avgTotal: number
}

interface UsageModelAgg {
  model: string
  provider: string
  calls: number
  input: number
  output: number
  total: number
}

interface TimingModelAgg {
  model: string
  provider: string
  transCount: number
  transMs: number
  optCount: number
  optMs: number
  calls: number
  avgTrans: number
  avgOpt: number
}

interface TipState {
  x: number
  y: number
  h: number
  w: number
  index: number
}

/** 当前时间范围与对照周期的边界。 */
function periodBounds(range: RangeId, sortedTs: number[], now: number) {
  const todayStart = startOfDay(now)
  const firstTs = sortedTs.length > 0 ? sortedTs[0] : now
  const allStart = startOfDay(firstTs)

  switch (range) {
    case 'today':
      return { curStart: todayStart, prevStart: todayStart - DAY_MS, prevEnd: todayStart, days: 1, allStart }
    case '7d':
      return { curStart: todayStart - 6 * DAY_MS, prevStart: todayStart - 13 * DAY_MS, prevEnd: todayStart - 6 * DAY_MS, days: 7, allStart }
    case '30d':
      return { curStart: todayStart - 29 * DAY_MS, prevStart: todayStart - 59 * DAY_MS, prevEnd: todayStart - 29 * DAY_MS, days: 30, allStart }
    case 'all':
    default:
      return { curStart: allStart, prevStart: 0, prevEnd: 0, days: Math.max(1, Math.floor((todayStart - allStart) / DAY_MS) + 1), allStart }
  }
}

/** 生成时间范围的桶标签，并给出把时间戳映射到桶下标的方法。 */
function makeBuckets(range: RangeId, days: number, allStart: number, now: number) {
  const todayStart = startOfDay(now)
  const labels: BucketLabel[] = []
  let index: (ts: number) => number

  if (range === 'today') {
    const curHour = new Date(now).getHours()
    for (let h = 0; h <= curHour; h++) labels.push({ label: h + '时', tipLabel: h + '时' })
    index = ts => Math.floor((ts - todayStart) / HOUR_MS)
  } else if (range === '7d' || range === '30d') {
    const n = range === '7d' ? 7 : 30
    for (let i = n - 1; i >= 0; i--) {
      const l = labelMD(todayStart - i * DAY_MS)
      labels.push({ label: l, tipLabel: l })
    }
    index = ts => Math.floor((ts - (todayStart - (n - 1) * DAY_MS)) / DAY_MS)
  } else {
    const bucketCount = Math.min(days, 14)
    const bucketDays = Math.max(1, Math.ceil(days / bucketCount))
    const bucketMs = bucketDays * DAY_MS
    const count = Math.ceil((now - allStart) / bucketMs)
    for (let i = 0; i < count; i++) {
      const s = allStart + i * bucketMs
      const e = s + bucketMs
      const l = labelMD(s)
      labels.push({ label: l, tipLabel: bucketDays > 1 ? l + '–' + labelMD(e - DAY_MS) : l })
    }
    index = ts => Math.floor((ts - allStart) / bucketMs)
  }

  return { labels, index }
}

export function UsageTab() {
  const [records, setRecords] = useState<UsageRecord[]>([])
  const [timing, setTiming] = useState<TimingRecord[]>([])
  const [view, setView] = useState<'token' | 'timing'>('token')
  const [range, setRange] = useState<RangeId>('today')
  const [scene, setScene] = useState<'all' | UsageScene>('all')
  const [tip, setTip] = useState<TipState | null>(null)
  const [confirming, setConfirming] = useState(false)
  const confirmTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const chartWrapRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    let cancelled = false
    const loadUsage = () => {
      invoke<UsageRecord[]>('get_usage_records')
        .then(r => { if (!cancelled) setRecords(r) })
        .catch(() => {})
    }
    const loadTiming = () => {
      invoke<TimingRecord[]>('get_timing_records')
        .then(r => { if (!cancelled) setTiming(r) })
        .catch(() => {})
    }
    loadUsage()
    loadTiming()

    let unlistenUsage: (() => void) | null = null
    let unlistenTiming: (() => void) | null = null
    onEvent<null>('usage-updated', loadUsage)
      .then(fn => { if (cancelled) fn(); else unlistenUsage = fn })
    onEvent<null>('timing-updated', loadTiming)
      .then(fn => { if (cancelled) fn(); else unlistenTiming = fn })

    return () => {
      cancelled = true
      unlistenUsage?.()
      unlistenTiming?.()
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
      const [u, t] = await Promise.all([
        invoke<UsageRecord[]>('get_usage_records'),
        invoke<TimingRecord[]>('get_timing_records'),
      ])
      setRecords(u)
      setTiming(t)
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

  // ===== Token 视图数据 =====
  const tokenView = useMemo(() => {
    const now = Date.now()
    const sorted = [...records].sort((a, b) => a.ts - b.ts)
    const b = periodBounds(range, sorted.map(r => r.ts), now)

    const inScene = (r: UsageRecord) => scene === 'all' || r.scene === scene
    const current = sorted.filter(r => r.ts >= b.curStart && inScene(r))

    const input = current.reduce((s, r) => s + r.inputTokens, 0)
    const output = current.reduce((s, r) => s + r.outputTokens, 0)
    const total = input + output
    const calls = current.length

    // 场景筛选按钮上的调用次数（当前时间范围、不限场景）
    const sceneCounts: Record<string, number> = { all: 0 }
    for (const r of sorted) {
      if (r.ts >= b.curStart) {
        sceneCounts.all++
        sceneCounts[r.scene] = (sceneCounts[r.scene] || 0) + 1
      }
    }

    // 按模型汇总
    const byModel = new Map<string, UsageModelAgg>()
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
      const midpoint = (b.allStart + now) / 2
      trendCur = 0
      for (const r of current) {
        const t = r.inputTokens + r.outputTokens
        if (r.ts < midpoint) trendPrev += t
        else trendCur += t
      }
    } else {
      trendPrev = sorted
        .filter(r => r.ts >= b.prevStart && r.ts < b.prevEnd && inScene(r))
        .reduce((s, r) => s + r.inputTokens + r.outputTokens, 0)
    }
    const trend = trendPrev > 0 ? Math.round(((trendCur - trendPrev) / trendPrev) * 100) : null

    // 图表分桶
    const { labels, index } = makeBuckets(range, b.days, b.allStart, now)
    const buckets: UsageBucket[] = labels.map(l => ({ ...l, scenes: {}, total: 0 }))
    for (const r of current) {
      const idx = index(r.ts)
      if (idx >= 0 && idx < buckets.length) {
        const t = r.inputTokens + r.outputTokens
        buckets[idx].scenes[r.scene] = (buckets[idx].scenes[r.scene] || 0) + t
        buckets[idx].total += t
      }
    }
    const maxBucket = Math.max(...buckets.map(x => x.total), 0)
    const labelStep = buckets.length > 20 ? Math.ceil(buckets.length / 8) : 1

    return {
      hasAny: sorted.length > 0,
      input, output, total, calls,
      sceneCounts, models, topModel, topShare,
      trend, buckets, maxBucket, labelStep,
      days: b.days,
    }
  }, [records, range, scene])

  // ===== 处理耗时视图数据 =====
  const timingView = useMemo(() => {
    const now = Date.now()
    const sorted = [...timing].sort((a, b) => a.ts - b.ts)
    const b = periodBounds(range, sorted.map(r => r.ts), now)
    const current = sorted.filter(r => r.ts >= b.curStart)

    const avg = (key: 'transcribeMs' | 'optimizeMs' | 'totalMs') =>
      current.length ? current.reduce((s, r) => s + r[key], 0) / current.length : 0
    const avgTotal = avg('totalMs')
    const avgTrans = avg('transcribeMs')
    const avgOpt = avg('optimizeMs')

    // 趋势：与上一周期的平均总耗时对比
    let trend: number | null = null
    if (range === 'all' && current.length > 0) {
      const midpoint = (b.allStart + now) / 2
      const prev = current.filter(r => r.ts < midpoint)
      const cur = current.filter(r => r.ts >= midpoint)
      const prevAvg = prev.length ? prev.reduce((s, r) => s + r.totalMs, 0) / prev.length : 0
      trend = prevAvg > 0 && cur.length > 0
        ? Math.round(((avgTotal - prevAvg) / prevAvg) * 100)
        : null
    } else if (current.length > 0) {
      const prev = sorted.filter(r => r.ts >= b.prevStart && r.ts < b.prevEnd)
      const prevAvg = prev.length ? prev.reduce((s, r) => s + r.totalMs, 0) / prev.length : 0
      trend = prevAvg > 0 ? Math.round(((avgTotal - prevAvg) / prevAvg) * 100) : null
    }

    // 按模型汇总：同一模型的转写、优化阶段分别归集
    const byModel = new Map<string, TimingModelAgg>()
    const touch = (model: string, provider: string) => {
      let agg = byModel.get(model)
      if (!agg) {
        agg = { model, provider, transCount: 0, transMs: 0, optCount: 0, optMs: 0, calls: 0, avgTrans: 0, avgOpt: 0 }
        byModel.set(model, agg)
      }
      return agg
    }
    for (const r of current) {
      const t = touch(r.transcribeModel, r.transcribeProvider)
      t.transCount++
      t.transMs += r.transcribeMs
      if (r.optimizeModel) {
        const o = touch(r.optimizeModel, r.optimizeProvider ?? '')
        o.optCount++
        o.optMs += r.optimizeMs
      }
    }
    const models = [...byModel.values()].map(m => ({
      ...m,
      calls: m.transCount + m.optCount,
      avgTrans: m.transCount ? m.transMs / m.transCount : 0,
      avgOpt: m.optCount ? m.optMs / m.optCount : 0,
    })).sort((a, b) => b.calls - a.calls)
    const maxCalls = Math.max(...models.map(m => m.calls), 1)

    // 图表分桶：每桶取平均
    const { labels, index } = makeBuckets(range, b.days, b.allStart, now)
    const bucketRecords: TimingRecord[][] = labels.map(() => [])
    for (const r of current) {
      const idx = index(r.ts)
      if (idx >= 0 && idx < bucketRecords.length) bucketRecords[idx].push(r)
    }
    const buckets: TimingBucket[] = labels.map((l, i) => {
      const items = bucketRecords[i]
      const n = items.length
      if (n === 0) return { ...l, count: 0, avgTrans: 0, avgOpt: 0, avgOther: 0, avgTotal: 0 }
      const sum = (key: 'transcribeMs' | 'optimizeMs' | 'otherMs' | 'totalMs') =>
        items.reduce((s, r) => s + r[key], 0)
      return {
        ...l,
        count: n,
        avgTrans: sum('transcribeMs') / n,
        avgOpt: sum('optimizeMs') / n,
        avgOther: sum('otherMs') / n,
        avgTotal: sum('totalMs') / n,
      }
    })
    const maxBucket = Math.max(...buckets.map(x => x.avgTrans + x.avgOpt), 0)
    const labelStep = buckets.length > 20 ? Math.ceil(buckets.length / 8) : 1

    return {
      hasAny: sorted.length > 0,
      avgTotal, avgTrans, avgOpt,
      calls: current.length, days: b.days, trend,
      models, maxCalls,
      buckets, maxBucket, labelStep,
    }
  }, [timing, range])

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

  const viewSeg = (
    <div className="view-seg">
      <button className={view === 'token' ? 'active' : ''} onClick={() => { setView('token'); setTip(null) }}>Token用量</button>
      <button className={view === 'timing' ? 'active' : ''} onClick={() => { setView('timing'); setTip(null) }}>处理耗时</button>
    </div>
  )

  const clearPanel = (
    <div className="usage-panel">
      <div className="usage-actions" style={{ justifyContent: 'flex-end' }}>
        <button
          className={`usage-clear-btn${confirming ? ' confirming' : ''}`}
          onClick={handleClear}
        >
          {confirming ? '确认清空？' : '清空记录'}
        </button>
      </div>
    </div>
  )

  // ===== Token 视图 =====
  if (view === 'token') {
    if (!tokenView.hasAny) {
      return (
        <div>
          <h2 className="content-title">用量统计</h2>
          {viewSeg}
          {rangeTabs}
          <div className="usage-empty">暂无用量记录，使用语音转写、图像识别或自动学习后自动累积</div>
        </div>
      )
    }

    const trendClass = tokenView.trend === null ? '' : tokenView.trend >= 0 ? 'usage-trend-up' : 'usage-trend-down'
    const rangeLabel = RANGES.find(r => r.id === range)?.label ?? ''
    const trendBase = RANGES.find(r => r.id === range)?.trendBase ?? ''

    return (
      <div onMouseLeave={() => setTip(null)}>
        <h2 className="content-title">用量统计</h2>
        {viewSeg}

        <div className="usage-toolbar">
          {rangeTabs}
          <div className="scene-chips">
            <button
              className={`scene-chip${scene === 'all' ? ' active' : ''}`}
              onClick={() => { setScene('all'); setTip(null) }}
            >
              全部 <span className="cnt">{fmtNum(tokenView.sceneCounts.all || 0)}次</span>
            </button>
            {SCENES.map(s => (
              <button
                key={s.id}
                className={`scene-chip${scene === s.id ? ' active' : ''}`}
                onClick={() => { setScene(s.id); setTip(null) }}
              >
                <span className="chip-dot" style={{ background: s.color }} />
                {s.label}
                <span className="cnt">{fmtNum(tokenView.sceneCounts[s.id] || 0)}</span>
              </button>
            ))}
          </div>
        </div>

        <div className="usage-cards">
          <div className="usage-stat-card">
            <div className="usage-stat-label">总消耗Token · {rangeLabel}</div>
            <div className="usage-stat-value num">{fmtTok(tokenView.total)}</div>
            <div className="usage-stat-sub num">输入 {fmtTok(tokenView.input)} · 输出 {fmtTok(tokenView.output)}</div>
          </div>
          <div className="usage-stat-card">
            <div className="usage-stat-label">API调用</div>
            <div className="usage-stat-value num">{fmtNum(tokenView.calls)}<span style={{ fontSize: 11, fontWeight: 400 }}> 次</span></div>
            <div className="usage-stat-sub num">日均 {fmtNum(Math.round(tokenView.calls / tokenView.days))} 次</div>
          </div>
          <div className="usage-stat-card">
            <div className="usage-stat-label">最常用模型</div>
            <div className="usage-stat-value small" title={tokenView.topModel?.model}>{tokenView.topModel ? tokenView.topModel.model : '—'}</div>
            <div className="usage-stat-sub num">{tokenView.topModel ? `占${tokenView.topShare}% · ${fmtTok(tokenView.topModel.total)}` : '暂无调用'}</div>
          </div>
          <div className="usage-stat-card">
            <div className="usage-stat-label">消耗趋势</div>
            <div className={`usage-stat-value num ${trendClass}`}>
              {tokenView.trend === null ? '—' : `${tokenView.trend >= 0 ? '+' : ''}${tokenView.trend}%`}
            </div>
            <div className="usage-stat-sub">
              {tokenView.trend === null ? '上期无记录' : `${trendBase}${tokenView.trend >= 0 ? '增加' : '减少'}`}
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
              {tokenView.buckets.map((b, i) => (
                <div
                  key={i}
                  className={`usage-bar-col${b.total === 0 ? ' empty' : ''}`}
                  onMouseEnter={e => handleBarEnter(e, i)}
                >
                  {b.total > 0 && tokenView.maxBucket > 0 && SCENES.map(s => {
                    const v = b.scenes[s.id]
                    if (!v) return null
                    return (
                      <div
                        key={s.id}
                        className="usage-bar-seg"
                        style={{ height: `${(v / tokenView.maxBucket) * 100}%`, background: s.color }}
                      />
                    )
                  })}
                </div>
              ))}
            </div>
            <div className="usage-x-axis">
              {tokenView.buckets.map((b, i) => (
                <div key={i} className="usage-x-label">
                  {i % tokenView.labelStep === 0 || i === tokenView.buckets.length - 1 ? b.label : ''}
                </div>
              ))}
            </div>
            {tip && tokenView.buckets[tip.index] && (() => {
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
                  <div className="tt-date">{tokenView.buckets[tip.index].tipLabel}</div>
                  {SCENES.filter(s => tokenView.buckets[tip.index].scenes[s.id]).map(s => (
                    <div key={s.id} className="tt-row">
                      <span className="tt-scene">
                        <span className="chip-dot" style={{ background: s.color }} />
                        {s.label}
                      </span>
                      <b>{fmtNum(tokenView.buckets[tip.index].scenes[s.id] || 0)}</b>
                    </div>
                  ))}
                  <div className="tt-row tt-total">
                    <span>合计</span>
                    <b>{fmtNum(tokenView.buckets[tip.index].total)}</b>
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
                {tokenView.models.length === 0 ? (
                  <tr><td colSpan={6} className="usage-table-empty">当前筛选条件下暂无调用</td></tr>
                ) : tokenView.models.map(m => {
                  const share = tokenView.total > 0
                    ? Math.round((m.total / tokenView.total) * 100)
                    : Math.round((m.calls / Math.max(tokenView.calls, 1)) * 100)
                  const barWidth = tokenView.models[0].total > 0
                    ? (m.total / tokenView.models[0].total) * 100
                    : (m.calls / Math.max(tokenView.models[0].calls, 1)) * 100
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

        {clearPanel}
      </div>
    )
  }

  // ===== 处理耗时视图 =====
  if (!timingView.hasAny) {
    return (
      <div>
        <h2 className="content-title">用量统计</h2>
        {viewSeg}
        {rangeTabs}
        <div className="usage-empty">暂无处理耗时记录，完成一次语音录音后自动累积</div>
      </div>
    )
  }

  const rangeLabel = RANGES.find(r => r.id === range)?.label ?? ''
  const trendBase = RANGES.find(r => r.id === range)?.trendBase ?? ''
  const trendClassT = timingView.trend === null ? '' : timingView.trend >= 0 ? 'usage-trend-up' : 'usage-trend-down'
  const shareT = timingView.avgTotal > 0 ? Math.round(timingView.avgTrans / timingView.avgTotal * 100) : 0
  const shareO = timingView.avgTotal > 0 ? Math.round(timingView.avgOpt / timingView.avgTotal * 100) : 0

  return (
    <div onMouseLeave={() => setTip(null)}>
      <h2 className="content-title">用量统计</h2>
      {viewSeg}

      <div className="usage-toolbar">
        {rangeTabs}
      </div>

      <div className="usage-cards">
        <div className="usage-stat-card">
          <div className="usage-stat-label">平均处理耗时 · {rangeLabel}</div>
          <div className="usage-stat-value num">{fmtMs(timingView.avgTotal)}</div>
          <div className="usage-stat-sub">
            {timingView.trend === null
              ? '上期无记录'
              : <>{trendBase}{timingView.trend >= 0 ? '变慢' : '变快'} · <span className={trendClassT}>{timingView.trend >= 0 ? '+' : ''}{timingView.trend}%</span></>}
          </div>
        </div>
        <div className="usage-stat-card">
          <div className="usage-stat-label">音频转写平均</div>
          <div className="usage-stat-value num">{fmtMs(timingView.avgTrans)}</div>
          <div className="usage-stat-sub num">占总耗时{shareT}%</div>
        </div>
        <div className="usage-stat-card">
          <div className="usage-stat-label">文本优化平均</div>
          <div className="usage-stat-value num">{fmtMs(timingView.avgOpt)}</div>
          <div className="usage-stat-sub num">占总耗时{shareO}%</div>
        </div>
        <div className="usage-stat-card">
          <div className="usage-stat-label">录音次数</div>
          <div className="usage-stat-value num">{fmtNum(timingView.calls)}<span style={{ fontSize: 11, fontWeight: 400 }}> 次</span></div>
          <div className="usage-stat-sub num">日均 {fmtNum(Math.round(timingView.calls / timingView.days))} 次</div>
        </div>
      </div>

      <div className="usage-panel">
        <div className="usage-panel-head">
          <span className="usage-panel-title">
            处理耗时{range === 'today' ? '（按小时）' : range === 'all' ? '（按天及以上汇总）' : '（按天）'}
          </span>
          <div className="usage-legend">
            <span className="usage-legend-item">
              <span className="chip-dot" style={{ background: TIMING_COLORS.transcribe }} />
              音频转写
            </span>
            <span className="usage-legend-item">
              <span className="chip-dot" style={{ background: TIMING_COLORS.optimize }} />
              文本优化
            </span>
          </div>
        </div>
        <div className="usage-chart-wrap" ref={chartWrapRef}>
          <div className="usage-chart">
            {timingView.buckets.map((b, i) => (
              <div
                key={i}
                className={`usage-bar-col${b.count === 0 ? ' empty' : ''}`}
                onMouseEnter={e => handleBarEnter(e, i)}
              >
                {b.count > 0 && timingView.maxBucket > 0 && (
                  <>
                    <div
                      className="usage-bar-seg"
                      style={{ height: `${(b.avgOpt / timingView.maxBucket) * 100}%`, background: TIMING_COLORS.optimize }}
                    />
                    <div
                      className="usage-bar-seg"
                      style={{ height: `${(b.avgTrans / timingView.maxBucket) * 100}%`, background: TIMING_COLORS.transcribe }}
                    />
                  </>
                )}
              </div>
            ))}
          </div>
          <div className="usage-x-axis">
            {timingView.buckets.map((b, i) => (
              <div key={i} className="usage-x-label">
                {i % timingView.labelStep === 0 || i === timingView.buckets.length - 1 ? b.label : ''}
              </div>
            ))}
          </div>
          {tip && timingView.buckets[tip.index] && (() => {
            const b = timingView.buckets[tip.index]
            const flipBelow = tip.y < 110
            return (
              <div
                className={`usage-tooltip${flipBelow ? ' below' : ''}`}
                style={{
                  display: 'block',
                  left: Math.max(85, Math.min(tip.x, tip.w - 85)),
                  top: flipBelow ? tip.y + tip.h + 8 : tip.y - 8,
                }}
              >
                <div className="tt-date">{b.tipLabel}{b.count > 0 ? ` · ${b.count}次` : ''}</div>
                {b.count > 0 ? (
                  <>
                    <div className="tt-row">
                      <span className="tt-scene">
                        <span className="chip-dot" style={{ background: TIMING_COLORS.transcribe }} />
                        音频转写
                      </span>
                      <b>{fmtMs(b.avgTrans)}</b>
                    </div>
                    {b.avgOpt > 0 && (
                      <div className="tt-row">
                        <span className="tt-scene">
                          <span className="chip-dot" style={{ background: TIMING_COLORS.optimize }} />
                          文本优化
                        </span>
                        <b>{fmtMs(b.avgOpt)}</b>
                      </div>
                    )}
                    <div className="tt-row">
                      <span>其他</span>
                      <b>{fmtMs(b.avgOther)}</b>
                    </div>
                    <div className="tt-row tt-total">
                      <span>平均总耗时</span>
                      <b>{fmtMs(b.avgTotal)}</b>
                    </div>
                  </>
                ) : (
                  <div className="tt-row"><span>无录音</span></div>
                )}
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
                <th>音频转写平均</th>
                <th>文本优化平均</th>
                <th>调用次数</th>
                <th>占比</th>
              </tr>
            </thead>
            <tbody>
              {timingView.models.length === 0 ? (
                <tr><td colSpan={5} className="usage-table-empty">当前时间范围内暂无调用</td></tr>
              ) : timingView.models.map(m => (
                <tr key={m.model + '|' + m.provider}>
                  <td>
                    <div className="usage-model-name" title={m.model}>{m.model}</div>
                    <div className="usage-model-provider">{m.provider}</div>
                  </td>
                  <td>
                    {m.transCount > 0 ? (
                      <>
                        <div className="num">{fmtMs(m.avgTrans)}</div>
                        <div className="usage-stage-sub num">{fmtNum(m.transCount)}次</div>
                      </>
                    ) : (
                      <span className="usage-stage-none">未参与</span>
                    )}
                  </td>
                  <td>
                    {m.optCount > 0 ? (
                      <>
                        <div className="num">{fmtMs(m.avgOpt)}</div>
                        <div className="usage-stage-sub num">{fmtNum(m.optCount)}次</div>
                      </>
                    ) : (
                      <span className="usage-stage-none">未参与</span>
                    )}
                  </td>
                  <td className="num"><b>{fmtNum(m.calls)}</b></td>
                  <td className="usage-share-cell">
                    <div className="usage-share-bar">
                      <div className="usage-share-fill" style={{ width: `${(m.calls / timingView.maxCalls) * 100}%` }} />
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {clearPanel}
    </div>
  )
}
