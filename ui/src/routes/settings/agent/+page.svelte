<!-- Sent to the AI agent (#463), a page of Settings' Add recipes. -->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import SentToAgent from '../SentToAgent.svelte';
</script>

<Screen title={m.settings_sent_page()}>
	<SentToAgent />
</Screen>
