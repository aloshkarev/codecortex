# Optimization Methodology

Use this shared flow for optimizer roles.

## Steps

1. Baseline first:
   - define target metric (latency, throughput, memory, CPU).
2. Identify hotspots:
   - profiler, tracing, benchmark evidence.
3. Prioritize:
   - algorithm/data-layout wins before micro-tweaks.
4. Apply minimal high-impact change.
5. Re-measure:
   - compare before/after and report trade-offs.

## Required Output

- baseline metrics
- optimized approach
- measured or expected gain
- risks (complexity, readability, portability)
