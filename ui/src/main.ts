import "./app.css";

// The theme saved last time, before anything paints (no flash of Sade).
try {
	const vars = JSON.parse(localStorage.getItem("glaude.theme.boot") ?? "null") as Record<string, string> | null;
	if (vars) for (const [k, v] of Object.entries(vars)) document.documentElement.style.setProperty(k, v);
} catch {
	/* ignore */
}
import { mount } from "svelte";
import App from "./App.svelte";
import { inTauri } from "$lib/ipc";

const app = mount(App, { target: document.getElementById("app")! });

// Browser-only helper so previews and Playwright can open the scripted
// demo session without a real agent.
if (!inTauri && import.meta.env.DEV) {
	import("$lib/state/sessions").then(({ startConversation }) => {
		(window as unknown as Record<string, unknown>).__demo = (provider: "claude" | "codex" = "claude") =>
			startConversation({
				provider,
				account: "default",
				cwd: "/Users/demo/Projects/glaude",
				resume: "demo",
				title: "رفع مشکل جهت متن",
			});
	});
}

export default app;
