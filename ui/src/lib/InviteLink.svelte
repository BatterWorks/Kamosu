<!--
	An invite just made (#648): a Kitchen's, a Cookbook's or a new account's,
	drawn one way. It says it works once, until it is used or cancelled (a
	Secret is never on a clock, ADR 0031), hands the link over the way a
	recipe's link is (`LinkToSend`), and keeps cancelling apart under a line,
	asking first (U11), so tidying the screen cannot end a link already sent
	(the audit's I8).

	It appears where the button that made it was, already under the finger
	that pressed it, so it takes no focus. The link is shown only here: the
	Core keeps its hash, so once the page is left it can be cancelled from a
	list but never shown again.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import Confirm from '#lib/Confirm.svelte';
	import NoticeCard from '#lib/NoticeCard.svelte';
	import LinkToSend from '#lib/LinkToSend.svelte';
	import { OperationError } from '#lib/api/client.js';

	interface Props {
		title: string;
		url: string;
		/** One line on what this invite gives, where it gives more than a place. */
		after?: string;
		/** Ends the invite. A refusal comes back in the sheet, in its own words. */
		cancel: () => Promise<void>;
		/** Its space above: under a button by default, none first in a Section. */
		class?: string;
	}

	let { title, url, after, cancel, class: spaced = 'mt-4' }: Props = $props();

	let asking = $state(false);
	let busy = $state(false);
	let failed = $state<string | undefined>(undefined);

	async function run() {
		busy = true;
		failed = undefined;
		try {
			await cancel();
			asking = false;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			busy = false;
		}
	}
</script>

<NoticeCard {title} tone="waiting" class={spaced}>
	<p class="text-body">{m.invite_works_once()}</p>
	{#if after}
		<p class="mt-1 text-ink-2">{after}</p>
	{/if}
	<div class="mt-4"><LinkToSend {url} /></div>
	<div class="mt-6 border-t border-rule pt-3">
		<button
			type="button"
			class="quiet-button text-support"
			onclick={() => {
				failed = undefined;
				asking = true;
			}}
		>
			{m.invite_cancel_ask()}
		</button>
	</div>
</NoticeCard>

{#if asking}
	<Confirm
		title={m.invite_cancel_title()}
		consequence={m.invite_cancel_said()}
		act={m.invite_cancel_do()}
		cancelLabel={m.invite_cancel_keep()}
		{busy}
		{failed}
		{run}
		cancel={() => (asking = false)}
	/>
{/if}
