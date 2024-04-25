use super::common::{Orientation, SignalDirection};
use super::ids::{LayerId, TechnologyLibraryId};
use super::units::*;
/**
 * # Technology and Component Libraries
 *
 * The technology library will have information about the library version, units, manufacturing grid, site and layers information.
 * In our case the:
 * - units represents
 * - manufacturing grid represents the resolution of the machine,
 * - site information represents the minimum width and height a standard cell area can occupy in the core area. All the cell in the core are will be multiples of standard cell sites.
 * The site information is relevant only for placement and used in the rows statement in the def file.
 * - layers information represents the work envelope of the machine and the thickness of the acrylic
 *
 * The component library contains an abstract view of each and every standard cell.
 * The component library contains basic information like name, class, site, origin, size, symmetry and pin information
 * The pins further specify their name, direction, shape and layer location.
 * A cell library contains the macro and standard cell information for a design.
 * That includes information about layer, vias, placement site sytpe and macro cell definitions.
 *
 *
 * ## Manufacturing Grid
 * Used for geoemetry alignments of shapes and cells. Defined in microns (millimeters for fluidics).
 *
 * ## Units
 * - Time
 * - Distance
 * - Power
 * - Current/Flow rate
 * - Voltage/Pressure
 * - Capacitance
 * - Resistance
 * - Frequency
 *
 * ## Layers
 * - Minimum Width (of paths)
 * - Preferred Direction
 * - Pitch (distance between routing tracks)
 * - Spacing
 * - Shape
 * - Thickness
 *
 * ### Types
 * - Cut (Membrane)
 * - Routing (Acrylic)
 * - Implant
 * - Overlap
 *
 *
 * ### Placement Sites
 * - Class (PAD | CORE)
 * - Symmetry
 * - Row Pattern
 * - Size
 * Normal row-based standard cells only have a single site without a pattern
 * The pattern indicates that the cell is a gate-array cell rather than a row-based standard cell.
 * Specified in the local coordinates of a component
 *
 *
 */
use std::collections::HashMap;
pub struct TechnologyLibrary {
    pub id: TechnologyLibraryId,
    pub name: String,
    pub version: f32,
    pub units: Units,
    pub layers: Vec<Layer>,
    pub rules: DesignRules,
}

pub struct Units {
    pub time: Time,
    pub distance: Distance,
    pub power: Power,
    pub effort: Effort,
    pub flow: Flow,
    pub resistance: Resistance,
    pub capacitance: Capacitance,
}

pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub index: usize,
    pub layer_type: LayerType,
    pub width: Option<f64>,
    pub pitch: Option<f64>,
    pub orientation: Option<Orientation>,
}

pub enum LayerType {
    /// Via Layer
    Cut,
    /// Routing layer
    Routing,
}

pub struct DesignRules {
    pub max_area: (f64, f64),
    pub max_resolution: f64,
    /// Separation rules
    pub minimum_spacing: f64,
    /// Size rules
    pub minimum_width: f64,
    /// Overlap rules
    pub minimum_overlap: f64,
    pub minimum_area: f64,
    pub minimum_aspect_ratio: f64,
}
