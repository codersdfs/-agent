import { useState } from "react";
import { useChat } from "./hooks/useChat";
import { MessageList } from "./components/Messages";
import { InputArea } from "./components/InputArea";

export default function App() {
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const {
    messages,
    isStreaming,
    pendingPermission,
    showProviderPicker,
    sendMessage,
    respondPermission,
    onDismissPicker,
  } = useChat();

  return (
    <div className="h-screen flex bg-codex-bg text-codex-text font-mono">
      {/* ── Sidebar ─────────────────────────────────────────────────── */}
      <aside
        className={`${
          sidebarOpen ? "w-60" : "w-0 overflow-hidden"
        } shrink-0 transition-all duration-200 border-r border-codex-border bg-codex-surface flex flex-col`}
      >
        <div className="p-3 border-b border-codex-border">
          <button className="w-full flex items-center gap-2 px-3 py-2.5 text-xs rounded-lg bg-codex-input border border-codex-border text-codex-text hover:bg-[#374151] transition-colors">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <path d="M7 1v12M1 7h12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
            New Chat
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          <div className="px-3 py-2 text-[0.65em] text-codex-text-muted uppercase tracking-widest">
            Sessions
          </div>
          {/* Placeholder sessions */}
          <div className="px-3 py-2 text-xs text-codex-text-dim rounded hover:bg-codex-hover cursor-pointer transition-colors truncate">
            ~ current session
          </div>
        </div>
      </aside>

      {/* ── Main Content ────────────────────────────────────────────── */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Header */}
        <header className="flex items-center justify-between shrink-0 h-12 px-5">
          <div className="flex items-center gap-3">
            <button
              onClick={() => setSidebarOpen(!sidebarOpen)}
              className="text-codex-text-dim hover:text-codex-text transition-colors"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M2 4h12M2 8h12M2 12h12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              </svg>
            </button>
            <span className="text-sm font-bold tracking-wide text-codex-text">
              Oha Agent
            </span>
            <span className="flex items-center gap-1.5 text-[0.6em] text-codex-green">
              <span className="w-1.5 h-1.5 rounded-full bg-codex-green inline-block" />
              Online
            </span>
          </div>
          <span className="text-[0.6em] text-codex-text-muted">
            {isStreaming ? "● streaming" : "idle"}
          </span>
        </header>

        {/* Messages */}
        <MessageList
          messages={messages}
          pendingPermission={pendingPermission}
          showProviderPicker={showProviderPicker}
          onAllow={() => respondPermission(true)}
          onDeny={() => respondPermission(false)}
          onDismissPicker={onDismissPicker}
        />

        {/* Input Area - centered */}
        <div className="shrink-0 px-4 pb-10 pt-4">
          <div className="max-w-3xl mx-auto">
            <InputArea
              onSend={sendMessage}
              disabled={isStreaming}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
