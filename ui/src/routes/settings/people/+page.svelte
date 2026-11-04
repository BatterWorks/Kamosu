<!--
	People (#103, #469): a page of This instance. Inviting comes first, above
	the list, since it is the one thing somebody arrives here wanting to do.

	**An Invite can name one of the Operator's own Kitchens** (#469), so a
	newcomer gets an account and that Kitchen from the one link, or none.
	Who may make an account does not change: an Operator alone.

	Each Person's acts are their own row's business, and one row is open at a
	time: four buttons under every Person at once made this list a wall of
	buttons with the names lost among them.

	**The Invites and recovery links not yet used are listed between the two**
	(#535), each with Revoke. A link sent to the wrong place, or forgotten,
	works until it is used or revoked (ADR 0031), so the Operator has to see
	it to end it. A row says what the link does and when it was made, never
	the link itself, which Kamosu keeps by its hash alone.
-->
<script lang="ts">
	import Refusal from '#lib/Refusal.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import Section from '#lib/shell/Section.svelte';
	import Confirm from '#lib/Confirm.svelte';
	import { Acting } from '#lib/operator/acting.svelte.js';
	import { kitchenName } from '#lib/cookbook.js';
	import { howLongAgo } from '#lib/dates.js';
	import type {
		ListAccountLinksOutput,
		ListAccountsOutput,
		ListKitchensOutput,
	} from '#lib/api/catalogue.js';
	import OperatorOnly from '../OperatorOnly.svelte';

	const kamosu = useKamosu();
	const acting = new Acting();

	type Account = ListAccountsOutput['accounts'][number];
	type Kitchen = ListKitchensOutput['kitchens'][number];
	type Link = ListAccountLinksOutput['links'][number];

	let accounts = $state<Account[]>([]);
	/** Why the accounts could not be listed, kept apart from what an act says. */
	let unlisted = $state<string | undefined>(undefined);
	let kitchens = $state<Kitchen[]>([]);
	let links = $state<Link[]>([]);
	/** Why the links could not be listed: never shown as none waiting. */
	let linksUnlisted = $state<string | undefined>(undefined);

	/**
	 * Asked only once OperatorOnly has agreed this Person is an Operator
	 * (#533). A Kamosu that cannot list them says why, rather than showing an
	 * Operator an empty list as though nobody held an account.
	 */
	async function loadAccounts() {
		try {
			accounts = (await kamosu.listAccounts({})).accounts;
			unlisted = undefined;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			unlisted = error.message;
		}
	}

	/** The Invites and recovery links still live (#535). */
	async function loadLinks() {
		try {
			links = (await kamosu.listAccountLinks({})).links;
			linksUnlisted = undefined;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			linksUnlisted = error.message;
		}
	}

	async function load() {
		await Promise.all([loadAccounts(), loadLinks()]);
		try {
			kitchens = (await kamosu.listKitchens()).kitchens;
			// The Operator's own Kitchen is where a housemate usually goes, as
			// the prototype chose it; None is one choice away.
			inviteKitchen = kitchens[0]?.id ?? '';
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
		}
	}

	// ── Inviting ──────────────────────────────────────────────────────────────

	let inviteAsOperator = $state(false);
	/** The Kitchen the newcomer arrives in: one of the Operator's own, or none. */
	let inviteKitchen = $state('');
	let invite = $state<{ link: string; kitchen: string | undefined } | undefined>(undefined);
	let recovery = $state<{ who: string; link: string } | undefined>(undefined);

	/** Whose acts are open. One at a time. */
	let opened = $state<string | undefined>(undefined);

	/** Which link the clipboard last took, so only that one says so. */
	let copied = $state<string | undefined>(undefined);

	/**
	 * A minted link as something you can send somebody. The Core mints a path,
	 * since it knows no address, and a path pasted into a message is not a
	 * link. The origin this page is read from is the one address the instance
	 * is known to answer on, because the Operator reached it there a moment
	 * ago. Deliberately not the public address: that one is for a Share Link
	 * going to a stranger, and it can be set, wrong, or absent while the
	 * instance is read perfectly well on the network it lives on.
	 */
	const reachable = (path: string) =>
		typeof window === 'undefined' ? path : new URL(path, window.location.origin).toString();

	async function copy(link: string) {
		await navigator.clipboard.writeText(link);
		copied = link;
	}

	const mintInvite = () =>
		acting.act(async () => {
			const kitchen = kitchens.find((each) => each.id === inviteKitchen);
			const minted = await kamosu.mintInvite({
				is_operator: inviteAsOperator,
				...(kitchen ? { kitchen_id: kitchen.id } : {}),
			});
			invite = { link: minted.link, kitchen: kitchen ? kitchenName(kitchen) : undefined };
			inviteAsOperator = false;
			copied = undefined;
		}, loadLinks);

	const mintRecovery = (who: string) =>
		acting.act(async () => {
			recovery = { who, link: (await kamosu.mintRecoveryLink({ name: who })).link };
			copied = undefined;
		}, loadLinks);

	const setOperator = (who: string, is_operator: boolean) =>
		acting.act(async () => {
			await kamosu.setOperator({ name: who, is_operator });
		}, loadAccounts);

	// ── Ending an account ─────────────────────────────────────────────────────

	let asking = $state<
		{ kind: 'delete' | 'disable'; who: string } | { kind: 'revoke'; link: Link } | undefined
	>(undefined);

	function close() {
		asking = undefined;
		acting.said = undefined;
	}

	const endAccount = (who: string, deleted: boolean) =>
		acting.act(
			async () => {
				if (deleted) await kamosu.deleteAccount({ name: who });
				else await kamosu.disableAccount({ name: who });
			},
			async () => {
				// A disabled or deleted Person's recovery link is no longer live.
				await Promise.all([loadAccounts(), loadLinks()]);
				close();
			},
		);

	/** A live link in words: what it does, when it was made, and the sheet's question. */
	function said(link: Link) {
		const when = howLongAgo(link.created_at);
		if (link.kind === 'recovery') {
			const who = link.person ?? '';
			return {
				what: m.operator_links_recovery({ who }),
				made: m.operator_links_recovery_made({ when }),
				ask: m.operator_links_revoke_recovery_title({ who }),
			};
		}
		return {
			what: link.operator ? m.operator_links_operator_invite() : m.operator_links_invite(),
			made: m.operator_links_invite_made({ when }),
			ask: m.operator_links_revoke_invite_title(),
		};
	}

	const revoke = (link: Link) =>
		acting.act(
			async () => {
				try {
					await kamosu.revokeAccountLink({ id: link.id });
				} finally {
					// Refused too, the list is asked again: a link somebody used
					// or revoked meanwhile is refused, and must leave the list.
					await loadLinks();
				}
			},
			// Closed first: the link is revoked even if the list then fails to
			// come back, and the sheet must not offer to revoke it twice.
			async () => close(),
		);
</script>

<!--
	A Secret Kamosu will never print again: an Invite, or a recovery link,
	shown whole with the address it is read from in front of it, so what is on
	the screen is what you send somebody. Copy is here because selecting forty
	hex characters by dragging on a phone is the failure it prevents.
-->
{#snippet shownOnce(said: string, link: string, dismiss: () => void, after?: string)}
	{@const whole = reachable(link)}
	<div class="mt-3 rounded-sm border border-accent bg-card p-3" role="alert">
		<p class="text-body font-semibold text-ink">{said}</p>
		<code class="mt-2 block rounded-sm bg-ground p-2 text-read break-all">{whole}</code>
		{#if after}
			<p class="mt-2 text-read text-ink-2">{after}</p>
		{/if}
		<div class="mt-3 flex gap-2">
			<button
				type="button"
				onclick={() => copy(whole)}
				class="min-h-12 flex-1 rounded-sm border border-accent bg-accent px-3 text-body font-medium
				text-on-accent"
			>
				{copied === whole ? m.operator_copied() : m.operator_copy()}
			</button>
			<button
				type="button"
				onclick={dismiss}
				class="min-h-12 flex-1 rounded-sm border border-accent px-3 text-body font-medium text-accent"
			>
				{m.operator_invite_done()}
			</button>
		</div>
	</div>
{/snippet}

<OperatorOnly title={m.settings_people()} onallowed={load}>
	{#if unlisted}
		<p class="mt-4 text-body text-support" role="alert"><Refusal words={unlisted} /></p>
	{/if}
	{#if acting.said && !asking}
		<p class="mt-4 text-body text-support" role="alert"><Refusal words={acting.said} /></p>
	{/if}

	<Section heading={m.operator_invite_heading()}>
		{#if invite}
			{@const minted = invite}
			{@render shownOnce(
				m.operator_invite_once(),
				minted.link,
				() => (invite = undefined),
				minted.kitchen ? m.operator_invite_into({ kitchen: minted.kitchen }) : undefined,
			)}
		{:else}
			<div class="grid gap-3">
				{#if kitchens.length > 0}
					<label class="grid gap-1 text-body text-ink">
						{m.operator_invite_kitchen()}
						<select
							bind:value={inviteKitchen}
							class="min-h-12 w-full rounded-sm border border-rule bg-card px-2 text-body"
						>
							<option value="">{m.operator_invite_no_kitchen()}</option>
							{#each kitchens as kitchen (kitchen.id)}
								<option value={kitchen.id}>{kitchenName(kitchen)}</option>
							{/each}
						</select>
					</label>
				{/if}
				<label class="flex min-h-12 items-center gap-3">
					<input type="checkbox" bind:checked={inviteAsOperator} class="h-6 w-6" />
					<span class="text-body text-ink">{m.operator_invite_as_operator()}</span>
				</label>
				<button
					type="button"
					disabled={acting.working}
					onclick={mintInvite}
					class="min-h-12 w-full rounded-sm bg-accent px-4 font-semibold text-on-accent
					disabled:opacity-60"
				>
					{m.operator_invite()}
				</button>
			</div>
		{/if}
	</Section>

	<Section heading={m.operator_links_heading()}>
		<p class="mb-3 text-read text-ink-2">{m.operator_links_said()}</p>
		{#if linksUnlisted}
			<p class="mb-3 text-body text-support" role="alert"><Refusal words={linksUnlisted} /></p>
		{/if}
		<ul class="overflow-hidden rounded-sm border border-rule bg-card">
			{#each links as link (link.id)}
				<li class="flex items-center gap-3 border-b border-rule p-3 last:border-b-0">
					<span class="min-w-0 flex-1">
						<span class="block text-body break-words text-ink">{said(link).what}</span>
						<span class="block text-read text-ink-2">{said(link).made}</span>
					</span>
					<button
						type="button"
						disabled={acting.working}
						onclick={() => (asking = { kind: 'revoke', link })}
						class="min-h-12 shrink-0 rounded-sm border border-support px-3 text-read
						font-medium text-support disabled:opacity-60"
					>
						{m.operator_links_revoke()}
					</button>
				</li>
			{:else}
				{#if !linksUnlisted}
					<li class="p-3 text-body text-ink-2">{m.operator_links_empty()}</li>
				{/if}
			{/each}
		</ul>
	</Section>

	<Section heading={m.operator_everyone_heading()}>
		<ul class="overflow-hidden rounded-sm border border-rule bg-card">
			{#each accounts as account (account.name)}
				{@const open = opened === account.name}
				<li class="border-b border-rule last:border-b-0">
					<button
						type="button"
						aria-expanded={open}
						onclick={() => (opened = open ? undefined : account.name)}
						class="flex w-full items-center gap-3 p-3 text-left"
					>
						<span class="min-w-0 flex-1">
							<span class="block truncate text-body font-medium text-ink">{account.name}</span>
							<span class="block text-read text-ink-2">
								{account.is_operator ? m.operator_role_operator() : m.operator_role_person()}
								{#if account.disabled}· {m.operator_role_disabled()}{/if}
							</span>
						</span>
						{#if account.is_you}
							<span
								class="shrink-0 rounded-sm bg-accent px-2 py-1 text-label text-on-accent uppercase"
								>{m.operator_you()}</span
							>
						{/if}
						<svg
							viewBox="0 0 24 24"
							aria-hidden="true"
							class="h-4 w-4 shrink-0 text-ink-2 {open ? 'rotate-90' : ''}"
							fill="none"
							stroke="currentColor"
							stroke-width="1.75"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="m9 5 7 7-7 7" />
						</svg>
					</button>

					{#if open}
						<div class="px-3 pb-3">
							<!-- Two by two, so four acts of different weight are read as
							     four. Delete takes the width when there is no Disable
							     beside it, a disabled Person having been disabled. -->
							<div class="grid grid-cols-2 gap-2">
								<button
									type="button"
									disabled={acting.working}
									onclick={() => setOperator(account.name, !account.is_operator)}
									class="min-h-12 rounded-sm border border-accent px-3 text-read font-medium
									text-accent disabled:opacity-60"
								>
									{account.is_operator ? m.operator_stand_down() : m.operator_make_operator()}
								</button>
								<button
									type="button"
									disabled={acting.working}
									onclick={() => mintRecovery(account.name)}
									class="min-h-12 rounded-sm border border-rule px-3 text-read font-medium
									text-ink disabled:opacity-60"
								>
									{m.operator_recovery()}
								</button>
								{#if !account.disabled}
									<button
										type="button"
										disabled={acting.working}
										onclick={() => (asking = { kind: 'disable', who: account.name })}
										class="min-h-12 rounded-sm border border-rule px-3 text-read font-medium
										text-ink disabled:opacity-60"
									>
										{m.operator_disable()}
									</button>
								{/if}
								<button
									type="button"
									disabled={acting.working}
									onclick={() => (asking = { kind: 'delete', who: account.name })}
									class="min-h-12 rounded-sm border border-support px-3 text-read font-medium
									text-support disabled:opacity-60 {account.disabled ? 'col-span-2' : ''}"
								>
									{m.operator_delete()}
								</button>
							</div>

							<!-- A recovery link appears under the Person it belongs to:
							     it is shown once, so it has to be where the eye is. -->
							{#if recovery && recovery.who === account.name}
								{@render shownOnce(
									m.operator_recovery_once({ who: recovery.who }),
									recovery.link,
									() => (recovery = undefined),
								)}
							{/if}
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	</Section>
</OperatorOnly>

{#if asking?.kind === 'revoke'}
	{@const link = asking.link}
	<Confirm
		title={said(link).ask}
		consequence={m.operator_links_revoke_consequence()}
		act={m.operator_links_revoke()}
		busy={acting.working}
		failed={acting.said}
		run={() => revoke(link)}
		cancel={close}
	/>
{:else if asking?.kind === 'delete'}
	{@const who = asking.who}
	<Confirm
		title={m.operator_account_delete_title({ who })}
		consequence={m.operator_account_delete_consequence({ who })}
		act={m.operator_account_delete_do()}
		instead={{ label: m.operator_disable(), run: () => endAccount(who, false) }}
		busy={acting.working}
		failed={acting.said}
		run={() => endAccount(who, true)}
		cancel={close}
	/>
{:else if asking}
	{@const who = asking.who}
	<Confirm
		title={m.operator_account_disable_title({ who })}
		consequence={m.operator_account_disable_consequence({ who })}
		act={m.operator_disable()}
		busy={acting.working}
		failed={acting.said}
		run={() => endAccount(who, false)}
		cancel={close}
	/>
{/if}
