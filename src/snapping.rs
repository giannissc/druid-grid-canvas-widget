use druid::kurbo::Circle;
use druid::widget::Painter;
use druid::{ Point, Data, theme, RenderContext, Size, Rect };
use druid_color_thesaurus::gray;

use crate::panning::PanningData;
use crate::zooming::ZoomData;

pub trait SnappingSystem: PanningData + ZoomData {
    fn get_cell_size(&self) -> f64;
    fn set_cell_size(&mut self, size: f64);
    fn get_grid_axis_state(&self) -> bool;
    fn set_grid_axis_state(&mut self, state: bool);
    fn get_snap_position(&self, desired_position: Point) -> Point {

        let scaled_cell_size = self.get_cell_size() * self.get_zoom_scale();

        Point { 
            x: (desired_position.x / scaled_cell_size).floor() * scaled_cell_size, 
            y: (desired_position.y / scaled_cell_size).floor() * scaled_cell_size, 
        }
    }
}

#[derive(Copy, Clone)]
pub struct SnappingSystemPainter;

impl SnappingSystemPainter {
    pub fn square_grid<T: Data + SnappingSystem>(&self) -> Painter<T> {
        Painter::new(|ctx, data: &T, env| {
            let scaled_cell_size = data.get_cell_size() * data.get_zoom_scale();
            let line_width = scaled_cell_size * 0.05;
            
            // Partial Paint Setup
            let screen_space = ctx.size();
            let damage_region  = ctx.region();
            let invalidation_rect = damage_region.bounding_box();

            // println!("Snapping Screen Space: {:?}", screen_space);
            // println!("Snapping Damage Region: {:?}", damage_region);

            // Background Painting Logic
            let rect = screen_space.to_rect();
            ctx.fill(rect, &env.get(theme::BACKGROUND_DARK));

            // Axes Painting Logic
            if data.get_grid_axis_state() {

                let start_point = data.get_snap_position(invalidation_rect.origin());
                let end_point = data.get_snap_position(Point {
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
                    from_point.y += data.get_offset_from_origin().y % scaled_cell_size;
                    let size = Size::new(ctx.size().width, line_width);
                    let rect = Rect::from_origin_size(from_point, size);
                    ctx.fill(rect, &gray::GAINSBORO)
                }

                for col in from_col..=to_col {
                    let mut from_point = Point::new(scaled_cell_size * col as f64 - line_width / 2.0, 0.0);
                    // Integrate translation data to line rendering
                    from_point.x += data.get_offset_from_origin().x % scaled_cell_size;
                    let size = Size::new(line_width, ctx.size().width);
                    let rect = Rect::from_origin_size(from_point, size);
                    ctx.fill(rect, &gray::GAINSBORO)
                }
            }
        })
    }

    pub fn dot_grid<T: Data + SnappingSystem>(&self) -> Painter<T> {
        Painter::new(|ctx, data: &T, env| {
            let scaled_cell_size = data.get_cell_size() * data.get_zoom_scale();
            let line_width = scaled_cell_size * 0.05;
            
            // Partial Paint Setup
            let screen_space = ctx.size();
            let damage_region  = ctx.region();
            let invalidation_rect = damage_region.bounding_box();

            println!("Snapping Screen Space: {:?}", screen_space);
            println!("Snapping Damage Region: {:?}", damage_region);

            // Background Painting Logic
            let rect = screen_space.to_rect();
            ctx.fill(rect, &env.get(theme::BACKGROUND_DARK));

            if data.get_grid_axis_state() {
                let start_point = data.get_snap_position(invalidation_rect.origin());
                let end_point = data.get_snap_position(Point {
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

                        // 
                        center.x += data.get_offset_from_origin().x % scaled_cell_size;
                        center.y += data.get_offset_from_origin().y % scaled_cell_size;

                        let circle = Circle::new(center, line_width);
                        ctx.fill(circle, &env.get(theme::BORDER_LIGHT));
                    }
                }
            }
        })
    }

}