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

// Plan types
export interface PlanStep {
  id: number
  action: string
  description: string
  file_path: string | null
  estimated_lines: number | null
  dependencies: number[]
}

export interface StructuredPlan {
  task_summary: string
  language: string
  steps: PlanStep[]
  files_affected: string[]
  estimated_complexity: string
  risk_level: string
}

export interface PlanGeneratedPayload {
  task_id: string
  plan: StructuredPlan
  raw_output: string
}

export async function generatePlan(task: string): Promise<string> {
  return invoke('generate_plan', { task })
}

export async function getPlan(): Promise<StructuredPlan | null> {
  return invoke('get_plan')
}

export async function approvePlan(): Promise<string> {
  return invoke('approve_plan')
}

export async function getPlanSystemPrompt(): Promise<string> {
  return invoke('get_plan_system_prompt')
}

// Build types
export interface BuildSessionEntry {
  step_index: number
  tool: string
  args: Record<string, unknown>
  success: boolean
  output_preview: string
  error: string | null
  gate_passed: boolean | null
  gate_score: number | null
  duration_ms: number
  retries: number
  timestamp_start: string
  timestamp_end: string
}

export interface PermissionRequest {
  id: string
  tool: string
  args: Record<string, unknown>
  reason: string
  step_id: number
  step_description: string
}

export interface BuildProgress {
  total_steps: number
  completed_steps: number
  current_step: number
  status: string
  total_retries: number
}

export interface BuildConfigResponse {
  auto_approve: boolean
}

export async function executeBuild(): Promise<BuildSessionEntry[]> {
  return invoke('execute_build')
}

export async function respondPermission(requestId: string, approved: boolean): Promise<string> {
  return invoke('respond_permission', { requestId, approved })
}

export async function getBuildSession(): Promise<BuildSessionEntry[]> {
  return invoke('get_build_session')
}

export async function getBuildConfig(): Promise<BuildConfigResponse> {
  return invoke('get_build_config')
}

export async function setBuildConfig(autoApprove: boolean): Promise<string> {
  return invoke('set_build_config', { autoApprove })
}

// Review Score types
export interface ViolationBreakdown {
  category: string
  count: number
  penalty: number
  messages: string[]
}

export interface LlmReviewIssue {
  category: string
  severity: string
  description: string
}

export interface ScoreBreakdown {
  gate_score: number
  llm_score: number | null
  combined_score: number
  gate_penalties: ViolationBreakdown[]
  llm_issues: LlmReviewIssue[]
  passed: boolean
  pass_threshold: number
}

export interface PromotionStats {
  total_patterns: number
  promoted: number
  frequency_1: number
  frequency_2: number
  frequency_3_plus: number
  demoted_last_run: number
}

export interface CombinedReviewOutput {
  gate_violations: GateViolation[]
  llm_review: string | null
  score_breakdown: ScoreBreakdown
}

export interface ReviewRequest {
  code: string
  context: string
}

export interface ScoreResponse {
  score_breakdown: ScoreBreakdown | null
  promotion_stats: PromotionStats | null
  retry_count: number
  max_retries: number
  pipeline_status: string
}

export async function runReview(request: ReviewRequest): Promise<CombinedReviewOutput> {
  return invoke('run_review', { request })
}

export async function getScoreBreakdown(): Promise<ScoreResponse> {
  return invoke('get_score_breakdown')
}

export async function getPromotionStats(): Promise<PromotionStats> {
  return invoke('get_promotion_stats')
}

export async function demoteStaleRules(): Promise<number> {
  return invoke('demote_stale_rules')
}

export async function resetRetryCount(): Promise<string> {
  return invoke('reset_retry_count')
}

// Memory types
export type MemoryLayer = 'session' | 'project' | 'user'

export interface MemoryEntry {
  id: string
  layer: MemoryLayer
  key: string
  value: string
  embedding: number[] | null
  timestamp: string
}

export interface MemorySearchResponse {
  entries: MemoryEntry[]
  relevance: number[]
}

export interface MemoryStoreRequest {
  key: string
  value: string
  layer: MemoryLayer
}

export interface MemorySearchRequest {
  query: string
  layer?: MemoryLayer
  limit?: number
}

export async function memoryStore(request: MemoryStoreRequest): Promise<string> {
  return invoke('memory_store', { request })
}

export async function memorySearch(request: MemorySearchRequest): Promise<MemorySearchResponse> {
  return invoke('memory_search', { request })
}

export async function memoryRemember(key: string, layer?: MemoryLayer): Promise<string | null> {
  return invoke('memory_remember', { key, layer })
}

export async function memoryCount(layer?: MemoryLayer): Promise<number> {
  return invoke('memory_count', { layer })
}

export async function memoryDelete(id: string): Promise<void> {
  return invoke('memory_delete', { id })
}

export async function memoryClear(layer?: MemoryLayer): Promise<number> {
  return invoke('memory_clear', { layer })
}
