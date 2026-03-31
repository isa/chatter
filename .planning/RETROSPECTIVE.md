# Retrospective

## Milestone: v1.1 — ChatterBox Engine Support

**Shipped:** 2026-03-31  
**Phases:** 6 directories scoped to v1.1 (04–09, with 04/05 superseded by 06) | **Plans:** 9 | **Tasks:** 13 (from automated milestone summary)

### What was built

Second TTS engine (ChatterBox) alongside Qwen3-TTS: cloning, variants, model/doctor integration, engine-specific CLI controls, and hardening from milestone audit (non-interactive engine mismatch, curated installs).

### What worked

- Implementing dispatch and CLI engine selection **with** ChatterBox inference (Phase 06) avoided an unused abstraction phase.
- Dedicated Phase 09 for audit gaps fixed real CLI/doctor footguns without expanding feature scope.

### What was inefficient

- Superseded Phase 04/05 plans stayed on disk without summaries, which skewed automated “plans complete” counts until manual interpretation.
- Milestone audit YAML stayed on `gaps_found` until closure metadata was updated at ship time.

### Key lessons

- When plans are superseded, mark roadmap checkboxes and optionally add stub summaries or exclude dirs from tooling so progress dashboards stay honest.
- Close the audit record (status + closure note) when gap phases land, so `/gsd-complete-milestone` pre-flight is unambiguous.

## Cross-Milestone Trends

| Milestone | Phases (exec) | Theme |
| --------- | ------------- | ----- |
| v1.0 | 01–03 | Qwen3-TTS MVP |
| v1.1 | 06–09 (+ superseded 04/05) | Multi-engine + ChatterBox |
