// Tab badges: running / needs attention. Filled by conversation stores.
import type { Tab } from "./layout.svelte";

export type Badge = "running" | "attention" | "unread" | null;

class Badges {
	byRef = $state<Record<string, Badge>>({});

	set(ref: string, badge: Badge) {
		if (this.byRef[ref] !== badge) this.byRef[ref] = badge;
	}
}

export const badges = new Badges();

export function tabBadge(tab: Tab): Badge {
	return tab.ref ? (badges.byRef[tab.ref] ?? null) : null;
}
