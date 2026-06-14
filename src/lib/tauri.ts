import { invoke } from '@tauri-apps/api/core'

export interface SendMessageRequest {
  content: string
  agent_type: string
}

export interface SendMessageResponse {
  message_id: string
  content: string
  agent_type: string
}

export async function sendMessage(request: SendMessageRequest): Promise<SendMessageResponse> {
  return invoke('send_message', { request })
}

export async function getAppInfo(): Promise<{ name: string; version: string; pipeline_status: string }> {
  return invoke('get_app_info')
}

export interface ToolRequest {
  tool: string
  args: Record<string, unknown>
}

export interface ToolResult {
  success: boolean
  output: string
  error: string | null
}

export async function executeTool(request: ToolRequest): Promise<ToolResult> {
  return invoke('execute_tool', { request })
}

export async function listTools(): Promise<string[]> {
  return invoke('list_tools')
}
