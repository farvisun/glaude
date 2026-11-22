// Thin typed wrapper over Tauri IPC. When the UI runs in a plain browser
// (vite dev, Playwright) calls go to the mock backend instead.

import type { Channel as TauriChannel } from "@tauri-apps/api/core";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Handler = (args: Record<string, unknown>) => unknown | Promise<unknown>;
let mockHandlers: Record<string, Handler> | null = null;

export function setMock(handlers: Record<string, Handler>) {
	mockHandlers = handlers;
}

export async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
	if (inTauri) {
		const { invoke } = await import("@tauri-apps/api/core");
		return invoke<T>(cmd, args);
	}
	if (!mockHandlers) {
		const { mockBackend } = await import("./mock");
		mockHandlers = mockBackend;
	}
	const h = mockHandlers[cmd];
	if (!h) throw new Error(`mock: no handler for ${cmd}`);
	return (await h(args)) as T;
}

/** A message channel from Rust to the UI. In the browser mock it is a
 *  plain callback holder the mock pushes into. */
export interface Chan<T> {
	onmessage: (msg: T) => void;
	raw: TauriChannel<T> | MockChannel<T>;
}

export class MockChannel<T> {
	onmessage: (msg: T) => void = () => {};
	push(msg: T) {
		this.onmessage(msg);
	}
}

export async function channel<T>(onmessage: (msg: T) => void): Promise<Chan<T>> {
	if (inTauri) {
		const { Channel } = await import("@tauri-apps/api/core");
		const ch = new Channel<T>();
		ch.onmessage = onmessage;
		return { onmessage, raw: ch };
	}
	const ch = new MockChannel<T>();
	ch.onmessage = onmessage;
	return { onmessage, raw: ch };
}

export interface ToolInfo {
	path: string | null;
	version: string | null;
}

export interface AppInfo {
	version: string;
	os: string;
	home: string;
	claude: ToolInfo;
	codex: ToolInfo;
}

export const api = {
	appInfo: () => call<AppInfo>("app_info"),
};
