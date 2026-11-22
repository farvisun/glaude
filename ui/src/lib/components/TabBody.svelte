<script lang="ts">
	import type { Tab } from "$lib/state/layout.svelte";
	import WelcomeTab from "./WelcomeTab.svelte";
	import SettingsTab from "./SettingsTab.svelte";
	import ConversationView from "./conv/ConversationView.svelte";
	import TerminalTab from "./TerminalTab.svelte";
	import AccountsTab from "./accounts/AccountsTab.svelte";

	let { tab, active }: { tab: Tab; active: boolean } = $props();
</script>

{#if tab.kind === "welcome"}
	<WelcomeTab />
{:else if tab.kind === "settings"}
	<SettingsTab />
{:else if tab.kind === "conversation" && tab.ref}
	<ConversationView convId={tab.ref} {active} />
{:else if tab.kind === "terminal"}
	<TerminalTab {tab} {active} />
{:else if tab.kind === "accounts"}
	<AccountsTab />
{/if}
