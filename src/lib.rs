use std::f64;
use wasm_bindgen::prelude::*;
use std::cmp::max;
use js_sys::Array;
use web_sys::console;

const SCALING_FACTOR: f64 = 4.0;

#[wasm_bindgen(start)]
fn start() {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    full_redraw(&canvas, &context);
    
    // Add event listener for window resize + redraw
    let closure = Closure::wrap(Box::new(move || { full_redraw(&canvas, &context); }) as Box<dyn Fn()>);
    web_sys::window().unwrap().add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();

}

fn full_redraw(canvas: &web_sys::HtmlCanvasElement, context: &web_sys::CanvasRenderingContext2d) {
    let width = calculate_window_width();
    let height = calculate_window_height();
    canvas.set_width(width);
    canvas.set_height(height);
    context.translate(width as f64 / 2.0, height as f64 / 2.0).unwrap(); // now 0,0 is the center of the canvas.
    redraw(context, width, height);
}

// enum left / right
#[derive(PartialEq)]
enum Gear {
    Left,
    Right,
}

// struct for points
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

impl std::ops::Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

// struct for gear specs
struct GearSpecs {
    teeth: f64,
    module: f64,
    tooth_angle: f64,
    clearance_mult: f64,
    backlash_mult: f64,
}

// debug config struct
struct DebugConfig {
    show_base_circle: bool,
    show_inner_circle: bool,
    show_outer_circle: bool,
    show_pitch_circle: bool,
}

impl DebugConfig {
    fn default() -> Self {
        Self {
            show_base_circle: false,
            show_inner_circle: false,
            show_outer_circle: false,
            show_pitch_circle: false,
        }
    }
}

fn redraw(context: &web_sys::CanvasRenderingContext2d, width: u32, height: u32) {
    context.clear_rect(0.0, 0.0, width as f64, height as f64);
    draw_background(context, width, height);

    let debug_config = DebugConfig::default();

    // Draw left gear (circle for now)
    let gear_1_spec = GearSpecs {
        teeth: 50.0,
        module: 15.0,
        tooth_angle: 20.0,
        clearance_mult: 0.167,
        backlash_mult: 0.05,
    };
    draw_gear(context, Gear::Left, gear_1_spec, &debug_config);

    // Draw right gear (circle for now)
    let gear_2_spec = GearSpecs {
        teeth: 10.0,
        module: 15.0,
        tooth_angle: 20.0,
        clearance_mult: 0.167,
        backlash_mult: 0.05,
    };
    draw_gear(context, Gear::Right, gear_2_spec, &debug_config);
}


