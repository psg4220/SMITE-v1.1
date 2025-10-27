/// A simple text-based table generator for Discord messages using code blocks
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    col_widths: Vec<usize>,
}

impl Table {
    /// Create a new table with the given headers
    pub fn new(headers: Vec<&str>) -> Self {
        let col_widths = headers.iter().map(|h| h.len()).collect();
        let headers = headers.iter().map(|h| h.to_string()).collect();
        Table {
            headers,
            rows: Vec::new(),
            col_widths,
        }
    }

    /// Add a row to the table
    pub fn add_row(&mut self, row: Vec<&str>) {
        let row_strings: Vec<String> = row.iter().map(|s| s.to_string()).collect();
        
        // Update column widths if needed
        for (i, col) in row_strings.iter().enumerate() {
            if i < self.col_widths.len() {
                self.col_widths[i] = self.col_widths[i].max(col.len());
            }
        }
        
        self.rows.push(row_strings);
    }

    /// Render the table as a formatted string for Discord
    pub fn render(&self) -> String {
        let mut output = String::from("```\n");
        
        // Add header
        output.push_str(&self.render_row(&self.headers));
        output.push('\n');
        
        // Add separator
        output.push_str(&self.render_separator());
        output.push('\n');
        
        // Add rows
        for row in &self.rows {
            output.push_str(&self.render_row(row));
            output.push('\n');
        }
        
        output.push_str("```");
        output
    }

    /// Render a single row with proper spacing
    fn render_row(&self, row: &[String]) -> String {
        let mut line = String::new();
        for (i, col) in row.iter().enumerate() {
            if i < self.col_widths.len() {
                let width = self.col_widths[i];
                line.push_str(&format!("{:<width$}", col, width = width));
                if i < row.len() - 1 {
                    line.push_str(" | ");
                }
            }
        }
        line
    }

    /// Render a separator line
    fn render_separator(&self) -> String {
        let mut line = String::new();
        for (i, &width) in self.col_widths.iter().enumerate() {
            line.push_str(&"-".repeat(width));
            if i < self.col_widths.len() - 1 {
                line.push_str("-+-");
            }
        }
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_table() {
        let mut table = Table::new(vec!["Name", "Age", "City"]);
        table.add_row(vec!["Alice", "30", "NYC"]);
        table.add_row(vec!["Bob", "25", "LA"]);
        
        let rendered = table.render();
        assert!(rendered.contains("Name"));
        assert!(rendered.contains("Age"));
        assert!(rendered.contains("Alice"));
        assert!(rendered.contains("Bob"));
    }
}
