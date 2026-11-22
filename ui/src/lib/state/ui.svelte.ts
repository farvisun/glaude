// Transient UI state: dialogs, palettes, loading flags.
import { layout } from "./layout.svelte";

class UiState {
	newConversationOpen = $state(false);
	searchOpen = $state(false);
	sessionsLoading = $state(false);

	openNewConversation() {
		this.newConversationOpen = true;
	}

	openSearch() {
		this.searchOpen = true;
	}

	openTerminal() {
		layout.openTab({ kind: "terminal", title: "Terminal" });
	}
}

export const ui = new UiState();
