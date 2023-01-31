use druid::kurbo::Circle;
use druid::widget::Painter;
use druid::{ Point, Data, theme, RenderContext, Size, Rect };
use druid_color_thesaurus::gray;

use crate::panning::PanningData;
use crate::zooming::ZoomData;

pub trait GridSnappingData: PanningData + ZoomData {
    fn get_cell_size(&self) -> f64;
    fn set_cell_size(&mut self, size: f64);
    fn get_grid_visibility(&self) -> bool;
    fn set_grid_visibility(&mut self, state: bool);
    fn move_to_grid_position(&self, desired_position: Point) -> Point {
        let (row, col) = self.get_grid_index(desired_position);
        self.get_grid_position(row, col)
    }

    fn get_grid_index(&self, position: Point) -> (isize, isize) {
        // Normalise translation offset
        let mut position_norm = position; 
        position_norm.x -= self.get_absolute_offset().x;
        position_norm.y -= self.get_absolute_offset().y;

        let scaled_cell_size = self.get_cell_size() * self.get_zoom_scale();

        let row = (position_norm.y / scaled_cell_size).floor() as isize;
        let col = (position_norm.x / scaled_cell_size).floor() as isize;

        (row, col)
    }

    fn get_grid_position(&self, row: isize, col:isize) -> Point {
        let scaled_cell_size = self.get_cell_size() * self.get_zoom_scale();

        Point { 
            x: col as f64 * scaled_cell_size + self.get_absolute_offset().x, 
            y: row as f64 * scaled_cell_size + self.get_absolute_offset().y, 
        }
    }
}

#[derive(Copy, Clone)]
pub struct GridSnappingSystemPainter{
    show_origin: bool,
    debug_offset: bool,
}

impl Default for GridSnappingSystemPainter {
    fn default() -> Self {
        Self { 
            show_origin: true, 
            debug_offset: true,
        }
    }
}

impl GridSnappingSystemPainter {
    pub fn square_grid<T: Data + GridSnappingData>(&self) -> Painter<T> {
        let origin_visibility = self.show_origin;
        let debug_visibility = self.debug_offset;

        Painter::new(move |ctx, data: &T, env| {
            let scaled_cell_size = data.get_cell_size() * data.get_zoom_scale();
            let line_width = scaled_cell_size * 0.05;
            
            // Partial Paint Setup
            let screen_space = ctx.size();
            let damage_region  = ctx.region();
            let invalidation_rect = damage_region.bounding_box();

            // Background Painting Logic
            let rect = screen_space.to_rect();
            ctx.fill(rect, &env.get(theme::BACKGROUND_DARK));

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
                    from_point.y += data.get_absolute_offset().y % scaled_cell_size;
                    let size = Size::new(ctx.size().width, line_width);
                    let rect = Rect::from_origin_size(from_point, size);
                    ctx.fill(rect, &gray::GAINSBORO)
                }

                for col in from_col..=to_col {
                    let mut from_point = Point::new(scaled_cell_size * col as f64 - line_width / 2.0, 0.0);
                    // Integrate translation data to line rendering
                    from_point.x += data.get_absolute_offset().x % scaled_cell_size;
                    let size = Size::new(line_width, ctx.size().width);
                    let rect = Rect::from_origin_size(from_point, size);
                    ctx.fill(rect, &gray::GAINSBORO)
                }
            }

            if origin_visibility {
                let center = Point::new(data.get_absolute_offset().x,data.get_absolute_offset().y);
                let circle = Circle::new(center, 5.0);
                ctx.fill(circle, &druid_color_thesaurus::red::CARMINE);
            }

            if debug_visibility {
                let center = Point::new(data.get_absolute_offset().x % scaled_cell_size,data.get_absolute_offset().y % scaled_cell_size);
                let circle = Circle::new(center, 5.0);
                ctx.fill(circle, &druid_color_thesaurus::pink::CORAL_PINK);

            }
        })
    }

    pub fn dot_grid<T: Data + GridSnappingData>(&self) -> Painter<T> {
        let origin_visibility = self.show_origin;
        let debug_visibility = self.debug_offset;

        Painter::new(move |ctx, data: &T, env| {
            let scaled_cell_size = data.get_cell_size() * data.get_zoom_scale();
            let line_width = scaled_cell_size * 0.05;
            
            // Partial Paint Setup
            let screen_space = ctx.size();
            let damage_region  = ctx.region();
            let invalidation_rect = damage_region.bounding_box();

            // Background Painting Logic
            let rect = screen_space.to_rect();
            ctx.fill(rect, &env.get(theme::BACKGROUND_DARK));

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
                    for col in from_col..=to_col {
                        let mut center = Point::new(
                            scaled_cell_size * col as f64, 
                            scaled_cell_size * row as f64, 
                        );

                        // Zoom UI functionality
                        center.x += data.get_absolute_offset().x % scaled_cell_size;
                        center.y += data.get_absolute_offset().y % scaled_cell_size;

                        let circle = Circle::new(center, line_width);
                        ctx.fill(circle, &env.get(theme::BORDER_LIGHT));
                    }
                }
            }

            if origin_visibility {
                let center = Point::new(data.get_absolute_offset().x,data.get_absolute_offset().y);
                let circle = Circle::new(center, 5.0);
                ctx.fill(circle, &druid_color_thesaurus::red::CARMINE);
            }

            if debug_visibility {
                let center = Point::new(data.get_absolute_offset().x % scaled_cell_size,data.get_absolute_offset().y % scaled_cell_size);
                let circle = Circle::new(center, 5.0);
                ctx.fill(circle, &druid_color_thesaurus::pink::CORAL_PINK);

            }
        })
    }

}