//! Opens a real window with the OS backend. Needs a display, so it is
//! ignored by default; CI runs it with `cargo test -- --ignored`.
use engine::gfx;
use engine::input::Input;
use engine::scene::Scene;

#[test]
#[ignore]
fn window_opens_polls_and_presents() {
    let scene = Scene::parse(include_str!("../scenes/default.scene")).unwrap();
    let mut win = engine::platform::open("smoke", 320, 180).expect("open window");
    let mut r = gfx::create("auto", 320, 180, win.as_ref()).expect("renderer");
    r.upload_scene(&scene).unwrap();
    let mut input = Input::default();
    for _ in 0..10 {
        input.begin_frame();
        assert!(win.poll(&mut input));
        r.render(&scene, &scene.camera, win.as_mut()).unwrap();
    }
    win.set_capture(true);
    win.set_capture(false);
}
