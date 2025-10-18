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
});