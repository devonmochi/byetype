import { invoke } from '@tauri-apps/api/core'
const bg = document.getElementById('screenshot-bg') as HTMLImageElement
const overlay = document.getElementById('overlay')!
const selection = document.getElementById('selection')!
const sizeLabel = document.getElementById('size-label')!

let startX = 0
let startY = 0
let dragging = false
let armed = false

function log(msg: string) {
  invoke('js_debug_log', { msg }).catch(() => {})
}

async function init() {
  log('screenshot/main.ts init started')
  const base64: string | null = await invoke('get_screenshot_image')
  if (!base64) {
    log('get_screenshot_image returned null, cancelling')
    await invoke('submit_screenshot_crop', { crop: null })
    return
  }
  log(`get_screenshot_image returned base64, len=${base64.length}`)
  bg.src = `data:image/png;base64,${base64}`
  bg.onload = () => {
    log('bg image loaded, arming in 300ms')
    overlay.style.display = 'none'
    setTimeout(() => {
      armed = true
      log('armed = true, ready for pointer events')
    }, 300)
  }
  bg.onerror = () => {
    log('bg image FAILED to load')
    invoke('submit_screenshot_crop', { crop: null })
  }
}

// Use Pointer Events + setPointerCapture to guarantee pointerup delivery
document.addEventListener('pointerdown', (e: PointerEvent) => {
  if (!armed || e.button !== 0) {
    if (!armed) log(`pointerdown ignored: armed=${armed}`)
    return
  }
  // Capture pointer to guarantee pointerup is delivered to this element
  ;(e.target as Element).setPointerCapture(e.pointerId)
  log(`pointerdown at (${e.clientX}, ${e.clientY}), captured pointerId=${e.pointerId}`)
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
  if (!armed) {
    log(`pointerup ignored: armed=${armed}`)
    return
  }
  if (!dragging) {
    log('pointerup ignored: not dragging')
    return
  }
  dragging = false
  log(`pointerup at (${e.clientX}, ${e.clientY})`)

  const x = Math.min(e.clientX, startX)
  const y = Math.min(e.clientY, startY)
  const w = Math.abs(e.clientX - startX)
  const h = Math.abs(e.clientY - startY)

  if (w < 5 || h < 5) {
    log(`selection too small (${w}x${h}), cancelling`)
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
  log(`submitting crop: x=${crop.x} y=${crop.y} w=${crop.w} h=${crop.h}`)
  await invoke('submit_screenshot_crop', { crop })
})

// Also handle lostpointercapture as a safety net
document.addEventListener('lostpointercapture', () => {
  if (dragging) {
    log('lostpointercapture while dragging — pointer capture was lost')
  }
})

document.addEventListener('keydown', async (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    log('ESC pressed, cancelling')
    await invoke('submit_screenshot_crop', { crop: null })
  }
})

init()
