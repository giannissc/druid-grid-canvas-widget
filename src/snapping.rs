///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// Imports
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
use druid::kurbo::Circle;
use druid::widget::{Painter,};
use druid::{ Point, theme, RenderContext, Size, Rect, Data, Lens,};
use druid_color_thesaurus::{gray};

use crate::panning::{PanData, PanDataAccess, };
use crate::zooming::{ZoomData, ZoomDataAccess,};

///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// GridSnapData
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
pub trait GridSnapDataAccess: PanDataAccess + ZoomDataAccess {
    fn get_cell_size(&self) -> f64;
    fn set_cell_size(&mut self, size: f64);
    fn get_grid_visibility(&self) -> bool;
    fn set_grid_visibility(&mut self, state: bool);
    fn move_to_grid_position(&self, desired_position: Point) -> Point;
}

#[derive(Clone, Data, Lens, PartialEq)]
pub struct GridSnapData {
    pub cell_size: f64,
    pub grid_visibility: bool,
    pub zoom_data: ZoomData,
    pub pan_data: PanData,
}

impl GridSnapData {
    pub fn new(cell_size: f64) -> Self {
        Self{
            cell_size,
            grid_visibility: true,
            zoom_data: ZoomData::new(),
            pan_data: PanData::new(),
        }
    }
    pub fn move_to_grid_position_2(&self, desired_position: Point) -> Point {
        let (row, col) = self.get_grid_index(desired_position);
        self.get_grid_position(row, col)
    }

    pub fn get_grid_index(&self, position: Point) -> (isize, isize) {
        // Normalise translation offset
        let mut position_norm = position; 
        position_norm.x -= self.pan_data.absolute_offset.x;
        position_norm.y -= self.pan_data.absolute_offset.y;

        let scaled_cell_size = self.cell_size * self.zoom_data.zoom_scale;

        let row = (position_norm.y / scaled_cell_size).floor() as isize;
        let col = (position_norm.x / scaled_cell_size).floor() as isize;

        (row, col)
    }

    pub fn get_grid_position(&self, row: isize, col:isize) -> Point {
        let scaled_cell_size = self.cell_size * self.zoom_data.zoom_scale;

        Point { 
            x: col as f64 * scaled_cell_size + self.pan_data.absolute_offset.x, 
            y: row as f64 * scaled_cell_size + self.pan_data.absolute_offset.y, 
        }
    }
}

impl GridSnapDataAccess for GridSnapData {
    fn get_cell_size(&self) -> f64 {
        self.cell_size
    }

    fn set_cell_size(&mut self, size: f64) {
        self.cell_size = size;
    }

    fn get_grid_visibility(&self) -> bool {
        self.grid_visibility
    }

    fn set_grid_visibility(&mut self, state: bool) {
        self.grid_visibility = state;
    }

    fn move_to_grid_position(&self, desired_position: Point) -> Point {
        self.move_to_grid_position_2(desired_position)
    }
}

impl ZoomDataAccess for GridSnapData {
    fn get_zoom_scale(&self) -> f64 {
        self.zoom_data.zoom_scale
    }

    fn set_zoom_scale(&mut self, scale: f64) {
        self.zoom_data.zoom_scale = scale;
    }
}

impl PanDataAccess for GridSnapData {
    fn get_absolute_offset(&self) -> Point {
        self.pan_data.absolute_offset
    }

    fn set_absolute_offset(&mut self, offset: Point) {
        self.pan_data.absolute_offset = offset;
    }

    fn get_relative_offset(&self) -> druid::Vec2 {
        self.pan_data.relative_offset
    }

