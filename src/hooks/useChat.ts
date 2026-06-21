import { useState, useRef, useCallback, useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Message, PermissionEvent } from "../lib/types";
import * as api from "../lib/tauri";

let uid = 0;
const nextId = () => `msg_${++uid}_${Date.now()}`;

const HELP_TEXT = `Available commands:

  **Chat** – just type a message and press Enter
  **/help** — show this help
  **/clear** — clear the conversation
  **/plan** &lt;task&gt; — generate a structured plan
  **/plan-status** — view the current plan
  **/plan-approve** — approve the plan for building
  **/build** — execute build from approved plan
  **/review** &lt;file&gt; [context] — run Gate + LLM review
  **/gate** &lt;file&gt; — run Gate checks only
  **/memory** store &lt;key&gt; &lt;value&gt; [layer]
  **/memory** search &lt;query&gt; [layer] [limit]
  **/memory** remember &lt;key&gt;
  **/memory** count
  **/memory** delete &lt;id&gt;
  **/memory** clear
  **/provider** — interactive provider & model selector
  **/model** — show current model config
  **/model** list — list available models
  **/model** &lt;name&gt; — switch to a model
  **/config** [show|set &lt;key&gt; &lt;value&gt;|providers]
  **/exit** — close the application`;

function addMsg(
  prev: Message[],
  msg: Message,
): Message[] {
  return [...prev, msg];
}

function updateLast(
  prev: Message[],
  updater: (m: Message) => Message,
): Message[] {
  const copy = [...prev];
  if (copy.length > 0) {
    copy[copy.length - 1] = updater(copy[copy.length - 1]);
  }
  return copy;
}

export interface UseChatReturn {
  messages: Message[];
  isStreaming: boolean;
  pendingPermission: PermissionEvent | null;
  showProviderPicker: boolean;
  sendMessage: (text: string) => Promise<void>;
  clearMessages: () => void;
  respondPermission: (approved: boolean) => Promise<void>;
  onDismissPicker: () => void;
}

