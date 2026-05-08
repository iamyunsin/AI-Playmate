import { useState, useRef, useEffect, KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ChatMessage, SendMessageRequest, SendMessageResponse } from "../types";

interface Props {
  sessionId: string;
}

export default function Chat({ sessionId }: Props) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  // Scroll to bottom whenever messages change
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const sendMessage = async () => {
    const content = input.trim();
    if (!content || loading) return;

    const userMsg: ChatMessage = {
      id: crypto.randomUUID(),
      session_id: sessionId,
      role: "user",
      content,
      timestamp: new Date().toISOString(),
    };

    setMessages((prev) => [...prev, userMsg]);
    setInput("");
    setLoading(true);
    setError(null);

    try {
      const response: SendMessageResponse = await invoke("send_message", {
        request: { session_id: sessionId, content } satisfies SendMessageRequest,
      });

      const assistantMsg: ChatMessage = {
        id: crypto.randomUUID(),
        session_id: sessionId,
        role: "assistant",
        content: response.reply,
        timestamp: new Date().toISOString(),
      };

      setMessages((prev) => [...prev, assistantMsg]);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  return (
    <div style={styles.container}>
      {/* Message list */}
      <div style={styles.messageList}>
        {messages.length === 0 && (
          <div style={styles.emptyState}>
            <div style={styles.emptyIcon}>🧠</div>
            <p style={styles.emptyTitle}>Start a conversation</p>
            <p style={styles.emptySubtitle}>
              I'll remember everything you tell me and make associative connections.
            </p>
          </div>
        )}

        {messages.map((msg) => (
          <div
            key={msg.id}
            style={{
              ...styles.messageBubble,
              ...(msg.role === "user" ? styles.userBubble : styles.aiBubble),
            }}
          >
            <span style={styles.roleLabel}>
              {msg.role === "user" ? "You" : "AI Playmate"}
            </span>
            <p style={styles.messageContent}>{msg.content}</p>
          </div>
        ))}

        {loading && (
          <div style={{ ...styles.messageBubble, ...styles.aiBubble }}>
            <span style={styles.roleLabel}>AI Playmate</span>
            <span style={styles.typing}>Thinking…</span>
          </div>
        )}

        {error && (
          <div style={styles.errorBox}>⚠ {error}</div>
        )}

        <div ref={bottomRef} />
      </div>

      {/* Input area */}
      <div style={styles.inputRow}>
        <textarea
          style={styles.textarea}
          placeholder="Type a message… (Enter to send, Shift+Enter for newline)"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          rows={3}
          disabled={loading}
        />
        <button
          style={{
            ...styles.sendBtn,
            ...(loading || !input.trim() ? styles.sendBtnDisabled : {}),
          }}
          onClick={sendMessage}
          disabled={loading || !input.trim()}
        >
          Send
        </button>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    height: "100%",
    overflow: "hidden",
  },
  messageList: {
    flex: 1,
    overflowY: "auto",
    padding: "24px 32px",
    display: "flex",
    flexDirection: "column",
    gap: 16,
  },
  emptyState: {
    margin: "auto",
    textAlign: "center",
    padding: 40,
  },
  emptyIcon: { fontSize: 64, marginBottom: 16 },
  emptyTitle: { fontSize: 22, fontWeight: 700, marginBottom: 8, color: "var(--text)" },
  emptySubtitle: { fontSize: 14, color: "var(--text-muted)", maxWidth: 360, margin: "0 auto" },
  messageBubble: {
    padding: "12px 16px",
    borderRadius: "var(--radius)",
    maxWidth: "72%",
    display: "flex",
    flexDirection: "column",
    gap: 6,
  },
  userBubble: {
    alignSelf: "flex-end",
    background: "var(--user-bubble)",
    borderBottomRightRadius: 4,
  },
  aiBubble: {
    alignSelf: "flex-start",
    background: "var(--ai-bubble)",
    borderBottomLeftRadius: 4,
  },
  roleLabel: {
    fontSize: 11,
    fontWeight: 600,
    color: "var(--accent)",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
  },
  messageContent: {
    fontSize: 15,
    lineHeight: 1.6,
    color: "var(--text)",
    whiteSpace: "pre-wrap",
    wordBreak: "break-word",
  },
  typing: { color: "var(--text-muted)", fontStyle: "italic", fontSize: 14 },
  errorBox: {
    background: "#2d1515",
    border: "1px solid #7f1d1d",
    borderRadius: 8,
    padding: "10px 14px",
    color: "#fc8181",
    fontSize: 13,
  },
  inputRow: {
    padding: "16px 32px 24px",
    borderTop: "1px solid var(--border)",
    display: "flex",
    gap: 12,
    alignItems: "flex-end",
    background: "var(--surface)",
  },
  textarea: {
    flex: 1,
    background: "var(--surface2)",
    border: "1px solid var(--border)",
    borderRadius: "var(--radius)",
    color: "var(--text)",
    padding: "12px 16px",
    fontSize: 15,
    lineHeight: 1.5,
    resize: "none",
    outline: "none",
    fontFamily: "inherit",
  },
  sendBtn: {
    background: "var(--accent)",
    color: "#fff",
    border: "none",
    borderRadius: "var(--radius)",
    padding: "12px 24px",
    fontWeight: 600,
    fontSize: 15,
    cursor: "pointer",
    transition: "background 0.15s",
    whiteSpace: "nowrap",
    alignSelf: "stretch",
    display: "flex",
    alignItems: "center",
  },
  sendBtnDisabled: {
    background: "var(--surface2)",
    color: "var(--text-muted)",
    cursor: "not-allowed",
  },
};
