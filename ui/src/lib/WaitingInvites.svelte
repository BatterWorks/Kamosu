<!--
	The invites nobody has opened yet (#648), as a Kitchen and a Cookbook list
	them: when each was made, and Cancel. The link itself is never shown, since
	the Core keeps only its hash; the screen asks before cancelling one (U11).
	Draws nothing while none is waiting.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { howLongAgo } from '#lib/dates.js';

	interface Props {
		invites: { invite_id: string; created_at: string }[];
		/** Ask before cancelling this one. */
		ask: (inviteId: string) => void;
	}

	let { invites, ask }: Props = $props();
</script>

{#if invites.length > 0}
	<div class="mt-4">
		<p class="text-body font-semibold text-ink">{m.invites_waiting()}</p>
		{#each invites as each (each.invite_id)}
			<div class="flex min-h-12 items-center justify-between gap-3 border-b border-rule">
				<span class="text-body text-ink"
					>{m.invite_made({ when: howLongAgo(each.created_at) })}</span
				>
				<button
					type="button"
					class="quiet-button shrink-0 text-support"
					onclick={() => ask(each.invite_id)}
				>
					{m.invite_cancel_short()}
				</button>
			</div>
		{/each}
	</div>
{/if}
