<script lang="ts" context="module">
    import { writable } from 'svelte/store';

    export type ToastType = 'success' | 'error';
    export type Toast = { id: number; message: string; type: ToastType };

    let toastId = 0;
    const toasts = writable<Toast[]>([]);
    const timeouts = new Map<number, ReturnType<typeof setTimeout>>();

    export function showToast(message: string, type: ToastType = 'success') {
        const id = toastId++;
        toasts.update(t => [...t, { id, message, type }]);

        const timeout = setTimeout(() => {
            toasts.update(t => t.filter(toast => toast.id !== id));
            timeouts.delete(id);
        }, 4000);

        timeouts.set(id, timeout);
    }

    export function clearToasts() {
        timeouts.forEach(timeout => clearTimeout(timeout));
        timeouts.clear();
        toasts.set([]);
    }
</script>

<script lang="ts">
    import { onDestroy } from 'svelte';

    onDestroy(() => {
        clearToasts();
    });
</script>

<div class="toast-container">
    {#each $toasts as toast (toast.id)}
        <div class="toast toast-{toast.type}">
            {toast.message}
        </div>
    {/each}
</div>

<style>
    .toast-container {
        position: fixed;
        bottom: 2rem;
        left: 2rem;
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        z-index: 9999;
    }

    .toast {
        padding: 0.875rem 1.25rem;
        font-size: 0.875rem;
        font-weight: 300;
        letter-spacing: 0.05em;
        border: 1px solid;
        color: var(--text-primary);
        min-width: 250px;
        max-width: 400px;
        animation: slideIn 0.3s ease-out;
    }

    @keyframes slideIn {
        from {
            transform: translateX(-100%);
            opacity: 0;
        }
        to {
            transform: translateX(0);
            opacity: 1;
        }
    }

    .toast-success {
        opacity: 0.95;
    }

    :global([data-theme='dark']) .toast-success {
        border-color: rgba(255, 255, 255, 0.3);
        background: rgba(255, 255, 255, 0.1);
    }

    :global([data-theme='light']) .toast-success {
        border-color: rgba(0, 0, 0, 0.2);
        background: rgba(0, 0, 0, 0.05);
        font-weight: 200;
    }

    .toast-error {
        opacity: 0.95;
    }

    :global([data-theme='dark']) .toast-error {
        border-color: rgba(255, 255, 255, 0.3);
        background: rgba(255, 255, 255, 0.1);
    }

    :global([data-theme='light']) .toast-error {
        border-color: rgba(0, 0, 0, 0.2);
        background: rgba(0, 0, 0, 0.05);
        font-weight: 200;
    }

    @media (max-width: 768px) {
        .toast-container {
            bottom: 1.5rem;
            left: 1.5rem;
            right: 1.5rem;
        }

        .toast {
            min-width: auto;
        }
    }
</style>
