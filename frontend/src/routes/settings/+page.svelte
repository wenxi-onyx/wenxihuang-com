<script lang="ts">
    import { authStore } from '$lib/stores/auth';
    import { userApi } from '$lib/api/client';
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import ThemeToggle from '$lib/components/ThemeToggle.svelte';
    import LoginButton from '$lib/components/LoginButton.svelte';
    import Toast, { showToast } from '$lib/components/Toast.svelte';

    const user = $derived($authStore.user);

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

    // Initialize form with user data
    onMount(() => {
        if (user) {
            username = user.username || '';
            firstName = user.first_name || '';
            lastName = user.last_name || '';
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
            <h2 class="section-title">PROFILE INFORMATION</h2>

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
                    class="submit-btn"
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
                    class="submit-btn"
                >
                    {passwordLoading ? 'CHANGING...' : 'CHANGE PASSWORD'}
                </button>
            </form>
        </section>
    </div>
</main>

<Toast />

<style>
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
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 0;
        padding-bottom: 1rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.1);
        width: 100%;
    }

    .page-header h1 {
        font-size: clamp(1.5rem, 4vw, 2.5rem);
        font-weight: 300;
        letter-spacing: 0.1em;
        text-transform: uppercase;
        margin: 0;
        color: var(--text-primary);
    }

    .nav-links {
        display: flex;
        gap: 2rem;
    }

    .nav-link {
        font-size: 0.875rem;
        font-weight: 300;
        letter-spacing: 0.1em;
        text-transform: uppercase;
        text-decoration: none;
        color: inherit;
        opacity: 0.7;
        transition: opacity 0.2s ease;
        background: transparent;
        border: none;
        cursor: pointer;
        padding: 0;
    }

    .nav-link:hover {
        opacity: 1;
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

    .section-title {
        font-size: 0.875rem;
        font-weight: 300;
        letter-spacing: 0.1em;
        color: var(--text-primary);
        margin: 0 0 1rem 0;
        opacity: 0.8;
    }

    :global([data-theme='dark']) .section-title {
        font-weight: 500;
    }

    :global([data-theme='light']) .section-title {
        font-weight: 200;
    }

    form {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
    }

    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    label {
        font-size: 0.75rem;
        font-weight: 300;
        text-transform: uppercase;
        letter-spacing: 0.1em;
        color: var(--text-primary);
        opacity: 0.7;
    }

    :global([data-theme='light']) label {
        font-weight: 200;
    }

    input {
        padding: 0.75rem 1rem;
        font-size: 1rem;
        line-height: 1.5;
        font-family: inherit;
        background: transparent;
        color: var(--text-primary);
        border: 1px solid var(--border-subtle);
        outline: none;
        transition: border-color 0.2s ease, opacity 0.2s ease;
    }

    input:focus {
        border-color: var(--border-active);
    }

    input:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    input::placeholder {
        color: var(--text-primary);
        opacity: 0.3;
    }

    .hint {
        font-size: 0.75rem;
        color: var(--text-secondary);
        margin: 0;
        opacity: 0.7;
    }

    .submit-btn {
        margin-top: 0.5rem;
        padding: 0.875rem 2rem;
        font-size: 0.875rem;
        font-weight: 300;
        text-transform: uppercase;
        letter-spacing: 0.1em;
        background: transparent;
        color: var(--text-primary);
        border: 1px solid;
        cursor: pointer;
        transition: all 0.3s ease;
    }

    :global([data-theme='dark']) .submit-btn {
        border-color: #ffffff;
        font-weight: 500;
    }

    :global([data-theme='light']) .submit-btn {
        border-color: #000000;
        font-weight: 200;
    }

    .submit-btn:hover:not(:disabled) {
        opacity: 1;
    }

    :global([data-theme='dark']) .submit-btn:hover:not(:disabled) {
        background: #ffffff;
        color: #000000;
    }

    :global([data-theme='light']) .submit-btn:hover:not(:disabled) {
        background: #000000;
        color: #ffffff;
    }

    .submit-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
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
