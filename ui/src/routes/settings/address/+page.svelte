<!--
	Public address (#103, #469): a page of This instance. Changing one already
	stored, never setting the first: the first is asked at the first Share
	Link and stays there (ADR 0029), since a question about domains does not
	belong in front of somebody who only wanted to send a recipe.

	Kamosu keeps only a Share Link's hash, so a link already sent is a string
	in somebody else's phone that nothing here can reach. The line says so.
	A passkey works at the address it was made at and no other (#370), so a
	change that would strand one asks first, and says how many.
-->
<script lang="ts">
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Confirm from '$lib/Confirm.svelte';
	import { Acting } from '$lib/operator/acting.svelte';
	import OperatorOnly from '../OperatorOnly.svelte';

	const kamosu = useKamosu();
	const acting = new Acting();

	let address = $state<string | null>(null);
	/** How many passkeys work at that address, which a new one would strand. */
	let passkeys = $state(0);
	let typed = $state('');
	let saved = $state(false);
	let asking = $state(false);

	async function load() {
		try {
			const answer = await kamosu.getPublicAddress({});
			address = answer.public_address;
			passkeys = answer.passkeys;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
		}
	}

	const save = () =>
		acting.act(async () => {
			address = (await kamosu.setPublicAddress({ public_address: typed })).public_address;
			passkeys = (await kamosu.getPublicAddress({})).passkeys;
			typed = '';
			saved = true;
			asking = false;
		});
</script>

<OperatorOnly title={m.settings_address()} onallowed={load}>
	<p class="mb-3 text-read text-ink-2">{m.operator_address_warning()}</p>
	<div class="rounded-sm border border-rule bg-card p-3">
		<p class="text-body text-ink">{address ?? m.operator_address_none()}</p>
		<!-- A form, so Enter in the box saves the address (#203). `novalidate`
		     leaves refusing one to the Core. -->
		<form
			novalidate
			onsubmit={(event) => {
				event.preventDefault();
				saved = false;
				if (passkeys > 0) asking = true;
				else void save();
			}}
		>
			<label class="mt-3 block">
				<span class="sr-only">{m.settings_address()}</span>
				<input
					type="url"
					bind:value={typed}
					placeholder="https://kamosu.example"
					class="min-h-12 w-full rounded-sm border border-rule bg-ground px-3 text-body"
				/>
			</label>
			<button
				type="submit"
				disabled={acting.working || typed.trim() === ''}
				class="mt-2 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
				text-accent disabled:opacity-60"
			>
				{m.operator_address_save()}
			</button>
		</form>
		{#if saved}
			<p class="mt-2 text-body text-ink" role="status">{m.operator_address_saved()}</p>
		{/if}
		{#if acting.said && !asking}
			<p class="mt-2 text-body text-support" role="alert"><Refusal words={acting.said} /></p>
		{/if}
	</div>
</OperatorOnly>

{#if asking}
	<Confirm
		title={m.operator_address_change_title()}
		consequence={passkeys === 1
			? m.operator_address_passkeys_one()
			: m.operator_address_passkeys({ count: passkeys })}
		act={m.operator_address_save()}
		busy={acting.working}
		failed={acting.said}
		run={() => void save()}
		cancel={() => {
			asking = false;
			acting.said = undefined;
		}}
	/>
{/if}
