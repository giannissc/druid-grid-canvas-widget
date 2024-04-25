use druid::kurbo::{BezPath, Shape};
use std::collections::{HashMap, HashSet};

use super::ids::{CellLibraryId, ShapeId};
use super::{
    common::{Direction, SignalDirection, SignalUse},
    ids::{CellId, CellInstId, LayerId, NetId, PinId, PinInstId},
};

// In ECS the MacroCell would be represented as an entity and the struct properties would be
// the Components. With this mental model MacroPin and MacroCell would share some Componenents (id, name, origin)
// but also have unique Components themselves such as direction and position for Port and bounding box and portID
/**
 * ## Macros/Components
 * - Class (see below)
 * - Origin - if none given it is assumed to be the center of the bounding box
 * - Size
 * - Symmetry (relevant for placement)
 * - Site (see below)
 * - Pins
 * - Obstractions Geometries (similar to layer geometries)
 *
 * ### Class
 * - Cover/Bump (fixed to the floorplan e.g. power routing)
 * - Ring
 * - Block (Hard | Soft)
 * - Pad (Input | OUTPUT |INOUT | POWER | SPACER | AREAIO)
 * - CORE [FEEDTHRU | TIEHIGH | TIELOW | SPACER | ANTENNACELL | WELLTAP]
 * - ENDCAP [PRE | POST | TOPLEFT | TOPRIGHT | BOTTOMLEFT | BOTTOMRIGHT]
 *
 * ### Pins
 * - Direction (INPUT | OUTPUT | INOUT (Power signals) | FEEDTHU)
 * - Use (SIGNAL | ANALOG | POWER | GROUND | CLOCK)
 * - Shape (FEEDTHRU | ABUTMENT (Power signals) | FEEDTHRU)
 * - Port
 *      - Class (NONE | CORE | BUMP)
 *      - Layer Geometry
 */
/**
 * A Cell is defined by an interface (pins) and other interconnected cell instances
 */
pub struct CellLibrary {
    pub id: CellLibraryId,
    pub name: String,
    pub version: f32,
    pub pins: HashMap<PinId, Pin>,
    pub cells: HashMap<CellId, Cell>,
    pub nets: HashMap<NetId, Net>,
    pub site: (f64, f64),
}

#[derive(Debug, Clone)]
pub struct Cell {
    // General
    pub id: CellId,
    pub name: String,
    pub size: (f64, f64),
    pub symmetry: Option<Symmetry>,
    pub class: CellClass,
    // I/O
    pub pins: Vec<PinId>,
    pub instances: HashSet<CellInstId>,
    pub instances_named: HashMap<String, CellInstId>,
    // Connections
    pub nets: HashSet<NetId>,

    // Physical
    pub shapes: HashMap<LayerId, HashMap<ShapeId, BezPath>>,
}
#[derive(Debug, Clone)]
pub enum CellClass {
    Pad,
    Core,
    Block,
    EndCap,
}

#[derive(Debug, Clone)]
pub struct Symmetry {
    pub x: bool,
    pub y: bool,
    pub r90: bool,
}

/**
 * ComponentInst are an instantiation of a Component.
 * The Component holds the shared properties for all ComponentInst.
 * ComponentInst hold properties specific to it and generally useful during placement and routing
 */
pub struct CellInst {
    pub id: CellInstId,
    pub name: String,
    pub source: CellSource,
    pub weight: Option<f64>,
    pub origin: Option<(f64, f64)>,
    pub preferred_origin: Option<(f64, f64)>,
    pub rotation: Option<Direction>,
}

enum CellSource {
    Netlist,
    User,
    Timing,
    Power,
}

pub struct Pin {
    pub id: PinId,
    pub name: String,
    pub position: (f64, f64),
    pub direction: SignalDirection,
}

/**
 * PinInst are an instantiation of a Pin.
 * The Pin holds the shared properties for all PinInst.
 * PinInst hold properties specific to it and generally useful during placement and routing
 */
pub struct PinInst {
    pub id: PinInstId,
    pub name: String,
    pub origin: Option<(f64, f64)>,
}

pub enum CellType {
    Hard,
    Soft,
}

/**
 * Nets represent the connections of a netlist
 * They reference the IDs of both the pins (used for routing) and components (used for placement)
 */
pub struct Net {
    pub name: String,
    pub source: NetSource,
    pub weight: f64,
    pub signal_use: SignalUse,
    pub pins: Vec<PinInstId>,
    pub components: Vec<CellInstId>,
}

pub enum NetSource {
    Netlist,
    User,
    Timing,
    Power,
    Test,
}
