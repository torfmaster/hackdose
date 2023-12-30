pub(crate) fn get_viewport() -> (u32, u32) {
    let window = gloo::utils::window();

    let width = (window.inner_width().unwrap().as_f64().unwrap() * 0.7) as u32;
    let height = (window.inner_height().unwrap().as_f64().unwrap() * 0.7) as u32;
    (width, height)
}
