import React, { useState, useEffect, useCallback } from 'react'
import { HistoryRecord, RetryStatusUpdate } from '../../../core/types'
import { onEvent } from '../../../lib/tauri-api'
import { invoke } from '@tauri-apps/api/core'

function formatTime(isoString: string): string {
  const date = new Date(isoString)
  const now = new Date()
  const isToday = date.toDateString() === now.toDateString()
  const h = date.getHours().toString().padStart(2, '0')
  const m = date.getMinutes().toString().padStart(2, '0')
  const s = date.getSeconds().toString().padStart(2, '0')
  if (isToday) return `${h}:${m}:${s}`
  const month = (date.getMonth() + 1).toString().padStart(2, '0')
  const day = date.getDate().toString().padStart(2, '0')
  return `${month}/${day} ${h}:${m}:${s}`
}

type StageStatus = 'success' | 'error' | 'pending' | 'processing'

interface StageInfo {
  audio: StageStatus
  transcribe: { status: StageStatus; text: string }
  optimize: { status: StageStatus; text: string }
}

function getStageInfo(record: HistoryRecord, retryStage?: string): StageInfo {
  if (retryStage) {
    if (retryStage === 'transcribing' || retryStage === 'retrying') {
      return {
        audio: 'success',
        transcribe: { status: 'processing', text: retryStage === 'retrying' ? '重试中...' : '转写中...' },
        optimize: { status: 'pending', text: '等待中' }
      }
    }
    if (retryStage === 'optimizing') {
      return {
        audio: 'success',
        transcribe: { status: 'success', text: record.transcribeText || '' },
        optimize: { status: 'processing', text: '优化中...' }
      }
    }
  }

  if (record.status === 'cancelled') {
    return {
      audio: record.audioPath ? 'success' : 'pending',
      transcribe: record.transcribeText
        ? { status: 'success', text: record.transcribeText }
        : { status: 'pending', text: '已取消' },
      optimize: { status: 'pending', text: '已取消' }
    }
  }

  const audio: StageStatus = record.audioPath ? 'success' : (record.status === 'failed' ? 'error' : 'success')

  let transcribe: StageInfo['transcribe']
  if (record.transcribeText) {
    transcribe = { status: 'success', text: record.transcribeText }
  } else if (record.status === 'failed') {
    transcribe = { status: 'error', text: record.errorMessage?.slice(0, 30) || '失败' }
  } else {
    transcribe = { status: 'pending', text: '\u2014' }
  }

  let optimize: StageInfo['optimize']
  if (record.optimizeText) {
    optimize = { status: 'success', text: record.optimizeText }
  } else if (record.transcribeText && record.status === 'failed') {
    optimize = { status: 'error', text: record.errorMessage?.slice(0, 30) || '失败' }
  } else if (!record.transcribeText && record.status === 'failed') {
    optimize = { status: 'pending', text: '\u2014' }
  } else {
    optimize = { status: 'pending', text: '\u2014' }
  }

  return { audio, transcribe, optimize }
}

function StageIndicator({ status }: { status: StageStatus }) {
  const colors: Record<StageStatus, string> = {
    success: '#34c759',
    error: '#ff3b30',
    pending: 'var(--text-disabled)',
    processing: '#ff9500'
  }
  const filled = status !== 'pending'
  return (
    <span style={{
      color: colors[status],
      fontSize: 8,
      flexShrink: 0
    }}>
      {filled ? '\u25CF' : '\u25CB'}
    </span>
  )
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false)

  const handleCopy = useCallback((e: React.MouseEvent) => {
    e.stopPropagation()
    navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }, [text])

  return (
    <span
      className="history-copy-btn"
      onClick={handleCopy}
      title={copied ? '已复制' : '复制'}
    >
      {copied ? '\u2713' : '\uD83D\uDCCB'}
    </span>
  )
}

interface RecordRowProps {
  record: HistoryRecord
  retryStage?: string
  onRetry: (id: number) => void
}

