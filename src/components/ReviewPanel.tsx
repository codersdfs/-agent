import { useEffect, useState, useCallback } from 'react'
import { useChatStore } from '../stores/chatStore'
import {
  getRules,
  checkGate,
  runReview,
  getScoreBreakdown,
  demoteStaleRules,
  resetRetryCount,
} from '../lib/tauri'

export function ReviewPanel() {
  const reviewMode = useChatStore((s) => s.reviewMode)
  const setReviewMode = useChatStore((s) => s.setReviewMode)
  const gateViolations = useChatStore((s) => s.gateViolations)
  const promotedRules = useChatStore((s) => s.promotedRules)
  const setPromotedRules = useChatStore((s) => s.setPromotedRules)
  const scoreBreakdown = useChatStore((s) => s.scoreBreakdown)
  const setScoreBreakdown = useChatStore((s) => s.setScoreBreakdown)
  const promotionStats = useChatStore((s) => s.promotionStats)
  const setPromotionStats = useChatStore((s) => s.setPromotionStats)
  const retryCount = useChatStore((s) => s.retryCount)
  const setRetryCount = useChatStore((s) => s.setRetryCount)
  const maxRetries = useChatStore((s) => s.maxRetries)

  const [contentToCheck, setContentToCheck] = useState('')
  const [checkResult, setCheckResult] = useState<string | null>(null)
  const [reviewOutput, setReviewOutput] = useState<string | null>(null)
  const [reviewLoading, setReviewLoading] = useState(false)

  const refreshAll = useCallback(async () => {
    const [rules, scoreResp] = await Promise.all([
      getRules().catch(() => [] as string[]),
      getScoreBreakdown().catch(() => null),
    ])
    setPromotedRules(rules)
    if (scoreResp) {
      setScoreBreakdown(scoreResp.score_breakdown)
      setPromotionStats(scoreResp.promotion_stats)
      setRetryCount(scoreResp.retry_count)
    }
  }, [setPromotedRules, setScoreBreakdown, setPromotionStats, setRetryCount])

  useEffect(() => {
    refreshAll()
  }, [refreshAll])

  async function handleCheckContent() {
    if (!contentToCheck.trim()) return
    try {
      const result = await checkGate({
        content: contentToCheck,
        context: 'manual check',
      })
      if (result.passed) {
        setCheckResult(`✓ Passed (score: ${result.score}) — no violations`)
      } else {
        const lines = result.violations.map(
          (v) => `[${v.category}] ${v.message}${v.tool_hint ? `\n  → Hint: ${v.tool_hint}` : ''}`
        )
        setCheckResult(`✗ Failed (score: ${result.score})\n\n${lines.join('\n\n')}`)
      }
    } catch (e) {
      setCheckResult(`Error: ${e}`)
    }
  }

  async function handleRunReview() {
    if (!contentToCheck.trim()) return
    setReviewLoading(true)
    try {
      const output = await runReview({
        code: contentToCheck,
        context: 'manual review',
      })
      setScoreBreakdown(output.score_breakdown)
      setReviewOutput(output.llm_review)

      const sb = output.score_breakdown
      let result = `Combined Score: ${sb.combined_score}/100 (${sb.passed ? 'PASS' : 'FAIL'})\n`
      result += `Gate Score: ${sb.gate_score}\n`
      if (sb.llm_score !== null) result += `LLM Score: ${sb.llm_score}\n`
      result += `Pass Threshold: ${sb.pass_threshold}\n\n`

      if (sb.gate_penalties.length > 0) {
        result += 'Gate Penalties:\n'
        for (const p of sb.gate_penalties) {
          result += `  ${p.category}: ${p.count}x (${p.penalty}pts)\n`
        }
      }

      if (sb.llm_issues.length > 0) {
        result += '\nLLM Issues:\n'
        for (const issue of sb.llm_issues) {
          result += `  [${issue.severity}/${issue.category}] ${issue.description}\n`
        }
      }

      setCheckResult(result)
      await refreshAll()
    } catch (e) {
      setCheckResult(`Error: ${e}`)
    } finally {
      setReviewLoading(false)
    }
  }

  async function handleDemote() {
    const count = await demoteStaleRules()
    await refreshAll()
    setCheckResult(`Demoted ${count} stale rules`)
  }

  async function handleResetRetry() {
    await resetRetryCount()
    setRetryCount(0)
  }

  const modeOptions = [
    { id: 'off' as const, label: 'Off', desc: 'Gate only' },
    { id: 'summary' as const, label: 'Summary', desc: 'Gate + LLM review after build' },
    { id: 'live' as const, label: 'Live', desc: 'Gate + LLM per diff' },
  ]

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold text-omega-200">Review Panel</h2>
        <div className="flex items-center gap-2">
          {retryCount > 0 && (
            <span className="text-xs text-yellow">
              Retry {retryCount}/{maxRetries}
            </span>
          )}
        </div>
      </div>

      {/* Score Breakdown */}
      {scoreBreakdown && (
        <div className={`border rounded p-3 ${
          scoreBreakdown.passed ? 'border-green/30 bg-green/5' : 'border-red/30 bg-red/5'
        }`}>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-semibold text-omega-200">
              {scoreBreakdown.passed ? '✓ PASSED' : '✗ FAILED'}
            </span>
            <span className="text-lg font-bold font-mono text-omega-100">
              {scoreBreakdown.combined_score}
              <span className="text-xs text-omega-500">/{scoreBreakdown.pass_threshold}</span>
            </span>
          </div>
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="bg-omega-800 rounded px-2 py-1">
              <span className="text-omega-500">Gate</span>
              <span className="float-right font-mono text-omega-200">{scoreBreakdown.gate_score}</span>
            </div>
            <div className="bg-omega-800 rounded px-2 py-1">
              <span className="text-omega-500">LLM</span>
              <span className="float-right font-mono text-omega-200">
                {scoreBreakdown.llm_score !== null ? scoreBreakdown.llm_score : '—'}
              </span>
            </div>
          </div>
          {scoreBreakdown.gate_penalties.length > 0 && (
            <div className="mt-2 space-y-1">
              <span className="text-[10px] text-omega-500 uppercase tracking-wider">Gate Penalties</span>
              {scoreBreakdown.gate_penalties.map((p, i) => (
                <div key={i} className="flex items-center gap-2 text-xs bg-omega-800/50 rounded px-2 py-1">
                  <span className={`text-[10px] px-1 py-0.5 rounded font-mono ${
                    p.category === 'Golden' ? 'bg-red-900/50 text-red-400'
                    : p.category === 'Structural' ? 'bg-blue-900/50 text-blue-400'
                    : p.category === 'Repeated' ? 'bg-purple-900/50 text-purple-400'
                    : 'bg-yellow-900/50 text-yellow-400'
                  }`}>
                    {p.category}
                  </span>
                  <span className="text-omega-400">{p.count}x</span>
                  <span className="text-omega-500">−{p.penalty}pts</span>
                </div>
              ))}
            </div>
          )}
          {scoreBreakdown.llm_issues.length > 0 && (
            <div className="mt-2 space-y-1">
              <span className="text-[10px] text-omega-500 uppercase tracking-wider">LLM Issues</span>
              {scoreBreakdown.llm_issues.map((issue, i) => (
                <div key={i} className="flex items-start gap-2 text-xs bg-omega-800/50 rounded px-2 py-1">
                  <span className={`text-[10px] px-1 py-0.5 rounded font-mono shrink-0 ${
                    issue.severity === 'error' ? 'bg-red-900/50 text-red-400'
                    : 'bg-yellow-900/50 text-yellow-400'
                  }`}>
                    {issue.category}
                  </span>
                  <span className="text-omega-400">{issue.description}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Promotion Stats */}
      {promotionStats && (
        <div className="bg-omega-800 border border-omega-700 rounded p-3">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-medium text-omega-400">Promotion Tracker</span>
            <span className="text-xs text-omega-500">{promotionStats.total_patterns} patterns</span>
          </div>
          <div className="grid grid-cols-4 gap-2 text-xs">
            <div className="text-center">
              <div className="text-omega-200 font-bold">{promotionStats.promoted}</div>
              <div className="text-omega-500">Active</div>
            </div>
            <div className="text-center">
              <div className="text-omega-200 font-bold">{promotionStats.frequency_1}</div>
              <div className="text-omega-500">Freq 1</div>
            </div>
            <div className="text-center">
              <div className="text-omega-200 font-bold">{promotionStats.frequency_2}</div>
              <div className="text-omega-500">Freq 2</div>
            </div>
            <div className="text-center">
              <div className="text-omega-200 font-bold">{promotionStats.frequency_3_plus}</div>
              <div className="text-omega-500">Freq 3+</div>
            </div>
          </div>
          {promotionStats.demoted_last_run > 0 && (
            <div className="mt-2 text-xs text-omega-500">
              {promotionStats.demoted_last_run} rules demoted
            </div>
          )}
        </div>
      )}

      {/* Review Mode */}
      <div className="space-y-2">
        <label className="text-xs text-omega-400">Review Mode</label>
        <div className="grid grid-cols-3 gap-2">
          {modeOptions.map((opt) => (
            <button
              key={opt.id}
              onClick={() => setReviewMode(opt.id)}
              className={`px-3 py-2 rounded text-xs text-left transition-colors ${
                reviewMode === opt.id
                  ? 'bg-accent text-white'
                  : 'bg-omega-800 text-omega-400 hover:bg-omega-700'
              }`}
            >
              <div className="font-medium">{opt.label}</div>
              <div className="opacity-70 mt-0.5">{opt.desc}</div>
            </button>
          ))}
        </div>
      </div>

      {/* Manual Gate + Review Check */}
      <div className="space-y-2">
        <label className="text-xs text-omega-400">Manual Code Review</label>
        <textarea
          value={contentToCheck}
          onChange={(e) => setContentToCheck(e.target.value)}
          placeholder="Paste code to run Gate + LLM review..."
          rows={5}
          className="w-full bg-omega-800 border border-omega-600 rounded px-3 py-1.5 text-sm text-omega-100 placeholder-omega-500 focus:outline-none focus:border-accent font-mono"
        />
        <div className="flex gap-2">
          <button
            onClick={handleCheckContent}
            disabled={!contentToCheck.trim()}
            className="bg-omega-700 hover:bg-omega-600 disabled:opacity-50 text-omega-200 text-xs px-3 py-1.5 rounded transition-colors"
          >
            Gate Only
          </button>
          <button
            onClick={handleRunReview}
            disabled={!contentToCheck.trim() || reviewLoading}
            className="bg-accent/80 hover:bg-accent disabled:opacity-50 text-white text-xs px-3 py-1.5 rounded transition-colors"
          >
            {reviewLoading ? 'Reviewing...' : 'Full Review'}
          </button>
          <button
            onClick={handleDemote}
            className="bg-omega-700 hover:bg-omega-600 text-omega-200 text-xs px-3 py-1.5 rounded transition-colors"
          >
            Demote Stale
          </button>
          {retryCount > 0 && (
            <button
              onClick={handleResetRetry}
              className="bg-omega-700 hover:bg-omega-600 text-omega-200 text-xs px-3 py-1.5 rounded transition-colors"
            >
              Reset Retry
            </button>
          )}
        </div>
        {checkResult && (
          <pre className="text-xs text-omega-300 bg-omega-800 rounded p-3 mt-2 whitespace-pre-wrap font-mono max-h-48 overflow-y-auto">
            {checkResult}
          </pre>
        )}
      </div>

      {/* LLM Review Output */}
      {reviewOutput && (
        <div className="space-y-2">
          <h3 className="text-xs font-medium text-omega-400">LLM Review Output</h3>
          <div className="bg-omega-800 border border-omega-700 rounded p-3 text-xs text-omega-300 whitespace-pre-wrap font-mono max-h-64 overflow-y-auto">
            {reviewOutput}
          </div>
        </div>
      )}

      {/* Session Gate Violations */}
      <div className="space-y-2">
        <h3 className="text-xs font-medium text-omega-400">Session Gate Violations</h3>
        {gateViolations.length === 0 ? (
          <p className="text-xs text-omega-500">No violations this session</p>
        ) : (
          <div className="space-y-1.5">
            {gateViolations.map((v, i) => (
              <div key={i} className="bg-omega-800 border border-yellow-700/30 rounded px-3 py-2">
                <div className="flex items-center gap-2">
                  <span className={`text-[10px] px-1.5 py-0.5 rounded font-mono ${
                    v.category === 'Golden' ? 'bg-red-900/50 text-red-400'
                    : v.category === 'Structural' ? 'bg-blue-900/50 text-blue-400'
                    : 'bg-yellow-900/50 text-yellow-400'
                  }`}>
                    {v.category}
                  </span>
                </div>
                <p className="text-xs text-omega-300 mt-1">{v.message}</p>
                {v.tool_hint && (
                  <p className="text-[11px] text-omega-500 mt-0.5">Hint: {v.tool_hint}</p>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Promoted Rules */}
      <div className="space-y-2">
        <h3 className="text-xs font-medium text-omega-400">Promoted Rules</h3>
        {promotedRules.length === 0 ? (
          <p className="text-xs text-omega-500">No promoted rules yet</p>
        ) : (
          <div className="space-y-1">
            {promotedRules.map((r, i) => (
              <div key={i} className="text-xs text-omega-400 bg-omega-800 rounded px-3 py-1.5 font-mono">
                {r}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
