import { create } from 'zustand'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type { GateViolation, BuildSessionEntry, PermissionRequest, BuildProgress, ScoreBreakdown, PromotionStats } from '../lib/tauri'

export interface Message {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: number
  gate_violations?: GateViolation[]
  review_comment?: string
}

export interface ProviderConfig {
  provider: string
  api_key?: string
  base_url?: string
  model: string
  max_tokens: number
  temperature: number
}

interface ChatTokenPayload {
  message_id: string
  token: string
  done: boolean
  model: string | null
}

export type ReviewMode = 'off' | 'summary' | 'live'

interface ChatState {
  messages: Message[]
  isProcessing: boolean
  currentMessageId: string | null
  streamedContent: string
  providerConfig: ProviderConfig
  unlisten: UnlistenFn | null
  reviewMode: ReviewMode
  gateViolations: GateViolation[]
  promotedRules: string[]
  pipelineStatus: string
  buildSession: BuildSessionEntry[]
  permissionRequest: PermissionRequest | null
  buildProgress: BuildProgress | null
  buildInProgress: boolean
  buildAutoApprove: boolean
  scoreBreakdown: ScoreBreakdown | null
  promotionStats: PromotionStats | null
  retryCount: number
  maxRetries: number

  sendMessage: (content: string, agentType?: string) => Promise<void>
  appendMessage: (msg: Message) => void
  setProcessing: (v: boolean) => void
  setProviderConfig: (config: Partial<ProviderConfig>) => void
  setupStreaming: () => Promise<void>
  teardownStreaming: () => void
  clearMessages: () => void
  setReviewMode: (mode: ReviewMode) => void
  setGateViolations: (v: GateViolation[]) => void
  setPromotedRules: (r: string[]) => void
  setBuildSession: (v: BuildSessionEntry[]) => void
  setPermissionRequest: (v: PermissionRequest | null) => void
  setBuildProgress: (v: BuildProgress | null) => void
  setBuildInProgress: (v: boolean) => void
  setBuildAutoApprove: (v: boolean) => void
  setScoreBreakdown: (v: ScoreBreakdown | null) => void
  setPromotionStats: (v: PromotionStats | null) => void
  setRetryCount: (v: number) => void
  setMaxRetries: (v: number) => void
  setupBuildListeners: () => Promise<UnlistenFn[]>
}

