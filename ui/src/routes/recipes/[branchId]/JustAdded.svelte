<!--
	The word that a recipe just added from a web page is saved, and where
	(#650, the UX audit's L8), under its photograph where the cook is looking,
	with *Undo*: a cook who imported the wrong page, or the same one twice,
	takes it back out in one tap, and lands on Recipes told so.

	Said once. The page asks whether its recipe was just added when it opens,
	and a reload, or coming back to the recipe later, finds nothing to say.
-->
<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { m } from '#lib/paraglide/messages.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import NoticeCard from '#lib/NoticeCard.svelte';
	import Refusal from '#lib/Refusal.svelte';
	import { noteTakenOut, takeAdded } from '#lib/adding/added.js';
	import { letterOf } from '../letters';

	interface Props {
		branchId: string;
		title: string;
	}

	let { branchId, title }: Props = $props();

	const kamosu = useKamosu();

	/** Read once, as the page opens: the note is for this one visit. */
	const added = untrack(() => takeAdded(branchId));
	let undoing = $state(false);
	let refused = $state<string | undefined>(undefined);

	async function undo() {
		undoing = true;
		refused = undefined;
		try {
			await kamosu.deleteRecipe({ branch_id: branchId });
			noteTakenOut();
			await goto('/recipes', { replace: true });
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			refused = error.message;
			undoing = false;
		}
	}
</script>

{#if added}
	<NoticeCard
		class="mx-gutter mt-3"
		tone="waiting"
		title={m.added_title()}
		actions={[{ label: m.added_undo(), act: undo, disabled: undoing }]}
	>
		<p>{m.added_where({ letter: letterOf(title) })}</p>
		{#if refused}
			<p class="mt-2 text-support" role="alert"><Refusal words={refused} /></p>
		{/if}
	</NoticeCard>
{/if}
