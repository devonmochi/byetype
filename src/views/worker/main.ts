import { TaskManager } from '../../core/task-manager'
import { HistoryManager } from '../../core/history'
import { listen } from '@tauri-apps/api/event'
import { getConfig } from '../../lib/tauri-api'

let taskManager: TaskManager | null = null
let currentTaskId: number | null = null

async function init() {
  const historyManager = new HistoryManager()
  await historyManager.init()
  taskManager = new TaskManager(historyManager)

  listen('recording-started', async () => {
    if (!taskManager) return
    const config = await getConfig()
    if (taskManager.canCreateTask(config)) {
      currentTaskId = taskManager.createTask()
      console.log(`[Worker] Task created: #${currentTaskId}`)
    }
  })

  listen<{ audio: string }>('recording-complete', (event) => {
    if (!taskManager || currentTaskId === null) return
    const taskId = currentTaskId
    currentTaskId = null
    console.log(`[Worker] Processing recording for task #${taskId}`)
    taskManager.processRecording(taskId, event.payload.audio)
  })

  listen<{ message: string }>('recording-error', (event) => {
    console.error(`[Worker] Recording error:`, event.payload.message)
    currentTaskId = null
  })

  console.log('[Worker] Pipeline ready')
}

init().catch(console.error)
