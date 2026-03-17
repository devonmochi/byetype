import {
  readTextFile,
  writeTextFile,
  exists,
  mkdir,
  readDir,
  remove,
  writeFile,
  readFile,
} from '@tauri-apps/plugin-fs'
import { appDataDir } from '@tauri-apps/api/path'
import type { HistoryRecord } from './types'
import { base64ToUint8Array, uint8ArrayToBase64 } from './base64'

export interface AddRecordParams {
  audioBase64: string | null
  transcribeText: string | null
  optimizeText: string | null
  status: 'completed' | 'failed'
  errorMessage?: string
}

const MAX_RECORDS = 3

export class HistoryManager {
  private records: HistoryRecord[] = []
  private writeQueue: Promise<void> = Promise.resolve()
  private lastId = 0
  private historyDir = ''
  private audioDir = ''
  private jsonPath = ''
  private onChange?: (records: HistoryRecord[]) => void

  constructor(onChange?: (records: HistoryRecord[]) => void) {
    this.onChange = onChange
  }

  async init(): Promise<void> {
    const dataDir = await appDataDir()
    this.historyDir = `${dataDir}/history`
    this.audioDir = `${this.historyDir}/audio`
    this.jsonPath = `${this.historyDir}/history.json`

    if (!(await exists(this.historyDir))) await mkdir(this.historyDir, { recursive: true })
    if (!(await exists(this.audioDir))) await mkdir(this.audioDir, { recursive: true })

    try {
      if (await exists(this.jsonPath)) {
        const raw = await readTextFile(this.jsonPath)
        this.records = JSON.parse(raw) as HistoryRecord[]
      }
    } catch {
      this.records = []
    }

    for (const record of this.records) {
      if (record.audioPath && !(await exists(record.audioPath))) {
        record.audioPath = null
      }
    }

    try {
      const referencedPaths = new Set(this.records.map(r => r.audioPath).filter(Boolean))
      const entries = await readDir(this.audioDir)
      for (const entry of entries) {
        if (entry.name) {
          const filePath = `${this.audioDir}/${entry.name}`
          if (!referencedPaths.has(filePath)) {
            try { await remove(filePath) } catch { /* ignore */ }
          }
        }
      }
    } catch { /* ignore */ }

    await this.persist()

    if (this.records.length > 0) {
      this.lastId = Math.max(...this.records.map(r => r.id))
    }
  }

  private nextId(): number {
    const now = Date.now()
    this.lastId = now > this.lastId ? now : this.lastId + 1
    return this.lastId
  }

  async addRecord(params: AddRecordParams): Promise<void> {
    this.writeQueue = this.writeQueue
      .then(() => this._addAndPersist(params))
      .catch(err => console.error('[History] Write failed:', err))
    return this.writeQueue
  }

  getRecords(): HistoryRecord[] {
    return [...this.records]
  }

  getRecord(id: number): HistoryRecord | null {
    return this.records.find(r => r.id === id) ?? null
  }

  async updateRecord(
    id: number,
    updates: Partial<Pick<HistoryRecord, 'transcribeText' | 'optimizeText' | 'status' | 'errorMessage'>>
  ): Promise<void> {
    this.writeQueue = this.writeQueue
      .then(() => this._updateAndPersist(id, updates))
      .catch(err => console.error('[History] Update failed:', err))
    return this.writeQueue
  }

  async getAudioBase64(id: number): Promise<string | null> {
    const record = this.getRecord(id)
    if (!record?.audioPath) return null
    try {
      const bytes = await readFile(record.audioPath)
      return uint8ArrayToBase64(new Uint8Array(bytes))
    } catch {
      return null
    }
  }

  private async _addAndPersist(params: AddRecordParams): Promise<void> {
    const id = this.nextId()
    let audioPath: string | null = null

    if (params.audioBase64) {
      const destPath = `${this.audioDir}/${id}.wav`
      const bytes = base64ToUint8Array(params.audioBase64)
      await writeFile(destPath, bytes)
      audioPath = destPath
    }

    const record: HistoryRecord = {
      id,
      createdAt: new Date().toISOString(),
      audioPath,
      transcribeText: params.transcribeText,
      optimizeText: params.optimizeText,
      status: params.status,
      ...(params.errorMessage ? { errorMessage: params.errorMessage } : {})
    }

    this.records.push(record)
    while (this.records.length > MAX_RECORDS) {
      const oldest = this.records.shift()!
      if (oldest.audioPath) {
        try { await remove(oldest.audioPath) } catch { /* ignore */ }
      }
    }

    await this.persist()
  }

  private async _updateAndPersist(
    id: number,
    updates: Partial<Pick<HistoryRecord, 'transcribeText' | 'optimizeText' | 'status' | 'errorMessage'>>
  ): Promise<void> {
    const record = this.records.find(r => r.id === id)
    if (!record) return
    Object.assign(record, updates)
    await this.persist()
  }

  private async persist(): Promise<void> {
    const data = JSON.stringify(this.records, null, 2)
    await writeTextFile(this.jsonPath, data)
    this.onChange?.(this.getRecords())
  }
}
