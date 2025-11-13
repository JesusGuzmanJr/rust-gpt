// Sidebar toggle functionality
document.addEventListener('DOMContentLoaded', () => {
    const sidebar = document.getElementById('chat-sidebar');
    const backdrop = document.getElementById('sidebar-backdrop');
    const menuBtn = document.getElementById('sidebar-menu-btn');
    const closeBtn = document.getElementById('sidebar-close-btn');

    if (sidebar && backdrop && menuBtn && closeBtn) {
        // Open sidebar
        menuBtn.addEventListener('click', () => {
            sidebar.classList.add('is-open');
            backdrop.classList.add('is-visible');
        });

        // Close sidebar
        const closeSidebar = () => {
            sidebar.classList.remove('is-open');
            backdrop.classList.remove('is-visible');
        };

        closeBtn.addEventListener('click', closeSidebar);
        backdrop.addEventListener('click', closeSidebar);

        // Close sidebar when selecting a chat item on mobile
        document.addEventListener('click', (event) => {
            const chatItem = event.target.closest('.chat-item__content');
            if (chatItem && sidebar.classList.contains('is-open')) {
                closeSidebar();
            }
        });

        // Close sidebar when clicking outside on mobile
        document.addEventListener('click', (event) => {
            const isClickInsideSidebar = sidebar.contains(event.target);
            const isClickOnMenuBtn = menuBtn.contains(event.target);
            const modal = document.getElementById('delete-confirmation-modal');
            const modalBackdrop = document.getElementById('modal-backdrop');
            const isModalOpen = modal && modal.classList.contains('is-visible');
            const isClickOnModal = modal && modal.contains(event.target);
            const isClickOnModalBackdrop = modalBackdrop && modalBackdrop.contains(event.target);

            // Don't close sidebar if modal is open or click is on modal elements
            if (!isClickInsideSidebar && !isClickOnMenuBtn && sidebar.classList.contains('is-open') && !isModalOpen && !isClickOnModal && !isClickOnModalBackdrop) {
                closeSidebar();
            }
        });
    }
});

// Chat title editing functionality
document.addEventListener('DOMContentLoaded', () => {
    const titleDisplay = document.getElementById('chat-title-display');
    const titleEdit = document.getElementById('chat-title-edit');
    const titleInput = document.getElementById('chat-title-input');
    const confirmButton = document.getElementById('chat-title-confirm');
    const cancelButton = document.getElementById('chat-title-cancel');
    const chatHeader = document.querySelector('.chat-header');

    if (titleDisplay && titleEdit && titleInput && confirmButton && cancelButton) {
        // Enter edit mode when clicking the title
        titleDisplay.addEventListener('click', () => {
            titleInput.value = titleDisplay.textContent;
            titleDisplay.style.display = 'none';
            titleEdit.style.display = 'flex';
            if (chatHeader) chatHeader.classList.add('title-editing');
            titleInput.focus();
            titleInput.select();
        });

        // Save title
        const saveTitle = () => {
            const newTitle = titleInput.value.trim();
            if (newTitle) {
                titleDisplay.textContent = newTitle;
            }
            titleEdit.style.display = 'none';
            titleDisplay.style.display = 'block';
            if (chatHeader) chatHeader.classList.remove('title-editing');
        };

        // Cancel edit
        const cancelEdit = () => {
            titleEdit.style.display = 'none';
            titleDisplay.style.display = 'block';
            if (chatHeader) chatHeader.classList.remove('title-editing');
        };

        // Confirm button click
        confirmButton.addEventListener('click', (event) => {
            event.preventDefault();
            event.stopPropagation();
            saveTitle();
        });

        // Cancel button click
        cancelButton.addEventListener('click', (event) => {
            event.preventDefault();
            event.stopPropagation();
            cancelEdit();
        });

        // Keyboard shortcuts
        titleInput.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') {
                event.preventDefault();
                // Click the confirm button to trigger HTMX request
                confirmButton.click();
            } else if (event.key === 'Escape') {
                event.preventDefault();
                cancelEdit();
            }
        });
    }
});

