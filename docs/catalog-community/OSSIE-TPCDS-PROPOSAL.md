# Proposal: a loss-report contract for catalog converters

## Scope

Add a machine-readable conversion report to Apache Ossie’s converter guidance
and use the pinned TPC-DS example as the common acceptance fixture. This is a
proposal artifact, not a claim that Apache Ossie has accepted the change.

The motivating evidence is `ossie/polaris-tpcds-report.json`. At upstream
revision `1d9ebcea`, the Polaris converter passes all 45 Java tests and a live
export/import through Polaris 1.7.0 preserves one model, five datasets, and all
31 fields. The same round trip omits four relationships, five metrics, model AI
context, and both input custom extensions. It creates five COMMON dataset and
31 POLARIS field extensions for physical reconstruction and emits four decimal
precision-default warnings. Those are useful, distinct outcomes that a single
“conversion succeeded” result hides.

## Proposed report

Every converter run should report four independent sections:

1. `structural`: model, dataset, field, relationship, and metric counts before
   and after conversion.
2. `semantic`: expression dialect choice, relationship/metric retention, AI
   context retention, and semantic equivalence checks when implemented.
3. `extensions`: input extensions retained, transformed, generated, or lost,
   grouped by vendor name.
4. `loss`: explicit warnings and unsupported constructs, with a status of
   `lossless`, `verified-with-loss`, or `failed`.

The report must bind the source artifact hash, converter revision, output hash,
and test/runtime versions. A converter should exit nonzero when requested in
strict mode and an unapproved loss appears.

## Candidate upstream unit

A focused contribution can add the JSON report schema, a reusable count/loss
collector in `converters/common`, and a Polaris TPC-DS golden test. The first
golden result should encode the present behavior rather than silently changing
it; follow-up converter work can then reduce the explicit losses one category
at a time.
