//! UIScene lifecycle for winit 0.30 on iOS.
//!
//! iOS 27 stops apps that don't adopt the scene lifecycle
//! (`_UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption`, a trap at launch), and winit
//! 0.30 (what eframe 0.36 uses) knows nothing about scenes. So the shell adopts it:
//! `ios/Info.plist` declares a scene configuration whose delegate is [`SceneDelegate`]
//! (`LCSceneDelegate`), and the window winit created is attached to the scene once both exist.
//! winit takes the app's lifecycle from `UIApplication…Notification`s, which UIKit still posts
//! for scene-based apps, so nothing else needs forwarding.
//!
//! Drop this module once eframe moves to a winit with its own scene support (0.31).
#![allow(unsafe_code)] // Objective-C class definition and the UIView pointer from the window handle

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{ClassType, MainThreadOnly, Message, define_class};
use objc2_foundation::NSObjectProtocol;
use objc2_ui_kit::{
    UIResponder, UIScene, UISceneConnectionOptions, UISceneDelegate, UISceneSession, UIView, UIWindow, UIWindowScene, UIWindowSceneDelegate,
};

thread_local! {
    // main thread only: UIKit calls the delegate there, and eframe creates the app there
    static SCENE: RefCell<Option<Retained<UIWindowScene>>> = const { RefCell::new(None) };
    static WINDOW: RefCell<Option<Retained<UIWindow>>> = const { RefCell::new(None) };
}

define_class!(
    /// The scene delegate named in `ios/Info.plist` (`UISceneDelegateClassName`).
    #[unsafe(super(UIResponder, NSObject))]
    #[name = "LCSceneDelegate"]
    #[thread_kind = MainThreadOnly]
    struct SceneDelegate;

    unsafe impl NSObjectProtocol for SceneDelegate {}

    unsafe impl UISceneDelegate for SceneDelegate {
        #[unsafe(method(scene:willConnectToSession:options:))]
        fn will_connect(&self, scene: &UIScene, _session: &UISceneSession, _options: &UISceneConnectionOptions) {
            let Some(scene) = scene.downcast_ref::<UIWindowScene>() else { return };
            log::info!("scene connected");
            SCENE.with_borrow_mut(|s| *s = Some(scene.retain()));
            attach();
        }
    }

    unsafe impl UIWindowSceneDelegate for SceneDelegate {}
);

/// Registers `LCSceneDelegate` with the Objective-C runtime; before the event loop starts, since
/// UIKit looks the class up by name when it connects the scene.
pub fn register() {
    let _ = SceneDelegate::class();
}

/// Hands over the window winit created (from the view in eframe's window handle).
pub fn set_window(handle: raw_window_handle::RawWindowHandle) {
    let raw_window_handle::RawWindowHandle::UiKit(h) = handle else { return };
    // SAFETY: on iOS the window handle's `ui_view` is winit's UIView, alive while the window is
    let view: &UIView = unsafe { h.ui_view.cast::<UIView>().as_ref() };
    let Some(window) = view.window() else { return };
    WINDOW.with_borrow_mut(|w| *w = Some(window));
    attach();
}

/// Puts the window on the scene once both exist (in either order) and shows it.
fn attach() {
    SCENE.with_borrow(|scene| {
        WINDOW.with_borrow(|window| {
            if let (Some(scene), Some(window)) = (scene, window) {
                window.setWindowScene(Some(scene));
                window.makeKeyAndVisible();
                log::info!("window attached to scene");
            }
        })
    });
}
