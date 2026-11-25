<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# md2html's documentation

The pages of this folder, in the order to read them. This page is the start page of a handbook as md2html reads one: `md2html docs` renders the folder as one book, every page with the others' contents in its outline and the pages before and after it at its foot, and `md2html --check docs` checks it (CI does, on every change). The project's own front page is [the README](../README.md); what md2html guarantees is in [SECURITY.md](../SECURITY.md), what changed in [CHANGELOG.md](../CHANGELOG.md), and where the work stands in [GOTO.md](../GOTO.md).

## For whoever uses md2html

1. [What md2html can do](capabilities.md): every feature, shown on a page it made, in seventeen languages (each translation beside it, `capabilities.de.md` and the others).
2. [The command line](command-line.md): every option, setting, exit code and environment variable, and what of it is stable.
3. [The showcase](showcase.md): a page that uses everything at once.

## For whoever changes md2html

4. [Contributing](CONTRIBUTING.md): how a change lands, and the order to read the rest in.
5. [How md2html is built](ARCHITECTURE.md): the modules in layers, and the directions between them.
6. [Testing](TESTING.md): each kind of test, what it guards and where it runs.
7. [Quality](QUALITY.md): the scorecard of the code and its checks.
8. [Maintaining](MAINTAINING.md): taking the project over, and every recurring chore.
9. [The supply chain](SUPPLY-CHAIN.md): the toolchain, the tools and the crates, and how each is trusted.

## The policies

The reasoning behind the rules, [the lint posture](policies/lint-posture.md) first:

10. [Lint posture](policies/lint-posture.md)
11. [Panics](policies/panic_policy.md)
12. [Allocation](policies/allocation_policy.md)
13. [Types](policies/type_policy.md)
14. [Concurrency](policies/concurrency_policy.md)
15. [Platforms](policies/platform_policy.md)
16. [Hostile input](policies/hostile_input_policy.md)
17. [Dependencies](policies/dependency_policy.md)
18. [Build scripts and generated code](policies/build_codegen_policy.md)
19. [Releases](policies/release_policy.md)

## The scaffolding

The template's machinery, kept as the template ships it:

20. [What the scaffolding is](forge/README.md)
21. [Getting started](forge/GETTING-STARTED.md)
22. [Growing the project](forge/COMPONENTS.md)
23. [Adopting and upgrading](forge/ADOPTING.md)
24. [Code signing](forge/CODE-SIGNING.md)

The maintainers' working notes, in a folder of this one that the repository does not hold, are not part of this book: no link here enters that folder, so md2html renders its pages as pages of their own and says so once.
