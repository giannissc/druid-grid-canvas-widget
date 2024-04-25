use std::{cmp::Ordering, collections::HashMap, hash::{Hash, Hasher}, fmt::Debug};

use graph_builder::{index::Idx, UndirectedCsrGraph};

use crate::utils::cassetta::TapeItem;

/// Routing
/// Convert to builder pattern
/// In the default case only the graph and source node are needed and the target and edge cost function can be added iteratively
pub trait ShortestPath {
    fn compute(&mut self, config:ShortestPathConfig, source: usize) -> Vec<TapeItem<(usize, usize), NodeType<Net>>>;
    fn reconstruct_path(&mut self) -> Vec<TapeItem<(usize, usize),NodeType<Net>>>;
    fn get_next_unresolved(&mut self) -> Option<PathNode>;
    fn get_next_path_node(&self) -> Option<PathNode>;
}

pub struct ShortestPathConfig
{
    pub graph: UndirectedCsrGraph<usize, usize>,
    pub goal: Option<usize>,
    pub boundary: (usize, usize),
}

pub struct ShortestPathAlgo
{
    config: ShortestPathConfig,
    algo_map: HashMap<String, Box<dyn ShortestPath>>,
    algo: Box<dyn ShortestPath>,
}

// A*
// Pattern Routing
//
pub struct ShortestTreeConfig<NI, NV, EV>
where
    NI: Idx,
{
    graph: UndirectedCsrGraph<NI, NV, EV>,
    edge_weight: dyn FnMut(EV) -> f32,
}

pub trait ShortestTree<K, V, NI, NV, EV>
where
    NI: Idx,
    K: Clone + Debug + Hash + Eq,
{
    fn compute_tree(graph: UndirectedCsrGraph<NI, NV, EV>, netlist: Vec<NI>)
        -> Vec<TapeItem<K, V>>;
}

// Physarum
//

//////////////////////////////////////////////////////////////////////////////////////
//
// DistanceHeuristic
//
//////////////////////////////////////////////////////////////////////////////////////
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PathHeuristic {
    Manhattan,
    Euclidean,
    Octile,
    Chebyshev,
    Zero,
}

impl PathHeuristic {
    pub fn cost_estimate(&self, from: (usize, usize), to: (usize, usize)) -> usize {
        let from_col = from.0 as isize;
        let from_row = from.1 as isize;

        let to_col = to.0 as isize;
        let to_row= to.1 as isize;

        match self {
            PathHeuristic::Manhattan => {((from_col - to_col).abs() + (from_row - to_row).abs()) as usize} ,
            PathHeuristic::Euclidean => {((from_col - to_col).abs().pow(2) + (from_row - to_row).abs().pow(2)) as usize},
            PathHeuristic::Octile => {((from_col - to_col).abs().max((from_row - to_row).abs())) as usize},
            PathHeuristic::Chebyshev => {((from_col - to_col).abs().max((from_row - to_row).abs())) as usize},
            PathHeuristic::Zero => {0},
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////
//
// PathNodes
//
//////////////////////////////////////////////////////////////////////////////////////
#[derive(Copy, Clone, Debug, Eq)]
pub struct PathNode {
    pub position: (usize, usize),
    pub cost_from_start: usize,
    pub cost_to_target: Option<usize>,
    pub cost_total: usize,
    pub orientation_cost: usize,
}

impl PathNode {
    pub fn new(from: (usize, usize), cost_from_start: usize,  to: (usize, usize), distance_heuristic: PathHeuristic, orientation_cost: usize) -> Self {
        let cost_to_target = distance_heuristic.cost_estimate(from, to);
        let cost_ord = cost_from_start + cost_to_target;
        Self {
            position: from,
            cost_from_start,
            cost_to_target: Some(cost_to_target),
            cost_total: cost_ord,
            orientation_cost,
        }
    }
    pub fn base(from: (usize, usize)) -> Self {
        PathNode {
            position: from,
            cost_from_start: 0,
            cost_to_target: None,
            cost_total: 0,
            orientation_cost: 0,
        }
    }

    pub fn with_start_cost(mut self, cost_start: usize) -> Self {
        self.cost_from_start = cost_start;
        self
    }

    pub fn with_cost_estimation(mut self, to: (usize, usize), distance_heuristic: PathHeuristic) -> Self {
        let target_cost = distance_heuristic.cost_estimate(self.position, to);
        self.cost_to_target = Some(target_cost);
        self.cost_total = self.cost_from_start + target_cost;
        self
    }

    pub fn with_orientation_cost(mut self, orientation_cost: usize) -> Self {
        self.orientation_cost = orientation_cost;
        self
    }

}

impl PartialEq for PathNode {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

impl Hash for PathNode {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.position.hash(hasher);
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let order_option = self.cost_total.partial_cmp(&other.cost_total);
        match order_option{
            Some(order) => {
                if let Ordering::Equal = order {
                    if self.orientation_cost > other.orientation_cost {
                        Some(Ordering::Greater)
                    } else if self.orientation_cost < other.orientation_cost {
                        Some(Ordering::Less)
                    } else {
                        Some(Ordering::Equal)
                    }
                } else {
                    Some(order)
                }
            },
            None => return None,
        }
    }
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        let order  = self.cost_total.cmp(&other.cost_total);

        if let Ordering::Equal = order {
            if self.orientation_cost > other.orientation_cost {
                Ordering::Greater
            } else if self.orientation_cost < other.orientation_cost {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        } else {
            order
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////
//
// GridNodeType
//
//////////////////////////////////////////////////////////////////////////////////////
// Add wight and bomb nodes?
pub type Net = usize;
pub type Cost = usize;
//type Weight = i32;
#[derive(Copy, Clone, Debug, Hash)]
pub enum NodeType<Net> {
    Obstacle,
    Boundary,
    Start(Net),
    Target(Net),
    //SteinerNode(Net),
    Unresolved(Cost),
    Resolved(Cost), 
    Route(Net, Cost),
}

impl NodeType<Net> {
    pub fn get_net(&self) -> Option<&Net>{
        match self{
            Self::Start(net) => Some(net),
            Self::Target(net) => Some(net),
            Self::Route(net, _) => Some(net),
            _ => None,
        }
    }

    pub fn get_cost(&self) -> Option<&Cost> {
        match  self {
            Self::Unresolved(cost) => Some(cost),
            Self::Resolved(cost) => Some(cost),
            Self::Route(_, cost) => Some(cost),
            _ => None,
            
        }
    }
}