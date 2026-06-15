import { useState } from 'react'
import { ChatPanel } from './components/ChatPanel'
import { SettingsPanel } from './components/SettingsPanel'
import { ReviewPanel } from './components/ReviewPanel'

type View = 'chat' | 'review' | 'settings'

export default function App() {
  const [activeView, setActiveView] = useState<View>('chat')

  const views: { id: View; label: string }[] = [
    { id: 'chat', label: 'Chat' },
    { id: 'review', label: 'Review' },
    { id: 'settings', label: 'Settings' },
  ]

  return (
    <div className="flex h-screen w-screen bg-omega-900 overflow-hidden">
      <aside className="w-48 bg-omega-800 border-r border-omega-700 flex flex-col shrink-0">
        <div className="h-11 flex items-center px-4 border-b border-omega-700">
          <span className="text-sm font-semibold text-omega-200">Omega Agent</span>
        </div>
        <nav className="flex-1 p-2 space-y-1 overflow-y-auto">
          {views.map((v) => (
            <button
              key={v.id}
              onClick={() => setActiveView(v.id)}
              className={`w-full text-left px-3 py-1.5 text-sm rounded transition-colors ${
                activeView === v.id
                  ? 'text-omega-100 bg-omega-700'
                  : 'text-omega-400 hover:text-omega-200 hover:bg-omega-700/50'
              }`}
            >
              {v.label}
            </button>
          ))}
        </nav>
        <div className="p-3 border-t border-omega-700">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-green" />
            <span className="text-xs text-omega-400">Ready</span>
          </div>
        </div>
      </aside>

      <main className="flex-1 flex flex-col min-w-0">
        {activeView === 'chat' && <ChatPanel />}
        {activeView === 'review' && <ReviewPanel />}
        {activeView === 'settings' && <SettingsPanel />}
      </main>
    </div>
  )
}
