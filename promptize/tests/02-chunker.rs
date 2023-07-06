use promptize::Promptize;

#[derive(Promptize)]
pub struct FileContent {
    pub filename: String,
    #[chunkable]
    pub file_content: String
}

fn main() {

}
