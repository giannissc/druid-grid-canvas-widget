// Use graph_builder as the foundation.
// Create new Tree and Lattice types and implement Into trait

// Graph → Tree
//  ⤷ Lattice/Mesh/Grid Graph
// Slotmap vs arena

use std::{
    collections::{hash_set::IntoIter, HashSet},
    fmt::Display,
    iter::FusedIterator,
    vec,
};

use bitvec::prelude::*;
use graph_builder::{DirectedCsrGraph, GraphBuilder, UndirectedCsrGraph};

// Used for physical design
// See pathfinding
#[derive(Debug, Clone, Eq)]
pub struct Lattice2D {
    /// Columns
    pub columns: usize,
    /// Rows
    pub rows: usize,
    /// Rectilinear vs Octilinear
    diagonal_mode: bool,
    /// represents gaps in the graph if dense is true and nodes otherwise
    dense: bool,
    /// Tracks present or absent vertices in the graph
    exclusions: HashSet<(usize, usize)>,
}

impl Lattice2D {
    // Constructors
    pub fn new(columns: usize, rows: usize) -> Self {
        Self {
            columns,
            rows,
            diagonal_mode: false,
            dense: false,
            exclusions: HashSet::new(),
        }
    }
    // Builders
    pub fn with_diagonal(mut self) -> Self {
        self.diagonal_mode = true;
        self
    }

    // Setters
    pub fn invert(&mut self) {
        self.dense = !self.dense
    }

    pub fn enable_diagonal(&mut self) {
        self.diagonal_mode = true;
    }

    pub fn disable_diagonal(&mut self) {
        self.diagonal_mode = false;
    }

    // Queries
    #[must_use]
    pub fn size(&self) -> usize {
        self.columns * self.rows
    }

    #[must_use]
    pub fn vertices_len(&self) -> usize {
        if self.dense {
            self.size() - self.exclusions.len()
        } else {
            self.exclusions.len()
        }
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        if self.dense {
            self.exclusions.len() == self.size()
        } else {
            self.exclusions.is_empty()
        }
    }
    #[must_use]
    pub fn is_full(&self) -> bool {
        if self.dense {
            self.exclusions.is_empty()
        } else {
            self.exclusions.len() == self.size()
        }
    }
    #[must_use]
    pub fn is_inside(&self, vertex: (usize, usize)) -> bool {
        vertex.0 < self.columns && vertex.1 < self.rows
    }

    pub fn is_area_obstructed(
        &self,
        from_vertex: (usize, usize),
        to_vertex: (usize, usize),
    ) -> bool {
        let from_column = from_vertex.0;
        let to_column = to_vertex.0;
        let from_row = from_vertex.1;
        let to_row = to_vertex.1;
        (from_column..=to_column)
            .flat_map(move |column| (from_row..=to_row).map(move |row| (column, row)))
            .any(|vertex| self.has_vertex(vertex))
    }
    #[must_use]
    pub fn has_vertex(&self, vertex: (usize, usize)) -> bool {
        self.is_inside(vertex) && (self.exclusions.contains(&vertex) ^ self.dense)
    }
    #[must_use]
    pub fn has_edge(&self, v1: (usize, usize), v2: (usize, usize)) -> bool {
        if !self.has_vertex(v1) || !self.has_vertex(v2) {
            return false;
        }
        let x = v1.0.abs_diff(v2.0);
        let y = v1.1.abs_diff(v2.1);
        x + y == 1 || (x == 1 && y == 1 && self.diagonal_mode)
    }
    #[must_use]
    pub fn to_vertex_index(&self, column: usize, row: usize) -> usize {
        column + row * self.columns
    }
    #[must_use]
    pub fn to_vertex_coords(&self, index: usize) -> (usize, usize) {
        let col = index % self.columns;
        let row = index / self.columns;
        (col, row)
    }

    // Utils
    pub fn area(
        &self,
        from_vertex: (usize, usize),
        to_vertex: (usize, usize),
    ) -> impl Iterator<Item = (usize, usize)> {
        let from_column = from_vertex.0;
        let to_column = to_vertex.0;
        let from_row = from_vertex.1;
        let to_row = to_vertex.1;
        (from_column..=to_column)
            .flat_map(move |column| (from_row..=to_row).map(move |row| (column, row)))
    }

