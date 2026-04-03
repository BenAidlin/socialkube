"use client";

import { useState, useEffect, useRef } from "react";
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';

interface ModelInfo {
  id: string;
  layers: number;
  is_local: boolean;
}

interface Message {
  role: "user" | "assistant";
  content: string;
}

export default function Home() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [sessionId, setSessionId] = useState<string>("");

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const API_URL = "http://localhost:3001";

  // Auto-scroll logic
  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages, loading]);

  useEffect(() => {
    // Generate a random session ID on mount
    setSessionId(Math.random().toString(36).substring(7));

    fetch(`${API_URL}/api/models`)
      .then((res) => res.json())
      .then((data) => {
        setModels(data);
        if (data.length > 0) setSelectedModel(data[0].id);
      })
      .catch((err) => console.error("Failed to fetch models:", err));
  }, []);

  const sendMessage = async () => {
    if (!input.trim() || !selectedModel) return;

    const userMsg: Message = { role: "user", content: input };
    setMessages((prev) => [...prev, userMsg]);
    const currentInput = input;
    setInput("");
    setLoading(true);

    try {
      const res = await fetch(`${API_URL}/api/inference`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model_id: selectedModel,
          prompt: currentInput,
          session_id: sessionId,
        }),
      });
      const data = await res.json();
      
      const content = data.error ? `Error: ${data.error}` : data.result;
      const assistantMsg: Message = { role: "assistant", content };
      setMessages((prev) => [...prev, assistantMsg]);
    } catch (err) {
      console.error("Inference failed:", err);
      setMessages((prev) => [...prev, { role: "assistant", content: "Error: Failed to connect to node." }]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100">
      {/* Header */}
      <header className="p-4 border-b border-zinc-200 dark:border-zinc-800 flex justify-between items-center bg-white dark:bg-zinc-900">
        <h1 className="text-xl font-bold tracking-tight">SocialKube Hub</h1>
        <div className="flex items-center gap-3">
          <label className="text-sm font-medium">Model:</label>
          <select 
            value={selectedModel}
            onChange={(e) => setSelectedModel(e.target.value)}
            className="bg-zinc-100 dark:bg-zinc-800 border border-zinc-300 dark:border-zinc-700 rounded px-2 py-1 text-sm outline-none focus:ring-2 focus:ring-blue-500"
          >
            {models.map((m) => (
              <option key={m.id} value={m.id}>
                {m.id} {m.is_local ? "(Local)" : `(Swarm - ${m.layers} layers)`}
              </option>
            ))}
          </select>
          <button 
            onClick={() => {
              setMessages([]);
              setSessionId(Math.random().toString(36).substring(7));
            }}
            className="text-xs text-zinc-500 hover:text-blue-500 underline"
          >
            New Chat
          </button>
        </div>
      </header>

      {/* Chat Area */}
      <main className="flex-1 overflow-y-auto p-4 space-y-4 max-w-4xl mx-auto w-full">
        {messages.length === 0 && (
          <div className="h-full flex flex-col items-center justify-center text-zinc-400">
            <p className="text-lg">Welcome to SocialKube Swarm.</p>
            <p className="text-sm">Select a model and start chatting.</p>
          </div>
        )}
        {messages.map((msg, i) => (
          <div key={i} className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}>
            <div className={`max-w-[85%] rounded-2xl px-4 py-2 ${
              msg.role === "user" 
                ? "bg-blue-600 text-white" 
                : "bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 shadow-sm"
            }`}>
              <div className="text-sm leading-relaxed prose dark:prose-invert max-w-none">
                <ReactMarkdown 
                  remarkPlugins={[remarkGfm]}
                  components={{
                    code({node, inline, className, children, ...props}: any) {
                      const match = /language-(\w+)/.exec(className || '')
                      return !inline && match ? (
                        <SyntaxHighlighter
                          style={oneDark}
                          language={match[1]}
                          PreTag="div"
                          {...props}
                        >
                          {String(children).replace(/\n$/, '')}
                        </SyntaxHighlighter>
                      ) : (
                        <code className={className} {...props}>
                          {children}
                        </code>
                      )
                    }
                  }}
                >
                  {msg.content}
                </ReactMarkdown>
              </div>
            </div>
          </div>
        ))}
        {loading && (
          <div className="flex justify-start">
            <div className="bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-2xl px-4 py-2 shadow-sm">
              <p className="text-sm animate-pulse italic">Thinking...</p>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </main>

      {/* Input Area */}
      <footer className="p-4 bg-white dark:bg-zinc-900 border-t border-zinc-200 dark:border-zinc-800">
        <div className="max-w-4xl mx-auto flex gap-3">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    sendMessage();
                }
            }}
            rows={1}
            placeholder="Type your prompt..."
            className="flex-1 bg-zinc-100 dark:bg-zinc-800 border border-zinc-300 dark:border-zinc-700 rounded-xl px-4 py-2 outline-none focus:ring-2 focus:ring-blue-500 resize-none"
          />
          <button
            onClick={sendMessage}
            disabled={loading}
            className="bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded-xl font-medium transition-colors disabled:opacity-50"
          >
            Send
          </button>
        </div>
      </footer>
    </div>
  );
}
