<!-- Import from another app (#412), a page of Settings' Add recipes (#469). -->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import ImportFromAnotherApp from '../ImportFromAnotherApp.svelte';
</script>

<Screen title={m.settings_import_page()}>
	<ImportFromAnotherApp />
</Screen>
