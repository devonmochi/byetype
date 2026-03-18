import { getCurrentWindow } from '@tauri-apps/api/window'

const bubble = document.getElementById('bubble')!
const currentWindow = getCurrentWindow()

const shapeMap: Record<string, (n: number) => string> = {
  recording: (n) =>
    `<div class="s-recording"><span class="num" style="transform:rotate(-12deg)">${n}</span></div>`,
  transcribing: (n) =>
    `<div class="s-transcribing"><span class="num">${n}</span></div>`,
  optimizing: (n) =>
    `<div class="s-optimizing"><span class="num">${n}</span></div>`,
  retrying: (n) =>
    `<div class="s-retrying"><span class="num" style="transform:rotate(-45deg)">${n}</span></div>`,
  completed: (_n) =>
    `<div class="s-completed"><span class="check">\u2713</span></div>`,
  failed: (_n) =>
    `<div class="s-failed"><span class="x">\u2715</span></div>`,
}

function render(status: string, taskNumber: number) {
  if (shapeMap[status]) {
    bubble.innerHTML = shapeMap[status](taskNumber)
  }
}

// Window-scoped listeners — each bubble only receives events targeted to it
currentWindow.listen<{ taskNumber: number; status: string }>('show-bubble', (event) => {
  const { taskNumber, status } = event.payload
  render(status, taskNumber)
})

currentWindow.listen<{ taskNumber: number; status: string }>('update-bubble', (event) => {
  const { taskNumber, status } = event.payload
  render(status, taskNumber)
})
