import assert from 'node:assert/strict';
import { test } from 'node:test';

import { judgeAudit } from './npm-audit.mjs';

// The shape `npm audit --json` writes: one entry per vulnerable package, whose
// `via` holds either the advisory itself or the name of the package it is
// vulnerable through.
function reportedAdvisory(name, id, severity) {
	return {
		source: 1,
		name,
		dependency: name,
		title: `${name} is unsafe`,
		url: `https://github.com/advisories/${id}`,
		severity,
		range: '<=1.0.0',
	};
}

const BRACES = {
	auditReportVersion: 2,
	vulnerabilities: {
		braces: {
			name: 'braces',
			severity: 'high',
			via: [reportedAdvisory('braces', 'GHSA-vfj7-8cjw-p6xm', 'high')],
			nodes: ['node_modules/braces'],
		},
		micromatch: {
			name: 'micromatch',
			severity: 'high',
			via: ['braces'],
			nodes: ['node_modules/micromatch'],
		},
	},
};

const EXCUSED_BRACES = [{ id: 'GHSA-vfj7-8cjw-p6xm', reason: 'dev-only file watcher' }];

test('a high advisory fails the audit', () => {
	const judged = judgeAudit(BRACES, []);
	assert.deepEqual(
		judged.failing.map((found) => found.id),
		['GHSA-vfj7-8cjw-p6xm'],
	);
});

test('a listed advisory is excused, and the packages it reaches through with it', () => {
	const judged = judgeAudit(BRACES, EXCUSED_BRACES);
	assert.deepEqual(judged.failing, []);
	assert.deepEqual(
		judged.excused.map((found) => [found.id, found.reason]),
		[['GHSA-vfj7-8cjw-p6xm', 'dev-only file watcher']],
	);
});

test('a low advisory is shown but does not fail the audit', () => {
	const report = {
		vulnerabilities: {
			cookie: {
				name: 'cookie',
				severity: 'low',
				via: [reportedAdvisory('cookie', 'GHSA-pxg6-pf52-xh8x', 'low')],
				nodes: ['node_modules/cookie'],
			},
		},
	};
	const judged = judgeAudit(report, []);
	assert.deepEqual(judged.failing, []);
	assert.deepEqual(
		judged.belowTheBar.map((found) => found.id),
		['GHSA-pxg6-pf52-xh8x'],
	);
});

test('an exception npm no longer reports is named, so it can be taken out', () => {
	const judged = judgeAudit({ vulnerabilities: {} }, EXCUSED_BRACES);
	assert.deepEqual(judged.stale, ['GHSA-vfj7-8cjw-p6xm']);
});

test('a report npm could not finish is a failure, never a clean tree', () => {
	const report = { error: { code: 'ENOTFOUND', summary: 'request to registry failed' } };
	assert.throws(() => judgeAudit(report, []), /ENOTFOUND/);
});

test('an exception without its reason is refused', () => {
	assert.throws(() => judgeAudit(BRACES, [{ id: 'GHSA-vfj7-8cjw-p6xm' }]), /reason/);
});