// Settings popover functionality
document.addEventListener('DOMContentLoaded', () => {
    const popover = document.getElementById('settings-popover');
    const settingsButton = document.getElementById('settings-btn');

    if (!popover || !settingsButton) return;

    let isOpen = false;

    // Toggle popover
    const togglePopover = () => {
        isOpen = !isOpen;
        popover.classList.toggle('show', isOpen);

        if (isOpen) {
            // Add click listener after a brief delay to avoid immediate closing
            setTimeout(() => {
                document.addEventListener('click', handleClickOutside);
            }, 10);
        } else {
            document.removeEventListener('click', handleClickOutside);
        }
    };

    // Close popover with animation
    const closePopover = () => {
        if (isOpen) {
            isOpen = false;
            popover.classList.remove('show');
            popover.classList.add('closing');

            // Remove closing class after animation completes
            const handleAnimationEnd = () => {
                popover.classList.remove('closing');
                popover.removeEventListener('animationend', handleAnimationEnd);
            };

            popover.addEventListener('animationend', handleAnimationEnd);

            document.removeEventListener('click', handleClickOutside);
        }
    };

    // Handle clicks outside popover
    const handleClickOutside = (event) => {
        const isClickInsidePopover = popover.contains(event.target);
        const isClickOnButton = settingsButton.contains(event.target);

        if (!isClickInsidePopover && !isClickOnButton) {
            closePopover();
        }
    };

    // Settings button click
    settingsButton.addEventListener('click', (event) => {
        event.stopPropagation();
        togglePopover();
    });

    // Close popover when pressing Escape key
    document.addEventListener('keydown', (event) => {
        if (event.key === 'Escape' && isOpen) {
            closePopover();
        }
    });

    // Update temperature display when slider changes
    const temperatureSlider = document.querySelector('input[name="temperature"]');
    const temperatureValue = document.querySelector('.form-value');

    if (temperatureSlider && temperatureValue) {
        temperatureSlider.addEventListener('input', (event) => {
            temperatureValue.textContent = parseFloat(event.target.value).toFixed(1);
        });
    }

    // Enable/disable send button based on textarea content
    const messageInput = document.getElementById('message-input');
    const sendButton = document.getElementById('send-btn');

    if (messageInput && sendButton) {
        const updateSendButton = () => {
            const hasContent = messageInput.value.trim().length > 0;
            sendButton.disabled = !hasContent;
        };

        // Auto-expand textarea
        const autoExpand = () => {
            // If empty, reset to min height
            if (messageInput.value === '') {
                messageInput.style.height = '48px'; // 3rem = 48px
                return;
            }

            // Store the current scroll position
            const scrollPos = messageInput.scrollTop;

            // Reset height to min-height to get accurate scrollHeight
            messageInput.style.height = '48px';

            // Calculate new height based on content
            const scrollHeight = messageInput.scrollHeight;

            // Only expand if content actually needs more space (threshold accounts for single-line padding)
            // If scrollHeight is <= 56px, it's still a single line, so keep at 48px
            if (scrollHeight <= 56) {
                messageInput.style.height = '48px';
            } else {
                // Multi-line content: set height to scrollHeight, capped at 200px
                const newHeight = Math.min(scrollHeight, 200);
                messageInput.style.height = newHeight + 'px';
            }

            // Restore scroll position if needed
            messageInput.scrollTop = scrollPos;
        };

        // Check on input
        messageInput.addEventListener('input', () => {
            updateSendButton();
            autoExpand();
        });

        // Initial check
        updateSendButton();
        autoExpand();

        // Clear textarea after successful message send
        const clearInput = () => {
            messageInput.value = '';
            messageInput.style.height = '48px';
            updateSendButton();
        };

        // Listen for htmx afterRequest event on both textarea and send button
        messageInput.addEventListener('htmx:afterRequest', (event) => {
            if (event.detail.successful) {
                clearInput();
            }
        });

        sendButton.addEventListener('htmx:afterRequest', (event) => {
            if (event.detail.successful) {
                clearInput();
            }
        });

        // Handle Enter key to trigger send button
        messageInput.addEventListener('keydown', (event) => {
            if (event.key === 'Enter' && !event.shiftKey) {
                event.preventDefault();
                if (!sendButton.disabled) {
                    sendButton.click();
                }
            }
        });
    }
});

// Delete confirmation modal - close functionality
document.addEventListener('DOMContentLoaded', () => {
    const modal = document.getElementById('delete-confirmation-modal');
    const modalBackdrop = document.getElementById('modal-backdrop');
    const confirmDeleteBtn = document.getElementById('confirm-delete-btn');
    const cancelDeleteBtn = document.getElementById('cancel-delete-btn');

    const closeModal = () => {
        if (modal) modal.classList.remove('is-visible');
        if (modalBackdrop) modalBackdrop.classList.remove('is-visible');
    };

    // Cancel delete
    if (cancelDeleteBtn) {
        cancelDeleteBtn.addEventListener('click', closeModal);
    }

    // Close modal on backdrop click
    if (modalBackdrop) {
        modalBackdrop.addEventListener('click', closeModal);
    }

    // Close modal after successful delete
    if (confirmDeleteBtn) {
        confirmDeleteBtn.addEventListener('htmx:afterRequest', (event) => {
            if (event.detail.successful) {
                closeModal();
            }
        });
    }

    // Handle keyboard shortcuts for modal
    document.addEventListener('keydown', (e) => {
        if (modal && modal.classList.contains('is-visible')) {
            if (e.key === 'Escape') {
                closeModal();
            } else if (e.key === 'Enter') {
                // Trigger delete on Enter key
                e.preventDefault();
                if (confirmDeleteBtn) {
                    confirmDeleteBtn.click();
                }
            }
        }
    });
});

