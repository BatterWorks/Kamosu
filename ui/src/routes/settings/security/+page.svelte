<!-- Your security record (#371, #469): yours alone, since the instance's is a page of This instance. -->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import SecurityRecord from '../SecurityRecord.svelte';
</script>

<Screen title={m.settings_security_record()}>
	<SecurityRecord whose="yours" />
</Screen>
