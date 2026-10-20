<!--
	Signing out on this device (#380): the question asked before it, from the
	foot of the Settings menu and from this device's row of the Sessions list
	alike.

	What the device wrote offline is sent first (#77), because signing out
	loses whatever has not gone: the next Session to begin here throws it
	away. Almost always it goes, and the sheet asks the ordinary question,
	which says the recipes kept here for offline use go too (#76). When some
	still cannot go, the sheet says how many instead, and signing out anyway
	is a choice of its own, as Todoist asks "Discard unsynced changes?"
	(docs/research/signing-out-in-other-apps.md).
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { OperationError } from '$lib/api/client';
	import { useKamosu } from '$lib/kamosu';
	import { useKeeping } from '$lib/offline/outbox';
	import { heardWhetherSignedIn } from '$lib/shell/signing-in.svelte';

	interface Props {
		/** The Session in your hand, where the screen already knows it. */
		sessionId?: string;
		onclose: () => void;
	}

	let { sessionId, onclose }: Props = $props();

	const kamosu = useKamosu();
	const keeping = useKeeping();

	/**
	 * Where the sheet stands: sending what waits, asking, warning that some
	 * changes could not go, or ending the Session.
	 */
	type Doing =
		| { at: 'sending'; waiting: number }
		| { at: 'asking' }
		| { at: 'warning'; waiting: number }
		| { at: 'leaving'; waiting: number };

	let doing = $state<Doing>({ at: 'sending', waiting: 0 });
	let failed = $state<string | undefined>(undefined);

	const waiting = $derived(doing.at === 'asking' ? 0 : doing.waiting);

	/** The title and the line under it, for what the sheet is asking. */
	const said = $derived(
		waiting === 0
			? { title: m.sign_out_ask_title(), consequence: m.sign_out_ask_consequence() }
			: waiting === 1
				? {
						title: m.sign_out_unsent_title_one(),
						consequence: m.sign_out_unsent_consequence_one(),
					}
				: {
						title: m.sign_out_unsent_title({ count: waiting }),
						consequence: m.sign_out_unsent_consequence(),
					},
	);

	async function send() {
		// The count stays as it was while trying again, so the sheet does not
		// jump to the ordinary question and back.
		doing = { at: 'sending', waiting };
		const left = await keeping.sendWaiting();
		doing = left === 0 ? { at: 'asking' } : { at: 'warning', waiting: left };
	}

	onMount(() => void send());

	/** Home is the sign-in form to somebody who is not signed in. */
	async function signedOut() {
		// Said before Home is drawn, so the sidebar is gone with the Session
		// rather than a moment after it (#194).
		heardWhetherSignedIn(false);
		await goto('/');
	}

	async function signOut() {
		const was = doing;
		doing = { at: 'leaving', waiting };
		failed = undefined;
		try {
			const id =
				sessionId ?? (await kamosu.listSessions()).sessions.find((session) => session.current)?.id;
			// With no Session in hand there is nothing to end.
			if (id) await kamosu.revokeSession({ session_id: id });
			await signedOut();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			// The Session had already ended, from another device or a
			// recovery link: this device was signed out, and only did not know.
			if (error.kind === 'unauthorized') return signedOut();
			failed = error.message;
			doing = was;
		}
	}

	function cancel() {
		if (doing.at !== 'leaving') onclose();
	}
</script>

<SheetFrame
	labelledby="sign-out-title"
	tall={78}
	class="overflow-y-auto border-t border-rule bg-ground px-gutter pt-6"
	onclose={cancel}
	onconfirm={doing.at === 'asking' ? signOut : undefined}
>
	<h2 id="sign-out-title" class="font-display text-list-title font-semibold text-ink">
		{said.title}
	</h2>

	<p class="mt-3 border-l-2 border-accent pl-3 text-body text-ink-2">{said.consequence}</p>

	{#if doing.at === 'sending'}
		<p class="mt-3 text-read text-ink-2" role="status">{m.sign_out_sending()}</p>
	{/if}

	{#if failed}
		<p class="mt-3 text-body text-support" role="alert">{failed}</p>
	{/if}

	<div class="mt-6 mb-4 flex flex-col gap-2">
		{#if waiting === 0}
			<button
				type="button"
				onclick={signOut}
				disabled={doing.at !== 'asking'}
				class="min-h-12 rounded-sm border border-accent bg-accent px-4 text-body font-medium
				text-card disabled:opacity-60"
			>
				{doing.at === 'leaving' ? m.sign_out_working() : m.sign_out()}
			</button>
		{:else}
			<button
				type="button"
				onclick={send}
				disabled={doing.at !== 'warning'}
				class="min-h-12 rounded-sm border border-accent bg-accent px-4 text-body font-medium
				text-card disabled:opacity-60"
			>
				{m.sign_out_wait()}
			</button>
			<button
				type="button"
				onclick={signOut}
				disabled={doing.at !== 'warning'}
				class="min-h-12 rounded-sm border border-support px-4 text-body font-medium
				text-support disabled:opacity-60"
			>
				{doing.at === 'leaving' ? m.sign_out_working() : m.sign_out_anyway()}
			</button>
		{/if}
		<button
			type="button"
			onclick={cancel}
			disabled={doing.at === 'leaving'}
			class="min-h-12 rounded-sm px-4 text-body font-medium text-accent disabled:opacity-60"
		>
			{m.sign_out_cancel()}
		</button>
	</div>
</SheetFrame>
