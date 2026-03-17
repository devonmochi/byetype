import React from 'react'
import ReactDOM from 'react-dom/client'
import { App } from './views/settings/App'
import './views/settings/index.css'
import { TaskManager } from './core/task-manager'
import { HistoryManager } from './core/history'
import { listen } from '@tauri-apps/api/event'
import { getConfig } from './lib/tauri-api'

let taskManager: TaskManager | null = null
let currentTaskId: number | null = null

async function initApp() {
  const historyManager = new HistoryManager()
  await historyManager.init()
  taskManager = new TaskManager(historyManager)

  listen('recording-started', async () => {
    if (!taskManager) return
    const config = await getConfig()
    if (taskManager.canCreateTask(config)) {
      currentTaskId = taskManager.createTask()
      console.log(`[Main] Task created: #${currentTaskId}`)
    }
  })

  listen<{ audio: string }>('recording-complete', (event) => {
    if (!taskManager || currentTaskId === null) return
    const taskId = currentTaskId
    currentTaskId = null
    console.log(`[Main] Processing recording for task #${taskId}`)
    taskManager.processRecording(taskId, event.payload.audio)
  })

  listen<{ message: string }>('recording-error', (event) => {
    console.error(`[Main] Recording error:`, event.payload.message)
    currentTaskId = null
  })
}

initApp().catch(console.error)

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
