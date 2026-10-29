<!--
	Your name (#113, #469): the card at the top of the Settings menu opens it.
	Nothing is keyed on a name (ADR 0015): a Version carries a Hand, and the
	server names that Hand live, so renaming rewrites no id and moves no
	fingerprint. What it does reach, your Versions and how you sign in, is
	said in the one line under the box.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Refusal from '$lib/Refusal.svelte';
	import Screen from '$lib/shell/Screen.svelte';

	const kamosu = useKamosu();

	let name = $state<string | undefined>(undefined);
	let draft = $state('');
	let renaming = $state(false);
	let failed = $state<string | undefined>(undefined);
	/** The name just taken, said once above the box. */
	let renamedTo = $state<string | undefined>(undefined);

	$effect(() => {
		let current = true;
		kamosu
			.getPerson()
			.then((person) => {
				if (!current) return;
				name = person.name;
				draft = person.name;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	async function rename() {
		renaming = true;
		failed = undefined;
		renamedTo = undefined;
		try {
			const answer = await kamosu.renamePerson({ name: draft });
			name = answer.name;
			draft = answer.name;
			renamedTo = answer.name;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			renaming = false;
		}
	}
</script>

<Screen title={m.settings_name_page()}>
	{#if name !== undefined}
		{#if renamedTo}
			<p role="status" class="mb-3 border-l-3 border-accent bg-card px-3 py-2 text-read text-ink">
				{m.you_name_changed({ name: renamedTo })}
			</p>
		{/if}
		<form
			class="grid gap-3"
			onsubmit={(event) => {
				event.preventDefault();
				void rename();
			}}
		>
			<label class="grid gap-1 text-body text-ink">
				{m.you_name_label()}
				<input
					class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
					bind:value={draft}
					required
				/>
			</label>
			<p class="-mt-1 text-read text-ink-2">{m.you_name_explained()}</p>
			{#if failed}
				<p class="text-body text-accent" role="alert"><Refusal words={failed} /></p>
			{/if}
			<button
				class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
				disabled={renaming || draft.trim() === name}
			>
				{m.you_name_confirm()}
			</button>
		</form>
	{/if}
</Screen>
