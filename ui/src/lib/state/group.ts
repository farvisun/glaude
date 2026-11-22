// How a conversation's items are laid out: a subagent's own messages and
// tool calls go inside its card, and the tool call behind a question or
// plan card is not shown twice.
import type { Item } from "$lib/types";

export function groupItems(items: Item[]): { main: Item[]; steps: Record<string, Item[]> } {
	const agentCalls = new Set<string>();
	const prompted = new Set<string>();
	for (const it of items) {
		if (it.kind === "tool" && (it.name === "Agent" || it.name === "Task")) agentCalls.add(it.id);
		else if ((it.kind === "question" || it.kind === "permission") && it.toolUseId) prompted.add(it.toolUseId);
	}
	const main: Item[] = [];
	const steps: Record<string, Item[]> = {};
	for (const it of items) {
		const parent = it.kind === "tool" || it.kind === "assistant" ? it.parent : undefined;
		if (parent && agentCalls.has(parent)) (steps[parent] ??= []).push(it);
		else if (it.kind === "tool" && prompted.has(it.id)) continue;
		else main.push(it);
	}
	return { main, steps };
}
