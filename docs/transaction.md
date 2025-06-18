# Transaction Database Guide

## Overview

From now on, all database operations should be done using transactions, and the session-based methods are deprecated.

## What Changed

### Before (Session-based)

```rust
// Single operation without transaction
pub fn create_many(conn: &mut PgConnection, data: Vec<NewEntry>) -> DieselResult<Vec<Self>> {
    diesel::insert_into(entries::table)
        .values(data)
        .returning(Self::as_returning())
        .get_results(conn)
}
```

### After (Transactional)

```rust
// Transactional version with automatic rollback on failure
pub fn create_many_transactional(conn: &mut PgConnection, data: Vec<NewEntry>) -> DieselResult<Vec<Self>> {
    conn.transaction(|conn| Self::create_many(conn, data))
}
```

**Note:** Don't forget always to check if you add this `use diesel::Connection;` in your code.

## New Transactional Methods

All models now have transactional versions of their existing methods:

### Entry Model

- `create_one_transactional()`
- `create_many_transactional()`
- `exists_transactional()`
- `get_by_pair_id_transactional()`
- `with_filters_transactional()`
- `get_existing_pairs_transactional()`
- `get_last_updated_timestamp_transactional()`

### FutureEntry Model

- `create_one_transactional()`
- `create_many_transactional()`
- `exists_transactional()`
- `get_by_pair_id_transactional()`
- `with_filters_transactional()`
- `get_existing_pairs_transactional()`
- `get_existing_perp_pairs_transactional()`

### Publishers Model

- `get_by_name_transactional()`
- `with_filters_transactional()`
- `get_account_address_by_name_transactional()`

### FundingRate Model

- `create_many_transactional()`
- `get_latest_transactional()`
- `get_at_transactional()`
- `get_in_range_transactional()`
- `get_in_range_aggregated_transactional()`

### OpenInterest Model

- `create_many_transactional()`
- `get_latest_transactional()`
- `get_at_transactional()`
- `get_in_range_transactional()`

## Transaction Management Utilities

### Transaction Builder

For complex operations involving multiple database operations:

```rust
use pragma_entities::transaction::TransactionBuilder;

let result = TransactionBuilder::new()
    .add_operation(|conn| {
        Entry::create_many(conn, spot_entries)?;
        Ok(())
    })
    .add_operation(|conn| {
        FutureEntry::create_many(conn, future_entries)?;
        Ok(())
    })
    .execute(conn)?;
```

### Helper Macros

```rust
use pragma_entities::{transactional, batch_transactional};

// Single operation
let result = transactional!(conn, Entry::create_many(conn, entries))?;

// Batch operations
let result = batch_transactional!(conn, |conn| {
    Entry::create_many(conn, entries)?;
    FutureEntry::create_many(conn, future_entries)?;
    Ok(())
})?;
```

## Usage Examples

### Simple Transactional Operation

```rust
// Before
let conn = pool.get().await?;
conn.interact(move |conn| Entry::create_many(conn, new_entries))
    .await?;

// After
let conn = pool.get().await?;
conn.interact(move |conn| Entry::create_many_transactional(conn, new_entries))
    .await?;
```

### Complex Transactional Operation

```rust
// Multiple operations in a single transaction
let conn = pool.get().await?;
conn.interact(move |conn| {
    conn.transaction(|conn| {
        // All operations succeed or all fail
        Entry::create_many(conn, spot_entries)?;
        FutureEntry::create_many(conn, future_entries)?;
        FundingRate::create_many(conn, funding_rates)?;
        Ok(())
    })
})
.await?;
```

### Using Transaction Builder

```rust
let conn = pool.get().await?;
conn.interact(move |conn| {
    TransactionBuilder::new()
        .add_operation(|conn| {
            Entry::create_many(conn, spot_entries)?;
            Ok(())
        })
        .add_operation(|conn| {
            FutureEntry::create_many(conn, future_entries)?;
            Ok(())
        })
        .add_operation(|conn| {
            // Validate data after insertion
            let count = Entry::with_filters(conn, filters)?;
            if count.is_empty() {
                return Err(diesel::result::Error::NotFound);
            }
            Ok(())
        })
        .execute(conn)
})
.await?;
```

## Benefits

1. **Data Consistency**: All operations in a transaction succeed or fail together
2. **Rollback Capability**: Failed operations automatically rollback changes
3. **Atomic Operations**: Complex multi-step operations are atomic
4. **Better Error Handling**: Clear transaction boundaries for error recovery
5. **Performance**: Reduced connection overhead for batch operations

## Considerations

1. **Connection Pool**: Transactions hold connections longer, ensure adequate pool size
2. **Deadlock Prevention**: Order operations consistently to avoid deadlocks
3. **Error Handling**: Transaction failures provide clear rollback semantics
4. **Testing**: Test both success and failure scenarios for transactions

## Testing

```rust
#[test]
fn test_transactional_rollback() {
    let conn = &mut establish_connection();
    
    // This should rollback on error
    let result = Entry::create_many_transactional(conn, invalid_entries);
    assert!(result.is_err());
    
    // Verify no data was committed
    let count = Entry::with_filters(conn, filters).unwrap();
    assert_eq!(count.len(), 0);
}
```

## Future Enhancements

1. **Nested Transactions**: Support for savepoints and nested transactions
2. **Distributed Transactions**: Support for cross-database transactions
3. **Transaction Monitoring**: Metrics and monitoring for transaction performance
4. **Async Transactions**: Full async support for transaction operations
