use any_chunk::Chunkable;

#[derive(Chunkable)]
pub struct FileContent {
    pub filename: String,
    #[chunkable]
    pub file_content: String
}

fn main() {

}
