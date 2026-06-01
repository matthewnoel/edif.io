<script lang="ts">
	import type { Story } from '$lib/dev/stories';

	let { story }: { story: Story } = $props();

	// A fresh copy per mount so editing controls never mutates the source fixture.
	// The parent wraps this in {#key story.id} so a new instance (and fresh state)
	// is created whenever the selected story changes.
	// svelte-ignore state_referenced_locally
	let liveProps = $state<Record<string, unknown>>({ ...story.props });

	const Preview = $derived(story.component);
</script>

{#if story.controls && story.controls.length > 0}
	<div class="controls">
		{#each story.controls as ctrl (ctrl.key)}
			<label class="control">
				<span class="control-label">{ctrl.label}</span>
				{#if ctrl.kind === 'text'}
					<input
						type="text"
						value={String(liveProps[ctrl.key] ?? '')}
						oninput={(e) => (liveProps[ctrl.key] = e.currentTarget.value)}
					/>
				{:else if ctrl.kind === 'boolean'}
					<input
						type="checkbox"
						checked={Boolean(liveProps[ctrl.key])}
						onchange={(e) => (liveProps[ctrl.key] = e.currentTarget.checked)}
					/>
				{:else if ctrl.kind === 'number'}
					<input
						type="number"
						min={ctrl.min}
						max={ctrl.max}
						step={ctrl.step}
						value={Number(liveProps[ctrl.key] ?? 0)}
						oninput={(e) => (liveProps[ctrl.key] = Number(e.currentTarget.value))}
					/>
				{:else if ctrl.kind === 'select'}
					<select
						value={String(liveProps[ctrl.key] ?? '')}
						onchange={(e) => (liveProps[ctrl.key] = e.currentTarget.value)}
					>
						{#each ctrl.options as opt (opt)}
							<option value={opt}>{opt}</option>
						{/each}
					</select>
				{/if}
			</label>
		{/each}
	</div>
{/if}

<div class="stage">
	<Preview {...liveProps} />
</div>

<style>
	.controls {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem 1.25rem;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid #e5e7eb;
		background: #fafafa;
	}

	.control {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8rem;
	}

	.control-label {
		font-weight: 600;
		color: #374151;
	}

	.control input[type='text'],
	.control input[type='number'],
	.control select {
		border: 1px solid #d1d5db;
		border-radius: 0.3rem;
		padding: 0.25rem 0.4rem;
		font-size: 0.8rem;
	}

	.stage {
		position: relative;
		flex: 1;
		min-height: 0;
		overflow: auto;
	}
</style>
