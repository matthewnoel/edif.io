<script lang="ts">
	import { page } from '$app/state';
	import Button from '$lib/components/Button.svelte';
	import { m } from '$lib/paraglide/messages';

	function reload() {
		location.reload();
	}
</script>

<main class="err">
	<h1>{m.error_title()}</h1>
	<p>
		{#if page.status === 404}
			{m.error_not_found()}
		{:else}
			{m.error_stale_deploy()}
		{/if}
	</p>
	<div class="actions">
		<Button label={m.btn_refresh()} onclick={reload} />
	</div>
</main>

<style>
	.err {
		max-width: 32rem;
		margin: 4rem auto;
		padding: 0 1rem;
		text-align: center;
	}
	.actions {
		display: flex;
		justify-content: center;
		margin-top: 1.5rem;
	}
</style>
