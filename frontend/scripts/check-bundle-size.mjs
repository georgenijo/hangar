import { readFileSync, readdirSync, statSync } from 'fs';
import { join, extname } from 'path';
import { gzipSync } from 'zlib';

const BUILD_DIR = new URL('../build/_app/', import.meta.url).pathname;
const LIMIT = 500_000;

function findJs(dir) {
	const results = [];
	for (const entry of readdirSync(dir, { withFileTypes: true })) {
		const full = join(dir, entry.name);
		if (entry.isDirectory()) {
			results.push(...findJs(full));
		} else if (extname(entry.name) === '.js') {
			results.push(full);
		}
	}
	return results;
}

let total = 0;
const chunks = [];

for (const file of findJs(BUILD_DIR)) {
	const raw = readFileSync(file);
	const gz = gzipSync(raw);
	chunks.push({ file: file.replace(BUILD_DIR, ''), size: gz.length });
	total += gz.length;
}

chunks.sort((a, b) => b.size - a.size);

for (const { file, size } of chunks) {
	console.log(`  ${(size / 1024).toFixed(1).padStart(7)} KB  ${file}`);
}

console.log(`\nTotal gzipped JS: ${(total / 1024).toFixed(1)} KB / ${(LIMIT / 1024).toFixed(0)} KB limit`);

if (total > LIMIT) {
	console.error(`\nBUNDLE TOO LARGE: ${(total / 1024).toFixed(1)} KB > ${(LIMIT / 1024).toFixed(0)} KB`);
	process.exit(1);
}

console.log('Bundle size OK');
