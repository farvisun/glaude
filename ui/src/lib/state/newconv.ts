// Defaults for new conversations: shared by the dialog (which remembers the
// last choices unless turned off) and Settings → New conversations.
import type { Provider } from "$lib/types";

const KEY = "glaude.newconv.v1";

export interface NewConvDefaults {
	provider: Provider;
	cwd: string;
	account: Record<Provider, string>;
	mode: Record<Provider, string>;
	model: Record<Provider, string>;
	effort: Record<Provider, string>;
	recent: string[];
	/** Update the defaults with each conversation's choices. */
	remember: boolean;
}

const fallback = (): NewConvDefaults => ({
	provider: "claude",
	cwd: "",
	account: { claude: "default", codex: "default" },
	mode: { claude: "default", codex: "full" },
	model: { claude: "", codex: "" },
	effort: { claude: "", codex: "" },
	recent: [],
	remember: true,
});

export function loadNewConv(): NewConvDefaults {
	const base = fallback();
	try {
		const saved = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<NewConvDefaults> | null;
		if (!saved) return base;
		return {
			...base,
			...saved,
			account: { ...base.account, ...saved.account },
			mode: { ...base.mode, ...saved.mode },
			model: { ...base.model, ...saved.model },
			effort: { ...base.effort, ...saved.effort },
		};
	} catch {
		return base;
	}
}

export function saveNewConv(v: NewConvDefaults) {
	localStorage.setItem(KEY, JSON.stringify(v));
}

export const claudeModels = [
	{ value: "", label: "Default" },
	{ value: "opus", label: "Opus" },
	{ value: "sonnet", label: "Sonnet" },
	{ value: "haiku", label: "Haiku" },
	{ value: "fable", label: "Fable" },
];

export const efforts: Record<Provider, { value: string; label: string }[]> = {
	claude: [
		{ value: "", label: "Default" },
		{ value: "low", label: "Low" },
		{ value: "medium", label: "Medium" },
		{ value: "high", label: "High" },
		{ value: "xhigh", label: "Extra high" },
		{ value: "max", label: "Max" },
	],
	codex: [
		{ value: "", label: "Default" },
		{ value: "minimal", label: "Minimal" },
		{ value: "low", label: "Low" },
		{ value: "medium", label: "Medium" },
		{ value: "high", label: "High" },
		{ value: "xhigh", label: "Extra high" },
	],
};
