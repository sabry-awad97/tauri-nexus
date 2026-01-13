import { createFileRoute } from "@tanstack/react-router";
import { useState, useRef, useEffect } from "react";
import { subscribe, useSubscription } from "@tauri-nexus/rpc-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { ChatMessage } from "@/generated/schemas";

const ROOMS = [
  { id: "general", name: "General", icon: "💬" },
  { id: "random", name: "Random", icon: "🎲" },
  { id: "tech", name: "Tech Talk", icon: "💻" },
];

const USER_COLORS: Record<string, string> = {
  Alice: "bg-indigo-500",
  Bob: "bg-violet-500",
  Charlie: "bg-pink-500",
  Diana: "bg-orange-500",
};

function MessageBubble({
  message,
  isFirst,
}: {
  message: ChatMessage;
  isFirst: boolean;
}) {
  const colorClass = USER_COLORS[message.userId] || "bg-slate-500";

  return (
    <div className={`${isFirst ? "mt-4 first:mt-0" : "mt-1"}`}>
      {isFirst && (
        <div className="flex items-center gap-2 mb-1">
          <Avatar className="size-6">
            <AvatarFallback className={`${colorClass} text-white text-xs`}>
              {message.userId[0]}
            </AvatarFallback>
          </Avatar>
          <span className="text-sm font-semibold">{message.userId}</span>
          <span className="text-xs text-muted-foreground">
            {new Date(message.timestamp).toLocaleTimeString()}
          </span>
        </div>
      )}
      <div className="inline-block px-3 py-2 rounded-lg bg-muted text-sm max-w-[80%]">
        {message.text}
      </div>
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

  const groupedMessages = messages.reduce<
    { message: ChatMessage; isFirst: boolean }[]
  >((acc, msg, i) => {
    const prevMsg = messages[i - 1];
    const isFirst = !prevMsg || prevMsg.userId !== msg.userId;
    acc.push({ message: msg, isFirst });
    return acc;
  }, []);

  return (
    <Card className="flex flex-col h-full">
      <CardHeader className="flex-row items-center justify-between space-y-0 border-b py-3">
        <Badge
          variant={isConnected ? "default" : "secondary"}
          className="gap-2"
        >
          <span
            className={`size-2 rounded-full ${isConnected ? "bg-green-500" : "bg-muted-foreground"}`}
          />
          {isConnected ? "Connected" : "Reconnecting..."}
        </Badge>
        <span className="text-xs text-muted-foreground">
          {messages.length} messages
        </span>
      </CardHeader>

      {error && (
        <div className="px-4 py-2 bg-destructive/10 text-destructive text-sm flex items-center gap-2">
          <span>⚠️</span> {error.message}
        </div>
      )}

      <CardContent className="flex-1 p-0 overflow-hidden">
        <ScrollArea className="h-[400px] p-4">
          {messages.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
              <span className="text-5xl mb-4">💬</span>
              <p>Waiting for messages...</p>
              <p className="text-xs">Messages will appear here automatically</p>
            </div>
          ) : (
            <div>
              {groupedMessages.map(({ message, isFirst }, i) => (
                <MessageBubble key={i} message={message} isFirst={isFirst} />
              ))}
              <div ref={messagesEndRef} />
            </div>
          )}
        </ScrollArea>
      </CardContent>

      <div className="p-4 border-t flex gap-3">
        <Input
          placeholder="Messages are simulated (read-only demo)"
          disabled
          className="flex-1"
        />
        <Button disabled>Send</Button>
      </div>
    </Card>
  );
}

function ChatPage() {
  const [activeRoom, setActiveRoom] = useState("general");

  return (
    <div className="p-8 max-w-6xl mx-auto space-y-8">
      <header>
        <h1 className="text-3xl font-bold mb-2">💬 Chat Room</h1>
        <p className="text-muted-foreground">
          Real-time chat messages with auto-reconnect
        </p>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-[200px_1fr] gap-6 min-h-[500px]">
        <Card className="h-fit">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Rooms</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1">
            {ROOMS.map((room) => (
              <Button
                key={room.id}
                variant={activeRoom === room.id ? "default" : "ghost"}
                className="w-full justify-start gap-2"
                onClick={() => setActiveRoom(room.id)}
              >
                <span>{room.icon}</span>
                <span>{room.name}</span>
              </Button>
            ))}

            <div className="pt-4 mt-4 border-t">
              <p className="text-xs font-semibold text-muted-foreground mb-3">
                Simulated Users
              </p>
              <div className="space-y-2">
                {Object.entries(USER_COLORS).map(([name, colorClass]) => (
                  <div
                    key={name}
                    className="flex items-center gap-2 text-sm text-muted-foreground"
                  >
                    <span className={`size-2 rounded-full ${colorClass}`} />
                    <span>{name}</span>
                  </div>
                ))}
              </div>
            </div>
          </CardContent>
        </Card>

        <ChatRoom roomId={activeRoom} />
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm text-muted-foreground">
            Code Example
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="bg-muted rounded-lg p-4 overflow-x-auto">
            <pre className="text-xs font-mono text-muted-foreground">{`const { isConnected } = useSubscription<ChatMessage>(
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
        </CardContent>
      </Card>
    </div>
  );
}

export const Route = createFileRoute("/streams/chat")({
  component: ChatPage,
});