    pub fn perimeter(
        &self,
        from_vertex: (usize, usize),
        to_vertex: (usize, usize),
    ) -> impl Iterator<Item = (usize, usize)> {
        let from_column = from_vertex.0;
        let to_column = to_vertex.0;
        let from_row = from_vertex.1;
        let to_row = to_vertex.1;
        (from_column..=to_column)
            .flat_map(move |column| vec![(column, from_row), (column, to_row)].into_iter())
            .chain(
                // Left and right border
                (from_row + 1..to_row)
                    .flat_map(move |row| vec![(0, row), (to_column, row)].into_iter()),
            )
    }

    pub fn resize(&mut self, column: usize, row: usize) -> bool {
        let mut truncated = false;
        if column < self.columns {
            truncated |=
                (column..self.columns).any(|c| (0..self.rows).any(|r| self.has_vertex((c, r))));
        }
        if row < self.rows {
            truncated |=
                (0..self.columns).any(|c| (row..self.rows).any(|r| self.has_vertex((c, r))));
        }
        self.exclusions.retain(|&(x, y)| x < column && y < row);
        if self.dense {
            for c in self.columns..column {
                for r in 0..row {
                    self.exclusions.insert((c, r));
                }
            }
            for c in 0..self.columns.min(column) {
                for r in self.rows..row {
                    self.exclusions.insert((c, r));
                }
            }
        }
        self.columns = column;
        self.rows = row;
        self.rebalance();
        truncated
    }

    pub fn rebalance(&mut self) {
        if self.exclusions.len() > self.columns * self.rows / 2 {
            self.exclusions = (0..self.columns)
                .flat_map(|column| (0..self.rows).map(move |row| (column, row)))
                .filter(|vertex| !self.exclusions.contains(vertex))
                .collect();
            self.invert();
        }
    }
    #[must_use]
    pub fn neighbours(&self, vertex: (usize, usize)) -> Vec<(usize, usize)> {
        if !self.has_vertex(vertex) {
            return vec![];
        }
        let (x, y) = vertex;
        let mut candidates = Vec::with_capacity(8);
        if x > 0 {
            // Left Neighbour
            candidates.push((x - 1, y));
            if self.diagonal_mode {
                if y > 0 {
                    // Top-Left Neighbour
                    candidates.push((x - 1, y - 1));
                }
                if y + 1 < self.rows {
                    // Bottom-Left Neightbour
                    candidates.push((x - 1, y + 1));
                }
            }
        }

        if x + 1 < self.columns {
            // Right Neighbour
            candidates.push((x + 1, y));
            if self.diagonal_mode {
                if y > 0 {
                    // Top-Right Neighbour
                    candidates.push((x + 1, y - 1));
                }
                if y + 1 < self.rows {
                    // Bottom-Right Neighbour
                    candidates.push((x + 1, y + 1));
                }
            }
        }

        if y > 0 {
            // Top Neighbour
            candidates.push((x, y - 1));
        }

        if y + 1 < self.rows {
            // Bottom Neighbour
            candidates.push((x, y + 1));
        }

        candidates.retain(|&vertex| self.has_vertex(vertex));
        candidates
    }

    // Manipulators
    pub fn add_vertex(&mut self, vertex: (usize, usize)) -> bool {
        if !self.is_inside(vertex) {
            return false;
        }

        let result = if self.dense {
            self.exclusions.remove(&vertex)
        } else {
            self.exclusions.insert(vertex)
        };

        self.rebalance();
        result
    }

    pub fn remove_vertex(&mut self, vertex: (usize, usize)) -> bool {
        if !self.is_inside(vertex) {
            return false;
        }

        let result = if self.dense {
            self.exclusions.insert(vertex)
        } else {
            self.exclusions.remove(&vertex)
        };

        self.rebalance();
        result
    }

