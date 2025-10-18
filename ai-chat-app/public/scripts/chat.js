class ChatApp {
  constructor() {
    this.messages = [
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
    ]

    this.chats = [
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
    ]

    this.activeChatId = "1"
    this.isSidebarOpen = false
    this.isThinking = false
    this.selectedModel = "model1"
    this.temperature = 0

    this.modelDetails = {
      model1: { embedding: "12,349", vocabulary: "340,332" },
      model2: { embedding: "8,192", vocabulary: "256,000" },
      model3: { embedding: "16,384", vocabulary: "512,000" },
      model4: { embedding: "4,096", vocabulary: "128,000" },
      model5: { embedding: "32,768", vocabulary: "1,024,000" },
    }

    this.init()
  }

  init() {
    this.cacheElements()
    this.bindEvents()
    this.renderChats()
    this.renderMessages()
    this.updateModelDetails()
  }

  cacheElements() {
    this.sidebar = document.getElementById("sidebar")
    this.sidebarOverlay = document.getElementById("sidebarOverlay")
    this.menuBtn = document.getElementById("menuBtn")
    this.closeSidebarBtn = document.getElementById("closeSidebarBtn")
    this.chatList = document.getElementById("chatList")
    this.messagesContainer = document.getElementById("messagesContainer")
    this.messageInput = document.getElementById("messageInput")
    this.sendBtn = document.getElementById("sendBtn")
    this.userDropdownBtn = document.getElementById("userDropdownBtn")
    this.userDropdownMenu = document.getElementById("userDropdownMenu")
    this.signOutBtn = document.getElementById("signOutBtn")
    this.chatTitle = document.getElementById("chatTitle")
    this.titleInput = document.getElementById("titleInput")
    this.titleActions = document.getElementById("titleActions")
    this.confirmTitleBtn = document.getElementById("confirmTitleBtn")
    this.cancelTitleBtn = document.getElementById("cancelTitleBtn")
    this.settingsBtn = document.getElementById("settingsBtn")
    this.settingsPopoverContent = document.getElementById("settingsPopoverContent")
    this.modelSelect = document.getElementById("modelSelect")
    this.modelDetails = document.getElementById("modelDetails")
    this.temperatureSlider = document.getElementById("temperatureSlider")
    this.temperatureValue = document.getElementById("temperatureValue")
  }

  bindEvents() {
    this.menuBtn.addEventListener("click", () => this.toggleSidebar())
    this.closeSidebarBtn.addEventListener("click", () => this.toggleSidebar())
    this.sidebarOverlay.addEventListener("click", () => this.toggleSidebar())
    this.sendBtn.addEventListener("click", () => this.handleSend())
    this.messageInput.addEventListener("input", (e) => this.handleInputChange(e))
    this.messageInput.addEventListener("keydown", (e) => this.handleKeyPress(e))
    this.userDropdownBtn.addEventListener("click", () => this.toggleDropdown())
    this.signOutBtn.addEventListener("click", () => this.handleSignOut())
    this.chatTitle.addEventListener("click", () => this.startEditingTitle())
    this.confirmTitleBtn.addEventListener("click", () => this.saveTitle())
    this.cancelTitleBtn.addEventListener("click", () => this.cancelEditingTitle())
    this.titleInput.addEventListener("keydown", (e) => this.handleTitleKeyPress(e))
    this.settingsBtn.addEventListener("click", () => this.toggleSettings())
    this.modelSelect.addEventListener("change", (e) => this.handleModelChange(e))
    this.temperatureSlider.addEventListener("input", (e) => this.handleTemperatureChange(e))

    document.addEventListener("click", (e) => this.handleClickOutside(e))
  }

  toggleSidebar() {
    this.isSidebarOpen = !this.isSidebarOpen
    if (this.isSidebarOpen) {
      this.sidebar.classList.add("chat-page__sidebar--open")
      this.sidebarOverlay.classList.add("chat-page__overlay--visible")
    } else {
      this.sidebar.classList.remove("chat-page__sidebar--open")
      this.sidebarOverlay.classList.remove("chat-page__overlay--visible")
    }
  }

  toggleDropdown() {
    const isVisible = this.userDropdownMenu.style.display === "block"
    this.userDropdownMenu.style.display = isVisible ? "none" : "block"
  }

  toggleSettings() {
    const isVisible = this.settingsPopoverContent.style.display === "block"
    this.settingsPopoverContent.style.display = isVisible ? "none" : "block"
  }

  handleClickOutside(e) {
    if (!e.target.closest("#userDropdown")) {
      this.userDropdownMenu.style.display = "none"
    }
    if (!e.target.closest("#settingsPopover")) {
      this.settingsPopoverContent.style.display = "none"
    }
  }

  handleInputChange(e) {
    const value = e.target.value
    this.sendBtn.disabled = !value.trim()

    e.target.style.height = "auto"
    e.target.style.height = Math.min(e.target.scrollHeight, 200) + "px"
  }

  handleKeyPress(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      this.handleSend()
    }
  }

  handleSend() {
    const content = this.messageInput.value.trim()
    if (!content) return

    const newMessage = {
      id: Date.now().toString(),
      content,
      role: "user",
      timestamp: new Date(),
    }

    this.messages.push(newMessage)
    this.messageInput.value = ""
    this.messageInput.style.height = "auto"
    this.sendBtn.disabled = true
    this.renderMessages()

    this.isThinking = true
    this.renderThinking()

    setTimeout(() => {
      const aiResponse = {
        id: (Date.now() + 1).toString(),
        content: "Thank you for your message. I'm processing your request...",
        role: "system",
        timestamp: new Date(),
      }
      this.messages.push(aiResponse)
      this.isThinking = false
      this.renderMessages()
    }, 1000)
  }

  handleSignOut() {
    window.location.href = "/sign-in.html"
  }

  startEditingTitle() {
    this.chatTitle.style.display = "none"
    this.titleInput.style.display = "block"
    this.titleActions.style.display = "flex"
    this.titleInput.value = this.chatTitle.textContent
    this.titleInput.focus()
  }

  saveTitle() {
    const newTitle = this.titleInput.value.trim()
    if (newTitle) {
      this.chatTitle.textContent = newTitle
    }
    this.cancelEditingTitle()
  }

  cancelEditingTitle() {
    this.chatTitle.style.display = "block"
    this.titleInput.style.display = "none"
    this.titleActions.style.display = "none"
  }

  handleTitleKeyPress(e) {
    if (e.key === "Enter") {
      e.preventDefault()
      this.saveTitle()
    } else if (e.key === "Escape") {
      this.cancelEditingTitle()
    }
  }

  handleModelChange(e) {
    this.selectedModel = e.target.value
    this.updateModelDetails()
  }

  handleTemperatureChange(e) {
    this.temperature = Number.parseFloat(e.target.value)
    this.temperatureValue.textContent = this.temperature.toFixed(1)
  }

  updateModelDetails() {
    const details = this.modelDetails[this.selectedModel]
    this.modelDetails.innerHTML = `
      <div>Embedding Size: ${details.embedding}</div>
      <div>Vocabulary: ${details.vocabulary}</div>
    `
  }

  renderChats() {
    this.chatList.innerHTML = this.chats
      .map(
        (chat) => `
      <div class="chat-item ${chat.id === this.activeChatId ? "chat-item--active" : ""}" data-id="${chat.id}">
        <div class="chat-item__content">
          <div class="chat-item__header">
            <svg class="chat-item__icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
            </svg>
            <span class="chat-item__title">${chat.title}</span>
            <span class="chat-item__time">${this.formatRelativeTime(chat.timestamp)}</span>
          </div>
          <p class="chat-item__message">${chat.lastMessage}</p>
        </div>
      </div>
    `,
      )
      .join("")
  }

  renderMessages() {
    this.messagesContainer.innerHTML = this.messages
      .map(
        (message, index) => `
      <div class="message message--${message.role}" style="animation-delay: ${index * 50}ms">
        <div class="message__bubble">
          <p class="message__content">${message.content}</p>
        </div>
        <div class="message__meta">
          <span class="message__time">${this.formatTime(message.timestamp)}</span>
        </div>
      </div>
    `,
      )
      .join("")

    this.messagesContainer.scrollTop = this.messagesContainer.scrollHeight
  }

  renderThinking() {
    const thinkingHTML = `
      <div class="message message--system message--thinking">
        <div class="message__bubble">
          <svg class="message__spinner" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
          </svg>
          <span class="message__content">Thinking...</span>
        </div>
      </div>
    `
    this.messagesContainer.insertAdjacentHTML("beforeend", thinkingHTML)
    this.messagesContainer.scrollTop = this.messagesContainer.scrollHeight
  }

  formatTime(date) {
    return date.toLocaleTimeString("en-US", {
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    })
  }

  formatRelativeTime(date) {
    const now = new Date()
    const diffInHours = Math.floor((now - date) / (1000 * 60 * 60))

    if (diffInHours < 1) return "Just now"
    if (diffInHours < 24) return `${diffInHours}h ago`
    const diffInDays = Math.floor(diffInHours / 24)
    if (diffInDays === 1) return "Yesterday"
    if (diffInDays < 7) return `${diffInDays}d ago`
    return date.toLocaleDateString()
  }
}

// Initialize the app
document.addEventListener("DOMContentLoaded", () => {
  new ChatApp()
})
