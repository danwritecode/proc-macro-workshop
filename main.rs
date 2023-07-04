use promptize::Promptize;

#[derive(Promptize, Debug)]
pub struct FileContent {
    user: String,
    system: String,
    pub filename: String,
    #[chunkable]
    pub file_content: String
}

// impl FileContent {
//     fn foo(&self) -> Self {
//         FileContent {
//             filename: "Foo".to_string(),
//             file_content: "bar".to_string()
//         }
//     }
// }

fn main() {
    let foo = FileContent::builder()
        .user("bar".to_string())
        .system("bar".to_string())
        .filename("Foo".to_string())
        .file_content("Foo".to_string())
        .build_prompt()
        .unwrap();

    println!("{:#?}", foo);
}
