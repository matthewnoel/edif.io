// Registry for the dev playground (/dev). A story is a component + initial props,
// optionally with interactive "controls" (knobs) that the playground renders as
// inputs and feeds back into the live props.
import type { Component } from 'svelte';
import { componentStories } from './stories/components';
import { pageStories } from './stories/pages';

// The registry is heterogeneous — each story holds a different component with its
// own props — so the prop type is erased here and supplied per story via `props`.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type AnyComponent = Component<any>;

export type Control =
	| { kind: 'text'; key: string; label: string }
	| { kind: 'boolean'; key: string; label: string }
	| { kind: 'number'; key: string; label: string; min?: number; max?: number; step?: number }
	| { kind: 'select'; key: string; label: string; options: string[] };

export type StoryGroup = 'Components' | 'Pages';

export type Story = {
	id: string;
	title: string;
	group: StoryGroup;
	component: AnyComponent;
	props: Record<string, unknown>;
	controls?: Control[];
};

export const stories: Story[] = [...componentStories, ...pageStories];
