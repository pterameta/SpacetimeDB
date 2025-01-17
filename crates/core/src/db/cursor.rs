use crate::error::DBError;
use spacetimedb_lib::relation::{DbTable, RowCount};
use spacetimedb_sats::ProductValue;

use super::datastore::locking_tx_datastore::Iter;

#[derive(Debug, Clone, Copy)]
pub enum CatalogKind {
    Table,
    Column,
    Index,
    Sequence,
}

/// Common wrapper for relational iterators that work like cursors.
pub struct TableCursor<'a> {
    pub table: DbTable,
    pub iter: Iter<'a>,
}

impl<'a> TableCursor<'a> {
    pub fn new(table: DbTable, iter: Iter<'a>) -> Result<Self, DBError> {
        Ok(Self { table, iter })
    }
}

/// Common wrapper for relational iterators of [Catalog].
pub struct CatalogCursor<I> {
    pub(crate) table: DbTable,
    #[allow(dead_code)]
    pub(crate) kind: CatalogKind,
    pub(crate) row_count: RowCount,
    pub(crate) iter: I,
}

impl<I> CatalogCursor<I> {
    pub fn new(table: DbTable, kind: CatalogKind, row_count: RowCount, iter: I) -> Self
    where
        I: Iterator<Item = ProductValue>,
    {
        Self {
            table,
            kind,
            row_count,
            iter,
        }
    }
}
