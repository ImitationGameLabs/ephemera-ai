

pub trait ContextSerialize {
    fn serialize(&self) -> String;
}

pub struct Context<T> {
    data: T,
}

impl<T: ContextSerialize> Context<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn serialize(&self) -> String {
        let content = self.data.serialize();
        let escaped_content = self.escape_system_tags(content);

        format!(
            "<context>\n{}\n</context>",
            escaped_content
        )
    }

    fn escape_system_tags(&self, content: String) -> String {
        let system_tags = vec!["context", "sys.memory", "sys.agent", "sys.state"];

        let mut escaped_content = content;
        for tag in &system_tags {
            escaped_content = escaped_content.replace(&format!("<{}>", tag), &format!("&lt;{}&gt;", tag));
            escaped_content = escaped_content.replace(&format!("</{}>", tag), &format!("&lt;/{}&gt;", tag));
        }

        escaped_content
    }
}