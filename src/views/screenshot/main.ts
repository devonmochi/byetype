import { invoke } from '@tauri-apps/api/core'
const bg = document.getElementById('screenshot-bg') as HTMLImageElement
const overlay = document.getElementById('overlay')!
const selection = document.getElementById('selection')!
const sizeLabel = document.getElementById('size-label')!

let startX = 0
let startY = 0
let dragging = false

async function init() {
  // Fetch the full screenshot from Rust
  const base64: string | null = await invoke('get_screenshot_image')
  if (!base64) {
    await invoke('submit_screenshot_crop', { crop: null })
    return
  }
  bg.src = `data:image/png;base64,${base64}`
  // Hide the dark overlay once image loads — selection box-shadow provides dimming
  bg.onload = () => {
    overlay.style.display = 'none'
  }
}

document.addEventListener('mousedown', (e: MouseEvent) => {
  if (e.button !== 0) return
  dragging = true
  startX = e.clientX
  startY = e.clientY
  selection.style.left = `${startX}px`
  selection.style.top = `${startY}px`
  selection.style.width = '0px'
  selection.style.height = '0px'
  selection.style.display = 'block'
  overlay.style.display = 'none' // hide flat overlay, selection box-shadow takes over
})

document.addEventListener('mousemove', (e: MouseEvent) => {
  if (!dragging) return
  const x = Math.min(e.clientX, startX)
  const y = Math.min(e.clientY, startY)
  const w = Math.abs(e.clientX - startX)
  const h = Math.abs(e.clientY - startY)
  selection.style.left = `${x}px`
  selection.style.top = `${y}px`
  selection.style.width = `${w}px`
  selection.style.height = `${h}px`

  // Show size label near the selection
  const dpr = window.devicePixelRatio || 1
  sizeLabel.textContent = `${Math.round(w * dpr)} x ${Math.round(h * dpr)}`
  sizeLabel.style.left = `${x}px`
  sizeLabel.style.top = `${y + h + 4}px`
  sizeLabel.style.display = 'block'
})

document.addEventListener('mouseup', async (e: MouseEvent) => {
  if (!dragging) return
  dragging = false

  const x = Math.min(e.clientX, startX)
  const y = Math.min(e.clientY, startY)
  const w = Math.abs(e.clientX - startX)
  const h = Math.abs(e.clientY - startY)

  if (w < 5 || h < 5) {
    // Too small — treat as cancel
    await invoke('submit_screenshot_crop', { crop: null })
    return
  }

  // Scale logical coords to physical pixels
  const dpr = window.devicePixelRatio || 1
  await invoke('submit_screenshot_crop', {
    crop: {
      x: Math.round(x * dpr),
      y: Math.round(y * dpr),
      w: Math.round(w * dpr),
      h: Math.round(h * dpr),
    }
  })
})

// ESC to cancel
document.addEventListener('keydown', async (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    await invoke('submit_screenshot_crop', { crop: null })
  }
})

init()
