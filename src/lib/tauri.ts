import { invoke } from '@tauri-apps/api/core'

export interface StreamMessageRequest {
  content: string
  agent_type: string
  provider?: {
    provider: string
    api_key?: string
    base_url?: string
    model: string
    max_tokens: number
    temperature: number
  }
  system_prompt?: string
}

export async function streamMessage(request: StreamMessageRequest): Promise<string> {
  return invoke('stream_message', { request })
}

export async function sendMessage(request: {
  content: string
  agent_type: string
  provider?: StreamMessageRequest['provider']
}): Promise<{ message_id: string; content: string; agent_type: string }> {
  return invoke('send_message', { request })
}

export async function listModels(config: StreamMessageRequest['provider']): Promise<string[]> {
  return invoke('list_models', { config })
}

export interface ToolRequest {
  tool: string
  args: Record<string, unknown>
}

export interface GateViolation {
  category: string
  message: string
  tool_hint: string | null
  line: number | null
}

export interface GateCheckResult {
  passed: boolean
  score: number
  violations: GateViolation[]
}

export interface ToolResult {
  success: boolean
  output: string
  error: string | null
  gate_result?: GateCheckResult | null
}

export async function executeTool(request: ToolRequest): Promise<ToolResult> {
  return invoke('execute_tool', { request })
}

export async function listTools(): Promise<string[]> {
  return invoke('list_tools')
}

export async function checkGate(request: {
  content: string
  context: string
  language?: string
}): Promise<GateCheckResult> {
  return invoke('check_gate', { request })
}

export async function getRules(): Promise<string[]> {
  return invoke('get_rules')
}

export async function resetRules(): Promise<string> {
  return invoke('reset_rules')
}

export async function setReviewMode(mode: string): Promise<string> {
  return invoke('set_review_mode', { mode })
}
