# RUST-FINAL — the proof layer of `swf-cli`, final state

**Repo** `rust-china-conf`, branch `fix/proof-layer-and-public-claims` (PR #2, not committed here).
**Crate** `rust/crates/swf-cli` — `src/main.rs`, one file, 41 tests green.
**Toolchain** pinned 1.92.0. Build: `cd rust && cargo build -p swf-cli`.
**Untouched** `rust/crates/robot-safety-gate` — its 3 seeded contract failures are the demo's premise and are still failing, by design.

---

## 0. The one-line answer

**Yes, `build-proof` can still be forged — by exactly one adversary, and the tool says so out loud.**
A party who can write *both* the receipt *and* the transcript files on disk can make them agree, because
nothing is signed. Every *other* forgery that has been tried against this tool — and eleven distinct ones
have been, by three adversarial reviewers — now exits 1. What changed in this round is that the tool no
longer *prints* anything it did not do. That was the last place the old defect was still alive.

---

## 1. The forgeries, and what stops each one now

The talk's thesis is that unverified claims get caught. The tool that makes that argument has now been
caught five times, always by the same failure mode: **something that looks verified but is only asserted.**

### Forgery 1 — the schema-v1 tautology *(closed earlier)*

`BuildProof` was *constructed* carrying `cache_scope`, `cache_cleared_before_each_cold_sample` and
`cache_cleared_before_each_parent_seed` as literals, and `validate_build_proof` then "validated" those
same literals. Three of its eight fail-closed conditions could never fire. A fabricated receipt printed
`BUILD PROOF PASS … measured ratio=2.999x vs native; saved=14000ms`, exit 0.

**What stops it:** schema 2 deleted those fields. Cache state is carried per sample as a `CacheClear`, and
`CacheClear` has exactly one constructor — `parse_cache_clear`, which takes the *text of a file*. There is
no code path in the binary that builds one from literals, in production or in tests.
Guards: `cache_state_is_never_asserted_by_construction`,
`schema_one_receipts_are_rejected_because_their_attestations_were_literals`.

### Forgery 2 — the fix moved the hole *(closed in this change set)*

Schema 2 introduced transcripts, and `check_clear_usable` then checked that `transcript_sha256` was 64
characters of lowercase hex and that `transcript_path` was non-empty. **It never opened the file.** A
receipt of 5 real native samples plus 10 hand-written Incredibuild samples — `remote_tasks=412`,
`transcript_path: "/tmp/does-not-exist.txt"`, `transcript_sha256:` sixty-four zeros — printed
`BUILD RECEIPT CONSISTENT` and `ib-parent-warm measured ratio=11.948x vs native; saved=7116ms`, exit 0.
Nothing was invented but JSON fields.

**What stops it:** `corroborate_clear`. At proof time every cache-clear record in the receipt — not only
the one the ordering selects — is resolved to a real path, the file is read once, the SHA-256 is recomputed
over the bytes that were read, and the receipt is refused unless the file corroborates its `exit_code`,
`started_at_ms`, `completed_at_ms`, `argv` and re-derived `scope`. Verified against the built binary:

```
$ swf-cli robot-demo build-proof --receipt fabrication-original.json ; echo $?
Error: ib-cold sample 1: cache clear "user clear"
Caused by:
    transcript /tmp/does-not-exist.txt is not present at its recorded path; if the retained
    transcripts were moved, or this receipt was produced on another host, pass --transcripts <dir>
1
```

One byte appended to a genuine transcript: exit 1, `hashes to 9fc2d609…, but the receipt records …; this
receipt does not describe this file`.

### Forgery 3 — the transcript could not testify *(closed, shell side)*

`cache-clear.sh` wrote `argv=$*` *after* the tool name had been shifted off, producing records like
`argv=-rf /path` that cannot say which tool ran. It now emits `# swf-cache-clear v2` with `tool=` and
`command=` lines; the parser accepts v1 and v2 via `CACHE_CLEAR_MARKERS`, cross-checks that `command` is
`tool` followed by `argv`, and **refuses a v1 file carrying a `tool=` line**, since the v1 recorder could
not have written one. v1 transcripts still corroborate a timeline, and the summary counts them separately
and says in words that *which tool* emptied the cache is not established for them.

### Forgery 4 — the prose was still lying *(closed in this round)*

Two reviewers, independently, refuted the claim that the printed paragraphs were true. The paragraph headed
`CHECKED FROM THIS RECEIPT'S OWN FIELDS` asserted *"one Build History record per caption reporting
success"*. Nothing on the build-proof path ever read a Build History record; that check lives in
`parse_ib_history` and runs only at `build-sample` record time, on another host, in another invocation.
At proof time `build_caption` was touched exactly once, an `is_empty()` test. There was not even a
uniqueness check. A receipt giving all 15 samples the caption `THE-SAME-CAPTION-FOR-EVERY-BUILD`, with no
Build History document anywhere on the machine, exited 0 while printing that sentence.

This is the schema-v1 defect rebuilt **in prose**: a record-time property presented as checked at proof
time. It was also contradicted three lines later by the NOT CHECKED paragraph.

**What stops it:** two changes, not one.
1. The clause moved to the NOT CHECKED tier, where it now names what actually happened: *"`build-sample`
   did check, WHEN EACH SAMPLE WAS RECORDED, that the Build History held exactly one record for that
   caption reporting success; that happened in another invocation, on another host, against a document this
   one cannot see."*
2. A real proof-time check was added in its place — the only thing about a caption this invocation *can*
   establish: **every sample and every parent seed must carry a distinct build caption.** A caption names
   one build to Incredibuild's Build History, so fifteen samples under one caption are at most one build
   measured fifteen times.

```
$ swf-cli robot-demo build-proof --receipt forged-same-caption.json ; echo $?
Error: build caption "THE-SAME-CAPTION-FOR-EVERY-BUILD" is carried by both native sample 1 and
ib-cold sample 1; one caption is one build, so these cannot both be measurements of their own
1
```

Guards: `two_builds_may_not_share_one_build_caption`,
`the_checked_tier_names_no_document_and_the_not_checked_tier_names_them_all`.

### Forgery 5 — the *count* of evidence was receipt-controlled *(closed in this round)*

`CacheEvidence::clears` incremented once per `CacheClear` **JSON object**, and `print_build_proof` called
that number *"corroborated clear transcript(s)"*. The ledger deduplicated the file *reads*, but nothing
deduplicated the *counter*. A receipt could therefore cite one genuine transcript five hundred times and
print `500 corroborated clear transcript(s)` over one file. Reproduced by two reviewers: 11 files on disk,
20 citations, printed 20, exit 0. The doc comment on the field and the test named
`the_printed_clear_count_counts_transcripts_opened_in_this_invocation` *both asserted the opposite of what
the code did*, and the test could not catch it because the fixture gave every sample its own transcript —
"count of files" and "count of records" were the same number in every case it built.

**What stops it:** three changes.
1. `CacheEvidence::transcripts` is now `ledger.read.len()` — distinct **files** opened, taken from the
   ledger *after* the walk, not counted as the receipt is walked. `anonymous_transcripts` is derived the
   same way.
2. The citation count is printed **beside** it, never instead of it, so inflating the number the receipt
   controls only makes the gap visible:
   `those 10 file(s) are cited by 18 cache-clear record(s) in this receipt`.
3. A sample citing the same resolved path twice is now **refused outright** — one file cited twice is one
   piece of evidence, not two.

Guards: `the_printed_clear_count_counts_transcripts_opened_in_this_invocation` (extended with a fixture
where the two readings differ, which is what the old one lacked), `a_sample_may_not_cite_one_transcript_twice`.

### Break 6 — the validator would have refused the truth *(fixed in this round)*

Not a forgery: the opposite. `ib-benchmark.sh` documents `empty_cache_hit_floor=1` — cargo invokes
`rustc -vV` twice per build and Incredibuild serves the second invocation the entry the first stored, an
intra-build self-hit — while `validate_build_proof` bailed unless an `ib-cold` sample reported
`cache_hits == Some(0)`, and `verify_cache_chain` bailed unless the parent seed reported `0`. The parent
seed builds on a freshly emptied cache too. A reviewer built the receipt the script's own documentation
describes and ran it: `Error: ib-cold sample 1 was not empty-cache (hits=Some(1))`, exit 1. Nothing fails
early, so **the benchmark would have run to completion and died on its own last command.**

**The fix, and why it is not a weakening.** `--empty-cache-hit-floor <N>`, default `0`:

* It is supplied by the **verifier** on the command line. The receipt cannot set it. A receipt may not
  choose the standard it is judged by — the same asymmetry as `--transcripts`.
* It **tightens as it loosens**: an emptied-cache build may report at most `N` hits, *and a warm build must
  report more than `N`*. Raising the floor to excuse a cold sample raises the bar every warm sample has to
  clear, so it cannot be used as a pure loosener.
* At the default `0` every pre-existing bail fires **exactly** as it did before (`hits > 0` and
  `hits != Some(0)` are the same predicate over `u64`).
* When it is non-zero the summary prints a paragraph saying so, and saying that **nothing here establishes
  that the hits it allowed were that self-hit.**

`ib-benchmark.sh` now passes `--empty-cache-hit-floor 1` with a comment tying it to the `method.txt` line,
so the two halves can no longer drift apart.

```
$ swf-cli robot-demo build-proof --receipt genuine-with-self-hit.json                        → exit 1
      Error: ib-cold sample 1 was not empty-cache (hits=Some(1), floor=0)
$ swf-cli … --empty-cache-hit-floor 1                                                        → exit 0
$ swf-cli … --empty-cache-hit-floor 1   (warm samples also only at 1)                        → exit 1
      Error: ib-parent-warm sample 1 has no verified cache hits above the empty-cache floor of 1
```

---

## 2. What one `build-proof` invocation actually establishes

The output is tiered, and the tiers are functions (`re_observed_paragraph`, `checked_paragraph`,
`not_checked_paragraph`) rather than literals inside `println!` — **so that tests can read the claims.**
As literals, nothing could, which is precisely how forgery 4 survived a green suite.

### Tier 1 — RE-OBSERVED (established by opening a file, in this invocation)

For **every** cache-clear record in the receipt, not only the one the ordering selects:

* the path resolves (as recorded, or by basename inside `--transcripts <dir>`) to a real file;
* the file is read **once**, and the SHA-256 is computed over exactly the bytes that are then parsed;
* that digest equals the receipt's `transcript_sha256`, byte for byte;
* the file re-parses as a `# swf-cache-clear v1`/`v2` transcript;
* its `argv`, `exit_code`, `started_at_ms`, `completed_at_ms` and the `scope` **re-derived from its own
  arguments** all equal the receipt's fields — every disagreement is collected and reported together;
* for a v2 transcript, `command` is `tool` followed by `argv`;
* a v1 file carrying `tool=` or `command=` is refused as edited after the fact;
* two recorded paths that relocate onto one basename are refused as ambiguous;
* the printed count is the number of **files opened**, taken from the ledger.

A receipt naming a file that is missing, that has changed by one byte, or that contradicts it, is
**refused**, not summarized.

### Tier 2 — CHECKED (cross-field logic over numbers the receipt asserts; no document is opened)

* ≥ `--min-samples` samples per mode carrying **distinct repetition numbers** — *distinct* is all that is
  shown, since the receipt's writer picks the numbers;
* a **distinct build caption** for every sample and every parent seed;
* remote task and remote core-time counters **present** for every IB sample and every parent seed — an
  absent counter is *unknown*, never zero — plus `local_tasks` and `cache_misses`; native samples carrying
  no IB telemetry at all;
* the **distribution contract** the verifier selected with `--distribution`:
  * `required` (default): every IB sample and every parent seed reports `remote_tasks > 0` and
    `remote_core_time_s > 0`, or the run did not measure distribution at all;
  * `excluded`: every IB sample and every parent seed reports `remote_tasks == 0` **and**
    `remote_core_time_s == 0` exactly — the contract for the cache-only profile
    (`rust/ib_profile.cache-only.xml`: rustc `local_only` with `<ib_cache enabled="true"/>`). This is not a
    relaxation: a run that leaked one task to a helper is refused, and every cache check stays in force.
    With distribution verified *absent*, the warm-vs-cold delta has nothing but the Build Cache to come
    from. Each setting refuses what the other demands, so `--distribution` cannot be used to excuse a run;
* cold-cache and parent-seed hits ≤ floor, warm-cache hits > floor, as recorded;
* `candidate_revision` ≠ `parent_revision`, every sample built the candidate, every seed built the parent;
* no two cache-using builds sharing a start instant;
* **for every `ib-cold` sample**: a local-user cache clear that exited 0 and completed before it, with no
  cache-using build in between;
* **for every `ib-parent-warm` sample**: that same ordering between the clear and its **parent seed**, and
  no cache-using build between that seed and the measured build — *the seed itself ran between the clear
  and the measured build, which is what warming a cache means.*

That last bullet is a correction. The old wording claimed "every IB build preceded by a local-user cache
clear … with no other cache-using build between the clear and it", which is **false for every warm sample
by design**, and self-refuting one clause after it reported "warm-cache hits>0": a build with nothing
between it and a cache clear cannot have cache hits.

### Tier 3 — NOT CHECKED

* **No document behind any counter is retained with a digest or re-read here** — not the Build History
  responses behind `remote_tasks`/`local_tasks`, not the cache-statistics output behind
  `cache_hits`/`cache_misses`, not the per-task Build Cache report, and not one of the parent-seed
  counters, which is what the attribution of a warm cache to its parent rests on.
* The Build History / one-record-reporting-success check happened at **`build-sample` record time**, on
  another host, against a document this invocation cannot see.
* **`wall_ms` is not corroborated by anything**, and every ratio printed is computed from it.
* swf-cli **does not observe the cache**: a transcript shows that the named tool ran and what it printed,
  not that the namespace was empty, and nothing rules out another process repopulating it between the clear
  and the build.
* Every timestamp is the runner's own clock, unattested. Nothing binds a transcript to the machine, user or
  filesystem that ran the build.
* **Neither the receipt nor the transcripts are signed**, so a party able to write both can make them
  agree — *including the tool names printed above*, which are read from those same transcripts.
* A non-zero `--empty-cache-hit-floor` is the verifier's own judgement; nothing corroborates that the hits
  it allowed were the intra-build self-hit.

---

## 3. Remaining structural holes, stated plainly

### H1. A party who writes both the receipt and the transcripts

**Status: open, structural, disclosed in the output.** A reviewer demonstrated it twice: once with
transcripts produced by the genuine `cache-clear.sh` wrapping a "cache tool" that prints
`cache namespace user: purged 0 entries (this tool is a lie)` and touches nothing; once with transcripts
hand-typed in Python that no recorder ever produced. Both exit 0 and print `ratio=119.949x`. The
`tool=` line — the one signal that exposes the first — is itself forger-chosen in the second.

**What it would take to close it:** the transcript must be signed by something the forger does not control
at receipt-writing time, and the signature must be over the transcript *and* the receipt fields that cite
it. In practice: a recorder that holds a key the benchmark user cannot read, or an append-only transparency
log the verifier queries independently, or receipts counter-signed by the Incredibuild coordinator that
actually ran the builds. Anything short of that and "these bytes and this JSON agree" remains the whole
claim. **This is not fixable inside the file format.**

### H2. The task and cache counters have no retained source document

`remote_tasks`, `local_tasks`, `remote_core_time_s`, `cache_hits`, `cache_misses` and every parent-seed
counter are read from Build History and cache-statistics responses at `build-sample` time, and **the
responses are not kept with digests.** At proof time they are the receipt's word.

**What it would take to close it:** the shape is already proven by the cache-clear path. `build-sample`
already *has* the documents — `ib-benchmark.sh` writes them to `raw/<caption>.history.json`,
`raw/<caption>.cache.txt` and `raw/<caption>.cache-report.txt`. Give each sample a `sources: [{kind, path,
sha256}]` list produced the same way a `CacheClear` is, and have `verify_cache_chain`'s sibling re-open,
re-hash and re-parse them and refuse unless the re-derived counters equal the receipt's. That is a
mechanical extension of `corroborate_clear` — about a day's work — and it would also close H3.

### H3. `wall_ms` is unattested, and every ratio comes from it

**Status: open.** No document is retained for the wall time, and nothing cross-checks it. A receipt whose
transcripts are all genuine can still carry any duration at all.

**What it would take to close it:** Build History already reports each build's start and end. Retaining
that response (H2) makes `wall_ms` cross-checkable against a second source; it does not make it *attested*,
which again needs H1.

### H4. The empty-cache hit floor is asserted, not shown

A non-zero floor is the verifier's judgement that the allowed hits were the `rustc -vV` self-hit. The
per-task Build Cache report **does** show it, block by block, and `ib-benchmark.sh` already writes it to
`raw/<caption>.cache-report.txt`.

**What it would take to close it:** retain that report with a digest (H2) and have the validator derive the
self-hit count from it rather than take a number on the command line. Then the floor becomes evidence
instead of a flag.

### H5. A benchmark is a measurement, not a proof

Even with H1–H4 closed, this tool would establish that a set of builds happened as described on one machine
at one time. It would not establish that the result generalizes, that the workload is representative, or
that the comparison is fair. `swf-cli` does not claim any of those and neither should the talk.

---

## 4. The guard tests — 41, and the regression each one protects

`cd rust && cargo test -p swf-cli` → **41 passed, 0 failed.** `cargo fmt -p swf-cli -- --check` clean;
`cargo clippy -p swf-cli --all-targets -- -D warnings` clean.

### The corroboration path (the hole this change set closed)

| test | regression it protects against |
|---|---|
| `the_receipt_that_printed_ratio_11_948x_from_invented_fields_is_now_refused` | The original fabrication, rebuilt byte for byte. Asserts every *receipt-only* check still passes it — so the refusal demonstrably comes from the filesystem — and that it can never print a ratio again. |
| `a_transcript_digest_that_does_not_match_the_file_is_refused` | A real file, a well-formed digest, different bytes. |
| `a_transcript_edited_after_the_receipt_was_written_is_refused` | One trailing space, which still parses identically. |
| `a_receipt_that_contradicts_the_transcript_on_disk_is_refused` | Receipt vs file on `started_at_ms`, `completed_at_ms`, `argv`, and a `scope` the receipt calls local-user while the file's own arguments say shared. |
| `a_receipt_claiming_exit_zero_over_a_transcript_recording_failure_is_refused` | A genuine digest of a genuine file that records a failed clear. Also pins that the *recording* path still refuses outright. |
| `every_clear_is_corroborated_not_only_the_one_the_ordering_selects` | A fabricated *earlier* clear, which `effective_clear` never selects and which used to sit in the receipt uncontested. |
| `the_digest_is_computed_over_the_bytes_that_are_parsed` | Hashing the path and then reading the path — two opens, so the bytes hashed are not provably the bytes parsed. Also pins that the digest still equals `evidence::sha256_file`, so recorder and verifier cannot drift. |
| `corroboration_has_no_opt_out` | A `skip_transcripts` / `assume_valid` / `trust_digest` escape hatch reappearing; a third `TranscriptSource` constructor; the digest comparison being dropped; the counts going back to being walked over the receipt. |
| `cache_state_is_never_asserted_by_construction` | A `CacheClear` built from literals anywhere in the binary. |
| `schema_one_receipts_are_rejected_because_their_attestations_were_literals` | Schema-1 receipts, whose attestations were the tautology. |

### Relocation (`--transcripts`)

| test | regression |
|---|---|
| `build_proof_defaults_to_the_recorded_paths_and_takes_a_relocation_directory` | Omitting the flag quietly meaning "do not check". |
| `transcripts_relocate_by_basename_and_relocation_is_never_a_bypass` | An empty directory passing; a tampered copy inside the relocation directory passing. |
| `two_recorded_paths_cannot_relocate_onto_the_same_transcript` | Two recorded paths collapsing onto one basename — ambiguity resolved by coin flip instead of refusal. |
| `a_recorded_path_cannot_escape_the_relocation_directory` | `../elsewhere/mine.txt` letting the receipt pick its own evidence. |

### The transcript grammar

| test | regression |
|---|---|
| `cache_clear_transcripts_are_parsed_not_assumed` | Defaulted or inferred header fields. |
| `v2_transcripts_must_name_a_tool_and_a_command_that_matches_their_argv` | A `command` line that is not `tool` + `argv`; a missing `tool` or `command`. |
| `a_v1_transcript_corroborates_the_timeline_but_never_claims_which_tool_ran` | A v1 transcript being counted as naming its tool. |
| `a_v1_transcript_naming_a_tool_could_not_have_been_written_by_the_v1_recorder` | A v1 file with a `tool=` line hand-added afterwards. |
| `the_cache_tool_a_transcript_names_is_reported_not_discarded` | `tool=/bin/true` being parsed and thrown away instead of surfaced. |

### The cache-ordering argument

| test | regression |
|---|---|
| `an_ib_sample_without_an_observed_clear_is_unknown_not_clean` | An empty `cache_clears` list reading as "clean". |
| `a_clear_is_rejected_unless_it_is_local_user_successful_and_retained` | Shared scope, unknown scope, non-zero exit, missing digest — the four receipt-only bails, still firing before any filesystem work. |
| `a_clear_must_precede_the_build_it_is_claimed_to_have_prepared` | A genuine clear that finished after the build began. |
| `a_clear_cannot_be_reused_across_an_intervening_cache_using_build` | Another cache-using build between the clear and the build it is credited to. |
| `warm_cache_hits_must_be_attributable_to_a_recorded_parent_seed` | A missing seed; a seed that ran on a warm cache; a seed that built the wrong revision; a seed with no remote tasks. |
| `native_samples_may_not_carry_cache_evidence` | Native samples smuggling IB telemetry. |
| `build_proof_requires_distribution_and_cache_hits` | Zero remote tasks, zero remote core time, incomplete telemetry, warm samples with no hits. |
| `build_proof_requires_five_samples_per_mode_with_distinct_repetitions` | Too few samples; two records claiming the same repetition. |

### The claims the tool prints *(new in this round)*

| test | regression |
|---|---|
| `the_checked_tier_names_no_document_and_the_not_checked_tier_names_them_all` | The CHECKED tier naming a document this invocation never opened (the Build History clause); the word "independent"; the warm-sample ordering sentence going back to a form that is false for warm samples; anything disappearing from NOT CHECKED. |
| `two_builds_may_not_share_one_build_caption` | Fifteen samples under one caption exiting 0; a seed borrowing a sample's caption or another seed's. |
| `the_printed_clear_count_counts_transcripts_opened_in_this_invocation` | The printed count going back to counting JSON records — with a fixture where the two readings differ, which the old version lacked. |
| `a_sample_may_not_cite_one_transcript_twice` | One file cited twice being summarized as two pieces of evidence. |
| `the_empty_cache_hit_floor_is_the_verifier_s_and_defaults_to_strict` | The floor defaulting to anything but 0; the floor loosening the cold test without tightening the warm one; the parent seed being judged by a different number. |

### The distribution contract (`--distribution required｜excluded`)

| test | regression |
|---|---|
| `the_cache_only_contract_forbids_distribution_rather_than_ignoring_it` | `excluded` degenerating into "ignore distribution". A distributed receipt must FAIL `excluded`, a cache-only receipt must FAIL the default `required`, and one leaked remote task or any remote core time must refuse `excluded` — including from a parent seed. |
| `the_cache_only_contract_leaves_every_cache_check_in_force` | A cache check being skipped because distribution was excluded: the flat-cache, warm-as-cold and missing-clear refusals all still fire under `excluded`. |
| `a_missing_remote_counter_never_satisfies_the_cache_only_contract` | An absent counter reading as zero, letting a receipt satisfy the cache-only contract by saying nothing at all. |

*(Four further tests — `identifiers_cannot_escape_evidence_or_scenario_directories`,
`clap_rejects_invalid_backend_timeout_and_paths`, `explicit_validation_scenarios_are_repeatable`,
`only_expected_timeouts_are_nonfatal_infrastructure_results` — and two parser tests,
`ib_history_parser_selects_caption_and_documented_counters` and
`cache_parser_requires_unambiguous_hits_and_misses`, guard the surrounding CLI and the record-time parsers.)*

### Mutation evidence (a green suite is not evidence that the suite works)

The fix was reverted ten different ways, on an isolated copy of the workspace, and the suite re-run each
time (`shasum -a 256` of the restored file identical, 41/41 green).

| reversion | tests that fail |
|---|---|
| gut `corroborate_clear` entirely | **13** |
| disable the digest comparison | 3 |
| disable the receipt/transcript field cross-check | 3 |
| relocation joins the whole recorded path | 2 |
| count citations instead of ledger files | 2 (incl. the opt-out guard) |
| allow a duplicate citation | 2 |
| drop the caption-uniqueness check | 1 |
| ignore the floor on the warm side | 1 |
| hash the path, then read it again | 1 (the opt-out guard) |
| put "one Build History record per caption reporting success" back in CHECKED | 1 |

Every failure is the test whose *name* describes that regression.

---

## 5. What the speaker may say on stage

**True as written:**

* "The receipt is a claim. The validator's job is to refuse claims it cannot corroborate."
* "At proof time this tool opens every cache-clear transcript the receipt names, re-hashes it over the
  bytes it parses, and refuses the receipt unless the file agrees with it on the argument vector, the exit
  status, both timestamps and the cache scope — for every clear, not just the one the timeline picks."
* "The receipt that printed an 11.9x speed-up from a file called `/tmp/does-not-exist.txt` and a digest of
  sixty-four zeros now exits 1. Here it is." *(run it live — it is a one-second demo)*
* "Change one byte of a transcript and the receipt that described it stops validating."
* "Everything this tool prints is in one of three tiers: what it re-observed by opening a file, what it
  checked by cross-referencing fields the receipt asserts, and what it did not check at all. The third tier
  is the longest one, and that is deliberate."
* "It told me three times that I had written something that looked verified and was only asserted. The
  third time, it was the printout itself."
* "Run it with `--distribution excluded` and it refuses any receipt in which a single task crossed the
  network — so when it does pass, whatever acceleration was measured came from the Build Cache and nowhere
  else. That flag is a demand, not a waiver: the same receipt fails the default contract, and the default
  contract's receipt fails this one."
* "A party who can write both the receipt and the transcripts can still make them agree. Nothing here is
  signed. The tool says so in its own output, above my signature and below the number."

**Must not be said:**

* ~~"This proves Incredibuild made the build 11.9x faster."~~ It proves nothing about the builds. Every
  task and cache counter, and every wall time the ratio is computed from, is the receipt's word.
* ~~"The benchmark is verified."~~ The **transcripts** are corroborated. The **builds** are not.
* ~~"This receipt cannot be forged."~~ See H1. Say instead: *"these specific forgeries are refused, and here
  is the one that is not."*
* ~~"Five independent samples per mode."~~ Distinct repetition numbers are not independence, and the
  receipt's writer picks them. The tool no longer prints the word, and neither should the slide.

**The honest framing, and it is a stronger story:** the tool was written to catch unverified claims and it
caught its own author three times. Twice in the logic, once in the prose. That is the talk.

---

## 6. Reproducing all of it

```bash
cd rust-china-conf/rust
cargo test -p swf-cli                                   # 41 passed
cargo fmt -p swf-cli -- --check                         # clean
cargo clippy -p swf-cli --all-targets -- -D warnings    # clean
cargo test --workspace                                  # 3 failures, ALL in robot-safety-gate, by design
                                                        #   boundary_251ms_rejects
                                                        #   stale_ages_are_rejected
                                                        #   configured_threshold_is_respected
```

`cargo fmt --all -- --check` exits 1 on three hunks in `robot-safety-gate` (`src/lib.rs:133`,
`tests/contract.rs:14`, `tests/contract.rs:73`). That is pre-existing drift in the crate the demo requires
to be left alone; it is **not** fixed here.

Blast radius: `rust/crates/swf-cli/src/main.rs`, `rust/crates/swf-cli/Cargo.toml` (`sha2`, `hex` — both
already in this workspace's graph via `swf-app`, so no new crate is compiled), `rust/Cargo.lock` (2 lines),
`scripts/robot-demo/ib-benchmark.sh`, and the new `rust/ib_profile.cache-only.xml`. `git diff HEAD -- rust/crates/robot-safety-gate/` is empty and
the seeded `let _ = (age_ms, policy);` is still at `src/lib.rs:147`.

Bail count in the production half of `main.rs`: **49 at HEAD → 66 now.** None removed. Three were reworded
to compare against `ProofPolicy::empty_cache_hit_floor`, and at the default floor of 0 each is the same
predicate it was; the distribution bails were split per contract, and the `required` side is the predicate
that was there before.

---

## 7. Appendix — the final code of the verification path

Everything below is `rust/crates/swf-cli/src/main.rs` as it stands, extracted verbatim: the types a
`CacheClear` can come from, the transcript parser, the single-read digest, the verifier policy (the
relocation source, the empty-cache hit floor and the distribution contract), the corroboration function,
the cache-chain and receipt validators, and the three paragraphs the tool prints.
This is the whole of what a reader needs to audit the claim in section 2.

```rust
/// Accepted cache-clear transcript headers. v2 added `tool=` and `command=`
/// because a v1 transcript recorded only the post-shift `argv`, so it could not
/// say which tool had run — it could not evidence what it claimed to evidence.
const CACHE_CLEAR_MARKERS: [&str; 2] = ["# swf-cache-clear v1", "# swf-cache-clear v2"];

/// The cache namespace a clearing operation acted on, as reported by the
/// Incredibuild cache-management tool. There is deliberately no `Default` and
/// no inference: a transcript whose argv swf-cli does not recognize yields
/// `Unknown`, and `Unknown` never validates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CacheScope {
    LocalUser,
    Shared,
    Unknown,
}

/// One cache-clearing operation the runner performed, transcribed from the
/// cache tool's own output and exit status. swf-cli only ever parses these; it
/// has no code path that constructs one.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CacheClear {
    scope: CacheScope,
    argv: String,
    exit_code: i32,
    started_at_ms: u64,
    completed_at_ms: u64,
    transcript_path: String,
    transcript_sha256: String,
}

/// Where proof time looks for the transcripts a receipt names.
///
/// There is deliberately no variant meaning "do not look". A receipt is a
/// claim; the transcripts are the only thing in reach that can corroborate it;
/// a receipt whose transcripts cannot be opened is refused, never summarized.
/// `--transcripts` exists because a receipt is legitimately checked on a
/// machine other than the one that produced it, and `ib-benchmark.sh` records
/// host-absolute paths. It MOVES the search. It does not waive it.
#[derive(Debug, Clone)]
struct TranscriptSource {
    relocate: Option<PathBuf>,
}

impl TranscriptSource {
    /// Resolve every transcript at the path the receipt recorded.
    fn as_recorded() -> Self {
        Self { relocate: None }
    }

    /// Resolve every transcript by its recorded BASENAME inside `dir`.
    fn relocated(dir: PathBuf) -> Self {
        Self {
            relocate: Some(dir),
        }
    }

    fn describe(&self) -> String {
        match &self.relocate {
            None => "the paths recorded in the receipt".to_string(),
            Some(dir) => dir.display().to_string(),
        }
    }

    fn resolve(&self, recorded: &str) -> Result<PathBuf> {
        if recorded.trim().is_empty() {
            bail!("the receipt records no transcript path");
        }
        let recorded_path = Path::new(recorded);
        match &self.relocate {
            None => {
                if !recorded_path.is_file() {
                    bail!(
                        "transcript {recorded} is not present at its recorded path; if the \
                         retained transcripts were moved, or this receipt was produced on \
                         another host, pass --transcripts <dir>"
                    );
                }
                Ok(recorded_path.to_path_buf())
            }
            Some(dir) => {
                // Only the final component is honoured. `Path::file_name` never
                // yields "." or "..", so the join cannot leave `dir`:
                // relocation narrows the search and can never widen it into a
                // path the receipt chose. Without that, a receipt naming
                // "../somewhere/mine.txt" would pick its own evidence.
                let base = recorded_path.file_name().with_context(|| {
                    format!("transcript path {recorded:?} has no file name to relocate")
                })?;
                let candidate = dir.join(base);
                if !candidate.is_file() {
                    bail!(
                        "transcript {} (recorded as {recorded}) is not in {}; relocation moves \
                         the search, it does not waive it",
                        base.to_string_lossy(),
                        dir.display()
                    );
                }
                Ok(candidate)
            }
        }
    }
}

/// What a cache-clear transcript itself says, read out of the file before any
/// receipt is consulted. `CacheClear` is this plus the identity of the file it
/// came from (path and digest). Keeping them as two types is what makes "the
/// receipt disagrees with its own evidence" expressible at all: with one type
/// the comparison would be a field against itself, which is the schema-v1
/// tautology rebuilt one layer down.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ClearTranscript {
    /// 1 or 2: which entry of `CACHE_CLEAR_MARKERS` the file opened with.
    marker_version: u32,
    /// The tool the runner actually invoked. A v1 transcript recorded only the
    /// post-shift argv and so cannot answer this: `None`.
    tool: Option<String>,
    /// `tool` followed by `argv`, as the runner spelled it. v2 only.
    command: Option<String>,
    argv: String,
    scope: CacheScope,
    exit_code: i32,
    started_at_ms: u64,
    completed_at_ms: u64,
}

/// One transcript that was opened, re-hashed and re-parsed in this invocation.
/// Nothing in here was read from a receipt.
#[derive(Debug, Clone)]
struct Corroborated {
    digest: String,
    transcript: ClearTranscript,
}

/// Per-invocation record of what has actually been read off the filesystem, so
/// a transcript cited by several samples is hashed once and so the printed
/// counts describe files rather than JSON fields.
#[derive(Debug, Default)]
struct TranscriptLedger {
    /// Resolved path -> what that file actually says.
    read: BTreeMap<PathBuf, Corroborated>,
    /// Basename -> the single recorded path it may stand for, under relocation.
    basenames: BTreeMap<String, String>,
}

/// Derived, never asserted: the cache namespace follows from the arguments the
/// runner actually passed to the cache tool. Lifted out of the parser so proof
/// time can re-derive it from the transcript instead of believing the receipt.
fn scope_of(argv: &str) -> CacheScope {
    match argv.split_whitespace().next() {
        Some("user") => CacheScope::LocalUser,
        Some("shared") | Some("service") | Some("all") | Some("global") => CacheScope::Shared,
        _ => CacheScope::Unknown,
    }
}

/// Read one `key=value` header out of a cache-clear transcript, refusing a
/// missing or repeated key rather than taking the first or the last.
fn clear_header<'a>(headers: &'a BTreeMap<String, Vec<&'a str>>, key: &str) -> Result<&'a str> {
    match headers.get(key).map(Vec::as_slice) {
        Some([only]) => Ok(only),
        Some(many) => bail!("cache-clear transcript repeats {key} {} times", many.len()),
        None => bail!("cache-clear transcript has no {key}"),
    }
}

/// Parse a transcript emitted by scripts/robot-demo/cache-clear.sh into what
/// the FILE says, with no receipt in sight.
///
/// Every field is read out of the text; nothing is defaulted and nothing is
/// inferred. A scope swf-cli does not recognize becomes `CacheScope::Unknown`,
/// which the validator rejects, so an unrecognized cache tool fails the proof
/// instead of silently passing it.
///
/// A non-zero exit is REPORTED here rather than rejected, so that proof time
/// can say "the receipt claims 0, the transcript records 3" instead of losing
/// that disagreement inside a parse error. `parse_cache_clear` below keeps the
/// rejection, so the recording path is unchanged.
fn parse_clear_transcript(text: &str) -> Result<ClearTranscript> {
    let (header_block, _) = text
        .split_once("\n--- transcript ---")
        .context("cache-clear transcript has no '--- transcript ---' separator")?;
    let mut lines = header_block.lines();
    let header = lines.next().map(str::trim_end);
    let marker_version = CACHE_CLEAR_MARKERS
        .iter()
        .position(|marker| Some(*marker) == header)
        .map(|index| index as u32 + 1)
        .with_context(|| {
            format!(
                "cache-clear transcript does not start with one of {CACHE_CLEAR_MARKERS:?}; got {header:?}"
            )
        })?;
    let mut headers: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("cache-clear transcript has a non-header line {line:?}"))?;
        headers
            .entry(key.trim().to_string())
            .or_default()
            .push(value.trim());
    }

    let argv = clear_header(&headers, "argv")?.to_string();
    let exit_code: i32 = clear_header(&headers, "exit_code")?
        .parse()
        .context("cache-clear exit_code is not an integer")?;
    let started_at_ms: u64 = clear_header(&headers, "started_at_ms")?
        .parse()
        .context("cache-clear started_at_ms is not an unsigned integer")?;
    let completed_at_ms: u64 = clear_header(&headers, "completed_at_ms")?
        .parse()
        .context("cache-clear completed_at_ms is not an unsigned integer")?;
    if completed_at_ms < started_at_ms || started_at_ms == 0 {
        bail!("cache clear {argv:?} has a nonsensical time range");
    }

    // v2 records which tool acted on the cache, because v1's `argv` was written
    // after the tool name had been shifted off and so could not say what ran. A
    // file carrying the v1 marker AND a tool= line could not have been written
    // by the v1 recorder, so it was edited afterwards and is not a transcript
    // of anything.
    let (tool, command) = if marker_version == 1 {
        for key in ["tool", "command"] {
            if headers.contains_key(key) {
                bail!(
                    "a v1 cache-clear transcript carries a {key} line that the v1 recorder \
                     could not have written"
                );
            }
        }
        (None, None)
    } else {
        let tool = clear_header(&headers, "tool")?.to_string();
        let command = clear_header(&headers, "command")?.to_string();
        if tool.is_empty() {
            bail!("cache-clear transcript names no tool");
        }
        let expected = format!("{tool} {argv}");
        if command.trim() != expected.trim() {
            bail!(
                "cache-clear transcript is internally inconsistent: command {command:?} is not \
                 tool {tool:?} followed by argv {argv:?}"
            );
        }
        (Some(tool), Some(command))
    };

    Ok(ClearTranscript {
        marker_version,
        tool,
        command,
        scope: scope_of(&argv),
        argv,
        exit_code,
        started_at_ms,
        completed_at_ms,
    })
}

/// The only way a `CacheClear` can come into existence: a transcript parsed out
/// of a file, plus the identity (path and digest) of the file it was parsed
/// from. swf-cli has no code path that constructs one from literals.
fn parse_cache_clear(text: &str, path: &Path, sha256: String) -> Result<CacheClear> {
    let parsed = parse_clear_transcript(text)?;
    if parsed.exit_code != 0 {
        bail!(
            "cache clear {:?} exited {}; the cache state after it is unknown \
             and swf-cli will not record it as cleared",
            parsed.argv,
            parsed.exit_code
        );
    }
    Ok(CacheClear {
        scope: parsed.scope,
        argv: parsed.argv,
        exit_code: parsed.exit_code,
        started_at_ms: parsed.started_at_ms,
        completed_at_ms: parsed.completed_at_ms,
        transcript_path: path.display().to_string(),
        transcript_sha256: sha256,
    })
}

/// Read a transcript ONCE and hash the bytes that were read.
///
/// `evidence::sha256_file` opens the file a second time, so the bytes it
/// hashes are not provably the bytes that are then parsed: a file can change
/// between the two opens, and a digest over content nobody parsed corroborates
/// nothing. Reading once and hashing the buffer removes the window entirely,
/// on the recording path and on the proof path alike.
///
/// This is byte-for-byte the digest `evidence::sha256_file` computes for the
/// same content -- `the_digest_is_computed_over_the_bytes_that_are_parsed`
/// pins that against a real file, so the two can never drift apart and a
/// receipt recorded by one remains checkable by the other.
fn read_and_digest(path: &Path) -> Result<(String, String)> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let digest = hex::encode(Sha256::digest(&bytes));
    let text = String::from_utf8(bytes)
        .with_context(|| format!("{} is not UTF-8 text", path.display()))?;
    Ok((digest, text))
}

fn load_cache_clear(path: &Path) -> Result<CacheClear> {
    let (sha256, text) = read_and_digest(path)
        .with_context(|| format!("hashing cache-clear transcript {}", path.display()))?;
    parse_cache_clear(&text, path, sha256)
        .with_context(|| format!("in cache-clear transcript {}", path.display()))
}

/// What the verifier requires of DISTRIBUTION, as opposed to caching.
///
/// Incredibuild's profile schema makes these independent knobs on one
/// declaration: `/opt/incredibuild/data/ib_profile.xsd` gives `type` the
/// enumeration {intercepted, static_intercepted, static_intercepted_fileops,
/// allow_remote, local_only} and makes `<ib_cache enabled="..."/>` a separate
/// child element of the same `<process>`. rustc can therefore be cached
/// without ever being distributed, and that configuration needs a contract of
/// its own.
///
/// Both variants are falsifiable demands, in opposite directions. There is
/// deliberately no variant meaning "do not look at the remote counters":
/// switching to `Excluded` to excuse a run with no remote tasks immediately
/// makes any remote task a rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Distribution {
    /// rustc declared `allow_remote`: work must actually have crossed the
    /// network. remote_tasks > 0 and remote_core_time > 0.
    Required,
    /// rustc declared `local_only`: nothing may have crossed the network, so
    /// whatever acceleration was measured is the Build Cache's alone.
    /// remote_tasks == 0 and remote_core_time == 0.
    Excluded,
}

impl Distribution {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "required" => Ok(Self::Required),
            "excluded" => Ok(Self::Excluded),
            other => bail!("unknown --distribution {other}; use required or excluded"),
        }
    }
}

impl std::fmt::Display for Distribution {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Required => "required",
            Self::Excluded => "excluded",
        })
    }
}

/// What the VERIFIER asks of a receipt, as opposed to what the receipt says
/// about itself. Every field here is chosen on the command line by the person
/// doing the checking, and nothing in it can be set by the party that wrote
/// the receipt. That asymmetry is the whole point: a receipt may not choose
/// the standard it is judged by. There is deliberately no field here that
/// means "do not check".
#[derive(Debug)]
struct ProofPolicy {
    /// Samples required per mode, each with a distinct repetition number.
    min_samples: usize,
    /// Where the cache-clear transcripts are looked for. Moving the search is
    /// the only thing this can do; it can never waive it.
    transcripts: TranscriptSource,
    /// How many Build Cache hits a build on an EMPTIED cache may report and
    /// still count as empty-cache -- and, symmetrically, how many a warm build
    /// must EXCEED. It therefore tightens the warm test by exactly as much as
    /// it loosens the cold one, so it cannot be used as a pure loosener.
    ///
    /// The default is 0, which is the strict rule: an emptied cache served
    /// nothing. It exists because cargo invokes `rustc -vV` twice in one
    /// build and the second invocation is served the entry the first one
    /// stored, so a genuine empty-cache Rust build under Incredibuild reports
    /// exactly one intra-build self-hit (see `empty_cache_hit_floor` in
    /// scripts/robot-demo/ib-benchmark.sh). Without this the validator would
    /// refuse every honest Rust receipt, which is a false refusal, not rigour.
    ///
    /// NOTHING HERE ESTABLISHES that a hit allowed by this floor was that
    /// self-hit. The per-task Build Cache report the runner already writes
    /// does say so, and is not retained with a digest; until it is, a non-zero
    /// floor is the verifier's own judgement and `print_build_proof` prints it
    /// as such.
    empty_cache_hit_floor: u64,
    /// Whether this receipt is being judged as a DISTRIBUTION experiment or a
    /// CACHE-ONLY one. Each setting is a demand the other would reject, so
    /// this cannot be used to excuse a run: see `Distribution`.
    distribution: Distribution,
}

impl ProofPolicy {
    /// The strict standard: transcripts resolved as told, an emptied cache
    /// that served anything is not empty, and every IB build distributed.
    fn checking(min_samples: usize, transcripts: TranscriptSource) -> Self {
        Self {
            min_samples,
            transcripts,
            empty_cache_hit_floor: 0,
            distribution: Distribution::Required,
        }
    }

    fn with_empty_cache_hit_floor(mut self, floor: u64) -> Self {
        self.empty_cache_hit_floor = floor;
        self
    }

    fn with_distribution(mut self, distribution: Distribution) -> Self {
        self.distribution = distribution;
        self
    }
}

/// A cache-using build in this receipt, used to prove that nothing ran between
/// a cache clear and the build it is claimed to have prepared.
#[derive(Debug)]
struct CacheBuild {
    started_at_ms: u64,
    label: String,
}

/// What the receipt's own records support about cache handling, derived rather
/// than asserted. `print_build_proof` reports these instead of echoing a flag.
#[derive(Debug)]
struct CacheEvidence {
    scope: CacheScope,
    /// DISTINCT FILES opened, re-hashed and re-parsed in this invocation:
    /// `TranscriptLedger::read.len()`, taken after the walk rather than
    /// counted as the receipt is walked.
    ///
    /// The previous field incremented once per `CacheClear` JSON object, so
    /// it counted citations. A receipt chooses how many times it cites a
    /// transcript; it does not choose how many files exist. Citing one
    /// genuine transcript twenty times printed "20 corroborated clear
    /// transcript(s)" over as few as one file -- the printed number was back
    /// to being a restatement of the receipt, which is the defect this whole
    /// layer exists to kill.
    transcripts: usize,
    /// How many cache-clear records in the receipt cited those files. Printed
    /// BESIDE `transcripts`, never instead of it, so that a receipt citing
    /// one transcript twenty times is visible as exactly that.
    citations: usize,
    /// Parent seeds the receipt attributes its warm cache to. Receipt-asserted:
    /// a seed carries counters and a start time, and no document behind them is
    /// retained or re-read. Printed under its own label for that reason.
    warm_seeds: usize,
    /// Distinct cache tools named by those transcripts, read off disk. Printed
    /// so that a transcript recording `tool=/bin/true` is visible rather than
    /// parsed and discarded.
    tools: BTreeSet<String>,
    /// Distinct files among `transcripts` written by the v1 recorder, which
    /// shifted the tool name off before recording argv and so cannot say what
    /// ran. Ledger-derived, for the same reason as `transcripts`.
    anonymous_transcripts: usize,
    /// Where those transcripts were resolved from in this invocation.
    source: String,
}

/// The latest clear that finished at or before `before_ms`.
fn effective_clear(clears: &[CacheClear], before_ms: u64) -> Option<&CacheClear> {
    clears
        .iter()
        .filter(|clear| clear.completed_at_ms <= before_ms)
        .max_by_key(|clear| clear.completed_at_ms)
}

/// Any cache-using build that started strictly between the two instants. One of
/// these means the clear cannot be attributed to the build that follows it.
fn intervening_build(builds: &[CacheBuild], after_ms: u64, before_ms: u64) -> Option<&CacheBuild> {
    builds
        .iter()
        .find(|build| build.started_at_ms > after_ms && build.started_at_ms < before_ms)
}

fn check_clear_usable(clear: &CacheClear, what: &str) -> Result<()> {
    if clear.exit_code != 0 {
        bail!(
            "{what}: cache clear {:?} exited {}",
            clear.argv,
            clear.exit_code
        );
    }
    match clear.scope {
        CacheScope::LocalUser => {}
        CacheScope::Shared => bail!(
            "{what}: cache clear {:?} acted on a shared cache; this benchmark may only \
             clear the invoking user's own cache",
            clear.argv
        ),
        CacheScope::Unknown => bail!(
            "{what}: cache clear {:?} has an unrecognized scope, so the cache namespace \
             it emptied is unknown",
            clear.argv
        ),
    }
    if clear.transcript_sha256.len() != 64
        || !clear
            .transcript_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        bail!(
            "{what}: cache clear {:?} has no retained transcript digest",
            clear.argv
        );
    }
    if clear.transcript_path.is_empty() {
        bail!(
            "{what}: cache clear {:?} has no retained transcript",
            clear.argv
        );
    }
    Ok(())
}

/// Open the transcript this clear names, recompute its digest, re-parse it, and
/// refuse unless the FILE corroborates every fact the RECEIPT asserts about it.
///
/// This is the function whose absence let a receipt citing
/// "/tmp/does-not-exist.txt" with a digest of sixty-four zeros print a measured
/// ratio and exit 0. `check_clear_usable` above asks whether the receipt's own
/// fields are well shaped, which is a question about a string; every answer it
/// can give is one the writer of the receipt chose. This asks whether there is
/// a file on disk that says the same thing.
///
/// Returns the path the transcript was actually read from -- so the caller
/// counts files rather than JSON objects, and can see one file cited twice --
/// and the cache tool the transcript names, or `None` for a v1 transcript,
/// whose recorder shifted the tool name off before writing `argv` and therefore
/// cannot say which tool acted on the cache. Cross-checking a v1 transcript's
/// argv against the receipt's argv is still worth doing -- it catches a
/// fabricated timeline -- but it cannot establish what ran, and the summary
/// says so rather than implying otherwise.
fn corroborate_clear(
    clear: &CacheClear,
    what: &str,
    source: &TranscriptSource,
    ledger: &mut TranscriptLedger,
) -> Result<(PathBuf, Option<String>)> {
    let subject = format!("{what}: cache clear {:?}", clear.argv);

    // Under relocation a basename stands for exactly one recorded path. Two
    // different recorded paths collapsing onto one file is an ambiguity about
    // which file this clear refers to, and an ambiguity is a refusal.
    if source.relocate.is_some() {
        let base = Path::new(&clear.transcript_path)
            .file_name()
            .map(|base| base.to_string_lossy().into_owned())
            .with_context(|| {
                format!(
                    "{subject}: transcript path {:?} has no file name",
                    clear.transcript_path
                )
            })?;
        match ledger.basenames.get(&base) {
            Some(first) if *first != clear.transcript_path => bail!(
                "{subject}: transcripts {:?} and {first:?} both relocate to {base:?}; which file \
                 this clear refers to cannot be established",
                clear.transcript_path
            ),
            _ => {
                ledger.basenames.insert(base, clear.transcript_path.clone());
            }
        }
    }

    let resolved = source
        .resolve(&clear.transcript_path)
        .with_context(|| subject.clone())?;

    let found = match ledger.read.get(&resolved) {
        Some(found) => found.clone(),
        None => {
            let (digest, text) = read_and_digest(&resolved)
                .with_context(|| format!("{subject}: opening transcript {}", resolved.display()))?;
            let transcript = parse_clear_transcript(&text).with_context(|| {
                format!("{subject}: re-parsing transcript {}", resolved.display())
            })?;
            let fresh = Corroborated { digest, transcript };
            ledger.read.insert(resolved.clone(), fresh.clone());
            fresh
        }
    };

    // The digest binds the receipt to THESE bytes -- the very bytes parsed
    // just above, hashed in the same read. A receipt that matches some other
    // file, or some other version of this file, is not evidence about the one
    // that was read.
    if found.digest != clear.transcript_sha256 {
        bail!(
            "{subject}: transcript {} hashes to {}, but the receipt records {}; this receipt \
             does not describe this file",
            resolved.display(),
            found.digest,
            clear.transcript_sha256
        );
    }

    // The receipt is a claim about this file; the file is the evidence. Where
    // they disagree the receipt is wrong. Every disagreement is collected, so
    // the operator sees the shape of the discrepancy and not only its first
    // field -- one fabricated transcript backing ten contradictory citations
    // looks very different from one mistyped timestamp.
    let actual = &found.transcript;
    let mut disagreements: Vec<String> = Vec::new();
    if actual.exit_code != clear.exit_code {
        disagreements.push(format!(
            "exit_code (receipt {}, transcript {})",
            clear.exit_code, actual.exit_code
        ));
    }
    if actual.started_at_ms != clear.started_at_ms {
        disagreements.push(format!(
            "started_at_ms (receipt {}, transcript {})",
            clear.started_at_ms, actual.started_at_ms
        ));
    }
    if actual.completed_at_ms != clear.completed_at_ms {
        disagreements.push(format!(
            "completed_at_ms (receipt {}, transcript {})",
            clear.completed_at_ms, actual.completed_at_ms
        ));
    }
    if actual.argv != clear.argv {
        disagreements.push(format!(
            "argv (receipt {:?}, transcript {:?})",
            clear.argv, actual.argv
        ));
    }
    if actual.scope != clear.scope {
        disagreements.push(format!(
            "scope (receipt {}, re-derived from the transcript's own arguments {})",
            clear.scope, actual.scope
        ));
    }
    if !disagreements.is_empty() {
        bail!(
            "{subject}: the receipt disagrees with transcript {}: {}",
            resolved.display(),
            disagreements.join("; ")
        );
    }

    // Redundant with check_clear_usable's reading of the receipt's own field,
    // but that field has now been shown equal to the file's, so this one is a
    // statement about the file.
    if actual.exit_code != 0 {
        bail!(
            "{subject}: transcript {} records exit {}",
            resolved.display(),
            actual.exit_code
        );
    }

    Ok((resolved, actual.tool.clone()))
}

/// Verify that every Incredibuild sample ran against the cache state it claims.
///
/// Nothing here reads a field that swf-cli wrote from a literal: the clears are
/// transcribed from the cache tool, the seed counters come from Incredibuild's
/// own statistics, and the ordering is checked against timestamps the runner
/// took around each build. Everything the ordering rests on is additionally
/// corroborated against the transcript file itself, because a timestamp only a
/// receipt asserts is a timestamp its writer chose.
fn verify_cache_chain(proof: &BuildProof, policy: &ProofPolicy) -> Result<CacheEvidence> {
    let source = &policy.transcripts;
    let mut builds: Vec<CacheBuild> = Vec::new();
    for sample in &proof.samples {
        if sample.mode == "native" {
            continue;
        }
        builds.push(CacheBuild {
            started_at_ms: sample.started_at_ms,
            label: format!("{} sample {}", sample.mode, sample.repetition),
        });
        if let Some(seed) = &sample.parent_seed {
            builds.push(CacheBuild {
                started_at_ms: seed.started_at_ms,
                label: format!("parent seed {}", seed.build_caption),
            });
        }
    }
    let mut seen: BTreeMap<u64, &str> = BTreeMap::new();
    for build in &builds {
        if let Some(other) = seen.insert(build.started_at_ms, &build.label) {
            bail!(
                "{} and {} report the same start time {}ms; cache ordering cannot be \
                 established",
                other,
                build.label,
                build.started_at_ms
            );
        }
    }

    let mut scope: Option<CacheScope> = None;
    let mut citations = 0usize;
    let mut warm_seeds = 0usize;
    let mut ledger = TranscriptLedger::default();
    let mut tools: BTreeSet<String> = BTreeSet::new();

    for sample in &proof.samples {
        if sample.mode == "native" {
            continue;
        }
        let what = format!("{} sample {}", sample.mode, sample.repetition);
        if sample.cache_clears.is_empty() {
            bail!("{what} records no cache clear, so the cache it built against is unknown");
        }
        let mut cited: BTreeSet<PathBuf> = BTreeSet::new();
        for clear in &sample.cache_clears {
            citations += 1;
            // Every clear, not only the one `effective_clear` selects below.
            // A fabricated clear that finishes LATER shadows a real one and
            // would be caught as the effective clear; a fabricated EARLIER one
            // would sit in the receipt uncontested while the summary counted
            // it. A printed count must be a count of corroborated things.
            check_clear_usable(clear, &what)?;
            let (resolved, tool) = corroborate_clear(clear, &what, source, &mut ledger)?;
            // One sample citing one file twice says nothing twice. It is
            // either a bug in the runner or an attempt to make the evidence
            // look deeper than it is, and neither is something to summarize.
            // The counts printed below are taken from the ledger and so are
            // already immune to it; this refuses the receipt outright.
            if !cited.insert(resolved.clone()) {
                bail!(
                    "{what} cites transcript {} more than once; one file cited twice is one \
                     piece of evidence, not two",
                    resolved.display()
                );
            }
            if let Some(tool) = tool {
                tools.insert(tool);
            }
            match scope {
                None => scope = Some(clear.scope),
                Some(previous) if previous != clear.scope => bail!(
                    "{what}: cache clears disagree about scope ({previous} vs {})",
                    clear.scope
                ),
                Some(_) => {}
            }
        }

        // The instant the cache had to be empty: for a cold sample that is its
        // own build; for a warm sample it is the parent seed that warms it.
        let (target_ms, target_label) = match (&sample.mode[..], &sample.parent_seed) {
            ("ib-cold", _) => (sample.started_at_ms, what.clone()),
            ("ib-parent-warm", Some(seed)) => {
                if seed.source_revision != proof.parent_revision {
                    bail!(
                        "{what}: parent seed built {}, expected parent {}",
                        seed.source_revision,
                        proof.parent_revision
                    );
                }
                if seed.started_at_ms >= sample.started_at_ms {
                    bail!("{what}: parent seed did not start before the measured build");
                }
                match policy.distribution {
                    Distribution::Required if seed.remote_tasks == 0 => {
                        bail!("{what}: parent seed has no verified remote tasks");
                    }
                    Distribution::Excluded if seed.remote_tasks != 0 => {
                        bail!(
                            "{what}: parent seed reported {} remote task(s) under \
                             --distribution excluded; a cache-only profile declares rustc \
                             local_only, so nothing may have been sent to a helper",
                            seed.remote_tasks
                        );
                    }
                    _ => {}
                }
                if seed.cache_hits > policy.empty_cache_hit_floor {
                    bail!(
                        "{what}: parent seed reported {} cache hit(s) against a floor of {}, \
                         so it did not run against an emptied cache",
                        seed.cache_hits,
                        policy.empty_cache_hit_floor
                    );
                }
                if seed.cache_misses == 0 {
                    bail!("{what}: parent seed populated no cache entries");
                }
                if let Some(between) =
                    intervening_build(&builds, seed.started_at_ms, sample.started_at_ms)
                {
                    bail!(
                        "{what}: {} ran between the parent seed and the measured build, so \
                         the warm cache cannot be attributed to the parent",
                        between.label
                    );
                }
                warm_seeds += 1;
                (seed.started_at_ms, format!("parent seed for {what}"))
            }
            ("ib-parent-warm", None) => bail!(
                "{what} records no parent seed, so its cache hits cannot be attributed to \
                 the parent revision"
            ),
            (other, _) => bail!("unknown benchmark mode {other}"),
        };

        let clear = effective_clear(&sample.cache_clears, target_ms).with_context(|| {
            format!("{target_label}: every recorded cache clear finished after the build started")
        })?;
        check_clear_usable(clear, &target_label)?;
        if let Some(between) = intervening_build(&builds, clear.completed_at_ms, target_ms) {
            bail!(
                "{target_label}: {} ran between the cache clear and this build, so the clear \
                 cannot be attributed to it",
                between.label
            );
        }
    }

    let scope = scope.unwrap_or(CacheScope::Unknown);
    if scope != CacheScope::LocalUser {
        bail!("expected isolated local-user cache scope, observed {scope}");
    }
    // Both counts are taken from the ledger, which holds one entry per FILE
    // actually opened in this invocation, so neither can be inflated by a
    // receipt that repeats itself.
    let anonymous_transcripts = ledger
        .read
        .values()
        .filter(|found| found.transcript.marker_version == 1)
        .count();
    Ok(CacheEvidence {
        scope,
        transcripts: ledger.read.len(),
        citations,
        warm_seeds,
        tools,
        anonymous_transcripts,
        source: source.describe(),
    })
}

fn validate_build_proof(proof: &BuildProof, policy: &ProofPolicy) -> Result<ProofSummary> {
    if proof.schema_version != BUILD_PROOF_SCHEMA_VERSION {
        if proof.schema_version == 1 {
            bail!(
                "build-proof schema 1 asserted its cache scope and cache-clearing procedure as \
                 literals that nothing observed; re-run scripts/robot-demo/ib-benchmark.sh to \
                 produce a schema {BUILD_PROOF_SCHEMA_VERSION} receipt"
            );
        }
        bail!("unsupported build-proof schema {}", proof.schema_version);
    }
    parse_identifier(&proof.run_id).map_err(anyhow::Error::msg)?;
    if proof.candidate_revision.is_empty() || proof.parent_revision.is_empty() {
        bail!("candidate_revision and parent_revision are required");
    }
    if proof.candidate_revision == proof.parent_revision {
        bail!("candidate_revision and parent_revision must differ");
    }

    let mut by_mode: BTreeMap<String, Vec<&BuildSample>> = BTreeMap::new();
    for sample in &proof.samples {
        if !["native", "ib-cold", "ib-parent-warm"].contains(&sample.mode.as_str()) {
            bail!("unknown benchmark mode {}", sample.mode);
        }
        if sample.wall_ms == 0 || sample.build_caption.is_empty() {
            bail!(
                "sample {} / {} has no wall time or caption",
                sample.mode,
                sample.repetition
            );
        }
        if sample.started_at_ms == 0 {
            bail!(
                "sample {} / {} has no recorded start time",
                sample.mode,
                sample.repetition
            );
        }
        if sample.source_revision != proof.candidate_revision {
            bail!(
                "sample {} / {} built {}, expected candidate {}",
                sample.mode,
                sample.repetition,
                sample.source_revision,
                proof.candidate_revision
            );
        }
        if sample.mode == "native" {
            if sample.remote_tasks.is_some()
                || sample.remote_core_time_s.is_some()
                || sample.cache_hits.is_some()
                || sample.parent_seed.is_some()
                || !sample.cache_clears.is_empty()
            {
                bail!(
                    "native sample {} contains IB-only telemetry",
                    sample.repetition
                );
            }
        } else {
            // Presence is demanded before either contract is applied, so that
            // `excluded` can never be satisfied by a counter that is simply
            // MISSING. An absent remote_tasks is unknown, not zero.
            let (Some(remote_tasks), Some(remote_core_time_s)) =
                (sample.remote_tasks, sample.remote_core_time_s)
            else {
                bail!(
                    "{} sample {} has no recorded remote-task counters",
                    sample.mode,
                    sample.repetition
                );
            };
            if sample.local_tasks.is_none() || sample.cache_misses.is_none() {
                bail!(
                    "{} sample {} has incomplete IB telemetry",
                    sample.mode,
                    sample.repetition
                );
            }
            match policy.distribution {
                Distribution::Required => {
                    if remote_tasks == 0 {
                        bail!(
                            "{} sample {} has no verified remote tasks",
                            sample.mode,
                            sample.repetition
                        );
                    }
                    if remote_core_time_s <= 0.0 {
                        bail!(
                            "{} sample {} has no verified remote core time",
                            sample.mode,
                            sample.repetition
                        );
                    }
                }
                // The cache-only contract. Stated as an equality against the
                // counters Incredibuild's own Build History reported, so a
                // cache-only claim is refused the moment anything reached a
                // helper -- including one stray task that would otherwise sit
                // unnoticed inside a "cache" result.
                Distribution::Excluded => {
                    if remote_tasks != 0 {
                        bail!(
                            "{} sample {} reported {} remote task(s) under --distribution \
                             excluded; a cache-only run distributes nothing",
                            sample.mode,
                            sample.repetition,
                            remote_tasks
                        );
                    }
                    if remote_core_time_s != 0.0 {
                        bail!(
                            "{} sample {} reported {}s of remote core time under \
                             --distribution excluded; a cache-only run distributes nothing",
                            sample.mode,
                            sample.repetition,
                            remote_core_time_s
                        );
                    }
                }
            }
        }
        // Cold and warm are judged against the same number, from opposite
        // sides: a cold build may not exceed the floor and a warm build must.
        // Raising the floor to excuse a cold sample therefore raises the bar
        // the warm samples have to clear, which is what stops it being a way
        // to make a receipt pass.
        if sample.mode == "ib-cold"
            && sample.cache_hits.unwrap_or(u64::MAX) > policy.empty_cache_hit_floor
        {
            bail!(
                "ib-cold sample {} was not empty-cache (hits={:?}, floor={})",
                sample.repetition,
                sample.cache_hits,
                policy.empty_cache_hit_floor
            );
        }
        if sample.mode == "ib-parent-warm"
            && sample.cache_hits.unwrap_or(0) <= policy.empty_cache_hit_floor
        {
            bail!(
                "ib-parent-warm sample {} has no verified cache hits above the empty-cache \
                 floor of {} (hits={:?}), so nothing distinguishes it from a cold build",
                sample.repetition,
                policy.empty_cache_hit_floor,
                sample.cache_hits
            );
        }
        by_mode.entry(sample.mode.clone()).or_default().push(sample);
    }

    // A build caption names ONE build to Incredibuild's Build History. Two
    // samples carrying one caption are therefore at most one build measured
    // twice, and the receipt does not say which measurement belongs to it.
    //
    // This is also the ONLY thing this invocation can establish about a
    // caption. `build-sample` checked, when the sample was recorded, that the
    // Build History held exactly one record for it reporting success; that
    // document is not retained, is not re-read here, and nothing printed below
    // claims otherwise.
    let mut captions: BTreeMap<&str, String> = BTreeMap::new();
    for sample in &proof.samples {
        let mut claimed: Vec<(&str, String)> = vec![(
            sample.build_caption.as_str(),
            format!("{} sample {}", sample.mode, sample.repetition),
        )];
        if let Some(seed) = &sample.parent_seed {
            claimed.push((
                seed.build_caption.as_str(),
                format!(
                    "the parent seed for {} sample {}",
                    sample.mode, sample.repetition
                ),
            ));
        }
        for (caption, owner) in claimed {
            if let Some(first) = captions.insert(caption, owner.clone()) {
                bail!(
                    "build caption {caption:?} is carried by both {first} and {owner}; one \
                     caption is one build, so these cannot both be measurements of their own"
                );
            }
        }
    }

    let mut stats = BTreeMap::new();
    for mode in ["native", "ib-cold", "ib-parent-warm"] {
        let samples = by_mode
            .get(mode)
            .with_context(|| format!("missing benchmark mode {mode}"))?;
        if samples.len() < policy.min_samples {
            bail!(
                "{mode} has {} sample(s), require at least {}",
                samples.len(),
                policy.min_samples
            );
        }
        let mut repetitions: Vec<usize> = samples.iter().map(|sample| sample.repetition).collect();
        repetitions.sort_unstable();
        repetitions.dedup();
        if repetitions.len() != samples.len() {
            bail!("{mode} contains duplicate repetition numbers");
        }
        stats.insert(mode.to_string(), mode_stats(samples));
    }
    let cache = verify_cache_chain(proof, policy)?;
    Ok(ProofSummary { stats, cache })
}

/// The three tiers `print_build_proof` prints, as functions returning the
/// text, so that the claims are values a test can read. The previous versions
/// existed only as literals inside `println!`, which is why two of them went
/// on asserting things the validator had never done: no test could see them.
///
/// Tier 1: what was RE-OBSERVED -- established in this invocation by opening a
/// file. Everything here is a statement about bytes on disk.
fn re_observed_paragraph(cache: &CacheEvidence) -> String {
    format!(
        "RE-OBSERVED IN THIS INVOCATION (transcripts resolved from {source}): {n} cache-clear \
         transcript file(s) were opened and read once, and the SHA-256 recomputed over exactly \
         the bytes that were then parsed matched the digest this receipt records, byte for \
         byte. Each file was re-parsed, and its argument vector, exit status, start and \
         completion instants, and the cache scope re-derived from its own arguments, all agree \
         with the fields this receipt carries -- for EVERY cache-clear record in the receipt, \
         not only the one the ordering selects. Where a transcript names its tool, its command \
         line is that tool followed by that argv. A receipt naming a file that is missing, that \
         has changed by one byte, or that contradicts it, is REFUSED here; it is not \
         summarized. This count is the number of FILES opened, taken from the ledger of reads, \
         not the number of records in the receipt.",
        source = cache.source,
        n = cache.transcripts
    )
}

/// Tier 2: what was CHECKED -- cross-field logic over numbers the receipt
/// asserts. Nothing in this tier opens a document, and nothing in it may
/// mention one. The Build History clause that used to live here was the
/// schema-v1 defect rebuilt in prose: a record-time check, performed on
/// another host by another invocation against a document this one cannot see,
/// printed under the word CHECKED. It is now in the tier below, where it is
/// true.
fn checked_paragraph(policy: &ProofPolicy) -> String {
    format!(
        "CHECKED FROM THIS RECEIPT'S OWN FIELDS: {min} sample(s) per mode carrying distinct \
         repetition numbers -- distinct is all that is shown, since the receipt's writer picks \
         the numbers; a distinct build caption for every sample and every parent seed; remote \
         task and remote core-time counters PRESENT for every IB sample (an absent counter is \
         unknown, never zero) and, under --distribution {distribution}, {distribution_rule}; \
         cold-cache and \
         parent-seed hits <={floor} and warm-cache hits >{floor} as recorded; for every ib-cold \
         sample, a local-user cache clear that exited 0 and completed before it with no \
         cache-using build in between; for every ib-parent-warm sample, that same ordering \
         between the clear and its PARENT SEED, and no cache-using build between that seed and \
         the measured build -- the seed itself ran between the clear and the measured build, \
         which is what warming a cache means. This is cross-field logic over numbers the \
         receipt asserts: no document is opened for any of it. The cache-clear fields the \
         ordering rests on are corroborated above; every counter it compares is not.",
        min = policy.min_samples,
        floor = policy.empty_cache_hit_floor,
        distribution = policy.distribution,
        distribution_rule = match policy.distribution {
            Distribution::Required =>
                "every IB sample and every parent seed reporting remote_tasks > 0 and \
                 remote_core_time > 0",
            Distribution::Excluded =>
                "every IB sample and every parent seed reporting remote_tasks == 0 and \
                 remote_core_time == 0, so nothing in this receipt was distributed",
        }
    )
}

/// Tier 3: what is NOT CHECKED. This is the tier that has to be complete,
/// because everything a reader is entitled to distrust has to be findable
/// here. It names the counters, the clock, the wall times and the signature
/// that is absent -- and the record-time checks that happened elsewhere.
fn not_checked_paragraph() -> String {
    "NOT CHECKED: the transcripts are corroborated; the builds are not. No document behind any \
     counter in this receipt is retained with a digest or re-read here -- not the Build History \
     responses behind remote_tasks and local_tasks, not the cache-statistics output behind \
     cache_hits and cache_misses, not the per-task Build Cache report, and not one of the \
     parent-seed counters, which is what the attribution of a warm cache to its parent rests \
     on. `build-sample` did check, WHEN EACH SAMPLE WAS RECORDED, that the Build History held \
     exactly one record for that caption reporting success; that happened in another \
     invocation, on another host, against a document this one cannot see, and this invocation \
     re-establishes none of it. Neither is wall_ms, from which every ratio above is computed: \
     nothing retains it and nothing cross-checks it. swf-cli does not observe the cache: a \
     transcript shows that the named tool ran and what it printed, not that the namespace was \
     empty, and nothing here rules out another process repopulating it between the clear and \
     the build. Every timestamp is the runner's own clock, unattested, and nothing binds a \
     transcript to the machine, user or filesystem that ran the build. Neither the receipt nor \
     the transcripts are signed, so a party able to write both can still make them agree -- \
     including the tool names printed above, which are read from those same transcripts. \
     Re-verify independently before quoting any ratio."
        .to_string()
}

fn print_build_proof(proof: &BuildProof, policy: &ProofPolicy) -> Result<()> {
    let ProofSummary { stats, cache } = validate_build_proof(proof, policy)?;
    println!("BUILD RECEIPT CONSISTENT  run={}", proof.run_id);
    println!(
        "candidate={} parent={}",
        proof.candidate_revision, proof.parent_revision
    );
    for mode in ["native", "ib-cold", "ib-parent-warm"] {
        let s = &stats[mode];
        println!(
            "{mode:<15} median={:>8.1}ms range={:>6}..{:>6}ms",
            s.median_ms, s.min_ms, s.max_ms
        );
    }
    let native = stats["native"].median_ms;
    for mode in ["ib-cold", "ib-parent-warm"] {
        let measured = stats[mode].median_ms;
        println!(
            "{mode:<15} measured ratio={:.3}x vs native; saved={:.0}ms",
            native / measured,
            native - measured
        );
    }
    // Printed on EVERY run, both settings, because "this was a cache-only
    // measurement" and "this was a distribution measurement" are different
    // claims and a reader must not have to infer which one a summary supports.
    match policy.distribution {
        Distribution::Required => println!(
            "DISTRIBUTION CONTRACT: --distribution required. Every IB sample and parent seed \
             in this receipt reports remote_tasks > 0 and remote_core_time > 0, so work \
             crossed the network. The wall times above therefore mix distribution and \
             caching, and this summary does NOT separate them."
        ),
        Distribution::Excluded => println!(
            "DISTRIBUTION CONTRACT: --distribution excluded (CACHE-ONLY). Every IB sample and \
             parent seed in this receipt reports remote_tasks == 0 and remote_core_time == 0 \
             as recorded, i.e. nothing was sent to a helper; the expected profile is rustc \
             type=\"local_only\" with <ib_cache enabled=\"true\"/>. Any acceleration above is \
             therefore the Build Cache's and not distribution's. This is a demand, not a \
             waiver: one remote task would have REFUSED this receipt. What is NOT established \
             here is that the profile actually said local_only -- these are the Build History \
             counters the receipt carries, and like every other counter in it they are \
             re-read from no retained document (see NOT CHECKED below)."
        ),
    }
    println!(
        "cache scope re-derived from {} corroborated clear transcript file(s): {}",
        cache.transcripts, cache.scope
    );
    // A receipt chooses how often it cites a file. Printing both numbers means
    // it can inflate the one it controls only by making the gap visible.
    if cache.citations != cache.transcripts {
        println!(
            "those {} file(s) are cited by {} cache-clear record(s) in this receipt",
            cache.transcripts, cache.citations
        );
    }
    println!(
        "cache tool(s) named by those transcripts: {}",
        if cache.tools.is_empty() {
            "none -- no transcript named the tool that ran".to_string()
        } else {
            cache.tools.iter().cloned().collect::<Vec<_>>().join(", ")
        }
    );
    // Deliberately on its own line and not in the corroboration sentence: a
    // parent seed is a set of counters and a start time that only the receipt
    // asserts, and standing next to the word "corroborated" was lending it
    // standing it has not got.
    println!(
        "{} parent-seed build(s) attributed to the parent revision ON THIS RECEIPT'S WORD \
         ALONE: no document behind a seed's counters is retained or re-read here",
        cache.warm_seeds
    );
    if cache.anonymous_transcripts > 0 {
        println!(
            "{} of those file(s) were written by the v1 recorder, which shifted the tool \
             name off before recording argv; WHICH TOOL emptied the cache is not established \
             for them. Re-run with scripts/robot-demo/cache-clear.sh, which emits {}.",
            cache.anonymous_transcripts, CACHE_CLEAR_MARKERS[1]
        );
    }
    if policy.empty_cache_hit_floor > 0 {
        println!(
            "EMPTY-CACHE HIT FLOOR: this check ran with --empty-cache-hit-floor {floor}, which \
             the VERIFIER passed on the command line and the receipt cannot set. An emptied \
             cache was allowed to report up to {floor} Build Cache hit(s), and a warm build was \
             required to report more than {floor}. cargo invokes `rustc -vV` twice per build \
             and the second invocation is served the entry the first stored, which is why the \
             honest floor for Rust is not zero. NOTHING HERE ESTABLISHES that the hits this \
             floor allowed were that self-hit: the per-task Build Cache report that would show \
             it is not retained with a digest. Run without the flag for the strict rule.",
            floor = policy.empty_cache_hit_floor
        );
    }
    println!("{}", re_observed_paragraph(&cache));
    println!("{}", checked_paragraph(policy));
    println!("{}", not_checked_paragraph());
    Ok(())
}
```