    pub fn add_vertex_area(
        &mut self,
        from_vertex: (usize, usize),
        to_vertex: (usize, usize),
    ) -> usize {
        if !self.is_inside(from_vertex)
            || !self.is_inside(to_vertex)
            || self.columns == 0
            || self.rows == 0
        {
            return 0;
        }
        let area = self.area(from_vertex, to_vertex);
        let count = if self.dense {
            area.filter(|vertex| self.exclusions.remove(vertex)).count()
        } else {
            area.filter(|vertex| self.exclusions.insert(*vertex))
                .count()
        };

        self.rebalance();
        count
    }

    pub fn remove_vertex_area(
        &mut self,
        from_vertex: (usize, usize),
        to_vertex: (usize, usize),
    ) -> usize {
        if !self.is_inside(from_vertex)
            || !self.is_inside(to_vertex)
            || self.columns == 0
            || self.rows == 0
        {
            return 0;
        }
        let area = self.area(from_vertex, to_vertex);
        let count = if self.dense {
            area.filter(|vertex| self.exclusions.insert(*vertex))
                .count()
        } else {
            area.filter(|vertex| self.exclusions.remove(vertex)).count()
        };

        self.rebalance();
        count
    }

    pub fn add_vertex_perimeter(
        &mut self,
        from_vertex: (usize, usize),
        to_vertex: (usize, usize),
    ) -> usize {
        if !self.is_inside(from_vertex)
            || !self.is_inside(to_vertex)
            || self.columns == 0
            || self.rows == 0
        {
            return 0;
        }
        let perimeter = self.perimeter(from_vertex, to_vertex);
        let count = if self.dense {
            perimeter
                .filter(|vertex| self.exclusions.remove(vertex))
                .count()
        } else {
            perimeter
                .filter(|vertex| self.exclusions.insert(*vertex))
                .count()
        };

        self.rebalance();
        count
    }

    pub fn remove_vertex_perimeter(
        &mut self,
        from_vertex: (usize, usize),
        to_vertex: (usize, usize),
    ) -> usize {
        if !self.is_inside(from_vertex)
            || !self.is_inside(to_vertex)
            || self.columns == 0
            || self.rows == 0
        {
            return 0;
        }
        let perimeter = self.perimeter(from_vertex, to_vertex);
        let count = if self.dense {
            perimeter
                .filter(|vertex| self.exclusions.insert(*vertex))
                .count()
        } else {
            perimeter
                .filter(|vertex| self.exclusions.remove(vertex))
                .count()
        };

        self.rebalance();
        count
    }

