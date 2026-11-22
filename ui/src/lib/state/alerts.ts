// Reminders (Settings → Background & alerts): an agent that has waited for
// you for a while, and a command or tool that runs unusually long. Each
// fires once per approval or call.
import { conversations } from "./conversations.svelte";
import { pendingOf, currentTool, toolStep, waitingText, fmtElapsed } from "./activity";
import { prefs } from "./prefs.svelte";
import { notifyAlert } from "./notify";

const fired = new Set<string>();
let timer: ReturnType<typeof setInterval> | null = null;

export function checkAlerts(now = Date.now()): { convId: string; body: string }[] {
	const out: { convId: string; body: string }[] = [];
	for (const conv of Object.values(conversations.byId)) {
		const wait = prefs.alertWaitMinutes * 60_000;
		const pending = wait > 0 ? pendingOf(conv) : undefined;
		const since = pending ? conv.timeOf(pending.id)?.start : undefined;
		if (pending && since && now - since >= wait && !fired.has(`wait:${pending.id}`)) {
			fired.add(`wait:${pending.id}`);
			out.push({ convId: conv.id, body: `Still waiting for you (${fmtElapsed(now - since)}): ${waitingText(pending)}` });
		}
		const long = prefs.alertLongMinutes * 60_000;
		const tool = long > 0 && conv.running ? currentTool(conv) : undefined;
		const started = tool ? conv.timeOf(tool.id)?.start : undefined;
		if (tool && started && now - started >= long && !fired.has(`long:${tool.id}`)) {
			fired.add(`long:${tool.id}`);
			out.push({ convId: conv.id, body: `${toolStep(tool)} has been running for ${fmtElapsed(now - started)}.` });
		}
	}
	return out;
}

export function startAlerts() {
	if (timer) return;
	timer = setInterval(() => {
		for (const a of checkAlerts()) {
			const conv = conversations.get(a.convId);
			if (conv) notifyAlert(conv, a.body);
		}
	}, 30_000);
}
