import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'

const bubble = document.getElementById('bubble')!
const currentWindow = getCurrentWindow()
let currentTaskId: number = 0

const shapeMap: Record<string, () => string> = {
  recording: () =>
    `<div class="s-recording">●</div>`,
  transcribing: () =>
    `<div class="s-transcribing">Thinking...<span class="cancel-btn">\u2715</span></div>`,
  optimizing: () =>
    `<div class="s-optimizing">Thinking...<span class="cancel-btn">\u2715</span></div>`,
  extracting: () =>
    `<div class="s-extracting">Extracting...<span class="cancel-btn">\u2715</span></div>`,
  retrying: () =>
    `<div class="s-retrying">Thinking...<span class="cancel-btn">\u2715</span></div>`,
  completed: () =>
    `<div class="s-completed"><span class="check">\u2713</span></div>`,
  failed: () =>
    `<div class="s-failed"><span class="x">\u2715</span></div>`,
}

function render(status: string) {
  if (shapeMap[status]) {
    bubble.innerHTML = shapeMap[status]()
    const cancelBtn = bubble.querySelector('.cancel-btn')
    if (cancelBtn) {
      cancelBtn.addEventListener('mousedown', (e) => {
        e.preventDefault()
        e.stopPropagation()
        if (currentTaskId > 0) {
          invoke('cancel_task', { taskId: currentTaskId })
        }
      })
    }
  }
}

// Window-scoped listeners — each bubble only receives events targeted to it
currentWindow.listen('clear-bubble', () => {
  bubble.innerHTML = ''
})

currentWindow.listen<{ taskNumber: number; status: string }>('show-bubble', (event) => {
  const { taskNumber, status } = event.payload
  currentTaskId = taskNumber
  render(status)
})

currentWindow.listen<{ taskNumber: number; status: string }>('update-bubble', (event) => {
  const { taskNumber, status } = event.payload
  currentTaskId = taskNumber
  render(status)
})
