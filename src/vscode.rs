#[cfg(target_arch = "wasm32")]
mod actual {
    use std::collections::HashMap;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(raw_module = "./run.js")]
    extern "C" {
        pub fn started();

        pub fn edited(
            new_text: &str,
            start_line: usize,
            start_col: usize,
            end_line: usize,
            end_col: usize,
        );

        #[wasm_bindgen(js_name = requestQuickFixes)]
        pub fn request_quick_fixes(id: usize, line: usize, col: usize);

        #[wasm_bindgen(js_name = requestCompletions)]
        pub fn request_completions(line: usize, col: usize);

        #[wasm_bindgen(js_name = executeCommand)]
        pub fn execute_command(command: String, args: JsValue);

        #[wasm_bindgen(js_name = executeWorkspaceEdit)]
        pub fn execute_workspace_edit(edit: JsValue);

        #[wasm_bindgen(js_name = telemetryEvent)]
        fn telemetry_event(cat: String, info: JsValue);

        #[wasm_bindgen(js_name = telemetryCrash)]
        pub fn telemetry_crash(msg: String);

        #[wasm_bindgen(js_name = registerBreakpoints)]
        pub fn register_breakpoints(lines: Vec<usize>);
    }

    #[allow(dead_code)]
    pub fn log_event(cat: &'static str, info: HashMap<&'static str, &str>) {
        telemetry_event(
            cat.to_string(),
            serde_wasm_bindgen::to_value(&info).unwrap(),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod shim {
    use std::collections::HashMap;

    pub fn started() {}
    pub fn edited(_: &str, _: usize, _: usize, _: usize, _: usize) {}
    pub fn request_quick_fixes(_: usize, _: usize, _: usize) {}
    pub fn request_completions(_: usize, _: usize) {}
    pub fn execute_command(_: String, _: wasm_bindgen::JsValue) {}
    pub fn execute_workspace_edit(_: wasm_bindgen::JsValue) {}
    pub fn register_breakpoints(_: Vec<usize>) {}
    pub fn log_event(_: &'static str, _: HashMap<&'static str, &str>) {}
}

#[cfg(target_arch = "wasm32")]
pub use actual::*;

#[cfg(not(target_arch = "wasm32"))]
pub use shim::*;
