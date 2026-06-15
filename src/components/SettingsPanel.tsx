import { useChatStore } from '../stores/chatStore'

const PROVIDER_OPTIONS = [
  { id: 'openai', label: 'OpenAI' },
  { id: 'anthropic', label: 'Anthropic' },
  { id: 'google', label: 'Google Gemini' },
  { id: 'mistral', label: 'Mistral' },
  { id: 'xai', label: 'xAI (Grok)' },
  { id: 'groq', label: 'Groq' },
  { id: 'cerebras', label: 'Cerebras' },
  { id: 'openrouter', label: 'OpenRouter' },
  { id: 'local', label: 'Local' },
] as const

export function SettingsPanel() {
  const providerConfig = useChatStore((s) => s.providerConfig)
  const setProviderConfig = useChatStore((s) => s.setProviderConfig)

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-4">
      <h2 className="text-sm font-semibold text-omega-200">Provider Settings</h2>

      <div className="space-y-3">
        <div>
          <label className="block text-xs text-omega-400 mb-1">Provider</label>
          <select
            value={providerConfig.provider}
            onChange={(e) => setProviderConfig({ provider: e.target.value })}
            className="w-full bg-omega-800 border border-omega-600 rounded px-3 py-1.5 text-sm text-omega-100 focus:outline-none focus:border-accent"
          >
            {PROVIDER_OPTIONS.map((p) => (
              <option key={p.id} value={p.id}>{p.label}</option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-xs text-omega-400 mb-1">Model</label>
          <input
            type="text"
            value={providerConfig.model}
            onChange={(e) => setProviderConfig({ model: e.target.value })}
            placeholder="gpt-4o"
            className="w-full bg-omega-800 border border-omega-600 rounded px-3 py-1.5 text-sm text-omega-100 placeholder-omega-500 focus:outline-none focus:border-accent"
          />
        </div>

        <div>
          <label className="block text-xs text-omega-400 mb-1">API Key</label>
          <input
            type="password"
            value={providerConfig.api_key}
            onChange={(e) => setProviderConfig({ api_key: e.target.value })}
            placeholder="sk-..."
            className="w-full bg-omega-800 border border-omega-600 rounded px-3 py-1.5 text-sm text-omega-100 placeholder-omega-500 focus:outline-none focus:border-accent"
          />
        </div>

        <div>
          <label className="block text-xs text-omega-400 mb-1">Base URL (optional)</label>
          <input
            type="text"
            value={providerConfig.base_url}
            onChange={(e) => setProviderConfig({ base_url: e.target.value })}
            placeholder="https://api.openai.com/v1"
            className="w-full bg-omega-800 border border-omega-600 rounded px-3 py-1.5 text-sm text-omega-100 placeholder-omega-500 focus:outline-none focus:border-accent"
          />
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="block text-xs text-omega-400 mb-1">Max Tokens</label>
            <input
              type="number"
              value={providerConfig.max_tokens}
              onChange={(e) => setProviderConfig({ max_tokens: parseInt(e.target.value) || 4096 })}
              className="w-full bg-omega-800 border border-omega-600 rounded px-3 py-1.5 text-sm text-omega-100 focus:outline-none focus:border-accent"
            />
          </div>
          <div>
            <label className="block text-xs text-omega-400 mb-1">Temperature</label>
            <input
              type="number"
              min="0"
              max="2"
              step="0.1"
              value={providerConfig.temperature}
              onChange={(e) => setProviderConfig({ temperature: parseFloat(e.target.value) || 0.7 })}
              className="w-full bg-omega-800 border border-omega-600 rounded px-3 py-1.5 text-sm text-omega-100 focus:outline-none focus:border-accent"
            />
          </div>
        </div>
      </div>

      <div className="border-t border-omega-700 pt-4 mt-4">
        <h3 className="text-xs font-semibold text-omega-300 mb-2">Pipeline</h3>
        <div className="grid grid-cols-2 gap-2 text-xs text-omega-400">
          <div className="bg-omega-800 rounded px-3 py-2">
            <div className="font-medium text-omega-200">Max Retries</div>
            <div>3 (per stage)</div>
          </div>
          <div className="bg-omega-800 rounded px-3 py-2">
            <div className="font-medium text-omega-200">Pass Threshold</div>
            <div>Score ≥ 80</div>
          </div>
        </div>
      </div>

      <div className="pt-2 text-xs text-omega-500">
        <p>API keys are stored in memory only and never persisted to disk.</p>
      </div>
    </div>
  )
}
