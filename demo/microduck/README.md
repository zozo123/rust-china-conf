# The safety-gate port onto microduck

`safety-gate-port.patch` applies to github.com/pollen-robotics/microduck at `a9ec4b2`
(Apache-2.0, upstream, unmodified). It is our work, not theirs, and was never pushed there.

It moves the freshness contract from the robosuite harness onto a real Rust biped whose
control loop runs at 50 Hz, and enforces it in the type system rather than at runtime:

  Sensors { observed_at: Instant }   private; Instant::now() is the only constructor, so a
                                     backend can carry a stamp but cannot write one
  ObservationAge                     no Default, no From<Duration>, no public field;
                                     ObservationAge::of(&Sensors) is the only way in
  Safety::apply(.., age: Option<ObservationAge>)
                                     by value, by type: a dispatch site with no observation
                                     does not compile

"I checked the age" stops being a claim a caller can make. `..Sensors::default()` stopped
compiling in safety.rs's own tests, which is the rule working.

The contract bound is 80 ms = 20 ms x (COAST_TICKS + 1), derived from robotd's own loop rate
rather than chosen. The seed is `let _ = age;` with the Limit variant, the wire name, the
config field and the params-registry entry all present -- everything a reviewer inspects says
the rule exists; only the comparison is missing.

WHAT THIS DEMO CANNOT DO, measured: the balance cliff is 46-66 ms, BELOW the 80 ms bound, so
the duck falls before the contract trips. There is no window where it succeeds while the
contract fails. The harness measures this every run and prints `calibration: no`. The
"robot succeeded, contract failed" hook belongs to the robosuite demo, not this one.

Repro: docs/results/duck-demo.md. Full arc 193.89 s.
