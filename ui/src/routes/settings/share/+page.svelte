<!-- Save from your phone (#452), a page of Settings' Add recipes (#469). -->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import ShareFromYourPhone from '../ShareFromYourPhone.svelte';
</script>

<Screen title={m.settings_share_page()}>
	<ShareFromYourPhone />
</Screen>
