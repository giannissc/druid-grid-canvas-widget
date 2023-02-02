use druid::im::{Vector};
use druid::{theme, AppLauncher, Color, LocalizedString, WindowDesc, Data, Lens, Widget, WidgetExt, WidgetId, Command, Target,};

use druid::widget::{Flex, Label, MainAxisAlignment, CrossAxisAlignment, Switch, Button, ControllerHost, LensWrap, Container,};

use druid_color_thesaurus::*;

use druid_grid_graph_widget::grid_canvas::{GridCanvasData, GridCanvas, UPDATE_GRID_PLAYBACK};
use druid_grid_graph_widget::panning::{PanningController};
use druid_grid_graph_widget::zooming::{ZoomController};
use druid_grid_graph_widget::{GridItem, StackItem, GridIndex};
use druid_grid_graph_widget::snapping::{GridSnapPainter, GridSnapData};
//////////////////////////////////////////////////////////////////////////////////////
// Constants
//////////////////////////////////////////////////////////////////////////////////////
pub const GRID_COLUMNS: usize = 81;
pub const GRID_ROWS: usize = 31;
pub const BACKGROUND: Color = black::ONYX;
pub const GRID_ID: WidgetId = WidgetId::reserved(1);

//////////////////////////////////////////////////////////////////////////////////////
//
// GridNodeType
//
//////////////////////////////////////////////////////////////////////////////////////
// Add wight and bomb nodes?
pub type Net = i32;
//type Weight = i32;
#[derive(Data, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum GridNodeType<Net> {
    Wall(Net),
    //WeightedNode(Weight),
    StartNode(Net),
    TargetNode(Net),
    //SteinerNode(Net),
    UnexploredNode(Net), //Rename to visitedNodes
    ExploredNode(Net),   //Rename to visitedNodes
    ChosenPath(Net),
}

impl GridNodeType<Net> {
    fn get_net(&self) -> &Net{
        match self{
            Self::Wall(net) => net,
            Self::StartNode(net) => net,
            Self::TargetNode(net) => net,
            Self::UnexploredNode(net) => net,
            Self::ExploredNode(net) => net,
            Self::ChosenPath(net) => net,
        }
    }
}

impl GridItem for GridNodeType<Net> {

    fn get_color(&self) -> &Color {
        match self{
            GridNodeType::Wall(_) => &black::LICORICE,
            GridNodeType::StartNode(_) => &blue::ARGENTINIAN_BLUE,
            GridNodeType::TargetNode(_) => &purple::PURPUREUS,
            GridNodeType::UnexploredNode(_) => &yellow::YELLOW_AMBER,
            GridNodeType::ExploredNode(_) => &brown::MAROON,
            GridNodeType::ChosenPath(_) => &green::ASH_GRAY,
        }
    }

    fn can_add(&self, other: Option<&Self>) -> bool {
        match other {
            None => true,
            Some(_) => false,
        }
    }

    fn can_remove(&self) -> bool {
        true
    }

    fn can_move(&self, other: Option<&Self>) -> bool {
        match other {
            None => true,
            Some(_) => false,
        }
    }
}


//////////////////////////////////////////////////////////////////////////////////////
//
// AppData
//
//////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Data, Lens, PartialEq)]
pub struct AppData{
    pub is_paused: bool,
    pub is_running: bool,
    pub updates_per_second: f64,
    pub grid_data: GridCanvasData<GridNodeType<Net>>,
}

impl AppData {
    pub fn to_period_milli(&self) -> u64 {
        (1000. / self.updates_per_second) as u64
    }
}

//////////////////////////////////////////////////////////////////////////////////////
//
// Main
//
//////////////////////////////////////////////////////////////////////////////////////

