<script lang="ts">
    import { authStore } from '$lib/stores/auth';
    import { adminApi } from '$lib/api/client';
    import ThemeToggle from '$lib/components/ThemeToggle.svelte';
    import LoginButton from '$lib/components/LoginButton.svelte';
    import Toast, { showToast } from '$lib/components/Toast.svelte';
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';

    const user = $derived($authStore.user);

    // Redirect non-admin users - reactive to auth state changes
    $effect(() => {
        // Only redirect if auth has finished loading
        if (!$authStore.loading && (!user || user.role !== 'admin')) {
            goto('/');
        }
    });

    // Form state
    let username = $state('');
    let password = $state('');
    let confirmPassword = $state('');
    let firstName = $state('');
    let lastName = $state('');
    let role = $state<'admin' | 'user'>('user');
    let loading = $state(false);

    async function handleSubmit(e: Event) {
        e.preventDefault();

        // Validation
        if (password !== confirmPassword) {
            showToast('Passwords do not match', 'error');
            return;
        }

        loading = true;

        try {
            await adminApi.createUser({
                username,
                password,
                first_name: firstName.trim() || null,
                last_name: lastName.trim() || null,
                role,
            });

            showToast('User created successfully!', 'success');

            // Clear form
            username = '';
            password = '';
            confirmPassword = '';
            firstName = '';
            lastName = '';
            role = 'user';
        } catch (error) {
            showToast(error instanceof Error ? error.message : 'Failed to create user', 'error');
        } finally {
            loading = false;
        }
    }
</script>

<ThemeToggle />
<LoginButton />

{#if user?.role === 'admin'}
<main class="admin-page">
    <div class="admin-content">
        <div class="header-content">
            <h1 class="page-title">ADMIN PANEL</h1>
            <nav class="nav-links">
                <a href="/">BACK</a>
            </nav>
        </div>

        <!-- Create New User Section -->
        <section class="admin-section">
            <h2 class="section-title">CREATE NEW USER</h2>

            <form onsubmit={handleSubmit}>
                <div class="form-group">
                    <label for="username">USERNAME *</label>
                    <input
                        type="text"
                        id="username"
                        bind:value={username}
                        minlength="3"
                        maxlength="20"
                        pattern="[a-zA-Z0-9_-]+"
                        title="Username must contain only letters, numbers, underscores, and hyphens"
                        required
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

                <div class="form-group">
                    <label for="password">PASSWORD *</label>
                    <input
                        type="password"
                        id="password"
                        bind:value={password}
                        minlength="6"
                        required
                    />
                </div>

                <div class="form-group">
                    <label for="confirmPassword">CONFIRM PASSWORD *</label>
                    <input
                        type="password"
                        id="confirmPassword"
                        bind:value={confirmPassword}
                        required
                    />
                </div>

                <div class="form-group">
                    <label for="roleToggle">ROLE *</label>
                    <div class="toggle-container">
                        <span class="toggle-label" class:active={role === 'user'}>USER</span>
                        <button
                            type="button"
                            id="roleToggle"
                            class="toggle-slider"
                            class:admin={role === 'admin'}
                            onclick={() => role = role === 'user' ? 'admin' : 'user'}
                            aria-label="Toggle user role between user and admin"
                        >
                            <div class="slider-knob"></div>
                        </button>
                        <span class="toggle-label" class:active={role === 'admin'}>ADMIN</span>
                    </div>
                </div>

                <button
                    type="submit"
                    disabled={loading}
                    class="submit-btn"
                >
                    {loading ? 'CREATING...' : 'CREATE USER'}
                </button>
            </form>
        </section>
    </div>
</main>
{/if}

<Toast />

<style>
    .admin-page {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: flex-start;
        padding: 4rem 2rem;
        min-height: 100vh;
    }

    .admin-content {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 3rem;
        width: 100%;
        max-width: 600px;
        margin-top: 6rem;
    }

    .header-content {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 1rem;
    }

    .page-title {
        font-size: clamp(1.5rem, 4vw, 2rem);
        font-weight: 300;
        letter-spacing: 0.15em;
        text-align: center;
        margin: 0;
        color: var(--text-primary);
    }

    :global([data-theme='dark']) .page-title {
        font-weight: 700;
    }

    :global([data-theme='light']) .page-title {
        font-family: 'Noto Sans JP', sans-serif;
        font-weight: 100;
        letter-spacing: 0.2em;
    }

    .nav-links {
        display: flex;
        gap: 2rem;
    }

    .nav-links a {
        font-size: 0.75rem;
        font-weight: 300;
        text-transform: uppercase;
        letter-spacing: 0.1em;
        text-decoration: underline;
        text-decoration-thickness: 0.5px;
        color: var(--text-primary);
        transition: opacity 0.3s ease;
    }

    :global([data-theme='light']) .nav-links a {
        font-weight: 200;
        text-decoration-thickness: 0.5px;
    }

    .nav-links a:hover {
        opacity: 0.6;
    }

    .admin-section {
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

    input[type="text"],
    input[type="password"] {
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

    .toggle-container {
        display: flex;
        align-items: center;
        gap: 1rem;
        margin-top: 0.5rem;
    }

    .toggle-label {
        font-size: 0.75rem;
        font-weight: 300;
        text-transform: uppercase;
        letter-spacing: 0.1em;
        color: var(--text-primary);
        opacity: 0.4;
        transition: opacity 0.3s ease;
    }

    :global([data-theme='light']) .toggle-label {
        font-weight: 200;
    }

    .toggle-label.active {
        opacity: 1;
    }

    .toggle-slider {
        position: relative;
        width: 3.5rem;
        height: 1.5rem;
        border: none;
        cursor: pointer;
        transition: all 0.3s ease;
        padding: 0;
    }

    /* Dark mode: gray background */
    :global([data-theme='dark']) .toggle-slider {
        background: rgba(255, 255, 255, 0.2);
    }

    /* Light mode: light gray background */
    :global([data-theme='light']) .toggle-slider {
        background: rgba(0, 0, 0, 0.15);
    }

    .slider-knob {
        position: absolute;
        top: 0;
        left: 0;
        width: 1.5rem;
        height: 1.5rem;
        transition: transform 0.3s ease;
    }

    /* Dark mode: white knob */
    :global([data-theme='dark']) .slider-knob {
        background: #ffffff;
    }

    /* Light mode: black knob */
    :global([data-theme='light']) .slider-knob {
        background: #000000;
    }

    .toggle-slider.admin .slider-knob {
        transform: translateX(2rem);
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
        .admin-page {
            padding: 3rem 1.5rem;
        }

        .admin-content {
            margin-top: 4rem;
            gap: 2rem;
        }

        .admin-section {
            padding: 1.5rem;
        }
    }
</style>
