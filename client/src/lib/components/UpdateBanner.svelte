<script lang="ts">
	import { updated } from '$app/state';
	import UpdateBannerView from '$lib/components/views/UpdateBannerView.svelte';

	function reload() {
		location.reload();
	}
</script>

<UpdateBannerView visible={updated.current} onrefresh={reload} />
