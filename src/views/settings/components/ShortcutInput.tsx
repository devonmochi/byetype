import React, { useCallback, useEffect, useRef, useState } from 'react'
import {
  beginShortcutCapture,
  cancelShortcutCapture,
  endShortcutCapture,
} from '../../../lib/tauri-api'

/**
 * Capture state machine (requirement §9):
 *   Idle -> RequestingCapture -> Capturing -> Committing -> Idle
 * The backend token is the authority: a stale/old capture can never end a new
 * one, and capture suppression lasts until the save flow finishes.
 */
type Phase = 'idle' | 'requesting' | 'capturing' | 'committing' | 'cancelling'

interface Props {
  /** Backend field key, e.g. "shortcut" / "extractShortcut". */
  field: string
  /** Human readable current value (already formatted for display). */
  display: string
  disabled?: boolean
  onCommit: (field: string, value: string) => Promise<void>
  onError: (message: string) => void
}

const MODIFIER_CODES = new Set([
  'ControlLeft',
  'ControlRight',
  'ShiftLeft',
  'ShiftRight',
  'AltLeft',
  'AltRight',
  'MetaLeft',
  'MetaRight',
  'OSLeft',
  'OSRight',
])

const MODIFIER_KEYS = new Set(['Control', 'Alt', 'Shift', 'Meta', 'AltGraph', 'CapsLock', 'NumLock'])

/** Map a physical key event to the canonical primary-key name. */
function mainKeyFromEvent(event: React.KeyboardEvent): string | null {
  const code = event.code
  if (code) {
    if (code.startsWith('Key') && code.length === 4) return code.slice(3).toUpperCase()
    if (code.startsWith('Digit') && code.length === 6) return code.slice(5)
    return code
  }
  const key = event.key
  if (!key) return null
  if (key === ' ') return 'Space'
  if (key === 'Esc') return 'Escape'
  if (key.length === 1) return key.toUpperCase()
  return key
}

function buildCombo(event: React.KeyboardEvent, includeAlt: boolean): string | null {
  const key = mainKeyFromEvent(event)
  if (!key) return null
  const parts: string[] = []
  if (event.ctrlKey) parts.push('Ctrl')
  if (includeAlt || event.altKey) parts.push('Alt')
  if (event.shiftKey) parts.push('Shift')
  if (event.metaKey) parts.push('Super')
  parts.push(key)
  return parts.join('+')
}

export function ShortcutInput({ field, display, disabled, onCommit, onError }: Props) {
  const [phase, setPhase] = useState<Phase>('idle')
  const phaseRef = useRef<Phase>('idle')
  const tokenRef = useRef<number | null>(null)
  const epochRef = useRef(0)
  const pendingRightAlt = useRef(false)
  const aliveRef = useRef(true)

  const setPhaseBoth = useCallback((next: Phase) => {
    phaseRef.current = next
    setPhase(next)
  }, [])

  const finish = useCallback(
    async (mode: 'end' | 'cancel') => {
      epochRef.current += 1
      const token = tokenRef.current
      tokenRef.current = null
      pendingRightAlt.current = false
      setPhaseBoth(mode === 'cancel' ? 'cancelling' : 'idle')
      if (token == null) {
        setPhaseBoth('idle')
        return
      }
      try {
        if (mode === 'end') await endShortcutCapture(field, token)
        else await cancelShortcutCapture(field, token)
      } catch {
        // The backend ignores stale/unknown tokens; nothing else to do here.
      } finally {
        setPhaseBoth('idle')
      }
    },
    [field, setPhaseBoth],
  )

  // Cancel the backend session when this field unmounts (route switch, refresh).
  useEffect(() => {
    aliveRef.current = true
    return () => {
      aliveRef.current = false
      const token = tokenRef.current
      tokenRef.current = null
      if (token != null) {
        cancelShortcutCapture(field, token).catch(() => {})
      }
    }
  }, [field])

  useEffect(() => {
    const onVisibilityChange = () => {
      if (
        document.hidden &&
        (phaseRef.current === 'capturing' || phaseRef.current === 'requesting')
      ) {
        void finish('cancel')
      }
    }
    document.addEventListener('visibilitychange', onVisibilityChange)
    return () => document.removeEventListener('visibilitychange', onVisibilityChange)
  }, [finish])

  const startCapture = useCallback(async () => {
    if (disabled) return
    if (phaseRef.current === 'capturing' || phaseRef.current === 'requesting') return
    const epoch = ++epochRef.current
    setPhaseBoth('requesting')
    try {
      const token = await beginShortcutCapture(field)
      if (!aliveRef.current || epoch !== epochRef.current) {
        // A newer focus/blur happened while we were waiting: discard this token.
        cancelShortcutCapture(field, token).catch(() => {})
        return
      }
      tokenRef.current = token
      pendingRightAlt.current = false
      setPhaseBoth('capturing')
    } catch {
      setPhaseBoth('idle')
    }
  }, [disabled, field, setPhaseBoth])

  const commit = useCallback(
    async (nextValue: string) => {
      if (phaseRef.current !== 'capturing') return
      setPhaseBoth('committing')
      try {
        await onCommit(field, nextValue)
        await finish('end')
      } catch (error: unknown) {
        const message =
          typeof error === 'string'
            ? error
            : (error as { message?: string } | null)?.message || '保存快捷键失败'
        onError(message)
        await finish('cancel')
      }
    },
    [field, finish, onCommit, onError, setPhaseBoth],
  )

  const handleKeyDown = (event: React.KeyboardEvent) => {
    if (phaseRef.current !== 'capturing') return

    // Tab must keep its normal focus-navigation meaning.
    if (event.key === 'Tab') {
      void finish('cancel')
      return
    }
    event.preventDefault()

    if (event.key === 'Escape') {
      void finish('cancel')
      return
    }
    if (event.repeat) return

    // Backspace primary key deletes the shortcut, with or without modifiers.
    if (event.code === 'Backspace' || event.key === 'Backspace') {
      pendingRightAlt.current = false
      void commit('')
      return
    }

    // Physical right Alt is tracked separately so "hold then release" saves the
    // bare AltRight key, while AltRight + another key becomes an Alt combo.
    if (event.code === 'AltRight') {
      pendingRightAlt.current = true
      return
    }

    if (MODIFIER_CODES.has(event.code) || MODIFIER_KEYS.has(event.key)) return

    const includeAlt = pendingRightAlt.current
    pendingRightAlt.current = false
    const combo = buildCombo(event, includeAlt)
    if (combo) void commit(combo)
  }

  const handleKeyUp = (event: React.KeyboardEvent) => {
    if (phaseRef.current !== 'capturing') return
    if (event.code === 'AltRight' && pendingRightAlt.current) {
      pendingRightAlt.current = false
      void commit('AltRight')
    }
  }

  const handleBlur = () => {
    if (phaseRef.current === 'capturing' || phaseRef.current === 'requesting') {
      void finish('cancel')
    }
  }

  const recording = phase === 'capturing' || phase === 'requesting'

  return (
    <input
      className={`kbd${recording ? ' recording' : ''}`}
      value={recording ? '' : display}
      placeholder={recording ? '按下快捷键…' : '未设置'}
      readOnly
      disabled={disabled}
      onFocus={startCapture}
      onBlur={handleBlur}
      onKeyDown={handleKeyDown}
      onKeyUp={handleKeyUp}
      style={{ width: 120, textAlign: 'center', cursor: 'pointer' }}
    />
  )
}
