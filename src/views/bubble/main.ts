import { getCurrentWindow } from '@tauri-apps/api/window'

const bubble = document.getElementById('bubble')!
const currentWindow = getCurrentWindow()

const shapeMap: Record<string, () => string> = {
  recording: () =>
    `<div class="s-recording">●</div>`,
  transcribing: () =>
    `<div class="s-transcribing">Thinking...</div>`,
  optimizing: () =>
    `<div class="s-optimizing">Thinking...</div>`,
  retrying: () =>
    `<div class="s-retrying">Thinking...</div>`,
  completed: () =>
    `<div class="s-completed"><span class="check">\u2713</span></div>`,
  failed: () =>
    `<div class="s-failed"><span class="x">\u2715</span></div>`,
}

function render(status: string) {
  if (shapeMap[status]) {
    bubble.innerHTML = shapeMap[status]()
  }
}

// Window-scoped listeners — each bubble only receives events targeted to it
currentWindow.listen<{ taskNumber: number; status: string }>('show-bubble', (event) => {
  const { status } = event.payload
  render(status)
})

currentWindow.listen<{ taskNumber: number; status: string }>('update-bubble', (event) => {
  const { status } = event.payload
  render(status)
})
