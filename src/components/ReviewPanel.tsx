import { useEffect, useState } from 'react'
import { useChatStore } from '../stores/chatStore'
import { getRules, checkGate } from '../lib/tauri'

export function ReviewPanel() {
  const reviewMode = useChatStore((s) => s.reviewMode)
  const setReviewMode = useChatStore((s) => s.setReviewMode)
  const gateViolations = useChatStore((s) => s.gateViolations)
  const promotedRules = useChatStore((s) => s.promotedRules)
  const setPromotedRules = useChatStore((s) => s.setPromotedRules)
  const [contentToCheck, setContentToCheck] = useState('')
  const [checkResult, setCheckResult] = useState<string | null>(null)

  useEffect(() => {
    getRules().then(setPromotedRules).catch(() => {})
  }, [setPromotedRules])

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

  const modeOptions = [
    { id: 'off' as const, label: 'Off', desc: 'Gate only' },
    { id: 'summary' as const, label: 'Summary', desc: 'Gate + LLM review after build' },
    { id: 'live' as const, label: 'Live', desc: 'Gate + LLM per diff' },
  ]

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-4">
      <h2 className="text-sm font-semibold text-omega-200">Review Panel</h2>

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

      <div className="space-y-2">
        <label className="text-xs text-omega-400">Manual Gate Check</label>
        <textarea
          value={contentToCheck}
          onChange={(e) => setContentToCheck(e.target.value)}
          placeholder="Paste code to check against active rules..."
          rows={5}
          className="w-full bg-omega-800 border border-omega-600 rounded px-3 py-1.5 text-sm text-omega-100 placeholder-omega-500 focus:outline-none focus:border-accent font-mono"
        />
        <button
          onClick={handleCheckContent}
          disabled={!contentToCheck.trim()}
          className="bg-omega-700 hover:bg-omega-600 disabled:opacity-50 text-omega-200 text-xs px-3 py-1.5 rounded transition-colors"
        >
          Check
        </button>
        {checkResult && (
          <pre className="text-xs text-omega-300 bg-omega-800 rounded p-3 mt-2 whitespace-pre-wrap font-mono">
            {checkResult}
          </pre>
        )}
      </div>

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
