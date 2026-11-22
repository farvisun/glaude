// Git for the Changes tab and worktree conversations.
import type { FileDiff } from "$lib/types";

export interface GitInfo {
	root: string;
	branch: string;
	base?: string | null;
	upstream?: string | null;
	ahead: number;
	behind: number;
	dirty: number;
	hasCommits: boolean;
	worktree: boolean;
	managed: boolean;
	gh: boolean;
}

export interface Commit {
	sha: string;
	short: string;
	subject: string;
	author: string;
	date: string;
}

export interface PullRequest {
	url: string;
	number: number;
	state: string;
	title: string;
}

export interface ProjectAction {
	name: string;
	command: string;
}

export interface ProjectConfig {
	worktree?: { setup?: string; cleanup?: string };
	actions?: ProjectAction[];
}

export type DiffSource = "uncommitted" | "branch" | "commit";

/** Files whose path or diff text contains `q` (case-insensitive). */
export function filterDiffs(diffs: FileDiff[], q: string): FileDiff[] {
	const t = q.trim().toLowerCase();
	if (!t) return diffs;
	return diffs.filter((d) => d.path.toLowerCase().includes(t) || d.diff.toLowerCase().includes(t));
}
