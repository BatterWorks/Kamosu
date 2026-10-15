<!--
	A Runner Key's Secret, shown once in a sheet of its own (#275, option 1 of
	15 October 2026). Closing it by Escape is the same as Done: the Secret is
	gone either way, and the sheet has said so.

	Whether it went onto the clipboard lives in here, as it does in
	`OneTimeSecret`, so each new key starts at "Copy" by being a new sheet.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { copyText } from '$lib/clipboard';
	import SheetFrame from '$lib/SheetFrame.svelte';

	let { name, secret, ondone }: { name: string; secret: string; ondone: () => void } = $props();

	let copied = $state(false);
</script>

<SheetFrame
	labelledby="runner-secret-title"
	tall={78}
	class="overflow-y-auto border-t border-rule bg-ground px-gutter pt-6"
	onclose={ondone}
>
	<h2 id="runner-secret-title" class="font-display text-list-title font-semibold text-ink">
		{m.operator_runner_secret_title({ name })}
	</h2>
	<p class="mt-3 text-body text-ink-2">{m.operator_runner_secret_once()}</p>
	<code class="mt-3 block rounded-sm bg-card p-2 text-read break-all select-all">{secret}</code>
	<div class="mt-6 mb-4 flex flex-col gap-2">
		<button
			type="button"
			onclick={async () => (copied = await copyText(secret))}
			class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent"
		>
			{copied ? m.access_key_secret_copied() : m.access_key_secret_copy()}
		</button>
		<button
			type="button"
			onclick={ondone}
			class="min-h-12 rounded-sm px-4 text-body font-medium text-accent"
		>
			{m.access_key_secret_dismiss()}
		</button>
	</div>
</SheetFrame>
