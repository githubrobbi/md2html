# CommonMark conformance examples

`spec-0.31.2.json` is the example set of the CommonMark Spec 0.31.2
(https://spec.commonmark.org/0.31.2/spec.json), by John MacFarlane,
licensed under CC BY-SA 4.0. It is vendored unchanged so the conformance
test (`src/spec_tests.rs`) runs offline.

The test fails when fewer examples pass than `BASELINE` in it (652, every
example), naming the ones that fail, so a change can only keep or improve
conformance. `SPEC_FAILURES=1 cargo test -p md2html commonmark_spec` asks
for every example to pass and shows each one that does not: its section,
its Markdown, the HTML it should make and the HTML md2html makes.
`docs/TESTING.md` places this test among the others.