fn main() {
    let main_window = WindowDesc::new(make_ui())
        .window_size((1000.0, 500.0))
        .title(LocalizedString::new("Placement & Routing Experiments"));

    let mut data = AppData {
        is_paused: false,
        is_running: false,
        updates_per_second: 10.0,
        grid_data: GridCanvasData::new(GridNodeType::Wall(1), 50.0),
    };

    let mut pattern = Vector::new();
    pattern.push_back(StackItem::Add(GridIndex{row:0, col:0}, GridNodeType::Wall(1), None));
    pattern.push_back(StackItem::Add(GridIndex{row:0, col:1}, GridNodeType::Wall(1), None));
    pattern.push_back(StackItem::Add(GridIndex{row:0, col:2}, GridNodeType::Wall(1), None));
    pattern.push_back(StackItem::Add(GridIndex{row:1, col:0}, GridNodeType::Wall(1), None));
    pattern.push_back(StackItem::Add(GridIndex{row:2, col:0}, GridNodeType::Wall(1), None));
    data.grid_data.submit_to_stack(pattern);

    AppLauncher::with_window(main_window)
        .configure_env(|env, _| {
            env.set(theme::SELECTION_TEXT_COLOR, Color::rgb8(0xA6, 0xCC, 0xFF));
            env.set(theme::WINDOW_BACKGROUND_COLOR, gray::DAVYS_GRAY);
            env.set(theme::CURSOR_COLOR, Color::BLACK);
            env.set(theme::BACKGROUND_LIGHT, Color::rgb8(230, 230, 230));
            env.set(theme::TEXT_COLOR, white::ALABASTER)
        })
        .log_to_console()
        .launch(data)
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppData>{
    let cell_size = 50.0;

    let snap_painter =  GridSnapPainter::default();
    let grid = GridCanvas::new().with_id(GRID_ID).lens(AppData::grid_data);

    let grid_container= grid.background(snap_painter.square_grid());

    let pan_control_host = ControllerHost::new(grid_container, PanningController::default());
    let zoom_control_host = ControllerHost::new(pan_control_host, ZoomController::default());

    Flex::column()
        .with_flex_child(zoom_control_host, 1.0) // Grid widget
        .with_child(make_control_bar())
        .main_axis_alignment(MainAxisAlignment::SpaceAround)
        .cross_axis_alignment(CrossAxisAlignment::Center)
}

fn make_control_bar() -> impl Widget<AppData>{
    Flex::row()
        .with_flex_child(make_grid_options(),1.0)
        .main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .must_fill_main_axis(true)
        .background(BACKGROUND)
}

fn make_grid_options() -> impl Widget<AppData>{
    Flex::column()
        .with_child(Label::new("Grid Options").with_text_size(20.0))
        .with_child(
            Flex::row()
                .with_child(Label::new("Playback: "))
                .with_child(Button::new("Next").lens(AppData::grid_data).on_click(|ctx, data, _env|{
                    data.grid_data.save_data.redo();
                    ctx.submit_command(Command::new(UPDATE_GRID_PLAYBACK, (), Target::Widget(GRID_ID)));
                }))
                .with_child(Button::new("Previous").lens(AppData::grid_data).on_click(|ctx, data, _env|{
                    data.grid_data.save_data.undo();
                    ctx.submit_command(Command::new(UPDATE_GRID_PLAYBACK, (), Target::Widget(GRID_ID)));
                }))
        )
        .with_child(
            Flex::row()
                .with_child(Label::new("Tool: "))
                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .must_fill_main_axis(true)
        )
        .with_child(
            Flex::row()
                .with_child(Label::new("Net: "))
                .with_child(Label::new(|data: &AppData, _: &_| {
                    format!("{:.1}", data.grid_data.grid_item.get_net())
                }))
                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .must_fill_main_axis(true)
        )
        .with_child(
            Flex::row()
                .with_child(Label::new("Show Axis: "))
                .with_child(Switch::new().lens(GridSnapData::grid_visibility).lens(GridCanvasData::snap_data).lens(AppData::grid_data))
                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .must_fill_main_axis(true)
        )
        .main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .padding(5.0)

}  
