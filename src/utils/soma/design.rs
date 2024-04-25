use std::collections::HashMap;

use druid::kurbo::{BezPath, Shape};

use super::cell_library::{CellInst, Net, PinInst};
use super::ids::*;
use super::technology::Layer;
/**
 * Design
 *
 * The design contains information to represent the abstract and physical realicaiton of a design.
 * A design file has detailed information about the physical placement and connectiveity of all it components and other structures in the design
 * It contains:
 * - Design Name
 * - Die area
 * - Nets (Name, Pins/Terminals, Weight, Source, Use)
 * - Blockages/Obstraction
 * - Slots
 * - Rows
 * - Regions
 * - Tracks
 * - GCell Grid (Rows, Columns, Spacing and Origin)
 * Vias
 */

pub struct Design {
    pub design_name: String,
    pub version: f32,
    /// Design Libraries
    pub technology_library: TechnologyLibraryId,
    pub cell_library: CellLibraryId,
    /// Logical Design
    pub components: HashMap<CellInstId, CellInst>,
    pub pins: HashMap<PinInstId, PinInst>,
    pub nets: HashMap<NetId, Net>,
    /// Floorplanning
    pub regions: Vec<()>,

    /// Routing Related
    gcell_grid: f64,

    /// Physical Design
    layers: HashMap<LayerId, Layer>,
    shapes: HashMap<ShapeId, BezPath>,
    design_area: Option<(f64, f64)>,
}
