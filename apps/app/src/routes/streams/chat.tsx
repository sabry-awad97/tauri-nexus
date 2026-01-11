import { createFileRoute } from "@tanstack/react-router";
import { useState, useRef, useEffect } from "react";
import { subscribe, useSubscription } from "@tauri-nexus/rpc-react";
import type { ChatMessage } from "../../rpc/contract";

const ROOMS = [
  { id: "general", name: "General", icon: "💬" },
  { id: "random", name: "Random", icon: "🎲" },
  { id: "tech", name: "Tech Talk", icon: "💻" },
];

const USER_COLORS: Record<string, string> = {
  Alice: "#6366f1",
  Bob: "#8b5cf6",
  Charlie: "#ec4899",
  Diana: "#f97316",
};

function MessageBubble({
  message,
  isFirst,
}: {
  message: ChatMessage;
  isFirst: boolean;
}) {
  const color = USER_COLORS[message.userId] || "#64748b";

  return (
    <div className={`message-bubble ${isFirst ? "first" : ""}`}>
      {isFirst && (
        <div className="message-header">
          <span className="message-avatar" style={{ backgroundColor: color }}>
            {message.userId[0]}
          </span>
          <span className="message-user" style={{ color }}>
            {message.userId}
          </span>
          <span className="message-time">
            {new Date(message.timestamp).toLocaleTimeString()}
          </span>
        </div>
      )}
      <div className="message-content">{message.text}</div>
    </div>
  );
}

function ChatRoom({ roomId }: { roomId: string }) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const { isConnected, error } = useSubscription<ChatMessage>(
    async () => subscribe<ChatMessage>("stream.chat", { roomId }),
    [roomId],
    {
      onEvent: (message) => {
        setMessages((prev) => [...prev.slice(-49), message]);
      },
      autoReconnect: true,
      maxReconnects: 10,
    },
  );

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  useEffect(() => {
    setMessages([]);
  }, [roomId]);

  // Group consecutive messages from same user
  const groupedMessages = messages.reduce<
    { message: ChatMessage; isFirst: boolean }[]
  >((acc, msg, i) => {
    const prevMsg = messages[i - 1];
    const isFirst = !prevMsg || prevMsg.userId !== msg.userId;
    acc.push({ message: msg, isFirst });
    return acc;
  }, []);

  return (
    <div className="chat-room">
      <div className="chat-header">
        <div
          className={`connection-badge ${isConnected ? "connected" : "disconnected"}`}
        >
          <span className="status-dot" />
          {isConnected ? "Connected" : "Reconnecting..."}
        </div>
        <span className="message-count">{messages.length} messages</span>
      </div>

      {error && (
        <div className="chat-error">
          <span>⚠️</span> {error.message}
        </div>
      )}

      <div className="messages-container">
        {messages.length === 0 ? (
          <div className="no-messages">
            <span className="no-messages-icon">💬</span>
            <p>Waiting for messages...</p>
            <p className="hint">Messages will appear here automatically</p>
          </div>
        ) : (
          groupedMessages.map(({ message, isFirst }, i) => (
            <MessageBubble key={i} message={message} isFirst={isFirst} />
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      <div className="chat-footer">
        <input
          type="text"
          placeholder="Messages are simulated (read-only demo)"
          disabled
          className="chat-input"
        />
        <button disabled className="send-btn">
          Send
        </button>
      </div>
    </div>
  );
}

function ChatPage() {
  const [activeRoom, setActiveRoom] = useState("general");

  return (
    <div className="page stream-page chat-page">
      <header className="page-header">
        <div>
          <h1 className="page-title">💬 Chat Room</h1>
          <p className="page-subtitle">
            Real-time chat messages with auto-reconnect
          </p>
        </div>
      </header>

      <div className="chat-layout">
        <aside className="rooms-sidebar">
          <h3>Rooms</h3>
          <div className="rooms-list">
            {ROOMS.map((room) => (
              <button
                key={room.id}
                className={`room-btn ${activeRoom === room.id ? "active" : ""}`}
                onClick={() => setActiveRoom(room.id)}
              >
                <span className="room-icon">{room.icon}</span>
                <span className="room-name">{room.name}</span>
              </button>
            ))}
          </div>

          <div className="users-online">
            <h4>Simulated Users</h4>
            <div className="users-list">
              {Object.entries(USER_COLORS).map(([name, color]) => (
                <div key={name} className="user-item">
                  <span
                    className="user-dot"
                    style={{ backgroundColor: color }}
                  />
                  <span>{name}</span>
                </div>
              ))}
            </div>
          </div>
        </aside>

        <main className="chat-main">
          <ChatRoom roomId={activeRoom} />
        </main>
      </div>

      <div className="code-example">
        <h3>Code Example</h3>
        <pre>{`const { isConnected } = useSubscription<ChatMessage>(
  async () => subscribe('stream.chat', { roomId: '${activeRoom}' }),
  [roomId],
  {
    onEvent: (message) => {
      setMessages(prev => [...prev, message]);
    },
    autoReconnect: true,
    maxReconnects: 10,
  }
);`}</pre>
      </div>
    </div>
  );
}

export const Route = createFileRoute("/streams/chat")({
  component: ChatPage,
});
