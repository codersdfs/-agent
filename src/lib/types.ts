// ─── Chat Messages ────────────────────────────────────────────────────────────

export type MessageType = "user" | "assistant" | "system" | "tool";

export interface Message {
  id: string;
  type: MessageType;
  content: string;
  timestamp: number;
  toolCall?: ToolCall;
}

// ─── Tool Calls ───────────────────────────────────────────────────────────────

export interface ToolCall {
  id: string;
  tool: string;
  args: Record<string, unknown>;
  success?: boolean;
  output?: string;
  error?: string | null;
  gateResult?: GateCheckResult;
  durationMs?: number;
}

// ─── Gate ─────────────────────────────────────────────────────────────────────

export interface GateCheckResult {
  passed: boolean;
  score: number;
  violations: GateViolationInfo[];
}

export interface GateViolationInfo {
  category: string;
  message: string;
  toolHint?: string | null;
  line?: number | null;
}

// ─── Chat Commands ────────────────────────────────────────────────────────────

export interface SendMessageRequest {
  content: string;
  agentType: string;
  provider?: unknown;
}

export interface SendMessageResponse {
  messageId: string;
  content: string;
  agentType: string;
}

export interface StreamMessageRequest {
  content: string;
  agentType: string;
  provider?: unknown;
  systemPrompt?: string | null;
}

// ─── Plan ─────────────────────────────────────────────────────────────────────

export interface PlanStep {
  id: number;
  action: string;
  description: string;
  filePath?: string | null;
  estimatedLines?: number | null;
  dependencies: number[];
}

export interface StructuredPlan {
  taskSummary: string;
  language: string;
  steps: PlanStep[];
  filesAffected: string[];
  estimatedComplexity: string;
  riskLevel: string;
}

export interface PlanGeneratedPayload {
  taskId: string;
  plan: StructuredPlan;
  rawOutput: string;
}

// ─── Build ────────────────────────────────────────────────────────────────────

export interface BuildSessionEntry {
  stepIndex: number;
  tool: string;
  args: Record<string, unknown>;
  success: boolean;
  outputPreview: string;
  error?: string | null;
  gatePassed?: boolean | null;
  gateScore?: number | null;
  durationMs: number;
  retries: number;
  timestampStart: string;
  timestampEnd: string;
}

export interface BuildConfigResponse {
  autoApprove: boolean;
}

// ─── Review ───────────────────────────────────────────────────────────────────

export interface ReviewRequest {
  code: string;
  context: string;
}

export interface ViolationBreakdown {
  category: string;
  count: number;
  penalty: number;
  messages: string[];
}

export interface LlmReviewIssue {
  category: string;
  severity: string;
  description: string;
}

export interface ScoreBreakdown {
  gateScore: number;
  llmScore: number;
  combinedScore: number;
  penalties: ViolationBreakdown[];
  llmIssues: LlmReviewIssue[];
  passed: boolean;
  passThreshold: number;
}

export interface PromotionStats {
  totalPatterns: number;
  promoted: number;
  frequency1: number;
  frequency2: number;
  frequency3Plus: number;
  demotedLastRun: number;
}

export interface CombinedReviewOutput {
  gateViolations: GateViolationInfo[];
  llmReview?: string | null;
  scoreBreakdown: ScoreBreakdown;
}

export interface ScoreResponse {
  scoreBreakdown?: ScoreBreakdown | null;
  promotionStats?: PromotionStats | null;
  retryCount: number;
  maxRetries: number;
  pipelineStatus: string;
}

// ─── Gate Command ─────────────────────────────────────────────────────────────

export interface GateCheckRequest {
  content: string;
  context: string;
  language?: string | null;
}

// ─── Memory ───────────────────────────────────────────────────────────────────

export interface MemoryStoreRequest {
  key: string;
  value: string;
  layer: string;
}

export interface MemorySearchRequest {
  query: string;
  layer?: string | null;
  limit?: number | null;
}

export interface MemoryEntry {
  id: string;
  key: string;
  value: string;
  layer: string;
  createdAt: string;
}

export interface MemorySearchResponse {
  entries: MemoryEntry[];
  relevance: number[];
}

// ─── Tools ────────────────────────────────────────────────────────────────────

export interface ToolRequest {
  tool: string;
  args: Record<string, unknown>;
}

export interface ToolResult {
  success: boolean;
  output: string;
  error?: string | null;
  gateResult?: GateCheckResult | null;
}

// ─── Permissions ──────────────────────────────────────────────────────────────

export interface PermissionEvent {
  requestId: string;
  tool: string;
  args: Record<string, unknown>;
  reason: string;
  stepId: number;
  stepDescription: string;
}
