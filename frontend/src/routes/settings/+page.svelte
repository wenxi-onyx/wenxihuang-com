<script lang="ts">
    import { authStore } from '$lib/stores/auth';
    import { userApi } from '$lib/api/client';
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import ThemeToggle from '$lib/components/ThemeToggle.svelte';
    import LoginButton from '$lib/components/LoginButton.svelte';
    import { showToast } from '$lib/components/Toast.svelte';

    const user = $derived($authStore.user);

    // Redirect unauthenticated users - reactive to auth state changes
    $effect(() => {
        // Only redirect if auth has finished loading
        if (!$authStore.loading && !user) {
            goto('/login');
        }
    });

    // Profile form state
    let username = $state('');
    let firstName = $state('');
    let lastName = $state('');
    let profileLoading = $state(false);

    // Password form state
    let currentPassword = $state('');
    let newPassword = $state('');
    let confirmPassword = $state('');
    let passwordLoading = $state(false);

    // API Key form state
    let anthropicApiKey = $state('');
    let apiKeyPreview = $state('');
    let hasApiKey = $state(false);
    let apiKeyLoading = $state(false);
    let showApiKey = $state(false);

    // Initialize form with user data
    onMount(async () => {
        if (user) {
            username = user.username || '';
            firstName = user.first_name || '';
            lastName = user.last_name || '';

            // Load API key status
            try {
                const apiKeyResponse = await userApi.getApiKey('anthropic');
                hasApiKey = apiKeyResponse.has_key;
                apiKeyPreview = apiKeyResponse.api_key_preview;
            } catch (error) {
                // Silently fail - user might not have API key set yet
            }
        }
    });

    function goBack() {
        // Check if there's a previous page and it's not the same as current
        const referrer = document.referrer;
        const currentUrl = window.location.href;

        // If there's a referrer and it's different from current page, go back
        if (referrer && referrer !== currentUrl && !referrer.includes('/settings')) {
            window.history.back();
        } else {
            // Otherwise, go to home
            goto('/');
        }
    }

    function handleProfileFormKeydown(e: KeyboardEvent) {
        // Only allow Enter to submit if focus is on the last input (lastName)
        if (e.key === 'Enter' && (e.target as HTMLElement).id !== 'lastName') {
            e.preventDefault();
        }
    }

    async function handleProfileUpdate(e: Event) {
        e.preventDefault();
        profileLoading = true;

        // Trim fields before sending
        const trimmedUsername = username.trim();
        const trimmedFirstName = firstName.trim();
        const trimmedLastName = lastName.trim();

        try {
            const response = await userApi.updateProfile({
                username: trimmedUsername,
                first_name: trimmedFirstName || null,
                last_name: trimmedLastName || null,
            });

            // Update user in store and local state
            authStore.updateUser(response.user);
            username = response.user.username;
            firstName = response.user.first_name || '';
            lastName = response.user.last_name || '';
            showToast('Profile updated successfully!', 'success');
        } catch (error) {
            showToast(error instanceof Error ? error.message : 'Failed to update profile', 'error');
        } finally {
            profileLoading = false;
        }
    }

    function handlePasswordFormKeydown(e: KeyboardEvent) {
        // Only allow Enter to submit if focus is on the last input (confirmPassword)
        if (e.key === 'Enter' && (e.target as HTMLElement).id !== 'confirmPassword') {
            e.preventDefault();
        }
    }

    async function handlePasswordChange(e: Event) {
        e.preventDefault();
        // Validation
        if (newPassword.length < 6) {
            showToast('Password must be at least 6 characters', 'error');
            return;
        }

        if (newPassword === currentPassword) {
            showToast('New password must be different from current password', 'error');
            return;
        }

        if (newPassword !== confirmPassword) {
            showToast('Passwords do not match', 'error');
            return;
        }

        passwordLoading = true;

        try {
            await userApi.changePassword({
                current_password: currentPassword,
                new_password: newPassword,
            });

            showToast('Password changed successfully!', 'success');
            // Clear form
            currentPassword = '';
            newPassword = '';
            confirmPassword = '';
        } catch (error) {
            showToast(error instanceof Error ? error.message : 'Failed to change password', 'error');
        } finally {
            passwordLoading = false;
        }
    }

    async function handleApiKeySave(e: Event) {
        e.preventDefault();
        apiKeyLoading = true;

        try {
            await userApi.saveApiKey({
                provider: 'anthropic',
                api_key: anthropicApiKey,
            });

            // Reload API key status
            const apiKeyResponse = await userApi.getApiKey('anthropic');
            hasApiKey = apiKeyResponse.has_key;
            apiKeyPreview = apiKeyResponse.api_key_preview;

            // Clear the input field
            anthropicApiKey = '';
            showApiKey = false;

            showToast('API key saved successfully!', 'success');
        } catch (error) {
            showToast(error instanceof Error ? error.message : 'Failed to save API key', 'error');
        } finally {
            apiKeyLoading = false;
        }
    }

    async function handleApiKeyDelete() {
        if (!confirm('Are you sure you want to delete your Anthropic API key?')) {
            return;
        }

        apiKeyLoading = true;

        try {
            await userApi.deleteApiKey('anthropic');
            hasApiKey = false;
            apiKeyPreview = '';
            anthropicApiKey = '';
            showToast('API key deleted successfully!', 'success');
        } catch (error) {
            showToast(error instanceof Error ? error.message : 'Failed to delete API key', 'error');
        } finally {
            apiKeyLoading = false;
        }
    }

    function handleApiKeyFormKeydown(e: KeyboardEvent) {
        // Only allow Enter to submit if focus is on the API key input
        if (e.key === 'Enter' && (e.target as HTMLElement).id !== 'anthropicApiKey') {
            e.preventDefault();
        }
    }
