import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { MemoryEntry } from "../types";

export default function MemoryPanel() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<MemoryEntry[]>([]);
  const [coreMemory, setCoreMemory] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const searchMemories = async () => {
    if (!query.trim()) return;
    setLoading(true);
    try {
      const memories: MemoryEntry[] = await invoke("search_memories", {
        query: query.trim(),
        topK: 10,
      });
      setResults(memories);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const loadCoreMemory = async () => {
    try {
      const mem: string = await invoke("get_core_memory");
      setCoreMemory(mem);
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>Memory Explorer</h2>

      {/* Core memory */}
      <section style={styles.section}>
        <div style={styles.sectionHeader}>
          <h3 style={styles.sectionTitle}>🧠 Core Memory</h3>
          <button style={styles.loadBtn} onClick={loadCoreMemory}>
            Load
          </button>
        </div>
        {coreMemory && (
          <pre style={styles.coreMemoryBox}>{coreMemory}</pre>
        )}
      </section>

      {/* Semantic search */}
      <section style={styles.section}>
        <h3 style={styles.sectionTitle}>🔍 Search Semantic Memories</h3>
        <div style={styles.searchRow}>
          <input
            style={styles.searchInput}
            placeholder="Search by topic, person, event…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && searchMemories()}
          />
          <button style={styles.searchBtn} onClick={searchMemories} disabled={loading}>
            {loading ? "…" : "Search"}
          </button>
        </div>

        <div style={styles.resultsList}>
          {results.length === 0 && !loading && (
            <p style={styles.noResults}>
              {query ? "No memories found." : "Enter a query to search memories."}
            </p>
          )}
          {results.map((mem) => (
            <div key={mem.id} style={styles.memoryCard}>
              <div style={styles.memoryMeta}>
                <span style={styles.importanceBadge}>
                  ★ {(mem.importance_score * 100).toFixed(0)}%
                </span>
                <span style={styles.accessCount}>
                  accessed {mem.access_count}×
                </span>
                <span style={styles.tags}>
                  {mem.tags.map((t) => `#${t}`).join(" ")}
                </span>
              </div>
              <p style={styles.memoryContent}>{mem.content}</p>
              <span style={styles.memoryDate}>
                {new Date(mem.created_at).toLocaleDateString()}
              </span>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    flex: 1,
    overflowY: "auto",
    padding: "28px 32px",
    display: "flex",
    flexDirection: "column",
    gap: 24,
  },
  title: { fontSize: 24, fontWeight: 700, color: "var(--text)" },
  section: {
    background: "var(--surface)",
    borderRadius: "var(--radius)",
    padding: 20,
    display: "flex",
    flexDirection: "column",
    gap: 12,
  },
  sectionHeader: { display: "flex", alignItems: "center", justifyContent: "space-between" },
  sectionTitle: { fontSize: 16, fontWeight: 600, color: "var(--text)" },
  loadBtn: {
    background: "var(--surface2)",
    border: "1px solid var(--border)",
    color: "var(--text)",
    borderRadius: 8,
    padding: "6px 14px",
    cursor: "pointer",
    fontSize: 13,
  },
  coreMemoryBox: {
    background: "var(--surface2)",
    borderRadius: 8,
    padding: 16,
    fontSize: 13,
    lineHeight: 1.7,
    color: "var(--text)",
    whiteSpace: "pre-wrap",
    overflow: "auto",
    maxHeight: 300,
    fontFamily: "monospace",
  },
  searchRow: { display: "flex", gap: 10 },
  searchInput: {
    flex: 1,
    background: "var(--surface2)",
    border: "1px solid var(--border)",
    borderRadius: 8,
    padding: "10px 14px",
    color: "var(--text)",
    fontSize: 14,
    outline: "none",
    fontFamily: "inherit",
  },
  searchBtn: {
    background: "var(--accent)",
    border: "none",
    borderRadius: 8,
    color: "#fff",
    padding: "10px 20px",
    cursor: "pointer",
    fontWeight: 600,
    fontSize: 14,
  },
  resultsList: { display: "flex", flexDirection: "column", gap: 10 },
  noResults: { color: "var(--text-muted)", fontSize: 14, textAlign: "center", padding: 20 },
  memoryCard: {
    background: "var(--surface2)",
    borderRadius: 8,
    padding: "12px 16px",
    display: "flex",
    flexDirection: "column",
    gap: 6,
    border: "1px solid var(--border)",
  },
  memoryMeta: { display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap" },
  importanceBadge: {
    fontSize: 12,
    color: "#f6c90e",
    fontWeight: 600,
  },
  accessCount: { fontSize: 12, color: "var(--text-muted)" },
  tags: { fontSize: 12, color: "var(--accent)" },
  memoryContent: { fontSize: 14, lineHeight: 1.6, color: "var(--text)" },
  memoryDate: { fontSize: 11, color: "var(--text-muted)" },
};
