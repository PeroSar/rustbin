use std::ffi::{CStr, c_char};

unsafe extern "C" {
    fn DetectLanguageByClassifier(content: *const c_char, content_len: i32) -> *mut c_char;
    fn FreeEnryString(value: *mut c_char);
}

pub fn detect_language_by_classifier(content: &str) -> Option<String> {
    if content.is_empty() || content.len() > i32::MAX as usize {
        return None;
    }

    let detected = unsafe {
        DetectLanguageByClassifier(content.as_ptr().cast::<c_char>(), content.len() as i32)
    };
    if detected.is_null() {
        return None;
    }

    let language = unsafe { CStr::from_ptr(detected) }
        .to_string_lossy()
        .into_owned();
    unsafe { FreeEnryString(detected) };

    Some(language)
}
