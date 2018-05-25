# Implementation

1. Split input text into blocks.
   ```rust
   // f: BufRead
   let mut lines = f.lines();
   let mut block = Vec::new();
   while let Some(line) = lines.next() {
       // handle IO errors
       let line = line?;
       if line.trim().len() == 0 {
           // parse_block: Iterator<String> -> Result<(), Error>
           parser.parse_block(block.drain(..))?;
       } else {
           block.push(line);
       }
   }
   ```
2. Parse blocks into document tree.
3. Traverse tree, outputting final HTML.
