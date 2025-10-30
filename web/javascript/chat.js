// Chat title editing functionality
document.addEventListener('DOMContentLoaded', () => {
    const titleDisplay = document.getElementById('chat-title-display');
    const titleEdit = document.getElementById('chat-title-edit');
    const titleInput = document.getElementById('chat-title-input');
    const confirmButton = document.getElementById('chat-title-confirm');
    const cancelButton = document.getElementById('chat-title-cancel');

    if (titleDisplay && titleEdit && titleInput && confirmButton && cancelButton) {
        // Enter edit mode when clicking the title
        titleDisplay.addEventListener('click', () => {
            titleInput.value = titleDisplay.textContent;
            titleDisplay.style.display = 'none';
            titleEdit.style.display = 'flex';
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
        };

        // Cancel edit
        const cancelEdit = () => {
            titleEdit.style.display = 'none';
            titleDisplay.style.display = 'block';
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
                saveTitle();
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
    }
});