fn draw_gear(context: &web_sys::CanvasRenderingContext2d, left_or_right: Gear, gear_spec: GearSpecs, debug_config: &DebugConfig) {
    // Gear specifications
    let teeth = gear_spec.teeth;
    let module = gear_spec.module;
    let tooth_angle = gear_spec.tooth_angle;
    let pressure_angle_rads = tooth_angle * f64::consts::PI / 180.0;
    let pitch_diameter = teeth * module;
    let base_diameter = pitch_diameter * pressure_angle_rads.cos();
    let addendum = module;
    let clearance = gear_spec.clearance_mult * module;
    let backlash_allowance = gear_spec.backlash_mult * module;
    let dedendum = clearance + module;
    let root_diameter = pitch_diameter - 2.0 * dedendum;
    let outer_diameter = pitch_diameter + 2.0 * addendum;
    let base_radius = base_diameter / 2.0;
    let root_radius = root_diameter / 2.0;
    let outer_radius = outer_diameter / 2.0;
    let pitch_radius = pitch_diameter / 2.0;

    let offset = Point {
        x: if left_or_right == Gear::Left { -pitch_radius } else { pitch_radius }, 
        y: 0.0 
    };

    // maybe draw debug circles
    if debug_config.show_base_circle {
        context.set_stroke_style_str("lightblue");
        draw_circle(context, offset.x, 0.0, base_radius);
    }
    if debug_config.show_inner_circle {
        context.set_stroke_style_str("purple");
        draw_circle(context, offset.x, 0.0, root_radius);
    }
    if debug_config.show_outer_circle {
        context.set_stroke_style_str("lightgreen");
        draw_circle(context, offset.x, 0.0, outer_radius);
    }
    if debug_config.show_pitch_circle {
        context.set_stroke_style_str("red");
        draw_circle(context, offset.x, 0.0, pitch_radius);
    }

    // Functions for the involute curve generation
    fn involute(base_radius: f64, theta: f64) -> Point {
        let x = base_radius * (theta.cos() + theta * theta.sin());
        let y = base_radius * (theta.sin() - theta * theta.cos());
        Point { x: x, y: y }
    }

    // Generate the involute gear profile
    let tooth_angle = 2.0 * f64::consts::PI / teeth;
    let involute_steps = 100;  // Resolution for the involute curve
    let theta_min = if root_radius > base_radius { f64::sqrt((root_radius / base_radius).powi(2) - 1.0) } else { 0.0 };
    let theta_max = f64::sqrt((outer_radius / base_radius).powi(2) - 1.0);  // Max theta for the involute
    let theta: Vec<f64> = (0..involute_steps).map(|i| i as f64 * (theta_max - theta_min) / involute_steps as f64 + theta_min).collect();

    let theta_pitch = f64::sqrt((pitch_radius / base_radius).powi(2) - 1.0);  // Max theta for the involute
    let mut pitch_correction = (involute(base_radius, theta_pitch).x / pitch_radius).acos();
    let clearance_correction = ((backlash_allowance / 2.0) / pitch_radius).asin();
    pitch_correction = pitch_correction - clearance_correction;

    // generate involute points
    let involute_points: Vec<Point> = theta.iter().map(|theta| involute(base_radius, *theta)).collect();
    let involute_points_neg: Vec<Point> = theta.iter().rev().map(|theta| involute(base_radius, -*theta)).collect();

    // draw involute points
    fn rotate_point(point: &Point, angle: f64) -> Point {
        let x_rot = point.x * angle.cos() - point.y * angle.sin();
        let y_rot = point.x * angle.sin() + point.y * angle.cos();
        Point { x: x_rot, y: y_rot }
    }

    context.set_stroke_style_str("black");
    context.set_line_dash(&JsValue::from(Vec::<f64>::new()));
    context.begin_path();
    
    // draw all teeth
    (0..teeth as u32).for_each(|i| {
        let angle_offset_rads = i as f64 * tooth_angle;

        let start_point = offset + (
            rotate_point(&Point { x: root_radius, y: 0.0 }, angle_offset_rads - pitch_correction)
        );
        context.move_to(start_point.x, start_point.y);
        (&involute_points).clone().into_iter().skip(1).for_each(|pt| {
            let rotated_point = rotate_point(&pt, angle_offset_rads - pitch_correction);
            context.line_to(offset.x + rotated_point.x, rotated_point.y);
        });
        
        let start_point_neg = offset + (
            rotate_point(&involute_points_neg[0], angle_offset_rads + tooth_angle / 2.0 + pitch_correction)
        );
        context.line_to(start_point_neg.x, start_point_neg.y);
        (&involute_points_neg).clone().into_iter().skip(1).for_each(|pt| {
            let rotated_point = rotate_point(&pt, angle_offset_rads + tooth_angle / 2.0 + pitch_correction);
            context.line_to(offset.x + rotated_point.x, rotated_point.y);
        });
        let end_involute_point = rotate_point(&Point { x: root_radius, y: 0.0 }, angle_offset_rads + tooth_angle / 2.0 + pitch_correction);
        context.line_to(offset.x + end_involute_point.x, end_involute_point.y);
        
        let end_point = rotate_point(&Point { x: root_radius, y: 0.0 }, angle_offset_rads + tooth_angle - pitch_correction);
        context.line_to(offset.x + end_point.x, end_point.y);
    });
    context.stroke();
}

fn calculate_window_width() -> u32 {
    web_sys::window().unwrap().inner_width().unwrap().as_f64().unwrap() as u32
}

fn calculate_window_height() -> u32 {
    web_sys::window().unwrap().inner_height().unwrap().as_f64().unwrap() as u32
}

fn draw_circle(context: &web_sys::CanvasRenderingContext2d, x: f64, y: f64, radius: f64) {
    context.begin_path();
    context
        .arc(x, y, radius, 0.0, f64::consts::PI * 2.0)
        .unwrap();
    context.stroke(); // Stroke the path after drawing
}

fn draw_background(context: &web_sys::CanvasRenderingContext2d, width: u32, height: u32) {
    context.set_fill_style_str("white");
    context.fill_rect(0.0, 0.0, width as f64, height as f64);

    // Draw the grid
    context.set_stroke_style_str("lightblue");
    context.set_line_width(1.0);

    let grid_spacing = 10.0; // Set the spacing for the grid lines to 50 pixels
    context.save(); // Save the current context state
    // context.rotate(f64::consts::PI / 4.0).unwrap(); // Rotate the context by 45 degrees

    // Draw horizontal lines
    let max_offset = max(width, height);
    let neg_offset = -(max_offset as f64);

    let start = (neg_offset) as i32;
    let end = (max_offset as f64) as i32;
    for i in (start..=end).step_by(grid_spacing as usize) {
        context.move_to(neg_offset, i as f64);
        context.line_to(max_offset as f64, i as f64);
        context.stroke();
    }
    
    // Draw vertical lines
    let start = (neg_offset) as i32;
    let end = (max_offset as f64) as i32;
    for i in (start..=end).step_by(grid_spacing as usize) {
        context.move_to(i as f64, neg_offset);
        context.line_to(i as f64, max_offset as f64);
        context.stroke();
    }

    context.restore(); // Restore the context to its original state

    // Draw tiny crosshair at 0, 0 for debugging
    context.set_stroke_style_str("red");
    context.set_line_width(1.0);
    context.begin_path();
    let offset = 5.0;
    context.move_to(0.0, -offset);
    context.line_to(0.0, offset);
    context.move_to(-offset, 0.0);
    context.line_to(offset, 0.0);
    context.stroke();
}
