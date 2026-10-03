//! The three answers of each row (docs/plans/links.md): its proof, its source, and its
//! uses.

use super::{Database, Node, Root, StoreError};
use std::collections::{BTreeSet, HashSet};

/// The model call that wrote a row, and what that call read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub call: u64,
    pub reads: Vec<Node>,
}

impl Database {
    /// The roots behind a node: follow the line or call that made it, and for a call, what
    /// it read, down to the input lines. The story program writes links back in time only,
    /// and the set of visited nodes ends a cycle that another program wrote.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn proof_of(&self, node: Node) -> Result<BTreeSet<Root>, StoreError> {
        let mut roots = BTreeSet::new();
        let mut visited = HashSet::new();
        let mut waiting = vec![node];
        while let Some(node) = waiting.pop() {
            if !visited.insert(node) {
                continue;
            }
            if let Node::Input(position) = node {
                roots.insert(self.root_of_input(position)?);
                continue;
            }
            match self.origin(node)? {
                Some(origin) => waiting.push(origin),
                None => {
                    roots.insert(Root::Lost);
                }
            }
            if let Node::Call(position) = node {
                waiting.extend(self.reads_of(position)?);
            }
        }
        // A cycle that another program wrote never reaches an input line.
        if roots.is_empty() {
            roots.insert(Root::Lost);
        }
        Ok(roots)
    }

    /// The call that wrote a node, and what it read. A node from a game line has none.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn source_of(&self, node: Node) -> Result<Option<Source>, StoreError> {
        let Some(Node::Call(call)) = self.origin(node)? else {
            return Ok(None);
        };
        let reads = self.reads_of(call)?;
        Ok(Some(Source { call, reads }))
    }

    /// The accepted calls that read a node. A refused or failed call uses nothing.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn uses_of(&self, node: Node) -> Result<Vec<u64>, StoreError> {
        self.accepted_readers_of(node)
    }
}

/// The root that a player sees: the weakest one. A row with no root is lost.
#[must_use]
pub fn weakest(roots: &BTreeSet<Root>) -> Root {
    roots.first().copied().unwrap_or(Root::Lost)
}
