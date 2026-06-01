<script lang="ts">
	import { onMount } from 'svelte';
	import RulesDialog from '$lib/components/RulesDialog.svelte';

	// RulesDialog persists its dismissal in localStorage; clear it on mount so the
	// dialog always shows when this story is opened.
	onMount(() => {
		try {
			localStorage.removeItem('rulesDialogDismissed');
		} catch {
			/* ignore */
		}
	});
</script>

<div class="backdrop">
	<RulesDialog />
</div>

<style>
	.backdrop {
		min-height: 60vh;
	}
</style>
