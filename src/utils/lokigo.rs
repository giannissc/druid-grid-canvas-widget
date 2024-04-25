use std::collections::HashMap;

use crate::utils::soma::{
    cell_library::Net,
    common::Polygon,
    design::Design,
    ids::{CellId, NetId},
};

/**
 *  Placement
 * */
pub enum PlacementState {
    /// The instance location is fixed and must not be changed by the placement engine. Hard blockage   ``
    Fixed,
    /// The instance can be put into another location by the placement engine
    Moveable,
    /// The instance can be ignored by the placement engine. Soft blockage
    Ignore,
}

pub struct RoutingProblem {
    pub design: Design,
    pub top_cell: CellId,
    // The ID's of the nets to be routed
    pub nets: Vec<NetId>,
    /// All nets start with a default weight of 1.0. With further iterations the relative importance of nets
    /// will be encoded using the net weight. A net weight of 0 signifies that a net is not important for wirelength
    /// A net with a weight > 1.0 is more important for wirelength and will be optimized more aggressively
    pub net_weight: HashMap<NetId, f64>,
    // All routing tracks should be contained within this boundary if specified (relevant for standard cell placement and )
    pub boundary: Option<Polygon>,
    // This is populated during global routing and it used by the detailed router to reduce the scope of the problem.
    pub routing_guides: (),
}
