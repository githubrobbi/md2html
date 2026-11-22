# CommonMark conformance examples

`spec-0.31.2.json` is the example set of the CommonMark Spec 0.31.2
(https://spec.commonmark.org/0.31.2/spec.json), by John MacFarlane,
licensed under CC BY-SA 4.0. It is vendored unchanged so the conformance
test (`src/spec_tests.rs`) runs offline.

`cargo test spec -- --nocapture` prints the pass count per section. The
test fails when the count falls below the baseline recorded in the test,
so a change can only keep or improve conformance.
