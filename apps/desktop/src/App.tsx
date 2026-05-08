import { useState } from "react";
import Chat from "./components/Chat";
import MemoryPanel from "./components/MemoryPanel";
import { v4 as uuidv4 } from "uuid";

type Panel = "chat" | "memory";

export default function App() {
  const [sessionId] = useState<string>(uuidv4());
  const [activePanel, setActivePanel] = useState<Panel>("chat");

  return (
    <div style={styles.root}>
      {/* Sidebar */}
      <aside style={styles.sidebar}>
        <div style={styles.logo}>
          <span style={styles.logoIcon}>🧠</span>
          <span style={styles.logoText}>AI Playmate</span>
        </div>
        <nav style={styles.nav}>
          <button
            style={{ ...styles.navBtn, ...(activePanel === "chat" ? styles.navBtnActive : {}) }}
            onClick={() => setActivePanel("chat")}
          >
            💬 Chat
          </button>
          <button
            style={{ ...styles.navBtn, ...(activePanel === "memory" ? styles.navBtnActive : {}) }}
            onClick={() => setActivePanel("memory")}
          >
            🗃 Memories
          </button>
        </nav>
      </aside>

      {/* Main content */}
      <main style={styles.main}>
        {activePanel === "chat" && <Chat sessionId={sessionId} />}
        {activePanel === "memory" && <MemoryPanel />}
      </main>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  root: {
    display: "flex",
    height: "100vh",
    background: "var(--bg)",
  },
  sidebar: {
    width: 220,
    flexShrink: 0,
    background: "var(--surface)",
    borderRight: "1px solid var(--border)",
    display: "flex",
    flexDirection: "column",
    padding: "20px 12px",
    gap: 8,
  },
  logo: {
    display: "flex",
    alignItems: "center",
    gap: 10,
    padding: "0 8px 16px",
    borderBottom: "1px solid var(--border)",
    marginBottom: 8,
  },
  logoIcon: { fontSize: 24 },
  logoText: { fontWeight: 700, fontSize: 16, color: "var(--text)" },
  nav: { display: "flex", flexDirection: "column", gap: 4 },
  navBtn: {
    background: "none",
    border: "none",
    color: "var(--text-muted)",
    borderRadius: 8,
    padding: "10px 12px",
    textAlign: "left",
    cursor: "pointer",
    fontSize: 14,
    fontWeight: 500,
    transition: "all 0.15s",
  },
  navBtnActive: {
    background: "var(--surface2)",
    color: "var(--text)",
  },
  main: {
    flex: 1,
    overflow: "hidden",
    display: "flex",
    flexDirection: "column",
  },
};
