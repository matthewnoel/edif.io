<script lang="ts">
	let {
		emoji,
		label,
		fraction,
		barColor,
		variant,
		labelColor
	}: {
		emoji: string;
		label: string;
		fraction: number;
		barColor: string;
		variant: 'offer' | 'buff' | 'debuff';
		labelColor?: string;
	} = $props();
</script>

<div class="badge {variant}">
	<span class="emoji">{emoji}</span>
	<span class="label" style:color={labelColor}>{label}</span>
	<div class="bar" style:background={barColor} style:width="{fraction * 100}%"></div>
</div>

<style>
	.badge {
		position: relative;
		display: flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.85rem;
		font-weight: 600;
		padding: 0.25rem 0.5rem 0.45rem;
		border-radius: 0.4rem;
		overflow: hidden;
	}

	.badge.offer {
		background: #f3f4f6;
	}

	.badge.buff {
		background: #fef3c7;
		color: #92400e;
		font-size: 0.9rem;
		font-weight: 700;
	}

	.badge.debuff {
		background: #dbeafe;
		color: #1e40af;
		font-size: 0.9rem;
		font-weight: 700;
	}

	.emoji {
		font-size: 1.1rem;
		line-height: 1;
	}

	.label {
		white-space: nowrap;
	}

	.bar {
		position: absolute;
		left: 0;
		bottom: 0;
		height: 3px;
		border-radius: 0 0 0.4rem 0.4rem;
		transition: width 60ms linear;
	}
</style>
