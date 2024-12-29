use base64::engine::general_purpose;
use base64::Engine;
use printpdf;
use std::cell::RefCell;
use std::f64;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::console;
use web_sys::HtmlInputElement;

#[wasm_bindgen(start)]
fn start() -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    document.body().unwrap().append_child(&canvas)?;
    let canvas_rc = Rc::new(RefCell::new(canvas));

    // create page state
    let page_state = PageState {
        left_gear_spec: GearSpecs {
            teeth: 50.0,
            diametric_pitch: 12.0,
            tooth_angle: 20.0,
            clearance_mult: 0.167,
            backlash_mult: 0.05,
        },
        right_gear_spec: GearSpecs {
            teeth: 10.0,
            diametric_pitch: 12.0,
            tooth_angle: 20.0,
            clearance_mult: 0.167,
            backlash_mult: 0.05,
        },
    };
    let page_state_rc = Rc::new(RefCell::new(page_state));

    // setup canvas drawing context + do initial redraw
    let context = canvas_rc
        .borrow()
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    let context_rc = Rc::new(RefCell::new(context));

    // do initial redraw
    full_redraw(
        &canvas_rc.borrow(),
        &context_rc.borrow(),
        &page_state_rc.borrow(),
    );

    // Add event listener for window resize + redraw
    let page_state_rc_clone = page_state_rc.clone();
    let canvas_rc_clone = canvas_rc.clone();
    let context_rc_clone = context_rc.clone();
    let closure = Closure::wrap(Box::new(move || {
        full_redraw(
            &canvas_rc_clone.borrow(),
            &context_rc_clone.borrow(),
            &page_state_rc_clone.borrow(),
        );
    }) as Box<dyn Fn()>);
    web_sys::window()
        .unwrap()
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
        .unwrap();

    // create left sidebar
    let page_state_rc_sidebar_clone = page_state_rc.clone();
    let canvas_rc_sidebar = canvas_rc.clone();
    let context_rc_sidebar = context_rc.clone();
    let print_gears_closure = Closure::wrap(Box::new(move || {
        print_gears(
            &canvas_rc_sidebar.borrow(),
            &context_rc_sidebar.borrow(),
            &page_state_rc_sidebar_clone.borrow(),
        )
        .unwrap();
    }) as Box<dyn Fn()>);
    let sidebar = create_sidebar(page_state_rc, &closure, &print_gears_closure)?;
    document.body().unwrap().append_child(&sidebar)?;
    print_gears_closure.forget();
    closure.forget();

    Ok(())
}

fn print_gears(
    canvas: &web_sys::HtmlCanvasElement,
    context: &web_sys::CanvasRenderingContext2d,
    page_state: &PageState,
) -> Result<(), JsValue> {
    let dpi = 300.0;
    let margin_inches = 0.25;

    // landscape letter paper size
    let width = dpi * (11.0 - margin_inches);
    let height = dpi * (8.5 - margin_inches);

    redraw(
        canvas,
        context,
        width as u32,
        height as u32,
        page_state,
        dpi as u32,
    );

    // export canvas to png
    let data_url = canvas.to_data_url()?;

    console::log_1(&JsValue::from_str("Exporting to PDF"));
    let mut doc = printpdf::PdfDocument::new("Export");
    // data url is a png, convert it to a raw image
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(data_url.split(',').last().unwrap())
        .unwrap();
    console::log_1(&JsValue::from_str("Decoding image"));

    let image = printpdf::RawImage::decode_from_bytes(&image_bytes).unwrap();

    // In the PDF, an image is an `XObject`, identified by a unique `ImageId`
    console::log_1(&JsValue::from_str("Adding image to PDF"));
    let image_xobject_id = doc.add_image(&image);

    console::log_1(&JsValue::from_str("Creating page"));
    let mut transform = printpdf::XObjectTransform::default();
    transform.rotate = Some(printpdf::XObjectRotation {
        angle_ccw_degrees: 90.0,
        rotation_center_x: printpdf::Px(0),
        rotation_center_y: printpdf::Px(0),
    });
    transform.translate_x = Some(printpdf::Pt(72.0 * (8.5 - margin_inches / 2.0)));
    transform.translate_y = Some(printpdf::Pt(72.0 * (margin_inches / 2.0)));
    let page1_contents = vec![printpdf::Op::UseXObject {
        id: image_xobject_id.clone(),
        transform: transform,
    }];

    let page1 = printpdf::PdfPage::new(
        printpdf::Mm(25.4 * 8.5),
        printpdf::Mm(25.4 * 11.0),
        page1_contents,
    );
    let pdf_bytes: Vec<u8> = doc
        .with_pages(vec![page1])
        .save(&printpdf::PdfSaveOptions::default());

    // download pdf bytes
    let document = web_sys::window().unwrap().document().unwrap();
    let a = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    a.set_attribute(
        "href",
        &("data:application/pdf;base64,".to_string()
            + &general_purpose::STANDARD.encode(pdf_bytes)),
    )?;
    a.set_attribute("target", "_blank")?;
    a.click();

    Ok(())
}

