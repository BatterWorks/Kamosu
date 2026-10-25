<!--
	A Demo's strip (#407, option B chosen on 23 October 2026): one line at the
	foot of every screen saying this is a demo, with the way to installing.

	"A Kamosu demo" brings the welcome card back at the top of Home, which is
	where it was set (option B of the card), and "Install your own" leads to the
	README's Quick start. The shell decides where it stands: on the tab bar on
	the phone, at the foot of the window beside the sidebar, and under Back and
	Start cooking on the cooking screen, in that screen's own colours. A sheet
	and its dimming cover it, since they stand above everything the shell draws.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { INSTALL_KAMOSU, openWelcomeAgain } from '$lib/demo.svelte';

	interface Props {
		/**
		 * Where the shell stands it: on the tab bar, which holds it; at the foot
		 * of the window beside the sidebar; or under the cooking screen's own
		 * buttons, above that screen and in its colours.
		 */
		place: 'tab-bar' | 'wide' | 'cooking';
	}

	let { place }: Props = $props();

	/**
	 * Each place: where it stands, its ground, and the ink of its two
	 * controls. At the foot of the window it is fixed there, a strip and not a
	 * sheet, so nothing rises; on the tab bar the bar holds it.
	 */
	const PLACES = {
		'tab-bar': {
			stands: 'bg-ground-2 text-ink-2',
			what: 'text-ink',
			install: 'text-accent',
		},
		wide: {
			stands: 'fixed bottom-0 inset-x-0 z-20 demo-strip-at-foot bg-ground-2 text-ink-2',
			what: 'text-ink',
			install: 'text-accent',
		},
		cooking: {
			stands:
				'fixed bottom-0 inset-x-0 z-30 demo-strip-at-foot bg-cook-panel text-cook-ink-2 [--demo-strip-rule:var(--color-cook-rule)]',
			what: 'text-cook-ink',
			install: 'text-cook-ink',
		},
	} as const;

	const placed = $derived(PLACES[place]);
</script>

<div class={['demo-strip', placed.stands]} data-demo-strip>
	<button
		type="button"
		class={['demo-strip-control underline decoration-dotted', placed.what]}
		onclick={() => {
			openWelcomeAgain();
			void goto('/');
		}}
	>
		{m.demo_name()}
	</button>
	<span aria-hidden="true">·</span>
	<a
		href={INSTALL_KAMOSU}
		target="_blank"
		rel="noopener"
		class={['demo-strip-control underline', placed.install]}
	>
		{m.demo_strip_install()}<span aria-hidden="true">&nbsp;→</span>
	</a>
</div>
