use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::prelude::*;
use serenity::prelude::Context;

pub struct Page {
    pub pages: Vec<CreateEmbed>,
    pub current_page: usize,
}

impl Page {
    /// Create a new pagination with pages
    pub fn new(pages: Vec<CreateEmbed>) -> Self {
        Page {
            pages,
            current_page: 0,
        }
    }

    /// Get the current page embed
    pub fn current_embed(&self) -> &CreateEmbed {
        &self.pages[self.current_page]
    }

    /// Move to next page
    pub fn next(&mut self) -> bool {
        if self.current_page < self.pages.len() - 1 {
            self.current_page += 1;
            true
        } else {
            false
        }
    }

    /// Move to previous page
    pub fn previous(&mut self) -> bool {
        if self.current_page > 0 {
            self.current_page -= 1;
            true
        } else {
            false
        }
    }

    /// Get total number of pages
    pub fn total_pages(&self) -> usize {
        self.pages.len()
    }

    /// Check if on first page
    pub fn is_first(&self) -> bool {
        self.current_page == 0
    }

    /// Check if on last page
    pub fn is_last(&self) -> bool {
        self.current_page == self.pages.len() - 1
    }

    /// Create a message with embed
    pub fn create_message(&self) -> CreateMessage {
        CreateMessage::default()
            .embed(self.current_embed().clone())
    }
}