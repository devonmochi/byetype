import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'

const bubble = document.getElementById('bubble')!
const currentWindow = getCurrentWindow()
let currentTaskId: number | null = null

type Look = {
  shape: 'is-round' | 'is-pill'
  color: string
  label?: string
  glyph?: string
  dot?: boolean
  cancel?: boolean
  preparing?: boolean
}

// 所有状态共用一个 DOM 元素，只换形状类和配色类。重建元素会重放
// bounceIn，看起来就是气泡在跳。
const looks: Record<string, Look> = {
  preparing:    { shape: 'is-round', color: 'c-recording', dot: true, preparing: true },
  recording:    { shape: 'is-round', color: 'c-recording', dot: true },
  transcribing: { shape: 'is-pill', color: 'c-thinking', label: 'Thinking...', cancel: true },
  extracting:   { shape: 'is-pill', color: 'c-thinking', label: 'Thinking...', cancel: true },
  optimizing:   { shape: 'is-pill', color: 'c-optimizing', label: 'Thinking...', cancel: true },
  retrying:     { shape: 'is-pill', color: 'c-retrying', label: 'Thinking...', cancel: true },
  learning:     { shape: 'is-pill', color: 'c-thinking', label: 'Learning...' },
  completed:    { shape: 'is-round', color: 'c-completed', glyph: '✓' },
  failed:       { shape: 'is-round', color: 'c-failed', glyph: '✕' },
}

function classFor(look: Look): string {
  return `shape ${look.shape} ${look.color}${look.preparing ? ' is-preparing' : ''}`
}

function setPart(el: HTMLElement, selector: string, visible: boolean, text?: string) {
  const node = el.querySelector(selector) as HTMLElement
  node.style.display = visible ? 'inline-flex' : 'none'
  if (text !== undefined) node.textContent = text
}

function applyLook(el: HTMLElement, look: Look) {
  el.className = classFor(look)
  setPart(el, '.dot', !!look.dot)
  setPart(el, '.label', !!look.label, look.label ?? '')
  setPart(el, '.glyph', !!look.glyph, look.glyph ?? '')
  setPart(el, '.cancel-btn', !!look.cancel)
}

function render(status: string) {
  const look = looks[status]
  if (!look) return

  const existing = bubble.querySelector('.shape') as HTMLElement | null
  if (existing) {
    applyLook(existing, look)
    return
  }

  // 首次创建：先在离屏节点上把类和内容设好，再挂到文档里。这样第一帧
  // 就是最终尺寸，不会从内容自然宽度过渡到 34px（那会看着像缺了半截）。
  const el = document.createElement('div')
  el.innerHTML =
    `<span class="dot"></span><span class="label"></span>` +
    `<span class="glyph"></span><span class="cancel-btn">✕</span>`
  applyLook(el, look)
  el.querySelector('.cancel-btn')!.addEventListener('mousedown', (e) => {
    e.preventDefault()
    e.stopPropagation()
    if (currentTaskId !== null) {
      invoke('cancel_task', { taskId: currentTaskId }).catch((err) => console.error('cancel_task failed:', err))
    }
  })
  bubble.appendChild(el)
}

// Window-scoped listeners — each bubble only receives events targeted to it
currentWindow.listen('clear-bubble', () => {
  bubble.innerHTML = ''
  currentTaskId = null
})

currentWindow.listen<{ taskNumber: number | null; status: string }>('show-bubble', (event) => {
  const { taskNumber, status } = event.payload
  currentTaskId = taskNumber
  render(status)
})

currentWindow.listen<{ taskNumber: number; status: string }>('update-bubble', (event) => {
  const { taskNumber, status } = event.payload
  currentTaskId = taskNumber
  render(status)
})
