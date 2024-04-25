use std::vec;

use druid::{Point, Rect};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Orientation {
    Vertical,
    Horizontal,
    Diag45,
    Diag135,
}

impl Orientation {
    pub fn get_direction(from: (usize, usize), to: (usize, usize)) -> Self {
        if from.0 == to.0 + 1 || from.0 == to.0 - 1 {
            Self::Horizontal
        } else {
            Self::Vertical
        }
    }
}

pub enum Direction {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}
#[derive(PartialEq)]
pub enum SignalDirection {
    None,
    Input,
    Output,
    InOut,
    Clock,
    Supply,
    Ground,
}

impl SignalDirection {
    pub fn is_input(&self) -> bool {
        self == &SignalDirection::Input
    }

    pub fn is_output(&self) -> bool {
        self == &SignalDirection::Output
    }

    pub fn is_clk(&self) -> bool {
        self == &SignalDirection::Clock
    }

    pub fn is_power(&self) -> bool {
        matches!(self, SignalDirection::Supply | SignalDirection::Ground)
    }
}

pub enum SignalUse {
    Clock,
    Ground,
    Power,
    Reset,
    Signal,
    Analog,
}

pub trait Netlist {
    fn port_direction(&self) -> SignalDirection;
    fn get_net_pins(&self, net: usize) -> Vec<usize>;
}

pub struct RoutingPath {
    pub width: Option<f64>,
}

pub struct Polygon(Vec<Point>);

impl From<Rect> for Polygon {
    fn from(value: Rect) -> Self {
        let point_top_left = Point {
            x: value.x0,
            y: value.y0,
        };
        let point_top_right = Point {
            x: value.x1,
            y: value.y0,
        };
        let point_bottom_right = Point {
            x: value.x1,
            y: value.y1,
        };
        Polygon(vec![point_top_left, point_top_right, point_bottom_right])
    }
}
