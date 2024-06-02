use glob::glob;
use mdspec::{md_reader, md_writer};

#[test]
fn test_roundtrip() {
    for entry in glob("testdata/markdown_output/**/nested.md").expect("Failed to read glob pattern")
    {
        let path = match entry {
            Ok(path) => {
                print!("- Testing path: {}... ", path.to_str().expect("utf8 path"));
                path
            }
            Err(err) => {
                eprintln!("{err:?}");
                continue;
            }
        };

        // Read current path into a String buffer.
        let input_string = std::fs::read_to_string(path).expect("file");

        let markdown_input = md_reader(&input_string);
        let mut markdown_output = md_writer(Vec::new());

        println!("---");
        for (event, _pos) in markdown_input.into_offset_iter() {
            println!("pos={_pos:03?} - event={event:?}");
            markdown_output.write(event).unwrap();
        }
        println!("---");

        let output_string = markdown_output.into_string();

        if input_string == output_string {
            println!("OK.");
        } else {
            println!("FAILURE!");
        };

        assert_eq!(&input_string, &output_string)
    }
}
