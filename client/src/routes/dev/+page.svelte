<script lang="ts">
	import { page } from '$app/state';
	import { stories, type StoryGroup } from '$lib/dev/stories';
	import StoryStage from '$lib/dev/StoryStage.svelte';

	const GROUP_ORDER: StoryGroup[] = ['Pages', 'Components'];

	const groups = $derived(
		GROUP_ORDER.map((group) => ({
			group,
			items: stories.filter((s) => s.group === group)
		})).filter((g) => g.items.length > 0)
	);

	const activeId = $derived(page.url.searchParams.get('story') ?? stories[0].id);
	const active = $derived(stories.find((s) => s.id === activeId) ?? stories[0]);

	const WIDTHS = [
		{ label: 'Mobile', value: '390px' },
		{ label: 'Tablet', value: '768px' },
		{ label: 'Full', value: '100%' }
	];
	let frameWidth = $state('100%');
</script>

<svelte:head>
	<title>edif.io · playground</title>
</svelte:head>

<div class="playground">
	<nav>
		<div class="brand">
			<strong class="shizuru-regular">edif.io</strong>
			<span>playground</span>
		</div>
		{#each groups as { group, items } (group)}
			<h3>{group}</h3>
			<ul>
				{#each items as s (s.id)}
					<li>
						<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- same-page query-string link -->
						<a href="?story={s.id}" class:active={s.id === active.id} data-sveltekit-noscroll>
							{s.title}
						</a>
					</li>
				{/each}
			</ul>
		{/each}
	</nav>

	<section class="content">
		<header class="toolbar">
			<div class="crumbs"><span>{active.group}</span> / <strong>{active.title}</strong></div>
			<div class="widths">
				{#each WIDTHS as w (w.value)}
					<button class:active={frameWidth === w.value} onclick={() => (frameWidth = w.value)}>
						{w.label}
					</button>
				{/each}
			</div>
		</header>
		<div class="viewport">
			<div class="frame" style:width={frameWidth} style:max-width="100%">
				{#key active.id}
					<StoryStage story={active} />
				{/key}
			</div>
		</div>
	</section>
</div>

<style>
	.playground {
		display: grid;
		grid-template-columns: 240px 1fr;
		height: 100vh;
		overflow: hidden;
	}

	nav {
		border-right: 1px solid #e5e7eb;
		overflow-y: auto;
		padding: 0 0 2rem;
		background: #fafafa;
	}

	.brand {
		display: flex;
		align-items: baseline;
		gap: 0.4rem;
		padding: 1rem 1rem 0.75rem;
		position: sticky;
		top: 0;
		background: #fafafa;
		border-bottom: 1px solid #e5e7eb;
	}

	.brand strong {
		font-size: 1.4rem;
	}

	.brand span {
		font-size: 0.75rem;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	h3 {
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: #6b7280;
		margin: 1rem 1rem 0.25rem;
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	a {
		display: block;
		padding: 0.35rem 1rem;
		font-size: 0.85rem;
		color: #111;
		text-decoration: none;
		border-left: 3px solid transparent;
	}

	a:hover {
		background: #f0f0f0;
	}

	a.active {
		background: #eef2ff;
		border-left-color: #6366f1;
		font-weight: 600;
	}

	.content {
		display: flex;
		flex-direction: column;
		min-width: 0;
		overflow: hidden;
	}

	.toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.6rem 1rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.crumbs {
		font-size: 0.85rem;
		color: #6b7280;
	}

	.crumbs strong {
		color: #111;
	}

	.widths {
		display: flex;
		gap: 0.25rem;
	}

	.widths button {
		border: 1px solid #d1d5db;
		background: #fff;
		border-radius: 0.3rem;
		padding: 0.25rem 0.6rem;
		font-size: 0.78rem;
		cursor: pointer;
	}

	.widths button.active {
		background: #111;
		color: #fff;
		border-color: #111;
	}

	.viewport {
		flex: 1;
		min-height: 0;
		overflow: auto;
		display: flex;
		justify-content: center;
		background: repeating-conic-gradient(#f6f6f6 0% 25%, #fff 0% 50%) 50% / 24px 24px;
	}

	.frame {
		display: flex;
		flex-direction: column;
		min-height: 100%;
		background: #fff;
		box-shadow: 0 0 0 1px #e5e7eb;
	}
</style>
