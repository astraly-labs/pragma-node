# Post-mortem: incorrect STRK/USD price

## Summary

On 2026-07-08, the production API returned a STRK/USD median near `30.4` instead
of `0.03`. The invalid value came from a Pyth input near `60.75`. Because only
two independent spot price sources remained for STRK/USD, PostgreSQL computed
the median as the average of the valid AVNU value and the invalid Pyth value.

The loss of source diversity followed the removal of Martin Delbert from the
production manifests on 2026-07-02. Martin Delbert had reconstructed CEX order
books and published their mid-prices for ingestion. Its removal left only AVNU
and Pyth in the off-chain price database.

## Impact

- A consumer observed `30.4` at `2026-07-08 18:00:17 UTC`.
- The bad Pyth rows ran from `17:39:52` through `17:52:38 UTC` (594 rows).
- Eleven one-minute STRK/USD buckets, from `17:41` through `17:51 UTC`, had
  medians near `30.4`.
- The completed 15-minute bucket starting at `17:45 UTC` was
  `30.3918562422`; it was still the latest completed bucket when queried just
  after `18:00 UTC`.
- The raw AVNU values remained near `0.0298`. The following 15-minute bucket
  returned to a valid value near `0.02969`.

## Root cause

The incident required both failures below:

1. Pyth published STRK/USD near `60.75` instead of near `0.03`.
2. Martin Delbert had been decommissioned as "unused", so CEX-derived prices
   no longer reached the `entries` table. Only AVNU and Pyth remained.

With two sources, `percentile_cont(0.5)` averages both values:

```text
(0.0298 + 60.7539) / 2 = 30.39185
```

The aggregation behaved as configured; the unsafe invariant was allowing a
critical pair to serve a two-source median.

## Detection gaps

- No alert enforced a minimum independent-source count for critical pairs.
- No alert compared each source with the cross-source median.
- The infrastructure cleanup treated a running data-path component as unused
  without proving that its Kafka output had no downstream consumer.
- The consumer's fee path had no sanity bound on a 1000x price discontinuity.

## Corrective actions

### Implemented

- Pulse now derives a USD mid-price directly from each valid spot order-book
  snapshot; crossed or incomplete books are rejected.
- The old Martin Delbert hop is not restored. The data flow is now
  `Pulse -> pragma-prices-v1 -> ingestor`.
- Pulse explicitly publishes `PriceEntry` values to `pragma-prices-v1`, which
  is the topic consumed by the production ingestor.
- The STRK-only rollout refreshes the existing REST-backed CEX snapshots every
  60 seconds. The default remains 10 minutes for other Pulse deployments.
- A production canary is restricted to STRK and selected CEX sources before
  the change is made durable.

### Prevention

- Alert when a critical pair has fewer than four sources for two consecutive
  15-minute buckets; target five or more sources during normal operation.
- Alert and reject a source update that jumps by more than 2x from its last
  accepted value until it returns to the previous range or is corroborated by
  the cross-source median.
- Require data-flow evidence before decommissioning Kafka processors: active
  consumer group, input/output rates, downstream topic consumers, and a
  canary query of the final API.
- Add a production regression check for STRK/USD that validates source count,
  component spread, and returned price.

## Recovery criteria

Recovery is complete only when all of the following hold:

1. CEX-derived STRK/USD prices are present in the off-chain database through
   the direct Pulse path.
2. The 15-minute API aggregate returns to at least five sources within one
   minute of a bucket boundary.
3. No CEX component deviates materially from the cross-source median.
4. The scoped STRK Pulse deployment is healthy and redelivers snapshots after
   a restart.

## Recovery status

Recovery criteria were met on 2026-07-12:

- Pulse `v0.1.21` was rolled out in a dedicated STRK spot deployment using
  Binance, MEXC, and OKX.
- The production API reported AVNU, Binance, MEXC, OKX, and Pyth.
- Across the 14:15 UTC bucket boundary, the source count returned from four to
  five in less than one minute.
- After replacing the canary image with the final release, Binance and MEXC
  both published initial and subsequent 60-second snapshots.

The low-source and source-divergence alerts remain prevention follow-ups; they
detect future loss of redundancy but are not part of the data-path correction.
