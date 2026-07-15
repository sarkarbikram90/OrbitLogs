-- Migration to optimize chunk intervals for high-throughput production workloads (6 hours chunks instead of 1 day)
SELECT set_chunk_time_interval('telemetry', INTERVAL '6 hours');
SELECT set_chunk_time_interval('events', INTERVAL '6 hours');