</script>

<ThemeToggle />
<LoginButton />

<main class="settings-page">
    <div class="settings-content">
        <header class="page-header">
            <h1>User Settings</h1>
            <nav class="nav-links">
                <button onclick={goBack} class="nav-link">BACK</button>
            </nav>
        </header>

        <!-- Profile Information Section -->
        <section class="settings-section">
            <h2 class="section-title">PROFILE</h2>

            <form onsubmit={handleProfileUpdate} onkeydown={handleProfileFormKeydown}>
                <div class="form-group">
                    <label for="username">USERNAME</label>
                    <input
                        type="text"
                        id="username"
                        bind:value={username}
                        required
                        minlength="3"
                        maxlength="20"
                        pattern="[a-zA-Z0-9_-]+"
                        title="Username must contain only letters, numbers, underscores, and hyphens"
                    />
                </div>

                <div class="form-group">
                    <label for="firstName">FIRST NAME</label>
                    <input
                        type="text"
                        id="firstName"
                        bind:value={firstName}
                        maxlength="50"
                    />
                </div>

                <div class="form-group">
                    <label for="lastName">LAST NAME</label>
                    <input
                        type="text"
                        id="lastName"
                        bind:value={lastName}
                        maxlength="50"
                    />
                </div>

                <button
                    type="submit"
                    disabled={profileLoading}
                    class="btn"
                >
                    {profileLoading ? 'SAVING...' : 'SAVE PROFILE'}
                </button>
            </form>
        </section>

        <!-- Change Password Section -->
        <section class="settings-section">
            <h2 class="section-title">CHANGE PASSWORD</h2>

            <form onsubmit={handlePasswordChange} onkeydown={handlePasswordFormKeydown}>
                <div class="form-group">
                    <label for="currentPassword">CURRENT PASSWORD</label>
                    <input
                        type="password"
                        id="currentPassword"
                        bind:value={currentPassword}
                        required
                    />
                </div>

                <div class="form-group">
                    <label for="newPassword">NEW PASSWORD</label>
                    <input
                        type="password"
                        id="newPassword"
                        bind:value={newPassword}
                        required
                        minlength="6"
                    />
                </div>

                <div class="form-group">
                    <label for="confirmPassword">CONFIRM NEW PASSWORD</label>
                    <input
                        type="password"
                        id="confirmPassword"
                        bind:value={confirmPassword}
                        required
                    />
                </div>

                <button
                    type="submit"
                    disabled={passwordLoading}
                    class="btn"
                >
                    {passwordLoading ? 'CHANGING...' : 'CHANGE PASSWORD'}
                </button>
            </form>
        </section>

        <!-- API Keys Section -->
        <section class="settings-section">
            <h2 class="section-title">API KEYS</h2>

            <form onsubmit={handleApiKeySave} onkeydown={handleApiKeyFormKeydown}>
                {#if hasApiKey && !showApiKey}
                    <div class="form-group">
                        <label for="anthropicApiKey">ANTHROPIC API KEY</label>
                        <div class="api-key-status">
                            <p class="api-key-preview">Current key: {apiKeyPreview}</p>
                        </div>
                    </div>

                    <button
                        type="button"
                        onclick={() => showApiKey = true}
                        class="btn"
                    >
                        UPDATE KEY
                    </button>
                {:else}
                    <div class="form-group">
                        <label for="anthropicApiKey">ANTHROPIC API KEY</label>
                        <input
                            type="password"
                            id="anthropicApiKey"
                            bind:value={anthropicApiKey}
                            placeholder="sk-ant-..."
                            required={!hasApiKey}
                        />
                    </div>

                    <button
                        type="submit"
                        disabled={apiKeyLoading || !anthropicApiKey}
                        class="btn"
                    >
                        {apiKeyLoading ? 'SAVING...' : hasApiKey ? 'UPDATE KEY' : 'SAVE KEY'}
                    </button>
                {/if}
            </form>
        </section>
    </div>
</main>

<style>
    /* Using shared styles: buttons.css (.btn, .btn-primary), forms.css (.form-group, label, input), layout.css (.page-header, .nav-links, .section-title) */

    .settings-page {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: flex-start;
        padding: 6rem 2rem 4rem 2rem;
        min-height: 100vh;
    }

    .settings-content {
        display: flex;
        flex-direction: column;
        gap: 3rem;
        width: 100%;
        max-width: 600px;
        margin-top: 0;
    }

    .page-header {
        margin-bottom: 0;
        width: 100%;
    }

    .nav-link {
        background: transparent;
        border: none;
        cursor: pointer;
        padding: 0;
    }

    .settings-section {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
        padding: 2rem;
        border: 1px solid var(--border-subtle);
        background: transparent;
        width: 100%;
    }

    form {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
    }

    .hint {
        font-size: 0.75rem;
        color: var(--text-secondary);
        margin: 0;
        opacity: 0.7;
    }

    .hint a {
        color: var(--text-primary);
        text-decoration: underline;
    }

    .api-key-preview {
        font-family: monospace;
        padding: 0.75rem;
        background: var(--bg-secondary);
        border: 1px solid var(--border-subtle);
        margin: 0;
    }

    .button-group {
        display: flex;
        gap: 0.75rem;
        flex-wrap: wrap;
    }

    .btn-secondary {
        background: transparent;
        color: var(--text-primary);
        border: 1px solid var(--border-subtle);
    }

    .btn-secondary:hover:not(:disabled) {
        background: var(--bg-secondary);
    }

    .btn-danger {
        background: transparent;
        color: #dc2626;
        border: 1px solid #dc2626;
    }

    .btn-danger:hover:not(:disabled) {
        background: #dc2626;
        color: white;
    }

    @media (max-width: 768px) {
        .settings-page {
            padding: 5rem 1.5rem 3rem 1.5rem;
        }

        .settings-content {
            gap: 2rem;
        }

        .page-header {
            flex-direction: column;
            gap: 1rem;
            align-items: flex-start;
        }

        .settings-section {
            padding: 1.5rem;
        }
    }
</style>
