import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import VoiceView from './VoiceView.svelte';

describe('VoiceView', () => {
	it('renders with correct id', () => {
		const { container } = render(VoiceView);
		const viewElement = container.querySelector('#view-voice');
		expect(viewElement).toBeTruthy();
		expect(viewElement?.classList.contains('view')).toBe(true);
	});

	it('displays mic pulse animation element', () => {
		const { container } = render(VoiceView);
		const micPulse = container.querySelector('.mic-pulse');
		expect(micPulse).toBeTruthy();
	});

	it('displays mic core icon', () => {
		const { container } = render(VoiceView);
		const micCore = container.querySelector('.mic-core');
		expect(micCore).toBeTruthy();
		expect(micCore?.textContent).toBe('◎');
	});

	it('displays push-to-talk label and keyboard hint', () => {
		const { container } = render(VoiceView);
		const micLabel = container.querySelector('.mic-label');
		expect(micLabel).toBeTruthy();
		expect(micLabel?.textContent).toContain('Push to Talk');
		expect(micLabel?.textContent).toContain('⌥ Space');
	});

	it('displays transcript card with mock data', () => {
		const { container } = render(VoiceView);
		const transcript = container.querySelector('.transcript');
		expect(transcript).toBeTruthy();

		// Check for transcript lines
		const transcriptLines = container.querySelectorAll('.t-line');
		expect(transcriptLines.length).toBeGreaterThan(0);
	});

	it('displays transcript lines with time and text', () => {
		const { container } = render(VoiceView);
		const firstLine = container.querySelector('.t-line');
		expect(firstLine).toBeTruthy();

		const time = firstLine?.querySelector('.t-time');
		const text = firstLine?.querySelector('.t-text');
		expect(time).toBeTruthy();
		expect(text).toBeTruthy();
	});

	it('distinguishes between user and system transcript lines', () => {
		const { container } = render(VoiceView);
		const userLines = container.querySelectorAll('.t-line-me');
		const systemLines = container.querySelectorAll('.t-line-sys');

		expect(userLines.length).toBeGreaterThan(0);
		expect(systemLines.length).toBeGreaterThan(0);
	});

	it('displays parsed intent card', () => {
		const { container } = render(VoiceView);
		const intentCard = container.querySelector('.intent-card');
		expect(intentCard).toBeTruthy();
	});

	it('displays intent key-value pairs', () => {
		const { container } = render(VoiceView);
		const intentRows = container.querySelectorAll('.intent-row');
		expect(intentRows.length).toBeGreaterThan(0);

		// Check for key and value elements
		const firstRow = intentRows[0];
		const key = firstRow.querySelector('.intent-k');
		const value = firstRow.querySelector('.intent-v');
		expect(key).toBeTruthy();
		expect(value).toBeTruthy();
	});

	it('displays confidence bar with percentage', () => {
		const { container } = render(VoiceView);
		const confidenceSection = container.querySelector('.intent-confidence');
		expect(confidenceSection).toBeTruthy();

		const confidenceBar = container.querySelector('.intent-bar');
		const confidenceFill = container.querySelector('.intent-fill');
		const confidenceVal = container.querySelector('.intent-val');

		expect(confidenceBar).toBeTruthy();
		expect(confidenceFill).toBeTruthy();
		expect(confidenceVal).toBeTruthy();
		expect(confidenceVal?.textContent).toMatch(/\d+%/);
	});
});
