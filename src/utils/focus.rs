use wasm_bindgen::{closure::Closure, JsCast};

pub fn focus_first_form_control_now() {
    let Some(win) = web_sys::window() else { return };
    let Some(doc) = win.document() else { return };
    let list = doc.get_elements_by_class_name("yal-form-control");
    if list.length() == 0 {
        return;
    }
    if let Some(el) = list.item(0) {
        if let Some(he) = el.dyn_ref::<web_sys::HtmlElement>() {
            let _ = he.focus();
        }
    }
}

pub fn raf_focus_first_form_control() {
    if let Some(win) = web_sys::window() {
        let cb = Closure::<dyn FnMut()>::new(focus_first_form_control_now);
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        cb.forget();
    }
}

fn focus_search_now() {
    let Some(win) = web_sys::window() else { return };
    let Some(doc) = win.document() else { return };
    if let Some(el) = doc.get_element_by_id("search") {
        if let Some(he) = el.dyn_ref::<web_sys::HtmlElement>() {
            let _ = he.focus();
        }
    }
}

pub fn raf_focus_search() {
    if let Some(win) = web_sys::window() {
        let cb = Closure::<dyn FnMut()>::new(focus_search_now);
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        cb.forget();
    }
}

pub fn focus_move(delta: i32) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Ok(list) = doc.query_selector_all(".yal-form-control") {
            let len = list.length() as i32;
            if len == 0 {
                return;
            }

            let active = doc.active_element();
            let mut idx: i32 = -1;
            for i in 0..len {
                if let Some(el) = list.item(i as u32) {
                    if let Some(ae) = &active {
                        if ae.is_same_node(Some(&el)) {
                            idx = i;
                            break;
                        }
                    }
                }
            }
            let next = if idx < 0 {
                0
            } else {
                let mut n = idx + delta;
                if n < 0 {
                    n = 0;
                }
                if n >= len {
                    n = len - 1;
                }
                n
            };
            if let Some(el) = list.item(next as u32) {
                let _ = el.dyn_ref::<web_sys::HtmlElement>().map(|h| h.focus());
            }
        }
    }
}

pub fn active_is_range() -> Option<web_sys::HtmlInputElement> {
    let doc = web_sys::window()?.document()?;
    let ae = doc.active_element()?;
    let input: web_sys::HtmlInputElement = ae.dyn_into().ok()?;
    if input.type_().to_lowercase() == "range" {
        Some(input)
    } else {
        None
    }
}

pub fn nudge_active_slider(delta: f64) {
    if let Some(input) = active_is_range() {
        let step = input.step().parse::<f64>().unwrap_or(1.0);
        let min = input.min().parse::<f64>().unwrap_or(0.0);
        let max = input.max().parse::<f64>().unwrap_or(100.0);
        let cur = input.value().parse::<f64>().unwrap_or(min);
        let mut v = cur + delta * step;
        if v < min {
            v = min;
        }
        if v > max {
            v = max;
        }
        input.set_value(&v.to_string());
        if let Ok(ev) = web_sys::Event::new("input") {
            let _ = input.dispatch_event(&ev);
        }
    }
}
