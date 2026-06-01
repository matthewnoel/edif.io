import type { Story } from '$lib/dev/stories';
import Button from '$lib/components/Button.svelte';
import TextInput from '$lib/components/TextInput.svelte';
import Select from '$lib/components/Select.svelte';
import Checkbox from '$lib/components/Checkbox.svelte';
import RangeInput from '$lib/components/RangeInput.svelte';
import CloseButton from '$lib/components/CloseButton.svelte';
import PowerUpBadge from '$lib/components/PowerUpBadge.svelte';
import UpdateBannerView from '$lib/components/views/UpdateBannerView.svelte';
import GameSetupFormStory from '$lib/dev/GameSetupFormStory.svelte';
import RulesDialogStory from '$lib/dev/RulesDialogStory.svelte';
import { gameModes } from '$lib/dev/fixtures';

const noop = () => {};

const selectOptions = [
	{ value: 'addition', label: 'Addition' },
	{ value: 'subtraction', label: 'Subtraction' },
	{ value: 'multiplication', label: 'Multiplication' }
];

export const componentStories: Story[] = [
	// Button
	{
		id: 'button-default',
		title: 'Button',
		group: 'Components',
		component: Button,
		props: { label: 'Create Room', onclick: noop, disabled: false },
		controls: [
			{ kind: 'text', key: 'label', label: 'label' },
			{ kind: 'boolean', key: 'disabled', label: 'disabled' }
		]
	},
	// TextInput
	{
		id: 'textinput-default',
		title: 'TextInput',
		group: 'Components',
		component: TextInput,
		props: { value: '', placeholder: 'Type your answer; press return.', disabled: false },
		controls: [
			{ kind: 'text', key: 'value', label: 'value' },
			{ kind: 'text', key: 'placeholder', label: 'placeholder' },
			{ kind: 'boolean', key: 'disabled', label: 'disabled' }
		]
	},
	{
		id: 'textinput-inline-button',
		title: 'TextInput · inline Go',
		group: 'Components',
		component: TextInput,
		props: {
			value: '42',
			placeholder: 'Enter the solution; press return.',
			inlineButtonLabel: 'Go',
			inlineButtonOnclick: noop
		},
		controls: [{ kind: 'text', key: 'value', label: 'value' }]
	},
	// Select
	{
		id: 'select-default',
		title: 'Select',
		group: 'Components',
		component: Select,
		props: { value: 'addition', options: selectOptions },
		controls: [
			{
				kind: 'select',
				key: 'value',
				label: 'value',
				options: selectOptions.map((o) => o.value)
			}
		]
	},
	// Checkbox
	{
		id: 'checkbox-default',
		title: 'Checkbox',
		group: 'Components',
		component: Checkbox,
		props: { checked: false, disabled: false },
		controls: [
			{ kind: 'boolean', key: 'checked', label: 'checked' },
			{ kind: 'boolean', key: 'disabled', label: 'disabled' }
		]
	},
	// RangeInput
	{
		id: 'rangeinput-default',
		title: 'RangeInput',
		group: 'Components',
		component: RangeInput,
		props: { value: 3, min: 1, max: 6, step: 1 },
		controls: [{ kind: 'number', key: 'value', label: 'value', min: 1, max: 6, step: 1 }]
	},
	// CloseButton
	{
		id: 'closebutton-default',
		title: 'CloseButton',
		group: 'Components',
		component: CloseButton,
		props: { onclick: noop, ariaLabel: 'Close' }
	},
	// PowerUpBadge
	{
		id: 'powerupbadge-default',
		title: 'PowerUpBadge',
		group: 'Components',
		component: PowerUpBadge,
		props: {
			emoji: '\u{1F4AA}',
			label: '2x Points',
			fraction: 0.6,
			barColor: '#92400e',
			variant: 'buff'
		},
		controls: [
			{ kind: 'text', key: 'emoji', label: 'emoji' },
			{ kind: 'text', key: 'label', label: 'label' },
			{ kind: 'number', key: 'fraction', label: 'fraction', min: 0, max: 1, step: 0.05 },
			{ kind: 'select', key: 'variant', label: 'variant', options: ['offer', 'buff', 'debuff'] }
		]
	},
	// RulesDialog
	{
		id: 'rulesdialog-open',
		title: 'RulesDialog',
		group: 'Components',
		component: RulesDialogStory,
		props: {}
	},
	// UpdateBanner
	{
		id: 'updatebanner-visible',
		title: 'UpdateBanner',
		group: 'Components',
		component: UpdateBannerView,
		props: { visible: true, onrefresh: noop },
		controls: [{ kind: 'boolean', key: 'visible', label: 'visible' }]
	},
	// GameSetupForm
	{
		id: 'gamesetupform-welcome',
		title: 'GameSetupForm · welcome',
		group: 'Components',
		component: GameSetupFormStory,
		props: {
			modes: gameModes,
			initialGameMode: 'arithmetic',
			showRoomCodeInput: true,
			showServerUrl: false,
			submitLabel: 'Create Room'
		}
	},
	{
		id: 'gamesetupform-welcome-debug',
		title: 'GameSetupForm · welcome+server',
		group: 'Components',
		component: GameSetupFormStory,
		props: {
			modes: gameModes,
			initialGameMode: 'arithmetic',
			showRoomCodeInput: true,
			showServerUrl: true,
			submitLabel: 'Create Room'
		}
	},
	{
		id: 'gamesetupform-edit',
		title: 'GameSetupForm · edit',
		group: 'Components',
		component: GameSetupFormStory,
		props: {
			modes: gameModes,
			initialGameMode: 'state-abbreviations',
			showRoomCodeInput: false,
			submitLabel: 'Update Room'
		}
	},
	{
		id: 'gamesetupform-no-modes',
		title: 'GameSetupForm · no modes',
		group: 'Components',
		component: GameSetupFormStory,
		props: { modes: [], showRoomCodeInput: true, submitLabel: 'Create Room' }
	}
];
