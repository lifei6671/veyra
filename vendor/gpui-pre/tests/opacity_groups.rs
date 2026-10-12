use gpui::{
    BackdropBlur, Bounds, ContentMask, OpacitySegment, PrimitiveBatch, Quad, Scene, point, size,
};

fn quad(marker: f32) -> Quad {
    let bounds = Bounds::new(
        point(marker.into(), 0.0.into()),
        size(20.0.into(), 20.0.into()),
    );
    Quad {
        bounds,
        content_mask: ContentMask { bounds },
        ..Quad::default()
    }
}

fn signature(scene: &Scene) -> String {
    let mut result = String::new();
    for segment in &scene.opacity_segments {
        match segment {
            OpacitySegment::Backdrop(blur) => result.push_str(&format!("B{};", blur.sigma.0)),
            OpacitySegment::Direct(direct) => result.push_str(&signature(direct)),
            OpacitySegment::Group { scene, opacity } => {
                result.push_str(&format!("G{opacity}[{}]", signature(scene)));
            }
        }
    }
    for quad in &scene.quads {
        result.push_str(&format!("Q{};", quad.bounds.origin.x.0));
    }
    result
}

// 保护 busy 节点前后卡片的绘制顺序，以及原 scene paint-cache 的全局索引。
#[test]
fn ordered_nested_groups_survive_flat_paint_replay() {
    let mut scene = Scene::default();
    let bounds = quad(0.).bounds;
    scene.push_layer(bounds);
    scene.insert_primitive(quad(1.));
    scene.begin_opacity_group(0.55);
    scene.insert_primitive(quad(2.));
    scene.begin_opacity_group(0.5);
    scene.insert_primitive(quad(3.));
    scene.end_opacity_group();
    scene.insert_primitive(quad(4.));
    scene.end_opacity_group();
    scene.insert_primitive(quad(5.));
    scene.pop_layer();
    scene.finish();
    assert_eq!(signature(&scene), "Q1;G0.55[Q2;G0.5[Q3;]Q4;]Q5;");
    let mut replay = Scene::default();
    replay.replay(0..scene.len(), &scene);
    replay.finish();
    assert_eq!(signature(&replay), signature(&scene));
    assert_eq!(replay.len(), scene.len());
    assert_eq!(scene.quads[0].order, 1);
}

// 保护复用缓存 child 时仍进入当前隔离组，不能画回根场景。
#[test]
fn cached_child_replay_inside_group_and_clear() {
    let mut previous = Scene::default();
    previous.insert_primitive(quad(2.));
    previous.insert_primitive(quad(3.));
    previous.finish();
    let mut scene = Scene::default();
    scene.begin_opacity_group(0.55);
    scene.replay(0..previous.len(), &previous);
    scene.end_opacity_group();
    scene.finish();
    assert_eq!(signature(&scene), "G0.55[Q2;Q3;]");
    scene.clear();
    assert!(scene.opacity_segments.is_empty());
    assert_eq!(scene.len(), 0);
    scene.insert_primitive(quad(9.));
    scene.finish();
    assert_eq!(signature(&scene), "Q9;");
}

// 默认页面不产生隔离段，保留上游 primitive batch 行为。
#[test]
fn default_scene_batching_unchanged() {
    let mut scene = Scene::default();
    scene.insert_primitive(quad(0.));
    scene.insert_primitive(quad(0.));
    scene.finish();
    assert!(scene.opacity_segments.is_empty());
    let batches: Vec<_> = scene.batches().collect();
    assert_eq!(batches.len(), 1);
    assert!(matches!(&batches[0], PrimitiveBatch::Quads(range) if range == &(0..2)));
}

// 背景过滤必须保留在缓存重放顺序中，不能被图元类型 batch 重排/吞掉。
#[test]
fn backdrop_barrier_replays_between_groups_and_foreground() {
    let mut scene = Scene::default();
    scene.insert_primitive(quad(1.));
    scene.begin_opacity_group(0.5);
    scene.insert_primitive(quad(2.));
    scene.end_opacity_group();
    let bounds = quad(0.).bounds;
    scene.insert_backdrop_blur(BackdropBlur {
        bounds,
        corner_radii: Default::default(),
        content_mask: ContentMask { bounds },
        sigma: 10.0.into(),
    });
    scene.insert_primitive(quad(3.));
    scene.finish();
    assert_eq!(signature(&scene), "Q1;G0.5[Q2;]B10;Q3;");
    assert!(scene.has_backdrop_blur());
    let mut replay = Scene::default();
    replay.replay(0..scene.len(), &scene);
    replay.finish();
    assert_eq!(signature(&replay), signature(&scene));
    assert_eq!(replay.len(), scene.len());
    replay.clear();
    assert!(!replay.has_backdrop_blur());
}
