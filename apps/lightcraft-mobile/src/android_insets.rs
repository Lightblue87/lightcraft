//! Android: how much of each window edge the system bars, display cutout and soft keyboard cover.
//!
//! winit 0.30 drops the activity's content-rect changes, and apps targeting Android 15 are drawn
//! edge to edge, so the window isn't resized for the keyboard either. The decor view's
//! `getWindowVisibleDisplayFrame` (API 1) gives the part of the screen left visible; comparing
//! it with the window's own bounds gives the insets. This is right whether or not Android resized
//! the window (`adjustResize` on older versions), since a resized window's bounds already exclude
//! the keyboard.
#![allow(unsafe_code)] // the JavaVM and activity pointers android-activity hands out

use jni::objects::JObject;
use jni::{JValue, JavaVM, jni_sig, jni_str};
use winit::platform::android::activity::AndroidApp;

/// Pixels each edge of the window loses: `[left, top, right, bottom]`.
pub fn window_insets(app: &AndroidApp) -> Option<[i32; 4]> {
    // SAFETY: android-activity's pointer to the process's JavaVM, valid for the process's life
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let r = vm.attach_current_thread(|env| -> jni::errors::Result<[i32; 4]> {
        // SAFETY: android-activity's global reference to the activity, valid while it runs
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let window = env.call_method(&activity, jni_str!("getWindow"), jni_sig!("()Landroid/view/Window;"), &[])?.l()?;
        let decor = env.call_method(&window, jni_str!("getDecorView"), jni_sig!("()Landroid/view/View;"), &[])?.l()?;
        // the window on screen
        let at = env.new_int_array(2)?;
        env.call_method(&decor, jni_str!("getLocationOnScreen"), jni_sig!("([I)V"), &[JValue::Object(&at)])?;
        let mut xy = [0; 2];
        at.get_region(env, 0, &mut xy)?;
        let w = env.call_method(&decor, jni_str!("getWidth"), jni_sig!("()I"), &[])?.i()?;
        let h = env.call_method(&decor, jni_str!("getHeight"), jni_sig!("()I"), &[])?.i()?;
        // what's visible of the screen
        let rect = env.new_object(jni_str!("android/graphics/Rect"), jni_sig!("()V"), &[])?;
        env.call_method(&decor, jni_str!("getWindowVisibleDisplayFrame"), jni_sig!("(Landroid/graphics/Rect;)V"), &[JValue::Object(&rect)])?;
        let mut side = |name| env.get_field(&rect, name, jni_sig!("I")).and_then(|v| v.i());
        let (left, top, right, bottom) = (side(jni_str!("left"))?, side(jni_str!("top"))?, side(jni_str!("right"))?, side(jni_str!("bottom"))?);
        Ok([(left - xy[0]).max(0), (top - xy[1]).max(0), (xy[0] + w - right).max(0), (xy[1] + h - bottom).max(0)])
    });
    match r {
        Ok(insets) => Some(insets),
        Err(e) => {
            log::warn!("window insets: {e}");
            None
        }
    }
}
