pub struct WindowSpecs {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
}

impl Default for WindowSpecs {
    fn default() -> Self {
        Self {
            title: "reng".into(),
            width: 960,
            height: 540,
            resizable: true,
        }
    }
}
