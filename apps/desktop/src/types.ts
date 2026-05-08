// Shared TypeScript types matching the Rust DTOs.

export interface ChatMessage {
  id: string;
  session_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: string;
}

export interface SendMessageRequest {
  session_id: string;
  content: string;
  system_prompt?: string;
}

export interface SendMessageResponse {
  reply: string;
  session_id: string;
  context_snippets_used: number;
}

export interface MemoryEntry {
  id: string;
  content: string;
  importance_score: number;
  created_at: string;
  last_accessed_at: string;
  access_count: number;
  tags: string[];
}

export interface Entity {
  id: string;
  name: string;
  entity_type: "person" | "place" | "topic" | "object" | "event" | "other";
  description?: string;
  first_seen: string;
  last_seen: string;
}
