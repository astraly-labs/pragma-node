/// "PRAGMA" to number is bigger than 2**40 - we alias it to "PRGM" to fit in 40 bits.
///
/// Needed for `StarkEx` signature.
/// See:
/// <https://docs.starkware.co/starkex/perpetual/becoming-an-oracle-provider-for-starkex.html>
pub const PRAGMA_ORACLE_NAME_FOR_STARKEX: &str = "PRAGMA";