    pub fn add_vertex_vector(&mut self, vector: BitVec) -> usize {
        if vector.len() != self.size() {
            return 0;
        }

        let mut count = 0;
        for (index, bit) in vector.iter().enumerate() {
            let vertex = self.to_vertex_coords(index);
            if self.dense {
                if (!*bit && self.exclusions.insert(vertex))
                    || (*bit && self.exclusions.remove(&vertex))
                {
                    count += 1;
                }
            } else {
                if (*bit && self.exclusions.insert(vertex))
                    || (!*bit && self.exclusions.remove(&vertex))
                {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn remove_vertex_vector(&mut self, vector: BitVec) -> usize {
        if vector.len() != self.size() {
            return 0;
        }

        let mut count = 0;
        for (index, bit) in vector.iter().enumerate() {
            let vertex = self.to_vertex_coords(index);
            if self.dense {
                if (*bit && self.exclusions.insert(vertex))
                    || (!*bit && self.exclusions.remove(&vertex))
                {
                    count += 1;
                }
            } else {
                if (!*bit && self.exclusions.insert(vertex))
                    || (*bit && self.exclusions.remove(&vertex))
                {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn add_border(&mut self) -> usize {
        self.add_vertex_perimeter((0, 0), (self.columns - 1, self.rows - 1))
    }

    pub fn remove_border(&mut self) -> usize {
        self.remove_vertex_perimeter((0, 0), (self.columns - 1, self.rows - 1))
    }

    pub fn clear(&mut self) -> bool {
        let result = !self.is_empty();
        self.dense = false;
        self.exclusions.clear();
        result
    }

    pub fn fill(&mut self) -> bool {
        let result = !self.is_full();
        self.dense = true;
        self.exclusions.clear();
        result
    }

    pub fn as_bitvec(&self) -> BitVec {
        (0..self.columns)
            .flat_map(move |column| (0..self.rows).map(move |row| (column, row)))
            .map(|vertex| self.has_vertex(vertex))
            .collect()
    }
}

impl IntoIterator for Lattice2D {
    type Item = (usize, usize);

    type IntoIter = IntoIter<(usize, usize)>;

    fn into_iter(self) -> Self::IntoIter {
        if self.dense {
            let mut set = HashSet::new();
            for vertex in (0..self.columns)
                .flat_map(move |column| (0..self.rows).map(move |row| (column, row)))
                .filter(|vertex| self.has_vertex(*vertex))
            {
                set.insert(vertex);
            }
            set.into_iter()
        } else {
            self.exclusions.into_iter()
        }
    }
}

impl IntoIterator for &Lattice2D {
    type Item = (usize, usize);

    type IntoIter = IntoIter<(usize, usize)>;

    fn into_iter(self) -> Self::IntoIter {
        if self.dense {
            let mut set = HashSet::new();
            for vertex in (0..self.columns)
                .flat_map(move |column| (0..self.rows).map(move |row| (column, row)))
                .filter(|vertex| self.has_vertex(*vertex))
            {
                set.insert(vertex);
            }
            set.into_iter()
        } else {
            self.exclusions.clone().into_iter()
        }
    }
}

impl PartialEq for Lattice2D {
    fn eq(&self, other: &Self) -> bool {
        self.vertices_len() == other.vertices_len()
            && self.into_iter().zip(other.into_iter()).all(|(a, b)| a == b)
    }
}

impl Into<UndirectedCsrGraph<usize, usize>> for Lattice2D {
    fn into(self) -> UndirectedCsrGraph<usize, usize> {
        let mut edges: HashSet<(usize, usize)> = HashSet::new();
        for column in 0..self.columns {
            // Columns
            for row in 0..self.rows {
                if self.has_vertex((column, row)) {
                    let self_index = self.to_vertex_index(column, row);

                    for (neighbour_col, neighbour_row) in self.neighbours((column, row)) {
                        let neighbour_index = self.to_vertex_index(neighbour_col, neighbour_row);
                        // For DirectedCsrGraph this check should be removed
                        if !edges.contains(&(neighbour_index, self_index)) {
                            edges.insert((self_index, neighbour_index));
                        }
                    }
                }
            }
        }

        GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(edges)
            .node_values(0..self.size())
            .build()
    }
}

impl Display for Lattice2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //General procedure
        // 1. Add \r\n
        // 2. Add column labels
        // 3. Add row labels
        // 4. Add index labels + parenthesis
        // 5. Add connections
        let index_digits = self.size().to_string().len();
        let row_digits = self.rows.to_string().len();
        let index_mid = (index_digits as f32 / 2.0).round() as usize;
        let line_width = row_digits + self.columns * (index_digits + 3) + 1;
        let line_n = self.rows * 2;
        let mut buffer_vec: Vec<u8> = " ".repeat(line_n * line_width).as_bytes().to_vec();
        // Add new line character
        for line in 1..=line_n {
            // buffer_vec[line * (line_width - 2)] = "\r".as_bytes()[0];
            buffer_vec[line * line_width - 1] = "\n".as_bytes()[0];
        }
        // Add column Headers
        let start_offset = row_digits + 2;
        let next_offset = index_digits + 3;
        for column in 0..self.columns {
            // Add all the column labels
            let column_str = column.to_string();
            let digits_budget = index_digits - column_str.len();
            let left_space = (digits_budget as f32 / 2.0).floor() as usize;
            let offset = start_offset + column * next_offset + left_space;
            let bytes = column_str.as_bytes();
            for (i, byte) in bytes.iter().enumerate() {
                buffer_vec[offset + i] = *byte;
            }
        }
        // Row Headers
        for row in 0..self.rows {
            let row_str = row.to_string();
            let digits_budget = row_digits - row_str.len();
            let bytes = row_str.as_bytes();
            for (i, byte) in bytes.iter().enumerate() {
                buffer_vec[line_width + 2 * row * line_width + digits_budget + i] = *byte;
            }
        }
        // Vertices
        let start_offset = line_width + row_digits + 2;
        let next_offset = index_digits + 3;
        for index in 0..self.size() {
            let (col, row) = self.to_vertex_coords(index);
            let index_str = index.to_string();
            let digits_budget = index_digits - index_str.len();
            let left_space: usize = (digits_budget as f32 / 2.0).floor() as usize;
            // println!("start_offset:{start_offset}");
            // println!("next_offset:{next_offset}");
            // println!("line_width:{line_width}");
            // println!("index:({index})");
            // println!("coords:({row},{col})");
            // println!("left_space:{left_space}");
            let offset = start_offset + 2 * row * line_width + col * next_offset + left_space;
            let bytes = index_str.as_bytes();
            for (i, byte) in bytes.iter().enumerate() {
                buffer_vec[offset + i] = *byte;
            }
            // Parentheses
            buffer_vec[offset - 1] = "(".as_bytes()[0];
            buffer_vec[offset + index_digits] = ")".as_bytes()[0];
            // Connections
            for (neighbour_col, neighbour_row) in self.neighbours((col, row)) {
                if neighbour_col > col && neighbour_row == row {
                    // Right neighbour
                    buffer_vec[offset + index_digits + 1] = "-".as_bytes()[0];
                } else if neighbour_row > row && neighbour_col == col {
                    // Bottom neighhbour
                    buffer_vec[offset + line_width + index_mid - 1] = "|".as_bytes()[0];
                } else if neighbour_col > col && neighbour_row > row {
                    // Bottom-right neighbour
                    buffer_vec[offset + line_width + index_digits + 1] = "\\".as_bytes()[0];
                } else if neighbour_col < col && neighbour_row > row {
                    // Bottom-left neighbour
                    if buffer_vec[offset + line_width - 2] == "\\".as_bytes()[0] {
                        buffer_vec[offset + line_width - 2] = "x".as_bytes()[0];
                    } else {
                        buffer_vec[offset + line_width - 2] = "/".as_bytes()[0];
                    }
                }
            }
        }
        write!(f, "\r\n{}", String::from_utf8_lossy(buffer_vec.as_slice()))
    }
}

impl Into<DirectedCsrGraph<usize>> for Lattice2D {
    fn into(self) -> DirectedCsrGraph<usize> {
        let mut edges: HashSet<(usize, usize)> = HashSet::new();
        for column in 0..self.columns {
            // Columns
            for row in 0..self.rows {
                if self.has_vertex((column, row)) {
                    let self_index = column + row * self.columns;

                    for (neighbour_col, neigbhour_row) in self.neighbours((column, row)) {
                        let neighbour_index = neighbour_col + neigbhour_row * self.columns;
                        edges.insert((self_index, neighbour_index));
                    }
                }
            }
        }

        GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(edges)
            .build()
    }
}

// Connected Components (UnionFind)
// See graph, path-finding-lib-rust or petgraph

// Used by netlist (Might be unecessary here)
// See grapes
// pub struct Tree {}

#[cfg(test)]
mod tests {
    use bitvec::prelude::*;
    use std::vec;

    use graph_builder::{Graph, GraphBuilder, UndirectedCsrGraph, UndirectedNeighbors};

    use super::Lattice2D;

    #[test]
    fn to_vertex_index_3x3() {
        let lattice = Lattice2D::new(3, 3);
        let index_list = vec![0, 1, 2, 3, 4, 5, 6, 7, 8];
        let coord_list = vec![
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
        ];
        for ((col, row), expected_index) in coord_list.iter().zip(index_list.iter()) {
            let index = lattice.to_vertex_index(*col, *row);
            assert_eq!(index, *expected_index, "{lattice}");
        }
    }

    #[test]
    fn to_vertex_index_4x3() {
        let lattice = Lattice2D::new(4, 3);
        let index_list = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let coord_list = vec![
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            (3, 2),
        ];
        for ((col, row), expected_index) in coord_list.iter().zip(index_list.iter()) {
            let index = lattice.to_vertex_index(*col, *row);
            assert_eq!(index, *expected_index, "{lattice}");
        }
    }

    #[test]
    fn to_vertex_index_3x4() {
        let lattice = Lattice2D::new(3, 4);
        let index_list = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let coord_list = vec![
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            (0, 3),
            (1, 3),
            (2, 3),
        ];
        for ((col, row), expected_index) in coord_list.iter().zip(index_list.iter()) {
            let index = lattice.to_vertex_index(*col, *row);
            assert_eq!(index, *expected_index, "{lattice}");
        }
    }
    #[test]
    fn to_vertex_coords_3x3() {
        let lattice = Lattice2D::new(3, 3);
        let index_list = vec![0, 1, 2, 3, 4, 5, 6, 7, 8];
        let coord_list = vec![
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
        ];
        for (index, expected_coord) in index_list.iter().zip(coord_list.iter()) {
            let coord = lattice.to_vertex_coords(*index);
            assert_eq!(coord, *expected_coord, "{lattice}");
        }
    }

    #[test]
    fn to_vertex_coords_4x3() {
        let lattice = Lattice2D::new(4, 3);
        let index_list = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let coord_list = vec![
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            (3, 2),
        ];
        for (index, expected_coord) in index_list.iter().zip(coord_list.iter()) {
            let coord = lattice.to_vertex_coords(*index);
            assert_eq!(coord, *expected_coord, "{lattice}");
        }
    }

    #[test]
    fn to_vertex_coords_3x4() {
        let lattice = Lattice2D::new(3, 4);
        let index_list = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let coord_list = vec![
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            (0, 3),
            (1, 3),
            (2, 3),
        ];
        for (index, expected_coord) in index_list.iter().zip(coord_list.iter()) {
            let coord = lattice.to_vertex_coords(*index);
            assert_eq!(coord, *expected_coord, "{lattice}");
        }
    }

    #[test]
    fn display_grid_3x3() {
        let mut lattice = Lattice2D::new(3, 3).with_diagonal();
        lattice.fill();
        lattice.remove_vertex((1, 2));
        let expected_str = "\r\n   0   1   2 \n0 (0)-(1)-(2)\n   | x | x | \n1 (3)-(4)-(5)\n   | /   \\ | \n2 (6) (7) (8)\n";

        assert_eq!(format!("{lattice}"), expected_str, "{lattice}");
    }

    #[test]
    fn display_grid_4x3() {
        let mut lattice = Lattice2D::new(4, 3).with_diagonal();
        lattice.fill();
        lattice.remove_vertex((1, 2));
        let expected_str = "\r\n   0    1    2    3  \n0 (0 )-(1 )-(2 )-(3 )\n   |  x |  x |  x |  \n1 (4 )-(5 )-(6 )-(7 )\n   |  /    \\ |  x |  \n2 (8 ) (9 ) (10)-(11)\n";

        assert_eq!(format!("{lattice}"), expected_str, "{lattice}");
    }

    #[test]
    fn display_grid_3x4() {
        let mut lattice = Lattice2D::new(3, 4).with_diagonal();
        lattice.fill();
        lattice.remove_vertex((1, 2));
        let expected_str = "\r\n   0    1    2  \n0 (0 )-(1 )-(2 )\n   |  x |  x |  \n1 (3 )-(4 )-(5 )\n   |  /    \\ |  \n2 (6 ) (7 ) (8 )\n   |  \\    / |  \n3 (9 )-(10)-(11)\n";

        assert_eq!(format!("{lattice}"), expected_str, "{lattice}");
    }

    #[test]
    fn rectilinear_neighbours() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.clear();
        lattice.add_vertex((1, 1));
        lattice.add_vertex((2, 1));
        lattice.add_vertex((3, 1));
        lattice.add_vertex((1, 2));
        lattice.add_vertex((2, 2));
        lattice.add_vertex((3, 2));
        lattice.add_vertex((1, 3));
        lattice.add_vertex((2, 3));
        lattice.add_vertex((3, 3));

        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![
                (6, 7),
                (6, 11),
                (7, 8),
                (7, 12),
                (8, 13),
                (11, 12),
                (11, 16),
                (12, 13),
                (12, 17),
                (13, 18),
                (16, 17),
                (17, 18),
            ])
            .node_values(0..size)
            .build();

        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node: {node}{lattice}"
            );
        }
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }

    #[test]
    fn octilinear_neighbours() {
        let mut lattice = Lattice2D::new(5, 5).with_diagonal();
        let size = lattice.size();
        lattice.clear();
        lattice.add_vertex((1, 1));
        lattice.add_vertex((2, 1));
        lattice.add_vertex((3, 1));
        lattice.add_vertex((1, 2));
        lattice.add_vertex((2, 2));
        lattice.add_vertex((3, 2));
        lattice.add_vertex((1, 3));
        lattice.add_vertex((2, 3));
        lattice.add_vertex((3, 3));

        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![
                (6, 7),
                (6, 11),
                (6, 12),
                (7, 8),
                (7, 12),
                (7, 11),
                (7, 13),
                (8, 12),
                (8, 13),
                (11, 12),
                (11, 16),
                (11, 17),
                (12, 13),
                (12, 16),
                (12, 17),
                (12, 18),
                (13, 17),
                (13, 18),
                (16, 17),
                (17, 18),
            ])
            .node_values(0..size)
            .build();

        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node: {node}{lattice}"
            );
        }
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }

    #[test]
    fn fill() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.fill();
        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![
                (0, 1),
                (0, 5),
                (1, 2),
                (1, 6),
                (2, 3),
                (2, 7),
                (3, 4),
                (3, 8),
                (4, 9),
                (5, 6),
                (5, 10),
                (6, 7),
                (6, 11),
                (7, 8),
                (7, 12),
                (8, 9),
                (8, 13),
                (9, 14),
                (10, 11),
                (10, 15),
                (11, 12),
                (11, 16),
                (12, 13),
                (12, 17),
                (13, 14),
                (13, 18),
                (14, 19),
                (15, 16),
                (15, 20),
                (16, 17),
                (16, 21),
                (17, 18),
                (17, 22),
                (18, 19),
                (18, 23),
                (19, 24),
                (20, 21),
                (21, 22),
                (22, 23),
                (23, 24),
            ])
            .node_values(0..size)
            .build();

        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
        assert_eq!(result_graph.node_count(), expected_graph.node_count());

        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node:{node}{lattice}"
            );
        }
    }

    #[test]
    fn empty() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.clear();
        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![])
            .node_values(0..size)
            .build();
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }
    #[test]
    fn add_vertex_single() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.clear();
        lattice.add_vertex((2, 2));
        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![])
            .node_values(0..size)
            .build();

        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node: {node}{lattice}"
            );
        }
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }

    #[test]
    fn remove_vertex_single() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.fill();
        lattice.remove_vertex((2, 2));
        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![
                (0, 1),
                (0, 5),
                (1, 2),
                (1, 6),
                (2, 3),
                (2, 7),
                (3, 4),
                (3, 8),
                (4, 9),
                (5, 6),
                (5, 10),
                (6, 7),
                (6, 11),
                (7, 8),
                (8, 9),
                (8, 13),
                (9, 14),
                (10, 11),
                (10, 15),
                (11, 16),
                (13, 14),
                (13, 18),
                (14, 19),
                (15, 16),
                (15, 20),
                (16, 17),
                (16, 21),
                (17, 18),
                (17, 22),
                (18, 19),
                (18, 23),
                (19, 24),
                (20, 21),
                (21, 22),
                (22, 23),
                (23, 24),
            ])
            .node_values(0..size)
            .build();

        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node: {node}{lattice}"
            );
        }
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }

    #[test]
    fn add_vertex_area() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.clear();
        lattice.add_vertex_area((1, 1), (3, 3));
        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![
                (6, 7),
                (6, 11),
                (7, 8),
                (7, 12),
                (8, 13),
                (11, 12),
                (11, 16),
                (12, 13),
                (12, 17),
                (13, 18),
                (16, 17),
                (17, 18),
            ])
            .node_values(0..size)
            .build();
        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node: {node}{lattice}"
            );
        }
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }

    #[test]
    fn remove_vertex_area() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.fill();
        lattice.remove_vertex_area((1, 1), (3, 3));
        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![
                (0, 1),
                (0, 5),
                (1, 2),
                (2, 3),
                (3, 4),
                (4, 9),
                (5, 10),
                (9, 14),
                (10, 15),
                (14, 19),
                (15, 20),
                (19, 24),
                (20, 21),
                (21, 22),
                (22, 23),
                (23, 24),
            ])
            .node_values(0..size)
            .build();
        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node: {node}{lattice}"
            );
        }
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }

    #[test]
    fn add_vertex_vector() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.clear();
        lattice.add_vertex_vector(bitvec![
            1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0,
        ]);
        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![
                (0, 1),
                (0, 5),
                (1, 2),
                (1, 6),
                (2, 3),
                (2, 7),
                (3, 4),
                (3, 8),
                (5, 6),
                (5, 10),
                (6, 7),
                (6, 11),
                (7, 8),
                (7, 12),
                (10, 11),
                (10, 15),
                (11, 12),
                (11, 16),
                (15, 16),
                (15, 20),
            ])
            .node_values(0..size)
            .build();

        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node:{node}{lattice}"
            );
        }
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }

    #[test]
    fn remove_vertex_vector() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.fill();
        lattice.remove_vertex_vector(bitvec![
            1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0,
        ]);
        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![
                (9, 14),
                (13, 14),
                (13, 18),
                (14, 19),
                (17, 18),
                (17, 22),
                (18, 19),
                (18, 23),
                (19, 24),
                (21, 22),
                (22, 23),
                (23, 24),
            ])
            .node_values(0..size)
            .build();

        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node:{node}{lattice}"
            );
        }
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }
    #[test]
    fn as_bitvec(){
                let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.clear();
        let expected_bitvec = bitvec![1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0,];
        lattice.add_vertex_vector(expected_bitvec.clone());
        let result_bitvec = lattice.clone().as_bitvec();
        assert_eq!(expected_bitvec, result_bitvec);
    }
    #[test]
    fn add_border() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.clear();
        lattice.add_border();
        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![
                (0, 1),
                (0, 5),
                (1, 2),
                (2, 3),
                (3, 4),
                (4, 9),
                (5, 10),
                (9, 14),
                (10, 15),
                (14, 19),
                (15, 20),
                (19, 24),
                (20, 21),
                (21, 22),
                (22, 23),
                (23, 24),
            ])
            .node_values(0..size)
            .build();
        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node: {node}{lattice}"
            );
        }
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }

    #[test]
    fn remove_border() {
        let mut lattice = Lattice2D::new(5, 5);
        let size = lattice.size();
        lattice.fill();
        lattice.remove_border();
        let result_graph: UndirectedCsrGraph<usize, usize> = lattice.clone().into();
        let expected_graph: UndirectedCsrGraph<usize, usize> = GraphBuilder::new()
            .csr_layout(graph_builder::CsrLayout::Sorted)
            .edges(vec![
                (6, 7),
                (6, 11),
                (7, 8),
                (7, 12),
                (8, 13),
                (11, 12),
                (11, 16),
                (12, 13),
                (12, 17),
                (13, 18),
                (16, 17),
                (17, 18),
            ])
            .node_values(0..size)
            .build();
        for node in 0..size {
            let result_neighbours: Vec<&usize> = result_graph.neighbors(node).collect();
            let expected_neighbours: Vec<&usize> = expected_graph.neighbors(node).collect();
            assert_eq!(
                result_neighbours, expected_neighbours,
                "node: {node}{lattice}"
            );
        }
        assert_eq!(result_graph.node_count(), expected_graph.node_count());
        assert_eq!(result_graph.edge_count(), expected_graph.edge_count());
    }
}
