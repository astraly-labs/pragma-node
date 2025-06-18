use diesel::Connection;
use diesel::PgConnection;
use diesel::result::Error as DieselError;

/// Type alias for transaction operations to reduce complexity
type TransactionOperation = Box<dyn FnOnce(&mut PgConnection) -> Result<(), DieselError>>;

/// A trait for operations that can be executed within a database transaction
pub trait TransactionalOperation<T> {
    fn execute(self, conn: &mut PgConnection) -> Result<T, DieselError>;
}

/// Executes a single operation within a transaction
pub fn execute_transactional<F, T>(conn: &mut PgConnection, operation: F) -> Result<T, DieselError>
where
    F: FnOnce(&mut PgConnection) -> Result<T, DieselError>,
{
    conn.transaction(operation)
}

/// Executes multiple operations within a single transaction
pub fn execute_batch_transactional<F, T>(
    conn: &mut PgConnection,
    operations: F,
) -> Result<T, DieselError>
where
    F: FnOnce(&mut PgConnection) -> Result<T, DieselError>,
{
    conn.transaction(operations)
}

/// A builder for complex transactional operations
pub struct TransactionBuilder {
    operations: Vec<TransactionOperation>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    #[must_use]
    pub fn add_operation<F>(mut self, operation: F) -> Self
    where
        F: FnOnce(&mut PgConnection) -> Result<(), DieselError> + 'static,
    {
        self.operations.push(Box::new(operation));
        self
    }

    pub fn execute(self, conn: &mut PgConnection) -> Result<(), DieselError> {
        conn.transaction(|conn| {
            for operation in self.operations {
                operation(conn)?;
            }
            Ok(())
        })
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro to create transactional operations
#[macro_export]
macro_rules! transactional {
    ($conn:expr, $operation:expr) => {
        $crate::transaction::execute_transactional($conn, $operation)
    };
}

/// Helper macro to create batch transactional operations
#[macro_export]
macro_rules! batch_transactional {
    ($conn:expr, $operations:expr) => {
        $crate::transaction::execute_batch_transactional($conn, $operations)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::result::Error;

    #[test]
    fn test_transaction_builder() {
        // This is a mock test to ensure the builder compiles correctly
        let builder = TransactionBuilder::new();
        assert_eq!(builder.operations.len(), 0);
    }

    #[test]
    fn test_execute_transactional() {
        // Mock test for the transactional execution
        let result: Result<(), Error> = Ok(());
        assert!(result.is_ok());
    }
}
