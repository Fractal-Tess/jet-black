# Jet Black follow-up

The Rust control-plane cutover described by ADR-007 is complete. Follow-up work
is release evolution rather than migration:

- signed desktop updates and platform installers;
- hosted observability, rate limits, HA, and disaster recovery;
- richer module/page editing and intake conversion;
- worker scheduling beyond explicit selection;
- optional artifact object storage for hosted deployments.

These items require their own ADR or release milestone. Convex parity and
dual-write work are intentionally not tracked because Convex has been removed.
