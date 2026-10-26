<!--
	Kitchens (#469): who sees and cooks your recipes (ADR 0041). Your
	Cookbook, who changes them, is a page of its own beside this, so the two
	invites are never side by side to be confused.

	**An invite is a link** (#469), as a Cookbook's and the instance's are: it
	opens a page showing the Kitchen as its members see it, with a way to
	join. The link is shown once, since the Core keeps only its hash, and it
	can be cancelled until somebody opens it. It replaced a code the other
	person pasted into a field here.
-->
<script lang="ts">
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import Confirm from '$lib/Confirm.svelte';
	import { copyText } from '$lib/clipboard';
	import { cookbookCalled, joinedNames, kitchenName } from '$lib/cookbook';
	import type { GetPersonOutput, ListKitchensOutput } from '$lib/api/catalogue';

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
	let copied = $state(false);

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

	const removeMember = (kitchenId: string, personId: string) =>
		withError(async () => {
			await kamosu.removeKitchenMember({ kitchen_id: kitchenId, person_id: personId });
			await load();
		});

	const invite = (kitchenId: string) =>
		withError(async () => {
			const minted = await kamosu.inviteToKitchen({ kitchen_id: kitchenId });
			copied = false;
			link = {
				kitchenId,
				inviteId: minted.invite_id,
				url: `${location.origin}/kitchen-invite/${minted.secret}`,
			};
		});

	const cancelInvite = (inviteId: string) =>
		withError(async () => {
			await kamosu.cancelKitchenInvite({ invite_id: inviteId });
			link = undefined;
		});

	async function copyLink() {
		if (!link) return;
		// A browser that will not copy still shows the link to select by hand.
		copied = await copyText(link.url);
	}

	const create = () =>
		withError(async () => {
			await kamosu.createKitchen({ name: newKitchenName });
			newKitchenName = '';
			await load();
		});

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

<Screen title={m.settings_kitchens_page()}>
	{#if signedIn}
		<p class="mb-3 text-read text-ink-2">{m.settings_kitchens_said()}</p>
		{#if failed}
			<p class="mb-4 text-body text-accent" role="alert"><Refusal words={failed} /></p>
		{/if}

		<ul class="grid gap-4">
			{#each kitchens as kitchen (kitchen.id)}
				<li class="min-w-0 rounded-sm border border-rule bg-card p-3">
					<!-- `min-w-0` in three places is what keeps this card inside a
					     narrow phone (#102). Both a grid item and a flex item take
					     `min-width: auto`, so neither will shrink below its content,
					     and a text box's content is about twenty characters wide
					     whatever the screen. It has to be on the <li> as well as on
					     the two boxes, or the grid track keeps the card's minimum. -->
					<form
						class="flex items-center gap-2"
						onsubmit={(event) => {
							event.preventDefault();
							const input = event.currentTarget.elements.namedItem('name') as HTMLInputElement;
							rename(kitchen.id, input.value);
						}}
					>
						<label class="sr-only" for={`kitchen-name-${kitchen.id}`}>
							{m.kitchen_name_label()}
						</label>
						<input
							id={`kitchen-name-${kitchen.id}`}
							name="name"
							class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-2 font-semibold text-ink"
							value={kitchen.name}
						/>
						<button class="quiet-button shrink-0 text-accent" type="submit">
							{m.kitchen_save()}
						</button>
					</form>

					<form
						class="mt-2 flex items-center gap-2"
						onsubmit={(event) => {
							event.preventDefault();
							const input = event.currentTarget.elements.namedItem('nickname') as HTMLInputElement;
							setNickname(kitchen.id, input.value);
						}}
					>
						<label class="shrink-0 text-label text-ink-2" for={`nickname-${kitchen.id}`}>
							{m.kitchen_nickname_label()}
						</label>
						<input
							id={`nickname-${kitchen.id}`}
							name="nickname"
							class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-ground px-2 text-body text-ink"
							placeholder={m.kitchen_nickname_placeholder()}
							value={kitchen.nickname ?? ''}
						/>
						<button class="quiet-button shrink-0 text-accent" type="submit">
							{m.kitchen_save()}
						</button>
					</form>

					<h3 class="mt-3 mb-1 text-label text-ink-2 uppercase">{m.kitchen_members()}</h3>
					<!-- Your own row carries no button: leaving is its own act at the
					     foot of the card, asked first with what each side keeps (#131,
					     screen choice 4). -->
					<ul class="grid gap-1">
						{#each kitchen.members as member (member.person_id)}
							<li class="flex items-center justify-between gap-3 text-body text-ink">
								<span>
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
										class="quiet-button shrink-0 text-accent"
										onclick={() => removeMember(kitchen.id, member.person_id)}
									>
										{m.kitchen_remove()}
									</button>
								{/if}
							</li>
						{/each}
					</ul>
					<!-- Whose recipes this Kitchen shows: every member's Cookbook,
					     your own first, as the Core answers them. -->
					{#if kitchen.cookbooks.length > 0}
						<p class="mt-2 text-read text-ink-2">
							{m.kitchen_recipes_here({
								cookbooks: joinedNames(kitchen.cookbooks.map(cookbookCalled)),
							})}
						</p>
					{/if}

					<!-- No focus move: the link appears where Invite someone was,
					     already under the finger that pressed it. -->
					{#if link?.kitchenId === kitchen.id}
						{@const made = link}
						<div class="mt-3 rounded-sm border border-accent bg-ground p-3" role="status">
							<p class="text-read text-ink-2">{m.kitchen_invite_link_said()}</p>
							<code
								class="mt-2 block rounded-sm border border-dashed border-rule bg-card p-2 text-read break-all"
								>{made.url}</code
							>
							<div class="mt-2 flex flex-wrap items-center gap-4">
								<button type="button" class="quiet-button text-accent" onclick={copyLink}>
									{copied ? m.cookbook_invite_copied() : m.cookbook_invite_copy()}
								</button>
								<button
									type="button"
									class="quiet-button text-support"
									onclick={() => cancelInvite(made.inviteId)}
								>
									{m.cookbook_invite_cancel()}
								</button>
							</div>
						</div>
					{:else}
						<button
							type="button"
							class="quiet-button mt-3 text-accent"
							onclick={() => invite(kitchen.id)}
						>
							{m.kitchen_invite()}
						</button>
					{/if}
					<div class="mt-3">
						<button
							type="button"
							class="quiet-button text-support"
							onclick={() => askToLeave(kitchen)}
						>
							{m.kitchen_leave_this()}
						</button>
					</div>
				</li>
			{/each}
		</ul>

		<Section heading={m.kitchen_create_title()}>
			<form
				class="grid gap-3"
				onsubmit={(event) => {
					event.preventDefault();
					create();
				}}
			>
				<label class="grid gap-1 text-body text-ink">
					{m.kitchen_create_name_label()}
					<input
						class="min-h-12 w-full rounded-sm border border-rule bg-card px-3"
						bind:value={newKitchenName}
						required
					/>
				</label>
				<button class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent">
					{m.kitchen_create()}
				</button>
			</form>
		</Section>
	{/if}
</Screen>

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
