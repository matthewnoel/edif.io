<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import { m } from '$lib/paraglide/messages';

	interface Props {
		visible: boolean;
		onrefresh: () => void;
	}

	let { visible, onrefresh }: Props = $props();
</script>

{#if visible}
	<div class="banner" role="status" aria-live="polite">
		<span>{m.update_available()}</span>
		<Button label={m.btn_refresh()} onclick={onrefresh} />
	</div>
{/if}

<style>
	.banner {
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		display: flex;
		gap: 1rem;
		align-items: center;
		justify-content: center;
		padding: 0.6rem 1rem;
		background: #fffbe6;
		border-top: 2px solid black;
		z-index: 9999;
	}
</style>
