use std::fmt;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ServerSentEvents {
    comment: Option<String>,
    event: Option<String>,
    id: Option<String>,
    retry: Option<u64>,
    data: Option<String>,
}

impl ServerSentEvents {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn with_retry(mut self, retry: u64) -> Self {
        self.retry = Some(retry);
        self
    }

    pub fn with_data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }

    pub fn encode(&self) -> String {
        let mut output = String::new();
        if let Some(comment) = &self.comment {
            output.push_str(": ");
            output.push_str(comment);
            output.push('\n');
        }
        if let Some(event) = &self.event {
            output.push_str("event: ");
            output.push_str(event);
            output.push('\n');
        }
        if let Some(id) = &self.id {
            output.push_str("id: ");
            output.push_str(id);
            output.push('\n');
        }
        if let Some(retry) = self.retry {
            output.push_str(&format!("retry: {retry}\n"));
        }
        if let Some(data) = &self.data {
            for line in data.split('\n') {
                output.push_str("data: ");
                output.push_str(line);
                output.push('\n');
            }
        }
        output.push('\n');
        output
    }
}

impl fmt::Display for ServerSentEvents {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.encode())
    }
}