// Message editing functionality
document.addEventListener('DOMContentLoaded', () => {
    let isEditingMessage = false;

    const enableChatInput = () => {
        const chatInput = document.getElementById('message-input');
        const sendButton = document.getElementById('send-btn');

        if (chatInput) chatInput.disabled = false;
        if (sendButton && chatInput) {
            // Only enable if there's content
            sendButton.disabled = chatInput.value.trim().length === 0;
        }
        isEditingMessage = false;
    };

    const disableChatInput = () => {
        const chatInput = document.getElementById('message-input');
        const sendButton = document.getElementById('send-btn');

        if (chatInput) chatInput.disabled = true;
        if (sendButton) sendButton.disabled = true;
        isEditingMessage = true;
    };

    // Use event delegation since messages are added dynamically
    document.addEventListener('click', (event) => {
        // Handle edit button clicks
        const editBtn = event.target.closest('.message__edit-btn');
        if (editBtn) {
            const messageWrapper = editBtn.closest('.message__wrapper');
            if (!messageWrapper) return;

            const message = messageWrapper.closest('.message--user');
            if (!message) return;

            const messageId = message.id;
            const displayBubble = message.querySelector(`#${messageId}-display`);
            const editBubble = message.querySelector(`#${messageId}-edit`);
            const editInput = message.querySelector(`#${messageId}-input`);
            const metaDisplay = message.querySelector(`#${messageId}-meta-display`);
            const metaEdit = message.querySelector(`#${messageId}-meta-edit`);

            if (displayBubble && editBubble && editInput && metaDisplay && metaEdit) {
                // Enter edit mode
                displayBubble.style.display = 'none';
                editBubble.style.display = 'block';
                metaDisplay.style.display = 'none';
                metaEdit.style.display = 'flex';
                editInput.focus();

                // Move cursor to end of text
                const length = editInput.value.length;
                editInput.setSelectionRange(length, length);

                // Disable bottom input field
                disableChatInput();

                // Auto-expand textarea to fit content
                editInput.style.height = 'auto';
                editInput.style.height = editInput.scrollHeight + 'px';
            }
        }

        // Handle cancel button clicks
        const cancelBtn = event.target.closest('.message__edit-cancel');
        if (cancelBtn) {
            const message = cancelBtn.closest('.message--user');
            if (!message) return;

            const messageId = message.id;
            const displayBubble = message.querySelector(`#${messageId}-display`);
            const editBubble = message.querySelector(`#${messageId}-edit`);
            const editInput = message.querySelector(`#${messageId}-input`);
            const metaDisplay = message.querySelector(`#${messageId}-meta-display`);
            const metaEdit = message.querySelector(`#${messageId}-meta-edit`);

            if (displayBubble && editBubble && editInput && metaDisplay && metaEdit) {
                // Exit edit mode without saving
                const originalText = displayBubble.querySelector('p')?.textContent || '';
                editInput.value = originalText;
                displayBubble.style.display = 'block';
                editBubble.style.display = 'none';
                metaDisplay.style.display = 'flex';
                metaEdit.style.display = 'none';

                // Re-enable bottom input field
                enableChatInput();
            }
        }

        // Handle confirm button clicks - set flag before HTMX request
        const confirmBtn = event.target.closest('.message__edit-confirm');
        if (confirmBtn) {
            // The re-enabling will happen in htmx:afterSwap
        }
    });

    // Handle keyboard shortcuts in edit mode
    document.addEventListener('keydown', (event) => {
        const editInput = event.target;
        if (editInput.classList.contains('message__edit-input')) {
            if (event.key === 'Escape') {
                event.preventDefault();
                const message = editInput.closest('.message--user');
                if (message) {
                    const messageId = message.id;
                    const cancelBtn = message.querySelector(`#${messageId}-meta-edit .message__edit-cancel`);
                    if (cancelBtn) cancelBtn.click();
                }
            } else if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
                // Ctrl+Enter or Cmd+Enter to save
                event.preventDefault();
                const message = editInput.closest('.message--user');
                if (message) {
                    const messageId = message.id;
                    const confirmBtn = message.querySelector(`#${messageId}-meta-edit .message__edit-confirm`);
                    if (confirmBtn) confirmBtn.click();
                }
            }
        }
    });

    // Auto-expand textarea as user types
    document.addEventListener('input', (event) => {
        const editInput = event.target;
        if (editInput.classList.contains('message__edit-input')) {
            editInput.style.height = 'auto';
            editInput.style.height = editInput.scrollHeight + 'px';
        }
    });

    // Re-enable input after successful edit (after the DOM swap completes)
    document.addEventListener('htmx:afterSwap', (event) => {
        // If we were editing and any swap happened in a message, re-enable
        // (This fires when confirm button successfully updates the message)
        if (isEditingMessage) {
            const swappedElement = event.detail.target;
            if (swappedElement && swappedElement.classList.contains('message--user')) {
                enableChatInput();
            }
        }
    });
});