export function fmtTokens(n?: number | null): string {
	if (n == null) return "–";
	if (n >= 1e9) return (n / 1e9).toFixed(1) + "B";
	if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
	if (n >= 1e3) return (n / 1e3).toFixed(1) + "k";
	return String(n);
}

export function fmtCost(usd?: number | null): string {
	if (usd == null) return "–";
	return usd < 1 ? `$${usd.toFixed(3)}` : `$${usd.toFixed(2)}`;
}

export function fmtDuration(ms?: number | null): string {
	if (ms == null) return "–";
	const s = Math.floor(ms / 1000);
	if (s < 60) return `${s}s`;
	const m = Math.floor(s / 60);
	if (m < 60) return `${m}m ${s % 60}s`;
	const h = Math.floor(m / 60);
	return `${h}h ${m % 60}m`;
}

/** Time until an ISO timestamp, e.g. "2h 14m". */
export function fmtUntil(iso?: string | null): string {
	if (!iso) return "";
	const ms = new Date(iso).getTime() - Date.now();
	if (!Number.isFinite(ms) || ms <= 0) return "now";
	const m = Math.round(ms / 60000);
	if (m < 60) return `${m}m`;
	const h = Math.floor(m / 60);
	if (h < 48) return `${h}h ${m % 60}m`;
	return `${Math.floor(h / 24)}d ${h % 24}h`;
}

/** Time since an ISO timestamp or epoch ms, e.g. "5m ago". */
export function fmtAgo(v?: string | number | null): string {
	if (v == null || v === "") return "never";
	const t = typeof v === "number" ? v : new Date(v).getTime();
	const s = Math.max(0, Math.round((Date.now() - t) / 1000));
	if (s < 60) return "just now";
	const m = Math.floor(s / 60);
	if (m < 60) return `${m}m ago`;
	const h = Math.floor(m / 60);
	if (h < 24) return `${h}h ago`;
	const d = Math.floor(h / 24);
	if (d < 30) return `${d}d ago`;
	return new Date(t).toLocaleDateString();
}

export function basename(p?: string | null): string {
	if (!p) return "";
	const parts = p.split(/[\\/]/).filter(Boolean);
	return parts[parts.length - 1] ?? p;
}

/** Colour tier for "percent left": >50 ok, >20 attention, else destructive. */
export function leftTone(left: number): "ok" | "attention" | "destructive" {
	return left > 50 ? "ok" : left > 20 ? "attention" : "destructive";
}
