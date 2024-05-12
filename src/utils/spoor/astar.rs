use std::collections::{BTreeSet, HashSet};

use graph_builder::UndirectedNeighborsWithValues;

use crate::utils::{cassetta::TapeItem, graphema::Lattice2D, soma::common::Orientation};

use super::core::{Net, NodeType, PathHeuristic, PathNode, ShortestPath};

pub struct Astar {
    unresolved_nodes: BTreeSet<PathNode>,
    resolved_nodes: HashSet<PathNode>,
    path_nodes: HashSet<PathNode>,
    distance_heuristic: PathHeuristic,
    previous_orientation: Option<Orientation>,
    previous_position: Option<(usize, usize)>,
}

impl Astar {
    pub fn new() -> Self {
        Self {
            unresolved_nodes: BTreeSet::new(),
            resolved_nodes: HashSet::new(),
            path_nodes: HashSet::new(),
            distance_heuristic: PathHeuristic::Manhattan,
            previous_orientation: None,
            previous_position: None,
        }
    }
}

impl ShortestPath for Astar {
    fn compute(
        &mut self,
        config: super::core::ShortestPathConfig,
        source: usize,
    ) -> Vec<TapeItem<(usize, usize), NodeType<Net>>> where {
        // Reset state
        self.unresolved_nodes.clear();
        self.resolved_nodes.clear();
        self.path_nodes.clear();
        let lattice = Lattice2D::new(config.boundary.0, config.boundary.1);
        let tape = Vec::new();
        if let Some(target_index) = config.goal {
            let from = lattice.to_vertex_coords(source);
            let to = lattice.to_vertex_coords(target_index);

            let path_node = PathNode::new(from, 0, to, self.distance_heuristic, 0);
            self.unresolved_nodes.insert(path_node); // Add source node to set
                                                     // While there are values in the unresolved set get the node with the lowest cost
            while let Some(node) = self.get_next_unresolved() {
                // Move the node from the unresolved to the resolved set
                let node_index = lattice.to_vertex_index(node.position.0, node.position.1);
                self.resolved_nodes.insert(node);
                for neighbour in config.graph.neighbors_with_values(node_index) {
                    let neighbour_pos = lattice.to_vertex_coords(neighbour.target);
                    // If the neighbour orientation from its parent is the same as the previous opientation don't increase the cost
                    let mut orientation_cost = node.orientation_cost;
                    if let Some(orientation) = self.previous_orientation {
                        if Orientation::get_direction(node.position, neighbour_pos) != orientation {
                            orientation_cost += 1;
                        }
                    }
                    let neighbour_node = PathNode::new(
                        neighbour_pos,
                        node.cost_from_start,
                        to,
                        self.distance_heuristic,
                        orientation_cost,
                    );
                    let other_node = self.resolved_nodes.remove(&neighbour_node);
                    // If a neighbour is the target node stop
                    if neighbour_pos == to {
                        self.resolved_nodes.insert(neighbour_node);
                        return tape;
                    }
                    // Record each neighbour that has not been encountered before or that has a lower cost than any previous one
                }

                return tape;
            }

            return Vec::new();
        }

        Vec::new()
    }

    fn reconstruct_path(&mut self) -> Vec<TapeItem<(usize, usize), NodeType<Net>>> {
        todo!()
    }

    fn get_next_unresolved(&mut self) -> Option<PathNode> {
        let node = self.unresolved_nodes.pop_first();
        if let Some(to) = node {
            if let Some(from) = self.previous_position {
                // This is executed the second time the function is called.
                self.previous_orientation = Some(Orientation::get_direction(from, to.position));
            }
            // This is obviously executed the first time the function is called (with the source node)
            self.previous_position = Some(to.position);
        }
        node
    }

    fn get_next_path_node(&self) -> Option<PathNode> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use bitvec::prelude::*;

    #[test]
    fn orientation_bias() {
        let test_graph = bitvec![
            1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0,
            1, 0, 0, 0, 0, 1, 1,
        ];
    }

    #[test]
    fn correct_cost() {
        let test_graph = bitvec![
            1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 0,
            0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1,
        ];
        let cost_map = bitvec![
            5, 6, 7, 8, 9, 0, 1, 1, 1, 4, 0, 1, 1, 10, 1, 1, 0, 1, 3, 0, 1, 1, 11, 1, 1, 0, 1, 2,
            0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1,
        ];
    }
}
