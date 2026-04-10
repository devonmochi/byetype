import { invoke } from '@tauri-apps/api/core'
const bg = document.getElementById('screenshot-bg') as HTMLImageElement
const overlay = document.getElementById('overlay')!
const selection = document.getElementById('selection')!
const sizeLabel = document.getElementById('size-label')!

let startX = 0
let startY = 0
let dragging = false
let armed = false

async function init() {
  const base64: string | null = await invoke('get_screenshot_image')
  if (!base64) {
    await invoke('submit_screenshot_crop', { crop: null })
    return
  }
  bg.src = `data:image/png;base64,${base64}`
  bg.onload = () => {
    overlay.style.display = 'none'
    setTimeout(() => {
      armed = true
    }, 300)
  }
  bg.onerror = () => {
    invoke('submit_screenshot_crop', { crop: null })
  }
}

// Use Pointer Events + setPointerCapture to guarantee pointerup delivery
document.addEventListener('pointerdown', (e: PointerEvent) => {
  if (!armed || e.button !== 0) return
  // Capture pointer to guarantee pointerup is delivered to this element
  ;(e.target as Element).setPointerCapture(e.pointerId)
  dragging = true
  startX = e.clientX
  startY = e.clientY
  selection.style.left = `${startX}px`
  selection.style.top = `${startY}px`
  selection.style.width = '0px'
  selection.style.height = '0px'
  selection.style.display = 'block'
  overlay.style.display = 'none'
})

document.addEventListener('pointermove', (e: PointerEvent) => {
  if (!dragging) return
  const x = Math.min(e.clientX, startX)
  const y = Math.min(e.clientY, startY)
  const w = Math.abs(e.clientX - startX)
  const h = Math.abs(e.clientY - startY)
  selection.style.left = `${x}px`
  selection.style.top = `${y}px`
  selection.style.width = `${w}px`
  selection.style.height = `${h}px`

  const dpr = window.devicePixelRatio || 1
  sizeLabel.textContent = `${Math.round(w * dpr)} x ${Math.round(h * dpr)}`
  sizeLabel.style.left = `${x}px`
  sizeLabel.style.top = `${y + h + 4}px`
  sizeLabel.style.display = 'block'
})

document.addEventListener('pointerup', async (e: PointerEvent) => {
  if (!armed) return
  if (!dragging) return
  dragging = false

  const x = Math.min(e.clientX, startX)
  const y = Math.min(e.clientY, startY)
  const w = Math.abs(e.clientX - startX)
  const h = Math.abs(e.clientY - startY)

  if (w < 5 || h < 5) {
    await invoke('submit_screenshot_crop', { crop: null })
    return
  }

  const dpr = window.devicePixelRatio || 1
  const crop = {
    x: Math.round(x * dpr),
    y: Math.round(y * dpr),
    w: Math.round(w * dpr),
    h: Math.round(h * dpr),
  }
  await invoke('submit_screenshot_crop', { crop })
})

// Also handle lostpointercapture as a safety net
document.addEventListener('keydown', async (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    await invoke('submit_screenshot_crop', { crop: null })
  }
})

init()