    fn set_relative_offset(&mut self, offset: druid::Vec2) {
        self.pan_data.relative_offset = offset;
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
/// 
/// GridSnapPainter
/// 
///////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Copy, Clone)]
pub struct GridSnapPainter{
    show_origin: bool,
    debug_offset: bool,
}

impl Default for GridSnapPainter {
    fn default() -> Self {
        Self { 
            show_origin: true, 
            debug_offset: true,
        }
    }
}

impl GridSnapPainter {
    pub fn square_grid<T: Data + GridSnapDataAccess>(&self) -> Painter<T> {
        let origin_visibility = self.show_origin;
        let debug_visibility = self.debug_offset;

        Painter::new(move |ctx, data: &T, _env| {
            // let scaled_cell_size = data.cell_size * data.zoom_data.zoom_scale;
            let scaled_cell_size = data.get_cell_size() * data.get_zoom_scale();
            let line_width = scaled_cell_size * 0.05;
            
            // Partial Paint Setup
            let screen_space = ctx.size();
            let damage_region  = ctx.region();
            let invalidation_rect = damage_region.bounding_box();

            // Background Painting Logic
            let rect = screen_space.to_rect();
            ctx.fill(rect, &gray::OUTER_SPACE);

            // Axes Painting Logic
            if data.get_grid_visibility() {

                let start_point = data.move_to_grid_position(invalidation_rect.origin());
                let end_point = data.move_to_grid_position(Point {
                    x: invalidation_rect.max_x(),
                    y: invalidation_rect.max_y(),
                });
                
                let from_row = (start_point.y / scaled_cell_size).floor() as usize;
                let from_col = (start_point.x /scaled_cell_size).floor() as usize;

                let to_row = (end_point.y / scaled_cell_size).ceil() as usize + 1;
                let to_col = (end_point.x / scaled_cell_size).ceil() as usize + 1;

                for row in from_row..= to_row {
                    let mut from_point = Point::new(0.0, scaled_cell_size * row as f64 - line_width / 2.0 );
                    // Integrate translation data to line rendering
                    // from_point.y += data.pan_data.absolute_offset.y % scaled_cell_size;
                    from_point.y += data.get_absolute_offset().y % scaled_cell_size;
                    let size = Size::new(ctx.size().width, line_width);
                    let rect = Rect::from_origin_size(from_point, size);
                    ctx.fill(rect, &gray::GAINSBORO)
                }

                for col in from_col..=to_col {
                    let mut from_point = Point::new(scaled_cell_size * col as f64 - line_width / 2.0, 0.0);
                    // Integrate translation data to line rendering
                    // from_point.x += data.pan_data.absolute_offset.x % scaled_cell_size;
                    from_point.x += data.get_absolute_offset().x % scaled_cell_size;
                    let size = Size::new(line_width, ctx.size().width);
                    let rect = Rect::from_origin_size(from_point, size);
                    ctx.fill(rect, &gray::GAINSBORO)
                }
            }

            if origin_visibility {
                // let center = Point::new(data.pan_data.absolute_offset.x, data.pan_data.absolute_offset.y);
                let center = Point::new(data.get_absolute_offset().x, data.get_absolute_offset().y);
                let circle = Circle::new(center, 5.0);
                ctx.fill(circle, &druid_color_thesaurus::red::CARMINE);
            }

            if debug_visibility {
                // let center = Point::new(data.pan_data.absolute_offset.x % scaled_cell_size,data.pan_data.absolute_offset.y % scaled_cell_size);
                let center = Point::new(data.get_absolute_offset().x % scaled_cell_size,data.get_absolute_offset().y % scaled_cell_size);
                let circle = Circle::new(center, 5.0);
                ctx.fill(circle, &druid_color_thesaurus::pink::CORAL_PINK);

            }
        })
    }

    pub fn dot_grid(&self) -> Painter<GridSnapData> {
        let origin_visibility = self.show_origin;
        let debug_visibility = self.debug_offset;

        Painter::new(move |ctx, data: &GridSnapData, env| {
            let scaled_cell_size = data.cell_size * data.zoom_data.zoom_scale;
            let line_width = scaled_cell_size * 0.05;
            
            // Partial Paint Setup
            let screen_space = ctx.size();
            let damage_region  = ctx.region();
            let invalidation_rect = damage_region.bounding_box();

            // Background Painting Logic
            let rect = screen_space.to_rect();
            ctx.fill(rect, &gray::MARENGO);

            if data.grid_visibility {
                let start_point = data.move_to_grid_position(invalidation_rect.origin());
                let end_point = data.move_to_grid_position(Point {
                    x: invalidation_rect.max_x(),
                    y: invalidation_rect.max_y(),
                });
                
                let from_row = (start_point.y / scaled_cell_size).floor() as usize;
                let from_col = (start_point.x /scaled_cell_size).floor() as usize;

                let to_row = (end_point.y / scaled_cell_size).ceil() as usize + 1;
                let to_col = (end_point.x / scaled_cell_size).ceil() as usize + 1;

                for row in from_row..= to_row {
                    for col in from_col..=to_col {
                        let mut center = Point::new(
                            scaled_cell_size * col as f64, 
                            scaled_cell_size * row as f64, 
                        );

                        // Zoom UI functionality
                        center.x += data.pan_data.absolute_offset.x % scaled_cell_size;
                        center.y += data.pan_data.absolute_offset.y % scaled_cell_size;

                        let circle = Circle::new(center, line_width);
                        ctx.fill(circle, &env.get(theme::BORDER_LIGHT));
                    }
                }
            }

            if origin_visibility {
                let center = Point::new(data.pan_data.absolute_offset.x, data.pan_data.absolute_offset.y);
                let circle = Circle::new(center, 5.0);
                ctx.fill(circle, &druid_color_thesaurus::red::CARMINE);
            }

            if debug_visibility {
                let center = Point::new(data.pan_data.absolute_offset.x % scaled_cell_size,data.pan_data.absolute_offset.y % scaled_cell_size);
                let circle = Circle::new(center, 5.0);
                ctx.fill(circle, &druid_color_thesaurus::pink::CORAL_PINK);

            }
        })
    }

}