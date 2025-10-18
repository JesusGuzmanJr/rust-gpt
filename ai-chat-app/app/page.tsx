"use client"

import type React from "react"
import { useState, useEffect, useRef } from "react"
import { useRouter } from "next/navigation" // Added useRouter import for navigation
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { Slider } from "@/components/ui/slider"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  ArrowRight,
  Plus,
  MessageSquare,
  Menu,
  X,
  Trash2,
  Pencil,
  ThumbsUp,
  ThumbsDown,
  Square,
  Check,
  Settings,
  Info,
} from "lucide-react"

type Message = {
  id: string
  content: string
  role: "system" | "user"
  timestamp: Date
}

type Chat = {
  id: string
  title: string
  lastMessage: string
  timestamp: Date
}

export default function ChatPage() {
  const router = useRouter() // Added router for navigation
  const [isSidebarOpen, setIsSidebarOpen] = useState(false)
  const [activeChatId, setActiveChatId] = useState<string>("1")
  const [recentChats, setRecentChats] = useState<Chat[]>([
    {
      id: "1",
      title: "Project Help",
      lastMessage: "I need help with my project.",
      timestamp: new Date(Date.now() - 3600000),
    },
    {
      id: "2",
      title: "Code Review",
      lastMessage: "Can you review this code?",
      timestamp: new Date(Date.now() - 7200000),
    },
    {
      id: "3",
      title: "Design Feedback",
      lastMessage: "What do you think about this design?",
      timestamp: new Date(Date.now() - 86400000),
    },
  ])

  const [swipedChatId, setSwipedChatId] = useState<string | null>(null)
  const [swipeOffset, setSwipeOffset] = useState(0)
  const [isDragging, setIsDragging] = useState(false)
  const [startX, setStartX] = useState(0)
  const [currentDragChatId, setCurrentDragChatId] = useState<string | null>(null)
  const [deletingChatId, setDeletingChatId] = useState<string | null>(null)

  const [messages, setMessages] = useState<Message[]>([
    {
      id: "1",
      content: "Hello! How can I assist you today?",
      role: "system",
      timestamp: new Date(Date.now() - 300000),
    },
    {
      id: "2",
      content: "I need help with my project.",
      role: "user",
      timestamp: new Date(Date.now() - 240000),
    },
    {
      id: "3",
      content: "I'd be happy to help! Could you tell me more about your project and what specific assistance you need?",
      role: "system",
      timestamp: new Date(Date.now() - 180000),
    },
  ])
  const [inputValue, setInputValue] = useState("")

  const [editingMessageId, setEditingMessageId] = useState<string | null>(null)
  const [editValue, setEditValue] = useState("")

  const [messageFeedback, setMessageFeedback] = useState<Record<string, "up" | "down" | null>>({})

  const [isThinking, setIsThinking] = useState(false)

  const [userEmail] = useState("user@example.com")
  const [tokensUsed] = useState(12450)

  const [chatTitle, setChatTitle] = useState("AI Chat")
  const [isEditingTitle, setIsEditingTitle] = useState(false)
  const [editTitleValue, setEditTitleValue] = useState("")

  const [temperature, setTemperature] = useState(0)
  const [selectedModel, setSelectedModel] = useState("model1")

  const messagesEndRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const modelDetails: Record<string, { embedding: string; vocabulary: string }> = {
    model1: { embedding: "12,349", vocabulary: "340,332" },
    model2: { embedding: "8,192", vocabulary: "256,000" },
    model3: { embedding: "16,384", vocabulary: "512,000" },
    model4: { embedding: "4,096", vocabulary: "128,000" },
    model5: { embedding: "32,768", vocabulary: "1,024,000" },
  }

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString("en-US", {
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    })
  }

  const formatRelativeTime = (date: Date) => {
    const now = new Date()
    const diffInHours = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60))

    if (diffInHours < 1) return "Just now"
    if (diffInHours < 24) return `${diffInHours}h ago`
    const diffInDays = Math.floor(diffInHours / 24)
    if (diffInDays === 1) return "Yesterday"
    if (diffInDays < 7) return `${diffInDays}d ago`
    return date.toLocaleDateString()
  }

  const handleSend = () => {
    if (!inputValue.trim()) return

    const newMessage: Message = {
      id: Date.now().toString(),
      content: inputValue,
      role: "user",
      timestamp: new Date(),
    }

    setMessages([...messages, newMessage])
    setInputValue("")

    if (textareaRef.current) {
      textareaRef.current.style.height = "auto"
    }

    setIsThinking(true)

    setTimeout(() => {
      const aiResponse: Message = {
        id: (Date.now() + 1).toString(),
        content: "Thank you for your message. I'm processing your request...",
        role: "system",
        timestamp: new Date(),
      }
      setMessages((prev) => [...prev, aiResponse])
      setIsThinking(false)
    }, 1000)
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInputValue(e.target.value)

    if (textareaRef.current) {
      textareaRef.current.style.height = "auto"
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 200)}px`
    }
  }

  const handleTouchStart = (e: React.TouchEvent, chatId: string) => {
    setStartX(e.touches[0].clientX)
    setCurrentDragChatId(chatId)
    setIsDragging(true)
    if (swipedChatId === chatId) {
      setStartX(e.touches[0].clientX + 80)
    }
  }

  const handleTouchMove = (e: React.TouchEvent) => {
    if (!isDragging || !currentDragChatId) return
    const currentX = e.touches[0].clientX
    const diff = startX - currentX
    if (diff >= 0 && diff <= 80) {
      setSwipeOffset(diff)
      setSwipedChatId(currentDragChatId)
    }
  }

  const handleTouchEnd = () => {
    setIsDragging(false)
    if (swipeOffset > 40) {
      setSwipeOffset(80)
    } else {
      setSwipeOffset(0)
      setSwipedChatId(null)
    }
    setCurrentDragChatId(null)
  }

  const handleMouseDown = (e: React.MouseEvent, chatId: string) => {
    setStartX(e.clientX)
    setCurrentDragChatId(chatId)
    setIsDragging(true)
    if (swipedChatId === chatId) {
      setStartX(e.clientX + 80)
    }
  }

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging || !currentDragChatId) return
    const currentX = e.clientX
    const diff = startX - currentX
    if (diff >= 0 && diff <= 80) {
      setSwipeOffset(diff)
      setSwipedChatId(currentDragChatId)
    }
  }

  const handleMouseUp = () => {
    setIsDragging(false)
    if (swipeOffset > 40) {
      setSwipeOffset(80)
    } else {
      setSwipeOffset(0)
      setSwipedChatId(null)
    }
    setCurrentDragChatId(null)
  }

  const handleDelete = (chatId: string) => {
    setDeletingChatId(chatId)
    setSwipedChatId(null)
    setSwipeOffset(0)

    setTimeout(() => {
      setRecentChats(recentChats.filter((chat) => chat.id !== chatId))
      setDeletingChatId(null)
    }, 300)
  }

  const handleEditMessage = (messageId: string, currentContent: string) => {
    setEditingMessageId(messageId)
    setEditValue(currentContent)
  }

  const handleCancelEdit = () => {
    setEditingMessageId(null)
    setEditValue("")
  }

  const handleSaveEdit = (messageId: string) => {
    if (!editValue.trim()) return

    setMessages(messages.map((msg) => (msg.id === messageId ? { ...msg, content: editValue } : msg)))
    setEditingMessageId(null)
    setEditValue("")
  }

  const handleEditKeyPress = (e: React.KeyboardEvent, messageId: string) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      handleSaveEdit(messageId)
    }
    if (e.key === "Escape") {
      setEditingMessageId(null)
      setEditValue("")
    }
  }

  const handleFeedback = (messageId: string, feedback: "up" | "down") => {
    setMessageFeedback((prev) => ({
      ...prev,
      [messageId]: prev[messageId] === feedback ? null : feedback,
    }))
  }

  const handleSignOut = () => {
    router.push("/sign-in")
  }

  const handleEditTitle = () => {
    setIsEditingTitle(true)
    setEditTitleValue(chatTitle)
  }

  const handleSaveTitle = () => {
    if (editTitleValue.trim()) {
      setChatTitle(editTitleValue.trim())
    }
    setIsEditingTitle(false)
    setEditTitleValue("")
  }

  const handleCancelTitleEdit = () => {
    setIsEditingTitle(false)
    setEditTitleValue("")
  }

  const handleTitleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault()
      handleSaveTitle()
    }
    if (e.key === "Escape") {
      handleCancelTitleEdit()
    }
  }

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" })
  }, [messages, isThinking])

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (swipedChatId && swipeOffset === 80) {
        const target = e.target as HTMLElement
        if (!target.closest("[data-delete-button]") && !target.closest("[data-chat-item]")) {
          setSwipedChatId(null)
          setSwipeOffset(0)
        }
      }
    }

    document.addEventListener("click", handleClickOutside)
    return () => document.removeEventListener("click", handleClickOutside)
  }, [swipedChatId, swipeOffset])

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      {isSidebarOpen && (
        <div className="fixed inset-0 z-40 bg-black/50 md:hidden" onClick={() => setIsSidebarOpen(false)} />
      )}

      <aside
        className={`fixed inset-y-0 left-0 z-50 w-64 flex-col border-r border-border bg-sidebar transition-transform duration-300 md:relative md:translate-x-0 ${
          isSidebarOpen ? "flex translate-x-0" : "hidden -translate-x-full md:flex"
        }`}
      >
        <div className="flex items-center justify-between border-b border-border p-4">
          <Button className="flex-1 justify-start gap-2" size="sm">
            <Plus className="h-4 w-4" />
            New Chat
          </Button>
          <Button variant="ghost" size="icon" className="ml-2 md:hidden" onClick={() => setIsSidebarOpen(false)}>
            <X className="h-5 w-5" />
            <span className="sr-only">Close sidebar</span>
          </Button>
        </div>
        <div className="flex-1 overflow-y-auto p-2">
          <div className="space-y-1">
            {recentChats.map((chat) => (
              <div
                key={chat.id}
                className="relative overflow-hidden"
                style={{
                  maxHeight: deletingChatId === chat.id ? "0" : "200px",
                  marginBottom: deletingChatId === chat.id ? "0" : "4px",
                  transition:
                    deletingChatId === chat.id
                      ? "max-height 0.3s ease-out, margin-bottom 0.3s ease-out"
                      : "max-height 0.3s ease-out, margin-bottom 0.3s ease-out",
                }}
              >
                <div
                  className="absolute right-0 top-0 flex h-full w-20 items-center justify-center rounded-lg bg-destructive"
                  style={{
                    transform: `translateX(${
                      deletingChatId === chat.id ? -220 : swipedChatId === chat.id ? 80 - swipeOffset : 80
                    }px)`,
                    transition:
                      deletingChatId === chat.id
                        ? "transform 0.3s ease"
                        : isDragging && currentDragChatId === chat.id
                          ? "none"
                          : "transform 0.3s ease",
                  }}
                >
                  <Button
                    data-delete-button
                    variant="ghost"
                    size="icon"
                    className="h-full w-full rounded-lg text-destructive-foreground hover:bg-destructive hover:text-destructive-foreground"
                    onClick={() => handleDelete(chat.id)}
                  >
                    <Trash2 className="h-5 w-5" />
                    <span className="sr-only">Delete chat</span>
                  </Button>
                </div>
                <button
                  data-chat-item
                  className={`relative flex w-full flex-col gap-1 rounded-lg px-3 py-2.5 text-left transition-colors ${
                    activeChatId === chat.id ? "bg-accent" : "bg-sidebar hover:bg-accent"
                  }`}
                  style={{
                    transform: `translateX(-${
                      deletingChatId === chat.id ? 300 : swipedChatId === chat.id ? swipeOffset : 0
                    }px)`,
                    opacity: deletingChatId === chat.id ? 0 : 1,
                    transition:
                      deletingChatId === chat.id
                        ? "transform 0.3s ease, opacity 0.3s ease"
                        : isDragging && currentDragChatId === chat.id
                          ? "none"
                          : "transform 0.3s ease",
                  }}
                  onClick={() => setActiveChatId(chat.id)}
                  onTouchStart={(e) => handleTouchStart(e, chat.id)}
                  onTouchMove={handleTouchMove}
                  onTouchEnd={handleTouchEnd}
                  onMouseDown={(e) => handleMouseDown(e, chat.id)}
                  onMouseMove={handleMouseMove}
                  onMouseUp={handleMouseUp}
                  onMouseLeave={handleMouseUp}
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2 min-w-0">
                      <MessageSquare
                        className={`h-4 w-4 flex-shrink-0 ${activeChatId === chat.id ? "text-primary" : "text-muted-foreground"}`}
                      />
                      <span
                        className={`truncate text-sm font-medium ${activeChatId === chat.id ? "text-foreground font-semibold" : "text-foreground"}`}
                      >
                        {chat.title}
                      </span>
                    </div>
                    <span className="flex-shrink-0 text-xs text-muted-foreground">
                      {formatRelativeTime(chat.timestamp)}
                    </span>
                  </div>
                  <p className="truncate pl-6 text-xs text-muted-foreground">{chat.lastMessage}</p>
                </button>
              </div>
            ))}
          </div>
        </div>
        <div className="border-t border-border p-4">
          <div className="flex flex-wrap items-center justify-center gap-3 text-xs text-muted-foreground">
            <a href="/about" className="flex items-center gap-1 hover:text-foreground transition-colors">
              <Info className="h-3 w-3" />
              <span>About</span>
            </a>
          </div>
        </div>
      </aside>

      <div className="flex flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-border px-6 py-4">
          <div className="flex items-center gap-3">
            <Button variant="ghost" size="icon" className="md:hidden" onClick={() => setIsSidebarOpen(true)}>
              <Menu className="h-5 w-5" />
              <span className="sr-only">Open sidebar</span>
            </Button>
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary">
              <span className="font-mono text-sm font-bold text-primary-foreground">AI</span>
            </div>
            {isEditingTitle ? (
              <div className="flex items-center gap-2">
                <Input
                  value={editTitleValue}
                  onChange={(e) => setEditTitleValue(e.target.value)}
                  onKeyDown={handleTitleKeyPress}
                  className="h-8 w-48 text-lg font-semibold"
                  autoFocus
                />
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-green-600 hover:bg-green-500/20 hover:text-green-600"
                  onClick={handleSaveTitle}
                >
                  <Check className="h-4 w-4" />
                  <span className="sr-only">Confirm</span>
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-muted-foreground hover:bg-accent"
                  onClick={handleCancelTitleEdit}
                >
                  <X className="h-4 w-4" />
                  <span className="sr-only">Cancel</span>
                </Button>
              </div>
            ) : (
              <h1
                className="font-sans text-lg font-semibold text-foreground cursor-pointer hover:text-primary transition-colors"
                onClick={handleEditTitle}
              >
                {chatTitle}
              </h1>
            )}
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm" className="gap-2 bg-transparent">
                <span className="max-w-[150px] truncate">{userEmail}</span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-56">
              <DropdownMenuLabel className="px-3 py-2">Account</DropdownMenuLabel>
              <DropdownMenuSeparator className="my-2" />
              <DropdownMenuItem disabled className="flex flex-col items-start px-3 py-2">
                <span className="text-xs text-muted-foreground">Tokens Used</span>
                <span className="font-mono text-sm font-semibold">{tokensUsed.toLocaleString()}</span>
              </DropdownMenuItem>
              <DropdownMenuSeparator className="my-2" />
              <DropdownMenuItem
                onClick={handleSignOut}
                className="cursor-pointer px-3 py-2 hover:border-0 focus:border-0"
              >
                <span>Sign out</span>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </header>

        <main className="flex-1 overflow-y-auto px-4 py-6">
          <div className="mx-auto max-w-3xl space-y-6">
            {messages.map((message, index) => (
              <div
                key={message.id}
                className={`flex ${message.role === "user" ? "justify-end" : "justify-start"}`}
                style={{
                  animation: "bubble-fade-in 0.2s ease-out forwards",
                  animationDelay: `${index * 50}ms`,
                  opacity: 0,
                }}
              >
                <div
                  className={`flex max-w-[80%] flex-col gap-2 ${message.role === "system" ? "items-start" : "items-end"}`}
                >
                  {editingMessageId === message.id ? (
                    <Input
                      value={editValue}
                      onChange={(e) => setEditValue(e.target.value)}
                      onKeyDown={(e) => handleEditKeyPress(e, message.id)}
                      className="min-h-[48px] w-full rounded-2xl border-input bg-muted px-4 py-3 text-foreground"
                      autoFocus
                    />
                  ) : (
                    <div
                      className={`rounded-2xl px-4 py-3 ${
                        message.role === "system"
                          ? "bg-chat-system-dark text-foreground"
                          : "bg-chat-user text-chat-user-text"
                      }`}
                      style={{
                        boxShadow:
                          message.role === "system"
                            ? "0 0 20px rgba(139, 92, 246, 0.15), 0 0 40px rgba(139, 92, 246, 0.08)"
                            : "0 0 20px rgba(139, 92, 246, 0.25), 0 0 40px rgba(139, 92, 246, 0.12)",
                      }}
                    >
                      <p className="text-pretty leading-relaxed">{message.content}</p>
                    </div>
                  )}
                  <div className="flex items-center gap-2">
                    <span
                      className={`px-2 text-xs text-muted-foreground ${message.role === "user" ? "text-right" : "text-left"}`}
                    >
                      {formatTime(message.timestamp)}
                    </span>
                    {message.role === "system" && (
                      <>
                        <Button
                          variant="ghost"
                          size="icon"
                          className={`h-6 w-6 rounded-full flex items-center justify-center transition-all ${
                            messageFeedback[message.id] === "up"
                              ? "bg-accent text-accent-foreground opacity-100"
                              : "text-muted-foreground opacity-50 hover:opacity-100 hover:bg-accent"
                          }`}
                          onClick={() => handleFeedback(message.id, "up")}
                        >
                          <ThumbsUp className="h-4 w-4" />
                          <span className="sr-only">Positive feedback</span>
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className={`h-6 w-6 rounded-full flex items-center justify-center transition-all ${
                            messageFeedback[message.id] === "down"
                              ? "bg-accent text-accent-foreground opacity-100"
                              : "text-muted-foreground opacity-50 hover:opacity-100 hover:bg-accent"
                          }`}
                          onClick={() => handleFeedback(message.id, "down")}
                        >
                          <ThumbsDown className="h-4 w-4" />
                          <span className="sr-only">Negative feedback</span>
                        </Button>
                      </>
                    )}
                    {message.role === "user" && (
                      <>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-6 w-6 opacity-50 hover:opacity-100"
                          onClick={() =>
                            editingMessageId === message.id
                              ? handleSaveEdit(message.id)
                              : handleEditMessage(message.id, message.content)
                          }
                        >
                          {editingMessageId === message.id ? (
                            <ArrowRight className="h-3 w-3" strokeWidth={3} />
                          ) : (
                            <Pencil className="h-3 w-3" />
                          )}
                          <span className="sr-only">
                            {editingMessageId === message.id ? "Save edit" : "Edit message"}
                          </span>
                        </Button>
                        {editingMessageId === message.id && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6 opacity-50 hover:opacity-100"
                            onClick={handleCancelEdit}
                          >
                            <X className="h-3 w-3" />
                            <span className="sr-only">Cancel edit</span>
                          </Button>
                        )}
                      </>
                    )}
                  </div>
                </div>
              </div>
            ))}

            {isThinking && (
              <div
                className="flex justify-start"
                style={{
                  animation: "bubble-fade-in 0.3s ease-out forwards",
                }}
              >
                <div
                  className="flex items-center gap-2 rounded-2xl bg-chat-system-dark px-4 py-3"
                  style={{
                    boxShadow: "0 0 20px rgba(139, 92, 246, 0.15), 0 0 40px rgba(139, 92, 246, 0.08)",
                  }}
                >
                  <Square className="h-4 w-4 text-primary animate-spin" />
                  <span className="text-sm text-muted-foreground">Thinking...</span>
                </div>
              </div>
            )}

            <div ref={messagesEndRef} />
          </div>
        </main>

        <div className="border-t border-border bg-background px-4 py-4">
          <div className="mx-auto max-w-3xl">
            <div className="flex items-end gap-2">
              <Popover>
                <PopoverTrigger asChild>
                  <Button variant="ghost" size="icon" className="h-12 w-12 rounded-xl text-muted-foreground">
                    <Settings className="h-5 w-5" />
                    <span className="sr-only">Settings</span>
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-64 dark:bg-zinc-700 dark:border-zinc-600" align="start" side="top">
                  <div className="space-y-4">
                    <div className="space-y-2">
                      <label className="text-sm font-medium">Model</label>
                      <Select value={selectedModel} onValueChange={setSelectedModel}>
                        <SelectTrigger className="w-full">
                          <SelectValue placeholder="Select a model" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="model1" className="py-2 px-3">
                            Model 1
                          </SelectItem>
                          <SelectItem value="model2" className="py-2 px-3">
                            Model 2
                          </SelectItem>
                          <SelectItem value="model3" className="py-2 px-3">
                            Model 3
                          </SelectItem>
                          <SelectItem value="model4" className="py-2 px-3">
                            Model 4
                          </SelectItem>
                          <SelectItem value="model5" className="py-2 px-3">
                            Model 5
                          </SelectItem>
                        </SelectContent>
                      </Select>
                      <div className="pt-1 space-y-0.5 text-xs text-muted-foreground">
                        <div>Embedding Size: {modelDetails[selectedModel].embedding}</div>
                        <div>Vocabulary: {modelDetails[selectedModel].vocabulary}</div>
                      </div>
                    </div>

                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <label className="text-sm font-medium">Temperature</label>
                        <span className="text-sm text-muted-foreground">{temperature.toFixed(1)}</span>
                      </div>
                      <Slider
                        value={[temperature]}
                        onValueChange={(value) => setTemperature(value[0])}
                        min={-1}
                        max={1}
                        step={0.1}
                        className="w-full"
                      />
                      <div className="flex justify-between text-xs text-muted-foreground">
                        <span>-1.0</span>
                        <span>0.0</span>
                        <span>1.0</span>
                      </div>
                    </div>
                  </div>
                </PopoverContent>
              </Popover>

              <div className="flex-1">
                <Textarea
                  ref={textareaRef}
                  value={inputValue}
                  onChange={handleInputChange}
                  onKeyDown={handleKeyPress}
                  placeholder="Type your message..."
                  className="min-h-[48px] max-h-[200px] resize-none rounded-xl border-input bg-muted px-4 py-3 text-foreground placeholder:text-muted-foreground"
                  rows={1}
                />
              </div>
              <Button onClick={handleSend} size="icon" className="h-12 w-12 rounded-xl" disabled={!inputValue.trim()}>
                <ArrowRight className="h-5 w-5" strokeWidth={3} />
                <span className="sr-only">Send message</span>
              </Button>
            </div>
          </div>
        </div>
      </div>

      <style jsx>{`
        @keyframes bubble-fade-in {
          0% {
            opacity: 0;
          }
          100% {
            opacity: 1;
          }
        }
      `}</style>
    </div>
  )
}
