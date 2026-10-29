<!--
	Keys for agents (#469): the Access Keys an agent or the phone's Shortcut
	uses to act as you (ADR 0031, ADR 0055), each ending on its own, and the
	form that makes one.

	Only an Operator is offered an Operator key (#369), and choosing one asks
	first, with what that hands an agent, rather than saying it under the
	choice before anybody presses anything.
-->
<script lang="ts">
	import { aMoment } from '$lib/dates';
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import Confirm from '$lib/Confirm.svelte';
	import OneTimeSecret from '$lib/OneTimeSecret.svelte';
	import { asksWhetherAdministering } from '$lib/operator/administering';
	import type { ListAccessKeysOutput } from '$lib/api/catalogue';

	const kamosu = useKamosu();

	type AccessKey = ListAccessKeysOutput['access_keys'][number];

	let signedIn = $state(false);
	let accessKeys = $state<AccessKey[]>([]);
	let mintedSecret = $state<string | undefined>(undefined);
	let newKeyName = $state('');
	let newKeyReadOnly = $state(false);
	/** Whether the key being made acts as Operator too (#369): an Operator's choice alone. */
	let newKeyOperator = $state(false);
	let minting = $state(false);
	let failed = $state<string | undefined>(undefined);
	/** Making an Operator key, asked first. */
	let askingOperator = $state(false);
	/** Whether this instance has a Runner, as the key list last read it (#452). */
	let hasRunner = $state(false);
	let mayAdminister = $state(false);

	async function load() {
		try {
			accessKeys = (await kamosu.listAccessKeys()).access_keys.filter((key) => !key.revoked);
			signedIn = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			signedIn = false;
			return;
		}
		// Where a phone's key sends what it shares is asked only where there
		// is an agent to send it to (#452).
		if (accessKeys.some((key) => key.add_only))
			kamosu
				.runnerStatus({})
				.then((status) => (hasRunner = status.has_runner))
				.catch((error: unknown) => {
					if (!(error instanceof OperationError)) throw error;
				});
	}

	$effect(() => {
		load();
	});

	$effect(() => {
		let current = true;
		asksWhetherAdministering(kamosu).then((answer) => {
			if (current) mayAdminister = answer.may === true;
		});
		return () => {
			current = false;
		};
	});

	function whenLastUsed(lastUsedAt: string | null): string {
		return lastUsedAt ? m.access_last_used({ when: aMoment(lastUsedAt) }) : m.access_never_used();
	}

	/** Where what one of the phone's keys shares goes (#452). */
	async function chooseKeyAgent(accessKeyId: string, on: boolean) {
		failed = undefined;
		try {
			await kamosu.setKeyAgentChoice({ access_key_id: accessKeyId, everything_to_agent: on });
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		}
		await load();
	}

	async function endKey(accessKeyId: string) {
		await kamosu.revokeAccessKey({ access_key_id: accessKeyId });
		await load();
	}

	async function mint() {
		minting = true;
		failed = undefined;
		try {
			const created = await kamosu.mintAccessKey({
				name: newKeyName,
				read_only: newKeyReadOnly,
				operator: mayAdminister && newKeyOperator,
			});
			mintedSecret = created.secret;
			newKeyName = '';
			newKeyReadOnly = false;
			newKeyOperator = false;
			askingOperator = false;
			await load();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			minting = false;
		}
	}
</script>

<Screen title={m.settings_keys()}>
	{#if signedIn}
		<p class="mb-3 text-read text-ink-2">{m.settings_keys_said()}</p>
		{#if accessKeys.length === 0}
			<p class="text-body text-ink-2">{m.access_keys_empty()}</p>
		{:else}
			<ul class="grid gap-2">
				{#each accessKeys as key (key.id)}
					<li class="rounded-sm border border-rule bg-card px-3 py-2">
						<div class="flex items-center justify-between gap-3">
							<div>
								<p class="text-body text-ink">
									{key.name}
									{#if key.operator}
										<span class="ml-1 text-label text-ink-2 uppercase"
											>{m.access_key_operator()}</span
										>
									{/if}
									{#if key.read_only}
										<span class="ml-1 text-label text-ink-2 uppercase"
											>{m.access_key_read_only()}</span
										>
									{/if}
									{#if key.add_only}
										<span class="ml-1 text-label text-ink-2 uppercase"
											>{m.access_key_add_only()}</span
										>
									{/if}
								</p>
								<p class="text-read text-ink-2">{whenLastUsed(key.last_used_at)}</p>
							</div>
							<button
								type="button"
								class="quiet-button shrink-0 text-accent"
								onclick={() => endKey(key.id)}
							>
								{m.access_end()}
							</button>
						</div>
						<!--
							A phone's key keeps where what it shares goes, and it
							changes here without reinstalling the Shortcut (#452).
						-->
						{#if key.add_only && hasRunner}
							<label
								class="mt-2 flex items-start gap-2 border-t border-rule pt-2 text-read text-ink"
							>
								<input
									class="mt-1"
									type="checkbox"
									checked={key.everything_to_agent}
									onchange={(event) => void chooseKeyAgent(key.id, event.currentTarget.checked)}
								/>
								{m.access_key_everything_to_agent()}
							</label>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}

		<Section heading={m.access_key_make_heading()}>
			<!--
				A new key takes the form's place: the form sits at the foot, so on a
				phone a key shown above it was off-screen with nothing saying it
				existed (#142, choice 1). The form comes back with Done, and the
				key never does.
			-->
			{#if mintedSecret}
				<OneTimeSecret
					secret={mintedSecret}
					once={m.access_key_secret_once()}
					copy={m.access_key_secret_copy()}
					copied={m.access_key_secret_copied()}
					dismiss={m.access_key_secret_dismiss()}
					ondismiss={() => (mintedSecret = undefined)}
					focus
				/>
			{:else}
				<form
					class="grid gap-3"
					onsubmit={(event) => {
						event.preventDefault();
						if (mayAdminister && newKeyOperator) askingOperator = true;
						else void mint();
					}}
				>
					<label class="grid gap-1 text-body text-ink">
						{m.access_key_name()}
						<input
							class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
							bind:value={newKeyName}
							required
						/>
					</label>
					<label class="flex items-center gap-2 text-body text-ink">
						<input type="checkbox" bind:checked={newKeyReadOnly} />
						{m.access_key_read_only_label()}
					</label>
					{#if mayAdminister}
						<fieldset class="grid gap-2">
							<legend class="mb-1 text-body text-ink">{m.access_key_acts_as()}</legend>
							<label class="flex items-center gap-2 text-body text-ink">
								<input type="radio" name="key-acts-as" value={false} bind:group={newKeyOperator} />
								{m.access_key_as_cook()}
							</label>
							<label class="flex items-center gap-2 text-body text-ink">
								<input type="radio" name="key-acts-as" value={true} bind:group={newKeyOperator} />
								{m.access_key_as_operator()}
							</label>
						</fieldset>
					{/if}
					{#if failed && !askingOperator}
						<p class="text-body text-accent" role="alert"><Refusal words={failed} /></p>
					{/if}
					<button
						class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
						disabled={minting}
					>
						{m.access_key_mint()}
					</button>
				</form>
			{/if}
		</Section>
	{/if}
</Screen>

{#if askingOperator}
	<Confirm
		title={m.access_key_operator_title()}
		consequence={m.access_key_as_operator_warning()}
		act={m.access_key_mint()}
		busy={minting}
		{failed}
		run={() => void mint()}
		cancel={() => {
			askingOperator = false;
			failed = undefined;
		}}
	/>
{/if}
