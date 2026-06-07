<script lang="ts">
	import '../app.css';
	import UpdateBanner from '$lib/components/UpdateBanner.svelte';
	import faviconSvg from '$lib/assets/favicon.svg';
	import faviconIco from '$lib/assets/favicon.ico';
	import favicon32 from '$lib/assets/favicon-32x32.png';
	import favicon192 from '$lib/assets/favicon-192x192.png';
	import favicon512 from '$lib/assets/favicon-512x512.png';
	import appleTouchIcon from '$lib/assets/apple-touch-icon.png';
	import { getLocale } from '$lib/paraglide/runtime';

	let { children } = $props();

	// Keep <html lang> in sync with the viewer language (client-only; SSR keeps
	// the base locale from app.html).
	$effect(() => {
		document.documentElement.lang = getLocale();
	});
</script>

<svelte:head>
	<link rel="icon" href={faviconSvg} type="image/svg+xml" />
	<link rel="icon" href={faviconIco} sizes="32x32" type="image/x-icon" />
	<link rel="icon" href={favicon32} sizes="32x32" type="image/png" />
	<link rel="icon" href={favicon192} sizes="192x192" type="image/png" />
	<link rel="icon" href={favicon512} sizes="512x512" type="image/png" />
	<link rel="apple-touch-icon" href={appleTouchIcon} />
</svelte:head>

{@render children()}
<UpdateBanner />
