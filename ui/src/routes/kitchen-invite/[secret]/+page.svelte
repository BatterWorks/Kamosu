<!--
	`/kitchen-invite/<secret>`: the page a Kitchen Invite link opens (#469,
	choice B of 26 October 2026). It shows the Kitchen as its members see it,
	its name, who is in it and whose recipes it holds, since joining means
	everyone in it sees your recipes and that is the thing to know before
	saying yes. The prototype is `.dev/prototypes/469-settings/`
	`invite-b-open-phone.html` and its states.

	Opening the link changes nothing. Only *Join* does, and *Not now* leaves
	the link unspent for later.

	A Kitchen Invite is for somebody who already has an account; making one is
	an Operator's Invite's job. So a reader who is not signed in gets the
	ordinary sign-in form here, saying where to ask for an account, and the
	same page once they are signed in. A Person who is signed in keeps their
	navigation on this page, as on a Cookbook Invite's (#214).

	The secret arrives as the `params` prop, so a test can render this route
	itself with a secret of its choosing.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { cookbookCalled, joinedNames } from '$lib/cookbook';
	import { heardWhetherSignedIn } from '$lib/shell/signing-in.svelte';
	import type { ReadKitchenInviteOutput } from '$lib/api/catalogue';
	import Account from '../../Account.svelte';
	import type { PageProps } from './$types';

	let { params }: PageProps = $props();

	const kamosu = useKamosu();

	/**
	 * What the link would do; undefined while asking; `'sign-in'` when there
	 * is nobody to ask for; `'gone'` for a link used, cancelled or never minted.
	 */
	let invite = $state<ReadKitchenInviteOutput | 'sign-in' | 'gone' | undefined>(undefined);
	let failed = $state<string | undefined>(undefined);
	let busy = $state(false);
	/** Bumped when the sign-in form succeeds, which asks again. */
	let asked = $state(0);

	$effect(() => {
		void asked;
		const secret = params.secret;
		let current = true;
		invite = undefined;
		kamosu
			.readKitchenInvite({ secret })
			.then((answer) => {
				heardWhetherSignedIn(true);
				if (current) invite = answer;
			})
			.catch(async (error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (!current) return;
				if (error.kind !== 'unauthorized') {
					invite = 'gone';
					return;
				}
				// A link that names nothing is refused as one somebody may not
				// read, so whether anybody is signed in is asked apart.
				const signedIn = await kamosu.listKitchens().then(
					() => true,
					(refused: unknown) => {
						if (!(refused instanceof OperationError)) throw refused;
						return false;
					},
				);
				heardWhetherSignedIn(signedIn);
				if (current) invite = signedIn ? 'gone' : 'sign-in';
			});
		return () => {
			current = false;
		};
	});

	async function join() {
		busy = true;
		failed = undefined;
		try {
			await kamosu.acceptKitchenInvite({ secret: params.secret });
			await goto('/settings/kitchens');
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			busy = false;
		}
	}
</script>

{#snippet nothing()}{/snippet}

{#snippet noAccount()}
	<p class="mt-4 text-read text-ink-2">{m.kitchen_invite_no_account()}</p>
{/snippet}

<svelte:head>
	<title>{typeof invite === 'object' ? invite.kitchen.name : m.app_name()}</title>
</svelte:head>

{#if invite === 'sign-in'}
	<Account
		onSignedIn={() => (asked += 1)}
		framing={{
			title: m.kitchen_invite_sign_in(),
			blurb: '',
			submit: m.account_login(),
			before: nothing,
			after: noAccount,
		}}
	/>
{:else}
	<div class="mx-auto max-w-2xl px-gutter pt-6 pb-8">
		{#if invite === undefined}
			<p class="text-body text-ink-2">{m.loading()}</p>
		{:else if invite === 'gone'}
			<p class="text-body text-ink" role="alert">{m.kitchen_invite_gone()}</p>
			<a class="mt-4 inline-block text-label text-accent underline" href="/">{m.home_title()}</a>
		{:else if invite.already_member}
			<p class="text-body text-ink">{m.kitchen_invite_already()}</p>
			<a class="mt-4 inline-block text-label text-accent underline" href="/settings/kitchens"
				>{m.settings_kitchens_page()}</a
			>
		{:else}
			{@const kitchen = invite.kitchen}
			<p class="text-label text-ink-2 uppercase">
				{m.kitchen_invite_kicker({ name: invite.invited_by.name })}
			</p>
			<div
				class="mt-2 min-w-0 rounded-sm border border-t-4 border-rule border-t-accent bg-card p-3"
			>
				<h1 class="font-display text-title font-semibold break-words text-ink">{kitchen.name}</h1>
				<h2 class="mt-3 mb-1 text-label text-ink-2 uppercase">{m.kitchen_members()}</h2>
				<ul class="grid gap-1">
					{#each kitchen.members as member (member.person_id)}
						<li class="text-body text-ink">{member.name}</li>
					{/each}
				</ul>
				{#if kitchen.cookbooks.length > 0}
					<p class="mt-2 text-read text-ink-2">
						{m.kitchen_recipes_here({
							cookbooks: joinedNames(kitchen.cookbooks.map(cookbookCalled)),
						})}
					</p>
				{/if}
			</div>
			<p class="mt-4 text-body text-ink">{m.kitchen_invite_joining()}</p>
			{#if failed}
				<p class="mt-3 text-body text-support" role="alert"><Refusal words={failed} /></p>
			{/if}
			<div class="mt-6 flex flex-col gap-2">
				<button
					type="button"
					onclick={join}
					disabled={busy}
					class="min-h-12 rounded-sm bg-accent px-4 text-body font-semibold text-on-accent
					disabled:opacity-60"
				>
					{m.kitchen_invite_join()}
				</button>
				<a
					href="/"
					class="flex min-h-12 items-center justify-center rounded-sm border border-rule px-4 text-body
					font-medium text-ink-2"
				>
					{m.kitchen_invite_not_now()}
				</a>
			</div>
		{/if}
	</div>
{/if}