function RecordRow({ record, retryStage, onRetry }: RecordRowProps) {
  const info = getStageInfo(record, retryStage)
  const isRetrying = !!retryStage
  const audioMissing = !record.audioPath

  return (
    <div className="history-row">
      <div className="history-row-top">
        <span className="history-time">{formatTime(record.createdAt)}</span>
        <div className="history-pipeline">
          <span className="history-stage-dot"><StageIndicator status={info.audio} /> 音频</span>
          <span className="history-arrow">{'\u2192'}</span>
          <span className="history-stage-dot"><StageIndicator status={info.transcribe.status} /> 转写</span>
          <span className="history-arrow">{'\u2192'}</span>
          <span className="history-stage-dot"><StageIndicator status={info.optimize.status} /> 优化</span>
        </div>
        <button
          className={`history-retry-btn${(record.status === 'failed' || record.status === 'cancelled') && !isRetrying && !audioMissing ? ' highlight' : ''}`}
          disabled={isRetrying || audioMissing}
          title={audioMissing ? '音频文件已丢失' : isRetrying ? '重试中' : '重试'}
          onClick={() => onRetry(record.id)}
        >
          {isRetrying ? '重试中' : '重试'}
        </button>
      </div>
      <div className="history-row-bottom">
        <div className="history-text-cell">
          <span className="history-text-label">转写</span>
          <span
            className="history-text-content"
            style={info.transcribe.status === 'error' ? { color: '#ff3b30' } : info.transcribe.status === 'processing' ? { color: '#ff9500' } : undefined}
            title={info.transcribe.text}
          >
            {info.transcribe.text}
          </span>
          {info.transcribe.status === 'success' && record.transcribeText && (
            <CopyButton text={record.transcribeText} />
          )}
        </div>
        <div className="history-text-cell">
          <span className="history-text-label">优化</span>
          <span
            className="history-text-content"
            style={info.optimize.status === 'error' ? { color: '#ff3b30' } : info.optimize.status === 'processing' ? { color: '#ff9500' } : undefined}
            title={info.optimize.text}
          >
            {info.optimize.text}
          </span>
          {info.optimize.status === 'success' && record.optimizeText && (
            <CopyButton text={record.optimizeText} />
          )}
        </div>
      </div>
    </div>
  )
}

export function HistoryTab() {
  const [records, setRecords] = useState<HistoryRecord[]>([])
  const [retryStatus, setRetryStatus] = useState<Map<number, string>>(new Map())

  useEffect(() => {
    invoke<HistoryRecord[]>('get_history').then(r => setRecords(r)).catch(() => {})

    let unlistenHistory: (() => void) | null = null
    let unlistenRetry: (() => void) | null = null

    onEvent<HistoryRecord[]>('history-updated', (newRecords) => {
      setRecords(newRecords)
      setRetryStatus(prev => {
        const next = new Map(prev)
        for (const id of prev.keys()) {
          const rec = newRecords.find(r => r.id === id)
          if (rec && (rec.status === 'completed' || rec.status === 'failed' || rec.status === 'cancelled')) {
            next.delete(id)
          }
        }
        return next.size !== prev.size ? next : prev
      })
    }).then(fn => { unlistenHistory = fn })

    onEvent<RetryStatusUpdate>('retry-status', (update) => {
      setRetryStatus(prev => {
        const next = new Map(prev)
        next.set(update.recordId, update.stage)
        return next
      })
    }).then(fn => { unlistenRetry = fn })

    return () => {
      unlistenHistory?.()
      unlistenRetry?.()
    }
  }, [])

  const handleRetry = useCallback(async (recordId: number) => {
    try {
      await invoke('retry_record', { recordId })
      setRetryStatus(prev => {
        const next = new Map(prev)
        next.set(recordId, 'transcribing')
        return next
      })
    } catch { /* ignore */ }
  }, [])

  return (
    <div>
      <h2 className="content-title">历史记录</h2>
      {records.length === 0 ? (
        <div className="history-empty">暂无录音记录</div>
      ) : (
        <div className="history-list">
          {[...records].reverse().map(record => (
            <RecordRow
              key={record.id}
              record={record}
              retryStage={retryStatus.get(record.id)}
              onRetry={handleRetry}
            />
          ))}
        </div>
      )}
    </div>
  )
}
