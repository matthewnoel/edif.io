<script lang="ts">
	import { page } from '$app/state';
	import Button from '$lib/components/Button.svelte';

	function reload() {
		location.reload();
	}
</script>

<main class="err">
	<h1>Something went wrong</h1>
	<p>
		{#if page.status === 404}
			We couldn't find that page.
		{:else}
			A new version of edif.io may have just been deployed. Refreshing usually fixes this.
		{/if}
	</p>
	<div class="actions">
		<Button label="Refresh" onclick={reload} />
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
