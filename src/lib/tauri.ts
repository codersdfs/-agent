import { invoke } from "@tauri-apps/api/core";
import type {
  SendMessageRequest,
  SendMessageResponse,
  StreamMessageRequest,
  ToolRequest,
  ToolResult,
  PlanGeneratedPayload,
  StructuredPlan,
  BuildSessionEntry,
  BuildConfigResponse,
  ReviewRequest,
  CombinedReviewOutput,
  ScoreResponse,
  GateCheckRequest,
  GateCheckResult,
  MemoryStoreRequest,
  MemorySearchRequest,
  MemorySearchResponse,
} from "./types";

// ─── Chat ─────────────────────────────────────────────────────────────────────

export async function sendMessage(
  request: SendMessageRequest
): Promise<SendMessageResponse> {
  return invoke("cmd_send_message", { request });
}

export async function streamMessage(
  request: StreamMessageRequest
): Promise<string> {
  return invoke("cmd_stream_message", { request });
}

export async function listModels(): Promise<string[]> {
  return invoke("cmd_list_models");
}

// ─── Tools ────────────────────────────────────────────────────────────────────

export async function executeTool(
  request: ToolRequest
): Promise<ToolResult> {
  return invoke("cmd_execute_tool", { request });
}

export async function listTools(): Promise<string[]> {
  return invoke("cmd_list_tools");
}

// ─── Plan ─────────────────────────────────────────────────────────────────────

export async function generatePlan(
  task: string
): Promise<PlanGeneratedPayload> {
  return invoke("cmd_generate_plan", { task });
}

export async function getPlan(): Promise<StructuredPlan | null> {
  return invoke("cmd_get_plan");
}

export async function approvePlan(): Promise<string> {
  return invoke("cmd_approve_plan");
}

export async function getPlanSystemPrompt(): Promise<string> {
  return invoke("cmd_get_plan_system_prompt");
}

// ─── Build ────────────────────────────────────────────────────────────────────

export async function executeBuild(): Promise<BuildSessionEntry[]> {
  return invoke("cmd_execute_build");
}

export async function respondPermission(
  requestId: string,
  approved: boolean
): Promise<string> {
  return invoke("cmd_respond_permission", { requestId, approved });
}

export async function getBuildSession(): Promise<BuildSessionEntry[]> {
  return invoke("cmd_get_build_session");
}

export async function getBuildConfig(): Promise<BuildConfigResponse> {
  return invoke("cmd_get_build_config");
}

export async function setBuildConfig(
  autoApprove: boolean
): Promise<string> {
  return invoke("cmd_set_build_config", { autoApprove });
}

// ─── Review ───────────────────────────────────────────────────────────────────

export async function runReview(
  request: ReviewRequest
): Promise<CombinedReviewOutput> {
  return invoke("cmd_run_review", { request });
}

export async function getScoreBreakdown(): Promise<ScoreResponse> {
  return invoke("cmd_get_score_breakdown");
}

export async function getPromotionStats(): Promise<unknown> {
  return invoke("cmd_get_promotion_stats");
}

export async function demoteStaleRules(): Promise<number> {
  return invoke("cmd_demote_stale_rules");
}

export async function resetRetryCount(): Promise<string> {
  return invoke("cmd_reset_retry_count");
}

// ─── Gate ─────────────────────────────────────────────────────────────────────

export async function checkGate(
  request: GateCheckRequest
): Promise<GateCheckResult> {
  return invoke("cmd_check_gate", { request });
}

export async function getRules(): Promise<string[]> {
  return invoke("cmd_get_rules");
}

export async function resetRules(): Promise<string> {
  return invoke("cmd_reset_rules");
}

export async function setReviewMode(mode: string): Promise<string> {
  return invoke("cmd_set_review_mode", { mode });
}

// ─── Providers ────────────────────────────────────────────────────────────────

export async function listProviders(): Promise<string[]> {
  return invoke("cmd_list_providers");
}

export async function fetchModels(
  baseUrl: string
): Promise<string[]> {
  return invoke("cmd_fetch_models", { baseUrl });
}

// ─── Provider Config ──────────────────────────────────────────────────────────

export async function getProviderConfig(): Promise<Record<string, string>> {
  return invoke("cmd_get_provider_config");
}

export async function setProviderConfig(
  key: string,
  value: string
): Promise<string> {
  return invoke("cmd_set_provider_config", { key, value });
}

// ─── Memory ───────────────────────────────────────────────────────────────────

export async function memoryStore(
  request: MemoryStoreRequest
): Promise<string> {
  return invoke("cmd_memory_store", { request });
}

export async function memorySearch(
  request: MemorySearchRequest
): Promise<MemorySearchResponse> {
  return invoke("cmd_memory_search", { request });
}

export async function memoryRemember(
  key: string,
  layer?: string | null
): Promise<string | null> {
  return invoke("cmd_memory_remember", { key, layer: layer ?? null });
}

export async function memoryCount(
  layer?: string | null
): Promise<number> {
  return invoke("cmd_memory_count", { layer: layer ?? null });
}

export async function memoryDelete(id: string): Promise<void> {
  return invoke("cmd_memory_delete", { id });
}

export async function memoryClear(
  layer?: string | null
): Promise<number> {
  return invoke("cmd_memory_clear", { layer: layer ?? null });
}
