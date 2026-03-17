import { emit } from '@tauri-apps/api/event'
import { transcribeAudio } from './transcribe'
import { optimizeText } from './optimize'
import { resolvePromptPath } from './prompt-loader'
import type { HistoryManager } from './history'
import type { AppConfig, TaskStatus } from './types'
import {
  showBubble,
  updateBubble,
  hideBubble,
  pasteText,
  getConfig,
  getBuiltinPromptPath,
} from '../lib/tauri-api'

interface Task {
  id: number
  status: TaskStatus
}

export class TaskManager {
  private taskCounter = 0
  private activeTasks: Map<number, Task> = new Map()
  private historyManager: HistoryManager

  constructor(historyManager: HistoryManager) {
    this.historyManager = historyManager
  }

  canCreateTask(config: AppConfig): boolean {
    return this.activeTasks.size < config.advanced.maxParallel
  }

  createTask(): number {
    if (this.activeTasks.size === 0) {
      this.taskCounter = 0
    }
    this.taskCounter++
    const task: Task = { id: this.taskCounter, status: 'recording' }
    this.activeTasks.set(task.id, task)
    showBubble(task.id).catch(console.error)
    return task.id
  }

  private async getResolvedPromptPaths(config: AppConfig) {
    const [agentBuiltin, rulesBuiltin, vocabularyBuiltin, optimizeBuiltin] = await Promise.all([
      getBuiltinPromptPath('agent.md'),
      getBuiltinPromptPath('rules.md'),
      getBuiltinPromptPath('vocabulary.md'),
      getBuiltinPromptPath('text-optimize.md'),
    ])
    return {
      transcribe: {
        agent: resolvePromptPath(config.transcribe.prompts.agent, agentBuiltin),
        rules: resolvePromptPath(config.transcribe.prompts.rules, rulesBuiltin),
        vocabulary: resolvePromptPath(config.transcribe.prompts.vocabulary, vocabularyBuiltin),
      },
      optimize: resolvePromptPath(config.optimize.prompt, optimizeBuiltin),
    }
  }

  async processRecording(taskId: number, audioBase64: string): Promise<void> {
    const task = this.activeTasks.get(taskId)
    if (!task) return

    const config = await getConfig()
    const paths = await this.getResolvedPromptPaths(config)

    let transcribeText: string | null = null
    let optimizeResult: string | null = null
    let errorMessage: string | undefined

    try {
      this.updateTask(taskId, 'transcribing')
      console.log(`[Task#${taskId}] 开始转写`)

      transcribeText = await transcribeAudio(audioBase64, config, paths.transcribe, {
        onRetry: (attempt) => {
          console.log(`[Task#${taskId}] 重试第 ${attempt} 次`)
          this.updateTask(taskId, 'retrying')
        },
        onTimeout: () => this.updateTask(taskId, 'retrying')
      })
      console.log(`[Task#${taskId}] 转写完成: ${transcribeText}`)

      let finalText = transcribeText
      if (config.optimize.enabled) {
        this.updateTask(taskId, 'optimizing')
        optimizeResult = await optimizeText(transcribeText, config, paths.optimize, {
          onRetry: (attempt) => {
            console.log(`[Task#${taskId}] 优化重试第 ${attempt} 次`)
            this.updateTask(taskId, 'retrying')
          },
          onTimeout: () => this.updateTask(taskId, 'retrying')
        })
        finalText = optimizeResult
        console.log(`[Task#${taskId}] 优化完成: ${finalText}`)
      }

      await pasteText(finalText)
      this.updateTask(taskId, 'completed')
      hideBubble(taskId, 500).catch(console.error)

      await this.historyManager.addRecord({
        audioBase64,
        transcribeText,
        optimizeText: optimizeResult,
        status: 'completed'
      })
    } catch (err) {
      console.error(`[Task#${taskId}] 失败:`, err)
      errorMessage = err instanceof Error ? err.message : String(err)
      this.updateTask(taskId, 'failed')
      hideBubble(taskId, 3000).catch(console.error)

      await this.historyManager.addRecord({
        audioBase64,
        transcribeText,
        optimizeText: optimizeResult,
        status: 'failed',
        errorMessage
      })
    } finally {
      this.activeTasks.delete(taskId)
    }
  }

  async retryFromHistory(recordId: number): Promise<{ accepted: boolean; reason?: string }> {
    const config = await getConfig()
    if (!this.canCreateTask(config)) {
      return { accepted: false, reason: 'parallel_limit' }
    }

    const audioBase64 = await this.historyManager.getAudioBase64(recordId)
    if (!audioBase64) {
      return { accepted: false, reason: 'audio_missing' }
    }

    const taskId = this.createTask()
    this._doRetry(taskId, recordId, audioBase64).catch(console.error)
    return { accepted: true }
  }

  private async _doRetry(taskId: number, recordId: number, audioBase64: string): Promise<void> {
    const config = await getConfig()
    const paths = await this.getResolvedPromptPaths(config)

    try {
      this.updateTask(taskId, 'transcribing')
      emit('retry-status', { recordId, stage: 'transcribing' })

      const text = await transcribeAudio(audioBase64, config, paths.transcribe, {
        onRetry: () => {
          this.updateTask(taskId, 'retrying')
          emit('retry-status', { recordId, stage: 'retrying' })
        },
        onTimeout: () => {
          this.updateTask(taskId, 'retrying')
          emit('retry-status', { recordId, stage: 'retrying' })
        }
      })

      let optimizeResult: string | null = null
      let finalText = text
      if (config.optimize.enabled) {
        this.updateTask(taskId, 'optimizing')
        emit('retry-status', { recordId, stage: 'optimizing' })
        optimizeResult = await optimizeText(text, config, paths.optimize)
        finalText = optimizeResult
      }

      await pasteText(finalText)
      this.updateTask(taskId, 'completed')
      hideBubble(taskId, 500).catch(console.error)

      await this.historyManager.updateRecord(recordId, {
        transcribeText: text,
        optimizeText: optimizeResult,
        status: 'completed',
        errorMessage: undefined
      })
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      this.updateTask(taskId, 'failed')
      hideBubble(taskId, 3000).catch(console.error)

      await this.historyManager.updateRecord(recordId, {
        errorMessage,
        status: 'failed'
      })
    } finally {
      this.activeTasks.delete(taskId)
    }
  }

  private updateTask(taskId: number, status: TaskStatus): void {
    const task = this.activeTasks.get(taskId)
    if (task) {
      task.status = status
      updateBubble(taskId, status).catch(console.error)
    }
  }
}
