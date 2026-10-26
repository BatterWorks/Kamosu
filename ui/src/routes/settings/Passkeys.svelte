<!--
	Your passkeys, a section of Settings › Signing in (#370, choice A of 19
	October 2026, moved there by #469): each its own card like a Session's
	row, then a full-width *Add a passkey*. The prototype is
	`.dev/prototypes/370-passkeys/pages/list-a.phone.html` and its states.

	Adding asks for the password first, in a sheet, before the device is asked
	for its face or fingerprint: a Session never expires, so without it
	whoever holds an unlocked phone could add a passkey of their own that
	ending the phone's Session would not reach.

	A passkey works at the address it was made at and no other. Where this
	page is not at that address, or the instance has no https:// name, the
	list still shows what there is, so a passkey can be removed, and one
	sentence says why none can be added or used here.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Refusal from '$lib/Refusal.svelte';
	import { aDate, aMoment } from '$lib/dates';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { thisDeviceName } from '$lib/device-name';
	import { browserHasPasskeys, makePasskey, passkeysWorkHere } from '$lib/passkeys';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import Confirm from '$lib/Confirm.svelte';
	import Section from '$lib/shell/Section.svelte';
	import type { ListPasskeysOutput } from '$lib/api/catalogue';

	type Passkey = ListPasskeysOutput['passkeys'][number];

	const kamosu = useKamosu();
	const uid = $props.id();

	let listed = $state<ListPasskeysOutput | undefined>(undefined);

	async function load() {
		listed = await kamosu.listPasskeys();
	}

	$effect(() => {
		void load();
	});

	/**
	 * Why nothing can be added or used on this page, or nothing when both can,
	 * with the address to open instead where there is one.
	 */
	const cannotHere = $derived.by(
		(): { said: string; open?: string; after?: string } | undefined => {
			if (!listed) return undefined;
			if (!listed.available) {
				return listed.unavailable_because === 'no_public_address' || !listed.public_address
					? {
							said: `${m.passkeys_unavailable_no_public_address()} ${m.passkeys_unavailable_fix_set()}`,
						}
					: {
							said: `${m.passkeys_unavailable({ address: listed.public_address })} ${m.passkeys_unavailable_fix()}`,
						};
			}
			if (!browserHasPasskeys()) return { said: m.passkeys_browser_cannot() };
			if (!passkeysWorkHere(listed.origin) && listed.origin) {
				return {
					said: m.passkeys_wrong_address({ here: window.location.host }),
					open: listed.origin,
					after: m.passkeys_wrong_address_after(),
				};
			}
			return undefined;
		},
	);

	function whenLastUsed(lastUsedAt: string | null): string {
		return lastUsedAt ? m.access_last_used({ when: aMoment(lastUsedAt) }) : m.access_never_used();
	}

	// --- Adding ----------------------------------------------------------

	let adding = $state(false);
	let password = $state('');
	let addBusy = $state(false);
	let addFailed = $state<string | undefined>(undefined);

	function startAdding() {
		adding = true;
		password = '';
		addFailed = undefined;
	}

	async function add() {
		addBusy = true;
		addFailed = undefined;
		try {
			const begun = await kamosu.startAddingPasskey({ password });
			const answer = await makePasskey(begun.options);
			if ('declined' in answer) {
				adding = false;
				return;
			}
			if ('already' in answer) {
				addFailed = m.passkeys_already();
				return;
			}
			await kamosu.finishAddingPasskey({
				ceremony_id: begun.ceremony_id,
				credential: answer.credential,
				name: thisDeviceName(),
			});
			adding = false;
			await load();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			addFailed =
				error.reason === 'wrong_password'
					? m.password_wrong()
					: error.reason === 'start_again'
						? m.passkeys_start_again()
						: error.message;
		} finally {
			addBusy = false;
			password = '';
		}
	}

	// --- Renaming --------------------------------------------------------

	let editing = $state<string | undefined>(undefined);
	let draftName = $state('');
	let renameBusy = $state(false);
	let renameFailed = $state<string | undefined>(undefined);

	function startRenaming(passkey: Passkey) {
		editing = passkey.id;
		draftName = passkey.name;
		renameFailed = undefined;
	}

	async function rename(passkeyId: string) {
		renameBusy = true;
		renameFailed = undefined;
		try {
			await kamosu.renamePasskey({ passkey_id: passkeyId, name: draftName });
			editing = undefined;
			await load();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			renameFailed = error.message;
		} finally {
			renameBusy = false;
		}
	}

	// --- Removing --------------------------------------------------------

	let removing = $state<Passkey | undefined>(undefined);
	let removeBusy = $state(false);
	let removeFailed = $state<string | undefined>(undefined);

	async function remove() {
		if (!removing) return;
		removeBusy = true;
		removeFailed = undefined;
		try {
			await kamosu.removePasskey({ passkey_id: removing.id });
			removing = undefined;
			await load();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			removeFailed = error.message;
		} finally {
			removeBusy = false;
		}
	}
</script>

{#if listed}
	<Section heading={m.passkeys_heading()}>
		{#if listed.available || listed.passkeys.length > 0}
			<p class="mb-3 text-read text-ink-2">{m.passkeys_explained()}</p>
		{/if}
		{#if listed.passkeys.length > 0}
			<ul class="mb-3 grid gap-2">
				{#each listed.passkeys as passkey (passkey.id)}
					<li class="rounded-sm border border-rule bg-card px-3 py-3">
						{#if editing === passkey.id}
							<form
								class="grid gap-2"
								onsubmit={(event) => {
									event.preventDefault();
									void rename(passkey.id);
								}}
							>
								<label class="grid gap-1 text-read text-ink-2">
									{m.sessions_name_label()}
									<input
										class="min-h-12 w-full rounded-sm border border-rule bg-ground px-3 text-body text-ink"
										bind:value={draftName}
										required
										{@attach (node) => node.focus()}
									/>
								</label>
								{#if renameFailed}
									<p class="text-body text-accent" role="alert"><Refusal words={renameFailed} /></p>
								{/if}
								<div class="flex flex-wrap items-center gap-4">
									<button
										class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
										disabled={renameBusy}
									>
										{m.sessions_name_save()}
									</button>
									<button
										type="button"
										class="quiet-button text-accent"
										onclick={() => (editing = undefined)}
									>
										{m.you_name_keep()}
									</button>
								</div>
							</form>
						{:else}
							<div class="flex items-start justify-between gap-3">
								<div class="min-w-0">
									<p class="text-body break-words text-ink">{passkey.name}</p>
									<p class="text-read text-ink-2">
										{m.passkeys_added({ when: aDate(passkey.created_at) })} · {whenLastUsed(
											passkey.last_used_at,
										)}
									</p>
									{#if !passkey.works_here}
										<p class="text-read text-support">
											{m.passkeys_made_for({ address: passkey.made_for })}
										</p>
									{/if}
								</div>
								<div class="flex shrink-0 items-center gap-3">
									<button
										type="button"
										class="quiet-button text-accent"
										onclick={() => {
											removing = passkey;
											removeFailed = undefined;
										}}
									>
										{m.passkeys_remove()}
									</button>
									<button
										type="button"
										class="quiet-button text-accent"
										onclick={() => startRenaming(passkey)}
									>
										{m.sessions_rename()}
									</button>
								</div>
							</div>
						{/if}
					</li>
				{/each}
			</ul>
		{:else if !cannotHere}
			<p class="mb-3 text-body text-ink-2">{m.passkeys_none()}</p>
		{/if}
		{#if cannotHere}
			<p class="mb-6 border-l-2 border-support pl-3 text-read text-ink-2">
				{cannotHere.said}
				{#if cannotHere.open}
					<a
						href="{cannotHere.open}/settings/signing-in"
						class="text-accent underline underline-offset-4">{cannotHere.open}</a
					>
					{cannotHere.after}
				{/if}
			</p>
		{:else}
			<button
				type="button"
				class="mb-6 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium text-accent"
				onclick={startAdding}
			>
				{m.passkeys_add()}
			</button>
		{/if}
	</Section>
{/if}

{#if adding}
	<SheetFrame
		labelledby="{uid}-add"
		tall={78}
		class="overflow-y-auto border-t border-rule bg-ground px-gutter pt-6"
		onclose={() => {
			if (!addBusy) adding = false;
		}}
	>
		<h2 id="{uid}-add" class="font-display text-list-title font-semibold text-ink">
			{m.passkeys_add_title()}
		</h2>
		<p class="mt-3 text-body text-ink-2">{m.passkeys_add_why()}</p>
		<form
			onsubmit={(event) => {
				event.preventDefault();
				void add();
			}}
		>
			<label class="mt-4 grid gap-1 text-body text-ink">
				{m.account_password()}
				<input
					class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
					type="password"
					autocomplete="current-password"
					bind:value={password}
					required
					data-sheet-focus
				/>
			</label>
			{#if addFailed}
				<p class="mt-2 text-body text-support" role="alert"><Refusal words={addFailed} /></p>
			{/if}
			<div class="mt-6 mb-4 flex flex-col gap-2">
				<button
					class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-60"
					disabled={addBusy}
				>
					{m.passkeys_add_continue()}
				</button>
				<button
					type="button"
					class="min-h-12 rounded-sm px-4 text-body font-medium text-accent"
					disabled={addBusy}
					onclick={() => (adding = false)}
				>
					{m.passkeys_add_not_now()}
				</button>
			</div>
		</form>
		<p class="mb-4 text-read text-ink-2">{m.passkeys_add_then({ name: thisDeviceName() })}</p>
	</SheetFrame>
{/if}

{#if removing}
	<Confirm
		title={m.passkeys_remove_title({ name: removing.name })}
		consequence={m.passkeys_remove_said()}
		act={m.passkeys_remove_do()}
		cancelLabel={m.you_name_keep()}
		busy={removeBusy}
		failed={removeFailed}
		run={remove}
		cancel={() => (removing = undefined)}
	/>
{/if}
