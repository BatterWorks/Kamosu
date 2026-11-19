<!--
	Kitchens (#469): who sees and cooks your recipes (ADR 0041). Your
	Cookbook, who changes them, is a page of its own beside this, so the two
	invites are never side by side to be confused.

	**A Kitchen's card leads with inviting** (K1, chosen on #648): who is in
	it, then *Invite someone to <Kitchen>*, the card's one main button. The
	rare acts, renaming, your own name for it, removing someone and leaving,
	are folded under one *Change this Kitchen* row, apart from inviting and
	each saying what it does (the audit's I3, I4, I5). Starting another
	Kitchen is a button at the page's foot that opens a panel (I6).

	**An invite is a link** (#469), as a Cookbook's and the instance's are: it
	opens a page showing the Kitchen as its members see it, with a way to
	join. The link is shown once, since the Core keeps only its hash, at the
	instance's public address when one is set (#648). Every invite nobody has
	opened yet is listed under the button, and can be cancelled from there
	(#648).
-->
<script lang="ts">
	import Refusal from '#lib/Refusal.svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import Screen from '#lib/shell/Screen.svelte';
	import Confirm from '#lib/Confirm.svelte';
	import SheetFrame from '#lib/SheetFrame.svelte';
	import InviteLink from '#lib/InviteLink.svelte';
	import WaitingInvites from '#lib/WaitingInvites.svelte';
	import { joinedNames, kitchenName } from '#lib/cookbook.js';
	import type { GetPersonOutput, ListKitchensOutput } from '#lib/api/catalogue.js';

	const kamosu = useKamosu();

	type Kitchen = ListKitchensOutput['kitchens'][number];

	let signedIn = $state(false);
	let me = $state<GetPersonOutput | undefined>(undefined);
	let kitchens = $state<Kitchen[]>([]);
	let failed = $state<string | undefined>(undefined);
	let newKitchenName = $state('');
	/**
	 * The link just made, for one Kitchen. Held only on this page: the Core
	 * keeps a hash, so once the page is left the link cannot be shown again.
	 */
	let link = $state<{ kitchenId: string; inviteId: string; url: string } | undefined>(undefined);
	/** Which Kitchen's rare acts are unfolded. One at a time. */
	let changing = $state<string | undefined>(undefined);
	/** The panel that starts another Kitchen. */
	let starting = $state(false);
	/** An invite from the list, asked about before it is cancelled. */
	let cancelling = $state<string | undefined>(undefined);
	/** A member asked about before they are removed. */
	let removing = $state<{ kitchen: Kitchen; personId: string; name: string } | undefined>(
		undefined,
	);
	/** Where the open sheet's act has got to: running, or refused in its own words. */
	let sheet = $state<{ busy: boolean; failed: string | undefined }>({
		busy: false,
		failed: undefined,
	});

	async function load() {
		try {
			const [person, answer] = await Promise.all([kamosu.getPerson(), kamosu.listKitchens()]);
			me = person;
			kitchens = answer.kitchens;
			signedIn = true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			signedIn = false;
		}
	}

	$effect(() => {
		load();
	});

	async function withError(action: () => Promise<void>) {
		failed = undefined;
		try {
			await action();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		}
	}

	const rename = (kitchenId: string, name: string) =>
		withError(async () => {
			await kamosu.renameKitchen({ kitchen_id: kitchenId, name });
			await load();
		});

	const setNickname = (kitchenId: string, nickname: string) =>
		withError(async () => {
			await kamosu.setKitchenNickname({ kitchen_id: kitchenId, nickname: nickname || null });
			await load();
		});

	/** Run the act the open sheet asked about, saying a refusal in the sheet. */
	async function answer(act: () => Promise<void>) {
		sheet = { busy: true, failed: undefined };
		try {
			await act();
			sheet = { busy: false, failed: undefined };
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			sheet = { busy: false, failed: error.message };
		}
	}

	const removeMember = () =>
		answer(async () => {
			const who = removing;
			if (!who) return;
			await kamosu.removeKitchenMember({ kitchen_id: who.kitchen.id, person_id: who.personId });
			removing = undefined;
			await load();
		});

	/** Which Kitchen's invite is being made, so a second tap makes no second one. */
	let inviting = $state<string | undefined>(undefined);

	const invite = (kitchenId: string) =>
		withError(async () => {
			if (inviting) return;
			inviting = kitchenId;
			const minted = await kamosu
				.inviteToKitchen({ kitchen_id: kitchenId })
				.finally(() => (inviting = undefined));
			link = {
				kitchenId,
				inviteId: minted.invite_id,
				// The instance's public address, where one is set (#648); the
				// address this page is read at only where none is.
				url: minted.url ?? `${location.origin}/kitchen-invite/${minted.secret}`,
			};
			await load();
		});

	/** Cancel the invite just made, from its card, which asks first itself. */
	async function cancelMade(inviteId: string) {
		await kamosu.cancelKitchenInvite({ invite_id: inviteId });
		// The list read again first, so the invite just ended never shows in it.
		await load();
		link = undefined;
	}

	const cancelListed = () =>
		answer(async () => {
			const inviteId = cancelling;
			if (!inviteId) return;
			await kamosu.cancelKitchenInvite({ invite_id: inviteId });
			cancelling = undefined;
			await load();
		});

	const create = () =>
		answer(async () => {
			await kamosu.createKitchen({ name: newKitchenName });
			newKitchenName = '';
			starting = false;
			await load();
		});

	/** The other members' names, for the line on whose recipes the Kitchen sees. */
	const otherNames = (kitchen: Kitchen) =>
		kitchen.members
			.filter((member) => member.person_id !== me?.person_id)
			.map((member) => member.name);

	/**
	 * Leaving a Kitchen, asked first with what each side keeps (#131, screen
	 * choice 4). The counts come from the Core's own dry run of the leave, so
	 * the sheet says what the act will do rather than a guess at it.
	 */
	let leaving = $state<{ kitchen: Kitchen; theyKeep: number; youKeep: number } | undefined>(
		undefined,
	);
	let leaveBusy = $state(false);
	let leaveFailed = $state<string | undefined>(undefined);

	const askToLeave = (kitchen: Kitchen) =>
		withError(async () => {
			const preview = await kamosu.previewLeavingKitchen({ kitchen_id: kitchen.id });
			leaveFailed = undefined;
			leaving = { kitchen, theyKeep: preview.they_keep, youKeep: preview.you_keep };
		});

	async function leave() {
		const asked = leaving;
		if (!asked || !me) return;
		leaveBusy = true;
		leaveFailed = undefined;
		try {
			await kamosu.removeKitchenMember({ kitchen_id: asked.kitchen.id, person_id: me.person_id });
			leaving = undefined;
			await load();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			leaveFailed = error.message;
		} finally {
			leaveBusy = false;
		}
	}
</script>

<Screen title={m.settings_kitchens_page()} blurb={m.settings_kitchens_said()}>
	{#if signedIn}
		{#if failed}
			<p class="mb-4 text-body text-accent" role="alert"><Refusal words={failed} /></p>
		{/if}

		<ul class="grid gap-4">
			{#each kitchens as kitchen (kitchen.id)}
				{@const called = kitchenName(kitchen)}
				{@const theirs = otherNames(kitchen)}
				{@const open = changing === kitchen.id}
				{@const waiting = kitchen.invites.filter((each) => each.invite_id !== link?.inviteId)}
				<!-- `min-w-0` on the card and on each text box is what keeps this
				     card inside a narrow phone (#102). Both a grid item and a flex
				     item take `min-width: auto`, so neither will shrink below its
				     content, and a text box's content is about twenty characters
				     wide whatever the screen. -->
				<li class="min-w-0 border border-rule bg-card p-4">
					<h2 class="font-display text-list-title font-semibold break-words text-ink">{called}</h2>
					{#if theirs.length === 0}
						<p class="mt-2 text-body text-ink">{m.kitchen_only_you()}</p>
					{:else}
						<ul class="mt-2">
							{#each kitchen.members as member (member.person_id)}
								<li class="flex min-h-12 items-center border-b border-rule text-body text-ink">
									{member.name}
									{#if member.person_id === me?.person_id}
										<span class="ml-1 text-label text-ink-2 uppercase"
											>{m.kitchen_member_you()}</span
										>
									{/if}
								</li>
							{/each}
						</ul>
						<p class="mt-3 text-read text-ink-2">
							{m.kitchen_everyone_sees({ others: theirs.join(', ') })}
						</p>
					{/if}

					{#if link?.kitchenId === kitchen.id}
						{@const made = link}
						<InviteLink
							title={m.kitchen_invite_link_for({ kitchen: called })}
							url={made.url}
							cancel={() => cancelMade(made.inviteId)}
						/>
					{:else}
						<button
							type="button"
							class="mt-4 main-button w-full"
							disabled={inviting === kitchen.id}
							aria-busy={inviting === kitchen.id ? 'true' : undefined}
							onclick={() => invite(kitchen.id)}
						>
							{inviting === kitchen.id
								? m.share_link_making()
								: m.kitchen_invite_to({ kitchen: called })}
						</button>
					{/if}

					<!-- Every invite nobody has opened yet, the one just made aside
					     since its card is above (#648). -->
					<WaitingInvites
						invites={waiting}
						ask={(inviteId) => {
							sheet = { busy: false, failed: undefined };
							cancelling = inviteId;
						}}
					/>

					<!-- The rule above the row is its own, so the row is a plain button. -->
					<div class="mt-4 border-t border-rule">
						<button
							type="button"
							class="grid min-h-[56px] w-full grid-cols-[1fr_auto] items-center py-2 text-left text-accent"
							aria-expanded={open}
							onclick={() => (changing = open ? undefined : kitchen.id)}
						>
							<span class="text-body font-medium">{m.kitchen_change()}</span>
							<svg
								viewBox="0 0 24 24"
								aria-hidden="true"
								class="row-span-2 size-[20px] text-ink-2 {open ? 'rotate-90' : ''}"
								fill="none"
								stroke="currentColor"
								stroke-width="1.75"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<path d="m9 5 7 7-7 7" />
							</svg>
							<span class="text-read text-ink-2">
								{theirs.length === 0 ? m.kitchen_change_said_alone() : m.kitchen_change_said()}
							</span>
						</button>
					</div>

					{#if open}
						<div class="mt-4 border-t border-rule pt-4">
							<form
								onsubmit={(event) => {
									event.preventDefault();
									const input = event.currentTarget.elements.namedItem('name') as HTMLInputElement;
									rename(kitchen.id, input.value);
								}}
							>
								<label
									class="block text-body font-semibold text-ink"
									for={`kitchen-name-${kitchen.id}`}
								>
									{m.kitchen_name_for_all()}
								</label>
								<div class="mt-2 flex gap-2">
									<input
										id={`kitchen-name-${kitchen.id}`}
										name="name"
										class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-3 text-body text-ink"
										value={kitchen.name}
									/>
									<button class="second-button shrink-0" type="submit">{m.kitchen_rename()}</button>
								</div>
							</form>

							<form
								class="mt-6"
								onsubmit={(event) => {
									event.preventDefault();
									const input = event.currentTarget.elements.namedItem(
										'nickname',
									) as HTMLInputElement;
									setNickname(kitchen.id, input.value);
								}}
							>
								<label
									class="block text-body font-semibold text-ink"
									for={`nickname-${kitchen.id}`}
								>
									{m.kitchen_own_name()}
								</label>
								<p class="text-read text-ink-2">{m.kitchen_own_name_said()}</p>
								<div class="mt-2 flex gap-2">
									<input
										id={`nickname-${kitchen.id}`}
										name="nickname"
										class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-3 text-body text-ink"
										placeholder={m.kitchen_nickname_placeholder()}
										value={kitchen.nickname ?? ''}
									/>
									<button class="second-button shrink-0" type="submit">{m.kitchen_save()}</button>
								</div>
							</form>

							{#if theirs.length > 0}
								<p class="mt-6 text-body font-semibold text-ink">{m.kitchen_people_in_it()}</p>
								<ul class="mt-2">
									{#each kitchen.members as member (member.person_id)}
										<li
											class="flex min-h-12 items-center justify-between gap-3 border-b border-rule text-body text-ink"
										>
											<span class="min-w-0 break-words">
												{member.name}
												{#if member.person_id === me?.person_id}
													<span class="ml-1 text-label text-ink-2 uppercase"
														>{m.kitchen_member_you()}</span
													>
												{/if}
											</span>
											{#if member.person_id !== me?.person_id}
												<button
													type="button"
													class="quiet-button shrink-0 text-support"
													onclick={() => {
														sheet = { busy: false, failed: undefined };
														removing = { kitchen, personId: member.person_id, name: member.name };
													}}
												>
													{m.kitchen_remove_ask({ name: member.name })}
												</button>
											{/if}
										</li>
									{/each}
								</ul>
							{/if}

							<!-- Leaving is asked first with what each side keeps (#131,
							     screen choice 4). -->
							<button
								type="button"
								class="mt-6 second-button w-full loses"
								onclick={() => askToLeave(kitchen)}
							>
								{m.kitchen_leave_ask({ kitchen: called })}
							</button>
						</div>
					{/if}
				</li>
			{/each}
		</ul>

		<button
			type="button"
			class="mt-8 second-button w-full"
			onclick={() => {
				sheet = { busy: false, failed: undefined };
				starting = true;
			}}
		>
			{m.kitchen_start_another()}
		</button>
	{/if}
</Screen>

{#if starting}
	<SheetFrame
		labelledby="start-title"
		class="overflow-y-auto border-t border-rule bg-ground px-gutter pt-6"
		onclose={() => {
			if (!sheet.busy) starting = false;
		}}
	>
		<form
			onsubmit={(event) => {
				event.preventDefault();
				create();
			}}
		>
			<h2 id="start-title" class="font-display text-list-title font-semibold text-ink">
				{m.kitchen_start_another()}
			</h2>
			<p class="mt-3 text-body text-ink">{m.kitchen_start_said()}</p>
			<label class="mt-4 block text-body font-semibold text-ink" for="new-kitchen-name">
				{m.kitchen_start_name()}
			</label>
			<input
				id="new-kitchen-name"
				class="mt-2 min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body"
				bind:value={newKitchenName}
				required
			/>
			{#if sheet.failed}
				<p class="mt-3 text-body text-support" role="alert"><Refusal words={sheet.failed} /></p>
			{/if}
			<div class="mt-6 mb-4 flex flex-col gap-2">
				<button
					type="submit"
					class="main-button w-full"
					disabled={sheet.busy}
					aria-busy={sheet.busy ? 'true' : undefined}
				>
					{m.kitchen_start_do()}
				</button>
				<button
					type="button"
					class="second-button w-full"
					disabled={sheet.busy}
					onclick={() => (starting = false)}
				>
					{m.operator_cancel()}
				</button>
			</div>
		</form>
	</SheetFrame>
{/if}

{#if cancelling}
	<Confirm
		title={m.invite_cancel_title()}
		consequence={m.invite_cancel_said()}
		act={m.invite_cancel_do()}
		cancelLabel={m.invite_cancel_keep()}
		busy={sheet.busy}
		failed={sheet.failed}
		run={cancelListed}
		cancel={() => (cancelling = undefined)}
	/>
{/if}

{#if removing}
	{@const who = removing}
	<Confirm
		title={m.kitchen_remove_title({ name: who.name, kitchen: kitchenName(who.kitchen) })}
		consequence={m.kitchen_remove_said({ name: who.name })}
		act={m.kitchen_remove_do({ name: who.name })}
		cancelLabel={m.kitchen_remove_keep({ name: who.name })}
		busy={sheet.busy}
		failed={sheet.failed}
		run={removeMember}
		cancel={() => (removing = undefined)}
	/>
{/if}

{#if leaving}
	{@const asked = leaving}
	{@const called = kitchenName(asked.kitchen)}
	{@const others = asked.kitchen.members.filter((member) => member.person_id !== me?.person_id)}
	<Confirm
		title={m.kitchen_leave_title({ kitchen: called })}
		consequence={m.kitchen_leave_said({ kitchen: called })}
		act={m.kitchen_leave_do({ kitchen: called })}
		cancelLabel={m.cookbook_stay()}
		busy={leaveBusy}
		failed={leaveFailed}
		run={leave}
		cancel={() => (leaving = undefined)}
	>
		<!-- What each side keeps, counted by the Core's own dry run: a copy
		     of what they cooked of the other's, and nothing more. -->
		<ul>
			{#if others.length > 0}
				<li class="flex justify-between gap-3 border-b border-rule py-2 text-body text-ink">
					<span>
						{others.length === 1
							? m.kitchen_leave_they_keep_one({ name: others[0]!.name })
							: m.kitchen_leave_they_keep({ names: joinedNames(others.map((o) => o.name)) })}
					</span>
					<span class="text-right text-read text-ink-2"
						>{asked.theyKeep === 0
							? m.kitchen_leave_they_keep_what_none()
							: asked.theyKeep === 1
								? m.kitchen_leave_they_keep_what_one()
								: m.kitchen_leave_they_keep_what({ count: asked.theyKeep })}</span
					>
				</li>
			{/if}
			<li class="flex justify-between gap-3 border-b border-rule py-2 text-body text-ink">
				<span>{m.kitchen_leave_you_keep()}</span>
				<span class="text-right text-read text-ink-2"
					>{asked.youKeep === 0
						? m.kitchen_leave_you_keep_what_none()
						: asked.youKeep === 1
							? m.kitchen_leave_you_keep_what_one()
							: m.kitchen_leave_you_keep_what({ count: asked.youKeep })}</span
				>
			</li>
		</ul>
		<p class="mt-3 text-read text-ink-2">{m.kitchen_leave_nothing_else()}</p>
	</Confirm>
{/if}
