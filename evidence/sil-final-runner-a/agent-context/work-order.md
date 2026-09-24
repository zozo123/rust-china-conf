Fix the seeded stale-observation regression in robot-safety-gate. At dispatch,
observations older than 250 ms must return StalePerception. Preserve the
simulated emergency-stop precedence and timestamp validation. Preserve
boundary behavior at 250 and 251 ms. Do not edit simulator code, fixtures,
the threshold, acceptance checks, runner scripts or evidence generation.
Return the patch and relevant test output.
