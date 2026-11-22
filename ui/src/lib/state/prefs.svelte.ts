// UI preferences kept in localStorage, applied live (zoom, density, fonts).

export type Palette = "lajevard" | "dood";
/** Where file links from replies open. */
export type Editor = "default" | "vscode" | "cursor" | "zed";
export type Density = "comfortable" | "compact";
export type MessageWidth = "narrow" | "default" | "wide" | "full";
export type FontChoice = "system" | "sf" | "jetbrains" | "menlo" | "custom";
export type SendKey = "enter" | "mod-enter";
export type CursorStyle = "block" | "bar" | "underline";

const KEY = "glaude.prefs.v1";

export const fontStacks: Record<Exclude<FontChoice, "custom">, string> = {
	system: 'ui-monospace, "SF Mono", "Cascadia Code", "JetBrains Mono", Menlo, Consolas, monospace',
	sf: '"SF Mono", SFMono-Regular, ui-monospace, Menlo, monospace',
	jetbrains: '"JetBrains Mono", ui-monospace, Menlo, monospace',
	menlo: "Menlo, Monaco, ui-monospace, monospace",
};

export const widths: Record<MessageWidth, string> = { narrow: "44rem", default: "52rem", wide: "64rem", full: "100%" };

/** Everything saved; new fields get their default when missing. */
const defaults = {
	palette: "lajevard" as Palette,
	progressPanel: true,
	/** Show the output of the user's Claude statusLine.command instead of the built-in line. */
	statuslineScript: false,
	editor: "default" as Editor,
	/** Fold consecutive tool calls into one line. */
	compactTools: true,
	// Appearance
	uiScale: 1,
	density: "comfortable" as Density,
	codeFont: "system" as FontChoice,
	codeFontCustom: "",
	messageWidth: "default" as MessageWidth,
	// Composer
	sendKey: "enter" as SendKey,
	// Background and alerts
	keepAgents: true,
	idleMinutes: 15,
	notifyDone: true,
	notifyWaiting: true,
	notifyErrors: true,
	notifySound: false,
	/** Notifications from the agent host while no window is open. */
	notifyClosed: true,
	/** Remind when an agent has waited for you this long (minutes, 0 = never). */
	alertWaitMinutes: 5,
	/** Tell when one command or tool runs this long (minutes, 0 = never). */
	alertLongMinutes: 10,
	// Terminal
	termFont: "system" as FontChoice,
	termFontCustom: "",
	termFontSize: 13,
	termCursor: "block" as CursorStyle,
	termCursorBlink: true,
	/** Empty: the login shell. */
	termShell: "",
};
export type Prefs = typeof defaults;

class PrefStore {
	palette = $state(defaults.palette);
	progressPanel = $state(defaults.progressPanel);
	statuslineScript = $state(defaults.statuslineScript);
	editor = $state(defaults.editor);
	compactTools = $state(defaults.compactTools);
	uiScale = $state(defaults.uiScale);
	density = $state(defaults.density);
	codeFont = $state(defaults.codeFont);
	codeFontCustom = $state(defaults.codeFontCustom);
	messageWidth = $state(defaults.messageWidth);
	sendKey = $state(defaults.sendKey);
	keepAgents = $state(defaults.keepAgents);
	idleMinutes = $state(defaults.idleMinutes);
	notifyDone = $state(defaults.notifyDone);
	notifyWaiting = $state(defaults.notifyWaiting);
	notifyErrors = $state(defaults.notifyErrors);
	notifySound = $state(defaults.notifySound);
	notifyClosed = $state(defaults.notifyClosed);
	alertWaitMinutes = $state(defaults.alertWaitMinutes);
	alertLongMinutes = $state(defaults.alertLongMinutes);
	termFont = $state(defaults.termFont);
	termFontCustom = $state(defaults.termFontCustom);
	termFontSize = $state(defaults.termFontSize);
	termCursor = $state(defaults.termCursor);
	termCursorBlink = $state(defaults.termCursorBlink);
	termShell = $state(defaults.termShell);

	constructor() {
		try {
			const saved = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<Prefs> | null;
			if (saved) this.assign(saved);
		} catch {
			/* ignore corrupt prefs */
		}
	}

	private assign(values: Partial<Prefs>) {
		const self = this as unknown as Record<string, unknown>;
		for (const [k, v] of Object.entries(values)) {
			const d = (defaults as Record<string, unknown>)[k];
			if (d !== undefined && typeof v === typeof d) self[k] = v;
		}
	}

	snapshot(): Prefs {
		const self = this as unknown as Record<string, unknown>;
		return Object.fromEntries(Object.keys(defaults).map((k) => [k, self[k]])) as Prefs;
	}

	save() {
		localStorage.setItem(KEY, JSON.stringify(this.snapshot()));
	}

	/** Change one setting and save. */
	set<K extends keyof Prefs>(key: K, value: Prefs[K]) {
		(this as unknown as Prefs)[key] = value;
		this.save();
	}

	setPalette(p: Palette) {
		this.set("palette", p);
	}

	/** Put every setting in a group back to its default. */
	reset(keys: (keyof Prefs)[]) {
		this.assign(Object.fromEntries(keys.map((k) => [k, defaults[k]])) as Partial<Prefs>);
		this.save();
	}

	fontStack(choice: FontChoice, custom: string): string {
		if (choice === "custom" && custom.trim()) return `${custom.trim()}, ${fontStacks.system}`;
		return fontStacks[choice === "custom" ? "system" : choice];
	}

	zoom(step: number) {
		const next = step === 0 ? 1 : Math.round(Math.min(1.5, Math.max(0.75, this.uiScale + step)) * 100) / 100;
		this.set("uiScale", next);
	}
}

export const prefs = new PrefStore();

export const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);