fn create_sidebar(
    state: Rc<RefCell<PageState>>,
    redraw_closure: &Closure<dyn Fn()>,
    print_gears_closure: &Closure<dyn Fn()>,
) -> Result<web_sys::Element, JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let sidebar = document.create_element("div")?;
    sidebar.set_attribute("style", "position: fixed; left: 0; top: 0; width: 200px; height: 100%; background-color: #f0f0f0;").unwrap();

    // add title
    let title = document.create_element("h2")?;
    title
        .set_attribute("style", "text-align: center; width: 100%;")
        .unwrap();
    title.set_text_content(Some("Gear Designer"));
    sidebar.append_child(&title)?;

    // add gear specs subtitle
    let gear_specs_subtitle = document.create_element("h3")?;
    gear_specs_subtitle
        .set_attribute("style", "text-align: center; width: 100%;")
        .unwrap();
    gear_specs_subtitle.set_text_content(Some("Gear Specs"));
    sidebar.append_child(&gear_specs_subtitle)?;

    // label for gear module input
    let gear_diametric_pitch_label = document.create_element("label")?;
    gear_diametric_pitch_label
        .set_attribute("for", "gear_diametric_pitch")
        .unwrap();
    gear_diametric_pitch_label.set_text_content(Some("Diametric Pitch:"));
    gear_diametric_pitch_label
        .set_attribute("style", "width: 80%; margin-left: 10%; margin-right: 10%;")
        .unwrap();
    sidebar.append_child(&gear_diametric_pitch_label)?;

    // gear module input
    let gear_diametric_pitch_input = document.create_element("input")?;
    gear_diametric_pitch_input
        .set_attribute("id", "gear_diametric_pitch")
        .unwrap();
    gear_diametric_pitch_input
        .set_attribute("type", "text")
        .unwrap();
    gear_diametric_pitch_input
        .set_attribute("placeholder", "Enter gear diametric pitch")
        .unwrap();
    gear_diametric_pitch_input
        .set_attribute(
            "value",
            &state.borrow().left_gear_spec.diametric_pitch.to_string(),
        )
        .unwrap();
    gear_diametric_pitch_input
        .set_attribute("style", "width: 80%; margin-left: 10%; margin-right: 10%;")
        .unwrap();
    sidebar.append_child(&gear_diametric_pitch_input)?;

    // add left gear subtitle
    let left_gear_subtitle = document.create_element("h3")?;
    left_gear_subtitle
        .set_attribute("style", "text-align: center; width: 100%;")
        .unwrap();
    left_gear_subtitle.set_text_content(Some("Left Gear"));
    sidebar.append_child(&left_gear_subtitle)?;

    // label for left gear teeth input
    let left_gear_teeth_label = document.create_element("label")?;
    left_gear_teeth_label
        .set_attribute("for", "left_gear_teeth")
        .unwrap();
    left_gear_teeth_label.set_text_content(Some("Teeth:"));
    left_gear_teeth_label
        .set_attribute("style", "width: 80%; margin-left: 10%; margin-right: 10%;")
        .unwrap();
    sidebar.append_child(&left_gear_teeth_label)?;

    // add text input for left gear teeth
    let left_gear_input = document.create_element("input")?;
    left_gear_input
        .set_attribute("id", "left_gear_teeth")
        .unwrap();
    left_gear_input.set_attribute("type", "text").unwrap();
    left_gear_input
        .set_attribute("placeholder", "Enter left gear teeth")
        .unwrap();
    left_gear_input
        .set_attribute("value", &state.borrow().left_gear_spec.teeth.to_string())
        .unwrap();
    left_gear_input
        .set_attribute("style", "width: 80%; margin-left: 10%; margin-right: 10%;")
        .unwrap();
    sidebar.append_child(&left_gear_input)?;

    // add right gear subtitle
    let right_gear_subtitle = document.create_element("h3")?;
    right_gear_subtitle
        .set_attribute("style", "text-align: center; width: 100%;")
        .unwrap();
    right_gear_subtitle.set_text_content(Some("Right Gear"));
    sidebar.append_child(&right_gear_subtitle)?;

    // label for right gear teeth input
    let right_gear_teeth_label = document.create_element("label")?;
    right_gear_teeth_label
        .set_attribute("for", "right_gear_teeth")
        .unwrap();
    right_gear_teeth_label.set_text_content(Some("Teeth:"));
    right_gear_teeth_label
        .set_attribute("style", "width: 80%; margin-left: 10%; margin-right: 10%;")
        .unwrap();
    sidebar.append_child(&right_gear_teeth_label)?;

    // add right gear input
    let right_gear_input = document.create_element("input")?;
    right_gear_input
        .set_attribute("id", "right_gear_teeth")
        .unwrap();
    right_gear_input.set_attribute("type", "text").unwrap();
    right_gear_input
        .set_attribute("placeholder", "Enter right gear teeth")
        .unwrap();
    right_gear_input
        .set_attribute("value", &state.borrow().right_gear_spec.teeth.to_string())
        .unwrap();
    right_gear_input
        .set_attribute("style", "width: 80%; margin-left: 10%; margin-right: 10%;")
        .unwrap();
    sidebar.append_child(&right_gear_input)?;

    // add button for print
    let print_button = document.create_element("button")?;
    print_button.set_attribute("id", "print_button").unwrap();
    print_button.set_text_content(Some("Print"));
    print_button
        .set_attribute(
            "style",
            "width: 100px; position: fixed; bottom: 20px; left: 20px;",
        )
        .unwrap();
    sidebar.append_child(&print_button)?;

    // update print button to create an alert with the current gear specs
    print_button
        .add_event_listener_with_callback("click", print_gears_closure.as_ref().unchecked_ref())?;
    print_button
        .add_event_listener_with_callback("click", redraw_closure.as_ref().unchecked_ref())?;

    // Add all event listeners to update state when input changes
    let closure = Closure::wrap(Box::new(move || {
        // get left gear input
        let value = left_gear_input
            .dyn_ref::<HtmlInputElement>()
            .unwrap()
            .value();
        if let Ok(teeth) = value.parse::<u32>() {
            state.borrow_mut().left_gear_spec.teeth = teeth as f64; // Update the state
        }
        // gear diametric pitch input
        let value = gear_diametric_pitch_input
            .dyn_ref::<HtmlInputElement>()
            .unwrap()
            .value();
        if let Ok(diametric_pitch) = value.parse::<f64>() {
            state.borrow_mut().left_gear_spec.diametric_pitch = diametric_pitch;
            state.borrow_mut().right_gear_spec.diametric_pitch = diametric_pitch;
        }

        // get right gear input
        let value = right_gear_input
            .dyn_ref::<HtmlInputElement>()
            .unwrap()
            .value();
        if let Ok(teeth) = value.parse::<u32>() {
            // Borrow the state mutably to update it
            state.borrow_mut().right_gear_spec.teeth = teeth as f64; // Update the state
        }
    }) as Box<dyn Fn()>);

    sidebar.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())?;
    closure.forget();

    // redraw after input is changed / parameters are updated
    sidebar.add_event_listener_with_callback("input", redraw_closure.as_ref().unchecked_ref())?;

    Ok(sidebar)
}

