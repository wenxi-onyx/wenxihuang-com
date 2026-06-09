<script lang="ts">
    import { authStore } from '$lib/stores/auth';
    import { adminApi } from '$lib/api/client';
    import ThemeToggle from '$lib/components/ThemeToggle.svelte';
    import LoginButton from '$lib/components/LoginButton.svelte';
    import { showToast } from '$lib/components/Toast.svelte';
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';

    const user = $derived($authStore.user);

    // Redirect non-admin users - reactive to auth state changes
    $effect(() => {
        // Only redirect if auth has finished loading
        if (!$authStore.loading && (!user || user.role !== 'admin')) {
            goto('/login');
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
        <header class="page-header">
            <h1>Admin Panel</h1>
            <nav class="nav-links">
                <a href="/">BACK</a>
            </nav>
        </header>

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
                    class="btn"
                >
                    {loading ? 'CREATING...' : 'CREATE USER'}
                </button>
            </form>
        </section>
    </div>
</main>
{/if}

<style>
    /* Using shared styles: buttons.css (.btn, .btn-primary), forms.css (.form-group, label, input), layout.css (.page-header, .nav-links, .section-title) */

    .admin-page {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: flex-start;
        padding: 6rem 2rem 4rem 2rem;
        min-height: 100vh;
    }

    .admin-content {
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

    .admin-section {
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

    @media (max-width: 768px) {
        .admin-page {
            padding: 5rem 1.5rem 3rem 1.5rem;
        }

        .admin-content {
            gap: 2rem;
        }

        .page-header {
            flex-direction: column;
            gap: 1rem;
            align-items: flex-start;
        }

        .admin-section {
            padding: 1.5rem;
        }
    }
</style>
