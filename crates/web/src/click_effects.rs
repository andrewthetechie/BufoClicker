//! Short lived click feedback rendered above the scrollable game columns.

use game::economy::ClickOutcome;
use wasm_bindgen::JsCast;
use web_sys::{Element, MouseEvent};

const BUFO_IMAGES: &[&str] = &[
    "./assets/images/bufo.png",
    "./assets/images/generators/bufo-smol.png",
    "./assets/images/generators/bufo-brain.png",
    "./assets/images/generators/bufo-cash-money.png",
    "./assets/images/generators/bufo-galaxy-brain.png",
    "./assets/images/generators/bufo-has-midas-touch.png",
    "./assets/images/generators/bufo-monstera.png",
    "./assets/images/generators/bufo-old.png",
    "./assets/images/generators/chonky-bufo-wants-to-be-held.png",
    "./assets/images/generators/hypnobufo.png",
    "./assets/images/generators/smol-bufo-feels-blessed.png",
    "./assets/images/upgrades/bufo-dapper.png",
    "./assets/images/upgrades/bufo-drake-yes.png",
    "./assets/images/upgrades/bufo-mindblown.png",
    "./assets/images/upgrades/bufo-simba.png",
    "./assets/images/upgrades/bufo-gives-star.png",
    "./assets/images/upgrades/bufo-give-money.png",
    "./assets/images/upgrades/bufo-chefkiss-with-hat.png",
    "./assets/images/upgrades/bufo-deal-with-it.png",
    "./assets/images/upgrades/bufo-gentleman.png",
    "./assets/images/upgrades/king-bufo.png",
    "./assets/images/upgrades/shut-up-and-take-my-bufo.png",
    "./assets/images/upgrades/bufo-caught-a-small-bufo.png",
    "./assets/images/upgrades/bufo-iron-throne.png",
    "./assets/images/upgrades/bufo-universe.png",
    "./assets/images/upgrades/confused-math-bufo.png",
];

fn random_between(min: f64, max: f64) -> f64 {
    min + js_sys::Math::random() * (max - min)
}

fn add_for(layer: &Element, child: &Element, duration_ms: u32) {
    let _ = layer.append_child(child);
    let child = child.clone();
    gloo_timers::callback::Timeout::new(duration_ms, move || child.remove()).forget();
}

pub fn show(event: &MouseEvent, outcome: &ClickOutcome, is_combo: bool) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Some(body) = document.body() else {
        return;
    };
    let layer = document
        .query_selector(".click-effect-layer")
        .ok()
        .flatten()
        .or_else(|| {
            let layer = document.create_element("div").ok()?;
            layer.set_class_name("click-effect-layer");
            body.append_child(&layer).ok()?;
            Some(layer)
        });
    let Some(layer) = layer else { return };
    let x = f64::from(event.client_x());
    let y = f64::from(event.client_y());

    let clicked_image = event
        .current_target()
        .and_then(|target| target.dyn_into::<Element>().ok())
        .and_then(|target| target.query_selector(".bufo-image").ok().flatten());
    if let Some(image) = clicked_image {
        let _ = image.set_attribute("style", "transform: scale(0.95)");
        gloo_timers::callback::Timeout::new(90, move || {
            let _ = image.remove_attribute("style");
        })
        .forget();
    }

    if let Ok(ripple) = document.create_element("div") {
        ripple.set_class_name("click-indicator");
        let _ = ripple.set_attribute("style", &format!("left:{x}px;top:{y}px"));
        add_for(&layer, &ripple, 1_000);
    }

    if let Ok(number) = document.create_element("div") {
        number.set_class_name(if is_combo {
            "floating-number combo"
        } else {
            "floating-number"
        });
        let left = (x + random_between(-15.0, 15.0)).max(0.0);
        let top = y + random_between(-10.0, 10.0);
        let combo_style = if is_combo {
            "background-color:rgba(76,175,80,.2);padding:4px 8px;border-radius:10px;"
        } else {
            ""
        };
        let _ = number.set_attribute(
            "style",
            &format!(
                "left:{left}px;top:{top}px;font-weight:bold;font-size:{};color:{};text-shadow:0 1px 2px rgba(0,0,0,.8);z-index:1000;{combo_style}",
                if is_combo { "1.4em" } else { "1.2em" },
                if is_combo { "#FFD700" } else { "#FFFFFF" }
            ),
        );
        if let Ok(value) = document.create_element("span") {
            value.set_class_name("value");
            value.set_text_content(Some(&format!("+{:.1}", outcome.earned)));
            let _ = number.append_child(&value);
        }
        add_for(&layer, &number, 1_500);
    }

    if let Ok(pop) = document.create_element("div") {
        pop.set_class_name("click-emoji-pop click-emoji-arc");
        let dx = random_between(-70.0, 70.0);
        let dy = -random_between(50.0, 100.0);
        let rotation = random_between(-50.0, 50.0);
        let duration = random_between(800.0, 1_000.0);
        let viewport_width = web_sys::window()
            .and_then(|window| window.inner_width().ok())
            .and_then(|width| width.as_f64())
            .unwrap_or(x);
        let pop_x = x.max(22.0).min((viewport_width - 22.0).max(22.0));
        let _ = pop.set_attribute(
            "style",
            &format!(
                "left:{}px;top:{y}px;--half-x:{}px;--peak-y:{dy}px;--half-r:{}deg;--full-x:{dx}px;--full-r:{rotation}deg;animation-duration:{duration}ms",
                pop_x,
                dx / 2.0,
                rotation / 2.0,
            ),
        );
        if let Ok(image) = document.create_element("img") {
            let index = (js_sys::Math::random() * BUFO_IMAGES.len() as f64) as usize;
            let _ = image.set_attribute("src", BUFO_IMAGES[index]);
            let _ = image.set_attribute("alt", "");
            let _ = image.set_attribute("draggable", "false");
            let _ = pop.append_child(&image);
        }
        add_for(&layer, &pop, duration.ceil() as u32);
    }
}
