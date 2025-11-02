<script lang="ts">
    import { authStore } from '$lib/stores/auth';

    let isOpen = $state(false);
    const user = $derived($authStore.user);

    function toggleMenu() {
        isOpen = !isOpen;
    }

    function handleLogout() {
        authStore.logout();
        isOpen = false;
    }

    function closeMenu() {
        isOpen = false;
    }
</script>

{#if user}
    <div class="relative">
        <button
            onclick={toggleMenu}
            class="flex items-center space-x-2 hover:opacity-80 focus:outline-none transition-opacity"
        >
            <div class="w-8 h-8 rounded-full bg-indigo-600 flex items-center justify-center text-white font-semibold">
                {user.username.charAt(0).toUpperCase()}
            </div>
            <span class="hidden md:block font-medium">{user.username}</span>
            <span class="px-2 py-1 text-xs font-semibold rounded-full {user.role === 'admin' ? 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300' : 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300'}">
                {user.role}
            </span>
        </button>

        {#if isOpen}
            <div class="absolute right-0 mt-2 w-48 bg-white dark:bg-gray-800 rounded-md shadow-lg py-1 z-10 border border-gray-200 dark:border-gray-700">
                <button
                    onclick={handleLogout}
                    class="block w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700"
                >
                    Sign out
                </button>
            </div>
        {/if}
    </div>
{/if}

<!-- Close menu when clicking outside -->
<svelte:window onclick={(e) => {
    const target = e.target as HTMLElement;
    if (isOpen && !target.closest('.relative')) {
        closeMenu();
    }
}} />
