import { useState, useRef, useEffect } from 'react'
import { useChatStore } from '../stores/chatStore'
import type { Message } from '../stores/chatStore'

function MessageBubble({ msg }: { msg: Message }) {
  const hasViolations = msg.gate_violations && msg.gate_violations.length > 0
  return (
    <div className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`${
          hasViolations ? '' : 'max-w-[75%]'
        } rounded-lg px-4 py-2 text-sm leading-relaxed ${
          msg.role === 'user'
            ? 'bg-accent text-white max-w-[75%]'
            : msg.role === 'system'
            ? 'bg-omega-700 text-omega-300 italic text-xs max-w-[85%]'
            : 'bg-omega-700 text-omega-100 max-w-[85%]'
        }`}
      >
        <div className="whitespace-pre-wrap">{msg.content}</div>
        {msg.gate_violations && msg.gate_violations.length > 0 && (
          <div className="mt-2 pt-2 border-t border-omega-600/50 space-y-1">
            <div className="text-[10px] text-omega-400 font-medium uppercase tracking-wider">Gate Violations</div>
            {msg.gate_violations.map((v, i) => (
              <div key={i} className="text-[11px] font-mono">
                <span className={`${
                  v.category === 'Golden' ? 'text-red' : v.category === 'Structural' ? 'text-blue' : 'text-yellow'
                }`}>
                  [{v.category}]
                </span>{' '}
                <span className="text-omega-300">{v.message}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

export function ChatPanel() {
  const [input, setInput] = useState('')
  const [activeAgent, setActiveAgent] = useState('plan')
  const endRef = useRef<HTMLDivElement>(null)

  const messages = useChatStore((s) => s.messages)
  const isProcessing = useChatStore((s) => s.isProcessing)
  const streamedContent = useChatStore((s) => s.streamedContent)
  const pipelineStatus = useChatStore((s) => s.pipelineStatus)
  const sendMessage = useChatStore((s) => s.sendMessage)
  const setupStreaming = useChatStore((s) => s.setupStreaming)
  const teardownStreaming = useChatStore((s) => s.teardownStreaming)

  useEffect(() => {
    setupStreaming()
    return () => { teardownStreaming() }
  }, [setupStreaming, teardownStreaming])

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, streamedContent])

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!input.trim() || isProcessing) return
    const content = input.trim()
    setInput('')
    await sendMessage(content, activeAgent)
  }

  const agents = [
    { id: 'plan', label: 'Plan', color: 'bg-blue' },
    { id: 'build', label: 'Build', color: 'bg-green' },
    { id: 'review', label: 'Review', color: 'bg-yellow' },
  ] as const

  function statusBadge(status: string) {
    const colors: Record<string, string> = {
      Idle: 'text-green',
      Planning: 'text-blue',
      Building: 'text-green',
      Reviewing: 'text-yellow',
      Failed: 'text-red',
      Completed: 'text-green',
    }
    return (
      <span className={`text-[10px] font-mono ${colors[status] || 'text-omega-400'}`}>
        ● {status}
      </span>
    )
  }

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="shrink-0 h-7 flex items-center px-4 border-b border-omega-700 bg-omega-800/50">
        <div className="flex items-center gap-3">
          {statusBadge(pipelineStatus)}
          <span className="text-[10px] text-omega-500">
            Agent: <span className="text-omega-300 capitalize">{activeAgent}</span>
          </span>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {messages.map((msg) => (
          <MessageBubble key={msg.id} msg={msg} />
        ))}

        {streamedContent && (
          <div className="flex justify-start">
            <div className="bg-omega-700 rounded-lg px-4 py-2 text-sm text-omega-100">
              {streamedContent}
              <span className="inline-block w-1.5 h-4 bg-accent ml-0.5 animate-pulse" />
            </div>
          </div>
        )}

        {isProcessing && !streamedContent && (
          <div className="flex justify-start">
            <div className="bg-omega-700 rounded-lg px-4 py-2 text-sm text-omega-400">
              <span className="inline-flex gap-1">
                <span className="animate-pulse">.</span>
                <span className="animate-pulse delay-100">.</span>
                <span className="animate-pulse delay-200">.</span>
              </span>
            </div>
          </div>
        )}

        <div ref={endRef} />
      </div>

      <form onSubmit={handleSubmit} className="shrink-0 border-t border-omega-700 p-3">
        <div className="flex gap-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Describe what you want to build..."
            disabled={isProcessing}
            className="flex-1 bg-omega-800 border border-omega-600 rounded-lg px-4 py-2 text-sm text-omega-100 placeholder-omega-500 focus:outline-none focus:border-accent transition-colors disabled:opacity-50"
          />
          <button
            type="submit"
            disabled={isProcessing || !input.trim()}
            className="bg-accent hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg px-5 py-2 text-sm font-medium transition-colors"
          >
            Send
          </button>
        </div>
        <div className="flex gap-3 mt-2">
          {agents.map((agent) => (
            <button
              key={agent.id}
              type="button"
              onClick={() => setActiveAgent(agent.id)}
              className={`text-xs px-2 py-0.5 rounded transition-colors ${
                activeAgent === agent.id
                  ? `${agent.color} text-white`
                  : 'text-omega-400 hover:text-omega-200'
              }`}
            >
              {agent.label}
            </button>
          ))}
        </div>
      </form>
    </div>
  )
}
