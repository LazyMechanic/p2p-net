use rand::distributions::Alphanumeric;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
}

impl Message {
    pub fn with_random_text() -> Message {
        let text: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();

        Message { text }
    }

    pub fn try_from_bytes(b: &[u8]) -> anyhow::Result<Message> {
        let text = String::from_utf8_lossy(b);
        Ok(Message {
            text: text.to_string(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(std::mem::size_of::<u64>() + self.text.as_bytes().len());
        let size = self.text.as_bytes().len() as u64;

        v.extend_from_slice(&size.to_le_bytes());
        v.extend_from_slice(self.text.as_bytes());

        v
    }
}
