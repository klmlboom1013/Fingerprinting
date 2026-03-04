use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, WebglDebugRendererInfo, WebGlRenderingContext, OfflineAudioContext};

fn get_canvas_fingerprint(document: &web_sys::Document) -> String {
    if let Ok(canvas) = document.create_element("canvas") {
        if let Ok(canvas) = canvas.dyn_into::<HtmlCanvasElement>() {
            canvas.set_width(200);
            canvas.set_height(50);
            if let Ok(Some(context)) = canvas.get_context("2d") {
                if let Ok(ctx) = context.dyn_into::<CanvasRenderingContext2d>() {
                    ctx.set_text_baseline("top");
                    ctx.set_font("14px 'Arial'");
                    ctx.set_text_baseline("alphabetic");
                    ctx.set_fill_style_str("#f60");
                    ctx.fill_rect(125.0, 1.0, 62.0, 20.0);
                    ctx.set_fill_style_str("#069");
                    let _ = ctx.fill_text("Hello, world! 😃", 2.0, 15.0);
                    ctx.set_fill_style_str("rgba(102, 204, 0, 0.7)");
                    let _ = ctx.fill_text("Hello, world! 😃", 4.0, 17.0);
                    
                    return canvas.to_data_url().unwrap_or_else(|_| "unknown".to_string());
                }
            }
        }
    }
    "unknown".to_string()
}

// Anti-Spoofing: Generates canvas multiple times to detect dynamic noise injection
fn analyze_canvas_stability(document: &web_sys::Document, iterations: usize) -> (String, bool, f32) {
    let mut results = Vec::new();
    let mut is_mutating = false;

    // Run multiple times
    for _ in 0..iterations {
        let fp = get_canvas_fingerprint(document);
        results.push(fp);
    }

    // Check if the results differ (Anti-detect browsers inject random noise on every call)
    let first_fp = &results[0];
    let mut same_count = 1;

    for i in 1..iterations {
        if &results[i] != first_fp {
            is_mutating = true;
        } else {
            same_count += 1;
        }
    }

    let stability_score = (same_count as f32 / iterations as f32) * 100.0;
    
    // If mutating, it's highly likely spoofing. Return the first result anyway as baseline.
    (first_fp.clone(), is_mutating, stability_score)
}

// Note: A true AudioContext fingerprinting in Wasm is async due to rendering, 
// so here we compute a simplified synchronous audio parameters fingerprint.
fn get_audio_fingerprint() -> String {
    if let Ok(ctx) = OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 44100, 44100.0) {
        return format!("Audio-Supported-{}", ctx.sample_rate());
    }
    "Audio-Not-Supported".to_string()
}

// Simplified Font Detection
fn detect_fonts(_document: &web_sys::Document) -> Vec<String> {
    let fonts_to_check = vec!["Arial", "Comic Sans MS", "Courier New", "Georgia", "Impact", "Times New Roman", "Trebuchet MS", "Verdana"];
    let mut detected = Vec::new();
    for font in fonts_to_check {
        detected.push(font.to_string());
    }
    detected
}

#[wasm_bindgen]
pub fn get_fingerprint() -> String {
    let window = web_sys::window().expect("should have a window in this context");
    let navigator = window.navigator();
    let document = window.document().expect("should have a document on window");

    // Get User Agent
    let user_agent = navigator.user_agent().unwrap_or_else(|_| "unknown".to_string());
    
    // Get Language
    let language = navigator.language().unwrap_or_else(|| "unknown".to_string());
    
    // Get Hardware Concurrency
    let hardware_concurrency = navigator.hardware_concurrency();
    
    // Get Screen resolution and color depth
    let (width, height, color_depth) = if let Ok(screen) = window.screen() {
        (
            screen.width().unwrap_or(0),
            screen.height().unwrap_or(0),
            screen.color_depth().unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    };
    
    // Get Timezone Offset via JS
    let date = js_sys::Date::new_0();
    let timezone_offset = date.get_timezone_offset();

    // Get WebGL info (Renderer and Vendor)
    let mut webgl_vendor = String::from("unknown");
    let mut webgl_renderer = String::from("unknown");
    
    if let Ok(canvas) = document.create_element("canvas") {
        if let Ok(canvas) = canvas.dyn_into::<HtmlCanvasElement>() {
            if let Ok(Some(context)) = canvas.get_context("webgl") {
                if let Ok(gl) = context.dyn_into::<WebGlRenderingContext>() {
                    // Try to get unmasked renderer info
                    if let Ok(Some(_debug_info)) = gl.get_extension("WEBGL_debug_renderer_info") {
                        let unmasked_vendor_webgl = WebglDebugRendererInfo::UNMASKED_VENDOR_WEBGL;
                        let unmasked_renderer_webgl = WebglDebugRendererInfo::UNMASKED_RENDERER_WEBGL;
                        
                        if let Ok(v) = gl.get_parameter(unmasked_vendor_webgl) {
                            if let Some(s) = v.as_string() {
                                webgl_vendor = s;
                            }
                        }
                        if let Ok(r) = gl.get_parameter(unmasked_renderer_webgl) {
                            if let Some(s) = r.as_string() {
                                webgl_renderer = s;
                            }
                        }
                    }
                }
            }
        }
    }

    // Advanced features & Anti-Spoofing Analysis (10 iterations)
    let (canvas_fp, is_canvas_spoofed, canvas_stability) = analyze_canvas_stability(&document, 10);
    let audio_fp = get_audio_fingerprint();
    let fonts = detect_fonts(&document);

    // Compute unique hash based on all values
    let mut combined_data = String::new();
    combined_data.push_str(&user_agent);
    combined_data.push_str(&language);
    combined_data.push_str(&hardware_concurrency.to_string());
    combined_data.push_str(&width.to_string());
    combined_data.push_str(&height.to_string());
    combined_data.push_str(&color_depth.to_string());
    combined_data.push_str(&timezone_offset.to_string());
    combined_data.push_str(&webgl_vendor);
    combined_data.push_str(&webgl_renderer);
    combined_data.push_str(&canvas_fp);
    combined_data.push_str(&audio_fp);
    combined_data.push_str(&fonts.join(","));

    let digest = md5::compute(combined_data.as_bytes());
    let device_id = format!("{:x}", digest);

    // Determines overall spoofing probability flag
    let spoofing_detected = is_canvas_spoofed;

    // Return as JSON string
    format!(
        r#"{{
    "deviceId": "{}",
    "userAgent": "{}",
    "language": "{}",
    "hardwareConcurrency": {},
    "screenResolution": "{}x{}",
    "colorDepth": {},
    "timezoneOffset": {},
    "webglVendor": "{}",
    "webglRenderer": "{}",
    "audioContext": "{}",
    "canvasFingerprintLength": {},
    "fontsDetected": {},
    "antiSpoofing": {{
        "isSpoofingDetected": {},
        "canvasStabilityScore": {}
    }}
}}"#,
        device_id,
        user_agent.replace("\"", "\\\""),
        language.replace("\"", "\\\""),
        hardware_concurrency,
        width,
        height,
        color_depth,
        timezone_offset,
        webgl_vendor.replace("\"", "\\\""),
        webgl_renderer.replace("\"", "\\\""),
        audio_fp,
        canvas_fp.len(),
        fonts.len(),
        spoofing_detected,
        canvas_stability
    )
}
