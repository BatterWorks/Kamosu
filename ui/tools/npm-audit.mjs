// The npm half of `just audit` (#374). `npm audit` has nowhere to record an
// advisory that has been read and judged not to apply, which cargo-deny keeps
// in deny.toml. This reads npm's report and `ui/npm-audit-exceptions.json`,
// the list of what Kamosu has decided to live with and why, and fails on any
// other advisory at high or critical.
//
// It judges advisories, not packages. npm marks every package an advisory
// travels through with the advisory's severity, so `micromatch` is "high"
// only because `braces` is. Excusing the advisory excuses its whole path.

import { execFile } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { readFile } from 'node:fs/promises';

// The bar `npm audit --audit-level=high` held before this script took over.
const FAILING_SEVERITIES = new Set(['high', 'critical']);

export function judgeAudit(report, exceptions) {
	// npm writes its own failure, a registry it could not reach say, as a
	// report with no vulnerabilities in it. That is not a clean tree.
	if (report.error || !report.vulnerabilities) {
		throw new Error(`npm audit did not finish: ${JSON.stringify(report.error ?? report)}`);
	}
	// The reason is the point of the list, as it is in deny.toml.
	for (const exception of exceptions) {
		if (!exception.id || !exception.reason?.trim()) {
			throw new Error(`an exception needs an id and its reason: ${JSON.stringify(exception)}`);
		}
	}
	const reasons = new Map(exceptions.map((exception) => [exception.id, exception.reason]));
	const found = new Map();
	for (const vulnerability of Object.values(report.vulnerabilities)) {
		for (const via of vulnerability.via) {
			if (typeof via === 'string') {
				continue;
			}
			const id = advisoryId(via.url);
			const advisory = found.get(id) ?? {
				id,
				package: via.name,
				title: via.title,
				url: via.url,
				severity: via.severity,
				nodes: [],
			};
			advisory.nodes.push(...vulnerability.nodes);
			found.set(id, advisory);
		}
	}

	// An exception left behind once its advisory has gone, as deny.toml's
	// would be by cargo-deny, is named so that someone takes it out.
	const stale = [...reasons.keys()].filter((id) => !found.has(id));
	const judged = { failing: [], excused: [], belowTheBar: [], stale };
	for (const advisory of found.values()) {
		if (reasons.has(advisory.id)) {
			judged.excused.push({ ...advisory, reason: reasons.get(advisory.id) });
		} else if (FAILING_SEVERITIES.has(advisory.severity)) {
			judged.failing.push(advisory);
		} else {
			judged.belowTheBar.push(advisory);
		}
	}
	return judged;
}

// An advisory's GHSA id, which is how the exceptions name it. npm's own
// `source` number is an internal one that GitHub's pages do not show.
function advisoryId(url) {
	return url.slice(url.lastIndexOf('/') + 1);
}

// `node tools/npm-audit.mjs`, from ui/, as `just audit` runs it. npm audit
// exits non-zero whenever it finds anything at all, so its exit code says
// nothing here: the judgement is this script's.
async function main() {
	const output = await new Promise((resolve, reject) => {
		execFile(
			'npm',
			['audit', '--json'],
			{ maxBuffer: 64 * 1024 * 1024 },
			(error, stdout, stderr) => {
				if (stdout) {
					resolve(stdout);
				} else {
					reject(error ?? new Error(stderr));
				}
			},
		);
	});
	const { ignore } = JSON.parse(await readFile('npm-audit-exceptions.json', 'utf8'));
	const judged = judgeAudit(JSON.parse(output), ignore);

	const line = (advisory) =>
		`  ${advisory.severity.padEnd(8)} ${advisory.package}: ${advisory.title}\n` +
		`           ${advisory.url}\n           in ${[...new Set(advisory.nodes)].join(', ')}`;
	for (const [heading, advisories] of [
		['Failing:', judged.failing],
		['Below the bar (read them, do not obey them):', judged.belowTheBar],
	]) {
		if (advisories.length > 0) {
			console.log(heading);
			console.log(advisories.map(line).join('\n'));
		}
	}
	if (judged.excused.length > 0) {
		console.log('Excused in npm-audit-exceptions.json:');
		for (const advisory of judged.excused) {
			console.log(`${line(advisory)}\n           because ${advisory.reason}`);
		}
	}
	for (const id of judged.stale) {
		console.log(
			`note: ${id} is excused but npm no longer reports it; take it out of npm-audit-exceptions.json.`,
		);
	}
	if (judged.failing.length > 0) {
		process.exitCode = 1;
	} else {
		console.log('npm audit: nothing at high or critical that is not excused.');
	}
}

// Through the real path, as import.meta.filename is: a gate that skipped
// itself on a symlinked checkout would pass with nothing audited.
if (realpathSync(process.argv[1]) === import.meta.filename) {
	await main();
}
