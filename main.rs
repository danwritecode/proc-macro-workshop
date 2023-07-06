use promptize::Promptize;


#[derive(Promptize, Debug, serde::Serialize)]
pub struct FileContent {
    system_prompt: String,
    user_prompt: String,
    pub filename: String,
    #[chunkable]
    pub file_content: String
}


fn main() {
    let contents = std::fs::read_to_string("/home/dan/documents/apps/temp/proc-macro-workshop/test_files/huge_file.rs").unwrap();
    let contents = contents
        .lines()
        .map(|l| l.trim())
        .collect::<String>();

    let foo = FileContent::builder()
        .system_prompt(format!("You are a computer system that responds only in JSON format with no other words except for the JSON."))
        .user_prompt(format!("You are a computer system that responds only in JSON format with no other words except for the JSON."))
        .filename("huge_file.rs".to_string())
        .file_content(contents)
        .build_prompt("gpt-4", 8192, 4000)
        .unwrap();

    println!("{:#?}", foo);
}
