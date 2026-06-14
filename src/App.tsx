import { ChatPanel } from './components/ChatPanel'

function App() {
  return (
    <div className="flex h-screen w-screen bg-omega-900 overflow-hidden">
      <aside className="w-56 bg-omega-800 border-r border-omega-700 flex flex-col shrink-0">
        <div className="h-12 flex items-center px-4 border-b border-omega-700">
          <span className="text-sm font-semibold text-omega-200">Omega Agent</span>
        </div>
        <nav className="flex-1 p-2 space-y-1 overflow-y-auto">
          {['Chat', 'Plan', 'Review', 'Memory', 'Tables', 'Settings'].map((item) => (
            <button
              key={item}
              className="w-full text-left px-3 py-1.5 text-sm text-omega-300 hover:text-omega-100 hover:bg-omega-700 rounded transition-colors"
            >
              {item}
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
        <ChatPanel />
      </main>
    </div>
  )
}

export default App
