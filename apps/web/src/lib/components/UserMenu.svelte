<script lang="ts">
    import { authStore } from '$lib/stores/auth';
    import { goto } from '$app/navigation';

    let isOpen = $state(false);
    const user = $derived($authStore.user);

    // Display first name if available, otherwise username, fallback to 'U'
    const displayName = $derived(user?.first_name || user?.username || 'User');
    const isAdmin = $derived(user?.role === 'admin');

    function toggleMenu() {
        isOpen = !isOpen;
    }

    function handleAdminPanel() {
        isOpen = false;
        goto('/admin');
    }

    function handleSettings() {
        isOpen = false;
        goto('/settings');
    }

    function handleLogout() {
        authStore.logout();
        isOpen = false;
    }

    function closeMenu() {
        isOpen = false;
    }
</script>

<style>
    .user-menu-wrapper {
        position: relative;
    }

    .user-menu-button {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.5rem 0.75rem;
        border-radius: 0.5rem;
        border: none;
        background: transparent;
        cursor: pointer;
        transition: all 0.3s ease;
        outline: none;
    }

    .avatar {
        width: 2rem;
        height: 2rem;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 600;
        font-size: 0.875rem;
    }

    .username {
        font-weight: 500;
        font-size: 0.9375rem;
    }

    /* Dark mode styles */
    :global([data-theme='dark']) .user-menu-button {
        color: #ffffff;
    }

    :global([data-theme='dark']) .user-menu-button:hover {
        background: rgba(255, 255, 255, 0.1);
    }

    :global([data-theme='dark']) .avatar {
        background: #ffffff;
        color: #000000;
    }

    :global([data-theme='dark']) .dropdown-menu {
        background: #1f1f1f;
        border-color: rgba(255, 255, 255, 0.2);
    }

    :global([data-theme='dark']) .menu-item {
        color: #e0e0e0;
    }

    :global([data-theme='dark']) .menu-item:hover {
        background: rgba(255, 255, 255, 0.1);
    }

    /* Light mode styles */
    :global([data-theme='light']) .user-menu-button {
        color: #000000;
    }

    :global([data-theme='light']) .user-menu-button:hover {
        background: rgba(0, 0, 0, 0.05);
    }

    :global([data-theme='light']) .avatar {
        background: #000000;
        color: #ffffff;
    }

    :global([data-theme='light']) .dropdown-menu {
        background: #ffffff;
        border-color: rgba(0, 0, 0, 0.1);
    }

    :global([data-theme='light']) .menu-item {
        color: #333333;
    }

    :global([data-theme='light']) .menu-item:hover {
        background: rgba(0, 0, 0, 0.05);
    }

    /* Dropdown menu */
    .dropdown-menu {
        position: absolute;
        right: 0;
        margin-top: 0.5rem;
        width: 12rem;
        border-radius: 0.375rem;
        box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
        padding: 0.25rem 0;
        z-index: 10;
        border: 1px solid;
    }

    .menu-item {
        display: block;
        width: 100%;
        text-align: left;
        padding: 0.625rem 1rem;
        font-size: 0.875rem;
        border: none;
        background: transparent;
        cursor: pointer;
        transition: background-color 0.2s ease;
    }

    /* Hide username on mobile */
    @media (max-width: 768px) {
        .username {
            display: none;
        }
    }
</style>

{#if user}
    <div class="user-menu-wrapper">
        <button
            class="user-menu-button"
            onclick={toggleMenu}
            aria-expanded={isOpen}
            aria-haspopup="true"
        >
            <div class="avatar">
                {displayName.charAt(0).toUpperCase()}
            </div>
            <span class="username">
                {displayName}
            </span>
        </button>

        {#if isOpen}
            <div class="dropdown-menu">
                {#if isAdmin}
                    <button class="menu-item" onclick={handleAdminPanel}>
                        Admin Panel
                    </button>
                {/if}
                <button class="menu-item" onclick={handleSettings}>
                    User Settings
                </button>
                <button class="menu-item" onclick={handleLogout}>
                    Logout
                </button>
            </div>
        {/if}
    </div>
{/if}

<!-- Close menu when clicking outside -->
<svelte:window onclick={(e) => {
    const target = e.target as HTMLElement;
    if (isOpen && !target.closest('.user-menu-wrapper')) {
        closeMenu();
    }
}} />
