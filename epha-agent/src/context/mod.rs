

use std::sync::{Arc, Mutex};

pub trait ContextSerialize {
    fn serialize(&self) -> String;
}

pub struct Context<T> {
    data: Arc<Mutex<T>>,
}

impl<T: ContextSerialize> Context<T> {
    pub fn new(data: Arc<Mutex<T>>) -> Self {
        Self { data }
    }

    pub fn data(&self) -> Arc<Mutex<T>> {
        self.data.clone()
    }

    pub fn serialize(&self) -> String {
        let guard = self.data.lock().unwrap();
        let content = guard.serialize();
        let escaped_content = Self::escape_system_tags(content);

        format!(
            "<context>\n{}\n</context>",
            escaped_content
        )
    }

    fn escape_system_tags(content: String) -> String {
        let system_tags = vec!["context", "sys.memory", "sys.agent", "sys.state"];

        let mut escaped_content = content;
        for tag in &system_tags {
            escaped_content = escaped_content.replace(&format!("<{}>", tag), &format!("&lt;{}&gt;", tag));
            escaped_content = escaped_content.replace(&format!("</{}>", tag), &format!("&lt;/{}&gt;", tag));
        }

        escaped_content
    }
}