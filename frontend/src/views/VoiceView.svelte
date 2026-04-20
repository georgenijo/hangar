<script lang="ts">
	// VoiceView: Static mock UI for voice control feature
	// Per PRD: stub/mock for now, real voice integration is Phase 10

	// Static mock transcript data
	const mockTranscript = [
		{ time: '14:32:01', text: 'Show me the active sessions', speaker: 'me' },
		{ time: '14:32:02', text: 'Showing 3 active sessions', speaker: 'sys' },
		{ time: '14:32:15', text: 'Create a new pipeline for issue 79', speaker: 'me' },
		{ time: '14:32:16', text: 'Pipeline created for issue #79', speaker: 'sys' }
	];

	// Static mock intent data
	const mockIntent = {
		action: 'create_pipeline',
		issue: '79',
		host: 'optiplex',
		auto_merge: 'false',
		review_required: 'true',
		notify: 'true',
		confidence: 0.92
	};
</script>

<div id="view-voice" class="view">
	<!-- Voice Stage with Mic Pulse Animation -->
	<div class="voice-stage">
		<div class="mic-pulse">
			<div class="mic-core">◎</div>
		</div>
		<div class="mic-label">
			<span>Push to Talk</span>
			<kbd class="kbd">⌥ Space</kbd>
		</div>
	</div>

	<!-- Transcript Card -->
	<div class="card" style="margin-bottom: 16px;">
		<div class="card-head">
			<h3>Transcript</h3>
			<span class="hint-text">Last 4 utterances</span>
		</div>
		<div class="transcript">
			{#each mockTranscript as line}
				<div class="t-line t-line-{line.speaker}">
					<div class="t-time">{line.time}</div>
					<div class="t-text">{line.text}</div>
				</div>
			{/each}
		</div>
	</div>

	<!-- Parsed Intent Card -->
	<div class="card">
		<div class="card-head">
			<h3>Parsed Intent</h3>
			<span class="hint-text">Extracted from last command</span>
		</div>
		<div class="intent-card">
			{#each Object.entries(mockIntent) as [key, value]}
				{#if key !== 'confidence'}
					<div class="intent-row">
						<div class="intent-k">{key}:</div>
						<div class="intent-v">{value}</div>
					</div>
				{/if}
			{/each}
			<div class="intent-confidence">
				<div class="intent-label">Confidence:</div>
				<div class="intent-bar">
					<div class="intent-fill" style="width: {mockIntent.confidence * 100}%"></div>
				</div>
				<div class="intent-val">{Math.round(mockIntent.confidence * 100)}%</div>
			</div>
		</div>
	</div>
</div>