export const useChatStore = create<ChatState>((set, get) => ({
  messages: [
    {
      id: 'system-1',
      role: 'system',
      content: 'Omega Agent ready. Configure a provider in Settings to start.',
      timestamp: Date.now(),
    },
  ],
  isProcessing: false,
  currentMessageId: null,
  streamedContent: '',
  providerConfig: {
    provider: 'openai',
    api_key: '',
    base_url: '',
    model: 'gpt-4o',
    max_tokens: 4096,
    temperature: 0.7,
  },
  unlisten: null,
  reviewMode: 'summary',
  gateViolations: [],
  promotedRules: [],
  pipelineStatus: 'Idle',
  buildSession: [],
  permissionRequest: null,
  buildProgress: null,
  buildInProgress: false,
  buildAutoApprove: false,
  scoreBreakdown: null,
  promotionStats: null,
  retryCount: 0,
  maxRetries: 3,

  appendMessage: (msg) => {
    set((s) => ({ messages: [...s.messages, msg] }))
  },

  setProcessing: (v) => set({ isProcessing: v }),

  setProviderConfig: (config) => {
    set((s) => ({ providerConfig: { ...s.providerConfig, ...config } }))
  },

  setReviewMode: (mode) => {
    set({ reviewMode: mode })
  },

  setGateViolations: (v) => set({ gateViolations: v }),

  setPromotedRules: (r) => set({ promotedRules: r }),

  setBuildSession: (v) => set({ buildSession: v }),

  setPermissionRequest: (v) => set({ permissionRequest: v }),

  setBuildProgress: (v) => set({ buildProgress: v }),

  setBuildInProgress: (v) => set({ buildInProgress: v }),

  setBuildAutoApprove: (v) => set({ buildAutoApprove: v }),

  setScoreBreakdown: (v) => set({ scoreBreakdown: v }),

  setPromotionStats: (v) => set({ promotionStats: v }),

  setRetryCount: (v) => set({ retryCount: v }),

  setMaxRetries: (v) => set({ maxRetries: v }),

  clearMessages: () => {
    set({
      messages: [
        {
          id: 'system-1',
          role: 'system',
          content: 'Omega Agent ready.',
          timestamp: Date.now(),
        },
      ],
    })
  },

  setupStreaming: async () => {
    const unlisten = await listen<ChatTokenPayload>('chat-token', (event) => {
      const payload = event.payload
      const state = get()

      if (!state.currentMessageId) {
        set({ currentMessageId: payload.message_id })
      }

      if (payload.done) {
        set((s) => {
          const finalContent = s.streamedContent
          const newMsg: Message = {
            id: payload.message_id,
            role: 'assistant',
            content: finalContent,
            timestamp: Date.now(),
            gate_violations: s.gateViolations.length > 0 ? [...s.gateViolations] : undefined,
          }
          return {
            messages: [...s.messages, newMsg],
            streamedContent: '',
            currentMessageId: null,
            isProcessing: false,
            gateViolations: [],
          }
        })
        return
      }

      set((s) => ({ streamedContent: s.streamedContent + payload.token }))
    })

    set({ unlisten })
  },

  teardownStreaming: () => {
    const { unlisten } = get()
    if (unlisten) {
      unlisten()
      set({ unlisten: null })
    }
  },

  sendMessage: async (content, agentType = 'plan') => {
    const state = get()
    if (!content.trim() || state.isProcessing) return

    const userMsg: Message = {
      id: crypto.randomUUID(),
      role: 'user',
      content: content.trim(),
      timestamp: Date.now(),
    }

    set((s) => ({
      messages: [...s.messages, userMsg],
      isProcessing: true,
      streamedContent: '',
      currentMessageId: null,
      pipelineStatus: agentType === 'plan' ? 'Planning' : agentType === 'review' ? 'Reviewing' : 'Building',
    }))

    // For build agent, show plan phase first then build
    if (agentType === 'build') {
      set((s) => ({
        messages: [...s.messages, {
          id: crypto.randomUUID(),
          role: 'system',
          content: '⚡ Build phase started — executing changes with Gate enforcement',
          timestamp: Date.now(),
        }],
      }))
    }

    try {
      const { streamMessage } = await import('../lib/tauri')
      const provider = state.providerConfig

      const systemPrompt = agentType === 'plan'
        ? 'You are a Plan agent. Analyze the task and produce a structured plan. Do not write code.'
        : agentType === 'review'
        ? 'You are a Code Review agent. Analyze the code for issues. Be critical and thorough. Use structured feedback.'
        : `You are a Build agent. Implement the requested changes. Write clean, correct code.
Gate enforcement is active — write/edit tools return GateResult in the response.
Review mode: ${state.reviewMode}.
If Gate violations appear, fix them before proceeding.`

      await streamMessage({
        content: userMsg.content,
        agent_type: agentType,
        provider: {
          provider: provider.provider,
          api_key: provider.api_key,
          base_url: provider.base_url,
          model: provider.model,
          max_tokens: provider.max_tokens,
          temperature: provider.temperature,
        },
        system_prompt: systemPrompt,
      })
    } catch (err) {
      const errorMsg: Message = {
        id: crypto.randomUUID(),
        role: 'system',
        content: `Error: ${err}`,
        timestamp: Date.now(),
      }
      set((s) => ({
        messages: [...s.messages, errorMsg],
        isProcessing: false,
        streamedContent: '',
        pipelineStatus: 'Failed',
      }))
    } finally {
      set((s) => ({
        pipelineStatus: s.pipelineStatus === 'Failed' ? 'Failed' : 'Idle',
      }))
    }
  },

  setupBuildListeners: async () => {
    const { listen } = await import('@tauri-apps/api/event')

    const unlistenStepStart = await listen<{
      step_index: number
      step_id: number
      description: string
      action: string
      file_path: string | null
    }>('build-step-start', (event) => {
      const p = event.payload
      set((s) => ({
        buildProgress: {
          total_steps: s.buildProgress?.total_steps ?? 0,
          completed_steps: s.buildProgress?.completed_steps ?? 0,
          current_step: p.step_index,
          status: `Executing: ${p.description}`,
          total_retries: s.buildProgress?.total_retries ?? 0,
        },
        pipelineStatus: 'Building',
      }))
    })

    const unlistenStepEnd = await listen<{
      step_index: number
      status: string
      gate_passed: boolean | null
      gate_score: number | null
      duration_ms: number
    }>('build-step-end', async (event) => {
      const p = event.payload
      set((s) => ({
        buildProgress: s.buildProgress ? {
          ...s.buildProgress,
          completed_steps: s.buildProgress.completed_steps + (p.status === 'completed' ? 1 : 0),
          current_step: s.buildProgress.current_step + 1,
          status: p.status === 'denied' ? 'Permission denied' : `Step ${p.status}`,
        } : null,
      }))
      // Refresh session log
      const { getBuildSession } = await import('../lib/tauri')
      getBuildSession().then((session) => set({ buildSession: session })).catch(() => {})
    })

    const unlistenPerm = await listen<PermissionRequest>('build-permission-request', (event) => {
      set({ permissionRequest: event.payload })
    })

    const unlistenToolExec = await listen<{
      tool: string
      attempt: number
      max_retries: number
    }>('build-tool-exec', (event) => {
      const p = event.payload
      set((s) => ({
        buildProgress: s.buildProgress ? {
          ...s.buildProgress,
          total_retries: p.attempt > 1 ? s.buildProgress.total_retries + 1 : s.buildProgress.total_retries,
          status: `Tool: ${p.tool} (attempt ${p.attempt}/${p.max_retries})`,
        } : null,
      }))
    })

    const unlistenComplete = await listen<{
      total_steps: number
      completed_steps: number
      duration_ms_total: number
    }>('build-complete', () => {
      set({
        buildInProgress: false,
        permissionRequest: null,
        pipelineStatus: 'Completed',
        buildProgress: null,
      })
    })

    return [unlistenStepStart, unlistenStepEnd, unlistenPerm, unlistenToolExec, unlistenComplete]
  },
}))
