use crate::{core::project::ClipboardContent, lock::get_app_read};

pub fn get_clipboard_contents<T, F>(mapper: F) -> T 
where 
    F: FnOnce(&ClipboardContent) -> T 
{
    let app = get_app_read();
    mapper(&app.clipboard)
    
}