fn full_redraw(
    canvas: &web_sys::HtmlCanvasElement,
    context: &web_sys::CanvasRenderingContext2d,
    page_state: &PageState,
) {
    let width = calculate_window_width_pixels();
    let height = calculate_window_height_pixels();
    canvas
        .set_attribute("style", "padding-left: 200px;")
        .unwrap();
    // 96 is a _reasonable_ default ppi, it's not exposed at all in browsers
    redraw(canvas, context, width - 200, height, page_state, 96);
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

// struct for page state
struct PageState {
    left_gear_spec: GearSpecs,
    right_gear_spec: GearSpecs,
}

// struct for gear specs
struct GearSpecs {
    teeth: f64,
    diametric_pitch: f64,
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

fn redraw(
    canvas: &web_sys::HtmlCanvasElement,
    context: &web_sys::CanvasRenderingContext2d,
    width: u32,
    height: u32,
    page_state: &PageState,
    ppi: u32,
) {
    canvas.set_width(width);
    canvas.set_height(height);
    draw_background(context, width, height, ppi);

    context
        .translate(width as f64 / 2.0, height as f64 / 2.0)
        .unwrap(); // now 0,0 is the center of the canvas.

    let debug_config = DebugConfig::default();

    // Draw left gear (circle for now)
    draw_gear(
        context,
        Gear::Left,
        &page_state.left_gear_spec,
        &debug_config,
        ppi,
    );

    // Draw right gear (circle for now)
    draw_gear(
        context,
        Gear::Right,
        &page_state.right_gear_spec,
        &debug_config,
        ppi,
    );
}

fn draw_gear(
    context: &web_sys::CanvasRenderingContext2d,
    left_or_right: Gear,
    gear_spec: &GearSpecs,
    debug_config: &DebugConfig,
    ppi: u32,
) {
    // Gear specifications
    let teeth = gear_spec.teeth;
    let module = (1.0 / gear_spec.diametric_pitch) * ppi as f64;
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
        x: if left_or_right == Gear::Left {
            -pitch_radius
        } else {
            pitch_radius
        },
        y: 0.0,
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
    let involute_steps = 100; // Resolution for the involute curve
    let theta_min = if root_radius > base_radius {
        f64::sqrt((root_radius / base_radius).powi(2) - 1.0)
    } else {
        0.0
    };
    let theta_max = f64::sqrt((outer_radius / base_radius).powi(2) - 1.0); // Max theta for the involute
    let theta: Vec<f64> = (0..involute_steps)
        .map(|i| i as f64 * (theta_max - theta_min) / involute_steps as f64 + theta_min)
        .collect();

    let theta_pitch = f64::sqrt((pitch_radius / base_radius).powi(2) - 1.0); // Max theta for the involute
    let mut pitch_correction = (involute(base_radius, theta_pitch).x / pitch_radius).acos();
    let clearance_correction = ((backlash_allowance / 2.0) / pitch_radius).asin();
    pitch_correction = pitch_correction - clearance_correction;

    // generate involute points
    let involute_points: Vec<Point> = theta
        .iter()
        .map(|theta| involute(base_radius, *theta))
        .collect();
    let involute_points_neg: Vec<Point> = theta
        .iter()
        .rev()
        .map(|theta| involute(base_radius, -*theta))
        .collect();

    // draw involute points
    fn rotate_point(point: &Point, angle: f64) -> Point {
        let x_rot = point.x * angle.cos() - point.y * angle.sin();
        let y_rot = point.x * angle.sin() + point.y * angle.cos();
        Point { x: x_rot, y: y_rot }
    }

    context.set_stroke_style_str("black");
    context
        .set_line_dash(&JsValue::from(Vec::<f64>::new()))
        .unwrap();
    context.begin_path();

    // draw all teeth
    (0..teeth as u32).for_each(|i| {
        let angle_offset_rads = i as f64 * tooth_angle;

        let start_point = offset
            + (rotate_point(
                &Point {
                    x: root_radius,
                    y: 0.0,
                },
                angle_offset_rads - pitch_correction,
            ));
        context.move_to(start_point.x, start_point.y);
        (&involute_points)
            .clone()
            .into_iter()
            .skip(1)
            .for_each(|pt| {
                let rotated_point = rotate_point(&pt, angle_offset_rads - pitch_correction);
                context.line_to(offset.x + rotated_point.x, rotated_point.y);
            });

        let start_point_neg = offset
            + (rotate_point(
                &involute_points_neg[0],
                angle_offset_rads + tooth_angle / 2.0 + pitch_correction,
            ));
        context.line_to(start_point_neg.x, start_point_neg.y);
        (&involute_points_neg)
            .clone()
            .into_iter()
            .skip(1)
            .for_each(|pt| {
                let rotated_point = rotate_point(
                    &pt,
                    angle_offset_rads + tooth_angle / 2.0 + pitch_correction,
                );
                context.line_to(offset.x + rotated_point.x, rotated_point.y);
            });
        let end_involute_point = rotate_point(
            &Point {
                x: root_radius,
                y: 0.0,
            },
            angle_offset_rads + tooth_angle / 2.0 + pitch_correction,
        );
        context.line_to(offset.x + end_involute_point.x, end_involute_point.y);

        let end_point = rotate_point(
            &Point {
                x: root_radius,
                y: 0.0,
            },
            angle_offset_rads + tooth_angle - pitch_correction,
        );
        context.line_to(offset.x + end_point.x, end_point.y);
    });
    context.stroke();
}

fn calculate_window_width_pixels() -> u32 {
    web_sys::window()
        .unwrap()
        .inner_width()
        .unwrap()
        .as_f64()
        .unwrap() as u32
}

fn calculate_window_height_pixels() -> u32 {
    web_sys::window()
        .unwrap()
        .inner_height()
        .unwrap()
        .as_f64()
        .unwrap() as u32
}

fn draw_circle(context: &web_sys::CanvasRenderingContext2d, x: f64, y: f64, radius: f64) {
    context.begin_path();
    context
        .arc(x, y, radius, 0.0, f64::consts::PI * 2.0)
        .unwrap();
    context.stroke(); // Stroke the path after drawing
}

fn draw_background(context: &web_sys::CanvasRenderingContext2d, width: u32, height: u32, ppi: u32) {
    context.clear_rect(0.0, 0.0, width as f64, height as f64);
    context.set_fill_style_str("white");
    context.fill_rect(0.0, 0.0, width as f64, height as f64);

    // Draw the grid
    context.set_stroke_style_str("lightblue");
    context.set_line_width(1.0);

    let grid_spacing = ppi as f64 / 2.0; // grid lines every half inch
    context.save(); // Save the current context state

    // Draw horizontal lines
    let height_offset = ((height as f64) / 2.0) as u32 % grid_spacing as u32;
    for i in (height_offset..=height + height_offset).step_by(grid_spacing as usize) {
        context.move_to(0.0, (i) as f64);
        context.line_to(width as f64, (i) as f64);
        context.stroke();
    }

    // Draw vertical lines
    let width_offset = ((width as f64) / 2.0) as u32 % grid_spacing as u32;
    for i in (width_offset..=width + width_offset).step_by(grid_spacing as usize) {
        context.move_to(i as f64, 0.0);
        context.line_to(i as f64, height as f64);
        context.stroke();
    }

    context.restore(); // Restore the context to its original state

    // Draw tiny crosshair in the middle for debugging
    context.set_stroke_style_str("red");
    context.set_line_width(1.0);
    context.begin_path();
    let offset = 5.0;
    context.move_to(width as f64 / 2.0, (height as f64 / 2.0) - offset);
    context.line_to(width as f64 / 2.0, (height as f64 / 2.0) + offset);
    context.move_to((width as f64 / 2.0) - offset, height as f64 / 2.0);
    context.line_to((width as f64 / 2.0) + offset, height as f64 / 2.0);
    context.stroke();
}
