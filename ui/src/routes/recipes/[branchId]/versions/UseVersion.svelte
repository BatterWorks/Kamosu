<!--
	Taking an older Version whole (#354): putting it back as the recipe,
	starting a variation from it, or, for somebody who does not write the
	recipe, keeping it as their own copy.

	CHOICE C OF 18 OCTOBER 2026, WITH CONFIRMATION. The acts are tiles at the
	foot of the older Version's page, and each opens this sheet, which says
	what will happen before it does. Putting back is confirmed here even
	though nothing is lost by it: the recipe the cook is standing in changes
	under them, and the sheet is where they are told the Thread keeps it.

	THE CORE DOES THE WORK. Each act is one call naming the Branch and the
	Version; the screen never rebuilds the content and saves it.

	A NAME ONLY WHERE ONE IS NEEDED. A variation always has one, since nothing
	else tells it from the recipe. A kept copy needs one only where the
	reader's Cookbook already holds its own copy of this recipe, which the
	Core answers as `keeping_needs_name`, and the sheet says why.

	WHAT THE CORE REFUSES IS SAID AS IT CAME, as in `RenameSheet`. A request
	that never reached Kamosu gets a plain failure line.
-->
<script lang="ts" module>
	export type Act = 'back' | 'variation' | 'keep';
</script>

<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { ChangedSinceOutput } from '$lib/api/catalogue';
	import { joinedNames } from '$lib/cookbook';

	interface Props {
		act: Act;
		branchId: string;
		versionId: string;
		/** The older Version laid over the recipe, as the page read it. */
		compared: ChangedSinceOutput;
		/** When the Version was saved, as the page says it. */
		when: string;
		/** Where the act landed: the Branch the reader goes to now. */
		onDone: (branchId: string) => void;
		onClose: () => void;
	}

	let { act, branchId, versionId, compared, when, onDone, onClose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();

	let name = $state('');
	let working = $state(false);
	let refused = $state<string | null>(null);

	/** The recipe as it stands is what each sentence names. */
	const title = $derived(compared.mine.content.title);
	/** `?? false` because a phone may hold an answer read before #354. */
	const named = $derived(
		act === 'variation' || (act === 'keep' && (compared.keeping_needs_name ?? false)),
	);

	const heading = $derived(
		act === 'back'
			? m.older_put_back()
			: act === 'variation'
				? m.older_variation()
				: m.older_keep(),
	);

	const said = $derived.by(() => {
		if (act === 'back') return m.older_put_back_said({ title, when });
		if (act === 'variation') return m.older_variation_said({ title });
		if (named) return m.older_keep_named({ title });
		const whose = compared.mine;
		if (whose.mine && whose.arrived) return m.older_keep_said_sent({ title, when });
		const names = joinedNames(whose.cookbook.authors.map((author) => author.name));
		return m.older_keep_said_theirs({ title, when, names });
	});

	const does = $derived(
		act === 'back'
			? m.older_put_back_do()
			: act === 'variation'
				? m.write_variation_do()
				: m.older_keep_do(),
	);

	async function run() {
		if (working || (named && name.trim() === '')) return;
		working = true;
		refused = null;
		try {
			if (act === 'back') {
				await kamosu.putBackVersion({ branch_id: branchId, version_id: versionId });
				onDone(branchId);
			} else if (act === 'variation') {
				const started = await kamosu.startVariation({
					branch_id: branchId,
					name: name.trim(),
					version_id: versionId,
				});
				onDone(started.branch_id);
			} else {
				const kept = await kamosu.keepVersionAsCopy({
					branch_id: branchId,
					version_id: versionId,
					...(named ? { name: name.trim() } : {}),
				});
				onDone(kept.branch_id);
			}
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			refused = error.reached ? error.message : m.older_act_failed();
		} finally {
			working = false;
		}
	}
</script>

<SheetFrame
	labelledby="{uid}-title"
	tall={78}
	class="overflow-y-auto bg-ground px-gutter pt-4"
	onclose={() => {
		if (!working) onClose();
	}}
	onconfirm={() => void run()}
>
	<h3 id="{uid}-title" class="text-label text-ink-2 uppercase">{heading}</h3>
	<p class="mt-2 text-body">{said}</p>
	{#if named}
		<label class="mt-4 block">
			<span class="block text-label text-ink-2 uppercase">{m.write_variation_name()}</span>
			<input
				bind:value={name}
				data-sheet-focus
				class="mt-1 w-full rounded-sm border border-rule bg-card px-3 py-2 text-body"
			/>
		</label>
	{/if}
	{#if refused}
		<p class="mt-3 text-read text-support" role="alert">{refused}</p>
	{/if}
	<button
		type="button"
		class="mt-4 block w-full rounded-sm bg-accent p-4 text-center font-display text-body text-on-accent disabled:opacity-60"
		disabled={working || (named && name.trim() === '')}
		onclick={run}
	>
		{does}
	</button>
	<button
		type="button"
		class="mt-2 mb-4 block w-full rounded-sm border border-rule p-4 text-center font-display text-body text-ink-2"
		disabled={working}
		onclick={onClose}
	>
		{m.write_back()}
	</button>
</SheetFrame>
