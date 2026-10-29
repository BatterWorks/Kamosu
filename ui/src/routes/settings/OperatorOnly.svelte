<!--
	A page of This instance (#469), for an Operator alone. It asks
	`list_accounts` and, refused, shows what a room that is not here shows
	(ADR 0040). It does not explain what it would have held. A Kamosu that
	could not answer is no refusal: its words are shown instead, since an
	empty room would lie to an Operator about why they cannot see their own
	instance.

	**ADR 0007 draws the line these pages must not cross.** The Operator
	administers and does not read: no view-as-user, no window onto another
	Person's recipes or Attempts. `list_accounts` answers no question about a
	recipe, an Attempt or a Kitchen, so the boundary lives in the Core and not
	in these pages' restraint.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import Screen from '$lib/shell/Screen.svelte';
	import { asksWhetherAdministering, type Administering } from '$lib/operator/administering';

	interface Props {
		title: string;
		/** The page, once the instance has agreed this Person administers it. */
		children: Snippet;
		/**
		 * Called once the instance has agreed, so a page asks nothing an
		 * Operator alone may ask until then.
		 */
		onallowed?: () => void;
	}

	let { title, children, onallowed }: Props = $props();

	const kamosu = useKamosu();

	let answer = $state<Administering | undefined>(undefined);

	$effect(() => {
		let current = true;
		asksWhetherAdministering(kamosu).then((asked) => {
			if (!current) return;
			answer = asked;
			if (asked.may === true) onallowed?.();
		});
		return () => {
			current = false;
		};
	});
</script>

<Screen {title}>
	{#if answer === undefined}
		<p class="text-body text-ink-2">{m.loading()}</p>
	{:else if answer.may === false}
		<p class="text-body text-ink-2">{m.operator_not_here()}</p>
	{:else if answer.may === 'unknown'}
		<p class="text-body text-support" role="alert"><Refusal words={answer.failed} /></p>
	{:else}
		{@render children()}
	{/if}
</Screen>