export function useChat(): UseChatReturn {
  const [messages, setMessages] = useState<Message[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [pendingPermission, setPendingPermission] = useState<PermissionEvent | null>(null);
  const [showProviderPicker, setShowProviderPicker] = useState(false);
  const permissionRef = useRef<PermissionEvent | null>(null);

  // ── Event listeners (setup once) ──────────────────────────────────────────

  useEffect(() => {
    let unlistenToken: UnlistenFn | undefined;
    let unlistenDone: UnlistenFn | undefined;
    let unlistenError: UnlistenFn | undefined;
    let unlistenPermission: UnlistenFn | undefined;

    (async () => {
      unlistenToken = await listen<string>("chat-token", (event) => {
        setMessages((prev) => updateLast(prev, (m) => ({
          ...m,
          content: m.content + event.payload,
        })));
      });

      unlistenDone = await listen<string>("chat-done", () => {
        setIsStreaming(false);
      });

      unlistenError = await listen<string>("chat-error", (event) => {
        setIsStreaming(false);
        setMessages((prev) => addMsg(prev, {
          id: nextId(),
          type: "system",
          content: `Error: ${event.payload}`,
          timestamp: Date.now(),
        }));
      });

      unlistenPermission = await listen<PermissionEvent>("permission-request", (event) => {
        const ev = event.payload;
        permissionRef.current = ev;
        setPendingPermission(ev);
        setMessages((prev) => addMsg(prev, {
          id: nextId(),
          type: "tool",
          content: `**Permission required:** ${ev.reason}\n\nTool: \`${ev.tool}\`\nFile: \`${ev.args.filePath ?? "N/A"}\``,
          timestamp: Date.now(),
          toolCall: {
            id: ev.requestId,
            tool: ev.tool,
            args: ev.args,
          },
        }));
      });
    })();

    return () => {
      unlistenToken?.();
      unlistenDone?.();
      unlistenError?.();
      unlistenPermission?.();
    };
  }, []);

  // ── Send / Slash commands ─────────────────────────────────────────────────

  const sendMessage = useCallback(async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;

    // Add user message
    setMessages((prev) => addMsg(prev, {
      id: nextId(),
      type: "user",
      content: trimmed,
      timestamp: Date.now(),
    }));

    // Slash commands
    if (trimmed.startsWith("/")) {
      const parts = trimmed.split(/\s+/);
      const cmd = parts[0].toLowerCase();
      const rest = parts.slice(1).join(" ");

      switch (cmd) {
        case "/help":
        case "/h": {
          setMessages((prev) => addMsg(prev, {
            id: nextId(),
            type: "system",
            content: HELP_TEXT,
            timestamp: Date.now(),
          }));
          return;
        }

        case "/clear": {
          setMessages([]);
          return;
        }

        case "/exit":
        case "/quit":
        case "/q": {
          setMessages((prev) => addMsg(prev, {
            id: nextId(),
            type: "system",
            content: "Goodbye.",
            timestamp: Date.now(),
          }));
          return;
        }

        case "/plan":
        case "/p": {
          if (!rest) {
            setMessages((prev) => addMsg(prev, {
              id: nextId(),
              type: "system",
              content: "Usage: `/plan <task description>`",
              timestamp: Date.now(),
            }));
            return;
          }
          setMessages((prev) => addMsg(prev, {
            id: nextId(),
            type: "assistant",
            content: "_Generating plan..._",
            timestamp: Date.now(),
          }));
          try {
            const payload = await api.generatePlan(rest);
            setMessages((prev) => updateLast(prev, () => ({
              id: nextId(),
              type: "assistant",
              content: `**Plan: ${payload.plan.taskSummary}**\n\n` +
                `Language: ${payload.plan.language} | Complexity: ${payload.plan.estimatedComplexity} | Risk: ${payload.plan.riskLevel}\n\n` +
                payload.plan.steps.map((s) =>
                  `### Step #${s.id}: ${s.action}\n${s.description}` +
                  (s.filePath ? `\n\`${s.filePath}\`` : "")
                ).join("\n\n"),
              timestamp: Date.now(),
            })));
          } catch (e) {
            setMessages((prev) => updateLast(prev, () => ({
              id: nextId(),
              type: "system",
              content: `Plan error: ${e}`,
              timestamp: Date.now(),
            })));
          }
          return;
        }

        case "/plan-status": {
          try {
            const plan = await api.getPlan();
            if (plan) {
              setMessages((prev) => addMsg(prev, {
                id: nextId(),
                type: "system",
                content: `**Current Plan:**\n\nTask: ${plan.taskSummary}\nLanguage: ${plan.language}\nSteps: ${plan.steps.length}`,
                timestamp: Date.now(),
              }));
            } else {
              setMessages((prev) => addMsg(prev, {
                id: nextId(),
                type: "system",
                content: "No plan has been generated yet.",
                timestamp: Date.now(),
              }));
            }
          } catch (e) {
            setMessages((prev) => addMsg(prev, {
              id: nextId(),
              type: "system",
              content: `Error: ${e}`,
              timestamp: Date.now(),
            }));
          }
          return;
        }

        case "/plan-approve":
        case "/approve": {
          try {
            const msg = await api.approvePlan();
            setMessages((prev) => addMsg(prev, {
              id: nextId(),
              type: "system",
              content: msg,
              timestamp: Date.now(),
            }));
          } catch (e) {
            setMessages((prev) => addMsg(prev, {
              id: nextId(),
              type: "system",
              content: `Error: ${e}`,
              timestamp: Date.now(),
            }));
          }
          return;
        }

        case "/build":
        case "/b": {
          const autoApprove = rest === "auto";
          setMessages((prev) => addMsg(prev, {
            id: nextId(),
            type: "assistant",
            content: "_Running build..._",
            timestamp: Date.now(),
          }));
          try {
            if (autoApprove) {
              await api.setBuildConfig(true);
            }
            const session = await api.executeBuild();
            const completed = session.filter((e) => e.success).length;
            const failed = session.filter((e) => !e.success).length;
            setMessages((prev) => updateLast(prev, () => ({
              id: nextId(),
              type: "assistant",
              content: `**Build complete:** ${completed} succeeded, ${failed} failed\n\n` +
                session.map((e) =>
                  `- Step ${e.stepIndex} (\`${e.tool}\`): ${e.success ? "✓" : "✗"} ${e.durationMs}ms`
                ).join("\n"),
              timestamp: Date.now(),
            })));
            if (autoApprove) {
              await api.setBuildConfig(false);
            }
          } catch (e) {
            setMessages((prev) => updateLast(prev, () => ({
              id: nextId(),
              type: "system",
              content: `Build error: ${e}`,
              timestamp: Date.now(),
            })));
          }
          return;
        }

        case "/review":
        case "/r": {
          if (!rest) {
            setMessages((prev) => addMsg(prev, {
              id: nextId(),
              type: "system",
              content: "Usage: `/review <file> [context]`",
              timestamp: Date.now(),
            }));
            return;
          }
          const [filePath, ...ctxParts] = rest.split(" ");
          const ctx = ctxParts.join(" ");
          setMessages((prev) => addMsg(prev, {
            id: nextId(),
            type: "assistant",
            content: `_Reviewing ${filePath}..._`,
            timestamp: Date.now(),
          }));
          try {
            const result = await api.runReview({ code: filePath, context: ctx });
            setMessages((prev) => updateLast(prev, () => ({
              id: nextId(),
              type: "assistant",
              content:
                `**Review Score:** ${result.scoreBreakdown.combinedScore} (passed: ${result.scoreBreakdown.passed})\n\n` +
                (result.scoreBreakdown.llmIssues.length > 0
                  ? result.scoreBreakdown.llmIssues.map((i) =>
                    `- [${i.severity}] ${i.category}: ${i.description}`
                  ).join("\n")
                  : "No issues found."),
              timestamp: Date.now(),
            })));
          } catch (e) {
            setMessages((prev) => updateLast(prev, () => ({
              id: nextId(),
              type: "system",
              content: `Review error: ${e}`,
              timestamp: Date.now(),
            })));
          }
          return;
        }

        case "/gate":
        case "/g": {
          if (!rest) {
            setMessages((prev) => addMsg(prev, {
              id: nextId(),
              type: "system",
              content: "Usage: `/gate <file>`",
              timestamp: Date.now(),
            }));
            return;
          }
          setMessages((prev) => addMsg(prev, {
            id: nextId(),
            type: "assistant",
            content: `_Gate checking ${rest}..._`,
            timestamp: Date.now(),
          }));
          try {
            const result = await api.checkGate({
              content: rest,
              context: `checking ${rest}`,
            });
            setMessages((prev) => updateLast(prev, () => ({
              id: nextId(),
              type: "assistant",
              content:
                `**Gate Score:** ${result.score} (passed: ${result.passed})\n\n` +
                (result.violations.length > 0
                  ? result.violations.map((v) =>
                    `- [${v.category}] ${v.message}`
                  ).join("\n")
                  : "No violations found."),
              timestamp: Date.now(),
            })));
          } catch (e) {
            setMessages((prev) => updateLast(prev, () => ({
              id: nextId(),
              type: "system",
              content: `Gate error: ${e}`,
              timestamp: Date.now(),
            })));
          }
          return;
        }

        case "/memory":
        case "/m": {
          const memParts = rest.split(/\s+/);
          const memCmd = memParts[0];
          const memArgs = memParts.slice(1);
          try {
            switch (memCmd) {
              case "store": {
                if (memArgs.length < 2) {
                  throw new Error("Usage: /memory store <key> <value> [layer]");
                }
                const layer = memArgs[2] ?? "session";
                const result = await api.memoryStore({
                  key: memArgs[0],
                  value: memArgs[1],
                  layer,
                });
                setMessages((prev) => addMsg(prev, {
                  id: nextId(),
                  type: "system",
                  content: result,
                  timestamp: Date.now(),
                }));
                return;
              }
              case "search": {
                if (memArgs.length < 1) {
                  throw new Error("Usage: /memory search <query> [layer] [limit]");
                }
                const res = await api.memorySearch({
                  query: memArgs[0],
                  layer: memArgs[1] ?? null,
                  limit: memArgs[2] ? parseInt(memArgs[2], 10) : 10,
                });
                if (res.entries.length === 0) {
                  setMessages((prev) => addMsg(prev, {
                    id: nextId(),
                    type: "system",
                    content: "No results found.",
                    timestamp: Date.now(),
                  }));
                } else {
                  setMessages((prev) => addMsg(prev, {
                    id: nextId(),
                    type: "system",
                    content: res.entries.map((e, i) =>
                      `[${i + 1}] **${e.key}** (layer: ${e.layer}, relevance: ${(res.relevance[i] * 100).toFixed(0)}%)\n\`${e.value}\``
                    ).join("\n\n"),
                    timestamp: Date.now(),
                  }));
                }
                return;
              }
              case "remember": {
                if (memArgs.length < 1) {
                  throw new Error("Usage: /memory remember <key>");
                }
                const val = await api.memoryRemember(memArgs[0]);
                setMessages((prev) => addMsg(prev, {
                  id: nextId(),
                  type: "system",
                  content: val ?? "Not found.",
                  timestamp: Date.now(),
                }));
                return;
              }
              case "count": {
                const count = await api.memoryCount();
                setMessages((prev) => addMsg(prev, {
                  id: nextId(),
                  type: "system",
                  content: `${count} entries`,
                  timestamp: Date.now(),
                }));
                return;
              }
              case "delete": {
                if (memArgs.length < 1) {
                  throw new Error("Usage: /memory delete <id>");
                }
                await api.memoryDelete(memArgs[0]);
                setMessages((prev) => addMsg(prev, {
                  id: nextId(),
                  type: "system",
                  content: "Deleted.",
                  timestamp: Date.now(),
                }));
                return;
              }
              case "clear": {
                const cleared = await api.memoryClear();
                setMessages((prev) => addMsg(prev, {
                  id: nextId(),
                  type: "system",
                  content: `Cleared ${cleared} entries.`,
                  timestamp: Date.now(),
                }));
                return;
              }
              default:
                throw new Error("Unknown memory subcommand. Try: store, search, remember, count, delete, clear");
            }
          } catch (e) {
            setMessages((prev) => addMsg(prev, {
              id: nextId(),
              type: "system",
              content: `Error: ${e}`,
              timestamp: Date.now(),
            }));
          }
          return;
        }

        case "/model": {
          const modelParts = rest.split(/\s+/);
          const modelCmd = modelParts[0];

          try {
            if (!modelCmd) {
              // Show current model config
              const cfg = await api.getProviderConfig();
              setMessages((prev) => addMsg(prev, {
                id: nextId(),
                type: "system",
                content:
                  `**Current Model:** ${cfg.model}\n` +
                  `Provider: ${cfg.provider}\n` +
                  `Max tokens: ${cfg.max_tokens}\n` +
                  `Temperature: ${cfg.temperature}\n` +
                  `API key: ${cfg.api_key || "(not set)"}`,
                timestamp: Date.now(),
              }));
              return;
            }

            if (modelCmd === "list") {
              const models = await api.listModels();
              setMessages((prev) => addMsg(prev, {
                id: nextId(),
                type: "system",
                content:
                  `**Available models:**\n\n` +
                  models.map((m) => `- ${m}`).join("\n"),
                timestamp: Date.now(),
              }));
              return;
            }

            // Treat remaining as model name to switch to
            const newModel = modelParts.join(" ");
            const msg = await api.setProviderConfig("model", newModel);
            setMessages((prev) => addMsg(prev, {
              id: nextId(),
              type: "system",
              content: msg,
              timestamp: Date.now(),
            }));
          } catch (e) {
            setMessages((prev) => addMsg(prev, {
              id: nextId(),
              type: "system",
              content: `Error: ${e}`,
              timestamp: Date.now(),
            }));
          }
          return;
        }

        case "/provider": {
          setShowProviderPicker(true);
          return;
        }

        case "/config": {
          const configParts = rest.split(/\s+/);
          const configCmd = configParts[0] || "show";
          try {
            switch (configCmd) {
              case "show": {
                const tools = await api.listTools();
                setMessages((prev) => addMsg(prev, {
                  id: nextId(),
                  type: "system",
                  content: `**Configuration**\n\nAvailable tools: ${tools.join(", ")}`,
                  timestamp: Date.now(),
                }));
                return;
              }
              case "providers": {
                const models = await api.listModels();
                setMessages((prev) => addMsg(prev, {
                  id: nextId(),
                  type: "system",
                  content: `**Current model list:**\n\n${models.map((m) => `- ${m}`).join("\n")}`,
                  timestamp: Date.now(),
                }));
                return;
              }
              default:
                setMessages((prev) => addMsg(prev, {
                  id: nextId(),
                  type: "system",
                  content: "Usage: /config [show|providers]",
                  timestamp: Date.now(),
                }));
            }
          } catch (e) {
            setMessages((prev) => addMsg(prev, {
              id: nextId(),
              type: "system",
              content: `Error: ${e}`,
              timestamp: Date.now(),
            }));
          }
          return;
        }

        default: {
          setMessages((prev) => addMsg(prev, {
            id: nextId(),
            type: "system",
            content: `Unknown command: \`${cmd}\`. Type \`/help\` for available commands.`,
            timestamp: Date.now(),
          }));
          return;
        }
      }
    }

    // Plain text — streaming chat
    setIsStreaming(true);
    setMessages((prev) => addMsg(prev, {
      id: nextId(),
      type: "assistant",
      content: "",
      timestamp: Date.now(),
    }));

    try {
      await api.streamMessage({
        content: trimmed,
        agentType: "plan",
        systemPrompt: null,
      });
    } catch (e) {
      setIsStreaming(false);
      setMessages((prev) => updateLast(prev, (m) => ({
        ...m,
        content: m.content || `Error: ${e}`,
      })));
    }
  }, []);

  const clearMessages = useCallback(() => {
    setMessages([]);
  }, []);

  const respondPermission = useCallback(async (approved: boolean) => {
    const ev = permissionRef.current;
    if (!ev) return;
    try {
      await api.respondPermission(ev.requestId, approved);
      setMessages((prev) => addMsg(prev, {
        id: nextId(),
        type: "system",
        content: approved ? `Approved: ${ev.reason}` : `Denied: ${ev.reason}`,
        timestamp: Date.now(),
      }));
    } catch (e) {
      setMessages((prev) => addMsg(prev, {
        id: nextId(),
        type: "system",
        content: `Permission error: ${e}`,
        timestamp: Date.now(),
      }));
    }
    permissionRef.current = null;
    setPendingPermission(null);
  }, []);

  const onDismissPicker = useCallback(() => {
    setShowProviderPicker(false);
  }, []);

  return {
    messages,
    isStreaming,
    pendingPermission,
    showProviderPicker,
    sendMessage,
    clearMessages,
    respondPermission,
    onDismissPicker,
  };
}
