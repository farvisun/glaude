// Which tab is being dragged, shared by tab strips and pane drop zones.
class DragState {
	tabId = $state<string | null>(null);
}

export const drag = new DragState();
