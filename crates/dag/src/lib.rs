//! Directed Acyclic Graph for spreadsheet dependency tracking.

use piptable_primitives::{cell_to_address, CellAddress};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellCoordinate {
    pub row_index: u32,
    pub column_index: u32,
    pub sheet_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_validation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional_format_id: Option<String>,
}

impl CellCoordinate {
    pub fn new(sheet_id: u32, row_index: u32, column_index: u32) -> Self {
        Self {
            row_index,
            column_index,
            sheet_id,
            data_validation_id: None,
            conditional_format_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellCoordinateRange {
    pub sheet_id: u32,
    pub start_row_index: u32,
    pub start_column_index: u32,
    pub end_row_index: u32,
    pub end_column_index: u32,
}

impl CellCoordinateRange {
    pub fn new(
        sheet_id: u32,
        start_row_index: u32,
        start_column_index: u32,
        end_row_index: u32,
        end_column_index: u32,
    ) -> Self {
        Self {
            sheet_id,
            start_row_index,
            start_column_index,
            end_row_index,
            end_column_index,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StaticReference {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NodePosition {
    Cell(CellCoordinate),
    Range(CellCoordinateRange),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeRef {
    Cell(CellCoordinate),
    Range(CellCoordinateRange),
    Static(StaticReference),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DagNodeKind {
    Cell,
    Range,
    Static,
}

#[derive(Debug, Clone)]
pub struct DagNode {
    pub key: String,
    pub position: Option<NodePosition>,
    input_keys: HashSet<String>,
    dependent_keys: HashSet<String>,
    kind: DagNodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DagNodeIdentifier {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<NodePosition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DagNodeJson {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<NodePosition>,
    pub input_keys: Vec<DagNodeIdentifier>,
    pub dependent_keys: Vec<DagNodeIdentifier>,
}

#[derive(Debug, thiserror::Error)]
pub enum DagError {
    #[error("circular dependency detected")]
    CircularDependency { cycle: Vec<String> },
    #[error("range dependency too large: {cells} cells (max {max})")]
    RangeTooLarge { cells: u64, max: u64 },
}

#[derive(Debug, Clone)]
pub struct Dag {
    nodes: HashMap<String, DagNode>,
    ranges: HashMap<String, CellCoordinateRange>,
    ranges_by_row: HashMap<u32, HashMap<u32, Vec<String>>>,
    dirty_nodes: HashSet<String>,
    max_range_cells: u64,
}

impl Dag {
    /// Validate that the graph is acyclic using a full topological traversal.
    pub fn validate_acyclic(&self) -> Result<(), DagError> {
        let keys: Vec<String> = self.nodes.keys().cloned().collect();
        self.topological_sort(keys, |node| node.input_keys.clone())?;
        Ok(())
    }
}

impl Default for Dag {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            ranges: HashMap::new(),
            ranges_by_row: HashMap::new(),
            dirty_nodes: HashSet::new(),
            max_range_cells: 10_000,
        }
    }
}

/// Controls how a delete operation mutates the graph.
#[derive(Debug, Clone)]
pub enum DeleteMode {
    /// Clear inputs but keep dependents.
    ClearInputs,
    /// Detach from inputs; keep dependents.
    DetachFromInputs,
    /// Remove node and all links.
    RemoveNode,
}

/// Batch DAG operations for efficient updates.
#[derive(Debug, Clone)]
pub enum DagOperation {
    /// Add a dependency edge.
    AddInput {
        formula: NodeRef,
        input: NodeRef,
        mark_as_dirty: bool,
    },
    /// Remove a dependency edge.
    RemoveInput {
        formula: NodeRef,
        input: NodeRef,
    },
    /// Delete or detach a node according to mode.
    Delete {
        position: NodeRef,
        mode: DeleteMode,
    },
}

pub fn is_cell_coordinate_within_cell_range(
    position: &CellCoordinate,
    range: &CellCoordinateRange,
) -> bool {
    position.sheet_id == range.sheet_id
        && position.row_index >= range.start_row_index
        && position.row_index <= range.end_row_index
        && position.column_index >= range.start_column_index
        && position.column_index <= range.end_column_index
}

pub fn is_cell_coordinate(position: &NodeRef) -> bool {
    matches!(position, NodeRef::Cell(_))
}

pub fn is_static_reference(position: &NodeRef) -> bool {
    matches!(position, NodeRef::Static(_))
}

pub fn make_key(position: &NodeRef) -> String {
    match position {
        NodeRef::Static(reference) => reference.id.clone(),
        NodeRef::Cell(cell) => {
            let addr = CellAddress::new(cell.row_index, cell.column_index);
            let address =
                cell_to_address(Some(addr), false, false, false, false).unwrap_or_default();
            let mut prefix = String::new();
            if let Some(id) = &cell.data_validation_id {
                prefix.push_str("dv=");
                prefix.push_str(id);
                prefix.push(';');
            }
            if let Some(id) = &cell.conditional_format_id {
                prefix.push_str("cf=");
                prefix.push_str(id);
                prefix.push(';');
            }
            format!("{}{}!{}", prefix, cell.sheet_id, address)
        }
        NodeRef::Range(range) => {
            let start = CellAddress::new(range.start_row_index, range.start_column_index);
            let end = CellAddress::new(range.end_row_index, range.end_column_index);
            let start_addr =
                cell_to_address(Some(start), false, false, false, false).unwrap_or_default();
            let end_addr =
                cell_to_address(Some(end), false, false, false, false).unwrap_or_default();
            format!("{}!{}:{}", range.sheet_id, start_addr, end_addr)
        }
    }
}

impl Dag {
    /// Create a new DAG with default limits.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a DAG with a custom maximum range size.
    pub fn with_max_range_cells(max_range_cells: u64) -> Self {
        Self {
            max_range_cells,
            ..Self::default()
        }
    }

    /// Return the maximum allowed range size (in cells).
    pub fn max_range_cells(&self) -> u64 {
        self.max_range_cells
    }

    /// Add a dependency edge from a formula cell to an input.
    pub fn add_node_input(
        &mut self,
        formula_cell: NodeRef,
        input_position: NodeRef,
        mark_as_dirty: bool,
    ) -> Result<(), DagError> {
        if let NodeRef::Range(range) = &input_position {
            let rows = range
                .end_row_index
                .saturating_sub(range.start_row_index)
                .saturating_add(1) as u64;
            let cols = range
                .end_column_index
                .saturating_sub(range.start_column_index)
                .saturating_add(1) as u64;
            let cells = rows.saturating_mul(cols);
            if cells > self.max_range_cells {
                return Err(DagError::RangeTooLarge {
                    cells,
                    max: self.max_range_cells,
                });
            }
        }
        let formula_key = self.ensure_node(&formula_cell);
        let input_key = self.ensure_node(&input_position);

        if formula_key == input_key {
            return Err(DagError::CircularDependency {
                cycle: vec![formula_key],
            });
        }

        let input_is_range = matches!(input_position, NodeRef::Range(_));
        let formula_is_cell = matches!(formula_cell, NodeRef::Cell(_));
        if formula_is_cell && input_is_range {
            if let (Some(NodePosition::Cell(cell)), Some(NodePosition::Range(range))) = (
                self.nodes
                    .get(&formula_key)
                    .and_then(|node| node.position.clone()),
                self.nodes
                    .get(&input_key)
                    .and_then(|node| node.position.clone()),
            ) {
                if is_cell_coordinate_within_cell_range(&cell, &range) {
                    return Err(DagError::CircularDependency {
                        cycle: vec![formula_key, input_key],
                    });
                }
                for (key, node) in &self.nodes {
                    let Some(NodePosition::Cell(pos)) = &node.position else {
                        continue;
                    };
                    if !is_cell_coordinate_within_cell_range(pos, &range) {
                        continue;
                    }
                    if self.has_path(&formula_key, key) {
                        return Err(DagError::CircularDependency {
                            cycle: vec![formula_key, key.clone()],
                        });
                    }
                }
            }
        }

        if self.has_path(&formula_key, &input_key) {
            return Err(DagError::CircularDependency {
                cycle: vec![formula_key, input_key],
            });
        }

        if mark_as_dirty {
            self.mark_as_dirty_key(&formula_key);
        }

        let is_static = matches!(
            self.nodes.get(&formula_key).map(|n| n.kind),
            Some(DagNodeKind::Static)
        );
        let is_cell = matches!(
            self.nodes.get(&formula_key).map(|n| n.kind),
            Some(DagNodeKind::Cell)
        );
        if is_static || is_cell {
            if let Some(node) = self.nodes.get_mut(&formula_key) {
                node.input_keys.insert(input_key.clone());
            }
        }
        if let Some(input_node) = self.nodes.get_mut(&input_key) {
            input_node.dependent_keys.insert(formula_key);
        }
        Ok(())
    }

    /// Add an input edge without marking dirty.
    pub fn add_node_input_to_graph(
        &mut self,
        formula_cell: NodeRef,
        input_position: NodeRef,
    ) -> Result<(), DagError> {
        self.add_node_input(formula_cell, input_position, false)
    }

    /// Add multiple inputs to a formula node.
    pub fn add_node_inputs<I>(
        &mut self,
        formula_cell: NodeRef,
        inputs: I,
        mark_as_dirty: bool,
    ) -> Result<(), DagError>
    where
        I: IntoIterator<Item = NodeRef>,
    {
        for input in inputs {
            self.add_node_input(formula_cell.clone(), input, mark_as_dirty)?;
        }
        Ok(())
    }

    /// Remove a dependency edge between two nodes.
    pub fn remove_node_input(&mut self, formula_cell: NodeRef, input_position: NodeRef) {
        let formula_key = self.key(&formula_cell);
        let input_key = self.key(&input_position);

        if let Some(node) = self.nodes.get_mut(&formula_key) {
            node.input_keys.remove(&input_key);
        }
        if let Some(node) = self.nodes.get_mut(&input_key) {
            node.dependent_keys.remove(&formula_key);
        }
    }

    /// Delete all nodes belonging to a sheet.
    pub fn delete_sheet(&mut self, sheet_id: u32) {
        let keys: Vec<String> = self
            .nodes
            .iter()
            .filter_map(|(key, node)| match &node.position {
                Some(NodePosition::Cell(cell)) if cell.sheet_id == sheet_id => Some(key.clone()),
                Some(NodePosition::Range(range)) if range.sheet_id == sheet_id => Some(key.clone()),
                _ => None,
            })
            .collect();

        for key in keys {
            let node_ref = self
                .nodes
                .get(&key)
                .and_then(|node| node.position.clone())
                .map(|pos| match pos {
                    NodePosition::Cell(cell) => NodeRef::Cell(cell),
                    NodePosition::Range(range) => NodeRef::Range(range),
                })
                .unwrap_or_else(|| NodeRef::Static(StaticReference { id: key.clone() }));
            self.delete_node(node_ref);
        }
    }

    /// Clear inputs for a cell node while keeping dependents intact.
    pub fn delete_cell(&mut self, pos: NodeRef) {
        // Clear incoming edges but keep dependents intact.
        let key = self.key(&pos);
        self.clear_node_inputs(pos);
        self.prune_if_orphan(&key);
    }

    /// Detach a node from its inputs, keep dependents intact.
    pub fn remove_cell_from_graph(&mut self, pos: NodeRef) {
        // Detach this node from its inputs, keep dependents intact.
        let key = self.key(&pos);
        let inputs = if let Some(node) = self.nodes.get_mut(&key) {
            if matches!(node.kind, DagNodeKind::Cell | DagNodeKind::Static) {
                node.input_keys.drain().collect()
            } else {
                Vec::new()
            }
        } else {
            return;
        };
        for input_key in inputs {
            if let Some(input_node) = self.nodes.get_mut(&input_key) {
                input_node.dependent_keys.remove(&key);
            }
        }
        self.prune_if_orphan(&key);
    }

    /// Fully remove a node and its dependency links.
    pub fn delete_node(&mut self, pos: NodeRef) {
        // Fully remove a node and its dependency links.
        let key = self.key(&pos);
        self.clear_node_inputs(pos);
        self.nodes.remove(&key);
        self.ranges.remove(&key);
        self.remove_range_index(&key);
        self.dirty_nodes.remove(&key);
    }

    /// Clear incoming edges from inputs; keep dependents referencing this node.
    pub fn clear_node_inputs(&mut self, pos: NodeRef) {
        // Clear incoming edges from inputs; keep dependents referencing this node.
        let key = self.key(&pos);
        let inputs = if let Some(node) = self.nodes.get_mut(&key) {
            if matches!(node.kind, DagNodeKind::Cell | DagNodeKind::Static) {
                node.input_keys.drain().collect()
            } else {
                Vec::new()
            }
        } else {
            return;
        };

        for input_key in inputs {
            if let Some(input_node) = self.nodes.get_mut(&input_key) {
                input_node.dependent_keys.remove(&key);
            }
        }
        self.mark_as_dirty_key(&key);
    }

    /// Mark a node as dirty for recalculation.
    pub fn mark_cell_as_dirty(&mut self, pos: NodeRef) {
        let key = self.ensure_node(&pos);
        self.mark_as_dirty_key(&key);
    }

    /// Return dirty nodes in recalculation order without clearing the dirty set.
    pub fn get_dirty_nodes(&self) -> Result<Vec<DagNodeIdentifier>, DagError> {
        let keys: Vec<String> = self.dirty_nodes.iter().cloned().collect();
        let dependents = self.topological_sort(keys, |node| self.dependents_with_ranges(node))?;
        Ok(self.identifiers_from_keys(&dependents))
    }

    /// Return dirty nodes and clear the dirty set.
    pub fn take_dirty_nodes(&mut self) -> Result<Vec<DagNodeIdentifier>, DagError> {
        let nodes = self.get_dirty_nodes()?;
        self.dirty_nodes.clear();
        Ok(nodes)
    }

    /// Peek dirty nodes using range-aware dependents.
    pub fn peek_dirty_nodes(&self) -> Result<Vec<DagNodeIdentifier>, DagError> {
        let keys: Vec<String> = self.dirty_nodes.iter().cloned().collect();
        let dependents = self.topological_sort(keys, |node| self.dependents_with_ranges(node))?;
        Ok(self.identifiers_from_keys(&dependents))
    }

    /// Return direct precedents for a node.
    pub fn get_precedents(&self, pos: NodeRef) -> Result<Vec<DagNodeIdentifier>, DagError> {
        self.visit_node(pos, |node| node.input_keys.clone())
    }

    pub fn has_precedents(&self, pos: NodeRef) -> bool {
        let key = self.key(&pos);
        self.nodes
            .get(&key)
            .map(|node| !node.input_keys.is_empty())
            .unwrap_or(false)
    }

    /// Return all precedents in topological order.
    pub fn get_all_precedents(&self, pos: NodeRef) -> Result<Vec<DagNodeIdentifier>, DagError> {
        let key = self.key(&pos);
        let nodes = self.topological_sort(vec![key], |node| node.input_keys.clone())?;
        Ok(self.identifiers_from_keys(&nodes))
    }

    /// Return dependents in topological order.
    pub fn get_dependents(&self, pos: NodeRef) -> Result<Vec<DagNodeIdentifier>, DagError> {
        self.visit_node(pos, |node| self.dependents_with_ranges(node))
    }

    /// Apply a batch of DAG operations.
    pub fn apply_operations<I>(&mut self, ops: I) -> Result<(), DagError>
    where
        I: IntoIterator<Item = DagOperation>,
    {
        for op in ops {
            match op {
                DagOperation::AddInput {
                    formula,
                    input,
                    mark_as_dirty,
                } => self.add_node_input(formula, input, mark_as_dirty)?,
                DagOperation::RemoveInput { formula, input } => {
                    self.remove_node_input(formula, input);
                }
                DagOperation::Delete { position, mode } => match mode {
                    DeleteMode::ClearInputs => self.delete_cell(position),
                    DeleteMode::DetachFromInputs => self.remove_cell_from_graph(position),
                    DeleteMode::RemoveNode => self.delete_node(position),
                },
            }
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        self.ranges.clear();
        self.ranges_by_row.clear();
        self.dirty_nodes.clear();
        self.nodes.clear();
    }

    pub fn cleanup(&mut self) {
        let keys: Vec<String> = self
            .ranges
            .keys()
            .filter(|key| {
                self.nodes
                    .get(*key)
                    .map(|node| node.dependent_keys.is_empty())
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        for key in keys {
            self.ranges.remove(&key);
            self.remove_range_index(&key);
        }
    }

    /// Serialize the DAG to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let nodes: Vec<(String, DagNodeJson)> = self
            .nodes
            .iter()
            .map(|(key, node)| (key.clone(), self.node_to_json(node)))
            .collect();
        serde_json::to_string(&nodes)
    }

    /// Restore DAG state from JSON and validate acyclicity.
    pub fn from_json(&mut self, nodes: Vec<(String, DagNodeJson)>) -> Result<(), DagError> {
        self.reset();
        for (key, node_json) in &nodes {
            let position = node_json.position.clone();
            let node_ref = position
                .clone()
                .map(|pos| match pos {
                    NodePosition::Cell(cell) => NodeRef::Cell(cell),
                    NodePosition::Range(range) => NodeRef::Range(range),
                })
                .unwrap_or_else(|| NodeRef::Static(StaticReference { id: key.clone() }));
            self.ensure_node(&node_ref);
            if let Some(existing) = self.nodes.get_mut(key) {
                existing.position = position.clone();
            }
        }

        for (key, node_json) in nodes {
            let Some(node) = self.nodes.get_mut(&key) else {
                continue;
            };
            for input in node_json.input_keys {
                node.input_keys.insert(input.key);
            }
            for dependent in node_json.dependent_keys {
                node.dependent_keys.insert(dependent.key);
            }
        }
        self.validate_acyclic()?;
        Ok(())
    }

    pub fn key(&self, position: &NodeRef) -> String {
        make_key(position)
    }

    pub fn has_node(&self, position: &NodeRef) -> bool {
        let key = self.key(position);
        self.nodes.contains_key(&key)
    }

    pub fn has_array_node(&self, position: &NodeRef) -> bool {
        let NodeRef::Cell(cell) = position else {
            return false;
        };
        !self.get_node_from_cell_ranges(cell).is_empty()
    }

    /// Return the direct input nodes (cells, ranges, or statics) for a position.
    pub fn inputs_for(&self, position: &NodeRef) -> Vec<NodeRef> {
        let key = self.key(position);
        let Some(node) = self.nodes.get(&key) else {
            return Vec::new();
        };
        node.input_keys
            .iter()
            .filter_map(|input_key| self.nodes.get(input_key))
            .map(|node| match &node.position {
                Some(NodePosition::Cell(cell)) => NodeRef::Cell(cell.clone()),
                Some(NodePosition::Range(range)) => NodeRef::Range(range.clone()),
                None => NodeRef::Static(StaticReference {
                    id: node.key.clone(),
                }),
            })
            .collect()
    }

    fn ensure_node(&mut self, position: &NodeRef) -> String {
        let key = self.key(position);
        if self.nodes.contains_key(&key) {
            if let NodeRef::Range(range) = position {
                self.ranges.insert(key.clone(), range.clone());
                self.add_range_index(&key, range);
            }
            return key;
        }

        let (kind, pos) = match position {
            NodeRef::Cell(cell) => (DagNodeKind::Cell, Some(NodePosition::Cell(cell.clone()))),
            NodeRef::Range(range) => (DagNodeKind::Range, Some(NodePosition::Range(range.clone()))),
            NodeRef::Static(_) => (DagNodeKind::Static, None),
        };

        let node = DagNode {
            key: key.clone(),
            position: pos,
            input_keys: HashSet::new(),
            dependent_keys: HashSet::new(),
            kind,
        };

        if let NodeRef::Range(range) = position {
            self.ranges.insert(key.clone(), range.clone());
            self.add_range_index(&key, range);
        }

        self.nodes.insert(key.clone(), node);
        key
    }

    fn mark_as_dirty_key(&mut self, key: &str) {
        if let Some(node) = self.nodes.get(key) {
            if node.kind == DagNodeKind::Cell {
                self.dirty_nodes.insert(key.to_string());
            }
        }
    }

    fn prune_if_orphan(&mut self, key: &str) {
        let remove = self
            .nodes
            .get(key)
            .map(|node| node.input_keys.is_empty() && node.dependent_keys.is_empty())
            .unwrap_or(false);
        if remove {
            self.nodes.remove(key);
            self.ranges.remove(key);
            self.remove_range_index(key);
            self.dirty_nodes.remove(key);
        }
    }

    fn visit_node<F>(&self, pos: NodeRef, callback: F) -> Result<Vec<DagNodeIdentifier>, DagError>
    where
        F: Fn(&DagNode) -> HashSet<String>,
    {
        let key = self.key(&pos);
        let nodes = self.topological_sort(vec![key], callback)?;
        Ok(self.identifiers_from_keys(&nodes))
    }

    fn identifiers_from_keys(&self, keys: &[String]) -> Vec<DagNodeIdentifier> {
        keys.iter()
            .filter_map(|key| self.nodes.get(key))
            .map(|node| self.node_identifier(node))
            .collect()
    }

    fn node_identifier(&self, node: &DagNode) -> DagNodeIdentifier {
        DagNodeIdentifier {
            key: node.key.clone(),
            position: node.position.clone(),
        }
    }

    fn node_to_json(&self, node: &DagNode) -> DagNodeJson {
        let input_keys = node
            .input_keys
            .iter()
            .filter_map(|key| self.nodes.get(key))
            .map(|node| self.node_identifier(node))
            .collect();
        let dependent_keys = node
            .dependent_keys
            .iter()
            .filter_map(|key| self.nodes.get(key))
            .map(|node| self.node_identifier(node))
            .collect();

        DagNodeJson {
            key: node.key.clone(),
            position: node.position.clone(),
            input_keys,
            dependent_keys,
        }
    }

    fn get_node_from_cell_ranges(&self, cell: &CellCoordinate) -> Vec<CellCoordinateRange> {
        let mut valid_ranges = Vec::new();
        let Some(sheet_rows) = self.ranges_by_row.get(&cell.sheet_id) else {
            return valid_ranges;
        };
        let Some(range_keys) = sheet_rows.get(&cell.row_index) else {
            return valid_ranges;
        };

        for key in range_keys {
            let Some(range) = self.ranges.get(key) else {
                continue;
            };
            if !is_cell_coordinate_within_cell_range(cell, range) {
                continue;
            }
            if let Some(range_node) = self.nodes.get(key) {
                if !range_node.dependent_keys.is_empty() {
                    valid_ranges.push(range.clone());
                }
            }
        }
        valid_ranges
    }

    fn add_range_index(&mut self, key: &str, range: &CellCoordinateRange) {
        let sheet = self.ranges_by_row.entry(range.sheet_id).or_default();
        let start = range.start_row_index.min(range.end_row_index);
        let end = range.start_row_index.max(range.end_row_index);
        for row in start..=end {
            let entry = sheet.entry(row).or_default();
            if !entry.iter().any(|k| k == key) {
                entry.push(key.to_string());
            }
        }
    }

    fn remove_range_index(&mut self, key: &str) {
        for sheet in self.ranges_by_row.values_mut() {
            for keys in sheet.values_mut() {
                keys.retain(|k| k != key);
            }
        }
    }

    fn dependents_with_ranges(&self, node: &DagNode) -> HashSet<String> {
        let mut dependents = node.dependent_keys.clone();
        if let Some(NodePosition::Cell(cell)) = &node.position {
            for range in self.get_node_from_cell_ranges(cell) {
                let range_key = self.key(&NodeRef::Range(range));
                if let Some(range_node) = self.nodes.get(&range_key) {
                    dependents.extend(range_node.dependent_keys.iter().cloned());
                }
            }
        }
        dependents
    }

    fn has_path(&self, from_key: &str, to_key: &str) -> bool {
        if from_key == to_key {
            return true;
        }
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(from_key.to_string());

        while let Some(key) = queue.pop_front() {
            if !visited.insert(key.clone()) {
                continue;
            }
            if key == to_key {
                return true;
            }
            if let Some(node) = self.nodes.get(&key) {
                for dep in &node.dependent_keys {
                    if !visited.contains(dep) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }
        false
    }

    fn topological_sort<F>(&self, roots: Vec<String>, children: F) -> Result<Vec<String>, DagError>
    where
        F: Fn(&DagNode) -> HashSet<String>,
    {
        let mut order = Vec::new();
        let mut state: HashMap<String, u8> = HashMap::new();
        let mut path_stack: Vec<String> = Vec::new();

        for root in roots {
            if !self.nodes.contains_key(&root) {
                continue;
            }
            if state.get(&root).copied().unwrap_or(0) != 0 {
                continue;
            }

            let mut stack: Vec<(String, bool)> = Vec::new();
            stack.push((root, false));

            while let Some((key, expanded)) = stack.pop() {
                if expanded {
                    state.insert(key.clone(), 2);
                    order.push(key);
                    path_stack.pop();
                    continue;
                }

                if state.get(&key).copied().unwrap_or(0) == 2 {
                    continue;
                }

                if state.get(&key).copied().unwrap_or(0) == 1 {
                    let mut cycle = path_stack.clone();
                    cycle.push(key);
                    return Err(DagError::CircularDependency { cycle });
                }

                state.insert(key.clone(), 1);
                path_stack.push(key.clone());
                stack.push((key.clone(), true));

                if let Some(node) = self.nodes.get(&key) {
                    for child in children(node) {
                        if state.get(&child).copied().unwrap_or(0) != 2 {
                            stack.push((child, false));
                        }
                    }
                }
            }
        }

        order.reverse();
        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_dependency_and_dirty_nodes() {
        let mut dag = Dag::new();
        let a1 = NodeRef::Cell(CellCoordinate::new(1, 0, 0));
        let b1 = NodeRef::Cell(CellCoordinate::new(1, 0, 1));

        dag.add_node_input(b1.clone(), a1.clone(), true).unwrap();
        dag.mark_cell_as_dirty(a1);

        let dirty = dag.get_dirty_nodes().unwrap();
        assert!(dirty.iter().any(|node| node.key == dag.key(&b1)));
    }

    #[test]
    fn test_range_dependents() {
        let mut dag = Dag::new();
        let a1 = NodeRef::Cell(CellCoordinate::new(1, 0, 0));
        let c1 = NodeRef::Cell(CellCoordinate::new(1, 0, 2));
        let range = NodeRef::Range(CellCoordinateRange::new(1, 0, 0, 1, 0));

        dag.add_node_input(c1.clone(), range, true).unwrap();
        dag.mark_cell_as_dirty(a1);

        let dirty = dag.get_dirty_nodes().unwrap();
        assert!(dirty.iter().any(|node| node.key == dag.key(&c1)));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut dag = Dag::new();
        let a1 = NodeRef::Cell(CellCoordinate::new(1, 0, 0));
        let b1 = NodeRef::Cell(CellCoordinate::new(1, 0, 1));

        dag.add_node_input(a1.clone(), b1.clone(), true).unwrap();
        let err = dag.add_node_input(b1, a1, true).unwrap_err();
        assert!(matches!(err, DagError::CircularDependency { .. }));
    }

    #[test]
    fn test_to_json_roundtrip() {
        let mut dag = Dag::new();
        let a1 = NodeRef::Cell(CellCoordinate::new(1, 0, 0));
        let b1 = NodeRef::Cell(CellCoordinate::new(1, 0, 1));
        dag.add_node_input(b1.clone(), a1.clone(), true).unwrap();

        let json = dag.to_json().unwrap();
        let nodes: Vec<(String, DagNodeJson)> = serde_json::from_str(&json).unwrap();

        let mut restored = Dag::new();
        restored.from_json(nodes).unwrap();
        assert!(restored.has_node(&a1));
        assert!(restored.has_node(&b1));
    }

    #[test]
    fn test_delete_sheet_keeps_static_nodes() {
        let mut dag = Dag::new();
        let static_ref = NodeRef::Static(StaticReference {
            id: "global".to_string(),
        });
        let cell = NodeRef::Cell(CellCoordinate::new(1, 0, 0));
        dag.add_node_input(cell, static_ref.clone(), true).unwrap();
        dag.delete_sheet(1);
        assert!(dag.has_node(&static_ref));
    }

    #[test]
    fn test_delete_cell_prunes_orphan() {
        let mut dag = Dag::new();
        let cell = NodeRef::Cell(CellCoordinate::new(1, 0, 0));
        dag.mark_cell_as_dirty(cell.clone());
        dag.delete_cell(cell.clone());
        assert!(!dag.has_node(&cell));
    }

    #[test]
    fn test_range_too_large() {
        let mut dag = Dag::with_max_range_cells(3);
        let formula = NodeRef::Cell(CellCoordinate::new(1, 0, 0));
        let range = NodeRef::Range(CellCoordinateRange::new(1, 0, 0, 1, 1));
        let err = dag.add_node_input(formula, range, true).unwrap_err();
        assert!(matches!(err, DagError::RangeTooLarge { .. }));
    }
}
