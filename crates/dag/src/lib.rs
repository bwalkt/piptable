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
}

#[derive(Debug, Clone, Default)]
pub struct Dag {
    nodes: HashMap<String, DagNode>,
    ranges: HashMap<String, CellCoordinateRange>,
    dirty_nodes: HashSet<String>,
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
            let address = cell_to_address(Some(addr), false, false, false, false)
                .unwrap_or_default();
            let mut prefix = String::new();
            if let Some(id) = &cell.data_validation_id {
                prefix.push_str(id);
            }
            if let Some(id) = &cell.conditional_format_id {
                prefix.push_str(id);
            }
            format!("{}{}!{}", prefix, cell.sheet_id, address)
        }
        NodeRef::Range(range) => {
            let start = CellAddress::new(range.start_row_index, range.start_column_index);
            let end = CellAddress::new(range.end_row_index, range.end_column_index);
            let start_addr = cell_to_address(Some(start), false, false, false, false)
                .unwrap_or_default();
            let end_addr = cell_to_address(Some(end), false, false, false, false)
                .unwrap_or_default();
            format!("{}!{}:{}", range.sheet_id, start_addr, end_addr)
        }
    }
}

impl Dag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node_input(
        &mut self,
        formula_cell: NodeRef,
        input_position: NodeRef,
        mark_as_dirty: bool,
    ) -> Result<(), DagError> {
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

    pub fn add_node_input_to_graph(
        &mut self,
        formula_cell: NodeRef,
        input_position: NodeRef,
    ) -> Result<(), DagError> {
        self.add_node_input(formula_cell, input_position, false)
    }

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

    pub fn delete_sheet(&mut self, sheet_id: u32) {
        let keys: Vec<String> = self
            .nodes
            .iter()
            .filter_map(|(key, node)| match &node.position {
                Some(NodePosition::Cell(cell)) if cell.sheet_id == sheet_id => Some(key.clone()),
                Some(NodePosition::Range(range)) if range.sheet_id == sheet_id => Some(key.clone()),
                None if node.kind == DagNodeKind::Static => Some(key.clone()),
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

    pub fn delete_cell(&mut self, pos: NodeRef) {
        self.clear_node_inputs(pos);
    }

    pub fn remove_cell_from_graph(&mut self, pos: NodeRef) {
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
    }

    pub fn delete_node(&mut self, pos: NodeRef) {
        let key = self.key(&pos);
        self.clear_node_inputs(pos);
        self.nodes.remove(&key);
        self.ranges.remove(&key);
        self.dirty_nodes.remove(&key);
    }

    pub fn clear_node_inputs(&mut self, pos: NodeRef) {
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

    pub fn mark_cell_as_dirty(&mut self, pos: NodeRef) {
        let key = self.ensure_node(&pos);
        self.mark_as_dirty_key(&key);
    }

    pub fn get_dirty_nodes(&mut self) -> Result<Vec<DagNodeIdentifier>, DagError> {
        let keys: Vec<String> = self.dirty_nodes.iter().cloned().collect();
        let dependents = self.topological_sort(keys, |node| self.dependents_with_ranges(node))?;
        self.dirty_nodes.clear();
        Ok(self.identifiers_from_keys(&dependents))
    }

    pub fn peek_dirty_nodes(&self) -> Result<Vec<DagNodeIdentifier>, DagError> {
        let keys: Vec<String> = self.dirty_nodes.iter().cloned().collect();
        let dependents = self.topological_sort(keys, |node| node.dependent_keys.clone())?;
        Ok(self.identifiers_from_keys(&dependents))
    }

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

    pub fn get_all_precedents(&self, pos: NodeRef) -> Result<Vec<DagNodeIdentifier>, DagError> {
        let key = self.key(&pos);
        let nodes = self.topological_sort(vec![key], |node| node.input_keys.clone())?;
        Ok(self.identifiers_from_keys(&nodes))
    }

    pub fn get_dependents(&self, pos: NodeRef) -> Result<Vec<DagNodeIdentifier>, DagError> {
        self.visit_node(pos, |node| self.dependents_with_ranges(node))
    }

    pub fn reset(&mut self) {
        self.ranges.clear();
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
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let nodes: Vec<(String, DagNodeJson)> = self
            .nodes
            .iter()
            .map(|(key, node)| (key.clone(), self.node_to_json(node)))
            .collect();
        serde_json::to_string(&nodes)
    }

    pub fn from_json(&mut self, nodes: Vec<(String, DagNodeJson)>) {
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

    fn ensure_node(&mut self, position: &NodeRef) -> String {
        let key = self.key(position);
        if self.nodes.contains_key(&key) {
            if let NodeRef::Range(range) = position {
                self.ranges.insert(key.clone(), range.clone());
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
        for range in self.ranges.values() {
            if !is_cell_coordinate_within_cell_range(cell, range) {
                continue;
            }
            if let Some(range_node) = self.nodes.get(&self.key(&NodeRef::Range(range.clone()))) {
                if !range_node.dependent_keys.is_empty() {
                    valid_ranges.push(range.clone());
                }
            }
        }
        valid_ranges
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
        let mut stack: Vec<String> = Vec::new();

        for root in roots {
            if !self.nodes.contains_key(&root) {
                continue;
            }
            if state.get(&root).copied().unwrap_or(0) == 0 {
                self.visit_topo(&root, &children, &mut state, &mut stack, &mut order)?;
            }
        }

        order.reverse();
        Ok(order)
    }

    fn visit_topo<F>(
        &self,
        key: &str,
        children: &F,
        state: &mut HashMap<String, u8>,
        stack: &mut Vec<String>,
        order: &mut Vec<String>,
    ) -> Result<(), DagError>
    where
        F: Fn(&DagNode) -> HashSet<String>,
    {
        state.insert(key.to_string(), 1);
        stack.push(key.to_string());

        if let Some(node) = self.nodes.get(key) {
            for child in children(node) {
                match state.get(&child).copied().unwrap_or(0) {
                    0 => {
                        self.visit_topo(&child, children, state, stack, order)?;
                    }
                    1 => {
                        let mut cycle = stack.clone();
                        cycle.push(child);
                        return Err(DagError::CircularDependency { cycle });
                    }
                    _ => {}
                }
            }
        }

        stack.pop();
        state.insert(key.to_string(), 2);
        order.push(key.to_string());
        Ok(())
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
        restored.from_json(nodes);
        assert!(restored.has_node(&a1));
        assert!(restored.has_node(&b1));
    }
}
