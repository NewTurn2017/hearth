import type { ChatMessage as ChatMessageType } from "../types";

export function ChatMessage({ message }: { message: ChatMessageType }) {
  const isUser = message.role === "user";

  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"} mb-2`}>
      <div
        className={`max-w-[85%] px-3 py-2 rounded-xl text-sm whitespace-pre-wrap ${
          isUser
            ? "bg-[var(--accent)] text-white rounded-br-sm"
            : "bg-[var(--bg-tertiary)] text-[var(--text-primary)] rounded-bl-sm"
        }`}
      >
        {message.content}
      </div>
    </div>
  );
}
