use piptable_dag::{
    CellCoordinate, CellCoordinateRange, Dag, DagError, DagOperation, DeleteMode, NodeRef,
    StaticReference,
};

fn cell(sheet_id: u32, row: u32, col: u32) -> NodeRef {
    NodeRef::Cell(CellCoordinate::new(sheet_id, row, col))
}

fn range(sheet_id: u32, start_row: u32, start_col: u32, end_row: u32, end_col: u32) -> NodeRef {
    NodeRef::Range(CellCoordinateRange::new(
        sheet_id, start_row, start_col, end_row, end_col,
    ))
}

#[test]
fn test_add_and_remove_inputs() {
    let mut dag = Dag::new();
    let formula = cell(0, 0, 0);
    let input = cell(0, 1, 0);

    dag.add_node_input(formula.clone(), input.clone(), true)
        .expect("add input");

    let precedents = dag.get_precedents(formula.clone()).expect("precedents");
    let input_key = dag.key(&input);
    assert!(precedents.iter().any(|node| node.key == input_key));

    let dependents = dag.get_dependents(input.clone()).expect("dependents");
    let formula_key = dag.key(&formula);
    assert!(dependents.iter().any(|node| node.key == formula_key));

    dag.remove_node_input(formula.clone(), input.clone());
    assert!(!dag.has_precedents(formula));
    let dependents = dag.get_dependents(input).expect("dependents");
    assert!(!dependents.iter().any(|node| node.key == formula_key));
}

#[test]
fn test_circular_dependency_detection() {
    let mut dag = Dag::new();
    let node = cell(0, 0, 0);
    let err = dag
        .add_node_input(node.clone(), node.clone(), false)
        .expect_err("self-cycle");
    assert!(matches!(err, DagError::CircularDependency { .. }));
}

#[test]
fn test_range_too_large() {
    let mut dag = Dag::with_max_range_cells(4);
    let formula = cell(0, 0, 0);
    let large = range(0, 0, 0, 2, 2); // 9 cells
    let err = dag
        .add_node_input(formula, large, false)
        .expect_err("range too large");
    assert!(matches!(err, DagError::RangeTooLarge { .. }));
}

#[test]
fn test_dirty_nodes_and_take() {
    let mut dag = Dag::new();
    let formula = cell(0, 0, 0);
    let input = cell(0, 1, 0);
    dag.add_node_input(formula.clone(), input, true)
        .expect("add input");

    let dirty = dag.get_dirty_nodes().expect("dirty");
    assert_eq!(dirty.len(), 1);

    let taken = dag.take_dirty_nodes().expect("take dirty");
    assert_eq!(taken.len(), 1);

    let dirty_after = dag.peek_dirty_nodes().expect("dirty empty");
    assert!(dirty_after.is_empty());
}

#[test]
fn test_apply_operations_and_delete() {
    let mut dag = Dag::new();
    let formula = cell(0, 0, 0);
    let input = cell(0, 1, 0);

    let ops = vec![DagOperation::AddInput {
        formula: formula.clone(),
        input: input.clone(),
        mark_as_dirty: false,
    }];
    dag.apply_operations(ops).expect("apply ops");
    assert!(dag.has_node(&formula));

    dag.apply_operations(vec![DagOperation::Delete {
        position: formula.clone(),
        mode: DeleteMode::RemoveNode,
    }])
    .expect("delete");
    assert!(!dag.has_node(&formula));
}

#[test]
fn test_range_nodes_and_array_lookup() {
    let mut dag = Dag::new();
    let formula = cell(0, 1, 0);
    let input_range = range(0, 0, 0, 0, 2);
    dag.add_node_input(formula, input_range.clone(), false)
        .expect("add range");

    let inside = cell(0, 0, 1);
    assert!(dag.has_array_node(&inside));

    let outside = cell(0, 1, 0);
    assert!(!dag.has_array_node(&outside));
}

#[test]
fn test_to_from_json_round_trip() {
    let mut dag = Dag::new();
    let formula = cell(0, 0, 0);
    let input = cell(0, 1, 0);
    let static_ref = NodeRef::Static(StaticReference {
        id: "GLOBAL".to_string(),
    });

    dag.add_node_input(formula.clone(), input.clone(), false)
        .expect("add input");
    dag.add_node_input(formula.clone(), static_ref.clone(), false)
        .expect("add static");

    let json = dag.to_json().expect("to json");
    let nodes: Vec<(String, piptable_dag::DagNodeJson)> =
        serde_json::from_str(&json).expect("parse json");

    let mut restored = Dag::new();
    restored.from_json(nodes).expect("from json");
    assert!(restored.has_node(&formula));
    assert!(restored.has_node(&input));
    assert!(restored.has_node(&static_ref));
}
