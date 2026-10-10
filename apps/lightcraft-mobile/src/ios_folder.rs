//! iOS: choose a folder in the Files app's document picker (UIDocumentPickerViewController) for
//! File ▸ Import from Folder…: on iCloud Drive, On My iPad, another app's storage, a USB drive or
//! a file server.
//!
//! It answers the UI's background [`Picker`](lightcraft_ui_egui::pick::Picker): [`pick`] shows
//! the picker and returns a channel at once, and the UI runs the command again with the folder
//! when it arrives (`pick::poll`). A folder outside the app's sandbox can only be read while the
//! system grants access to it, so access starts as it is picked and lasts until the app quits (or
//! [`KEEP`] newer folders were picked); the import review starts on Copy (`import_copies`), so the
//! library keeps its own copies.
#![allow(unsafe_code)] // Objective-C classes, messages and the runtime

use std::sync::mpsc::{Sender, channel};

use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject, Bool, NSObject};
use objc2::{MainThreadOnly, define_class, extern_class, extern_methods, msg_send};
use objc2_foundation::{NSArray, NSObjectProtocol, NSString, NSURL};
use objc2_ui_kit::{UIResponder, UIViewController};

use lightcraft_ui_egui::pick::{PickKind, PickRequest};

/// How many picked folders stay readable at once (each holds a sandbox extension).
const KEEP: usize = 16;

extern_class!(
    /// UIKit's document picker.
    #[unsafe(super(UIViewController, UIResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "UIDocumentPickerViewController"]
    struct DocumentPicker;
);

impl DocumentPicker {
    extern_methods!(
        /// iOS 14+: `types` are `UTType`s.
        #[unsafe(method(initForOpeningContentTypes:))]
        fn init_for_opening(this: Allocated<Self>, types: &NSArray<AnyObject>) -> Retained<Self>;

        #[unsafe(method(setDelegate:))]
        fn set_delegate(&self, delegate: Option<&AnyObject>);

        #[unsafe(method(setAllowsMultipleSelection:))]
        fn set_allows_multiple_selection(&self, allows: bool);
    );
}

define_class!(
    /// The picker's delegate: hands the chosen folder to the UI.
    #[unsafe(super(NSObject))]
    #[name = "LCFolderPickerDelegate"]
    #[thread_kind = MainThreadOnly]
    struct FolderDelegate;

    unsafe impl NSObjectProtocol for FolderDelegate {}

    impl FolderDelegate {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn did_pick(&self, _picker: &DocumentPicker, urls: &NSArray<NSURL>) {
            let path = urls.firstObject().and_then(|url| {
                // SAFETY: a URL the picker handed over; balanced by `stopAccessing…` in `keep`
                let granted: Bool = unsafe { msg_send![&*url, startAccessingSecurityScopedResource] };
                let path = url.path().map(|p| p.to_string());
                if granted.as_bool() {
                    keep(url);
                }
                path
            });
            answer(path.into_iter().collect());
        }

        #[unsafe(method(documentPickerWasCancelled:))]
        fn cancelled(&self, _picker: &DocumentPicker) {
            answer(Vec::new());
        }
    }
);

thread_local! {
    // UIKit holds the delegate weakly: keep it, and where the answer goes, while the picker is up
    static UP: std::cell::RefCell<Option<(Retained<FolderDelegate>, Sender<Vec<String>>)>> = const { std::cell::RefCell::new(None) };
    // folders being read (their access ends when they leave)
    static ACCESSED: std::cell::RefCell<Vec<Retained<NSURL>>> = const { std::cell::RefCell::new(Vec::new()) };
    static CTX: std::cell::RefCell<Option<egui::Context>> = const { std::cell::RefCell::new(None) };
}

/// Repaint `ctx` when a folder has been chosen (main thread).
pub fn init(ctx: egui::Context) {
    CTX.with_borrow_mut(|c| *c = Some(ctx));
}

/// The UI's background picker: folders only (other dialogs are answered by the synchronous
/// services, e.g. the photo picker).
pub fn pick(request: PickRequest) -> Result<std::sync::mpsc::Receiver<Vec<String>>, String> {
    if request.kind != PickKind::Folder {
        return Err("only folders are picked here".into());
    }
    let mtm = objc2::MainThreadMarker::new().ok_or("not on the main thread")?;
    let root = crate::ios_scene::root_view_controller().ok_or("no view controller to present from")?;
    // `UTTypeFolder`, looked up at run time (UIKit loads UniformTypeIdentifiers)
    let ty = AnyClass::get(c"UTType").ok_or("no UTType class")?;
    // SAFETY: `+[UTType typeWithIdentifier:]` takes a UTI string and may return nil
    let folder: Option<Retained<AnyObject>> = unsafe { msg_send![ty, typeWithIdentifier: &*NSString::from_str("public.folder")] };
    let folder = folder.ok_or("no folder type")?;
    let types = NSArray::from_retained_slice(&[folder]);
    let picker = DocumentPicker::init_for_opening(DocumentPicker::alloc(mtm), &types);
    picker.set_allows_multiple_selection(false);
    // SAFETY: a fresh NSObject subclass instance
    let delegate: Retained<FolderDelegate> = unsafe { msg_send![FolderDelegate::alloc(mtm), init] };
    picker.set_delegate(Some(&delegate));
    let (tx, rx) = channel();
    // a picker already up answers "cancelled" (its sender is dropped) and this one takes over
    UP.with_borrow_mut(|u| *u = Some((delegate, tx)));
    // SAFETY: presenting a view controller from the root one, on the main thread
    let _: () = unsafe { msg_send![&*root, presentViewController: &*picker, animated: true, completion: std::ptr::null::<AnyObject>()] };
    Ok(rx)
}

/// Send the picker's answer (empty: cancelled) and wake the UI.
fn answer(paths: Vec<String>) {
    if let Some((_, tx)) = UP.with_borrow_mut(Option::take) {
        let _ = tx.send(paths);
    }
    CTX.with_borrow(|c| {
        if let Some(c) = c {
            c.request_repaint();
        }
    });
}

/// Keep reading `url`; the oldest folder beyond [`KEEP`] gives its access back.
fn keep(url: Retained<NSURL>) {
    ACCESSED.with_borrow_mut(|a| {
        a.push(url);
        while a.len() > KEEP {
            let old = a.remove(0);
            // SAFETY: balancing the `startAccessing…` that put it here
            let _: () = unsafe { msg_send![&*old, stopAccessingSecurityScopedResource] };
        }
    });
}
